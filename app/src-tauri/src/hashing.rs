//! SHA-256 file hashing, shared by the Windows app and the Linux restore CLI.
//!
//! This lives outside `backup/checksum.rs` because that module pulls in Tauri,
//! which the Linux binary cannot build. The hash itself is pure `std` + `sha2`,
//! and both sides must compute it identically — a backup verified on Windows
//! has to verify byte-for-byte on Ubuntu, or the manifest is worthless.

use anyhow::{Context, Result};
use sha2::{Digest, Sha256};
use std::io::Read;
use std::path::Path;

pub fn hash_file(path: &Path) -> Result<String> {
    let mut file =
        std::fs::File::open(path).with_context(|| format!("Cannot open for hashing: {path:?}"))?;
    let mut hasher = Sha256::new();
    let mut buf = vec![0u8; 1024 * 1024];
    loop {
        let n = file.read(&mut buf)?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
    }
    Ok(hex::encode(hasher.finalize()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn hash_is_deterministic_and_content_sensitive() {
        let mut f = tempfile::NamedTempFile::new().unwrap();
        f.write_all(b"ferry test content").unwrap();
        let a = hash_file(f.path()).unwrap();
        assert_eq!(a, hash_file(f.path()).unwrap());

        let mut g = tempfile::NamedTempFile::new().unwrap();
        g.write_all(b"ferry test contenu").unwrap();
        assert_ne!(a, hash_file(g.path()).unwrap());
    }
}
