//! Single writer: when two instances open the same home on one machine, the second is read-only.
//!
//! ─── No stale locks ───
//!
//! A lock that is just a file outlives a killed process (SIGKILL, power loss, crash), and deciding whether it
//! is stale means guessing by pid, timestamp or heartbeat. Each can be wrong (pids are reused, clocks roll
//! back, heartbeats need their own state), and a wrong guess yields two writers at once.
//!
//! Instead the lock's lifetime belongs to the kernel: `flock` is an advisory lock on an open file descriptor,
//! released as soon as the process ends, SIGKILL and power loss included. Getting it means nobody holds it;
//! failing means somebody does. The app never deletes lock files, and nobody needs to by hand.
//!
//! The holder text in the file is only for display ("who holds it") and never affects the decision.
//!
//! ─── Another machine ───
//!
//! Taking the lock (the kernel lock and the writer mark together) is the sealed layer's, one implementation with
//! the command line's sealed write (`zikaron_seal::writer::take`); this module keeps the instance's mode, the
//! words for who holds it, and the turn the desktop takes per write while the command line writes alone
//! ([`Lock::release`], [`Lock::regain`]).
//!
//! A kernel lock only works on one machine: a data folder on a synced or shared disk opened on two machines
//! would get a writer on each, and the two ledgers would diverge. So each home keeps a writer mark
//! ([`MARK_FILE`]): one line naming the machine that writes it (this machine's mark, [`this_machine`], lives
//! in the machine directory, outside every data folder). Opening a home whose mark names another machine
//! makes this instance a reader ([`Mode::OtherMachine`]), until the user explicitly chooses "write from this
//! machine" ([`Lock::take_over`]), which rewrites the mark. There is no heartbeat or expiry, since both depend
//! on the wall clock and sync delays, which cannot be relied on.

use crate::fault::Fault;
use crate::home::{Home, Slot};
use std::path::PathBuf;
use zikaron_seal::writer::{self, Took};

/// The lock file's name.
pub const LOCK_FILE: &str = zikaron_seal::envelope::WRITER_LOCK;

/// The writer mark's file name (in a home's `settings/` directory): which machine writes this home.
pub const MARK_FILE: &str = writer::MARK_FILE;

/// The file name of this machine's mark (in the machine directory, outside every data folder).
pub const MACHINE_FILE: &str = writer::MACHINE_FILE;

/// This instance's access mode for a home.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Mode {
    /// Writer: this instance writes the home.
    Writer,
    /// Reader: another instance on this machine holds the lock.
    Reader,
    /// Reader: the home's writer mark names another machine, or cannot be read by this version (a read error,
    /// a wrong shape, or a later version's format: [`Mark::Unread`], reported via [`Lock::holder`] and
    /// [`Lock::mark_trouble`]; the mark is never rewritten). This instance holds the kernel lock, and writes
    /// only after the user takes the home over ([`Lock::take_over`]).
    OtherMachine,
}

impl Mode {
    pub fn as_str(self) -> &'static str {
        match self {
            Mode::Writer => "writer",
            Mode::Reader => "reader",
            Mode::OtherMachine => "other-machine",
        }
    }

    pub fn writable(self) -> bool {
        matches!(self, Mode::Writer)
    }
}

/// A lock on a home, released when this value is dropped or the process exits. A writer may let the kernel lock
/// go between writes while the command line writes alone ([`Lock::release`]) and take it again for the next
/// write ([`Lock::regain`]); its mode stays.
pub struct Lock {
    file: Option<zikaron_os::LockFile>,
    mode: Mode,
    path: PathBuf,
    holder: String,
    /// Why the home's writer mark could not be read ([`Mark::Unread`]), when that made this lock a reader.
    trouble: Option<Fault>,
}

impl Lock {
    pub fn mode(&self) -> Mode {
        self.mode
    }

    pub fn path(&self) -> &std::path::Path {
        &self.path
    }

    /// "Who holds it", for display only; never used in the decision.
    pub fn holder(&self) -> &str {
        &self.holder
    }

