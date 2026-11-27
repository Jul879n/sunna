//! Add a machine, native: the hosts of `~/.ssh/config` to pick from (or type one), each added with one
//! click (`hc machine add`). When the machine needs something first (herdr-core installed there, a
//! password, a new host key) the dialog explains what to run in a terminal and watches until it shows up.
//! Opened by `mod.open machine-add` (the sidebar's Add machine, Settings > Machines, the welcome) or by a
//! mod's `openOverlay({ mod: "machine-add" })`, unless the person keeps a mod folder of that name.
//!
//! Plain rules at the top (the ssh config reader, shell quoting, what an `hc` answer means), tested at the
//! bottom; the dialog below only draws them.

use super::dialog::{self, Dialog};
use super::native::{icon, r, tone, z};
use super::*;
use crate::i18n;
use gpui_base::input::{InputEvent, InputState};
use std::collections::HashMap;

const SERVER: &[u8] = include_bytes!("../../defaults/mods/workspace-chrome/server.svg");
const CHECK: &[u8] = include_bytes!("../../defaults/mods/workspace-chrome/circle-check.svg");
const SPINNER: &[u8] = include_bytes!("../../defaults/mods/workspace-chrome/spinner.svg");
const MAC_SETTINGS: &str = "x-apple.systempreferences:com.apple.preference.security?Privacy_AllFiles";

// Rules ------------------------------------------------------------------------------

/// A host entry people can pick.
#[derive(Clone, Debug, PartialEq, Default)]
pub(super) struct Host {
    pub alias: String,
    pub aliases: Vec<String>,
    pub hostname: Option<String>,
    pub user: Option<String>,
}

/// `target` as one shell argument.
pub(super) fn shell_arg(target: &str) -> String {
    if !target.is_empty() && target.chars().all(|c| c.is_ascii_alphanumeric() || "_@%+=:,./-".contains(c)) {
        target.to_owned()
    } else {
        format!("'{}'", target.replace('\'', "'\\''"))
    }
}

/// The words of an `Include` line (a quoted path stays one).
fn words(value: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut cur = String::new();
    let (mut quoted, mut any) = (false, false);
    for ch in value.chars() {
        match ch {
            '"' => {
                quoted = !quoted;
                any = true;
            }
            c if c.is_whitespace() && !quoted => {
                if any || !cur.is_empty() {
                    out.push(std::mem::take(&mut cur));
                }
                any = false;
            }
            c => cur.push(c),
        }
    }
    if any || !cur.is_empty() {
        out.push(cur);
    }
    out
}

/// Whether `name` matches a pattern with `*` and `?`.
fn wildcard(pattern: &[char], name: &[char]) -> bool {
    match (pattern.first(), name.first()) {
        (None, None) => true,
        (Some('*'), _) => wildcard(&pattern[1..], name) || (!name.is_empty() && wildcard(pattern, &name[1..])),
        (Some('?'), Some(_)) => wildcard(&pattern[1..], &name[1..]),
        (Some(p), Some(n)) if p == n => wildcard(&pattern[1..], &name[1..]),
        _ => false,
    }
}

/// An Include's files, as ssh finds them: `~` is home, a relative path is under `~/.ssh`, and a pattern in the
/// file name matches the files of that folder, sorted.
pub(super) fn include_files(value: &str, home: &str) -> Vec<String> {
    let mut out = Vec::new();
    for w in words(value) {
        let path = if let Some(rest) = w.strip_prefix("~/") {
            format!("{home}/{rest}")
        } else if w.starts_with('/') || (w.len() > 2 && w.as_bytes()[1] == b':') {
            w
        } else {
            format!("{home}/.ssh/{w}")
        };
        let path = path.replace('\\', "/");
        if !path.contains(['*', '?', '[']) {
            out.push(path);
            continue;
        }
        let Some((dir, pattern)) = path.rsplit_once('/') else { continue };
        if dir.contains(['*', '?', '[']) {
            continue;
        }
        let pattern: Vec<char> = pattern.chars().collect();
        let mut found: Vec<String> = std::fs::read_dir(dir)
            .into_iter()
            .flatten()
            .flatten()
            .filter(|e| e.path().is_file() && wildcard(&pattern, &e.file_name().to_string_lossy().chars().collect::<Vec<_>>()))
            .map(|e| format!("{dir}/{}", e.file_name().to_string_lossy()))
            .collect();
        found.sort();
        out.extend(found);
    }
    out
}

