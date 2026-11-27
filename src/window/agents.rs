//! The agents list (`{"native": "agents"}`, once the `agents` mod): every agent by what it needs,
//! in sections (Needs you, Done, Working, Idle) or grouped by space, pinned ones on top,
//! sub-agents under their parent with tree lines, the time each has been in its state, and a pin
//! on hover. The window also keeps the log the Activity panel shows (`activity.events`, newest first):
//! a change of state is news when an agent needs you or stops working. What is decided (order,
//! groups, captions, tree, events) is plain functions, tested at the end; the block only draws them.
//!
//! The state the mod kept in its `localStorage` (`data/agents/store.json`: `changed`, `pins`) is read
//! and written in the same file and format.

use super::native::{icon, r, tip, tone, z};
use super::spaces::{click, hf, items, opt_text, sid, text, vf};
use super::*;
use crate::i18n;
use std::collections::BTreeSet;

macro_rules! chrome_icon {
    ($name:literal) => {
        include_bytes!(concat!("../../defaults/mods/workspace-chrome/", $name, ".svg")).as_slice()
    };
}
const SPINNER: &[u8] = chrome_icon!("spinner");
const CHEVRON_DOWN: &[u8] = chrome_icon!("chevron-down");
const CHEVRON_RIGHT: &[u8] = chrome_icon!("chevron-right");
const PIN: &[u8] = chrome_icon!("pin");
const PIN_OFF: &[u8] = chrome_icon!("pin-off");
const CLOCK: &[u8] = chrome_icon!("clock");
const LAYERS: &[u8] = chrome_icon!("layers");
const CHECK: &[u8] = chrome_icon!("check");

const ROW_H: f32 = 32.0;
const MAX_EVENTS: usize = 100;
const OLD_MS: u64 = 30 * 86_400_000;

// Pure rules ----------------------------------------------------------------------------------

/// How urgent a status is: needs you > done > working > idle; anything else (no status yet) is 0.
pub(super) fn rank(status: &str) -> u8 {
    match status {
        "blocked" => 4,
        "done" => 3,
        "working" => 2,
        "idle" => 1,
        _ => 0,
    }
}

/// A sub-agent without a pane is not in the agents list: its own word, an error counting as blocked.
/// It is never done (it reports to its parent, whose done is the news).
pub(super) fn kid_status<'a>(raw: &'a str, listed: Option<&'a str>) -> &'a str {
    match listed.unwrap_or(raw) {
        "error" => "blocked",
        "done" => "idle",
        s => s,
    }
}

/// What a change of status is news for: "blocked" (needs you) or "done" (it stopped working).
pub(super) fn event_for(before: Option<&str>, after: &str) -> Option<&'static str> {
    let before = before?;
    if before == after {
        return None;
    }
    match (before, after) {
        (_, "blocked") => Some("blocked"),
        ("working", "idle" | "done") => Some("done"),
        _ => None,
    }
}

/// The log with `event` on top, capped.
pub(super) fn with_event(mut events: Vec<Value>, event: Value) -> Vec<Value> {
    events.insert(0, event);
    events.truncate(MAX_EVENTS);
    events
}

/// Events newer than the moment the log was last looked at.
pub(super) fn unread(events: &[Value], seen: u64) -> usize {
    events.iter().filter(|e| e.get("at").and_then(Value::as_u64).unwrap_or(0) > seen).count()
}

/// "<1m", "5m", "3h", "2d": how long an agent has been in its state.
pub(super) fn duration(ms: u64) -> String {
    let m = ms / 60_000;
    match m {
        0 => "<1m".into(),
        1..60 => format!("{m}m"),
        _ => {
            let h = m / 60;
            if h < 24 { format!("{h}h") } else { format!("{}d", h / 24) }
        }
    }
}

/// "now", "5m", "3h", "2d" for an event in the log.
pub(super) fn since_text(at: u64, now: u64) -> Option<String> {
    let m = now.saturating_sub(at) / 60_000;
    Some(match m {
        0 => return None,
        1..60 => format!("{m}m"),
        60..1440 => format!("{}h", m / 60),
        _ => format!("{}d", m / 1440),
    })
}

/// The agent's last state and when it changed; `done` and `idle` count as one (looking at a finished
/// agent turns done into idle, and looking is not the agent doing anything).
#[derive(Clone, Debug, PartialEq)]
pub(super) struct Stamp {
    pub s: String,
    pub at: u64,
}

pub(super) fn stamp_word(status: &str) -> &str {
    if status == "done" { "idle" } else { status }
}

/// Stamps the agents whose state changed; returns whether anything did and the news (events, oldest first).
/// Entries gone for over 30 days are dropped, but only when the list is not empty (right after a start it is).
pub(super) fn track(list: &[Value], now: u64, stamps: &mut HashMap<String, Stamp>) -> (bool, Vec<Value>) {
    let (mut dirty, mut news) = (false, Vec::new());
    for a in list {
        let address = text(a, "address");
        let word = stamp_word(text(a, "status")).to_owned();
        if stamps.get(address).is_some_and(|s| s.s == word) {
            continue;
        }
        if let Some(kind) = event_for(stamps.get(address).map(|s| s.s.as_str()), &word) {
            let name = [text(a, "space_name"), text(a, "tab_name"), address].into_iter().find(|s| !s.is_empty()).unwrap_or_default();
            news.push(json!({ "kind": kind, "address": address, "name": name, "at": now }));
        }
        stamps.insert(address.to_owned(), Stamp { s: word, at: now });
        dirty = true;
    }
    if dirty && !list.is_empty() {
        let listed: BTreeSet<&str> = list.iter().map(|a| text(a, "address")).collect();
        stamps.retain(|k, s| listed.contains(k.as_str()) || now.saturating_sub(s.at) <= OLD_MS);
    }
    (dirty, news)
}

/// A pane running a known agent that is not in the agents list: the shape of an agent, status "none".
pub(super) fn unreported(list: &[Value], panes: &[Value], spaces: &[Value]) -> Vec<Value> {
    let known: BTreeSet<&str> = list.iter().map(|a| text(a, "address")).collect();
    let mut where_tab: HashMap<&str, (&Value, &Value)> = HashMap::new();
    for sp in spaces {
        for tab in items(sp, "tabs") {
            where_tab.insert(text(tab, "id"), (sp, tab));
        }
    }
    panes
        .iter()
        .filter(|p| p.get("terminal").is_some_and(|t| t.is_string()) && p.get("status").is_none_or(Value::is_null) && p["program"]["agent"] == true && !known.contains(text(p, "terminal")) && where_tab.contains_key(text(p, "tab")))
        .map(|p| {
            let (sp, tab) = where_tab[text(p, "tab")];
            json!({
                "address": p["terminal"], "pane": p["pane"], "space": sp["id"], "space_name": sp["name"], "tab": tab["id"], "tab_index": tab["index"], "tab_name": tab["name"],
                "status": "none", "app": p["app"], "program": p["program"], "title": p["title"], "seq": 0, "pinned": false, "focused": p["focused"], "unreported": true,
            })
        })
        .collect()
}

