//! Everything after the files: apps, browsers, Wi-Fi, look-and-feel. Each step
//! returns lines for the final summary and never fails the restore — the
//! files are the promise, these are the "your computer is back" part.

use crate::ui::{has, Ui};
use ferry_lib::ubuntu::apps::{parse_arg, to_args, valid_package, InstallItem, Manager};
use ferry_lib::ubuntu::{bookmarks, firefox, settings};
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

fn home() -> PathBuf {
    PathBuf::from(std::env::var("HOME").unwrap_or_else(|_| ".".into()))
}

// ── Apps ─────────────────────────────────────────────────────────────────────

pub fn apps(ui: &Ui, staging: &Path) -> Vec<String> {
    let items: Vec<InstallItem> = std::fs::read_to_string(staging.join("linux-install.json"))
        .ok()
        .and_then(|t| serde_json::from_str::<Vec<InstallItem>>(&t).ok())
        .unwrap_or_default()
        .into_iter()
        .filter(|i| i.packages.iter().all(|p| valid_package(p)))
        .collect();
    if items.is_empty() {
        return Vec::new();
    }

    let rows: Vec<(bool, String)> = items
        .iter()
        .map(|i| {
            let label = if i.alternative {
                format!("{} — instead of {} (a different app)", i.linux_name, i.app)
            } else {
                format!("{} (you had {})", i.linux_name, i.app)
            };
            (!i.alternative, label)
        })
        .collect();
    let chosen = ui.checklist(
        "Ferry can install these apps for you. Untick anything you don't want.",
        &rows,
    );
    if chosen.is_empty() {
        return vec!["Apps: none installed — linux-apps.md in the Restored folder lists them all.".into()];
    }
    let picked: Vec<&InstallItem> = chosen.iter().filter_map(|&i| items.get(i)).collect();

    let Ok(exe) = std::env::current_exe() else {
        return vec!["Apps: could not start the installer.".into()];
    };
    // pkexec shows Ubuntu's own password window; sudo is the terminal fallback.
    let elevate = if has("pkexec") { "pkexec" } else { "sudo" };
    let progress = ui.progress("Installing apps — enter your Ubuntu password when asked…", true);
    let child = Command::new(elevate)
        .arg(exe)
        .arg("--install-apps")
        .args(to_args(&picked))
        .stdout(Stdio::piped())
        .spawn();
    let Ok(mut child) = child else {
        return vec!["Apps: could not ask for administrator rights.".into()];
    };

    let mut ok: Vec<String> = Vec::new();
    if let Some(out) = child.stdout.take() {
        for line in BufReader::new(out).lines().map_while(Result::ok) {
            if let Some(status) = line.strip_prefix("# ") {
                progress.text(status);
            } else if let Some(pkg) = line.strip_prefix("ok ") {
                ok.push(pkg.to_string());
            }
        }
    }
    let _ = child.wait();
    drop(progress);

    let (done, failed): (Vec<&InstallItem>, Vec<&InstallItem>) =
        picked.into_iter().partition(|i| i.packages.iter().all(|p| ok.contains(p)));
    let names = |v: &[&InstallItem]| v.iter().map(|i| i.linux_name.as_str()).collect::<Vec<_>>().join(", ");
    let mut lines = Vec::new();
    if !done.is_empty() {
        lines.push(format!("Apps installed: {}.", names(&done)));
    }
    if !failed.is_empty() {
        lines.push(format!(
            "Apps not installed: {} — try Ubuntu's App Center, or see linux-apps.md.",
            names(&failed)
        ));
    }
    lines
}

/// `ferry-restore --install-apps apt:vlc snap:spotify …`, run as root via
/// pkexec. Prints `# status` lines and one `ok|fail <package>` per package.
pub fn install_as_root(args: &[String]) -> i32 {
    let items: Vec<(Manager, String, bool)> = args.iter().filter_map(|a| parse_arg(a)).collect();
    if items.len() != args.len() {
        eprintln!("Refusing: not a package list Ferry produced.");
        return 2;
    }
    // Fresh installs often have unattended-upgrades holding the apt lock:
    // wait for it instead of failing.
    let apt = |extra: &[&str]| {
        let mut a = vec!["-o", "DPkg::Lock::Timeout=600"];
        a.extend_from_slice(extra);
        run("apt-get", &a)
    };
    if items.iter().any(|(m, _, _)| *m == Manager::Apt) {
        println!("# Updating the package list…");
        apt(&["update"]);
    }
    for (manager, pkg, classic) in &items {
        println!("# Installing {pkg}…");
        let ok = match manager {
            Manager::Apt => apt(&["install", "-y", pkg]),
            Manager::Snap if *classic => run("snap", &["install", pkg, "--classic"]),
            Manager::Snap => run("snap", &["install", pkg]),
        };
        println!("{} {pkg}", if ok { "ok" } else { "fail" });
    }
    0
}

