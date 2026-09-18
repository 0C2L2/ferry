/// Backend trust-boundary helpers for Ferry.
///
/// The web frontend is untrusted input for destructive filesystem operations.
/// These helpers validate drive letters, USB roots, relative paths, filenames,
/// and staging directories before any erase/copy/extract/download happens.
use anyhow::{bail, Context, Result};
use std::path::{Component, Path, PathBuf};
use windows::Win32::Storage::FileSystem::GetDriveTypeW;
use windows::Win32::System::WindowsProgramming::DRIVE_REMOVABLE;
use windows::core::PCWSTR;

/// Accept `E`, `E:`, `E:\` (any case) and return the uppercase drive letter.
pub fn validate_drive_letter(input: &str) -> Result<char> {
    let trimmed = input.trim().trim_end_matches(['\\', '/']);
    let letter_part = trimmed.trim_end_matches(':');
    let mut chars = letter_part.chars();
    match (chars.next(), chars.next()) {
        (Some(c), None) if c.is_ascii_alphabetic() => Ok(c.to_ascii_uppercase()),
        _ => bail!("Invalid drive letter: expected a single A-Z drive such as \"E:\""),
    }
}

/// Re-check at the OS level that a drive letter is currently removable.
pub fn ensure_removable_drive_letter(letter: char) -> Result<()> {
    let drive = format!("{}:\\", letter.to_ascii_uppercase());
    let drive_w: Vec<u16> = drive.encode_utf16().chain(std::iter::once(0)).collect();
    // Safety: drive_w is a valid null-terminated wide string.
    let drive_type = unsafe { GetDriveTypeW(PCWSTR(drive_w.as_ptr())) };
    if drive_type != DRIVE_REMOVABLE {
        bail!("Drive {} is not removable; refusing destructive operation", drive);
    }
    Ok(())
}

fn drive_letter_of_path(path: &Path) -> Result<char> {
    match path.components().next() {
        Some(Component::Prefix(prefix)) => match prefix.kind() {
            std::path::Prefix::Disk(byte) | std::path::Prefix::VerbatimDisk(byte) => {
                Ok((byte as char).to_ascii_uppercase())
            }
            _ => bail!("Path is not on a local drive: {}", path.display()),
        },
        _ => bail!("Path is not on a local drive: {}", path.display()),
    }
}

/// Canonicalize a USB root, confirm it exists, and confirm it is removable.
/// Returns the canonical path plus its drive letter.
pub fn validate_usb_root(input: &str) -> Result<(PathBuf, char)> {
    let canonical = PathBuf::from(input)
        .canonicalize()
        .with_context(|| format!("USB path does not exist: {input}"))?;
    let letter = drive_letter_of_path(&canonical)?;
    ensure_removable_drive_letter(letter)?;
    Ok((canonical, letter))
}

/// Ensure `path` is inside `root`. Both should already be canonicalized.
pub fn ensure_inside(root: &Path, path: &Path) -> Result<()> {
    if path.starts_with(root) {
        Ok(())
    } else {
        bail!(
            "Path escapes its allowed root: {} is not inside {}",
            path.display(),
            root.display()
        )
    }
}

/// Sanitize a renderer/manifest-supplied relative path.
/// Only normal path components are allowed: no drives, roots, `.`, or `..`.
pub fn sanitize_relative_path(input: &str) -> Result<PathBuf> {
    let normalized = input.trim().replace('\\', "/");
    if normalized.is_empty() || normalized.starts_with('/') {
        bail!("Invalid relative path");
    }
    if normalized.len() >= 2 && normalized.as_bytes()[1] == b':' {
        bail!("Invalid relative path");
    }

    let mut out = PathBuf::new();
    for part in normalized.split('/') {
        if part.is_empty() || part == "." || part == ".." {
            bail!("Invalid relative path");
        }
        out.push(part);
    }
    if out.as_os_str().is_empty() {
        bail!("Invalid relative path");
    }
    Ok(out)
}