/// The named hosts (no patterns) of an ssh config and the files it includes (ssh stops 16 deep).
pub(super) fn parse_config(text: &str, home: &str, hosts: &mut Vec<Host>, depth: usize) {
    let mut current: Option<usize> = None;
    for raw in text.lines() {
        let line = raw.trim();
        let Some(first) = line.find(|c: char| c.is_whitespace() || c == '=') else { continue };
        let key = line[..first].to_lowercase();
        if key.is_empty() || key.starts_with('#') {
            continue;
        }
        let value = line[first..].trim_start_matches(|c: char| c.is_whitespace() || c == '=').trim();
        match key.as_str() {
            "host" | "match" => {
                let names: Vec<&str> = if key == "host" {
                    value.split_whitespace().filter(|h| !h.contains(['*', '?', '!']) && !hosts.iter().any(|x| x.alias == *h || x.aliases.iter().any(|a| a == h))).collect()
                } else {
                    Vec::new()
                };
                current = names.first().map(|first| {
                    hosts.push(Host { alias: (*first).to_owned(), aliases: names[1..].iter().map(|s| (*s).to_owned()).collect(), ..Host::default() });
                    hosts.len() - 1
                });
            }
            "include" if depth < 16 => {
                for file in include_files(value, home) {
                    let text = std::fs::read_to_string(&file).unwrap_or_default();
                    parse_config(&text, home, hosts, depth + 1);
                }
            }
            "hostname" => {
                if let Some(i) = current {
                    hosts[i].hostname = Some(value.to_owned());
                }
            }
            "user" => {
                if let Some(i) = current {
                    hosts[i].user = Some(value.to_owned());
                }
            }
            _ => {}
        }
    }
}

/// What `hc machine add` said.
#[derive(Clone, Debug, PartialEq)]
pub(super) enum Outcome {
    Added,
    /// It needs the person at a terminal (a password, a new host key, herdr-core to install).
    Terminal,
    Failed(String),
}

fn mentions_login(message: &str) -> bool {
    let m = message.to_lowercase();
    ["password", "passphrase", "host key", "permission denied", "login"].iter().any(|w| m.contains(w))
}

/// Reads the answer of `hc machine add <target> --json`.
pub(super) fn classify_add(code: i32, stdout: &str, stderr: &str) -> Outcome {
    if code == 0 {
        return Outcome::Added;
    }
    let body = if stdout.is_empty() { stderr } else { stdout };
    let (kind, message) = match serde_json::from_str::<Value>(body) {
        Ok(v) => {
            let e = v.get("error").cloned().unwrap_or_else(|| json!({ "message": if stderr.is_empty() { stdout } else { stderr }.trim() }));
            (e.get("kind").and_then(Value::as_str).unwrap_or_default().to_owned(), e.get("message").and_then(Value::as_str).unwrap_or_default().to_owned())
        }
        Err(_) => {
            let text = if !stderr.is_empty() { stderr } else if !stdout.is_empty() { stdout } else { "" };
            (String::new(), if text.trim().is_empty() { i18n::t("machine-add.it_didn_t_answer") } else { text.trim().to_owned() })
        }
    };
    if ["hc_missing", "incompatible", "login_needed", "auth_failed", "host_key"].contains(&kind.as_str()) || mentions_login(&message) {
        return Outcome::Terminal;
    }
    Outcome::Failed(if message.is_empty() { i18n::t("machine-add.it_didn_t_work") } else { message })
}

fn is_added(added: &[Value], target: &str) -> bool {
    added.iter().any(|m| ["target", "name", "host"].iter().any(|k| m.get(*k).and_then(Value::as_str) == Some(target)))
}

fn text<'a>(v: &'a Value, k: &str) -> &'a str {
    v.get(k).and_then(Value::as_str).unwrap_or_default()
}

// Talking to the core ----------------------------------------------------------------