/// The pane that speaks for a tab: its first agent, else its first focused terminal, else its first terminal.
pub(super) fn lead_panes(panes: &[Value]) -> HashMap<String, &Value> {
    let mut leads: HashMap<String, &Value> = HashMap::new();
    for p in panes.iter().filter(|p| p.get("terminal").is_some_and(Value::is_string)) {
        let tab = text(p, "tab").to_owned();
        let take = match leads.get(&tab) {
            None => true,
            Some(cur) => cur["program"]["agent"] != true && (p["program"]["agent"] == true || (cur["focused"] != true && p["focused"] == true)),
        };
        if take {
            leads.insert(tab, p);
        }
    }
    leads
}

/// A tab's caption: its own name, or what its lead agent says it is doing when the name is automatic.
pub(super) fn title(tab_name: &str, named: bool, lead: Option<&Value>) -> String {
    let prog = lead.and_then(|l| opt_text(&l["program"], "name"));
    let raw = lead.map_or("", |l| text(l, "title"));
    let live = super::tabs::clean(raw);
    // clean() would strip a path's leading ~ or /: test the title with only its status glyph gone.
    let bare = raw.trim_start_matches(|c: char| !(c.is_alphanumeric() || c == '~' || c == '/'));
    let drive = {
        let b: Vec<char> = raw.chars().take(3).collect();
        b.len() == 3 && b[0].is_ascii_alphabetic() && b[1] == ':' && b[2] == '\\'
    };
    let path = bare.starts_with(['~', '/']) || drive;
    let terminal = lead.map_or("", |l| opt_text(l, "terminal").or_else(|| opt_text(l, "address")).unwrap_or_default());
    let key = lead.map_or("", |l| text(&l["program"], "key"));
    let telling = !live.is_empty() && live != terminal && Some(live.as_str()) != prog && !path && !live.contains('@') && live.to_lowercase() != key;
    if (Some(tab_name) == prog || !named) && telling { live } else { tab_name.to_owned() }
}

fn digits(s: &str) -> bool {
    !s.is_empty() && s.chars().all(|c| c.is_ascii_digit())
}

/// Each agent's caption and detail. A row reads "space · detail": the detail is a tab someone named or the
/// agent's own title, never an automatic tab number unless two rows would otherwise read the same.
pub(super) fn captions(list: &[Value], spaces: &[Value], panes: &[Value]) -> HashMap<String, (String, Option<String>)> {
    let tabs: HashMap<&str, &Value> = spaces.iter().flat_map(|s| items(s, "tabs")).map(|t| (text(t, "id"), t)).collect();
    let leads = lead_panes(panes);
    let mut per_tab: HashMap<&str, usize> = HashMap::new();
    for a in list {
        *per_tab.entry(text(a, "tab")).or_default() += 1;
    }
    let labels: Vec<String> = list
        .iter()
        .map(|a| match tabs.get(text(a, "tab")) {
            Some(tab) => {
                let many = per_tab.get(text(a, "tab")).copied().unwrap_or(0) > 1;
                let named = !many && tab["named"] == true;
                title(text(tab, "name"), named, if many { Some(a) } else { leads.get(text(a, "tab")).copied() })
            }
            None => [text(a, "title"), text(&a["program"], "name")].into_iter().find(|s| !s.is_empty()).unwrap_or_default().to_owned(),
        })
        .collect();
    let mut detail: Vec<Option<String>> = labels.iter().map(|l| (!digits(l)).then(|| l.clone())).collect();
    let mut seen: HashMap<(String, String), usize> = HashMap::new();
    for (a, d) in list.iter().zip(&detail) {
        *seen.entry((text(a, "space_name").to_owned(), d.clone().unwrap_or_default())).or_default() += 1;
    }
    for (i, a) in list.iter().enumerate() {
        if detail[i].is_none() && seen.get(&(text(a, "space_name").to_owned(), String::new())).copied().unwrap_or(0) > 1 {
            detail[i] = Some(labels[i].clone());
        }
    }
    list.iter().zip(labels.into_iter().zip(detail)).map(|(a, l)| (text(a, "address").to_owned(), l)).collect()
}

/// The name a row shows: the space when no space header is above it (priority, pinned), else its detail.
pub(super) fn row_name(a: &Value, label: &str, detail: Option<&str>, full: bool) -> String {
    if full {
        return [text(a, "space_name"), label].into_iter().find(|s| !s.is_empty()).unwrap_or_default().to_owned();
    }
    match detail {
        Some(d) => d.to_owned(),
        None if digits(label) => [text(&a["program"], "name"), label].into_iter().find(|s| !s.is_empty()).unwrap_or_default().to_owned(),
        None => label.to_owned(),
    }
}

/// Live sub-agents of `addr`, depth first, each with the tree lines in front of it (`├ └ │`).
pub(super) fn tree<'a>(terminals: &'a [Value], addr: &str, lead: &[&'static str]) -> Vec<(&'a Value, Vec<&'static str>)> {
    let kids: Vec<&Value> = terminals.iter().filter(|k| text(k, "parent") == addr && text(k, "state") == "live").collect();
    let mut out = Vec::new();
    for (i, k) in kids.iter().enumerate() {
        let last = i + 1 == kids.len();
        let mut mine = lead.to_vec();
        mine.push(if last { "\u{2514}" } else { "\u{251c}" });
        out.push((*k, mine));
        let mut next = lead.to_vec();
        next.push(if last { "" } else { "\u{2502}" });
        out.extend(tree(terminals, text(k, "address"), &next));
    }
    out
}

#[derive(Clone, Debug, PartialEq)]
pub(super) enum Heading {
    Pinned,
    Section(u8),
    Space(String),
}

#[derive(Clone, Debug, PartialEq)]
pub(super) struct Group {
    pub id: String,
    pub heading: Heading,
    pub members: Vec<usize>,
}