/// Confirm a source file exists and is under the current user's profile.
/// Returns the canonical source path. Missing/changed files fail closed.
pub fn ensure_source_under_home(source: &Path) -> Result<PathBuf> {
    let home = std::env::var("USERPROFILE")
        .or_else(|_| std::env::var("HOME"))
        .unwrap_or_else(|_| "C:\\Users\\User".to_string());
    let home_canon = PathBuf::from(home)
        .canonicalize()
        .context("Cannot resolve user profile directory")?;
    let source_canon = source
        .canonicalize()
        .with_context(|| format!("Source file is missing: {}", source.display()))?;
    ensure_inside(&home_canon, &source_canon)?;
    Ok(source_canon)
}

/// Sanitize a download filename. Query strings must already be stripped.
pub fn sanitize_filename(input: &str) -> Result<String> {
    let name = input.trim();
    if name.is_empty() || name.len() > 180 {
        bail!("Invalid download filename");
    }
    if name == "." || name == ".." {
        bail!("Invalid download filename");
    }
    if name.contains(['/', '\\', ':', '<', '>', '"', '|', '?', '*']) {
        bail!("Invalid download filename");
    }
    Ok(name.to_string())
}

pub fn is_http_url(url: &str) -> bool {
    let lower = url.trim().to_lowercase();
    (lower.starts_with("http://") || lower.starts_with("https://"))
        && !url.trim().contains([' ', '\t', '\n', '\r'])
        && url.trim().len() <= 2048
}

/// Validate or create a staging directory, constrained to the OS temp dir.
pub fn validate_staging_dir(input: &str) -> Result<PathBuf> {
    let temp_canon = std::env::temp_dir()
        .canonicalize()
        .context("Cannot resolve temp directory")?;
    let staging = PathBuf::from(input);
    let canonical = if staging.exists() {
        staging.canonicalize().context("Cannot resolve staging directory")?
    } else {
        let parent = staging.parent().context("Invalid staging directory")?;
        let parent_canon = parent
            .canonicalize()
            .context("Staging parent directory does not exist")?;
        ensure_inside(&temp_canon, &parent_canon)?;
        std::fs::create_dir_all(&staging).context("Cannot create staging directory")?;
        staging.canonicalize().context("Cannot resolve staging directory")?
    };
    ensure_inside(&temp_canon, &canonical)?;
    Ok(canonical)
}

/// Validate a decrypted-backup directory: it must live in temp or on removable media.
pub fn validate_backup_dir(input: &str) -> Result<PathBuf> {
    let canonical = PathBuf::from(input)
        .canonicalize()
        .with_context(|| format!("Backup directory does not exist: {input}"))?;
    let temp_canon = std::env::temp_dir()
        .canonicalize()
        .context("Cannot resolve temp directory")?;
    if canonical.starts_with(&temp_canon) {
        return Ok(canonical);
    }
    let letter = drive_letter_of_path(&canonical)?;
    ensure_removable_drive_letter(letter)?;
    Ok(canonical)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn drive_letter_allowlists_single_letter() {
        assert_eq!(validate_drive_letter("e").unwrap(), 'E');
        assert_eq!(validate_drive_letter("E:").unwrap(), 'E');
        assert_eq!(validate_drive_letter("E:\\").unwrap(), 'E');
        assert!(validate_drive_letter("E:\\Windows").is_err());
        assert!(validate_drive_letter("E'; rm").is_err());
        assert!(validate_drive_letter("").is_err());
    }

    #[test]
    fn relative_paths_cannot_escape() {
        assert!(sanitize_relative_path("Users/John/file.txt").is_ok());
        assert!(sanitize_relative_path("..\\Windows\\x").is_err());
        assert!(sanitize_relative_path("C:/Windows/x").is_err());
        assert!(sanitize_relative_path("/absolute/x").is_err());
        assert!(sanitize_relative_path("").is_err());
    }

    #[test]
    fn filenames_are_constrained() {
        assert!(sanitize_filename("ubuntu-24.04.1-desktop-amd64.iso").is_ok());
        assert!(sanitize_filename("..").is_err());
        assert!(sanitize_filename("a/b.iso").is_err());
        assert!(sanitize_filename("x?.iso").is_err());
    }
}