fn run(program: &str, args: &[&str]) -> bool {
    Command::new(program)
        .args(args)
        .env("DEBIAN_FRONTEND", "noninteractive")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .is_ok_and(|s| s.success())
}

// ── Browsers ─────────────────────────────────────────────────────────────────

pub fn browsers(ui: &Ui, staging: &Path, restored: &Path) -> Vec<String> {
    let data = staging.join("BrowserData");
    let mut lines = Vec::new();

    // Chrome/Edge/Brave: bookmarks as an import file any browser accepts.
    // (Their saved passwords are locked to Windows and cannot move.)
    let out_dir = restored.join("Browser bookmarks");
    let mut made = Vec::new();
    for browser in ["Chrome", "Edge", "Brave"] {
        let Ok(profiles) = std::fs::read_dir(data.join(browser)) else { continue };
        for profile in profiles.flatten() {
            let Ok(json) = std::fs::read_to_string(profile.path().join("Bookmarks")) else { continue };
            let Ok((html, count)) = bookmarks::chromium_to_html(&json) else { continue };
            if count == 0 || std::fs::create_dir_all(&out_dir).is_err() {
                continue;
            }
            let name = format!("{browser} - {}.html", profile.file_name().to_string_lossy());
            if std::fs::write(out_dir.join(&name), html).is_ok() {
                made.push(format!("{browser} ({count})"));
            }
        }
    }
    if !made.is_empty() {
        lines.push(format!(
            "Bookmarks from {} are in Restored/Browser bookmarks — in your browser choose \
             Bookmarks → Import and pick the file.",
            made.join(", ")
        ));
    }

    lines.extend(restore_firefox(ui, &data.join("Firefox")));
    lines
}

/// Firefox profiles are cross-platform: bookmarks, history and (unless a
/// Primary Password was set) saved passwords move over as-is.
fn restore_firefox(ui: &Ui, backup: &Path) -> Vec<String> {
    // Several Windows profiles: take the one with the most history.
    let Some(src) = std::fs::read_dir(backup)
        .into_iter()
        .flatten()
        .flatten()
        .map(|e| e.path())
        .filter_map(|p| Some((std::fs::metadata(p.join("places.sqlite")).ok()?.len(), p)))
        .max_by_key(|(size, _)| *size)
        .map(|(_, p)| p)
    else {
        return Vec::new();
    };

    let roots = [
        home().join("snap/firefox/common/.mozilla/firefox"), // Ubuntu's snap Firefox
        home().join(".mozilla/firefox"),
    ];
    let find = || roots.iter().find_map(|r| firefox::default_profile(r)).filter(|p| p.is_dir());

    let mut target = find();
    if target.is_none()
        && has("firefox")
        && ui.ask(
            "Firefox needs to open once so Ferry can move your bookmarks, history and \
             passwords into it. Ferry will open it now — close Firefox to continue.",
        )
    {
        let _ = Command::new("firefox").status();
        target = find();
    }
    let Some(dst) = target else {
        return vec!["Firefox data was not moved (Firefox isn't set up yet).".into()];
    };

    while firefox_running() {
        if !ui.ask("Please close Firefox so Ferry can add your data. Closed it?") {
            return vec!["Firefox data was not moved (Firefox was open).".into()];
        }
    }
    for file in ["places.sqlite", "logins.json", "key4.db"] {
        let from = src.join(file);
        if !from.is_file() {
            continue;
        }
        let to = dst.join(file);
        if to.exists() {
            let _ = std::fs::rename(&to, dst.join(format!("{file}.before-ferry")));
        }
        // The new profile's journal belongs to the database being replaced.
        for suffix in ["-wal", "-shm"] {
            let _ = std::fs::remove_file(dst.join(format!("{file}{suffix}")));
        }
        if std::fs::copy(&from, &to).is_err() {
            return vec![format!("Firefox: could not copy {file}.")];
        }
    }
    vec!["Firefox: your bookmarks, history and saved passwords are back.".into()]
}

fn firefox_running() -> bool {
    Command::new("pgrep").args(["-x", "firefox"]).stdout(Stdio::null()).status().is_ok_and(|s| s.success())
}

