//! Bots, native (`{"native": "bots"}`, once the `bots` mod): the sidebar's Bots section opens this panel.
//! A bot is a folder `bots/<name>/` of the app's home with `SOUL.md` (what it is for), `bot.json` (agent,
//! folder, triggers, limits) and `runs.json` (its history); nothing about that storage changed, so the
//! bots made by the old mod keep working. Every second minute-tick the window looks at each switched-on
//! bot: a `daily` trigger fires at its time (limited to some weekdays, dropped when missed by more than its
//! grace), an `interval` one every N minutes, a `github` one polls `gh` and starts the bot for each new
//! issue or PR. A run starts the bot's agent in a space named `bot: <name>` (one tab per run); a bot never
//! runs twice at once, never more than `max_runs_per_day`, and a `precheck` command that fails skips the run.
//!
//! Commands (so `sunna do` does what the panel does): `bot.new`, `bot.run`, `bot.enable`, `bot.delete`.
//! Shared keys kept: `bots.new` and `bots.templates` (the sidebar asks the panel for the form or the gallery).
//! The scheduling rules are plain functions over the JSON files' values and are tested at the end.

use super::native::{contrast, fade, icon, r, tip, tone, z};
use super::settings_ui::timefmt;
use super::spaces::{hf, vf};
use super::*;
use crate::i18n;
use gpui_base::input::{Input, InputEvent, InputState};

pub(crate) const COMMANDS: [&str; 4] = ["bot.new", "bot.run", "bot.enable", "bot.delete"];

const CLOSE: &[u8] = include_bytes!("../../assets/icons/pane/close.svg");
const BUTTON_H: f32 = 32.0;
const MAX_RUNS: usize = 100;
/// One color per bot, by its name.
const MARK: [u32; 6] = [0x22a99a, 0xe3814f, 0x5b8def, 0x8b7cf6, 0x3fae6a, 0xd9649a];

// Templates -------------------------------------------------------------------------------------

pub(super) const CATEGORIES: [&str; 4] = ["releases", "support", "planning", "personal"];

