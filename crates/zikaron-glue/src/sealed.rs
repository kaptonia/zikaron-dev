//! Magic prefix of ZIKARON Desk's sealed local data: every local file the app keeps (ledger entries included)
//! starts with [`MAGIC_V2`], then its envelope (the app's `local` module seals and opens it; only the app holds
//! the key). Shared with the CLI so it can tell a sealed file from a plain one without knowing more.
//!
//! Only the envelope written is read; the first one an earlier version wrote ([`MAGIC`]) is not this version's
//! sealed data and reads as not sealed.

/// Prefix of the first-version sealed file an earlier version wrote: kept as a family literal only (no other
/// product may take it); never read, never written.
pub const MAGIC: &[u8] = b"zikaron-local/1\n";

/// Prefix of a second-version sealed file (the version read and written).
pub const MAGIC_V2: &[u8] = b"zikaron-local/2\n";

/// Whether bytes are a sealed local file of the envelope written.
pub fn is_sealed(bytes: &[u8]) -> bool {
    bytes.starts_with(MAGIC_V2)
}
