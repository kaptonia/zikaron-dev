//! Writing a home ZIKARON Desk keeps sealed, with the command line's credential: the desktop's switch "the
//! command line writes alone" issued it (`zikaron_seal::credential`), and it is the one thing this path is
//! given. No passcode, no `--key`: the signing key is the one the desktop keeps for the home's identity and seat.
//!
//! One implementation with the desktop, the sealed layer's (`zikaron-seal`): the credential opens to the master
//! key, which must be this machine's vault's; the home's writer lock is taken for this one write (the desktop
//! holding it is refused by name, `E_HOME_HELD`); the home's label says whose it is, the identity register which
//! key signs, the vault gives that key; the entry is written into the sealed ledger by the sealed layer's
//! `Ledger::append` after the same write gate as every other entry, and queued for anchoring in the home's sealed
//! queue (`zikaron_seal::queue`). The command line's own `anchor` and contract calls never come here.
//!
//! A home is sealed when the ledger's home has a label (`envelope::LABEL_FILE` in its settings room).

use crate::codes::{Key, Reason};
use crate::out::{self, s, Answer, Said};
use std::path::{Path, PathBuf};
use zikaron_seal::envelope::{self as env, Doc, Expect, Ident, Owner};
use zikaron_seal::keybox::{self, LocalKey, Vault};
use zikaron_seal::names::NameKey;
use zikaron_seal::{Code, Trouble};

/// Whether `ledger` (as given to `--ledger`) is the ledger of a home the desktop keeps sealed.
pub fn is_home(ledger: &str) -> bool {
    Path::new(ledger).parent().is_some_and(|h| env::label_path(h).is_file())
}

/// A sealed home opened for one write: its ledger and what reads and writes it, the signing key and its address,
/// the entries as they open, and the home's writer lock (held until this is dropped).
pub struct Opened {
    home: PathBuf,
    ledger: zikaron_seal::ledger::Ledger,
    local: LocalKey,
    names: NameKey,
    number: String,
    pub key: [u8; 32],
    pub author: String,
    pub items: Vec<Vec<u8>>,
    _lock: zikaron_os::LockFile,
}

/// The sealed layer's refusal as this command line's answer: the credential's three by their own reasons, the
/// desktop's lock by its own, anything else unreadable with the code and its evidence.
pub fn refused(t: &Trouble) -> Answer {
    let members = vec![(Key::Token, s(&word(t.code))), (Key::Detail, s(&t.tail))];
    match t.code {
        Code::CredentialExpired => out::denied(Reason::CredentialExpired, members).telling(Said::CredentialExpired.text().to_string()),
        Code::CredentialMissing => out::denied(Reason::CredentialMissing, members).telling(Said::CredentialMissing.text().to_string()),
        Code::CredentialShape => out::denied(Reason::CredentialShape, members).telling(Said::CredentialShape.text().to_string()),
        Code::HomeHeld => out::denied(Reason::HomeHeld, members).telling(Said::HomeHeld.text().to_string()),
        Code::HomeOtherMachine => out::denied(Reason::HomeOtherMachine, members).telling(Said::HomeOtherMachine.text().to_string()),
        _ => out::denied(Reason::Unreadable, members),
    }
}

/// The sealed layer's code as one word in capitals (`CredentialExpired` → `CREDENTIAL_EXPIRED`), the form the
/// desktop's codes are spelled in.
fn word(c: Code) -> String {
    let mut out = String::new();
    for (i, ch) in format!("{c:?}").chars().enumerate() {
        if ch.is_ascii_uppercase() && i > 0 {
            out.push('_');
        }
        out.push(ch.to_ascii_uppercase());
    }
    out
}

