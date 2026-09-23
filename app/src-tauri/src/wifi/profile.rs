//! Portable reader for the Wi-Fi profiles netsh exports (`WiFi-<ssid>.xml`).
//!
//! Lives outside the Windows-only export/import modules so the Linux restore
//! CLI can read the same files and recreate the networks with NetworkManager.

use std::path::Path;

/// How a saved network authenticates, reduced to what can be recreated.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Security {
    Open,
    /// WPA/WPA2 personal — a shared passphrase.
    Psk,
    /// WPA3 personal (SAE) — also a shared passphrase.
    Sae,
    /// 802.1X / enterprise (`WPA2`, `WPA`, `WPA3ENT`…): a per-user login, not a
    /// shared key. netsh has no key to export, so it cannot be recreated —
    /// the user must sign in again. Carries the raw value for the message.
    Enterprise(String),
}

#[derive(Debug, Clone)]
pub struct WifiProfile {
    pub ssid: String,
    pub security: Security,
    /// Plaintext passphrase, present only for Psk/Sae exported with `key=clear`.
    pub key: Option<String>,
}

pub fn classify(authentication: &str) -> Security {
    match authentication.trim().to_lowercase().as_str() {
        "open" => Security::Open,
        "wpa2psk" | "wpapsk" => Security::Psk,
        "wpa3sae" => Security::Sae,
        other => Security::Enterprise(other.to_string()),
    }
}

pub fn parse(xml: &str) -> Option<WifiProfile> {
    let ssid = tag_contents(xml, "name")?;
    let security = classify(&tag_contents(xml, "authentication").unwrap_or_default());
    let key = tag_contents(xml, "keyMaterial").filter(|k| !k.is_empty());
    Some(WifiProfile { ssid, security, key })
}

/// Every parseable `*.xml` profile in `dir`, sorted by SSID. Unreadable or
/// malformed files are skipped rather than failing the whole restore.
pub fn read_dir(dir: &Path) -> Vec<WifiProfile> {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut out: Vec<WifiProfile> = entries
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.extension().and_then(|x| x.to_str()) == Some("xml"))
        .filter_map(|p| std::fs::read_to_string(p).ok())
        .filter_map(|xml| parse(&xml))
        .collect();
    out.sort_by_key(|p| p.ssid.to_lowercase());
    out
}

pub fn tag_contents(xml: &str, tag: &str) -> Option<String> {
    let open = format!("<{tag}>");
    let close = format!("</{tag}>");
    let start = xml.find(&open)? + open.len();
    let end = xml[start..].find(&close)?;
    Some(decode_entities(xml[start..start + end].trim()))
}

fn decode_entities(s: &str) -> String {
    s.replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&apos;", "'")
        .replace("&amp;", "&") // last, so "&amp;lt;" decodes to "&lt;", not "<"
}

#[cfg(test)]
mod tests {
    use super::*;

    const WPA2: &str = r#"<WLANProfile><name>Caf&amp; Bar</name><MSM><security><authEncryption>
        <authentication>WPA2PSK</authentication></authEncryption><sharedKey>
        <keyMaterial>hunter2</keyMaterial></sharedKey></security></MSM></WLANProfile>"#;

    #[test]
    fn personal_network_parses_with_key() {
        let p = parse(WPA2).unwrap();
        assert_eq!(p.ssid, "Caf& Bar");
        assert_eq!(p.security, Security::Psk);
        assert_eq!(p.key.as_deref(), Some("hunter2"));
    }

    #[test]
    fn security_types_seen_on_a_real_machine_classify_correctly() {
        // From a real export: a WPA3 home network and a WPA2-Enterprise
        // university network that has no exportable key.
        assert_eq!(classify("WPA3SAE"), Security::Sae);
        assert_eq!(classify("WPA2"), Security::Enterprise("wpa2".into()));
        assert_eq!(classify("open"), Security::Open);
    }

    #[test]
    fn entity_decoding_does_not_double_decode() {
        assert_eq!(decode_entities("a&amp;lt;b"), "a&lt;b");
    }
}
