//! Apps `ferry-restore` can install unattended (`Backup/linux-install.json`).
//!
//! Only the curated table's plain `sudo apt install …` / `sudo snap install …`
//! hints qualify. Anything needing a vendor download or an extra repository
//! stays a manual step in `linux-apps.md` — Ferry never adds package sources.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Manager {
    Apt,
    Snap,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InstallItem {
    /// The name it had on Windows.
    pub app: String,
    pub linux_name: String,
    pub manager: Manager,
    pub packages: Vec<String>,
    /// snap `--classic` confinement (VS Code needs it).
    #[serde(default)]
    pub classic: bool,
    /// A different program doing a similar job — offered, never pre-ticked.
    #[serde(default)]
    pub alternative: bool,
}

/// Parse a curated install hint into (manager, packages, classic).
pub fn parse_hint(hint: &str) -> Option<(Manager, Vec<String>, bool)> {
    let words: Vec<&str> = hint.split_whitespace().collect();
    match words.as_slice() {
        ["sudo", "apt", "install", pkgs @ ..] if !pkgs.is_empty() => pkgs
            .iter()
            .all(|p| valid_package(p))
            .then(|| (Manager::Apt, pkgs.iter().map(|p| p.to_string()).collect(), false)),
        ["sudo", "snap", "install", pkg] if valid_package(pkg) => {
            Some((Manager::Snap, vec![pkg.to_string()], false))
        }
        ["sudo", "snap", "install", pkg, "--classic"] if valid_package(pkg) => {
            Some((Manager::Snap, vec![pkg.to_string()], true))
        }
        _ => None,
    }
}

/// Debian/snap package-name charset. Checked on BOTH sides: the list is read
/// back out of a backup file and ends up as arguments to a root process, so
/// a tampered backup must not be able to smuggle in an option like `-o…`.
pub fn valid_package(name: &str) -> bool {
    name.len() <= 64
        && name.starts_with(|c: char| c.is_ascii_lowercase() || c.is_ascii_digit())
        && name
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || matches!(c, '-' | '+' | '.'))
}

/// Encode the chosen items for the root helper's argv: `apt:vlc`,
/// `snap:spotify`, `snap-classic:code` — one argument per package.
pub fn to_args(items: &[&InstallItem]) -> Vec<String> {
    let mut out = Vec::new();
    for item in items {
        let kind = match (item.manager, item.classic) {
            (Manager::Apt, _) => "apt",
            (Manager::Snap, false) => "snap",
            (Manager::Snap, true) => "snap-classic",
        };
        out.extend(item.packages.iter().map(|p| format!("{kind}:{p}")));
    }
    out
}

/// Inverse of `to_args`, run as root — rejects anything it didn't produce.
pub fn parse_arg(arg: &str) -> Option<(Manager, String, bool)> {
    let (kind, pkg) = arg.split_once(':')?;
    if !valid_package(pkg) {
        return None;
    }
    let (manager, classic) = match kind {
        "apt" => (Manager::Apt, false),
        "snap" => (Manager::Snap, false),
        "snap-classic" => (Manager::Snap, true),
        _ => return None,
    };
    Some((manager, pkg.to_string(), classic))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn curated_hints_parse() {
        assert_eq!(
            parse_hint("sudo apt install nodejs npm"),
            Some((Manager::Apt, vec!["nodejs".into(), "npm".into()], false))
        );
        assert_eq!(
            parse_hint("sudo snap install code --classic"),
            Some((Manager::Snap, vec!["code".into()], true))
        );
        assert_eq!(parse_hint("Download the .deb from zoom.us/download"), None);
        assert_eq!(parse_hint("Already installed on Ubuntu"), None);
    }

    #[test]
    fn nothing_but_package_names_reaches_root() {
        assert_eq!(parse_hint("sudo apt install vlc;reboot"), None);
        assert_eq!(parse_hint("sudo apt install -o=Foo vlc"), None);
        assert_eq!(parse_arg("apt:-oDPkg::Pre-Invoke=x"), None);
        assert_eq!(parse_arg("sh:vlc"), None);
        assert_eq!(parse_arg("apt:VLC"), None);
    }

    #[test]
    fn args_round_trip() {
        let item = InstallItem {
            app: "Visual Studio Code".into(),
            linux_name: "VS Code".into(),
            manager: Manager::Snap,
            packages: vec!["code".into()],
            classic: true,
            alternative: false,
        };
        let args = to_args(&[&item]);
        assert_eq!(args, ["snap-classic:code"]);
        assert_eq!(parse_arg(&args[0]), Some((Manager::Snap, "code".into(), true)));
    }
}
