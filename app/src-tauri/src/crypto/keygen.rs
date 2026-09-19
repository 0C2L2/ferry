/// Derive an AES-256 key from a user password using Argon2id.
/// The salt is random and stored alongside Backup.enc so it can be
/// re-derived at restore time without storing the key anywhere.
use anyhow::{Context, Result};
use argon2::password_hash::SaltString;
use argon2::{Algorithm, Argon2, Params, Version};
use rand::rngs::OsRng;

pub const KEY_LEN: usize = 32; // 256-bit AES key

// OWASP recommended Argon2id parameters (2024):
// m = 19 456 KiB, t = 2, p = 1.
const ARGON2_M_COST: u32 = 19_456;
const ARGON2_T_COST: u32 = 2;
const ARGON2_P_COST: u32 = 1;

/// Generate a fresh random salt and derive a 32-byte AES key from `password`.
/// Returns (key_bytes, salt_string) — the salt must be stored as `backup.salt`
/// on the USB so the key can be re-derived at restore time.
pub fn derive_key(password: &str) -> Result<([u8; KEY_LEN], String)> {
    let salt = SaltString::generate(&mut OsRng);
    let key = stretch_password(password, salt.as_str())?;
    Ok((key, salt.to_string()))
}

/// Re-derive the key from a password and a previously stored salt string.
pub fn derive_key_with_salt(password: &str, salt_str: &str) -> Result<[u8; KEY_LEN]> {
    stretch_password(password, salt_str)
}

fn stretch_password(password: &str, salt_str: &str) -> Result<[u8; KEY_LEN]> {
    // Build Argon2id with explicit OWASP parameters rather than relying on Default.
    let params = Params::new(ARGON2_M_COST, ARGON2_T_COST, ARGON2_P_COST, Some(KEY_LEN))
        .context("Failed to build Argon2 parameters")?;
    let argon2 = Argon2::new(Algorithm::Argon2id, Version::V0x13, params);

    // SaltString::as_str() returns the base64-encoded salt WITHOUT the PHC prefix.
    // Argon2::hash_password_into expects raw bytes for the salt.
    let salt_string = SaltString::from_b64(salt_str)
        .map_err(|e| anyhow::anyhow!("Invalid salt encoding: {}", e))?;
    // Decode the base64 salt to raw bytes for hash_password_into.
    // Must use a let binding so the scratch buffer outlives the borrow.
    let mut scratch = vec![0u8; 64];
    let salt_bytes = salt_string
        .decode_b64(scratch.as_mut_slice())
        .map_err(|e| anyhow::anyhow!("Failed to decode salt: {}", e))?;

    let mut key = [0u8; KEY_LEN];
    argon2
        .hash_password_into(password.as_bytes(), salt_bytes, &mut key)
        .map_err(|e| anyhow::anyhow!("Argon2 key derivation failed: {}", e))?;

    Ok(key)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn same_password_and_salt_produces_same_key() {
        let (key1, salt) = derive_key("hunter2-test-pw!").unwrap();
        let key2 = derive_key_with_salt("hunter2-test-pw!", &salt).unwrap();
        assert_eq!(key1, key2);
    }

    #[test]
    fn different_password_produces_different_key() {
        let (_, salt) = derive_key("password1-test!!").unwrap();
        let k1 = derive_key_with_salt("password1-test!!", &salt).unwrap();
        let k2 = derive_key_with_salt("password2-test!!", &salt).unwrap();
        assert_ne!(k1, k2);
    }

    #[test]
    fn key_is_correct_length() {
        let (key, _) = derive_key("lengthcheck-pass-123").unwrap();
        assert_eq!(key.len(), KEY_LEN);
    }
}