/// `(id, category, SOUL.md, triggers as JSON)`. Names and «what it does first» are the locale keys
/// `bots.template_<id>` and `bots.first_<id>` (id with `-` as `_`).
pub(super) const TEMPLATES: [(&str, &str, &str, &str); 8] = [
    ("pr-review", "releases", "Revisas PRs donde te piden revisión.\n\nLee el diff con `gh pr diff`, comenta problemas concretos con `gh pr review --comment`; nunca apruebes ni hagas merge.\n", r#"[{"type":"github","query":"is:pr is:open review-requested:@me","every_minutes":10}]"#),
    ("ci-failures", "releases", "Vigilas los PRs propios con CI en rojo.\n\nPara cada PR abierto con checks fallidos lee el log con `gh run view --log-failed`, explica la causa probable en un comentario con `gh pr comment` y propone el arreglo; no hagas push.\n", r#"[{"type":"github","query":"is:pr is:open author:@me status:failure","every_minutes":15}]"#),
    ("release-notes", "releases", "Redactas las notas de la próxima versión.\n\nLee los commits y PRs desde el último tag (`git log` y `gh pr list --state merged`) y escribe `NOTAS.md` con secciones Nuevo y Corregido, una línea por cambio, en lenguaje de usuario.\n", r#"[{"type":"manual"}]"#),
    ("issue-triage", "support", "Clasificas los issues nuevos.\n\nPara cada issue sin etiquetas propone etiquetas y prioridad con `gh issue comment`; no cierres issues.\n", r#"[{"type":"github","query":"is:issue is:open no:label","every_minutes":10}]"#),
    ("docs-sync", "planning", "Mantienes la documentación al día.\n\nCada mañana compara los cambios recientes del código con README y docs, y anota en `DOCS-PENDIENTES.md` lo que quedó desactualizado; no edites la documentación.\n", r#"[{"type":"daily","at":"09:00"}]"#),
    ("daily-summary", "planning", "Cada mañana resumes la actividad del repo.\n\nEscribe en `RESUMEN.md` los commits de ayer, los PRs abiertos y los issues nuevos.\n", r#"[{"type":"daily","at":"09:00"}]"#),
    ("day-plan", "personal", "Preparas mi plan del día.\n\nLee mis PRs abiertos, revisiones pendientes e issues asignados con `gh` y escribe `HOY.md` con las 3 prioridades y lo que está bloqueado.\n", r#"[{"type":"daily","at":"08:30"}]"#),
    ("blank", "personal", "Describe aquí el propósito del bot.\n", r#"[{"type":"manual"}]"#),
];

pub(super) fn template(id: &str) -> Option<&'static (&'static str, &'static str, &'static str, &'static str)> {
    TEMPLATES.iter().find(|t| t.0 == id)
}

fn template_triggers(id: &str) -> Value {
    template(id).and_then(|t| serde_json::from_str(t.3).ok()).unwrap_or_else(|| json!([{ "type": "manual" }]))
}

/// A template with a schedule or GitHub trigger starts switched off; a manual one is ready.
pub(super) fn starts_off(triggers: &Value) -> bool {
    triggers.as_array().is_some_and(|l| l.iter().any(|t| t["type"] != "manual"))
}

fn key_of(id: &str) -> String {
    id.replace('-', "_")
}

/// The templates matching the search text (their name or what they do first) and the category (`all`: every one).
pub(super) fn filter_templates(query: &str, category: &str, label: impl Fn(&str) -> String) -> Vec<&'static str> {
    let q = query.trim().to_lowercase();
    TEMPLATES.iter().filter(|t| (category == "all" || t.1 == category) && (q.is_empty() || label(t.0).to_lowercase().contains(&q))).map(|t| t.0).collect()
}

// Rules ------------------------------------------------------------------------------------------

/// A bot's folder name: letters a-z, digits and dashes.
pub(super) fn valid_name(name: &str) -> bool {
    !name.is_empty() && name.bytes().all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
}

/// The first paragraph of SOUL.md, on one line.
pub(super) fn purpose(soul: &str) -> String {
    soul.trim().split("\n\n").next().unwrap_or_default().split_whitespace().collect::<Vec<_>>().join(" ")
}

pub(super) fn default_config() -> Value {
    json!({ "agent": "claude", "folder": "~", "enabled": true, "yolo": false, "max_runs_per_day": 10, "triggers": [{ "type": "manual" }] })
}

/// The defaults with what bot.json says over them.
pub(super) fn merged(mine: Value) -> Value {
    let mut out = default_config();
    if let (Some(o), Value::Object(m)) = (out.as_object_mut(), mine) {
        o.extend(m);
    }
    out
}

pub(super) fn empty_state() -> Value {
    json!({ "runs": [], "seen": [], "last_poll": {} })
}

fn runs(state: &Value) -> &[Value] {
    state.get("runs").and_then(Value::as_array).map_or(&[], Vec::as_slice)
}

pub(super) fn parse_ms(iso: &str) -> i64 {
    timefmt::parse_iso(iso).map_or(0, |s| s * 1000)
}

pub(super) fn iso_ms(ms: i64) -> String {
    timefmt::iso(ms.div_euclid(1000))
}

/// The seconds the zone is ahead of UTC at `ms`.
pub(super) fn zone_offset(ms: i64) -> i64 {
    let secs = ms.div_euclid(1000);
    let (y, mo, d, h, mi, s) = timefmt::local(secs);
    timefmt::days(y, i64::from(mo), i64::from(d)) * 86_400 + i64::from(h) * 3600 + i64::from(mi) * 60 + i64::from(s) - secs
}

const DAY: i64 = 86_400_000;

/// Local midnight (as unix ms) of the day `ms` falls in, `off` seconds ahead of UTC.
fn day_start(ms: i64, off: i64) -> i64 {
    (ms + off * 1000).div_euclid(DAY) * DAY - off * 1000
}

/// 0 = Sunday, as JavaScript counts.
fn weekday(ms: i64, off: i64) -> i64 {
    ((ms + off * 1000).div_euclid(DAY) + 4).rem_euclid(7)
}

/// Runs started today. A skipped one (its precheck said no) never started an agent: it does not count.
pub(super) fn runs_today(list: &[Value], now: i64, off: i64) -> usize {
    let today = day_start(now, off);
    list.iter().filter(|r| r["status"] != "skipped" && day_start(parse_ms(r["at"].as_str().unwrap_or_default()), off) == today).count()
}

fn last_of(list: &[Value], kind: &str) -> i64 {
    list.iter().filter(|r| r["trigger"] == kind).map(|r| parse_ms(r["at"].as_str().unwrap_or_default())).max().unwrap_or(0)
}

/// The daily trigger's time today, as unix ms.
fn daily_at(trg: &Value, now: i64, off: i64) -> i64 {
    let mut p = trg["at"].as_str().unwrap_or_default().split(':').map(|x| x.parse::<i64>().unwrap_or(0));
    day_start(now, off) + p.next().unwrap_or(0) * 3_600_000 + p.next().unwrap_or(0) * 60_000
}

/// Does the trigger run on the weekday of `ms`? No `days` (or an empty list) is every day.
fn day_ok(trg: &Value, ms: i64, off: i64) -> bool {
    trg["days"].as_array().is_none_or(|d| d.is_empty() || d.iter().any(|x| x.as_i64() == Some(weekday(ms, off))))
}

/// The triggers of a bot that fire at `now`, given its history. `github` fires through its own poll.
pub(super) fn due_triggers(config: &Value, state: &Value, now: i64, off: i64) -> Vec<Value> {
    let history = runs(state);
    let mut out = Vec::new();
    for trg in config["triggers"].as_array().map_or(&[][..], Vec::as_slice) {
        match trg["type"].as_str() {
            Some("interval") if now - last_of(history, "interval") >= trg["minutes"].as_i64().unwrap_or(0) * 60_000 => out.push(trg.clone()),
            Some("daily") => {
                let at = daily_at(trg, now, off);
                // `grace_minutes` drops a slot that was missed by longer than that.
                let late = trg["grace_minutes"].as_i64().unwrap_or(0) > 0 && now - at > trg["grace_minutes"].as_i64().unwrap_or(0) * 60_000;
                if day_ok(trg, at, off) && !late && now >= at && last_of(history, "daily") < at {
                    out.push(trg.clone());
                }
            }
            Some("github") if now - parse_ms(state["last_poll"]["github"].as_str().unwrap_or_default()) >= trg["every_minutes"].as_i64().unwrap_or(0) * 60_000 => out.push(trg.clone()),
            _ => {}
        }
    }
    out
}

/// When the bot fires next (unix ms), or None for a manual one. `github` waits its poll interval.
pub(super) fn next_run_at(config: &Value, state: &Value, now: i64, off: i64) -> Option<i64> {
    let history = runs(state);
    let mut times = Vec::new();
    for trg in config["triggers"].as_array().map_or(&[][..], Vec::as_slice) {
        match trg["type"].as_str() {
            Some("interval") => {
                let last = last_of(history, "interval");
                times.push(now.max((if last == 0 { now } else { last }) + trg["minutes"].as_i64().unwrap_or(0) * 60_000));
            }
            Some("github") => {
                let last = parse_ms(state["last_poll"]["github"].as_str().unwrap_or_default());
                times.push(now.max((if last == 0 { now } else { last }) + trg["every_minutes"].as_i64().unwrap_or(0) * 60_000));
            }
            Some("daily") => {
                let mut at = daily_at(trg, now, off);
                if at <= now && last_of(history, "daily") >= at {
                    at += DAY;
                }
                for _ in 0..7 {
                    if day_ok(trg, at, off) {
                        break;
                    }
                    at += DAY;
                }
                times.push(now.max(at));
            }
            _ => {}
        }
    }
    times.into_iter().min()
}

/// How far `ms` is from `now`: the unit (`now`, `min`, `hour`, `day`), the amount and whether it is ahead.
pub(super) fn rel_time(ms: i64, now: i64) -> (&'static str, i64, bool) {
    let min = ((ms - now).abs() as f64 / 60_000.0).round() as i64;
    let (unit, n) = if min < 1 { ("now", 0) } else if min < 60 { ("min", min) } else if min < 1440 { ("hour", (min as f64 / 60.0).round() as i64) } else { ("day", (min as f64 / 1440.0).round() as i64) };
    (unit, n, ms > now)
}

/// The history with a run added, capped.
pub(super) fn with_run(mut state: Value, run: Value) -> Value {
    let mut list = runs(&state).to_vec();
    list.push(run);
    let skip = list.len().saturating_sub(MAX_RUNS);
    state["runs"] = Value::Array(list.split_off(skip));
    state
}

/// The one trigger worth naming on a card: its kind, time and minutes.
pub(super) fn schedule_of(triggers: &Value) -> Option<Value> {
    let t = triggers.as_array()?.iter().find(|x| matches!(x["type"].as_str(), Some("interval" | "daily" | "github")))?;
    Some(json!({ "type": t["type"], "at": t["at"], "minutes": if t["minutes"].is_null() { t["every_minutes"].clone() } else { t["minutes"].clone() }, "days": t["days"], "grace": t["grace_minutes"] }))
}

/// The names of the weekdays a trigger is limited to, in the locale's short names (`days` 0 = Sunday); empty for every day.
pub(super) fn days_text(days: &Value, names: &[String]) -> String {
    let mut picked: Vec<i64> = days.as_array().map_or_else(Vec::new, |d| d.iter().filter_map(Value::as_i64).filter(|n| (0..7).contains(n)).collect());
    picked.sort_unstable();
    picked.dedup();
    if picked.len() == 7 {
        return String::new();
    }
    picked.iter().filter_map(|n| names.get(*n as usize).cloned()).collect::<Vec<_>>().join(", ")
}

/// Black or white text, whichever reads better on `fill`.
fn text_on(fill: u32) -> Hsla {
    let rgb = [((fill >> 16) & 255) as f32 / 255.0, ((fill >> 8) & 255) as f32 / 255.0, (fill & 255) as f32 / 255.0];
    if contrast(rgb, [0.0; 3]) >= contrast(rgb, [1.0; 3]) { gpui::black() } else { gpui::white() }
}

fn hue_of(name: &str) -> u32 {
    MARK[name.chars().map(|c| c as usize).sum::<usize>() % MARK.len()]
}

// Storage ----------------------------------------------------------------------------------------

#[derive(Clone, Debug, PartialEq)]
pub(super) struct Bot {
    pub name: String,
    pub soul: String,
    pub config: Value,
    pub state: Value,
}

fn root() -> PathBuf {
    mods::app_dir().join("bots")
}

fn dir(name: &str) -> PathBuf {
    root().join(name)
}

fn read_json(path: &std::path::Path, fallback: Value) -> Value {
    std::fs::read_to_string(path).ok().and_then(|t| serde_json::from_str::<Value>(&t).ok()).filter(|v| !v.is_null()).unwrap_or(fallback)
}

fn write_json(path: &std::path::Path, value: &Value) -> std::io::Result<()> {
    crate::persist::write_atomic(path, format!("{}\n", serde_json::to_string_pretty(value).unwrap_or_default()))
}

/// One bot read from its folder.
pub(super) fn load_bot(name: &str) -> Bot {
    let d = dir(name);
    Bot { name: name.to_owned(), soul: std::fs::read_to_string(d.join("SOUL.md")).unwrap_or_default(), config: merged(read_json(&d.join("bot.json"), json!({}))), state: read_json(&d.join("runs.json"), empty_state()) }
}

/// Every bot, by name.
pub(super) fn load_bots() -> Vec<Bot> {
    let mut names: Vec<String> = std::fs::read_dir(root()).map(|d| d.flatten().filter(|e| e.path().is_dir()).filter_map(|e| e.file_name().into_string().ok()).filter(|n| !n.starts_with('.')).collect()).unwrap_or_default();
    names.sort();
    names.iter().map(|n| load_bot(n)).collect()
}

/// Changes a bot's history on disk (read again first, so what another run wrote is kept).
fn update_state(name: &str, change: impl FnOnce(Value) -> Value) {
    let path = dir(name).join("runs.json");
    let next = change(read_json(&path, empty_state()));
    if let Err(e) = write_json(&path, &next) {
        tracing::warn!("bot {name}: runs.json: {e}");
    }
}

fn set_enabled(name: &str, on: bool) -> std::io::Result<()> {
    let path = dir(name).join("bot.json");
    let mut config = read_json(&path, json!({}));
    config["enabled"] = json!(on);
    write_json(&path, &config)
}

/// Makes a bot's folder from a template; Err says why not.
pub(super) fn create_bot(name: &str, template_id: &str, agent: &str, folder: &str, yolo: bool) -> Result<(), String> {
    let (_, _, soul, _) = template(template_id).ok_or_else(|| format!("no template {template_id:?}"))?;
    if !valid_name(name) {
        return Err("a bot's name is letters a-z, digits and dashes".into());
    }
    if dir(name).exists() {
        return Err(i18n::t("bots.name_taken"));
    }
    let triggers = template_triggers(template_id);
    let mut config = default_config();
    config["agent"] = json!(agent);
    config["folder"] = json!(folder);
    config["yolo"] = json!(yolo);
    config["enabled"] = json!(!starts_off(&triggers));
    config["triggers"] = triggers;
    std::fs::create_dir_all(dir(name)).map_err(|e| e.to_string())?;
    crate::persist::write_atomic(&dir(name).join("SOUL.md"), soul).map_err(|e| e.to_string())?;
    write_json(&dir(name).join("bot.json"), &config).map_err(|e| e.to_string())
}

fn cwd_of(bot: &Bot) -> String {
    let folder = bot.config["folder"].as_str().unwrap_or("~");
    let home = mods::home().to_string_lossy().into_owned();
    if folder == "~" { home } else if let Some(rest) = folder.strip_prefix("~/") { format!("{home}/{rest}") } else { folder.to_owned() }
}

fn tab_name(name: &str, n: i64) -> String {
    format!("bot {name} #{n}")
}

/// Is the agent of this bot's last run still working?
fn is_working(bot: &Bot, agents: &[Value]) -> bool {
    runs(&bot.state).last().and_then(|r| r["tab"].as_str()).is_some_and(|tab| agents.iter().any(|a| a["tab_name"] == tab && a["status"] == "working"))
}

/// working: its last run's agent is busy; idle: waiting for a trigger; off: switched off.
pub(super) fn live_state(bot: &Bot, agents: &[Value]) -> &'static str {
    if bot.config["enabled"] == false { "off" } else if is_working(bot, agents) { "working" } else { "idle" }
}

/// Why «Run now» would be refused (a locale key), None when it can start.
pub(super) fn why_not(bot: &Bot, agents: &[Value], now: i64, off: i64) -> Option<&'static str> {
    if runs_today(runs(&bot.state), now, off) as i64 >= bot.config["max_runs_per_day"].as_i64().unwrap_or(10) {
        Some("bots.why_cap")
    } else if is_working(bot, agents) {
        Some("bots.why_busy")
    } else {
        None
    }
}

/// What a start found out, off the UI thread.
enum Prepared {
    /// Not now: the daily limit or the previous run is still working.
    Refused,
    /// A note went into the history instead of a run (`skipped`, `failed`).
    Noted,
    AgentMissing(String),
    Go { n: i64, tab: String, cwd: String, argv: Vec<String>, task: String },
}

fn prepare(name: &str, trigger: &str, item: &str, now: i64, agents: &[Value], startable: Vec<Value>) -> Prepared {
    let bot = load_bot(name);
    let off = zone_offset(now);
    let config = &bot.config;
    if runs_today(runs(&bot.state), now, off) as i64 >= config["max_runs_per_day"].as_i64().unwrap_or(10) || is_working(&bot, agents) {
        return Prepared::Refused;
    }
    let n = runs(&bot.state).last().and_then(|r| r["id"].as_i64()).unwrap_or(0) + 1;
    let note = |status: &str| {
        update_state(name, |s| with_run(s, json!({ "id": n, "at": iso_ms(now), "trigger": trigger, "tab": null, "status": status })));
        Prepared::Noted
    };
    // The precheck is a command: when it fails the run is skipped. «Run now» ignores it.
    if trigger != "manual" && let Some(cmd) = config["precheck"].as_str().filter(|c| !c.trim().is_empty()) {
        let ok = super::files::exec(None, "sh", &["-c", cmd], &cwd_of(&bot), 60).is_ok_and(|(code, _, _)| code == 0);
        if !ok {
            return note("skipped");
        }
    }
    let want = config["agent"].as_str().unwrap_or("claude");
    let found = mods::known_agents(None, startable).ok().and_then(|v| v.as_array().cloned()).and_then(|l| l.into_iter().find(|a| a["key"] == want && a["installed"] == true));
    let Some(agent) = found else {
        if trigger != "manual" {
            note("failed");
        }
        return Prepared::AgentMissing(want.to_owned());
    };
    let strings = |v: &Value| v.as_array().map(|l| l.iter().filter_map(|s| s.as_str().map(str::to_owned)).collect::<Vec<_>>()).unwrap_or_default();
    // The permission prompts go off only when the bot says so, and never for a GitHub item: it is text from outside.
    let mut argv = strings(&agent["base"]);
    if config["yolo"] == true && trigger != "github" {
        argv.extend(strings(&agent["yolo"]));
    }
    let task = format!("{}\n\n---\n{}", bot.soul, i18n::tf("bots.trigger_context", &[("trigger", trigger), ("item", item)]));
    Prepared::Go { n, tab: tab_name(name, n), cwd: cwd_of(&bot), argv, task }
}

thread_local! {
    static LAUNCHING: RefCell<HashSet<String>> = RefCell::new(HashSet::new());
    static TICKING: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

// The window: scheduler and commands -------------------------------------------------------------------

impl AppWindow {
    /// Starts a bot's run. True when an agent was started; false when refused or nothing started.
    async fn bot_launch(this: &WeakEntity<AppWindow>, cx: &mut AsyncWindowContext, name: String, trigger: String, item: String, now: i64) -> bool {
        if LAUNCHING.with_borrow_mut(|l| !l.insert(name.clone())) {
            return false;
        }
        let started = Self::bot_start(this, cx, &name, &trigger, &item, now).await;
        LAUNCHING.with_borrow_mut(|l| l.remove(&name));
        started
    }

    async fn bot_start(this: &WeakEntity<AppWindow>, cx: &mut AsyncWindowContext, name: &str, trigger: &str, item: &str, now: i64) -> bool {
        let Ok((agents, spaces, startable)) = this.update(cx, |_, _| mods::snapshot_with(|s| (s.agents.as_array().cloned().unwrap_or_default(), s.spaces.as_array().cloned().unwrap_or_default(), crate::programs::startable()))) else { return false };
        let (n2, t2, i2) = (name.to_owned(), trigger.to_owned(), item.to_owned());
        let prepared = cx.background_executor().spawn(async move { prepare(&n2, &t2, &i2, now, &agents, startable) }).await;
        match prepared {
            Prepared::Refused | Prepared::Noted => false,
            Prepared::AgentMissing(agent) => {
                let _ = this.update_in(cx, |this, window, cx| this.bot_toast(i18n::tf("bots.agent_missing", &[("agent", &agent)]), window, cx));
                false
            }
            Prepared::Go { n, tab, cwd, argv, task } => {
                let space = spaces.iter().find(|s| s["name"] == format!("bot: {name}")).and_then(|s| s["id"].as_str().map(str::to_owned));
                let args = json!({ "name": format!("bot: {name}"), "tab": tab, "folder": cwd, "argv": argv, "task": task, "focus": false });
                let _ = this.update_in(cx, |this, window, cx| match space {
                    Some(id) => {
                        this.run("tab.new", json!({ "space": id, "name": tab, "folder": cwd, "argv": argv, "task": task, "focus": false }), window, cx);
                    }
                    None => {
                        this.run("space.new", args, window, cx);
                    }
                });
                let (name, trigger, tab2) = (name.to_owned(), trigger.to_owned(), tab.clone());
                cx.background_executor().spawn(async move { update_state(&name, |s| with_run(s, json!({ "id": n, "at": iso_ms(now), "trigger": trigger, "tab": tab2 }))) }).await;
                true
            }
        }
    }

    fn bot_toast(&mut self, title: String, window: &mut Window, cx: &mut Context<Self>) {
        let spec = json!({ "kind": "toast", "notification": { "title": title, "level": "error" } });
        self.open_overlay(mods::new_overlay_id(), spec, window, cx);
    }

    /// Looks at every bot once: the due triggers run, GitHub ones are polled. Called each minute.
    pub(super) fn bots_tick(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if TICKING.replace(true) {
            return;
        }
        cx.spawn_in(window, async move |this, cx| {
            let now = model::now_ms() as i64;
            let off = zone_offset(now);
            let bots = cx.background_executor().spawn(async { load_bots() }).await;
            for bot in bots.into_iter().filter(|b| b.config["enabled"] != false) {
                for trg in due_triggers(&bot.config, &bot.state, now, off) {
                    // One failing bot (no gh, a bad folder) must not stop the others.
                    if trg["type"] == "github" {
                        Self::bot_poll(&this, cx, &bot, &trg, now).await;
                    } else {
                        Self::bot_launch(&this, cx, bot.name.clone(), trg["type"].as_str().unwrap_or("manual").to_owned(), String::new(), now).await;
                    }
                }
            }
            let _ = this.update_in(cx, |this, window, cx| this.cleanup_tick(window, cx));
            TICKING.set(false);
            let _ = this.update(cx, |_, cx| cx.notify());
        })
        .detach();
    }

    /// Asks GitHub for what matches the trigger's query; each new item starts the bot once. The first good look
    /// only learns what exists, so a bot never starts for everything already open.
    async fn bot_poll(this: &WeakEntity<AppWindow>, cx: &mut AsyncWindowContext, bot: &Bot, trg: &Value, now: i64) {
        let (cwd, query) = (cwd_of(bot), trg["query"].as_str().unwrap_or_default().to_owned());
        let looked = cx
            .background_executor()
            .spawn(async move { super::files::exec(None, "gh", &["api", "-X", "GET", "search/issues", "-f", &format!("q={query}"), "-f", "per_page=20", "--jq", ".items[] | [.html_url, .title] | @tsv"], &cwd, 60) })
            .await;
        // Even a failed look waits its interval, but only a good one counts as having learned what exists.
        let name = bot.name.clone();
        let stamp = iso_ms(now);
        let good = looked.as_ref().is_ok_and(|(code, _, _)| *code == 0);
        let (n2, s2) = (name.clone(), stamp.clone());
        cx.background_executor().spawn(async move { update_state(&n2, |mut s| { s["last_poll"]["github"] = json!(s2); s }) }).await;
        let (true, Ok((_, out, _))) = (good, looked) else { return };
        let learned = bot.state["learned"] == true;
        let mut seen: Vec<String> = bot.state["seen"].as_array().map(|l| l.iter().filter_map(|s| s.as_str().map(str::to_owned)).collect()).unwrap_or_default();
        for line in out.lines().filter(|l| !l.is_empty()) {
            let (url, title) = line.split_once('\t').unwrap_or((line, ""));
            if seen.iter().any(|s| s == url) {
                continue;
            }
            // Only what was launched counts as seen: a refused one (limit, busy) is tried again next time.
            if !learned || Self::bot_launch(this, cx, name.clone(), "github".into(), format!("{title}\n{url}"), now).await {
                seen.push(url.to_owned());
            }
        }
        let keep = seen.split_off(seen.len().saturating_sub(500));
        cx.background_executor().spawn(async move { update_state(&name, |mut s| { s["learned"] = json!(true); s["seen"] = json!(keep); s }) }).await;
    }

    /// `bot.new`, `bot.run`, `bot.enable` and `bot.delete`.
    pub(super) fn bot_command(&mut self, name: &str, a: &Value, window: &mut Window, cx: &mut Context<Self>) -> Result<(), String> {
        let bot = a.get("name").and_then(Value::as_str).ok_or("name is the bot's folder name")?.to_owned();
        if !valid_name(&bot) {
            return Err("a bot's name is letters a-z, digits and dashes".into());
        }
        let exists = dir(&bot).is_dir();
        match name {
            "bot.new" => {
                let installed = mods::known_agents(None, crate::programs::startable()).ok().and_then(|v| v.as_array().cloned()).unwrap_or_default();
                let agent = a.get("agent").and_then(Value::as_str).map(str::to_owned).or_else(|| installed.iter().find(|x| x["installed"] == true).and_then(|x| x["key"].as_str().map(str::to_owned))).unwrap_or_else(|| "claude".into());
                create_bot(&bot, a.get("template").and_then(Value::as_str).unwrap_or("blank"), &agent, a.get("folder").and_then(Value::as_str).unwrap_or("~"), a.get("yolo").and_then(Value::as_bool).unwrap_or(false))?;
            }
            "bot.enable" if exists => set_enabled(&bot, a.get("on").and_then(Value::as_bool).unwrap_or(true)).map_err(|e| e.to_string())?,
            "bot.delete" if exists => {
                if a.get("confirmed").and_then(Value::as_bool) != Some(true) {
                    self.confirm(&i18n::tf("bots.delete_title", &[("name", &bot)]), &i18n::t("bots.delete_detail"), "bot.delete", json!({ "name": bot, "confirmed": true }), window, cx);
                    return Ok(());
                }
                std::fs::remove_dir_all(dir(&bot)).map_err(|e| e.to_string())?;
            }
            "bot.run" if exists => {
                cx.spawn_in(window, async move |this, cx| {
                    let started = Self::bot_launch(&this, cx, bot, "manual".into(), String::new(), model::now_ms() as i64).await;
                    let _ = this.update_in(cx, |this, window, cx| {
                        if !started {
                            this.bot_toast(i18n::t("bots.not_started"), window, cx);
                        }
                        cx.notify();
                    });
                })
                .detach();
            }
            _ => return Err(format!("no bot named {bot:?}")),
        }
        cx.notify();
        Ok(())
    }
}

// The panel ----------------------------------------------------------------------------------------

struct Form {
    name: String,
    template: &'static str,
    agent: Option<String>,
    folder: String,
    yolo: bool,
    /// `(key, name)` of the agents installed here.
    installed: Vec<(String, String)>,
}

struct Gallery {
    category: String,
    query: String,
}

pub(super) struct BotsView {
    app: WeakEntity<AppWindow>,
    bots: Vec<Bot>,
    loaded: bool,
    form: Option<Form>,
    gallery: Option<Gallery>,
    history: Option<String>,
    name_input: Entity<InputState>,
    search_input: Entity<InputState>,
    seen_new: Option<Value>,
    seen_templates: Option<Value>,
    scroll: ScrollHandle,
    name_typing: bool,
    search_typing: bool,
}

impl BotsView {
    pub(super) fn new(app: WeakEntity<AppWindow>, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let name_input = cx.new(|cx| InputState::new(window, cx).placeholder("my-bot"));
        let search_input = cx.new(|cx| InputState::new(window, cx).placeholder(i18n::t("bots.templates_search")));
        for input in [&name_input, &search_input] {
            input.update(cx, |s, _| s.set_editor_style(super::settings_ui::controls::input_style()));
        }
        cx.subscribe_in(&name_input, window, |this, input, ev: &InputEvent, _, cx| match ev {
            InputEvent::Change => {
                let text = input.read(cx).value().trim().to_owned();
                if let Some(f) = &mut this.form {
                    f.name = text;
                }
                cx.notify();
            }
            InputEvent::Focus | InputEvent::Blur => {
                this.name_typing = matches!(ev, InputEvent::Focus);
                cx.notify();
            }
            _ => {}
        })
        .detach();
        cx.subscribe_in(&search_input, window, |this, input, ev: &InputEvent, _, cx| match ev {
            InputEvent::Change => {
                if let Some(g) = &mut this.gallery {
                    g.query = input.read(cx).value().to_string();
                }
                cx.notify();
            }
            InputEvent::Focus | InputEvent::Blur => {
                this.search_typing = matches!(ev, InputEvent::Focus);
                cx.notify();
            }
            _ => {}
        })
        .detach();
        // The scheduler writes the histories: look at the disk now and then, and the ages stay true.
        cx.spawn(async move |this, cx| loop {
            if this.update(cx, |this, cx| this.reload(cx)).is_err() {
                break;
            }
            cx.background_executor().timer(Duration::from_secs(15)).await;
        })
        .detach();
        Self {
            app, bots: Vec::new(), loaded: false, form: None, gallery: None, history: None, name_input, search_input,
            seen_new: mods::shared("bots.new"), seen_templates: mods::shared("bots.templates"), scroll: ScrollHandle::new(), name_typing: false, search_typing: false,
        }
    }

    fn reload(&mut self, cx: &mut Context<Self>) {
        cx.spawn(async move |this, cx| {
            let bots = cx.background_executor().spawn(async { load_bots() }).await;
            let _ = this.update(cx, |this, cx| {
                if this.bots != bots || !this.loaded {
                    this.bots = bots;
                    this.loaded = true;
                    cx.notify();
                }
            });
        })
        .detach();
    }

    /// Runs one of the commands (so the panel and `sunna do` do the same) and reads the folders again.
    fn command(&mut self, name: &'static str, args: Value, window: &mut Window, cx: &mut Context<Self>) {
        let _ = self.app.update(cx, |app, cx| app.run(name, args, window, cx));
        self.reload(cx);
    }

    fn close(&self, window: &mut Window, cx: &mut Context<Self>) {
        let _ = self.app.update(cx, |app, cx| app.run("layout.hide", json!({ "id": "panel" }), window, cx));
    }

    fn new_form(&mut self, template: &'static str, window: &mut Window, cx: &mut Context<Self>) {
        self.gallery = None;
        self.form = Some(Form { name: String::new(), template, agent: None, folder: "~".into(), yolo: false, installed: Vec::new() });
        self.name_input.update(cx, |s, cx| {
            s.set_value("", window, cx);
            s.focus(window, cx);
        });
        let startable = crate::programs::startable();
        cx.spawn(async move |this, cx| {
            let found = cx.background_executor().spawn(async move { mods::known_agents(None, startable) }).await;
            let _ = this.update(cx, |this, cx| {
                if let (Some(f), Ok(Value::Array(list))) = (&mut this.form, found) {
                    f.installed = list.iter().filter(|a| a["installed"] == true).filter_map(|a| Some((a["key"].as_str()?.to_owned(), a["name"].as_str()?.to_owned()))).collect();
                    f.agent = f.installed.first().map(|a| a.0.clone()).or_else(|| Some("claude".into()));
                }
                cx.notify();
            });
        })
        .detach();
        cx.notify();
    }

    fn open_gallery(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.form = None;
        self.gallery = Some(Gallery { category: "all".into(), query: String::new() });
        self.search_input.update(cx, |s, cx| {
            s.set_value("", window, cx);
            s.focus(window, cx);
        });
        cx.notify();
    }

    /// A 32 px button that works with the mouse and, focused, with Enter or Space.
    fn chip(&self, id: impl Into<ElementId>, label: String, on: bool, cx: &mut Context<Self>, act: impl Fn(&mut Self, &mut Window, &mut Context<Self>) + Clone + 'static) -> Stateful<Div> {
        let key_act = act.clone();
        hf().id(id)
            .h(z(BUTTON_H))
            .px(z(10.0))
            .flex_none()
            .rounded(r(6.0))
            .border_1()
            .border_color(c(if on { "accent" } else { "border" }))
            .bg(c(if on { "selected" } else { "background" }))
            .text_size(z(12.0))
            .text_color(if on { c("foreground") } else { tone("muted") })
            .cursor_pointer()
            .hover(|s| s.bg(c("hover")))
            .focusable()
            .tab_stop(true)
            .focus_visible(|s| s.border_color(graphic("accent")))
            .on_key_down(cx.listener(move |this, e: &KeyDownEvent, window, cx| {
                if matches!(e.keystroke.key.as_str(), "enter" | "space") {
                    key_act(this, window, cx);
                    cx.stop_propagation();
                    cx.notify();
                }
            }))
            .on_click(cx.listener(move |this, _: &ClickEvent, window, cx| {
                act(this, window, cx);
                cx.notify();
            }))
            .child(label)
    }

    fn dot(color: Hsla, hollow: bool) -> Div {
        div().size(z(8.0)).flex_none().rounded_full().when(hollow, |d| d.border_1().border_color(color)).when(!hollow, |d| d.bg(color))
    }

    fn rel(ms: i64, now: i64) -> String {
        let (unit, n, ahead) = rel_time(ms, now);
        if unit == "now" {
            return i18n::t("bots.rel_now");
        }
        i18n::tf(if ahead { "bots.rel_in" } else { "bots.rel_ago" }, &[("n", &n.to_string()), ("unit", &i18n::t(&format!("bots.unit_{unit}")))])
    }

    /// «daily 09:00 · Mon, Tue», «every 10 min», «manual»: what starts it.
    fn schedule(config: &Value) -> String {
        let Some(s) = schedule_of(&config["triggers"]) else { return i18n::t("bots.trigger_manual") };
        let kind = s["type"].as_str().unwrap_or_default();
        let mut text = i18n::tf(&format!("bots.schedule_{kind}"), &[("at", s["at"].as_str().unwrap_or_default()), ("min", &s["minutes"].as_i64().map(|m| m.to_string()).unwrap_or_default())]);
        let names: Vec<String> = i18n::t("bots.weekdays").split(',').map(|d| d.trim().to_owned()).collect();
        let days = days_text(&s["days"], &names);
        if !days.is_empty() {
            text = format!("{text} · {days}");
        }
        text
    }

    /// One run's dot: working, blocked or done by its tab's agent; hollow when the tab is gone.
    fn run_tone(run: &Value, agents: &[Value]) -> (Hsla, bool) {
        match run["status"].as_str() {
            Some("failed") => return (tone("blocked"), false),
            Some("skipped") => return (tone("dim"), true),
            _ => {}
        }
        match agents.iter().find(|a| a["tab_name"] == run["tab"]) {
            None => (tone("dim"), true),
            Some(a) => (match a["status"].as_str() { Some("working") => c("working"), Some("blocked") => tone("blocked"), _ => c("done") }, false),
        }
    }

    fn template_card(&self, id: &'static str, cx: &mut Context<Self>) -> AnyElement {
        let key = key_of(id);
        let (_, category, _, triggers) = template(id).copied().unwrap_or(("blank", "personal", "", "[]"));
        let sched = Self::schedule(&json!({ "triggers": serde_json::from_str::<Value>(triggers).unwrap_or_default() }));
        let act = move |this: &mut Self, window: &mut Window, cx: &mut Context<Self>| this.new_form(id, window, cx);
        let key_act = act;
        vf().id(SharedString::from(format!("bot-tpl-card-{id}")))
            .w_full()
            .flex_none()
            .p(z(12.0))
            .gap(z(6.0))
            .rounded(r(10.0))
            .border_1()
            .border_color(c("border"))
            .bg(c("surface"))
            .cursor_pointer()
            .hover(|s| s.bg(c("hover")))
            .focusable()
            .tab_stop(true)
            .focus_visible(|s| s.border_color(graphic("accent")))
            .on_key_down(cx.listener(move |this, e: &KeyDownEvent, window, cx| {
                if matches!(e.keystroke.key.as_str(), "enter" | "space") {
                    key_act(this, window, cx);
                    cx.stop_propagation();
                }
            }))
            .on_click(cx.listener(move |this, _: &ClickEvent, window, cx| act(this, window, cx)))
            .child(div().text_size(z(12.0)).text_color(tone("dim")).child(i18n::t(&format!("bots.cat_{category}"))))
            .child(div().truncate().font_weight(FontWeight::SEMIBOLD).text_size(z(13.0)).child(i18n::t(&format!("bots.template_{key}"))))
            .child(div().text_size(z(12.0)).line_height(z(16.0)).max_h(z(32.0)).overflow_hidden().text_color(tone("muted")).child(i18n::t(&format!("bots.first_{key}"))))
            .child(div().text_size(z(12.0)).text_color(tone("dim")).child(sched))
            .into_any_element()
    }

    /// No bot yet: say so and offer the templates, like Orca's first screen.
    fn empty_view(&self, cx: &mut Context<Self>) -> AnyElement {
        vf().id("bots-empty")
            .flex_1()
            .min_h(px(0.0))
            .overflow_y_scroll()
            .track_scroll(&self.scroll)
            .p(z(12.0))
            .gap(z(8.0))
            .child(div().font_weight(FontWeight::SEMIBOLD).text_size(z(14.0)).child(i18n::t("bots.empty_title")))
            .child(div().text_size(z(12.0)).text_color(tone("muted")).child(i18n::t("bots.templates_note")))
            .children(TEMPLATES.iter().filter(|t| t.0 != "blank").map(|t| self.template_card(t.0, cx)))
            .child(self.chip("bot-empty-blank", i18n::t("bots.templates_blank"), false, cx, |this, window, cx| this.new_form("blank", window, cx)).w_full().justify_center())
            .into_any_element()
    }

    fn gallery_view(&self, cx: &mut Context<Self>) -> AnyElement {
        let Some(g) = &self.gallery else { return div().into_any_element() };
        let label = |id: &str| format!("{} {}", i18n::t(&format!("bots.template_{}", key_of(id))), i18n::t(&format!("bots.first_{}", key_of(id))));
        let found = filter_templates(&g.query, &g.category, label);
        let cats = std::iter::once("all").chain(CATEGORIES).map(|cat| {
            let pick = cat.to_owned();
            self.chip(format!("bot-cat-{cat}"), i18n::t(&format!("bots.cat_{cat}")), g.category == cat, cx, move |this, _, _| {
                if let Some(g) = &mut this.gallery {
                    g.category = pick.clone();
                }
            })
            .into_any_element()
        });
        vf().flex_1()
            .min_h(px(0.0))
            .child(
                vf().flex_none()
                    .p(z(10.0))
                    .gap(z(8.0))
                    .child(
                        hf().id("bot-search")
                            .h(z(BUTTON_H))
                            .px(z(10.0))
                            .rounded(r(8.0))
                            .border_1()
                            .border_color(c(if self.search_typing { "accent" } else { "border" }))
                            .cursor_text()
                            .text_size(z(12.0))
                            .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                            .child(div().flex_1().child(Input::new(&self.search_input))),
                    )
                    .child(hf().gap(z(6.0)).flex_wrap().children(cats.collect::<Vec<_>>()))
                    .child(div().text_size(z(12.0)).text_color(tone("dim")).child(i18n::t("bots.templates_note"))),
            )
            .child(fade(8.0).my(z(0.0)))
            .child(
                vf().id("bots-templates")
                    .flex_1()
                    .min_h(px(0.0))
                    .overflow_y_scroll()
                    .track_scroll(&self.scroll)
                    .p(z(8.0))
                    .gap(z(8.0))
                    .children(found.iter().map(|id| self.template_card(id, cx)).collect::<Vec<_>>())
                    .when(found.is_empty(), |d| {
                        d.child(
                            vf().gap(z(8.0))
                                .p(z(8.0))
                                .items_start()
                                .child(div().text_size(z(12.0)).text_color(tone("muted")).child(i18n::tf("bots.templates_none", &[("query", &g.query)])))
                                .child(self.chip("bot-tpl-blank", i18n::t("bots.templates_blank"), false, cx, |this, window, cx| this.new_form("blank", window, cx))),
                        )
                    }),
            )
            .into_any_element()
    }

    fn form_view(&self, cx: &mut Context<Self>) -> AnyElement {
        let Some(f) = &self.form else { return div().into_any_element() };
        let taken = self.bots.iter().any(|b| b.name == f.name);
        let valid = valid_name(&f.name) && !taken;
        let bad = !f.name.is_empty() && !valid;
        let templates = TEMPLATES.iter().map(|t| {
            let id = t.0;
            self.chip(format!("bot-tpl-{id}"), i18n::t(&format!("bots.template_{}", key_of(id))), f.template == id, cx, move |this, _, _| {
                if let Some(f) = &mut this.form {
                    f.template = id;
                }
            })
            .into_any_element()
        }).collect::<Vec<_>>();
        let agents = f.installed.iter().map(|(key, name)| {
            let pick = key.clone();
            self.chip(format!("bot-agent-{key}"), name.clone(), f.agent.as_deref() == Some(key), cx, move |this, _, _| {
                if let Some(f) = &mut this.form {
                    f.agent = Some(pick.clone());
                }
            })
            .into_any_element()
        }).collect::<Vec<_>>();
        let tilde = {
            let home = mods::home().to_string_lossy().into_owned();
            f.folder.strip_prefix(&home).map_or_else(|| f.folder.clone(), |rest| format!("~{rest}"))
        };
        vf().flex_none()
            .p(z(10.0))
            .gap(z(8.0))
            .child(
                hf().id("bot-name")
                    .h(z(BUTTON_H))
                    .px(z(10.0))
                    .rounded(r(8.0))
                    .border_1()
                    .border_color(if bad { tone("blocked") } else { c(if self.name_typing { "accent" } else { "border" }) })
                    .cursor_text()
                    .text_size(z(12.0))
                    .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                    .child(div().flex_1().child(Input::new(&self.name_input))),
            )
            .when(taken, |d| d.child(div().text_size(z(12.0)).text_color(tone("blocked")).child(i18n::t("bots.name_taken"))))
            .when(bad && !taken, |d| d.child(div().text_size(z(12.0)).text_color(tone("blocked")).child(i18n::t("bots.name_rules"))))
            .child(hf().gap(z(6.0)).flex_wrap().children(templates))
            .child(hf().gap(z(6.0)).flex_wrap().children(agents))
            .child(
                hf().gap(z(6.0))
                    .flex_wrap()
                    .child(self.chip("bot-folder", tilde, false, cx, |this, window, cx| {
                        let paths = cx.prompt_for_paths(PathPromptOptions { files: false, directories: true, multiple: false, prompt: None });
                        cx.spawn_in(window, async move |this2, cx| {
                            let Ok(Ok(Some(paths))) = paths.await else { return };
                            let _ = this2.update(cx, |t, cx| {
                                if let (Some(f), Some(p)) = (&mut t.form, paths.first()) {
                                    f.folder = p.to_string_lossy().into_owned();
                                }
                                cx.notify();
                            });
                        })
                        .detach();
                        let _ = this;
                    }))
                    .child(self.chip("bot-yolo", "yolo".into(), f.yolo, cx, |this, _, _| {
                        if let Some(f) = &mut this.form {
                            f.yolo = !f.yolo;
                        }
                    })),
            )
            .when(f.yolo, |d| d.child(div().text_size(z(12.0)).text_color(tone("blocked")).child(i18n::t("bots.yolo_warning"))))
            .child(
                hf().gap(z(6.0))
                    .child(self.chip("bot-create", i18n::t("bots.create"), valid, cx, move |this, window, cx| {
                        let Some(f) = &this.form else { return };
                        if !valid_name(&f.name) || this.bots.iter().any(|b| b.name == f.name) {
                            return;
                        }
                        let args = json!({ "name": f.name, "template": f.template, "agent": f.agent, "folder": f.folder, "yolo": f.yolo });
                        this.form = None;
                        this.command("bot.new", args, window, cx);
                    }))
                    .child(self.chip("bot-cancel", i18n::t("settings.cancel"), false, cx, |this, _, _| this.form = None)),
            )
            .into_any_element()
    }

    fn card(&self, bot: &Bot, agents: &[Value], now: i64, off: i64, cx: &mut Context<Self>) -> AnyElement {
        let name = bot.name.clone();
        let last = runs(&bot.state).last();
        let live = live_state(bot, agents);
        let state_tone = match live { "working" => c("working"), "idle" => c("done"), _ => tone("dim") };
        let hue = hue_of(&bot.name);
        let next = (bot.config["enabled"] != false).then(|| next_run_at(&bot.config, &bot.state, now, off)).flatten();
        let why = why_not(bot, agents, now, off);
        let enabled = bot.config["enabled"] != false;
        let open = self.history.as_deref() == Some(&bot.name);
        let next_text = if live == "off" { i18n::t("bots.paused") } else { next.map_or_else(|| i18n::t("bots.trigger_manual"), |n| Self::rel(n, now)) };
        let last_tone = last.map(|r| Self::run_tone(r, agents));
        let meta = |dot: Div, text: String| hf().min_h(z(20.0)).gap(z(6.0)).text_size(z(12.0)).text_color(tone("muted")).child(dot).child(div().child(text));
        let (n_run, n_pause, n_soul, n_hist, n_del) = (name.clone(), name.clone(), name.clone(), name.clone(), name.clone());
        let run_id = SharedString::from(format!("bot-run-{name}"));
        let run_act = {
            let n = n_run.clone();
            move |this: &mut Self, window: &mut Window, cx: &mut Context<Self>| {
                if why.is_none() {
                    this.command("bot.run", json!({ "name": n }), window, cx);
                }
            }
        };
        let run_key = run_act.clone();
        let primary = hf().id(run_id)
            .h(z(BUTTON_H))
            .px(z(12.0))
            .flex_none()
            .rounded(r(7.0))
            .border_1()
            .text_size(z(12.0))
            .font_weight(FontWeight::SEMIBOLD)
            .when(why.is_none(), |d| d.bg(c("accent")).border_color(c("accent")).text_color(on_accent()).cursor_pointer().hover(|s| s.opacity(0.9)).focusable().tab_stop(true).focus_visible(|s| s.border_color(graphic("accent"))))
            .when(why.is_some(), |d| d.bg(c("raised")).border_color(c("raised")).text_color(tone("dim")))
            .on_key_down(cx.listener(move |this, e: &KeyDownEvent, window, cx| {
                if matches!(e.keystroke.key.as_str(), "enter" | "space") {
                    run_key(this, window, cx);
                    cx.stop_propagation();
                }
            }))
            .on_click(cx.listener(move |this, _: &ClickEvent, window, cx| run_act(this, window, cx)))
            .child(i18n::t("bots.run_now"));
        vf().id(SharedString::from(format!("bot-{name}")))
            .w_full()
            .flex_none()
            .p(z(12.0))
            .gap(z(10.0))
            .rounded(r(10.0))
            .border_1()
            .border_color(c("border"))
            .bg(c("surface"))
            .child(
                hf().gap(z(12.0))
                    .items_start()
                    .child(div().size(z(32.0)).flex_none().rounded(r(9.0)).bg(rgb(hue)).flex().items_center().justify_center().text_size(z(15.0)).font_weight(FontWeight::BOLD).text_color(text_on(hue)).child(name.chars().next().map(|c| c.to_uppercase().to_string()).unwrap_or_else(|| "?".into())))
                    .child(
                        vf().flex_1()
                            .min_w(px(0.0))
                            .gap(z(2.0))
                            .child(div().truncate().font_weight(FontWeight::SEMIBOLD).text_size(z(14.0)).child(name.clone()))
                            .child(div().truncate().text_size(z(12.0)).text_color(tone("dim")).child(format!("{} · {}", bot.config["agent"].as_str().unwrap_or("claude"), Self::schedule(&bot.config))))
                            .child(div().text_size(z(12.0)).line_height(z(16.0)).max_h(z(32.0)).overflow_hidden().text_color(tone("muted")).child(purpose(&bot.soul))),
                    )
                    .child(hf().id(SharedString::from(format!("bot-state-{name}"))).h(z(24.0)).px(z(8.0)).gap(z(6.0)).flex_none().rounded(r(6.0)).bg(c("raised")).text_size(z(12.0)).text_color(tone("muted")).child(Self::dot(state_tone, live == "off")).child(i18n::t(&format!("bots.status_{live}")))),
            )
            .child(
                vf().gap(z(4.0))
                    .child(meta(Self::dot(tone("dim"), true), format!("{}: {}", i18n::t("bots.next_run"), next_text)))
                    .child(meta(last_tone.map_or_else(|| Self::dot(tone("dim"), true), |(color, hollow)| Self::dot(color, hollow)), format!("{}: {}", i18n::t("bots.last_run"), last.map_or_else(|| i18n::t("bots.never"), |r| Self::rel(parse_ms(r["at"].as_str().unwrap_or_default()), now))))),
            )
            .child(
                hf().gap(z(6.0))
                    .flex_wrap()
                    .child(primary)
                    .child(self.chip(format!("bot-on-{name}"), i18n::t(if enabled { "bots.pause" } else { "bots.resume" }), false, cx, move |this, window, cx| this.command("bot.enable", json!({ "name": n_pause, "on": !enabled }), window, cx)))
                    .child(self.chip(format!("bot-soul-{name}"), i18n::t("bots.edit_soul"), false, cx, move |this, window, cx| {
                        let path = dir(&n_soul).join("SOUL.md").to_string_lossy().into_owned();
                        let _ = this.app.update(cx, |app, cx| app.run("file.open", json!({ "path": path, "line": 1, "on": "" }), window, cx));
                    }))
                    .child(self.chip(format!("bot-hist-{name}"), i18n::t("bots.detail"), open, cx, move |this, _, _| this.history = if this.history.as_deref() == Some(&n_hist) { None } else { Some(n_hist.clone()) }))
                    .child(div().flex_1())
                    .child(self.chip(format!("bot-del-{name}"), i18n::t("bots.delete"), false, cx, move |this, window, cx| {
                        let _ = this.app.update(cx, |app, cx| app.run("bot.delete", json!({ "name": n_del }), window, cx));
                    }).text_color(tone("blocked"))),
            )
            .when_some(why, |d, key| d.child(div().text_size(z(12.0)).text_color(tone("muted")).child(i18n::t(key))))
            .when(open, |d| d.child(Self::detail(bot)).child(self.runs_view(bot, agents, now, cx)))
            .into_any_element()
    }

    /// Where it runs and what it can do: the bot acts as the person, so this says so before they switch it on.
    fn detail(bot: &Bot) -> Div {
        let line = |text: String, color: Hsla| div().text_size(z(12.0)).line_height(z(16.0)).text_color(color).child(text);
        let mut lines = vec![
            line(i18n::tf("bots.runs_on", &[("folder", bot.config["folder"].as_str().unwrap_or("~")), ("agent", bot.config["agent"].as_str().unwrap_or("claude"))]), tone("muted")),
            line(i18n::t("bots.perm_scope"), tone("muted")),
            line(i18n::t(if bot.config["yolo"] == true { "bots.perm_yolo" } else { "bots.perm_default" }), if bot.config["yolo"] == true { tone("blocked") } else { tone("muted") }),
        ];
        if let Some(cmd) = bot.config["precheck"].as_str().filter(|c| !c.trim().is_empty()) {
            lines.push(line(i18n::tf("bots.precheck_line", &[("cmd", cmd)]), tone("muted")));
        }
        if let Some(grace) = bot.config["triggers"].as_array().and_then(|l| l.iter().find_map(|t| t["grace_minutes"].as_i64())).filter(|g| *g > 0) {
            lines.push(line(i18n::tf("bots.grace_line", &[("min", &grace.to_string())]), tone("muted")));
        }
        lines.push(line(i18n::tf("bots.limit_line", &[("n", &bot.config["max_runs_per_day"].as_i64().unwrap_or(10).to_string())]), tone("muted")));
        vf().gap(z(3.0)).p(z(8.0)).rounded(r(8.0)).bg(c("raised")).children(lines)
    }

    /// The latest runs, newest first, each with its agent's state as a dot.
    fn runs_view(&self, bot: &Bot, agents: &[Value], now: i64, cx: &mut Context<Self>) -> Div {
        let list: Vec<&Value> = runs(&bot.state).iter().rev().take(8).collect();
        vf().gap(z(2.0))
            .child(hf().gap(z(6.0)).text_size(z(12.0)).font_weight(FontWeight::SEMIBOLD).text_color(tone("muted")).child(div().child(i18n::t("bots.recent_runs"))).child(div().text_color(tone("dim")).child(runs(&bot.state).len().to_string())))
            .when(list.is_empty(), |d| d.child(div().text_size(z(12.0)).text_color(tone("dim")).child(i18n::t("bots.no_runs"))))
            .children(list.into_iter().map(|run| {
                let (color, hollow) = Self::run_tone(run, agents);
                let tab = run["tab"].as_str().unwrap_or_default().to_owned();
                let id = run["id"].as_i64().unwrap_or(0);
                let act = move |this: &mut Self, window: &mut Window, cx: &mut Context<Self>| {
                    let found = mods::snapshot_with(|s| s.spaces.as_array().into_iter().flatten().flat_map(|sp| sp["tabs"].as_array().into_iter().flatten()).find(|t| !tab.is_empty() && t["name"] == tab.as_str()).and_then(|t| t["id"].as_str().map(str::to_owned)));
                    if let Some(tab_id) = found {
                        let _ = this.app.update(cx, |app, cx| app.run("tab.select", json!({ "id": tab_id }), window, cx));
                    }
                };
                let key_act = act.clone();
                let status = run["status"].as_str().map(|s| format!(" · {}", i18n::t(&format!("bots.run_{s}")))).unwrap_or_default();
                hf().id(SharedString::from(format!("bot-r-{}-{id}", bot.name)))
                    .h(z(BUTTON_H))
                    .px(z(6.0))
                    .gap(z(8.0))
                    .rounded(r(6.0))
                    .border_2()
                    .border_color(Hsla { a: 0.0, ..c("accent") })
                    .text_size(z(12.0))
                    .text_color(tone("muted"))
                    .cursor_pointer()
                    .hover(|s| s.bg(c("hover")))
                    .focusable()
                    .tab_stop(true)
                    .focus_visible(|s| s.border_color(graphic("accent")))
                    .on_key_down(cx.listener(move |this, e: &KeyDownEvent, window, cx| {
                        if matches!(e.keystroke.key.as_str(), "enter" | "space") {
                            key_act(this, window, cx);
                            cx.stop_propagation();
                        }
                    }))
                    .on_click(cx.listener(move |this, _: &ClickEvent, window, cx| act(this, window, cx)))
                    .child(Self::dot(color, hollow))
                    .child(div().flex_1().truncate().child(format!("#{id} · {}{status}", run["trigger"].as_str().unwrap_or_default())))
                    .child(div().flex_none().text_color(tone("dim")).child(Self::rel(parse_ms(run["at"].as_str().unwrap_or_default()), now)))
            }))
    }
}

impl Render for BotsView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        // The sidebar asks for the form or the gallery by setting a shared key.
        let asked = mods::shared("bots.new");
        if asked != self.seen_new {
            self.seen_new = asked;
            if self.form.is_none() {
                self.new_form("blank", window, cx);
            }
        }
        let asked = mods::shared("bots.templates");
        if asked != self.seen_templates {
            self.seen_templates = asked;
            self.open_gallery(window, cx);
        }
        let now = model::now_ms() as i64;
        let off = zone_offset(now);
        let agents = mods::snapshot_with(|s| s.agents.as_array().cloned().unwrap_or_default());
        let cards: Vec<AnyElement> = self.bots.iter().map(|b| self.card(b, &agents, now, off, cx)).collect();
        let list = vf().id("bots-list")
            .flex_1()
            .min_h(px(0.0))
            .overflow_y_scroll()
            .track_scroll(&self.scroll)
            .p(z(8.0))
            .gap(z(8.0))
            .when(!self.loaded, |d| d.child(div().p(z(8.0)).text_size(z(12.0)).text_color(tone("muted")).child(i18n::t("bots.loading"))))
            .children(cards)
            .into_any_element();
        let gallery_open = self.gallery.is_some();
        let body = if gallery_open {
            self.gallery_view(cx)
        } else if self.loaded && self.bots.is_empty() && self.form.is_none() {
            self.empty_view(cx)
        } else {
            list
        };
        vf().id("bots")
            .size_full()
            .min_h(px(0.0))
            .rounded(r(10.0))
            .overflow_hidden()
            .bg(c("surface"))
            .border_1()
            .border_color(c("border"))
            .text_color(c("foreground"))
            .text_size(z(13.0))
            .child(
                hf().h(z(46.0))
                    .flex_none()
                    .pl(z(14.0))
                    .pr(z(5.0))
                    .gap(z(8.0))
                    .child(div().font_weight(FontWeight::SEMIBOLD).child(i18n::t("tabs.bots")))
                    .child(div().flex_1())
                    .child(self.chip("bot-templates", i18n::t("bots.templates"), gallery_open, cx, |this, window, cx| if this.gallery.is_some() { this.gallery = None } else { this.open_gallery(window, cx) }))
                    .child(self.chip("bot-new", i18n::t("bots.new"), self.form.is_some(), cx, |this, window, cx| this.new_form("blank", window, cx)))
                    .child(
                        hf().id("panel-close")
                            .size(z(BUTTON_H))
                            .flex_none()
                            .justify_center()
                            .rounded(r(6.0))
                            .border_1()
                            .border_color(Hsla { a: 0.0, ..c("accent") })
                            .cursor_pointer()
                            .hover(|s| s.bg(c("hover")))
                            .focusable()
                            .tab_stop(true)
                            .focus_visible(|s| s.border_color(graphic("accent")))
                            .tooltip(tip(i18n::t("menu.close")))
                            .on_key_down(cx.listener(|this, e: &KeyDownEvent, window, cx| {
                                if matches!(e.keystroke.key.as_str(), "enter" | "space") {
                                    this.close(window, cx);
                                    cx.stop_propagation();
                                }
                            }))
                            .on_click(cx.listener(|this, _: &ClickEvent, window, cx| this.close(window, cx)))
                            .child(icon(CLOSE, 13.0, tone("muted"))),
                    ),
            )
            .child(fade(8.0).my(z(0.0)))
            .when(self.form.is_some() && !gallery_open, |d| d.child(self.form_view(cx)).child(fade(8.0).my(z(0.0))))
            .child(body)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use core::prelude::v1::test;

    const H: i64 = 3_600_000;
    /// 2026-01-07 (a Wednesday) 12:00 UTC.
    const NOON: i64 = 1_767_787_200_000;

    fn cfg(triggers: Value) -> Value {
        merged(json!({ "triggers": triggers }))
    }
    fn run(id: i64, at: i64, trigger: &str) -> Value {
        json!({ "id": id, "at": iso_ms(at), "trigger": trigger, "tab": format!("bot x #{id}") })
    }

    #[test]
    fn names_are_letters_digits_and_dashes() {
        assert!(valid_name("my-bot-2"));
        for bad in ["", "My", "a b", "a_b", "../x", "ñ"] {
            assert!(!valid_name(bad), "{bad}");
        }
    }

    #[test]
    fn a_templates_purpose_is_its_first_paragraph_on_one_line() {
        assert_eq!(purpose("Revisas PRs.\nde noche\n\nLee el diff"), "Revisas PRs. de noche");
        assert_eq!(purpose(""), "");
        for (id, _, soul, triggers) in TEMPLATES {
            assert!(!purpose(soul).is_empty(), "{id}");
            assert!(serde_json::from_str::<Value>(triggers).unwrap().is_array(), "{id}");
        }
    }

    #[test]
    fn schedules_and_github_templates_start_off_and_manual_ones_start_on() {
        assert!(starts_off(&template_triggers("pr-review")));
        assert!(starts_off(&template_triggers("day-plan")));
        assert!(!starts_off(&template_triggers("release-notes")));
        assert!(!starts_off(&template_triggers("blank")));
    }

    #[test]
    fn the_gallery_filters_by_category_and_text() {
        let label = |id: &str| id.to_owned();
        assert_eq!(filter_templates("", "all", label).len(), 8);
        assert_eq!(filter_templates("", "planning", label), ["docs-sync", "daily-summary"]);
        assert_eq!(filter_templates(" TRIAGE ", "all", label), ["issue-triage"]);
        assert!(filter_templates("zzz", "all", label).is_empty());
    }

    #[test]
    fn the_defaults_sit_under_what_bot_json_says() {
        let c = merged(json!({ "agent": "codex", "precheck": "true" }));
        assert_eq!((c["agent"].as_str(), c["max_runs_per_day"].as_i64(), c["precheck"].as_str()), (Some("codex"), Some(10), Some("true")));
        assert_eq!(merged(Value::Null)["enabled"], json!(true));
    }

    #[test]
    fn local_days_follow_the_zone() {
        // 23:30 UTC is already the next day three hours east.
        let late = NOON + 11 * H + H / 2;
        assert_ne!(day_start(late, 0), day_start(late, 3 * 3600));
        assert_eq!(weekday(NOON, 0), 3);
        assert_eq!(weekday(NOON, 12 * 3600), 4);
        assert_eq!(day_start(NOON, 0), NOON - 12 * H);
    }

    #[test]
    fn a_skipped_run_does_not_count_against_the_daily_limit() {
        let mut skipped = run(2, NOON - H, "daily");
        skipped["status"] = json!("skipped");
        let list = vec![run(1, NOON - 2 * H, "daily"), skipped, run(3, NOON - 30 * H, "daily")];
        assert_eq!(runs_today(&list, NOON, 0), 1);
    }

    #[test]
    fn an_interval_fires_once_enough_minutes_passed() {
        let c = cfg(json!([{ "type": "interval", "minutes": 10 }]));
        assert_eq!(due_triggers(&c, &empty_state(), NOON, 0).len(), 1);
        let st = with_run(empty_state(), run(1, NOON - 5 * 60_000, "interval"));
        assert!(due_triggers(&c, &st, NOON, 0).is_empty());
        assert_eq!(next_run_at(&c, &st, NOON, 0), Some(NOON + 5 * 60_000));
        assert_eq!(due_triggers(&c, &st, NOON + 5 * 60_000, 0).len(), 1);
    }

    #[test]
    fn a_daily_fires_after_its_time_once_a_day_on_its_weekdays_within_its_grace() {
        let c = cfg(json!([{ "type": "daily", "at": "09:00" }]));
        assert_eq!(due_triggers(&c, &empty_state(), NOON, 0).len(), 1);
        assert!(due_triggers(&c, &empty_state(), NOON - 4 * H, 0).is_empty());
        // Already ran today at 09:05: not again, and the next one is tomorrow at 09:00.
        let st = with_run(empty_state(), run(1, NOON - 3 * H + 5 * 60_000, "daily"));
        assert!(due_triggers(&c, &st, NOON, 0).is_empty());
        assert_eq!(next_run_at(&c, &st, NOON, 0), Some(NOON - 3 * H + 24 * H));
        // Weekdays: Wednesday is 3; a Mon/Fri bot waits for Friday (2 days later).
        let mf = cfg(json!([{ "type": "daily", "at": "09:00", "days": [1, 5] }]));
        assert!(due_triggers(&mf, &empty_state(), NOON, 0).is_empty());
        assert_eq!(next_run_at(&mf, &empty_state(), NOON, 0), Some(NOON - 3 * H + 2 * 24 * H));
        let wed = cfg(json!([{ "type": "daily", "at": "09:00", "days": [3] }]));
        assert_eq!(due_triggers(&wed, &empty_state(), NOON, 0).len(), 1);
        // Grace: three hours late with a 30 minute grace the slot is dropped; inside it, it fires.
        let strict = cfg(json!([{ "type": "daily", "at": "09:00", "grace_minutes": 30 }]));
        assert!(due_triggers(&strict, &empty_state(), NOON, 0).is_empty());
        assert_eq!(due_triggers(&strict, &empty_state(), NOON - 3 * H + 10 * 60_000, 0).len(), 1);
    }

    #[test]
    fn a_github_trigger_waits_its_poll_interval() {
        let c = cfg(json!([{ "type": "github", "query": "is:pr", "every_minutes": 10 }]));
        assert_eq!(due_triggers(&c, &empty_state(), NOON, 0).len(), 1);
        let mut st = empty_state();
        st["last_poll"]["github"] = json!(iso_ms(NOON - 4 * 60_000));
        assert!(due_triggers(&c, &st, NOON, 0).is_empty());
        assert_eq!(next_run_at(&c, &st, NOON, 0), Some(NOON + 6 * 60_000));
        assert_eq!(next_run_at(&cfg(json!([{ "type": "manual" }])), &st, NOON, 0), None);
    }

    #[test]
    fn times_are_told_against_now() {
        assert_eq!(rel_time(NOON + 20_000, NOON), ("now", 0, true));
        assert_eq!(rel_time(NOON - 5 * 60_000, NOON), ("min", 5, false));
        assert_eq!(rel_time(NOON + 2 * H, NOON), ("hour", 2, true));
        assert_eq!(rel_time(NOON - 3 * 24 * H, NOON), ("day", 3, false));
    }

    #[test]
    fn the_history_keeps_the_last_hundred_runs() {
        let mut st = empty_state();
        for i in 0..105 {
            st = with_run(st, run(i, NOON, "manual"));
        }
        assert_eq!(runs(&st).len(), 100);
        assert_eq!(runs(&st)[0]["id"], json!(5));
    }

    #[test]
    fn the_card_names_the_schedule_and_the_weekdays() {
        assert_eq!(schedule_of(&json!([{ "type": "manual" }])), None);
        let s = schedule_of(&json!([{ "type": "github", "every_minutes": 15 }])).unwrap();
        assert_eq!((s["type"].as_str(), s["minutes"].as_i64()), (Some("github"), Some(15)));
        let names: Vec<String> = ["dom", "lun", "mar", "mié", "jue", "vie", "sáb"].map(String::from).to_vec();
        assert_eq!(days_text(&json!([5, 1, 1]), &names), "lun, vie");
        assert_eq!(days_text(&json!([0, 1, 2, 3, 4, 5, 6]), &names), "");
        assert_eq!(days_text(&Value::Null, &names), "");
    }

    #[test]
    fn run_now_says_why_it_would_be_refused() {
        let mut bot = Bot { name: "x".into(), soul: String::new(), config: cfg(json!([{ "type": "manual" }])), state: empty_state() };
        assert_eq!(why_not(&bot, &[], NOON, 0), None);
        bot.state = with_run(empty_state(), run(1, NOON - H, "manual"));
        let working = vec![json!({ "tab_name": "bot x #1", "status": "working" })];
        assert_eq!(why_not(&bot, &working, NOON, 0), Some("bots.why_busy"));
        assert_eq!(live_state(&bot, &working), "working");
        assert_eq!(live_state(&bot, &[]), "idle");
        bot.config["max_runs_per_day"] = json!(1);
        assert_eq!(why_not(&bot, &[], NOON, 0), Some("bots.why_cap"));
        bot.config["enabled"] = json!(false);
        assert_eq!(live_state(&bot, &[]), "off");
    }

    #[test]
    fn the_mark_letter_reads_on_every_color() {
        for fill in MARK {
            let rgb = [((fill >> 16) & 255) as f32 / 255.0, ((fill >> 8) & 255) as f32 / 255.0, (fill & 255) as f32 / 255.0];
            let text = if text_on(fill) == gpui::black() { [0.0; 3] } else { [1.0; 3] };
            assert!(contrast(rgb, text) >= 4.5, "{fill:x}");
        }
    }
}
