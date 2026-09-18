/// Derive an AES-256 key from a user password using Argon2id.
/// The salt is random and stored alongside Backup.enc so it can be
/// re-derived at restore time without storing the key anywhere.
use anyhow::Result;
use argon2::{Argon2, password_hash::SaltString};
use rand::rngs::OsRng;

pub const KEY_LEN: usize = 32; // 256-bit AES key

/// Generate a fresh random salt and derive a 32-byte AES key from `password`.
/// Returns (key_bytes, salt_base64) — the salt must be stored in the USB root
/// as `backup.salt` so the key can be re-derived at restore time.
pub fn derive_key(password: &str) -> Result<([u8; KEY_LEN], String)> {
    let salt = SaltString::generate(&mut OsRng);
    let key = stretch_password(password, salt.as_str())?;
    Ok((key, salt.to_string()))
}

/// Re-derive the key from a password and a previously stored salt string.
pub fn derive_key_with_salt(password: &str, salt_b64: &str) -> Result<[u8; KEY_LEN]> {
    stretch_password(password, salt_b64)
}

fn stretch_password(password: &str, salt: &str) -> Result<[u8; KEY_LEN]> {
    let argon2 = Argon2::default(); // Argon2id, m=19MB, t=2, p=1 (OWASP recommended)
    let salt_string = SaltString::from_b64(salt)
        .map_err(|e| anyhow::anyhow!("Invalid salt: {}", e))?;

    let mut key = [0u8; KEY_LEN];
    argon2
        .hash_password_into(password.as_bytes(), salt_string.as_str().as_bytes(), &mut key)
        .map_err(|e| anyhow::anyhow!("Argon2 key derivation failed: {}", e))?;

    Ok(key)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn same_password_and_salt_produces_same_key() {
        let (key1, salt) = derive_key("hunter2").unwrap();
        let key2 = derive_key_with_salt("hunter2", &salt).unwrap();
        assert_eq!(key1, key2);
    }

    #[test]
    fn different_password_produces_different_key() {
        let (_, salt) = derive_key("password1").unwrap();
        let k1 = derive_key_with_salt("password1", &salt).unwrap();
        let k2 = derive_key_with_salt("password2", &salt).unwrap();
        assert_ne!(k1, k2);
    }
}

