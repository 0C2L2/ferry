//! `ferry-restore` — the Linux side of a Windows → Ubuntu migration.
//!
//! The Ferry GUI is Windows-only (diskpart, registry, netsh, winget), so once
//! the user has installed Ubuntu it cannot run. This program ships on the USB
//! next to `Backup.enc`: it decrypts the backup, puts the files on the desktop,
//! then brings back apps, browser data, Wi-Fi and the desktop's look.
//!
//! It reuses `ferry_lib::crypto` and `ferry_lib::restore` rather than
//! reimplementing them. Two parsers of one encryption format is how a silent
//! corruption bug ships, and verify-then-copy is the promise the product rests on.
//!
//! Usage:
//!   double-click "Restore with Ferry" on the USB   # windows via zenity
//!   ferry-restore                                   # terminal, finds the backup
//!   ferry-restore /media/you/FERRY_DATA
//!   ferry-restore --install-apps apt:vlc …          # internal: the root helper

mod extras;
mod ui;

use anyhow::{bail, Context, Result};
use std::path::{Path, PathBuf};
use ui::Ui;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.first().map(String::as_str) == Some("--install-apps") {
        std::process::exit(extras::install_as_root(&args[1..]));
    }
    let ui = Ui::detect(&args);
    if let Err(e) = run(&ui, &args) {
        ui.error(&format!("Restore failed: {e:#}"));
        std::process::exit(1);
    }
}

fn run(ui: &Ui, args: &[String]) -> Result<()> {
    ui.say("Ferry restore\n");

    let usb = match args.iter().find(|a| !a.starts_with("--")) {
        Some(arg) => PathBuf::from(arg),
        None => match find_backup() {
            Ok(found) => found,
            Err(e) => ui.pick_folder().ok_or(e).context(
                "Could not find Backup.enc automatically. \
                 Pass the path to the Ferry USB, e.g. `ferry-restore /media/you/FERRY_DATA`",
            )?,
        },
    };
    if !usb.join("Backup.enc").is_file() || !usb.join("backup.salt").is_file() {
        bail!("{} does not look like a Ferry USB (needs Backup.enc and backup.salt)", usb.display());
    }
    ui.say(&format!("Found backup on {}", usb.display()));

    let password = ui.password()?;
    let staging = {
        let _busy = ui.progress("Decrypting your backup…", true);
        // Empty staging path = use a fresh temp directory.
        ferry_lib::crypto::decrypt::run_decrypt(usb, &password, PathBuf::new())
            .context("Decryption failed — wrong password, or the backup is damaged")?
    };

    let summary = {
        let progress = ui.progress("Restoring your files…", false);
        ferry_lib::restore::copy::restore_to_desktop(staging.clone(), &|done, total, item| {
            progress.set(done * 100 / total.max(1), item);
        })
        .context("Restore failed")?
    };
    let restored = ferry_lib::restore::copy::get_desktop()?.join("Restored");

    let mut report = vec![format!(
        "{} files are back in the Restored folder on your desktop.",
        summary.restored
    )];
    if !summary.failed.is_empty() {
        report.push(format!("{} files could not be restored:", summary.failed.len()));
        report.extend(summary.failed.iter().take(10).map(|f| format!("  ! {f}")));
    }
    report.extend(extras::apps(ui, &staging));
    report.extend(extras::browsers(ui, &staging, &restored));
    report.extend(extras::wifi(&staging.join("WiFi")));
    report.extend(extras::look(&staging));

    ui.done(&report, &restored);
    Ok(())
}

/// Look for a Ferry backup on the usual Linux mount points, plus next to the
/// binary itself — the common case is the user running it straight off the USB.
fn find_backup() -> Result<PathBuf> {
    let mut candidates: Vec<PathBuf> = Vec::new();

    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            candidates.push(dir.to_path_buf());
        }
    }
    for root in ["/media", "/mnt", "/run/media"] {
        collect_mounts(Path::new(root), &mut candidates);
    }

    candidates
        .into_iter()
        .find(|dir| dir.join("Backup.enc").is_file() && dir.join("backup.salt").is_file())
        .context("no Ferry backup found on any mounted drive")
}

/// Mounts nest one or two deep (`/media/<user>/<label>`), so walk both levels
/// rather than assuming one layout.
fn collect_mounts(root: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(root) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        if let Ok(inner) = std::fs::read_dir(&path) {
            out.extend(inner.flatten().map(|e| e.path()).filter(|p| p.is_dir()));
        }
        out.push(path);
    }
}
