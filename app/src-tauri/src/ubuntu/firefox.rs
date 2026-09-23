//! Which profile folder Firefox on Ubuntu will actually open.

use std::path::{Path, PathBuf};

/// `root` is the directory holding `profiles.ini`. Since Firefox 67 each
/// install has its own default (`Default=<path>` under an `[Install…]` section,
/// mirrored in `installs.ini`); it wins over the legacy `Default=1` flag.
pub fn default_profile(root: &Path) -> Option<PathBuf> {
    let profiles = std::fs::read_to_string(root.join("profiles.ini")).ok()?;
    let installs = std::fs::read_to_string(root.join("installs.ini")).unwrap_or_default();
    let (path, relative) = pick(&profiles, &installs)?;
    Some(if relative { root.join(path) } else { PathBuf::from(path) })
}

/// Returns (path, is_relative).
fn pick(profiles_ini: &str, installs_ini: &str) -> Option<(String, bool)> {
    let mut install_default = None;
    let mut flagged = None;
    let mut first = None;

    for text in [installs_ini, profiles_ini] {
        let mut section = String::new();
        let (mut path, mut relative, mut is_default): (Option<String>, bool, bool) = (None, true, false);
        // A trailing sentinel section flushes the last real one.
        for line in text.lines().map(str::trim).chain(["[end]"]) {
            if line.starts_with('[') {
                if section.starts_with("Profile") {
                    if let Some(p) = path.take() {
                        if is_default && flagged.is_none() {
                            flagged = Some((p.clone(), relative));
                        }
                        first.get_or_insert((p, relative));
                    }
                }
                section = line.trim_matches(['[', ']']).to_string();
                (path, relative, is_default) = (None, true, false);
                continue;
            }
            let Some((key, value)) = line.split_once('=') else { continue };
            match key {
                "Default" if !section.starts_with("Profile") && value != "1" => {
                    install_default.get_or_insert((value.to_string(), true));
                }
                "Default" => is_default = value == "1",
                "Path" => path = Some(value.to_string()),
                "IsRelative" => relative = value != "0",
                _ => {}
            }
        }
    }
    install_default.or(flagged).or(first)
}

#[cfg(test)]
mod tests {
    use super::pick;

    #[test]
    fn per_install_default_wins() {
        let profiles = "[Profile1]\nName=default\nIsRelative=1\nPath=abc.default\nDefault=1\n\n\
                        [Profile0]\nName=default-release\nIsRelative=1\nPath=xyz.default-release\n\n\
                        [General]\nStartWithLastProfile=1\n";
        let installs = "[4F96D1932A9F858E]\nDefault=xyz.default-release\nLocked=1\n";
        assert_eq!(pick(profiles, installs), Some(("xyz.default-release".into(), true)));
        // Without installs.ini the legacy Default=1 flag decides.
        assert_eq!(pick(profiles, ""), Some(("abc.default".into(), true)));
    }

    #[test]
    fn absolute_profile_paths_are_kept() {
        let profiles = "[Profile0]\nName=x\nIsRelative=0\nPath=/data/ff\n";
        assert_eq!(pick(profiles, ""), Some(("/data/ff".into(), false)));
    }
}
