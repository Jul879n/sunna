//! Claude Code reports its state to Sunna through hooks: each one writes the Program Status
//! Protocol (`OSC 7501`) to the terminal, which the core turns into `working`, `blocked` or
//! `idle`. The hooks are ours when their command carries `MARK`; anything else in
//! `~/.claude/settings.json` is left as it is.
use serde_json::{Value, json};
use std::path::PathBuf;

const MARK: &str = "# sunna-agent-state";
/// Hook event → the state it reports.
const EVENTS: [(&str, &str); 4] = [("UserPromptSubmit", "working"), ("PreToolUse", "working"), ("Notification", "blocked"), ("Stop", "idle")];

fn command(state: &str) -> String {
    format!("printf '\\033]7501;state={state}:app=claude-code\\033\\\\' > /dev/tty 2>/dev/null; true {MARK}")
}

fn ours(entry: &Value) -> bool {
    entry["hooks"].as_array().is_some_and(|h| h.iter().any(|c| c["command"].as_str().is_some_and(|c| c.contains(MARK))))
}

/// Are all our hooks in these settings?
pub fn installed(settings: &Value) -> bool {
    EVENTS.iter().all(|(event, _)| settings["hooks"][event].as_array().is_some_and(|l| l.iter().any(ours)))
}

/// Adds our hooks (once) next to the ones already there.
pub fn install(settings: &mut Value) {
    remove(settings);
    if !settings.is_object() { *settings = json!({}); }
    let hooks = settings.as_object_mut().unwrap().entry("hooks").or_insert_with(|| json!({}));
    if !hooks.is_object() { *hooks = json!({}); }
    for (event, state) in EVENTS {
        let list = hooks.as_object_mut().unwrap().entry(event).or_insert_with(|| json!([]));
        if let Some(list) = list.as_array_mut() {
            list.push(json!({ "matcher": "", "hooks": [{ "type": "command", "command": command(state) }] }));
        }
    }
}

/// Takes our hooks out, and the empty lists they leave.
pub fn remove(settings: &mut Value) {
    let Some(hooks) = settings.get_mut("hooks").and_then(Value::as_object_mut) else { return };
    for (event, _) in EVENTS {
        if let Some(list) = hooks.get_mut(event).and_then(Value::as_array_mut) {
            list.retain(|e| !ours(e));
        }
        if hooks.get(event).and_then(Value::as_array).is_some_and(Vec::is_empty) { hooks.remove(event); }
    }
    if hooks.is_empty() { settings.as_object_mut().unwrap().remove("hooks"); }
}

fn path() -> PathBuf { crate::mods::home().join(".claude/settings.json") }

fn read() -> Result<Value, String> {
    match std::fs::read_to_string(path()) {
        Ok(t) => serde_json::from_str(&t).map_err(|_| crate::i18n::t("settings.agent_hooks_unreadable")),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(json!({})),
        Err(e) => Err(e.to_string()),
    }
}

pub fn is_installed() -> bool { read().is_ok_and(|s| installed(&s)) }

/// Installs or removes the hooks in the real file, keeping a copy of it once (`settings.json.sunna.bak`).
pub fn set(on: bool) -> Result<(), String> {
    let mut settings = read()?;
    let file = path();
    let backup = file.with_extension("json.sunna.bak");
    if file.is_file() && !backup.exists() { let _ = std::fs::copy(&file, &backup); }
    if on { install(&mut settings) } else { remove(&mut settings) }
    if let Some(dir) = file.parent() { std::fs::create_dir_all(dir).map_err(|e| e.to_string())?; }
    let tmp = file.with_extension("json.sunna.tmp");
    std::fs::write(&tmp, serde_json::to_string_pretty(&settings).unwrap_or_default() + "\n").map_err(|e| e.to_string())?;
    std::fs::rename(&tmp, &file).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn installing_is_idempotent_and_keeps_other_hooks() {
        let mut s = json!({"model": "x", "hooks": {"Stop": [{"hooks": [{"type": "command", "command": "other"}]}]}});
        assert!(!installed(&s));
        install(&mut s);
        install(&mut s);
        assert!(installed(&s));
        assert_eq!(s["hooks"]["Stop"].as_array().unwrap().len(), 2);
        assert_eq!(s["hooks"]["Stop"][0]["hooks"][0]["command"], "other");
        assert_eq!(s["model"], "x");
    }
    #[test]
    fn removing_leaves_the_rest_and_no_empty_lists() {
        let mut s = json!({"hooks": {"Stop": [{"hooks": [{"type": "command", "command": "other"}]}]}});
        let before = s.clone();
        install(&mut s);
        remove(&mut s);
        assert_eq!(s, before);
        let mut empty = json!({});
        install(&mut empty);
        remove(&mut empty);
        assert_eq!(empty, json!({}));
    }
    #[test]
    fn the_command_writes_the_protocol_to_the_terminal() {
        let c = command("working");
        assert!(c.contains("7501;state=working:app=claude-code") && c.contains("/dev/tty") && c.ends_with(MARK));
    }
}