/// The list's order (indexes into `list`) and its groups. By priority: most urgent first, then the most recent
/// change, then the newest agent; grouped: as they come, a space's agents together. Pinned ones lead either way.
pub(super) fn arrange(list: &[Value], urgency: &HashMap<String, u8>, since: &HashMap<String, u64>, by_priority: bool) -> (Vec<usize>, Vec<Group>) {
    let u = |i: usize| urgency.get(text(&list[i], "address")).copied().unwrap_or(0);
    let at = |i: usize| since.get(text(&list[i], "address")).copied().unwrap_or(0);
    let seq = |i: usize| list[i].get("seq").and_then(Value::as_u64).unwrap_or(0);
    let mut idx: Vec<usize> = (0..list.len()).collect();
    if by_priority {
        idx.sort_by(|&x, &y| u(y).cmp(&u(x)).then(at(y).cmp(&at(x))).then(seq(y).cmp(&seq(x))));
    }
    let pinned = |i: usize| list[i]["pinned"] == true;
    let (head, rest): (Vec<usize>, Vec<usize>) = idx.into_iter().partition(|&i| pinned(i));
    let mut groups = Vec::new();
    if !head.is_empty() {
        groups.push(Group { id: "pinned".into(), heading: Heading::Pinned, members: head.clone() });
    }
    for &i in &rest {
        let (id, heading) = if by_priority {
            let r = u(i).max(1);
            (format!("sec-{r}"), Heading::Section(r))
        } else {
            (format!("sp-{}", text(&list[i], "space")), Heading::Space(text(&list[i], "space_name").to_owned()))
        };
        match groups.last_mut() {
            Some(g) if g.id == id => g.members.push(i),
            _ => groups.push(Group { id, heading, members: vec![i] }),
        }
    }
    let order = head.into_iter().chain(rest).collect();
    (order, groups)
}

/// What the person chose in the sort menu.
fn by_priority() -> bool {
    mods::shared("agents.sort").and_then(|v| v.as_str().map(|s| s != "grouped")).unwrap_or(true)
}

/// The agents the list shows: reported ones, then panes running a known agent with no report yet.
pub(super) fn all_agents(agents: &[Value], panes: &[Value], spaces: &[Value]) -> Vec<Value> {
    let extra = unreported(agents, panes, spaces);
    agents.iter().cloned().chain(extra).collect()
}

// State ------------------------------------------------------------------------------------------

struct State {
    folded: HashSet<String>,
    hovered: Option<String>,
    stamps: HashMap<String, Stamp>,
    loaded: bool,
    started: bool,
    pending_pins: Vec<String>,
    migrating: HashSet<String>,
    sort_open: bool,
    scroll: ScrollHandle,
}

thread_local! {
    static ST: RefCell<State> = RefCell::new(State {
        folded: HashSet::new(), hovered: None, stamps: HashMap::new(), loaded: false, started: false, pending_pins: Vec::new(), migrating: HashSet::new(), sort_open: false, scroll: ScrollHandle::new(),
    });
}

fn with<R>(f: impl FnOnce(&mut State) -> R) -> R {
    ST.with_borrow_mut(f)
}

fn store_path() -> PathBuf {
    mods::app_dir().join("data").join("agents").join("store.json")
}

fn store_get(key: &str) -> Option<String> {
    let file: Value = serde_json::from_str(&std::fs::read_to_string(store_path()).ok()?).ok()?;
    file.get(key)?.as_str().map(str::to_owned)
}

fn store_set(key: &str, value: Option<String>) {
    let path = store_path();
    let mut file = std::fs::read_to_string(&path).ok().and_then(|t| serde_json::from_str::<Value>(&t).ok()).filter(Value::is_object).unwrap_or_else(|| json!({}));
    match value {
        Some(v) => file[key] = json!(v),
        None => {
            file.as_object_mut().map(|o| o.remove(key));
        }
    }
    if let Some(dir) = path.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    let _ = std::fs::write(path, serde_json::to_string_pretty(&file).unwrap_or_default());
}

fn parse_stamps(raw: &str) -> HashMap<String, Stamp> {
    let Ok(Value::Object(map)) = serde_json::from_str::<Value>(raw) else { return HashMap::new() };
    map.into_iter().filter_map(|(k, v)| Some((k, Stamp { s: v.get("s")?.as_str()?.to_owned(), at: v.get("at")?.as_u64()? }))).collect()
}

fn stamps_json(stamps: &HashMap<String, Stamp>) -> String {
    Value::Object(stamps.iter().map(|(k, s)| (k.clone(), json!({ "s": s.s, "at": s.at }))).collect()).to_string()
}

fn read_events() -> Vec<Value> {
    mods::shared("activity.events").and_then(|v| v.as_array().cloned()).unwrap_or_default()
}

// The window's side ---------------------------------------------------------------------------

impl AppWindow {
    /// Once a frame: stamps the agents whose state changed (and logs what is news for the Activity panel),
    /// moves the pins an older version kept in a file onto the terminals, and keeps `agent.next` in the list's order.
    pub(super) fn sync_agents(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let wants = self.chrome.walk().iter().any(|n| n.content().and_then(|c| self.native_of(c)).is_some_and(|(x, _)| x == "agents" || x == "activity"));
        if !wants {
            return;
        }
        let (agents, panes, spaces, terminals) = mods::snapshot_with(|s| {
            let list = |v: &Value| v.as_array().cloned().unwrap_or_default();
            (list(&s.agents), list(&s.panes), list(&s.spaces), list(&s.terminals))
        });
        let list = all_agents(&agents, &panes, &spaces);
        let now = model::now_ms();
        let (news, dirty_stamps, start) = with(|s| {
            if !std::mem::replace(&mut s.loaded, true) {
                s.stamps = store_get("changed").map(|r| parse_stamps(&r)).unwrap_or_default();
                s.pending_pins = store_get("pins").and_then(|r| serde_json::from_str::<Vec<String>>(&r).ok()).unwrap_or_default();
            }
            let (dirty, news) = track(&list, now, &mut s.stamps);
            if dirty {
                store_set("changed", Some(stamps_json(&s.stamps)));
            }
            (news, dirty, !std::mem::replace(&mut s.started, true))
        });
        let _ = dirty_stamps;
        if !news.is_empty() {
            let feed = news.into_iter().fold(read_events(), with_event);
            mods::set_shared("activity.events".into(), Value::Array(feed));
        }
        if start {
            cx.spawn(async move |this, cx| loop {
                cx.background_executor().timer(Duration::from_secs(30)).await;
                if this.update(cx, |_, cx| cx.notify()).is_err() {
                    break;
                }
            })
            .detach();
        }
        self.migrate_pins(&terminals, window, cx);
    }