    /// Why the home's writer mark could not be read, when this lock is a reader for that ([`Mark::Unread`]).
    pub fn mark_trouble(&self) -> Option<&Fault> {
        self.trouble.as_ref()
    }

    /// Whether the kernel lock is in hand now.
    pub fn held(&self) -> bool {
        self.file.is_some()
    }

    /// Let the kernel lock go (a writer between writes while the command line writes alone); the mode stays.
    pub fn release(&mut self) {
        self.file = None;
    }

    /// Take the kernel lock again for a write: true when in hand (already, or now); false when another holder
    /// on this machine has it. A home whose mark now names another machine (or no longer reads) turns this lock
    /// into that reader, and is false.
    pub fn regain(&mut self, home: &Home) -> Result<bool, Fault> {
        if self.file.is_some() {
            return Ok(true);
        }
        let t = writer::take(home.root(), &crate::home::machine_dir()?)?;
        match t.took {
            Took::Writer => {
                self.file = Some(t.held);
                Ok(true)
            }
            Took::Busy { .. } => Ok(false),
            Took::OtherMachine => {
                self.file = Some(t.held);
                self.mode = Mode::OtherMachine;
                self.holder = crate::lang::filln(crate::lang::Key::TailOtherMachine, &[&mark_path(home).display().to_string()]);
                Ok(false)
            }
            Took::Unread(tr) => {
                let f = Fault::from(tr);
                self.file = Some(t.held);
                self.mode = Mode::OtherMachine;
                self.holder = f.evidence();
                self.trouble = Some(f);
                Ok(false)
            }
        }
    }

    /// The user's "write from this machine": rewrite the home's writer mark to this machine and become a writer.
    /// Only applies to [`Mode::OtherMachine`]; a reader blocked by another local instance cannot take over,
    /// since the kernel lock decides that.
    pub fn take_over(&mut self, home: &Home) -> Result<(), Fault> {
        if self.mode != Mode::OtherMachine {
            return Ok(());
        }
        write_mark(home)?;
        self.mode = Mode::Writer;
        self.holder = format!("pid {}", std::process::id());
        self.trouble = None;
        Ok(())
    }
}

/// This machine's mark, kept in the machine directory and created on first use (`zikaron_seal::writer`).
pub fn this_machine() -> Result<String, Fault> {
    Ok(writer::this_machine(&crate::home::machine_dir()?)?)
}

/// Where a home's writer mark lives.
pub fn mark_path(home: &Home) -> PathBuf {
    writer::mark_path(home.root())
}

/// What a home's writer mark says. A missing mark is distinct from an unreadable one.
#[derive(Clone, Debug, PartialEq)]
pub enum Mark {
    /// No mark yet (a home from an older version, or never opened by a writer).
    None,
    /// The machine the mark names.
    Machine(String),
    /// A mark this version cannot read: a read error, or a file that is not a single mark line (which is also
    /// how a later version's format appears).
    Unread(Fault),
}

/// Read a home's writer mark.
pub fn mark_read(home: &Home) -> Mark {
    match writer::mark_read(home.root()) {
        writer::Mark::None => Mark::None,
        writer::Mark::Machine(m) => Mark::Machine(m),
        writer::Mark::Unread(t) => Mark::Unread(t.into()),
    }
}

/// The machine a home's writer mark names (`None`: no mark, or one this version cannot read).
pub fn mark_of(home: &Home) -> Option<String> {
    match mark_read(home) {
        Mark::Machine(m) => Some(m),
        _ => None,
    }
}

/// Mark a home as written by this machine (through the write gate, `home::put_at`).
pub fn write_mark(home: &Home) -> Result<(), Fault> {
    let me = this_machine()?;
    crate::home::put_at(&home.dir(Slot::Settings), MARK_FILE, format!("{me}\n").as_bytes())
}

/// Ask the kernel for an exclusive lock on this descriptor, without waiting. True when obtained.
///
/// The single-writer rule rests on this: a kernel lock is released when the process ends, so stale locks
/// cannot arise.
pub fn grab(file: &std::fs::File) -> bool {
    crate::platform::lock_now(file)
}

