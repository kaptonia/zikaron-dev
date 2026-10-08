//! Reading grant codes and files typed or dropped by the user. Vault import and the check page both use this:
//! pasted text starting with `zikaron-grant:` is decoded by the kit crate; anything else is a path, and a file
//! whose content (ignoring ASCII whitespace) starts with that prefix is decoded too; otherwise the whole file
//! is returned as one entry's bytes (validated by the caller).
//!
//! Keeping one reader avoids two call sites disagreeing (e.g. on a payload file with a trailing newline).
//! Decoding is done only by the kit crate's `badge::decode` (kit law §6); errors carry its tokens
//! with segment number and inner token.

use crate::fault::{Fault, Known};
use std::path::Path;

/// The form of the input.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Form {
    /// Pasted payload text (the same form as a scanned code).
    Payload,
    /// A file containing a payload.
    PayloadFile,
    /// A file containing an entry's bytes.
    EntryFile,
    /// A grant file (single-file container, `grantfilex`): opened and verified, with the chain taken from the
    /// grant code inside.
    GrantFile,
}

impl Form {
    pub fn as_str(self) -> &'static str {
        match self {
            Form::Payload => "payload",
            Form::PayloadFile => "payload_file",
            Form::EntryFile => "entry_file",
            Form::GrantFile => "grant_file",
        }
    }
}

/// Decode a payload with the kit crate; errors carry its token, segment number and inner token.
pub fn decode(payload: &[u8]) -> Result<Vec<Vec<u8>>, Fault> {
    match zikaron_kit::badge::decode(payload) {
        Ok(entries) => Ok(entries.into_iter().map(|e| e.bytes).collect()),
        Err(r) => Err(refused(r)),
    }
}

/// The kit crate's rejection of a grant code as a `PAYLOAD_REFUSED` error (token, segment and inner token).
pub fn refused(r: zikaron_kit::badge::DecodeReject) -> Fault {
    Fault::known(
        Known::PayloadRefused,
        format!(
            "{}{}{}",
            r.token.as_str(),
            r.index.map(|i| crate::lang::filln(crate::lang::Key::Tail087, &[&(i).to_string()])).unwrap_or_default(),
            r.inner.map(|t| format!(" {t:?}")).unwrap_or_default()
        ),
    )
}

/// Read the input. Returns the form and one or more byte strings: one per payload segment, the whole file for
/// an entry file, or the chain inside a grant file. Empty input is an error.
pub fn take(typed: &str) -> Result<(Form, Vec<Vec<u8>>), Fault> {
    let t = take_full(typed)?;
    Ok((t.form, t.hops))
}

/// One input read, with the opened container when it is a grant file.
pub struct Taken {
    pub form: Form,
    pub hops: Vec<Vec<u8>>,
    pub file: Option<(std::path::PathBuf, crate::grantfilex::Opened)>,
}

/// Whether bytes look like a grant code: they start with its prefix, in any letter case. A code with a
/// wrongly cased prefix is still treated as a code, so the kit crate reports `E_BADGE_PREFIX` instead of it
/// being read as a path or an entry.
fn written_as_code(b: &[u8]) -> bool {
    let p = zikaron_kit::tokens::BADGE_PREFIX.as_bytes();
    b.len() >= p.len() && b[..p.len()].eq_ignore_ascii_case(p)
}

/// Read the input in full: pasted code, payload file, grant file, or entry file.
pub fn take_full(typed: &str) -> Result<Taken, Fault> {
    let t = typed.trim();
    if t.is_empty() {
        return Err(Fault::known(Known::FieldMissing, crate::lang::t(crate::lang::Key::Tail199).to_string()));
    }
    if written_as_code(t.as_bytes()) {
        return Ok(Taken { form: Form::Payload, hops: decode(t.as_bytes())?, file: None });
    }
    let p = Path::new(t);
    if crate::grantfilex::is_grant_file(p) {
        let o = crate::grantfilex::open(p)?;
        return Ok(Taken { form: Form::GrantFile, hops: o.hops.clone(), file: Some((p.to_path_buf(), o)) });
    }
    let bytes = std::fs::read(p).map_err(|e| crate::fault::classify(&e, &p.display().to_string()))?;
    // A payload in a file: ignore ASCII whitespace (trailing newline, indentation) when checking the prefix
    // and decoding; entry bytes are kept unchanged.
    let trimmed: Vec<u8> = bytes.iter().copied().filter(|c| !c.is_ascii_whitespace()).collect();
    if written_as_code(&trimmed) {
        return Ok(Taken { form: Form::PayloadFile, hops: decode(&trimmed)?, file: None });
    }
    Ok(Taken { form: Form::EntryFile, hops: vec![bytes], file: None })
}