fn hc(args: &[&str]) -> Result<(i32, String, String), String> {
    mods::run_blocking("hc", &args.iter().map(|s| (*s).to_owned()).collect::<Vec<_>>(), None, Duration::from_secs(120))
}

/// The machines added so far (never the temporary ones).
fn list_machines() -> Vec<Value> {
    let Ok((0, out, _)) = hc(&["machine", "ls", "--json"]) else { return Vec::new() };
    serde_json::from_str::<Value>(&out)
        .ok()
        .and_then(|v| v.get("machines").and_then(Value::as_array).cloned())
        .unwrap_or_default()
        .into_iter()
        .filter(|m| m.get("temporary").and_then(Value::as_bool) != Some(true))
        .collect()
}

/// A Mac asks on its own screen before anything reads Desktop, Documents or Downloads, and the asking
/// program waits: its name and where its core lives.
fn probe_mac(name: &str) -> Option<String> {
    let args = ["-c".to_owned(), "uname; command -v herdr-core || echo \"$HOME/.local/bin/herdr-core\"".to_owned()];
    let (_, out, _) = mods::run_remote(name, "sh", &args, None).ok().map(|(c, o, e)| (c, String::from_utf8_lossy(&o).into_owned(), e))?;
    let mut lines = out.trim().lines();
    (lines.next()? == "Darwin").then(|| lines.next().unwrap_or("~/.local/bin/herdr-core").to_owned())
}

// The dialog -------------------------------------------------------------------------

#[derive(Clone, PartialEq)]
enum Row {
    Adding,
    Failed(String),
}

pub(super) struct MachineView {
    app: WeakEntity<AppWindow>,
    id: String,
    hosts: Option<Vec<Host>>,
    added: Vec<Value>,
    state: HashMap<String, Row>,
    pending: HashSet<String>,
    refreshing: bool,
    input: Entity<InputState>,
    /// target -> (machine name, core path) for an added Mac.
    macs: HashMap<String, (String, String)>,
    opened: HashSet<String>,
    copied: Option<String>,
    focus: dialog::Focus,
}

impl MachineView {
    fn new(app: WeakEntity<AppWindow>, id: String, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let input = dialog::input("", "user@host", window, cx, |this, ev, window, cx| match ev {
            InputEvent::PressEnter { .. } => this.add(this.typed(cx), window, cx),
            InputEvent::Change => cx.notify(),
            _ => {}
        });
        input.update(cx, |s, cx| s.focus(window, cx));
        let this = Self {
            app,
            id,
            hosts: None,
            added: Vec::new(),
            state: HashMap::new(),
            pending: HashSet::new(),
            refreshing: false,
            input,
            macs: HashMap::new(),
            opened: HashSet::new(),
            copied: None,
            focus: dialog::Focus::new(cx),
        };
        cx.spawn_in(window, async move |this, cx| {
            let home = mods::home().to_string_lossy().into_owned();
            let hosts = cx
                .background_executor()
                .spawn(async move {
                    let mut hosts = Vec::new();
                    let text = std::fs::read_to_string(format!("{home}/.ssh/config")).unwrap_or_default();
                    parse_config(&text, &home, &mut hosts, 0);
                    hosts
                })
                .await;
            let _ = this.update_in(cx, |this, window, cx| {
                this.hosts = Some(hosts);
                this.refresh(window, cx);
            });
            // While something waits for the person at a terminal (or machines are added), look again every 3 s.
            loop {
                cx.background_executor().timer(Duration::from_secs(3)).await;
                if this.update_in(cx, |this, window, cx| this.poll(window, cx)).is_err() {
                    break;
                }
            }
        })
        .detach();
        this
    }

    fn typed(&self, cx: &App) -> String {
        self.input.read(cx).value().trim().to_owned()
    }

    fn close(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let id = self.id.clone();
        let _ = self.app.update(cx, |app, cx| app.close_overlay(&id, window, cx));
    }

    fn poll(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if (!self.pending.is_empty() || !self.added.is_empty()) && !self.refreshing {
            self.refresh(window, cx);
        }
    }