/// The same primitive, blocking until obtained (used for the key vault file's lock).
///
/// The home's writer lock does not wait: failing means another instance holds it, and this one must become a
/// reader and tell the user at once. The vault file's lock waits, so a passcode attempt queues instead of
/// failing as "wrong passcode" while another process changes the vault.
pub fn grab_waiting(file: &std::fs::File) -> bool {
    crate::platform::lock_wait(file)
}

/// Where a home's lock file lives. Used both to take the lock and to check whether a held lock is this home's.
pub fn path_of(home: &Home) -> PathBuf {
    writer::lock_path(home.root())
}

/// Whether the held lock is this home's writer lock.
///
/// `flock` is per open file descriptor: a second descriptor to the same lock file in the same process counts
/// as another holder, so re-entering one's own home (switching roles, entering a home after import) would make
/// this process a reader. The lock is therefore taken once per process and reused when re-entering the same
/// home. Paths are compared after resolving aliases such as `/tmp` and `/private/tmp`.
pub fn holds_writer(held: Option<&Lock>, home: &Home) -> bool {
    let Some(l) = held else { return false };
    if !l.mode().writable() {
        return false;
    }
    crate::home::same_place(l.path(), &path_of(home))
}

/// Take the lock (`zikaron_seal::writer::take`): obtained means writer (or [`Mode::OtherMachine`] per the
/// writer mark), not obtained means reader. Either way a `Lock` is returned.
pub fn take(home: &Home) -> Result<Lock, Fault> {
    take_as(home, false)
}

/// [`take`], knowing whether the command line writes alone (`alone`). Then the command line may hold the kernel
/// lock for a write of its own just as this instance opens the home: that is its turn, not another instance, so
/// this one is still the writer, with the lock to take at its own next write ([`Lock::regain`]); the writer mark
/// decides as ever.
pub fn take_as(home: &Home, alone: bool) -> Result<Lock, Fault> {
    let path = path_of(home);
    let t = writer::take(home.root(), &crate::home::machine_dir()?)?;
    Ok(match t.took {
        Took::Writer => Lock { file: Some(t.held), mode: Mode::Writer, path, holder: format!("pid {}", std::process::id()), trouble: None },
        Took::OtherMachine => {
            let holder = crate::lang::filln(crate::lang::Key::TailOtherMachine, &[&mark_path(home).display().to_string()]);
            Lock { file: Some(t.held), mode: Mode::OtherMachine, path, holder, trouble: None }
        }
        // An unreadable mark is no license to write: stay read-only, report why, and leave the mark alone (a
        // later version or another machine may be writing).
        Took::Unread(tr) => {
            let f = Fault::from(tr);
            Lock { file: Some(t.held), mode: Mode::OtherMachine, path, holder: f.evidence(), trouble: Some(f) }
        }
        Took::Busy { .. } if alone => {
            let me = this_machine()?;
            match mark_read(home) {
                Mark::Machine(m) if m == me => Lock { file: None, mode: Mode::Writer, path, holder: format!("pid {}", std::process::id()), trouble: None },
                Mark::None => Lock { file: None, mode: Mode::Writer, path, holder: format!("pid {}", std::process::id()), trouble: None },
                Mark::Machine(_) => {
                    let holder = crate::lang::filln(crate::lang::Key::TailOtherMachine, &[&mark_path(home).display().to_string()]);
                    Lock { file: None, mode: Mode::OtherMachine, path, holder, trouble: None }
                }
                Mark::Unread(f) => Lock { file: None, mode: Mode::OtherMachine, path, holder: f.evidence(), trouble: Some(f) },
            }
        }
        Took::Busy { holder } => {
            let holder = if holder.trim().is_empty() { crate::lang::t(crate::lang::Key::Tail184).to_string() } else { holder };
            Lock { file: Some(t.held), mode: Mode::Reader, path, holder, trouble: None }
        }
    })
}