// ── Wi-Fi ────────────────────────────────────────────────────────────────────

/// Recreate the saved Windows networks in NetworkManager.
pub fn wifi(wifi_dir: &Path) -> Vec<String> {
    use ferry_lib::wifi::profile::{read_dir, Security};

    let profiles = read_dir(wifi_dir);
    if profiles.is_empty() {
        return Vec::new();
    }
    let existing = match Command::new("nmcli").args(["-t", "-f", "NAME", "connection", "show"]).output() {
        Ok(out) if out.status.success() => String::from_utf8_lossy(&out.stdout).to_string(),
        _ => return vec!["Wi-Fi: NetworkManager is not available — passwords are in WiFi-Passwords.txt in the Restored folder.".into()],
    };
    let existing: std::collections::HashSet<&str> = existing.lines().collect();

    let (mut added, mut sign_in, mut failed) = (Vec::new(), Vec::new(), Vec::new());
    for p in profiles {
        if existing.contains(p.ssid.as_str()) {
            continue;
        }
        let key_mgmt = match (&p.security, &p.key) {
            (Security::Open, _) => None,
            (Security::Psk, Some(_)) => Some("wpa-psk"),
            (Security::Sae, Some(_)) => Some("sae"),
            _ => {
                sign_in.push(p.ssid);
                continue;
            }
        };
        let mut args: Vec<&str> = vec!["connection", "add", "type", "wifi", "con-name", &p.ssid, "ssid", &p.ssid];
        // ponytail: the passphrase is briefly visible in this process's argv to
        // other local users; switch to an nmcli keyfile import if that matters.
        if let (Some(mgmt), Some(key)) = (key_mgmt, p.key.as_deref()) {
            args.extend(["wifi-sec.key-mgmt", mgmt, "wifi-sec.psk", key]);
        }
        let ok = Command::new("nmcli").args(&args).output().is_ok_and(|o| o.status.success());
        if ok { &mut added } else { &mut failed }.push(p.ssid);
    }

    let mut lines = Vec::new();
    if !added.is_empty() {
        lines.push(format!("Wi-Fi networks saved: {}.", added.join(", ")));
    }
    if !sign_in.is_empty() {
        lines.push(format!("Sign in again (work/school login): {}.", sign_in.join(", ")));
    }
    if !failed.is_empty() {
        lines.push(format!("Wi-Fi not added: {} — passwords are in WiFi-Passwords.txt.", failed.join(", ")));
    }
    lines
}

// ── Look and feel ────────────────────────────────────────────────────────────

pub fn look(staging: &Path) -> Vec<String> {
    let dir = staging.join(settings::DIR);
    let Some(s) = std::fs::read_to_string(dir.join(settings::FILE))
        .ok()
        .and_then(|t| serde_json::from_str::<settings::PersonalSettings>(&t).ok())
    else {
        return Vec::new();
    };
    if !has("gsettings") {
        return Vec::new();
    }
    let gset = |schema: &str, key: &str, value: &str| {
        Command::new("gsettings").args(["set", schema, key, value]).status().is_ok_and(|s| s.success())
    };

    let mut applied = Vec::new();
    if let Some(dark) = s.dark_mode {
        let ok = gset("org.gnome.desktop.interface", "color-scheme", if dark { "prefer-dark" } else { "default" });
        // Older (GTK3) apps follow the theme name rather than color-scheme.
        gset("org.gnome.desktop.interface", "gtk-theme", if dark { "Yaru-dark" } else { "Yaru" });
        if ok {
            applied.push(if dark { "dark mode" } else { "light mode" });
        }
    }
    if let Some(name) = s.wallpaper.filter(|n| settings::valid_wallpaper_name(n)) {
        let dest_dir = home().join(".local/share/backgrounds");
        let dest = dest_dir.join(format!("ferry-{name}"));
        if std::fs::create_dir_all(&dest_dir).is_ok() && std::fs::copy(dir.join(&name), &dest).is_ok() {
            let uri = format!("file://{}", dest.display()).replace(' ', "%20");
            if gset("org.gnome.desktop.background", "picture-uri", &uri) {
                gset("org.gnome.desktop.background", "picture-uri-dark", &uri);
                applied.push("your wallpaper");
            }
        }
    }
    if applied.is_empty() {
        Vec::new()
    } else {
        vec![format!("Look and feel: {} applied.", applied.join(" and "))]
    }
}