    /// Reads the machines again; a pending target that showed up is done (and checked for a Mac).
    fn refresh(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.refreshing = true;
        let pending: Vec<String> = self.pending.iter().cloned().collect();
        cx.spawn_in(window, async move |this, cx| {
            let (machines, arrived) = cx
                .background_executor()
                .spawn(async move {
                    let machines = list_machines();
                    let arrived: Vec<(String, Option<(String, String)>)> = pending
                        .into_iter()
                        .filter(|t| is_added(&machines, t))
                        .map(|t| {
                            let name = machines.iter().find(|m| text(m, "target") == t || text(m, "name") == t).map(|m| text(m, "name").to_owned());
                            let mac = name.and_then(|n| probe_mac(&n).map(|core| (n, core)));
                            (t, mac)
                        })
                        .collect();
                    (machines, arrived)
                })
                .await;
            let _ = this.update_in(cx, |this, window, cx| {
                this.refreshing = false;
                this.added = machines;
                for (target, mac) in arrived {
                    this.pending.remove(&target);
                    this.state.remove(&target);
                    if this.typed(cx) == target {
                        this.input.update(cx, |s, cx| s.set_value("", window, cx));
                    }
                    if let Some(mac) = mac {
                        this.macs.insert(target, mac);
                    }
                }
                cx.notify();
            });
        })
        .detach();
    }

    fn add(&mut self, target: String, window: &mut Window, cx: &mut Context<Self>) {
        let target = target.trim().to_owned();
        if target.is_empty() || self.state.get(&target) == Some(&Row::Adding) {
            return;
        }
        self.pending.remove(&target);
        self.state.insert(target.clone(), Row::Adding);
        cx.notify();
        let t = target.clone();
        cx.spawn_in(window, async move |this, cx| {
            let (outcome, machines, mac) = cx
                .background_executor()
                .spawn(async move {
                    let outcome = match hc(&["machine", "add", &t, "--json"]) {
                        Ok((code, out, err)) => classify_add(code, &out, &err),
                        Err(e) => classify_add(1, "", &e),
                    };
                    let machines = list_machines();
                    let mac = (outcome == Outcome::Added)
                        .then(|| machines.iter().find(|m| text(m, "target") == t || text(m, "name") == t).map(|m| text(m, "name").to_owned()))
                        .flatten()
                        .and_then(|n| probe_mac(&n).map(|core| (n, core)));
                    (outcome, machines, mac)
                })
                .await;
            let _ = this.update_in(cx, |this, window, cx| {
                this.added = machines;
                match outcome {
                    Outcome::Added => {
                        this.state.remove(&target);
                        if this.typed(cx) == target {
                            this.input.update(cx, |s, cx| s.set_value("", window, cx));
                        }
                        if let Some(mac) = mac {
                            this.macs.insert(target, mac);
                        }
                    }
                    Outcome::Terminal => {
                        this.pending.insert(target.clone());
                        let message = i18n::tf("machine-add.couldn_t_add_target_here_it", &[("target", &target), ("quoted", &shell_arg(&target))]);
                        this.state.insert(target, Row::Failed(message));
                    }
                    Outcome::Failed(message) => {
                        this.state.insert(target, Row::Failed(message));
                    }
                }
                cx.notify();
            });
        })
        .detach();
    }

    fn copy(&mut self, command: String, cx: &mut Context<Self>) {
        cx.write_to_clipboard(ClipboardItem::new_string(command.clone()));
        self.copied = Some(command);
        cx.notify();
    }

    fn open_mac_settings(&mut self, target: String, name: String, window: &mut Window, cx: &mut Context<Self>) {
        self.opened.insert(target);
        cx.notify();
        cx.spawn_in(window, async move |_, cx| {
            cx.background_executor().spawn(async move { let _ = mods::run_remote(&name, "open", &[MAC_SETTINGS.to_owned()], None); }).await;
        })
        .detach();
    }

    // Drawing ------------------------------------------------------------------------

    fn copy_button(&self, id: String, command: String, cx: &mut Context<Self>) -> Stateful<Div> {
        let this = cx.entity();
        let copied = self.copied.as_deref() == Some(command.as_str());
        let label = i18n::t(if copied { "machine-add.copied" } else { "machine-add.copy" });
        dialog::side_button(id, label, move |_, cx| this.update(cx, |v, cx| v.copy(command.clone(), cx)))
    }

