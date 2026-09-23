#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

// The Ferry GUI is Windows-only: it drives diskpart, the registry, netsh and
// winget. On other targets this binary exists only so the crate still builds —
// the Linux entry point is the separate `ferry-restore` CLI.
#[cfg(windows)]
fn main() {
    ferry_lib::run();
}

#[cfg(not(windows))]
fn main() {
    eprintln!("The Ferry app runs on Windows. To restore a backup here, use `ferry-restore`.");
    std::process::exit(1);
}
