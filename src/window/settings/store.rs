//! Where the settings pages read and write: settings.json (as an object, so
//! keys the app doesn't know about stay) and the mods' shared values.

use crate::{i18n, mods, settings};
use serde_json::{Map, Value, json};

pub type Object = Map<String, Value>;

/// The notification a failed write shows.
pub struct Failure {
    pub title: String,
    pub body: Option<String>,
}

/// `text` as settings.json: empty is an empty object; anything that doesn't
/// parse (mid-edit) or isn't an object is broken, and nothing may be written over it.
pub fn parse(text: &str) -> Result<Object, Failure> {
    if text.trim().is_empty() {
        return Ok(Object::new());
    }
    match serde_json::from_str::<Value>(text) {
        Ok(Value::Object(o)) => Ok(o),
        _ => Err(Failure { title: i18n::t("settings.settings_json_has_errors_fix_it"), body: None }),
    }
}

/// The file's text after `change`, keeping every other key.
pub fn rewrite(text: &str, change: impl FnOnce(&mut Object)) -> Result<(Object, String), Failure> {
    let mut next = parse(text)?;
    change(&mut next);
    let out = serde_json::to_string_pretty(&next).unwrap_or_default() + "\n";
    Ok((next, out))
}

/// settings.json as an object (a missing file is empty).
pub fn read() -> Result<Object, Failure> {
    match std::fs::read_to_string(settings::path()) {
        Ok(t) => parse(&t),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Object::new()),
        Err(e) => Err(Failure { title: i18n::t("settings.cannot_read_settings_json"), body: Some(e.to_string()) }),
    }
}

/// Reads, changes and writes settings.json back, keeping every other key.
pub fn edit(change: impl FnOnce(&mut Object)) -> Result<Object, Failure> {
    let text = match std::fs::read_to_string(settings::path()) {
        Ok(t) => t,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => String::new(),
        Err(e) => return Err(Failure { title: i18n::t("settings.cannot_read_settings_json"), body: Some(e.to_string()) }),
    };
    let (next, out) = rewrite(&text, change)?;
    crate::persist::write_atomic(&settings::path(), out)
        .map_err(|e| Failure { title: i18n::t("settings.not_saved_to_settings_json"), body: Some(e.to_string()) })?;
    Ok(next)
}

/// `text` of another JSON file of the app's home (keys.json, programs.json) after `change`;
/// an empty or missing file starts from `base`. A file that isn't an object is never written over.
pub fn rewrite_file(name: &str, text: Option<&str>, base: &str, change: impl FnOnce(&mut Object)) -> Result<String, Failure> {
    let broken = || Failure { title: i18n::tf("settings.file_has_errors", &[("name", name)]), body: None };
    let text = match text {
        Some(t) if !t.trim().is_empty() => t,
        _ => base,
    };
    let mut next = match serde_json::from_str::<Value>(text) {
        Ok(Value::Object(o)) => o,
        _ => return Err(broken()),
    };
    change(&mut next);
    Ok(serde_json::to_string_pretty(&next).unwrap_or_default() + "\n")
}

/// Reads, changes and writes back another file of the app's home.
pub fn edit_file(name: &str, base: &str, change: impl FnOnce(&mut Object)) -> Result<(), Failure> {
    let path = crate::mods::app_dir().join(name);
    let text = match std::fs::read_to_string(&path) {
        Ok(t) => Some(t),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => None,
        Err(e) => return Err(Failure { title: i18n::tf("settings.file_has_errors", &[("name", name)]), body: Some(e.to_string()) }),
    };
    let out = rewrite_file(name, text.as_deref(), base, change)?;
    crate::persist::write_atomic(&path, out).map_err(|e| Failure { title: i18n::tf("settings.file_not_saved", &[("name", name)]), body: Some(e.to_string()) })
}

/// A number as JSON the way a person would write it: whole numbers without a fraction.
pub fn num(n: f64) -> Value {
    if n.fract() == 0.0 && n.abs() < 1e15 { json!(n as i64) } else { json!(n) }
}