    /// A card in the theme's red tint (fill and a full border) that says what to do outside the app.
    fn card() -> Div {
        div().flex().flex_col().gap(z(8.0)).p(z(12.0)).rounded(r(10.0)).border_1().border_color(Hsla { a: 0.45, ..c("blocked") }).bg(Hsla { a: 0.10, ..c("blocked") })
    }

    fn row(&self, target: &str, label: &str, sub: &str, window: &Window, cx: &mut Context<Self>) -> Div {
        let _ = window;
        let this = cx.entity();
        let done = is_added(&self.added, target);
        let state = self.state.get(target);
        let machine = self.added.iter().find(|m| text(m, "target") == target || text(m, "name") == target || text(m, "host") == target).cloned();
        let right = if done {
            div().flex().items_center().gap(z(5.0)).text_size(z(12.0)).text_color(tone("ansi2")).child(icon(CHECK, 13.0, tone("ansi2"))).child(i18n::t("machine-add.added")).into_any_element()
        } else if state == Some(&Row::Adding) {
            div().flex().items_center().gap(z(6.0)).text_size(z(12.0)).text_color(tone("muted")).child(icon(SPINNER, 12.0, tone("muted"))).child(i18n::t("machine-add.connecting")).into_any_element()
        } else {
            let t = target.to_owned();
            dialog::side_button(format!("ma-add-{target}"), i18n::t("machine-add.add"), move |w, cx| this.update(cx, |v, cx| v.add(t.clone(), w, cx))).into_any_element()
        };
        let mut col = div().flex().flex_col().flex_none().child(
            div()
                .h(z(44.0))
                .px(z(10.0))
                .flex()
                .items_center()
                .gap(z(10.0))
                .child(icon(SERVER, 15.0, tone("muted")))
                .child(div().flex_none().text_size(z(13.0)).child(label.to_owned()))
                .child(div().flex_1().min_w_0().truncate().text_size(z(12.0)).text_color(tone("muted")).child(sub.to_owned()))
                .child(right),
        );
        if let Some(Row::Failed(message)) = state {
            let mut m = div().px(z(35.0)).pb(z(6.0)).flex().flex_col().gap(z(6.0)).child(dialog::message(message.clone(), true));
            if self.pending.contains(target) {
                m = m.child(div().flex().child(self.copy_button(format!("ma-copy-{target}"), format!("hc machine add {}", shell_arg(target)), cx)));
            }
            col = col.child(m);
        }
        if let Some((name, core)) = self.macs.get(target) {
            let (t, n) = (target.to_owned(), name.clone());
            let this = cx.entity();
            col = col.child(
                Self::card()
                    .child(div().text_size(z(13.0)).font_weight(FontWeight::SEMIBOLD).child(i18n::tf("machine-add.name_is_a_mac_allow", &[("name", name)])))
                    .child(dialog::message(i18n::t("machine-add.macos_asks_on_the_mac_s"), false))
                    .child(div().px(z(8.0)).py(z(5.0)).rounded(r(6.0)).border_1().border_color(c("border")).bg(c("background")).text_size(z(12.0)).child(core.clone()))
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(z(8.0))
                            .child(dialog::side_button(format!("ma-mac-{target}"), i18n::tf("machine-add.open_the_setting_on_name", &[("name", name)]), move |w, cx| this.update(cx, |v, cx| v.open_mac_settings(t.clone(), n.clone(), w, cx))))
                            .when(self.opened.contains(target), |d| d.child(dialog::message(i18n::t("machine-add.it_s_open_on_the_mac_s"), false))),
                    ),
            );
        }
        if let Some(m) = machine.filter(|m| m.get("keeps_running_after_logout").and_then(Value::as_bool) == Some(false)) {
            let name = text(&m, "name").to_owned();
            let user = m.get("logout_linger_user").and_then(Value::as_str).unwrap_or("<user>");
            let command = format!("sudo loginctl enable-linger -- {}", shell_arg(user));
            col = col.child(
                Self::card()
                    .child(div().text_size(z(13.0)).font_weight(FontWeight::SEMIBOLD).child(i18n::tf("machine-add.agents_on_name_stop_when", &[("name", &name)])))
                    .child(dialog::message(i18n::t("machine-add.the_server_ends_your_programs"), false))
                    .child(dialog::message(i18n::tf("machine-add.run_this_on_name_once", &[("name", &name)]), false))
                    .child(div().flex().items_center().gap(z(8.0)).child(div().flex_1().min_w_0().text_size(z(12.0)).child(command.clone())).child(self.copy_button(format!("ma-logout-{name}"), command, cx))),
            );
        }
        col
    }
}

