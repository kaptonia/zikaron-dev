//! Local IPC endpoint (the "door"): a channel one process keeps open for other processes of the same user on
//! this machine. On unix it is a socket file in the machine directory, accessible to its owner only; on Windows
//! a named pipe whose ACL allows only this user and rejects remote clients. Once a peer connects, the OS is
//! asked which user the other end runs as, and another user is refused on both sides (the listener refuses a
//! foreign client; the client refuses an endpoint owned by another user). There is no token: anyone able to
//! read one could already reach the endpoint.
//!
//! This module decides where an endpoint lives ([`place`]); the protocol over it is the caller's.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

/// Prefix of every endpoint name (the token follows).
pub const NAME_HEAD: &str = "zikaron-door-";

/// The location of the endpoint named by `token`, for machine directory `machine`.
pub fn place(machine: &Path, token: &str) -> PathBuf {
    crate::imp::door_place(machine, &format!("{NAME_HEAD}{token}"))
}

/// Maximum endpoint path length in bytes on this OS (unix: a local socket path).
pub const PLACE_MAX: usize = crate::imp::DOOR_PATH_MAX;

/// Whether `place` is too long for an endpoint on this OS; such a path is refused (`InvalidInput`), never
/// truncated.
pub fn too_long(place: &Path) -> bool {
    place.as_os_str().len() > PLACE_MAX
}

static STRANGER: AtomicBool = AtomicBool::new(false);

/// Treat every peer as another user from now on in this process (`false`: ask the OS). For tests of both
/// refusals; production never calls it.
pub fn pretend_stranger(on: bool) {
    STRANGER.store(on, Ordering::SeqCst);
}

fn stranger() -> std::io::Result<()> {
    if STRANGER.load(Ordering::SeqCst) {
        return Err(std::io::Error::new(std::io::ErrorKind::PermissionDenied, "the other end is another user (pretended)"));
    }
    Ok(())
}

/// An open endpoint; [`Listener::accept`] waits for the next client.
pub struct Listener {
    door: crate::imp::DoorListener,
    shut: Arc<AtomicBool>,
}

/// Closes an endpoint from a thread other than the one waiting on it.
#[derive(Clone)]
pub struct Closer {
    place: PathBuf,
    shut: Arc<AtomicBool>,
}

/// One connection, on either side: a bidirectional byte stream.
pub struct Stream(crate::imp::DoorStream);

/// Open the endpoint at `place`. A stale endpoint left by a dead process (a socket file nobody answers) is
/// removed first; a live endpoint, or anything else at the path, is `AddrInUse` and left untouched; a path too
/// long for this OS is `InvalidInput`.
pub fn listen(place: &Path) -> std::io::Result<(Listener, Closer)> {
    if too_long(place) {
        return Err(crate::named("door", place.display(), std::io::Error::new(std::io::ErrorKind::InvalidInput, format!("longer than {PLACE_MAX} bytes"))));
    }
    let door = crate::imp::door_listen(place).map_err(|e| crate::named("door", place.display(), e))?;
    let shut = Arc::new(AtomicBool::new(false));
    Ok((Listener { door, shut: shut.clone() }, Closer { place: place.to_path_buf(), shut }))
}

impl Listener {
    /// The next client, or `None` once the endpoint is closed ([`Closer::close`]). A client running as another
    /// user is refused (`PermissionDenied`) and the endpoint stays open.
    pub fn accept(&self) -> std::io::Result<Option<Stream>> {
        let got = self.door.accept();
        if self.shut.load(Ordering::SeqCst) {
            return Ok(None);
        }
        let s = got?;
        stranger()?;
        Ok(Some(Stream(s)))
    }
}

impl Closer {
    /// Close the endpoint: a waiting `accept` wakes and returns `None`, and the path is removed. Connections
    /// accepted earlier remain the caller's to finish.
    pub fn close(&self) {
        if !self.shut.swap(true, Ordering::SeqCst) {
            crate::imp::door_knock(&self.place);
            crate::imp::door_remove(&self.place);
        }
    }

    /// Whether [`Closer::close`] was called.
    pub fn closed(&self) -> bool {
        self.shut.load(Ordering::SeqCst)
    }
}

/// Connect to the endpoint at `place`: none there (or a stale one) is `NotFound`; one owned by another user is
/// `PermissionDenied`; a path too long for this OS is `InvalidInput`.
pub fn connect(place: &Path) -> std::io::Result<Stream> {
    if too_long(place) {
        return Err(crate::named("door", place.display(), std::io::Error::new(std::io::ErrorKind::InvalidInput, format!("longer than {PLACE_MAX} bytes"))));
    }
    let s = crate::imp::door_connect(place).map_err(|e| crate::named("door", place.display(), e))?;
    stranger().map_err(|e| crate::named("door", place.display(), e))?;
    Ok(Stream(s))
}

impl Stream {
    /// How long a read waits before returning `TimedOut` (`None`: forever).
    pub fn set_read_deadline(&self, d: Option<std::time::Duration>) -> std::io::Result<()> {
        self.0.set_read_deadline(d)
    }
}

impl std::io::Read for Stream {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        self.0.read(buf)
    }
}

