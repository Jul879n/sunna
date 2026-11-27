//! Explicit, person-reviewed feedback. No usage-data consent or install id.
//! It goes as a prefilled issue in Sunna's GitHub repository, opened in the browser: the person reads and confirms it there.
//! Screenshots are what the person pastes or picks, each shown before sending.
use serde_json::{Value, json};
use std::path::PathBuf;

pub const MAX_IMAGES: usize = 3;
const MAX_IMAGE_BYTES: usize = 5 * 1024 * 1024;

/// Where feedback goes: a new issue here.
const REPO: &str = "Jul879n/sunna";
/// The most percent-encoded message a link carries (browsers and GitHub cut long addresses).
const MAX_ENCODED: usize = 6000;

pub fn draft(message: &str, email: &str, images: usize, from: &str, layout: &str) -> Value {
    json!({
        "v": 1,
        "from": if from == "agent" { "agent" } else { "person" },
        "message": message.trim(),
        "email": if email.trim().is_empty() { None } else { Some(email.trim()) },
        "app": {
            "version": env!("CARGO_PKG_VERSION"),
            "os": std::env::consts::OS,
            "arch": std::env::consts::ARCH,
            "channel": "stable",
        },
        "core": { "version": crate::update::core().version.as_deref().map(crate::telemetry::version) },
        "layout": if crate::window::shipped_layouts().contains(&layout) { layout } else { "other" },
        "images": images,
    })
}

/// Where the form keeps the screenshots it shows (its data folder: `image()` reads from the app home).
fn shots_dir() -> PathBuf { crate::mods::app_dir().join("data").join("frame-feedback") }

fn sniff(bytes: &[u8]) -> Option<&'static str> {
    if bytes.starts_with(b"\x89PNG\r\n\x1a\n") { Some("png") }
    else if bytes.starts_with(&[0xff, 0xd8, 0xff]) { Some("jpg") }
    else { None }
}

/// Keeps a pasted or picked image for the form: PNG or JPEG as is, any other
/// image as PNG. Returns its path from the app home.
pub fn attach(bytes: &[u8]) -> Result<String, String> {
    let (bytes, ext) = match sniff(bytes) {
        Some(ext) => (bytes.to_vec(), ext),
        None => {
            let img = image::load_from_memory(bytes).map_err(|_| crate::i18n::t("rs.feedback.that_isn_t_an_image"))?;
            let mut png = std::io::Cursor::new(Vec::new());
            img.write_to(&mut png, image::ImageFormat::Png).map_err(|_| crate::i18n::t("rs.feedback.couldn_t_convert_that_image"))?;
            (png.into_inner(), "png")
        }
    };
    if bytes.len() > MAX_IMAGE_BYTES { return Err(crate::i18n::t("rs.feedback.that_image_is_over_5")); }
    let dir = shots_dir();
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let stamp = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_nanos()).unwrap_or_default();
    let name = format!("shot-{stamp}.{ext}");
    std::fs::write(dir.join(&name), bytes).map_err(|e| e.to_string())?;
    Ok(format!("data/frame-feedback/{name}"))
}

/// Picker and external drops share the same conversion and size limits.
pub fn attach_files(paths: &[PathBuf]) -> Vec<Value> {
    paths.iter().take(MAX_IMAGES).map(|p| {
        match std::fs::read(p) {
            Ok(bytes) => match attach(&bytes) {
                Ok(path) => json!({ "path": path }),
                Err(error) => json!({ "error": error }),
            },
            Err(_) => json!({ "error": crate::i18n::t("rs.feedback.couldn_t_read_that_file") }),
        }
    }).collect()
}

/// Drops the form's screenshots (a new form starts empty; a sent one is done with them).
pub fn clear_shots() {
    let Ok(entries) = std::fs::read_dir(shots_dir()) else { return };
    for e in entries.flatten() {
        if e.file_name().to_string_lossy().starts_with("shot-") { let _ = std::fs::remove_file(e.path()); }
    }
}

/// A path `attach` returned, checked again before it leaves the machine.
fn shot_file(rel: &str) -> Result<PathBuf, String> {
    let name = rel.strip_prefix("data/frame-feedback/").filter(|n| n.starts_with("shot-") && !n.contains(['/', '\\']))
        .ok_or_else(|| crate::i18n::t("rs.feedback.unknown_screenshot"))?;
    let path = shots_dir().join(name);
    let bytes = std::fs::read(&path).map_err(|_| "A screenshot is gone. Remove it and add it again.".to_owned())?;
    if bytes.len() > MAX_IMAGE_BYTES || sniff(&bytes).is_none() { return Err("A screenshot isn't a PNG or JPEG under 5 MB.".into()); }
    Ok(path)
}

pub fn available() -> bool { true }