impl Render for MachineView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let this = cx.entity();
        let hosts = self.hosts.clone().unwrap_or_default();
        let typed = self.typed(cx);
        let heading = |key: &str| dialog::label(i18n::t(key));
        let listed: Vec<Div> = hosts
            .iter()
            .filter(|h| !is_added(&self.added, &h.alias))
            .map(|h| {
                let sub = [h.user.clone(), h.hostname.clone()].into_iter().flatten().collect::<Vec<_>>().join("@");
                let sub = if sub.is_empty() && !h.aliases.is_empty() { i18n::tf("machine-add.also_aliases", &[("aliases", &h.aliases.join(", "))]) } else { sub };
                self.row(&h.alias, &h.alias, &sub, window, cx)
            })
            .collect();
        let added: Vec<Div> = self.added.clone().iter().map(|m| self.row(text(m, "target"), text(m, "name"), text(m, "target"), window, cx)).collect();
        let focused = self.input.read(cx).focus_handle(cx).is_focused(window);
        let t = typed.clone();
        let go_this = this.clone();
        // The line typed failed to add: its outline says so beside the message.
        let failed = matches!(self.state.get(&typed), Some(Row::Failed(_)));
        let input_row = div()
            .flex()
            .flex_row()
            .gap(z(8.0))
            .child(div().flex_1().min_w_0().child(dialog::field_box(focused, failed).child(div().flex_1().h_full().child(gpui_base::input::Input::new(&self.input)))))
            .child(dialog::side_button_enabled("ma-add-typed", i18n::t("machine-add.add"), !typed.is_empty(), move |w, cx| go_this.update(cx, |v, cx| v.add(t.clone(), w, cx))));
        let body = div()
            .flex()
            .flex_col()
            .gap(z(8.0))
            .child(heading(if hosts.is_empty() && self.hosts.is_some() { "machine-add.your_ssh_config_lists_no" } else { "machine-add.from_your_ssh_config" }))
            .child(div().id("ma-hosts").flex().flex_col().max_h(z(240.0)).overflow_y_scroll().when(self.hosts.is_none(), |d| d.child(dialog::message(i18n::t("machine-add.reading_ssh_config"), false))).children(listed))
            .when(!added.is_empty(), |d| d.child(div().id("ma-added").flex().flex_col().max_h(z(240.0)).overflow_y_scroll().children(added)))
            .child(heading("machine-add.or_type_how_ssh_reaches"))
            .child(input_row)
            .when_some(self.state.get(&typed).filter(|_| !typed.is_empty()), |d, s| match s {
                Row::Failed(m) => d.child(dialog::message(m.clone(), true)),
                Row::Adding => d,
            });
        Dialog::new(i18n::t("machine-add.add_a_machine"), &self.focus, dialog::run(&this, Self::close))
            .wide()
            .fields(1)
            .description(i18n::t("machine-add.a_vps_a_mac_mini"))
            .no_cancel()
            .body(body)
            .render("machine-add")
    }
}