    /// Pins kept offline stay until their terminal returns; each is pinned on the terminal and kept until the core echoes the label.
    fn migrate_pins(&mut self, terminals: &[Value], window: &mut Window, cx: &mut Context<Self>) {
        let pending = with(|s| s.pending_pins.clone());
        if pending.is_empty() {
            return;
        }
        let mut keep = Vec::new();
        let mut to_pin = Vec::new();
        for address in &pending {
            let Some(t) = terminals.iter().find(|t| text(t, "address") == address && text(t, "state") == "live") else {
                keep.push(address.clone());
                continue;
            };
            if t["labels"]["herdr.pinned"].as_str().is_some_and(|v| !v.is_empty()) {
                continue;
            }
            if with(|s| s.migrating.insert(address.clone())) {
                to_pin.push(address.clone());
            }
            keep.push(address.clone());
        }
        for address in to_pin {
            self.run("terminal.pin", json!({ "terminal": address, "pinned": true }), window, cx);
        }
        if keep.len() != pending.len() {
            store_set("pins", (!keep.is_empty()).then(|| json!(keep).to_string()));
            with(|s| s.pending_pins = keep);
        }
    }

    pub(super) fn native_agents(&self, cx: &mut Context<Self>) -> AnyElement {
        let (agents, panes, spaces, terminals) = mods::snapshot_with(|s| {
            let list = |v: &Value| v.as_array().cloned().unwrap_or_default();
            (list(&s.agents), list(&s.panes), list(&s.spaces), list(&s.terminals))
        });
        let priority = by_priority();
        let now = model::now_ms();
        let listed_status: HashMap<String, String> = all_agents(&agents, &panes, &spaces).iter().map(|a| (text(a, "address").to_owned(), text(a, "status").to_owned())).collect();
        let mut list = all_agents(&agents, &panes, &spaces);
        // A kept sub-agent (one with a pane) is in the list too: where a tree shows it, it has no row, unless pinned.
        let nested: HashSet<String> = list.iter().flat_map(|a| tree(&terminals, text(a, "address"), &[]).into_iter().map(|(k, _)| text(k, "address").to_owned())).collect();
        list.retain(|a| a["pinned"] == true || !nested.contains(text(a, "address")));
        let urgency: HashMap<String, u8> = list
            .iter()
            .map(|a| {
                let own = rank(text(a, "status"));
                let kids = tree(&terminals, text(a, "address"), &[]).into_iter().map(|(k, _)| rank(kid_status(text(k, "status"), listed_status.get(text(k, "address")).map(String::as_str)))).max().unwrap_or(0);
                (text(a, "address").to_owned(), own.max(kids))
            })
            .collect();
        let (hovered, stamps, folded, sort_open, scroll) = with(|s| (s.hovered.clone(), s.stamps.clone(), s.folded.clone(), s.sort_open, s.scroll.clone()));
        let since: HashMap<String, u64> = stamps.iter().map(|(k, s)| (k.clone(), s.at)).collect();
        let (order, groups) = arrange(&list, &urgency, &since, priority);
        mods::set_agent_order(order.iter().map(|&i| text(&list[i], "address").to_owned()).collect());
        let caps = captions(&list, &spaces, &panes);
        let focused_agent = mods::snapshot_with(|s| opt_text(&s.focused, "agent").map(str::to_owned));

        let mut rows: Vec<AnyElement> = Vec::new();
        for g in &groups {
            let open = !folded.contains(&g.id);
            rows.push(self.group_header(g, &list, open, priority, cx));
            if !open {
                continue;
            }
            for &i in &g.members {
                let a = &list[i];
                let subs = tree(&terminals, text(a, "address"), &[]);
                let caption = caps.get(text(a, "address")).cloned().unwrap_or_default();
                let full = priority || a["pinned"] == true;
                rows.push(self.agent_row(a, &caption, full, &subs, &listed_status, &stamps, hovered.as_deref() == Some(text(a, "address")), folded.contains(text(a, "address")), now, priority && a["pinned"] != true, cx));
                if !folded.contains(text(a, "address")) {
                    for (k, lines) in &subs {
                        rows.push(self.sub_row(k, lines, &listed_status, focused_agent.as_deref(), cx));
                    }
                }
            }
        }
        let empty = list.is_empty();
        vf()
            .size_full()
            .min_h(px(0.0))
            .bg(c("chrome"))
            .text_size(z(13.0))
            .child(super::native::sidebar_separator())
            .child(self.agents_header(priority, sort_open, cx))
            .when(sort_open, |d| d.child(self.sort_menu(priority, cx)))
            .when(empty, |d| d.child(div().px(z(16.0)).py(z(8.0)).text_size(z(12.0)).text_color(tone("dim")).child(i18n::t("agents.no_agent_status_yet_agents"))))
            .child(vf().id("ag-list").flex_1().min_h(px(0.0)).px(z(8.0)).pb(z(8.0)).overflow_y_scroll().track_scroll(&scroll).children(rows))
            .into_any_element()
    }

    fn agents_header(&self, priority: bool, open: bool, cx: &mut Context<Self>) -> AnyElement {
        let sort = i18n::t(if priority { "agents.priority" } else { "agents.grouped_by_space" });
        hf().h(z(ROW_H + 8.0))
            .flex_none()
            .pl(z(18.0))
            .pr(z(8.0))
            .pt(z(6.0))
            .justify_between()
            .child(div().text_size(z(11.5)).font_weight(FontWeight::MEDIUM).text_color(tone("dim")).child(i18n::t("agents.agents")))
            .child(
                hf().id("ag-sort")
                    .size(z(ROW_H))
                    .justify_center()
                    .rounded(r(6.0))
                    .cursor_pointer()
                    .border_2()
                    .border_color(Hsla { a: 0.0, ..c("accent") })
                    .when(open, |d| d.bg(c("hover")))
                    .hover(|s| s.bg(c("hover")))
                    .focusable()
                    .tab_stop(true)
                    .focus_visible(|s| s.border_color(graphic("accent")))
                    .tooltip(tip(i18n::tf("agents.sort_agents_sort", &[("sort", &sort)])))
                    .on_key_down(cx.listener(|_, e: &KeyDownEvent, _, cx| {
                        if matches!(e.keystroke.key.as_str(), "enter" | "space") {
                            with(|s| s.sort_open = !s.sort_open);
                            cx.stop_propagation();
                            cx.notify();
                        }
                    }))
                    .on_click(click(cx, |_, _, cx| {
                        with(|s| s.sort_open = !s.sort_open);
                        cx.notify();
                    }))
                    .child(icon(if priority { CLOCK } else { LAYERS }, 14.0, tone("muted"))),
            )
            .into_any_element()
    }