/// The defaults of the values the pages keep in shared() (the mods repeat
/// them, so a missing value always means "as before").
pub fn shared_default(key: &str) -> Value {
    match key {
        "cleanup.auto" | "cleanup.unchanged" | "cleanup.bots" => json!(false),
        "cleanup.merged" => json!(true),
        "cleanup.every_hours" => json!(24),
        "cleanup.unchanged_days" => json!(1),
        "space.card" => json!({ "git": true, "ago": true, "agents": true }),
        "space.branch_names" | "worktree.fresh" => json!(false),
        "sidebar.buttons" => json!({ "search": true, "bots": true }),
        "status.items" => json!({ "machine": true, "branch": true, "agents": true, "awake": true, "hint": true }),
        "status.format" => json!("total"),
        "tabs.order" => json!("strip"),
        "files.ignored" | "worktree.nest" | "worktree.external" => json!(true),
        "files.open" => json!("click"),
        "menu.open_in" => json!([]),
        "worktree.prefix_mode" => json!("none"),
        "worktree.prefix" => json!(""),
        "worktree.template" => json!("{name}"),
        "changes.base" => json!("head"),
        "changes.groups" => json!("flat"),
        "editor.font_size" => json!(12),
        "editor.line_numbers" | "editor.folding" => json!(true),
        "editor.wrap" => json!(false),
        _ => Value::Null,
    }
}

/// `shared(key) ?? default`.
pub fn get(key: &str) -> Value {
    mods::shared(key).unwrap_or_else(|| shared_default(key))
}

pub fn get_str(key: &str) -> String {
    get(key).as_str().unwrap_or_default().to_owned()
}

/// A boolean kept as is: false only when stored false (`default_true`), or true only when stored true.
pub fn flag(key: &str) -> bool {
    match shared_default(key) {
        Value::Bool(true) => get(key) != json!(false),
        _ => get(key) == json!(true),
    }
}

/// One yes/no stored as a field of an object under `key`.
#[allow(dead_code)] // for the pages still to port
pub fn object_flag(key: &str, field: &str) -> bool {
    let default = shared_default(key).get(field).and_then(Value::as_bool).unwrap_or(false);
    mods::shared(key).and_then(|v| v.get(field).and_then(Value::as_bool)).unwrap_or(default)
}

#[allow(dead_code)] // for the pages still to port
pub fn set_object_flag(key: &str, field: &str, on: bool) {
    let mut now = shared_default(key);
    if let (Some(base), Some(Value::Object(stored))) = (now.as_object_mut(), mods::shared(key)) {
        base.extend(stored);
    }
    now[field] = json!(on);
    mods::set_shared(key.to_owned(), now);
}

#[cfg(test)]
mod tests {
    use super::{flag, object_flag, rewrite, shared_default};
    use crate::i18n;
    use serde_json::{Value, json};

    #[test]
    fn every_shared_key_has_a_default() {
        for key in ["space.card", "space.branch_names", "sidebar.buttons", "status.items", "status.format", "tabs.order", "files.ignored", "files.open",
            "menu.open_in", "worktree.nest", "worktree.external", "worktree.prefix_mode", "worktree.prefix", "worktree.template", "worktree.fresh", "changes.base", "changes.groups",
            "cleanup.auto", "cleanup.merged", "cleanup.unchanged", "cleanup.bots", "cleanup.every_hours", "cleanup.unchanged_days"]
        {
            assert_ne!(shared_default(key), Value::Null, "{key}");
        }
        assert_eq!(shared_default("worktree.template"), json!("{name}"));
        assert_eq!(shared_default("status.items")["awake"], json!(true));
    }

    #[test]
    fn a_change_keeps_what_else_the_file_holds() {
        let (next, out) = rewrite("{\"font_size\": 15}", |q| drop(q.insert("dock_bounce".into(), json!(true)))).ok().unwrap();
        assert_eq!(Value::Object(next), serde_json::from_str::<Value>(r#"{"font_size":15,"dock_bounce":true}"#).unwrap());
        assert!(out.ends_with("}\n"));
        assert!(rewrite("", |_| {}).is_ok());
    }

    #[test]
    fn a_broken_file_is_never_written_over() {
        for raw in ["{\"font_size\": 15, \"theme\": \"x\",}", "null", "[]", "3"] {
            let failure = rewrite(raw, |_| panic!("must not run")).err().unwrap();
            assert_eq!(failure.title, i18n::t("settings.settings_json_has_errors_fix_it"));
        }
    }

    #[test]
    fn flags_follow_their_defaults() {
        // Nothing stored in the test process: defaults.
        assert!(flag("worktree.nest"));
        assert!(!flag("worktree.fresh"));
        assert!(object_flag("space.card", "git"));
        assert!(!object_flag("space.card", "nope"));
    }
}