impl AppWindow {
    /// Opens the Add a machine dialog (once: a second open closes it).
    pub(super) fn open_machine_add(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let id = "machine-add".to_owned();
        if self.overlays.iter().any(|o| o.id == id) {
            return self.close_overlay(&id, window, cx);
        }
        let app = cx.weak_entity();
        let view = cx.new(|cx| MachineView::new(app, id.clone(), window, cx));
        self.push_dialog_overlay(id, dialog::WIDE + 48.0, view.into(), window, cx);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    // `super::*` brings gpui's own `test` macro: the plain one is the std attribute.
    use core::prelude::v1::test;

    #[test]
    fn instructions_quote_a_target_as_one_shell_argument() {
        assert_eq!(shell_arg("my-host"), "my-host");
        assert_eq!(shell_arg("ssh://user@[::1]:2222"), "'ssh://user@[::1]:2222'");
        assert_eq!(shell_arg("host';echo $HOME"), "'host'\\'';echo $HOME'");
    }

    #[test]
    fn what_needs_a_terminal_is_told_apart_from_what_failed() {
        for kind in ["hc_missing", "incompatible", "login_needed", "auth_failed", "host_key"] {
            let out = json!({ "error": { "kind": kind, "message": "needs attention" } }).to_string();
            assert_eq!(classify_add(1, &out, ""), Outcome::Terminal, "{kind}");
        }
        for message in ["Enter password", "Enter passphrase", "new host key", "Permission denied", "needs a login"] {
            let err = json!({ "error": { "kind": "other", "message": message } }).to_string();
            assert_eq!(classify_add(1, "", &err), Outcome::Terminal, "{message}");
        }
        let offline = json!({ "error": { "kind": "offline", "message": "connection refused" } }).to_string();
        assert_eq!(classify_add(1, &offline, ""), Outcome::Failed("connection refused".into()));
        assert_eq!(classify_add(0, "{}", ""), Outcome::Added);
        assert_eq!(classify_add(1, "", ""), Outcome::Failed(i18n::t("machine-add.it_didn_t_answer")));
        assert_eq!(classify_add(1, "plain text", ""), Outcome::Failed("plain text".into()));
    }

    #[test]
    fn the_terminal_message_names_the_command_and_the_help() {
        let message = i18n::tf("machine-add.couldn_t_add_target_here_it", &[("target", "my-host"), ("quoted", &shell_arg("my-host"))]);
        assert!(message.contains("Couldn't add my-host here"));
        assert!(message.contains("`hc machine add my-host`"));
        assert!(message.contains("`hc machine --help`"));
    }

    #[test]
    fn a_target_counts_as_added_by_target_name_or_host() {
        let added = vec![json!({ "target": "a@b", "name": "remote", "host": "b" })];
        assert!(is_added(&added, "a@b") && is_added(&added, "remote") && is_added(&added, "b"));
        assert!(!is_added(&added, "other"));
    }

    fn hosts_under(dir: &str) -> Vec<(String, Option<String>)> {
        let home = std::env::temp_dir().join(format!("{dir}-{}", std::process::id()));
        std::fs::create_dir_all(home.join(".ssh/config.d")).unwrap();
        std::fs::write(home.join(".ssh/config.d/a-work"), "Host work\n  HostName 10.0.0.1\n").unwrap();
        std::fs::write(home.join(".ssh/config.d/b-lab"), "Host lab\n").unwrap();
        let mut hosts = Vec::new();
        parse_config("Include config.d/*\nHost direct\nHost *\n  User x\nHost a b\n", &home.to_string_lossy(), &mut hosts, 0);
        std::fs::remove_dir_all(&home).unwrap();
        hosts.into_iter().map(|h| (h.alias, h.hostname)).collect()
    }

    #[test]
    fn every_file_a_wildcard_include_matches_is_listed_even_under_a_home_with_a_space() {
        let want: Vec<(String, Option<String>)> = [("work", Some("10.0.0.1")), ("lab", None), ("direct", None), ("a", None)].iter().map(|(a, h)| ((*a).to_owned(), h.map(str::to_owned))).collect();
        assert_eq!(hosts_under("sunna-ssh"), want);
        assert_eq!(hosts_under("sunna ssh"), want);
    }

    #[test]
    fn host_aliases_user_and_patterns() {
        let mut hosts = Vec::new();
        parse_config("Host one two\n  User me\n  HostName h\nHost *.x\nMatch all\n  User no\n", "/h", &mut hosts, 0);
        assert_eq!(hosts, vec![Host { alias: "one".into(), aliases: vec!["two".into()], hostname: Some("h".into()), user: Some("me".into()) }]);
    }

    #[test]
    fn wildcards_match_stars_and_question_marks() {
        let m = |p: &str, n: &str| wildcard(&p.chars().collect::<Vec<_>>(), &n.chars().collect::<Vec<_>>());
        assert!(m("*", "a-work") && m("a?c", "abc") && m("*.conf", "x.conf"));
        assert!(!m("*.conf", "x.cfg") && !m("a?c", "ac"));
    }
}