    fn sort_menu(&self, priority: bool, cx: &mut Context<Self>) -> AnyElement {
        let option = |id: &'static str, key: &'static str, bytes: &'static [u8], on: bool, cx: &mut Context<Self>| {
            let pick = move |_: &mut AppWindow, _: &mut Window, cx: &mut Context<AppWindow>| {
                mods::set_shared("agents.sort".into(), json!(id));
                with(|s| s.sort_open = false);
                cx.notify();
            };
            hf().id(sid("sort", id))
                .h(z(ROW_H))
                .px(z(8.0))
                .gap(z(9.0))
                .rounded(r(7.0))
                .cursor_pointer()
                .text_size(z(12.5))
                .text_color(c("foreground"))
                .border_2()
                .border_color(Hsla { a: 0.0, ..c("accent") })
                .hover(|s| s.bg(c("hover")))
                .focusable()
                .tab_stop(true)
                .focus_visible(|s| s.border_color(graphic("accent")))
                .on_key_down(cx.listener(move |this, e: &KeyDownEvent, window, cx| {
                    if matches!(e.keystroke.key.as_str(), "enter" | "space") {
                        pick(this, window, cx);
                        cx.stop_propagation();
                    }
                }))
                .on_click(click(cx, pick))
                .child(icon(bytes, 14.0, tone("muted")))
                .child(div().flex_1().child(i18n::t(key)))
                .when(on, |d| d.child(icon(CHECK, 14.0, c("foreground"))))
        };
        vf().flex_none()
            .mx(z(8.0))
            .mb(z(6.0))
            .p(z(5.0))
            .rounded(r(8.0))
            .bg(c("popover"))
            .border_1()
            .border_color(c("border"))
            .child(div().px(z(8.0)).pt(z(4.0)).pb(z(6.0)).text_size(z(11.0)).text_color(tone("muted")).child(i18n::t("agents.sort_agents")))
            .child(option("priority", "agents.priority", CLOCK, priority, cx))
            .child(option("grouped", "agents.grouped_by_space", LAYERS, !priority, cx))
            .into_any_element()
    }

    fn group_header(&self, g: &Group, list: &[Value], open: bool, priority: bool, cx: &mut Context<Self>) -> AnyElement {
        let (label, color) = match &g.heading {
            Heading::Pinned => (i18n::t("agents.pinned"), tone("dim")),
            Heading::Section(r) => (
                i18n::t(match r {
                    4 => "agents.needs_you",
                    3 => "agents.done",
                    2 => "agents.working",
                    _ => "agents.idle",
                }),
                match r {
                    4 => tone("blocked"),
                    3 => tone("done"),
                    2 => tone("working"),
                    _ => tone("dim"),
                },
            ),
            Heading::Space(name) => (name.clone(), tone("dim")),
        };
        let top = g.members.iter().map(|&i| text(&list[i], "status")).max_by_key(|s| rank(s)).unwrap_or_default().to_owned();
        let id = g.id.clone();
        let toggle = move |_: &mut AppWindow, _: &mut Window, cx: &mut Context<AppWindow>| {
            with(|s| {
                if !s.folded.remove(&id) {
                    s.folded.insert(id.clone());
                }
            });
            cx.notify();
        };
        let key_toggle = toggle.clone();
        hf().id(sid("ag-group", &g.id))
            .h(z(ROW_H))
            .mt(z(6.0))
            .px(z(8.0))
            .gap(z(6.0))
            .flex_none()
            .rounded(r(7.0))
            .text_size(z(11.0))
            .font_weight(FontWeight::MEDIUM)
            .cursor_pointer()
            .border_2()
            .border_color(Hsla { a: 0.0, ..c("accent") })
            .hover(|s| s.bg(c("hover")))
            .focusable()
            .tab_stop(true)
            .focus_visible(|s| s.border_color(graphic("accent")))
            .on_key_down(cx.listener(move |this, e: &KeyDownEvent, window, cx| {
                if matches!(e.keystroke.key.as_str(), "enter" | "space" | "left" | "right") {
                    key_toggle(this, window, cx);
                    cx.stop_propagation();
                }
            }))
            .on_click(click(cx, toggle))
            .child(div().truncate().text_color(color).child(label))
            .when(priority || !open, |d| d.child(div().text_color(tone("dim")).child(g.members.len().to_string())))
            .child(div().flex_1())
            .when(!open && !priority && rank(&top) > 1, |d| d.child(status_mark(&top, true)))
            .child(icon(if open { CHEVRON_DOWN } else { CHEVRON_RIGHT }, 11.0, tone("dim")))
            .into_any_element()
    }

    #[allow(clippy::too_many_arguments)]
    fn agent_row(&self, a: &Value, caption: &(String, Option<String>), full: bool, subs: &[(&Value, Vec<&'static str>)], listed: &HashMap<String, String>, stamps: &HashMap<String, Stamp>, hovered: bool, folded: bool, now: u64, compact: bool, cx: &mut Context<Self>) -> AnyElement {
        let address = text(a, "address").to_owned();
        let pane = text(a, "pane").to_owned();
        let pinned = a["pinned"] == true;
        let name = row_name(a, &caption.0, caption.1.as_deref(), full);
        let more = caption.1.clone().filter(|_| full);
        let idle = text(a, "status") == "idle" && a["focused"] != true;
        let machine = address.split_once('/').map(|(m, _)| m.to_owned());
        let when = stamps.get(&address).map_or(0, |s| s.at);
        let top = subs.iter().map(|(k, _)| kid_status(text(k, "status"), listed.get(text(k, "address")).map(String::as_str))).max_by_key(|s| rank(s)).unwrap_or_default().to_owned();
        let focus_pane = {
            let pane = pane.clone();
            move |this: &mut AppWindow, window: &mut Window, cx: &mut Context<AppWindow>| this.run("pane.focus", json!({ "pane": pane.clone() }), window, cx)
        };
        let fold_id = address.clone();
        let toggle_fold = move |cx: &mut Context<AppWindow>| {
            with(|s| {
                if !s.folded.remove(&fold_id) {
                    s.folded.insert(fold_id.clone());
                }
            });
            cx.notify();
        };
        let (pin_address, key_address) = (address.clone(), address.clone());
        let pin_button = hf()
            .id(sid("pin", &address))
            .size(z(ROW_H - 4.0))
            .flex_none()
            .justify_center()
            .rounded(r(5.0))
            .cursor_pointer()
            .hover(|s| s.bg(c("hover")))
            .tooltip(tip(i18n::t(if pinned { "agents.unpin" } else { "agents.pin_to_the_top" })))
            .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
            .on_click(click(cx, move |this, window, cx| {
                this.run("terminal.pin", json!({ "terminal": pin_address.clone(), "pinned": !pinned }), window, cx);
                // The row moves away from under the pointer, and no hover-out comes.
                with(|s| s.hovered = None);
                cx.notify();
            }))
            .child(icon(if pinned { PIN_OFF } else { PIN }, 13.0, tone("dim")));
        let key_fold = toggle_fold.clone();
        let click_focus = focus_pane.clone();
        let hover_address = address.clone();
        hf().id(sid("ag", &address))
            .h(z(ROW_H))
            .w_full()
            .flex_none()
            .px(z(8.0))
            .gap(z(9.0))
            .rounded(r(8.0))
            .cursor_pointer()
            .border_2()
            .border_color(Hsla { a: 0.0, ..c("accent") })
            .when(a["focused"] == true, |d| d.bg(c("selected")))
            .hover(|s| s.bg(c("hover")))
            .focusable()
            .tab_stop(true)
            .focus_visible(|s| s.border_color(graphic("accent")))
            .on_hover(cx.listener(move |_, over: &bool, _, cx| {
                with(|s| {
                    if *over {
                        s.hovered = Some(hover_address.clone());
                    } else if s.hovered.as_deref() == Some(hover_address.as_str()) {
                        s.hovered = None;
                    }
                });
                cx.notify();
            }))
            .on_key_down(cx.listener(move |this, e: &KeyDownEvent, window, cx| {
                match e.keystroke.key.as_str() {
                    "enter" | "space" => focus_pane(this, window, cx),
                    "p" => this.run("terminal.pin", json!({ "terminal": key_address.clone(), "pinned": !pinned }), window, cx),
                    "left" | "right" => key_fold(cx),
                    _ => return,
                }
                cx.stop_propagation();
            }))
            .on_click(click(cx, click_focus))
            .child(program_glyph(&a["program"], 16.0))
            .child(
                hf().flex_1()
                    .min_w(px(0.0))
                    .gap(z(5.0))
                    .child(div().id("agent-name").min_w(px(0.0)).max_w(z(if more.is_some() { 120.0 } else { 200.0 })).truncate().text_color(if idle { tone("muted") } else { c("foreground") }).tooltip(tip(name.to_string())).child(name))
                    .when_some(more, |d, m| d.child(div().flex_1().min_w(px(0.0)).truncate().text_color(tone("dim")).child(format!("\u{b7} {m}")))),
            )
            .when_some(machine, |d, m| d.child(div().text_size(z(10.0)).text_color(tone("dim")).child(m)))
            .when(!subs.is_empty(), |d| {
                let count = subs.len();
                d.child(
                    hf().id(sid("fold", &address))
                        .h(z(ROW_H - 8.0))
                        .px(z(3.0))
                        .gap(z(3.0))
                        .rounded(r(4.0))
                        .text_size(z(11.0))
                        .text_color(if folded { tone(&top) } else { tone("dim") })
                        .cursor_pointer()
                        .hover(|s| s.bg(c("hover")))
                        .on_click(click(cx, move |_, _, cx| toggle_fold(cx)))
                        .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                        .child(icon(if folded { CHEVRON_RIGHT } else { CHEVRON_DOWN }, 11.0, if folded { tone(&top) } else { tone("dim") }))
                        .when(folded, |d| d.child(count.to_string())),
                )
            })
            .when(hovered, |d| d.child(pin_button))
            .when(!hovered && when > 0, |d| d.child(div().flex_none().text_size(z(11.0)).text_color(tone("dim")).child(duration(now.saturating_sub(when)))))
            .child(status_mark(text(a, "status"), compact))
            .into_any_element()
    }

    fn sub_row(&self, k: &Value, lines: &[&'static str], listed: &HashMap<String, String>, shown: Option<&str>, cx: &mut Context<Self>) -> AnyElement {
        let address = text(k, "address").to_owned();
        let status = kid_status(text(k, "status"), listed.get(&address).map(String::as_str)).to_owned();
        let name = [text(k, "name"), text(k, "template"), text(&k["program"], "name"), address.as_str()].into_iter().find(|s| !s.is_empty()).unwrap_or_default().to_owned();
        let on = shown == Some(address.as_str());
        let in_list = listed.contains_key(&address);
        let go = {
            let address = address.clone();
            move |this: &mut AppWindow, window: &mut Window, cx: &mut Context<AppWindow>| {
                let (cmd, args) = if in_list { ("pane.focus", json!({ "terminal": address.clone() })) } else { ("terminal.peek", json!({ "terminal": address.clone() })) };
                this.run(cmd, args, window, cx)
            }
        };
        let key_go = go.clone();
        hf().id(sid("sub", &address))
            .h(z(ROW_H))
            .w_full()
            .flex_none()
            .pl(z(30.0))
            .pr(z(8.0))
            .gap(z(6.0))
            .rounded(r(8.0))
            .cursor_pointer()
            .text_size(z(12.5))
            .border_2()
            .border_color(Hsla { a: 0.0, ..c("accent") })
            .when(on, |d| d.bg(c("raised")))
            .hover(|s| s.bg(c("hover")))
            .focusable()
            .tab_stop(true)
            .focus_visible(|s| s.border_color(graphic("accent")))
            .on_key_down(cx.listener(move |this, e: &KeyDownEvent, window, cx| {
                if matches!(e.keystroke.key.as_str(), "enter" | "space") {
                    key_go(this, window, cx);
                    cx.stop_propagation();
                }
            }))
            .on_click(click(cx, go))
            .child(div().flex_none().text_color(tone("dim")).child(lines.iter().map(|l| if l.is_empty() { "\u{a0}\u{a0}".to_owned() } else { format!("{l} ") }).collect::<String>()))
            .child(program_glyph(&k["program"], 14.0))
            .child(div().id("agent-row-name").flex_1().min_w(px(0.0)).truncate().text_color(if on { c("foreground") } else { tone("muted") }).tooltip(tip(name.to_string())).child(name))
            .child(status_mark(&status, false))
            .into_any_element()
    }
}

// Drawing helpers shared with the Activity panel --------------------------------------------------

/// The agent's icon from the app's program table: a one-color glyph or a picture; a gap when it has none.
pub(super) fn program_glyph(program: &Value, size: f32) -> AnyElement {
    match opt_text(program, "icon") {
        Some(path) if program["tint"] != false => svg().external_path(mods::app_dir().join(path).to_string_lossy().into_owned()).size(z(size)).flex_none().text_color(c("foreground")).into_any_element(),
        Some(path) => img(mods::app_dir().join(path)).w(z(size)).h(z(size)).flex_none().into_any_element(),
        None => div().size(z(size)).flex_none().into_any_element(),
    }
}

/// The state mark: a spinner while working, a `!` (or "Needs you" in full) when blocked, a ring when idle, a dot when done.
pub(super) fn status_mark(status: &str, compact: bool) -> AnyElement {
    match status {
        "working" => icon(SPINNER, 12.0, c("working")).mx(z(1.0)).with_animation(SharedString::from(format!("ag-spin-{}", rand_id())), Animation::new(Duration::from_millis(900)).repeat(), |s, delta| s.with_transformation(Transformation::rotate(percentage(delta)))).into_any_element(),
        "blocked" if compact => hf().size(z(14.0)).flex_none().rounded(r(4.0)).bg(Hsla { a: 0.15, ..c("blocked") }).justify_center().text_size(z(10.5)).font_weight(FontWeight::BOLD).text_color(tone("blocked")).child("!").into_any_element(),
        "blocked" => hf().flex_none().px(z(6.0)).h(z(18.0)).rounded(r(5.0)).bg(Hsla { a: 0.15, ..c("blocked") }).text_size(z(10.5)).font_weight(FontWeight::SEMIBOLD).text_color(tone("blocked")).child(i18n::t("agents.needs_you_2")).into_any_element(),
        "idle" => div().size(z(7.0)).mx(z(3.0)).flex_none().rounded(r(4.0)).border(z(1.5)).border_color(c("idle")).into_any_element(),
        "done" => div().size(z(7.0)).mx(z(3.0)).flex_none().rounded(r(4.0)).bg(c("done")).into_any_element(),
        _ => div().size(z(7.0)).mx(z(3.0)).flex_none().into_any_element(),
    }
}

/// A different id per mark so each spinner runs its own animation.
fn rand_id() -> u64 {
    thread_local!(static N: std::cell::Cell<u64> = const { std::cell::Cell::new(0) });
    N.with(|n| {
        n.set(n.get().wrapping_add(1) % 4096);
        n.get()
    })
}

#[cfg(test)]
mod tests {
    use super::{Heading, OLD_MS, Stamp, arrange, captions, duration, event_for, kid_status, lead_panes, parse_stamps, rank, row_name, since_text, stamps_json, title, track, tree, unread, unreported, with_event};
    use crate::window::spaces::text;
    use serde_json::{Value, json};
    use std::collections::HashMap;

    fn agent(address: &str, status: &str, space: &str) -> Value {
        json!({ "address": address, "pane": format!("p-{address}"), "space": space, "space_name": space, "tab": format!("tab-{address}"), "tab_name": "1", "status": status, "title": "", "seq": 0, "pinned": false, "program": { "name": "claude", "key": "claude", "agent": true } })
    }

    #[test]
    fn urgency_is_blocked_then_done_then_working_then_idle() {
        assert!(rank("blocked") > rank("done") && rank("done") > rank("working") && rank("working") > rank("idle") && rank("idle") > rank("none"));
        assert_eq!(rank("error"), 0);
    }

    #[test]
    fn a_sub_agent_is_never_done_and_an_error_needs_you() {
        assert_eq!(kid_status("working", Some("done")), "idle");
        assert_eq!(kid_status("error", None), "blocked");
        assert_eq!(kid_status("working", None), "working");
        assert_eq!(kid_status("idle", Some("working")), "working");
    }

    #[test]
    fn only_needing_you_or_stopping_work_is_an_event() {
        assert_eq!(event_for(None, "blocked"), None);
        assert_eq!(event_for(Some("working"), "blocked"), Some("blocked"));
        assert_eq!(event_for(Some("working"), "idle"), Some("done"));
        assert_eq!(event_for(Some("working"), "done"), Some("done"));
        assert_eq!(event_for(Some("idle"), "working"), None);
        assert_eq!(event_for(Some("blocked"), "blocked"), None);
        assert_eq!(event_for(Some("idle"), "blocked"), Some("blocked"));
    }

    #[test]
    fn the_log_keeps_the_newest_100_and_counts_what_is_newer_than_the_last_look() {
        let mut feed = Vec::new();
        for at in 1..=105u64 {
            feed = with_event(feed, json!({ "kind": "done", "address": "a", "name": "x", "at": at }));
        }
        assert_eq!(feed.len(), 100);
        assert_eq!(feed[0]["at"], 105);
        assert_eq!(unread(&feed, 100), 5);
        assert_eq!(unread(&feed, 0), 100);
        assert_eq!(unread(&[], 0), 0);
    }

    #[test]
    fn tracking_stamps_changes_logs_news_and_treats_done_as_idle() {
        let mut stamps = HashMap::new();
        let (dirty, news) = track(&[agent("t1", "working", "api")], 10, &mut stamps);
        assert!(dirty && news.is_empty());
        // Working to blocked is news; the stamp moves.
        let (_, news) = track(&[agent("t1", "blocked", "api")], 20, &mut stamps);
        assert_eq!(news, vec![json!({ "kind": "blocked", "address": "t1", "name": "api", "at": 20 })]);
        assert_eq!(stamps["t1"], Stamp { s: "blocked".into(), at: 20 });
        // Same state again: nothing.
        let (dirty, news) = track(&[agent("t1", "blocked", "api")], 30, &mut stamps);
        assert!(!dirty && news.is_empty());
        // Idle then done is one state: no stamp, no news.
        track(&[agent("t2", "idle", "web")], 40, &mut stamps);
        let (dirty, news) = track(&[agent("t2", "done", "web")], 50, &mut stamps);
        assert!(!dirty && news.is_empty());
        assert_eq!(stamps["t2"].at, 40);
    }

    #[test]
    fn old_stamps_of_agents_that_are_gone_are_dropped_but_not_right_after_a_start() {
        let mut stamps = HashMap::from([("gone".to_owned(), Stamp { s: "idle".into(), at: 0 })]);
        track(&[agent("t1", "idle", "a")], OLD_MS + 5, &mut stamps);
        assert!(!stamps.contains_key("gone"));
        let mut stamps = HashMap::from([("gone".to_owned(), Stamp { s: "idle".into(), at: 0 })]);
        track(&[], OLD_MS + 5, &mut stamps);
        assert!(stamps.contains_key("gone"));
    }

    #[test]
    fn durations_and_event_ages_read_in_the_largest_unit() {
        assert_eq!(duration(30_000), "<1m");
        assert_eq!(duration(5 * 60_000), "5m");
        assert_eq!(duration(3 * 3_600_000), "3h");
        assert_eq!(duration(50 * 3_600_000), "2d");
        assert_eq!(since_text(100, 100), None);
        assert_eq!(since_text(0, 90 * 60_000).as_deref(), Some("1h"));
        assert_eq!(since_text(0, 3 * 86_400_000).as_deref(), Some("3d"));
    }

    #[test]
    fn stamps_round_trip_through_the_file_the_mod_wrote() {
        let raw = r#"{"t1":{"s":"working","at":5},"bad":3}"#;
        let stamps = parse_stamps(raw);
        assert_eq!(stamps.len(), 1);
        assert_eq!(parse_stamps(&stamps_json(&stamps)), stamps);
        for raw in ["{{{not json", "null", "42", "\"old shape\"", "[]"] {
            assert!(parse_stamps(raw).is_empty(), "{raw}");
        }
    }

    #[test]
    fn by_priority_the_urgent_come_first_and_neighbours_share_a_section() {
        let list = vec![agent("a", "idle", "x"), agent("b", "blocked", "y"), agent("c", "working", "x"), agent("d", "blocked", "z")];
        let urgency: HashMap<String, u8> = list.iter().map(|a| (text(a, "address").to_owned(), rank(text(a, "status")))).collect();
        let since = HashMap::from([("b".to_owned(), 5u64), ("d".to_owned(), 9u64)]);
        let (order, groups) = arrange(&list, &urgency, &since, true);
        assert_eq!(order, [3, 1, 2, 0]);
        assert_eq!(groups.iter().map(|g| (g.id.as_str(), g.members.clone())).collect::<Vec<_>>(), [("sec-4", vec![3, 1]), ("sec-2", vec![2]), ("sec-1", vec![0])]);
        assert_eq!(groups[0].heading, Heading::Section(4));
    }

    #[test]
    fn grouped_by_space_keeps_the_order_and_pinned_ones_lead() {
        let mut list = vec![agent("a", "idle", "x"), agent("b", "working", "x"), agent("c", "idle", "y")];
        list[2]["pinned"] = json!(true);
        let (order, groups) = arrange(&list, &HashMap::new(), &HashMap::new(), false);
        assert_eq!(order, [2, 0, 1]);
        assert_eq!(groups.iter().map(|g| (g.id.as_str(), g.members.clone())).collect::<Vec<_>>(), [("pinned", vec![2]), ("sp-x", vec![0, 1])]);
        assert_eq!(groups[1].heading, Heading::Space("x".into()));
    }

    #[test]
    fn sub_agents_hang_under_their_parent_with_tree_lines() {
        let t = |address: &str, parent: &str, state: &str| json!({ "address": address, "parent": parent, "state": state });
        let all = vec![t("t2", "t1", "live"), t("t3", "t1", "live"), t("t4", "t2", "live"), t("t5", "t1", "finished")];
        let got: Vec<(String, Vec<&str>)> = tree(&all, "t1", &[]).into_iter().map(|(k, l)| (text(k, "address").to_owned(), l)).collect();
        assert_eq!(got, [("t2".to_owned(), vec!["\u{251c}"]), ("t4".to_owned(), vec!["\u{2502}", "\u{2514}"]), ("t3".to_owned(), vec!["\u{2514}"])]);
    }

    #[test]
    fn a_tab_caption_follows_the_agents_live_title_unless_the_person_named_the_tab() {
        let lead = json!({ "terminal": "t1", "title": "\u{2733} Fixing the login", "program": { "name": "Claude", "key": "claude" } });
        assert_eq!(title("1", false, Some(&lead)), "Fixing the login");
        assert_eq!(title("Claude", true, Some(&lead)), "Fixing the login");
        assert_eq!(title("api", true, Some(&lead)), "api");
        let path = json!({ "terminal": "t1", "title": "~/dev/app", "program": { "name": "Claude", "key": "claude" } });
        assert_eq!(title("1", false, Some(&path)), "1");
        let at = json!({ "terminal": "t1", "title": "me@host", "program": { "name": "Claude", "key": "claude" } });
        assert_eq!(title("1", false, Some(&at)), "1");
        assert_eq!(title("1", false, None), "1");
    }

    #[test]
    fn a_row_says_its_space_only_when_no_header_does_and_never_an_automatic_number() {
        let a = agent("t1", "idle", "api");
        assert_eq!(row_name(&a, "1", None, true), "api");
        assert_eq!(row_name(&a, "login", Some("login"), false), "login");
        assert_eq!(row_name(&a, "1", None, false), "claude");
        assert_eq!(row_name(&a, "main", None, false), "main");
    }

    #[test]
    fn captions_use_the_tab_name_and_spell_out_a_number_only_when_two_rows_would_read_alike() {
        let spaces = vec![json!({ "id": "s1", "tabs": [{ "id": "tab-a", "name": "1", "named": false }, { "id": "tab-b", "name": "2", "named": false }, { "id": "tab-c", "name": "login", "named": true }] })];
        let mut a = agent("a", "idle", "api");
        a["tab"] = json!("tab-a");
        let mut b = agent("b", "idle", "api");
        b["tab"] = json!("tab-b");
        let mut c = agent("c", "idle", "web");
        c["tab"] = json!("tab-c");
        let got = captions(&[a, b, c], &spaces, &[]);
        // Two number-named tabs in one space would read the same: they get their numbers.
        assert_eq!(got["a"], ("1".to_owned(), Some("1".to_owned())));
        assert_eq!(got["b"], ("2".to_owned(), Some("2".to_owned())));
        assert_eq!(got["c"], ("login".to_owned(), Some("login".to_owned())));
    }

    #[test]
    fn a_pane_running_a_known_agent_without_a_report_gets_a_row_with_no_status() {
        let spaces = vec![json!({ "id": "s1", "name": "api", "tabs": [{ "id": "tb", "index": 1, "name": "1" }] })];
        let panes = vec![
            json!({ "pane": "p1", "terminal": "t1", "tab": "tb", "status": null, "program": { "agent": true, "name": "Pi" }, "title": "x", "focused": false }),
            json!({ "pane": "p2", "terminal": "t2", "tab": "tb", "status": null, "program": { "agent": false } }),
            json!({ "pane": "p3", "terminal": "t3", "tab": "tb", "status": "idle", "program": { "agent": true } }),
        ];
        let got = unreported(&[], &panes, &spaces);
        assert_eq!(got.len(), 1);
        assert_eq!((text(&got[0], "address"), text(&got[0], "status"), text(&got[0], "space_name")), ("t1", "none", "api"));
        // Already listed: no second row.
        assert!(unreported(&[agent("t1", "idle", "api")], &panes, &spaces).is_empty());
    }

    #[test]
    fn the_lead_pane_of_a_tab_is_its_first_agent_else_its_first_focused_terminal() {
        let panes = vec![
            json!({ "pane": "p1", "terminal": "t1", "tab": "x", "focused": false, "program": { "agent": false } }),
            json!({ "pane": "p2", "terminal": "t2", "tab": "x", "focused": true, "program": { "agent": false } }),
            json!({ "pane": "p3", "terminal": "t3", "tab": "x", "focused": false, "program": { "agent": true } }),
            json!({ "pane": "p4", "mod": "editor", "terminal": null, "tab": "x" }),
        ];
        assert_eq!(text(lead_panes(&panes)["x"], "pane"), "p3");
        assert_eq!(text(lead_panes(&panes[..2])["x"], "pane"), "p2");
    }
}
