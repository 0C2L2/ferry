// `profile` parses exported Wi-Fi profiles and is portable, so the Linux
// restore CLI can recreate networks. Everything that shells out to netsh is
// Windows-only.
pub mod profile;
#[cfg(windows)]
pub mod export;
#[cfg(windows)]
pub mod import;
#[cfg(windows)]
pub mod list;
