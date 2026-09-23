//! Captures the look of the Windows desktop — wallpaper and dark mode — so
//! `ferry-restore` can make Ubuntu feel like the user's own machine.
use crate::ubuntu::settings::{PersonalSettings, DIR, FILE};
use anyhow::{Context, Result};
use std::path::{Path, PathBuf};
use winreg::enums::HKEY_CURRENT_USER;
use winreg::RegKey;

pub fn save(backup_dir: &Path) -> Result<()> {
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let dir = backup_dir.join(DIR);
    std::fs::create_dir_all(&dir).context("Cannot create Backup/Settings")?;

    let dark_mode = hkcu
        .open_subkey(r"Software\Microsoft\Windows\CurrentVersion\Themes\Personalize")
        .and_then(|k| k.get_value::<u32, _>("AppsUseLightTheme"))
        .ok()
        .map(|light| light == 0);

    let wallpaper = wallpaper_source(&hkcu).and_then(|src| {
        let ext = src
            .extension()
            .and_then(|e| e.to_str())
            .map(str::to_lowercase)
            .filter(|e| e.len() <= 5 && e.chars().all(|c| c.is_ascii_alphanumeric()))
            // TranscodedWallpaper has no extension; Windows writes it as JPEG.
            .unwrap_or_else(|| "jpg".into());
        let name = format!("wallpaper.{ext}");
        std::fs::copy(&src, dir.join(&name)).ok().map(|_| name)
    });

    let settings = PersonalSettings { dark_mode, wallpaper };
    std::fs::write(dir.join(FILE), serde_json::to_string_pretty(&settings)?)
        .context("Cannot write settings.json")
}

/// The user's own wallpaper, or None for a stock Windows one (those are
/// Microsoft's images, not the user's, and look odd on Ubuntu).
fn wallpaper_source(hkcu: &RegKey) -> Option<PathBuf> {
    let windir = std::env::var("WINDIR").unwrap_or_else(|_| r"C:\Windows".into()).to_lowercase();
    let set: String = hkcu.open_subkey(r"Control Panel\Desktop").ok()?.get_value("WallPaper").ok()?;
    if set.trim().is_empty() || set.to_lowercase().starts_with(&windir) {
        return None;
    }
    let original = PathBuf::from(&set);
    if original.is_file() {
        return Some(original);
    }
    // The original may have been moved or deleted; Windows keeps its own copy.
    let cached = PathBuf::from(std::env::var("APPDATA").ok()?)
        .join(r"Microsoft\Windows\Themes\TranscodedWallpaper");
    cached.is_file().then_some(cached)
}