fn validate(body: &Value) -> Result<(), String> {
    let message = body["message"].as_str().unwrap_or_default();
    if message.is_empty() { return Err(crate::i18n::t("rs.feedback.write_a_message_first")); }
    if message.encode_utf16().count() > 4000 { return Err(crate::i18n::t("rs.feedback.keep_the_message_to_4000")); }
    if let Some(email) = body["email"].as_str() {
        let valid = email.split_once('@').is_some_and(|(local, domain)| {
            !local.is_empty() && domain.split_once('.').is_some_and(|(a, b)| !a.is_empty() && !b.is_empty())
                && !domain.contains('@')
        });
        if email.len() > 254 || !valid || email.chars().any(char::is_whitespace) {
            return Err(crate::i18n::t("rs.feedback.enter_a_plain_email_address"));
        }
    }
    Ok(())
}

/// Percent-encodes `text` for a query value.
fn encode(text: &str) -> String {
    let mut out = String::with_capacity(text.len() * 3);
    for b in text.bytes() {
        if b.is_ascii_alphanumeric() || matches!(b, b'-' | b'.' | b'_' | b'~') { out.push(b as char) } else { out.push_str(&format!("%{b:02X}")) }
    }
    out
}

/// The text up to `max` encoded bytes, cut at a character.
fn fit(text: &str, max: usize) -> String {
    let mut used = 0;
    let mut end = 0;
    for (i, c) in text.char_indices() {
        let mut buf = [0u8; 4];
        let n: usize = c.encode_utf8(&mut buf).bytes().map(|b| if b.is_ascii_alphanumeric() || matches!(b, b'-' | b'.' | b'_' | b'~') { 1 } else { 3 }).sum();
        if used + n > max { return format!("{}…", &text[..end]); }
        used += n;
        end = i + c.len_utf8();
    }
    text.to_owned()
}

/// The new-issue address for a draft: the message, and what the person already sees in the form
/// (version, system, core, layout). Never the email: an issue is public.
pub fn issue_url(body: &Value) -> String {
    let message = body["message"].as_str().unwrap_or_default().trim();
    let first = message.lines().next().unwrap_or_default();
    let title: String = first.chars().take(70).collect();
    let mut data = body.clone();
    if let Some(o) = data.as_object_mut() {
        o.remove("message");
        o.remove("email");
    }
    let tail = format!("\n\n<details><summary>Sunna</summary>\n\n```json\n{}\n```\n\n</details>\n", serde_json::to_string_pretty(&data).unwrap_or_default());
    let issue = format!("{}{tail}", fit(message, MAX_ENCODED.saturating_sub(encode(&tail).len())));
    format!("https://github.com/{REPO}/issues/new?title={}&body={}", encode(&format!("feedback: {title}")), encode(&issue))
}

/// Opens the prefilled issue in the browser. Screenshots stay in the app's folder: they can't
/// travel in a link, so the person drops them into the issue.
pub async fn send(body: Value, shots: Vec<String>) -> Result<(), String> {
    validate(&body)?;
    if shots.len() > MAX_IMAGES || body["images"].as_u64() != Some(shots.len() as u64) {
        return Err(crate::i18n::t("rs.feedback.up_to_3_screenshots"));
    }
    for s in &shots { shot_file(s)?; }
    crate::links::open(&crate::links::Target::Url(issue_url(&body)));
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn payload_is_text_only_and_custom_layouts_are_hidden() {
        let body = draft("  broken  ", "  ", 2, "agent", "private-layout");
        assert_eq!(body["message"], "broken");
        assert_eq!(body["email"], Value::Null);
        assert_eq!(body["from"], "agent");
        assert_eq!(body["layout"], "other");
        assert_eq!(body["images"], 2);
        assert_eq!(body.as_object().unwrap().len(), 8);
        assert!(body.get("install").is_none());
        assert_eq!(draft("ok", "a@b.c", 0, "person", "main")["layout"], "main");
    }
    #[test]
    fn invalid_message_and_email_are_explained() {
        for (message, email) in [("", ""), (&"x".repeat(4001), ""), ("ok", "bad"), ("ok", "a b@c.d")] {
            assert!(validate(&draft(message, email, 0, "person", "main")).is_err());
        }
        assert!(validate(&draft("ok", "a@b.c", 0, "person", "main")).is_ok());
    }
    #[test]
    fn the_issue_carries_the_message_but_never_the_email() {
        let url = issue_url(&draft("It broke\nwhen I saved", "me@example.com", 0, "person", "main"));
        assert!(url.starts_with("https://github.com/Jul879n/sunna/issues/new?title=feedback%3A%20It%20broke&body="), "{url}");
        assert!(url.contains("when%20I%20saved"));
        assert!(!url.contains("example"));
        assert!(!url.to_lowercase().contains("herdr"));
        let long = issue_url(&draft(&"ñ".repeat(4000), "", 0, "person", "main"));
        assert!(long.len() < 8000, "{}", long.len());
    }
    #[test]
    fn only_png_and_jpeg_leave_as_they_are() {
        assert_eq!(sniff(b"\x89PNG\r\n\x1a\nrest"), Some("png"));
        assert_eq!(sniff(&[0xff, 0xd8, 0xff, 0xe0]), Some("jpg"));
        assert_eq!(sniff(b"GIF89a"), None);
        assert!(shot_file("data/frame-feedback/../../settings.json").is_err());
        assert!(shot_file("/etc/passwd").is_err());
    }
}
