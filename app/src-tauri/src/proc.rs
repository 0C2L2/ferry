//! Every external program Ferry runs (PowerShell, diskpart, netsh, winget)
//! starts here, never via `Command::new` directly.
//!
//! The release build is a GUI-subsystem app with no console, so Windows gives
//! each console child its own new window — a black PowerShell/cmd box flashing
//! up before every step. CREATE_NO_WINDOW runs them invisibly; output is still
//! captured through the pipes as before.
use std::os::windows::process::CommandExt;

const CREATE_NO_WINDOW: u32 = 0x0800_0000;

pub fn hidden(program: &str) -> std::process::Command {
    let mut cmd = std::process::Command::new(program);
    cmd.creation_flags(CREATE_NO_WINDOW);
    cmd
}

pub fn hidden_async(program: &str) -> tokio::process::Command {
    let mut cmd = tokio::process::Command::new(program);
    cmd.creation_flags(CREATE_NO_WINDOW);
    cmd
}
