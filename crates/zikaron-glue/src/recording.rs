//! The recording convention: how a `history` entry's `content` and `mode` are filled for bytes (`zikaron/1`
//! §6.2 requires both and leaves their meaning to the writer). Shared by the app and the CLI's `history --file`,
//! so the same file gives the same two members either way.
//!
//! `mark` is always [`FAMILY`]; `toolchain` is the sha256 of that literal's UTF-8 bytes. The toolchain names the
//! convention (which bytes are hashed, by which algorithm), not the program that hashed them.

use zikaron::cryptox;

/// The family literal: every `content` written under it is the sha256 of some bytes (a file's, a manifest's,
/// a commit object's).
pub const FAMILY: &str = "bytes-sha256/1";

/// The mode's `toolchain`: the sha256 of [`FAMILY`]'s UTF-8 bytes.
pub fn toolchain() -> [u8; 32] {
    cryptox::sha256(FAMILY.as_bytes())
}

/// A file's `content` under this convention: the sha256 of its bytes.
pub fn content_of(bytes: &[u8]) -> [u8; 32] {
    cryptox::sha256(bytes)
}

/// One row of a directory's manifest under this convention: a regular file's path from the directory and the
/// sha256 of its bytes in hex (`zikaron::hexfmt`).
pub type DirRow = (String, String);

/// A path inside a directory as its manifest spells it: its names from the directory joined by this system's
/// separator, as walking the directory spells it ([`dir_rows`]). A directory recomputed from the names a record
/// package keeps ([`crate::extract`]) joins them here, so both give the same rows.
pub fn dir_path(names: &[&str]) -> String {
    names.join(std::path::MAIN_SEPARATOR_STR)
}

/// A directory's rows: one per regular file under `root`, descended in name order; anything else (a symbolic
/// link, a device file) is left out and counted, never followed. A directory or file that cannot be read stops
/// the walk with the system's error and where it happened. Signing a directory and taking a directory out of a
/// record package both walk it here.
pub fn dir_rows(root: &std::path::Path) -> Result<(Vec<DirRow>, usize), (std::io::Error, std::path::PathBuf)> {
    let mut rows = Vec::new();
    let mut skipped = 0usize;
    walk(root, root, &mut rows, &mut skipped)?;
    Ok((rows, skipped))
}

fn walk(root: &std::path::Path, at: &std::path::Path, out: &mut Vec<DirRow>, skipped: &mut usize) -> Result<(), (std::io::Error, std::path::PathBuf)> {
    let mut names: Vec<std::path::PathBuf> = std::fs::read_dir(at)
        .map_err(|e| (e, at.to_path_buf()))?
        .map(|e| e.map(|e| e.path()).map_err(|e| (e, at.to_path_buf())))
        .collect::<Result<_, _>>()?;
    names.sort();
    for p in names {
        let md = std::fs::symlink_metadata(&p).map_err(|e| (e, p.clone()))?;
        if md.is_dir() {
            walk(root, &p, out, skipped)?;
        } else if md.is_file() {
            let b = std::fs::read(&p).map_err(|e| (e, p.clone()))?;
            let rel = p.strip_prefix(root).unwrap_or(&p).display().to_string();
            out.push((rel, zikaron::hexfmt::encode(&content_of(&b))));
        } else {
            *skipped += 1;
        }
    }
    Ok(())
}

/// A directory's manifest: `{"files":[{"path","sha256"},…]}`, the rows sorted by path, as one canonical JSON value,
/// so the same tree gives the same value anywhere.
pub fn dir_manifest(rows: &[DirRow]) -> zikaron::json::Value {
    use zikaron::json::Value;
    let mut rows: Vec<&DirRow> = rows.iter().collect();
    rows.sort();
    let arr = rows
        .into_iter()
        .map(|(path, sha)| Value::Obj(vec![("path".to_string(), Value::Str(path.clone())), ("sha256".to_string(), Value::Str(sha.clone()))]))
        .collect();
    Value::Obj(vec![("files".to_string(), Value::Arr(arr))])
}

/// A directory's `content` under this convention: the sha256 of its manifest's canonical bytes, with those bytes.
pub fn dir_content(rows: &[DirRow]) -> ([u8; 32], Vec<u8>) {
    let bytes = zikaron::json::canon_bytes(&dir_manifest(rows));
    (content_of(&bytes), bytes)
}
