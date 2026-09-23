//! Puts the Ubuntu side of Ferry on the USB, next to `Backup.enc`: the
//! `ferry-restore` program (bundled into the app as a resource) and a
//! launcher so the user can start it by double-click instead of a terminal.
use crate::safety::validate_backup_dir;
use anyhow::{Context, Result};
use tauri::path::BaseDirectory;
use tauri::{AppHandle, Manager};

/// GNOME's Files app won't run a program on double-click, but it will run a
/// `.desktop` launcher once the user right-clicks it → "Allow Launching".
/// `%k` is the launcher's own path, so it finds `ferry-restore` beside it on
/// whatever mount point the USB gets. Backslashes are doubled twice over:
/// once for the desktop-file string, once for the Exec quoting rules.
const LAUNCHER: &str = "[Desktop Entry]\n\
Type=Application\n\
Name=Restore with Ferry\n\
Comment=Put your files, Wi-Fi, apps and settings back\n\
Exec=sh -c \"cd \\\\\"\\\\$(dirname \\\\\"\\\\$1\\\\\")\\\\\" && exec ./ferry-restore --gui\" sh %k\n\
Icon=system-software-install\n\
Terminal=false\n";

#[tauri::command]
pub async fn copy_restore_tool(app: AppHandle, usb_root: String) -> Result<(), String> {
    run(&app, &usb_root).map_err(|e| format!("{e:#}"))
}

fn run(app: &AppHandle, usb_root: &str) -> Result<()> {
    let usb = validate_backup_dir(usb_root)?;
    let tool = app
        .path()
        .resolve("ferry-restore", BaseDirectory::Resource)
        .context("Cannot locate the bundled ferry-restore")?;
    std::fs::copy(&tool, usb.join("ferry-restore"))
        .context("Cannot copy ferry-restore to the USB (this build may be missing it)")?;
    std::fs::write(usb.join("Restore with Ferry.desktop"), LAUNCHER)
        .context("Cannot write the restore launcher")
}

#[cfg(test)]
mod tests {
    #[test]
    fn launcher_exec_line_has_spec_escaping() {
        // What a .desktop parser must see before Exec unquoting.
        assert!(super::LAUNCHER.contains(
            r#"Exec=sh -c "cd \\"\\$(dirname \\"\\$1\\")\\" && exec ./ferry-restore --gui" sh %k"#
        ));
    }
}
