// `detect` scans Windows drive letters for a backup; the Linux CLI does its own
// mount walk in src/restore_cli/main.rs. `copy` is shared by both.
pub mod copy;
#[cfg(windows)]
pub mod detect;
