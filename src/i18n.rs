//! Interface text in the person's language: flat JSON tables in defaults/locales.

use std::collections::BTreeMap;
use std::sync::OnceLock;
use std::sync::atomic::{AtomicBool, Ordering};

type Table = BTreeMap<String, String>;

static SPANISH: AtomicBool = AtomicBool::new(false);

fn parse(text: &str) -> Table {
    serde_json::from_str(text).unwrap_or_else(|e| {
        tracing::error!("defaults/locales is malformed, showing keys: {e}");
        Table::new()
    })
}

fn en() -> &'static Table {
    static T: OnceLock<Table> = OnceLock::new();
    T.get_or_init(|| parse(include_str!("../defaults/locales/en.json")))
}

fn es() -> &'static Table {
    static T: OnceLock<Table> = OnceLock::new();
    T.get_or_init(|| parse(include_str!("../defaults/locales/es.json")))
}

/// "es" or "en". None or an unknown value: the system's language.
pub fn resolve(language: Option<&str>) -> &'static str {
    match language {
        Some("es") => return "es",
        Some("en") => return "en",
        _ => {}
    }
    if cfg!(test) {
        return "en";
    }
    // macOS: an app started from Finder has no LANG; the system's choice is in AppleLanguages.
    #[cfg(target_os = "macos")]
    if let Some(es) = macos_prefers_spanish() {
        return if es { "es" } else { "en" };
    }
    for var in ["LC_ALL", "LC_MESSAGES", "LANG"] {
        if let Ok(v) = std::env::var(var)
            && !v.is_empty()
        {
            return if v.starts_with("es") { "es" } else { "en" };
        }
    }
    "en"
}

/// Whether the first language in the system's list is Spanish (`defaults read -g AppleLanguages`), asked once.
#[cfg(target_os = "macos")]
fn macos_prefers_spanish() -> Option<bool> {
    static ANSWER: OnceLock<Option<bool>> = OnceLock::new();
    *ANSWER.get_or_init(|| {
        let out = crate::spawn::command("defaults").args(["read", "-g", "AppleLanguages"]).output().ok()?;
        let text = String::from_utf8_lossy(&out.stdout);
        let first = text.split('"').nth(1).or_else(|| text.lines().nth(1).map(str::trim))?;
        Some(first.starts_with("es"))
    })
}

pub fn set(locale: &str) {
    SPANISH.store(locale == "es", Ordering::Relaxed);
}

pub fn locale() -> &'static str {
    if SPANISH.load(Ordering::Relaxed) { "es" } else { "en" }
}

pub fn t(key: &str) -> String {
    let active = if SPANISH.load(Ordering::Relaxed) { es() } else { en() };
    active.get(key).or_else(|| en().get(key)).cloned().unwrap_or_else(|| key.to_owned())
}

pub fn tf(key: &str, vars: &[(&str, &str)]) -> String {
    let mut text = t(key);
    for (name, value) in vars {
        text = text.replace(&format!("{{{name}}}"), value);
    }
    text
}

/// The key of a count's phrase: `<key>.one` for exactly 1, `<key>.other` for anything else (0 included).
pub fn plural_key(key: &str, n: usize) -> String {
    format!("{key}.{}", if n == 1 { "one" } else { "other" })
}

/// A phrase that counts something, in its singular or plural form: «1 evento», «2 eventos».
pub fn tn(key: &str, n: usize, vars: &[(&str, &str)]) -> String {
    tf(&plural_key(key, n), vars)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_spanish_key_exists_in_english() {
        let a: Vec<_> = en().keys().collect();
        let b: Vec<_> = es().keys().collect();
        assert_eq!(a, b);
    }

    #[test]
    fn the_locale_files_parse_and_are_not_empty() {
        for text in [include_str!("../defaults/locales/en.json"), include_str!("../defaults/locales/es.json")] {
            let table: Table = serde_json::from_str(text).expect("locale file is valid JSON");
            assert!(!table.is_empty());
        }
    }

    #[test]
    fn a_count_is_singular_only_at_exactly_one() {
        assert_eq!(plural_key("activity.feed_count", 1), "activity.feed_count.one");
        assert_eq!(plural_key("activity.feed_count", 0), "activity.feed_count.other");
        assert_eq!(plural_key("activity.feed_count", 2), "activity.feed_count.other");
        for (table, one, other) in [(es(), "1 evento", "2 eventos"), (en(), "1 event", "2 events")] {
            let say = |n: usize| table[&plural_key("activity.feed_count", n)].replace("{count}", &n.to_string());
            assert_eq!((say(1).as_str(), say(2).as_str()), (one, other));
        }
    }

    #[test]
    fn every_singular_phrase_has_its_plural_and_back() {
        for table in [en(), es()] {
            for key in table.keys() {
                if let Some(base) = key.strip_suffix(".one") {
                    assert!(table.contains_key(&format!("{base}.other")), "{key} has no .other");
                }
                if let Some(base) = key.strip_suffix(".other") {
                    assert!(table.contains_key(&format!("{base}.one")), "{key} has no .one");
                }
            }
        }
    }

    #[test]
    fn unknown_keys_fall_back_to_the_key() {
        assert_eq!(t("no.such.key"), "no.such.key");
        assert_eq!(tf("no.such.{n}", &[("n", "1")]), "no.such.1");
    }
}
