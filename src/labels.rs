//! Terminal labels the app keeps with each terminal (`sunna.space`, `sunna.tab`, `sunna.seen`, ...).
//! Terminals labeled before the rename carry `herdr.*`: those are still read, and a `sunna.*`
//! label wins when both exist. Only `sunna.*` is written.
use serde_json::Value;

/// The label `key` (`"space"`, `"tab"`, ...) in a `labels` object.
pub fn get<'a>(labels: &'a Value, key: &str) -> Option<&'a Value> {
    labels.get(format!("sunna.{key}")).or_else(|| labels.get(format!("herdr.{key}")))
}

/// The label `key` of a terminal record.
pub fn of<'a>(record: &'a Value, key: &str) -> Option<&'a Value> {
    record.get("labels").and_then(|l| get(l, key))
}

/// The label as text.
pub fn text<'a>(record: &'a Value, key: &str) -> Option<&'a str> {
    of(record, key).and_then(Value::as_str)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    #[test]
    fn the_new_label_wins_and_the_old_one_still_reads() {
        let old = json!({"labels": {"herdr.space": "a"}});
        assert_eq!(text(&old, "space"), Some("a"));
        let both = json!({"labels": {"herdr.space": "a", "sunna.space": ""}});
        assert_eq!(text(&both, "space"), Some(""));
        assert_eq!(text(&json!({}), "space"), None);
    }
}
