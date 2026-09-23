//! Personal look-and-feel carried to Ubuntu (`Backup/Settings/settings.json`).
//!
//! Only what the Ubuntu installer does NOT already ask for: language, keyboard
//! layout and time zone are set during installation, so copying them here
//! would just fight the user's fresh choices.

use serde::{Deserialize, Serialize};

pub const DIR: &str = "Settings";
pub const FILE: &str = "settings.json";

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct PersonalSettings {
    /// Windows "app mode" was dark.
    pub dark_mode: Option<bool>,
    /// File name of the copied wallpaper inside `Settings/`.
    pub wallpaper: Option<String>,
}

/// The wallpaper name comes back out of a backup file: only accept the plain
/// `wallpaper.<ext>` names the Windows side writes, never a path.
pub fn valid_wallpaper_name(name: &str) -> bool {
    name.strip_prefix("wallpaper.")
        .is_some_and(|ext| !ext.is_empty() && ext.len() <= 5 && ext.chars().all(|c| c.is_ascii_alphanumeric()))
}

#[cfg(test)]
mod tests {
    #[test]
    fn wallpaper_name_cannot_escape_the_folder() {
        assert!(super::valid_wallpaper_name("wallpaper.jpg"));
        assert!(!super::valid_wallpaper_name("wallpaper./../x"));
        assert!(!super::valid_wallpaper_name("../wallpaper.jpg"));
    }
}