/// Open the sealed home whose ledger is `ledger` with this machine's credential, at `now` (seconds since the
/// epoch).
pub fn open(ledger: &str, now: u64) -> Result<Opened, Trouble> {
    crate::seam();
    let ledger_dir = PathBuf::from(ledger);
    let home = ledger_dir.parent().map(Path::to_path_buf).unwrap_or_default();
    let machine = zikaron_os::machine::here().map(|(m, _)| m).map_err(|_| Trouble::new(Code::CredentialMissing, String::new()))?;
    // The credential: found by its name (which says the account base), opened at `now` (an expired one is
    // removed on the way out), and its master key checked against this machine's vault.
    let account = zikaron_seal::credential::find(&machine)?;
    let mk = zikaron_seal::credential::open(&machine, &account, now)?;
    let mk: [u8; 32] = mk.bytes().try_into().map_err(|_| Trouble::new(Code::CredentialShape, account.clone()))?;
    keybox::hold_master(&mk);
    let vault = Vault::new(machine.clone(), zikaron_seal::places::keybox_file_for(&account), zikaron_seal::places::keybox_lock_file_for(&account));
    if !keybox::master_is_this(&vault)? {
        return Err(Trouble::new(Code::CredentialShape, format!("{} master", zikaron_seal::credential::path(&machine, &account).display())));
    }
    // The home's writer lock, for this one write: the desktop holding it, or a home another machine writes, is
    // refused by name.
    let lock = take_lock(&home, &machine)?;
    let local = keybox::local_key()?;
    let names = keybox::name_key()?;
    let label = env::read_label(&home, &local)?.ok_or_else(|| Trouble::new(Code::LocalSeal, env::label_path(&home).display().to_string()))?;
    // Which key signs for this home: its identity's address on its seat, in the identity register (a home of
    // no identity's seat is signed for by the current identity and seat, as on the desktop).
    let register_name = zikaron_seal::places::registry_file_for(&account);
    let register_at = machine.join(&register_name);
    let raw = std::fs::read(&register_at).map_err(|e| Trouble::io(&e, &register_at.display().to_string()))?;
    let ex = Expect { owner: Some(Owner::Machine), doc: Doc::Registry, rel: Some(register_name.clone()) };
    let plain = env::open_with(&local, &ex, &raw, &register_name, Some(&names))?;
    let book = zikaron_seal::identities::Book::parse(&plain).map_err(|w| Trouble::new(Code::IdentitiesShape, w))?;
    let (identity, seat) = match (label.whose.clone(), book.current.clone()) {
        (Some(w), _) => w,
        (None, Some((id, seat))) => (id, seat.to_string()),
        (None, None) => return Err(Trouble::new(Code::IdentitiesShape, env::label_path(&home).display().to_string())),
    };
    let line = book.lines.iter().find(|l| l.id == identity).ok_or_else(|| Trouble::new(Code::IdentitiesShape, identity.clone()))?;
    let address = line.address(&seat).ok_or_else(|| Trouble::new(Code::IdentitiesShape, format!("{identity} {seat}")))?;
    let author = zikaron::hexfmt::encode(&address);
    let secret = keybox::get(&vault, &zikaron_seal::places::key_slot_for(&account, &author))?.ok_or_else(|| Trouble::new(Code::KeyboxSlot, author.clone()))?;
    let key: [u8; 32] = secret.as_slice().try_into().map_err(|_| Trouble::new(Code::KeyMalformed, author.clone()))?;
    let mut secret = secret;
    zikaron_os::wipe(&mut secret);
    if crate::entry::address(&key).as_deref() != Some(author.as_str()) {
        return Err(Trouble::new(Code::KeyMalformed, author));
    }
    let ledger = zikaron_seal::ledger::Ledger::open(&ledger_dir)?;
    let items = ledger.pile(&keys(&local, &names, &label.number))?.items;
    Ok(Opened { home, ledger, local, names, number: label.number, key, author, items, _lock: lock })
}

fn keys<'a>(local: &'a LocalKey, names: &'a NameKey, number: &str) -> zikaron_seal::ledger::Keys<'a> {
    zikaron_seal::ledger::Keys { local, names, owner: Some(Owner::Home(number.to_string())) }
}

/// Take the home's writer lock now through the one way both ends take it (`zikaron_seal::writer::take`), or
/// refuse: the desktop (or another writer here) holds it; another machine's mark is on the home, or a mark this
/// version cannot read.
fn take_lock(home: &Path, machine: &Path) -> Result<zikaron_os::LockFile, Trouble> {
    let t = zikaron_seal::writer::take(home, machine)?;
    match t.took {
        zikaron_seal::writer::Took::Writer => Ok(t.held),
        zikaron_seal::writer::Took::Busy { .. } => Err(Trouble::new(Code::HomeHeld, home.display().to_string())),
        zikaron_seal::writer::Took::OtherMachine => Err(Trouble::new(Code::HomeOtherMachine, zikaron_seal::writer::mark_path(home).display().to_string())),
        zikaron_seal::writer::Took::Unread(u) => Err(Trouble::new(Code::HomeOtherMachine, u.tail)),
    }
}

impl Opened {
    /// The ledger directory, as answered.
    pub fn ledger_root(&self) -> String {
        self.ledger.root().to_string_lossy().into_owned()
    }

    /// Write one entry (its id and canonical bytes) into the sealed ledger, and, when it was written, queue it
    /// for anchoring at `now` as the desktop queues each entry it writes. True when written, false when the same
    /// entry was already there.
    pub fn append(&self, id: &str, bytes: &[u8], now: u64) -> Result<bool, Trouble> {
        let name = zikaron_store::layout::EntryName::parse(id.trim_start_matches("0x")).ok_or_else(|| Trouble::new(Code::Ledger, id.to_string()))?;
        let stored = self.ledger.append(&name, bytes, &keys(&self.local, &self.names, &self.number), &self.number)?;
        let written = matches!(stored, zikaron_store::ledger::Stored::Written);
        if written {
            self.queue(id, now)?;
        }
        Ok(written)
    }

    /// Queue `id` in the home's sealed queue: read, push, write back whole (the lock is held).
    fn queue(&self, id: &str, now: u64) -> Result<(), Trouble> {
        use zikaron_seal::queue::{Queue, FILE};
        let rel = format!("{}/{FILE}", env::SETTINGS_DIR);
        let dir = self.home.join(env::SETTINGS_DIR);
        let owner = Owner::Home(self.number.clone());
        let mut q = match std::fs::read(dir.join(FILE)) {
            Ok(raw) => {
                let ex = Expect { owner: Some(owner.clone()), doc: Doc::Queue, rel: Some(rel.clone()) };
                let plain = env::open_with(&self.local, &ex, &raw, &rel, Some(&self.names))?;
                Queue::parse(&plain).map_err(|w| Trouble::new(Code::QueueShape, format!("{rel}: {w:?}")))?
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Queue::default(),
            Err(e) => return Err(Trouble::io(&e, &dir.join(FILE).display().to_string())),
        };
        q.push(id, now);
        let plain = q.to_bytes();
        let sealed = env::seal_checked(&self.local, &Ident { owner, doc: Doc::Queue, logical: rel }, &plain)?;
        zikaron_seal::land::put_at(&dir, FILE, &sealed)
    }
}
