pub mod keygen;
pub mod stream;

// Decryption is shared - the Linux ferry-restore CLI needs it. Encryption
// only ever happens on the Windows side, during backup.
pub mod decrypt;
#[cfg(windows)]
pub mod encrypt;
