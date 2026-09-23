//! Chromium `Bookmarks` (JSON) → the Netscape bookmarks HTML that every
//! browser's "Import bookmarks" accepts. Chrome, Edge and Brave all use this
//! JSON format, so one converter brings all three back on any Linux browser.

use anyhow::{Context, Result};
use serde_json::Value;

/// Returns the HTML and the number of bookmarks in it.
pub fn chromium_to_html(json: &str) -> Result<(String, usize)> {
    let doc: Value = serde_json::from_str(json).context("Bookmarks file is not valid JSON")?;
    let roots = doc["roots"].as_object().context("Bookmarks file has no roots")?;

    let mut out = String::from(
        "<!DOCTYPE NETSCAPE-Bookmark-file-1>\n\
         <META HTTP-EQUIV=\"Content-Type\" CONTENT=\"text/html; charset=UTF-8\">\n\
         <TITLE>Bookmarks</TITLE>\n<H1>Bookmarks</H1>\n<DL><p>\n",
    );
    let mut count = 0;
    for (key, node) in roots {
        if node["children"].as_array().is_some_and(|c| !c.is_empty()) {
            let toolbar = if key == "bookmark_bar" { " PERSONAL_TOOLBAR_FOLDER=\"true\"" } else { "" };
            out.push_str(&format!("<DT><H3{toolbar}>{}</H3>\n", escape(name_of(node))));
            write_children(node, &mut out, &mut count);
        }
    }
    out.push_str("</DL><p>\n");
    Ok((out, count))
}

fn write_children(folder: &Value, out: &mut String, count: &mut usize) {
    out.push_str("<DL><p>\n");
    for child in folder["children"].as_array().into_iter().flatten() {
        match child["type"].as_str() {
            Some("url") => {
                let url = child["url"].as_str().unwrap_or_default();
                out.push_str(&format!("<DT><A HREF=\"{}\">{}</A>\n", escape(url), escape(name_of(child))));
                *count += 1;
            }
            Some("folder") => {
                out.push_str(&format!("<DT><H3>{}</H3>\n", escape(name_of(child))));
                write_children(child, out, count);
            }
            _ => {}
        }
    }
    out.push_str("</DL><p>\n");
}

fn name_of(node: &Value) -> &str {
    node["name"].as_str().unwrap_or("Bookmarks")
}

fn escape(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;").replace('"', "&quot;")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nested_folders_and_escaping() {
        let json = r#"{"roots":{
            "bookmark_bar":{"name":"Bookmarks bar","type":"folder","children":[
                {"type":"url","name":"A & B","url":"https://a.example/?x=1&y=\"2\""},
                {"type":"folder","name":"Work","children":[
                    {"type":"url","name":"<Docs>","url":"https://docs.example"}]}]},
            "other":{"name":"Other","type":"folder","children":[]}}}"#;
        let (html, count) = chromium_to_html(json).unwrap();
        assert_eq!(count, 2);
        assert!(html.contains("<H3 PERSONAL_TOOLBAR_FOLDER=\"true\">Bookmarks bar</H3>"));
        assert!(html.contains("HREF=\"https://a.example/?x=1&amp;y=&quot;2&quot;\">A &amp; B</A>"));
        assert!(html.contains("<DT><H3>Work</H3>"));
        assert!(html.contains("&lt;Docs&gt;"));
        assert!(!html.contains("Other"), "empty roots are left out");
    }
}