impl std::io::Write for Stream {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.0.write(buf)
    }
    fn flush(&mut self) -> std::io::Result<()> {
        self.0.flush()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Read, Write};

    /// `pretend_stranger` is process-wide, so these tests run serially.
    static TURN: std::sync::Mutex<()> = std::sync::Mutex::new(());

    fn scratch(tag: &str) -> PathBuf {
        // Kept short: local socket paths have a small length limit.
        let d = std::env::temp_dir().join(format!("zkd-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).expect("scratch");
        d
    }

    /// Bytes flow both ways; closing wakes the waiter and removes the path; a closed endpoint cannot be reached.
    #[test]
    fn a_door_carries_bytes_both_ways_and_closes() {
        let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
        let d = scratch("both");
        let at = place(&d, "t1");
        let (l, closer) = listen(&at).expect("open");
        let server = std::thread::spawn(move || {
            let mut s = l.accept().expect("accept").expect("open still");
            let mut b = [0u8; 4];
            s.read_exact(&mut b).expect("read");
            s.write_all(&[b[0] + 1, b[1] + 1, b[2] + 1, b[3] + 1]).expect("write");
            drop(s);
            // Closed while waiting: woken, and returns `None`.
            l.accept().expect("woken")
        });
        let mut c = connect(&at).expect("in");
        c.write_all(&[1, 2, 3, 4]).expect("write");
        let mut back = [0u8; 4];
        c.read_exact(&mut back).expect("read");
        assert_eq!(back, [2, 3, 4, 5]);
        std::thread::sleep(std::time::Duration::from_millis(50));
        closer.close();
        assert!(server.join().expect("server").is_none(), "a closed door says so");
        assert_eq!(connect(&at).err().map(|e| e.kind()), Some(std::io::ErrorKind::NotFound), "nobody at a closed door");
        let _ = std::fs::remove_dir_all(&d);
    }

    /// A stale endpoint from a dead process is replaced when a new one opens; a live one is not.
    #[cfg(unix)]
    #[test]
    fn a_door_left_behind_is_replaced_and_an_open_one_is_not() {
        let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
        let d = scratch("left");
        let at = place(&d, "t2");
        // Stale: opened and never closed by a process that stopped (the path exists, nobody answers).
        let (left, _never_closed) = listen(&at).expect("left");
        drop(left);
        assert!(at.exists());
        let (_l, closer) = listen(&at).expect("a door left behind is replaced");
        assert_eq!(listen(&at).err().map(|e| e.kind()), Some(std::io::ErrorKind::AddrInUse), "an open door is not taken");
        closer.close();
        // Anything other than an endpoint at the path is never removed.
        std::fs::write(&at, b"not a door").expect("file");
        assert_eq!(listen(&at).err().map(|e| e.kind()), Some(std::io::ErrorKind::AddrInUse));
        assert_eq!(std::fs::read(&at).expect("kept"), b"not a door");
        let _ = std::fs::remove_dir_all(&d);
    }

    /// The socket file is accessible to its owner only.
    #[cfg(unix)]
    #[test]
    fn the_door_is_its_owners_only() {
        let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
        let d = scratch("own");
        let at = place(&d, "t3");
        let (_l, closer) = listen(&at).expect("open");
        assert!(crate::is_owner_only(&at).expect("read"));
        closer.close();
        let _ = std::fs::remove_dir_all(&d);
    }

    /// A path too long for this OS is refused by name for both listen and connect, never truncated.
    #[test]
    fn a_place_too_long_is_refused_by_name() {
        let long = std::env::temp_dir().join("d".repeat(PLACE_MAX + 1));
        assert!(too_long(&long));
        assert_eq!(listen(&long).err().map(|e| e.kind()), Some(std::io::ErrorKind::InvalidInput));
        assert_eq!(connect(&long).err().map(|e| e.kind()), Some(std::io::ErrorKind::InvalidInput));
        assert!(!long.exists());
    }

    /// Another user is refused on both sides: the client is told, and the endpoint stays open.
    #[test]
    fn another_user_is_refused_both_ways() {
        let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
        let d = scratch("other");
        let at = place(&d, "t4");
        let (l, closer) = listen(&at).expect("open");
        let (tx, rx) = std::sync::mpsc::channel();
        let server = std::thread::spawn(move || {
            let _ = tx.send(l.accept().map(|s| s.is_some()).map_err(|e| e.kind()));
            l.accept().map(|s| s.is_some()).map_err(|e| e.kind())
        });
        pretend_stranger(true);
        let refused = connect(&at).err().map(|e| e.kind());
        let first = rx.recv().expect("first");
        pretend_stranger(false);
        assert_eq!(refused, Some(std::io::ErrorKind::PermissionDenied), "the one who came in refused a door kept by a stranger");
        assert_eq!(first, Err(std::io::ErrorKind::PermissionDenied), "the door refused the stranger");
        closer.close();
        assert_eq!(server.join().expect("server"), Ok(false), "and was still open after it");
        let _ = std::fs::remove_dir_all(&d);
    }

    /// A read with a deadline returns `TimedOut` when nothing arrives.
    #[test]
    fn a_read_waits_no_longer_than_its_deadline() {
        let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
        let d = scratch("dl");
        let at = place(&d, "t5");
        let (l, closer) = listen(&at).expect("open");
        let server = std::thread::spawn(move || {
            let mut s = l.accept().expect("accept").expect("open");
            s.set_read_deadline(Some(std::time::Duration::from_millis(100))).expect("deadline");
            let mut b = [0u8; 1];
            s.read(&mut b).map_err(|e| e.kind())
        });
        let _c = connect(&at).expect("in");
        assert_eq!(server.join().expect("server").err(), Some(std::io::ErrorKind::TimedOut));
        closer.close();
        let _ = std::fs::remove_dir_all(&d);
    }
}
