//! The pop-up menus the window used to get from shipped mods (`menu`, `new-menu`, `awake-menu`), native
//! on the context menu (`ctxmenu.rs`): the pane's right-click menu, the tab's, the global one, the «+» of
//! the tab bar (terminal, agents, space, folder, worktrees) and the coffee cup's keep-awake modes. Every
//! line is a command, so `sunna do` can do what a menu does. The lists are plain functions of what the
//! window knows (terminals, panes, agents, key bindings), tested at the end.
//!
//! A layout's `{"mod": "menu"}` / `{"mod": "new-menu"}` / `{"mod": "awake-menu"}` overlay (and `help`) still
//! works: `open_overlay` opens these instead, unless the person keeps a mod folder of that name.
//! Commands: `worktree.new`, `worktree.open`, `worktree.cleanup` (the dialogs for the active space).

use super::ctxmenu::Item;
use super::*;
use crate::i18n;

pub(crate) const COMMANDS: [&str; 3] = ["worktree.new", "worktree.open", "worktree.cleanup"];

const PLUS: &[u8] = include_bytes!("../../defaults/mods/workspace-chrome/plus.svg");
const PEN: &[u8] = include_bytes!("../../defaults/mods/workspace-chrome/square-pen.svg");
const X: &[u8] = include_bytes!("../../defaults/mods/workspace-chrome/x.svg");
const TERMINAL: &[u8] = include_bytes!("../../defaults/mods/workspace-chrome/terminal.svg");
const SETTINGS: &[u8] = include_bytes!("../../defaults/mods/workspace-chrome/settings.svg");
const PANEL_RIGHT: &[u8] = include_bytes!("../../defaults/mods/workspace-chrome/panel-right.svg");
const SQUARE: &[u8] = include_bytes!("../../defaults/mods/workspace-chrome/square.svg");
const FOLDER: &[u8] = include_bytes!("../../defaults/mods/workspace-chrome/folder.svg");
const BRANCH: &[u8] = include_bytes!("../../defaults/mods/workspace-chrome/branch.svg");
const SHELL: &[u8] = include_bytes!("../../defaults/icons/shell.svg");

type Bindings = [(String, String, String)];

// Pure rules ----------------------------------------------------------------------------------

/// How a binding's keys read: `cmd-shift-o` is `⇧⌘O` on a Mac and `Super+Shift+O` elsewhere; several strokes
/// are told apart by a space.
pub(super) fn key_label(keys: &str, mac: bool) -> String {
    const ORDER: [&str; 5] = ["ctrl", "alt", "shift", "cmd", "fn"];
    let name = |m: &str| match (m, mac) {
        ("ctrl", true) => "⌃",
        ("alt", true) => "⌥",
        ("shift", true) => "⇧",
        ("cmd", true) => "⌘",
        ("ctrl", false) => "Ctrl",
        ("alt", false) => "Alt",
        ("shift", false) => "Shift",
        ("cmd", false) => "Super",
        _ => "Fn",
    };
    keys.split_whitespace()
        .map(|stroke| {
            let mut parts: Vec<&str> = stroke.split('-').collect();
            let mut key = parts.pop().unwrap_or_default().to_owned();
            if key.is_empty() {
                key = "-".into();
                parts.pop();
            }
            let mut held: Vec<String> = parts
                .iter()
                .map(|m| match m.to_lowercase().as_str() {
                    "secondary" => (if mac { "cmd" } else { "ctrl" }).to_owned(),
                    "super" | "win" => "cmd".to_owned(),
                    other => other.to_owned(),
                })
                .collect();
            if key.len() == 1 && key.chars().all(|c| c.is_ascii_uppercase()) {
                held.push("shift".into());
            }
            let label = if key.chars().count() == 1 {
                key.to_uppercase()
            } else {
                let mut chars = key.chars();
                chars.next().map(|f| f.to_uppercase().collect::<String>() + &chars.as_str().to_lowercase()).unwrap_or_default()
            };
            let mut out: Vec<String> = ORDER.iter().filter(|m| held.iter().any(|h| h == *m)).map(|m| name(m).to_owned()).collect();
            out.push(label);
            out.join(if mac { "" } else { "+" })
        })
        .collect::<Vec<_>>()
        .join(" ")
}

/// The keys bound (in the global section) to `action` (`tab.new`, or `pane.split:right`); empty when none.
pub(super) fn shortcut(bindings: &Bindings, action: &str, mac: bool) -> String {
    bindings.iter().find(|(section, _, a)| section == "global" && a == action).map(|(_, keys, _)| key_label(keys, mac)).unwrap_or_default()
}

fn str_of<'a>(v: &'a Value, key: &str) -> &'a str {
    v.get(key).and_then(Value::as_str).unwrap_or_default()
}

/// A pane's right-click menu. `target` is `{pane, terminal}`; a pane shown outside a tab (a layout's terminal
/// block, a window) has nothing to split, zoom or close.
pub(super) fn pane_items(target: &Value, terminals: &[Value], panes: &[Value], bindings: &Bindings, mac: bool) -> Vec<Item> {
    let (pane, address) = (str_of(target, "pane"), str_of(target, "terminal"));
    let term = terminals.iter().find(|t| str_of(t, "address") == address);
    let parent = term.and_then(|t| t.get("parent").and_then(Value::as_str)).and_then(|p| terminals.iter().find(|t| str_of(t, "address") == p));
    let to_program = term.is_some_and(|t| t.pointer("/labels/herdr.right-click").and_then(Value::as_str) == Some("program"));
    let in_tab = panes.iter().any(|p| str_of(p, "pane") == pane);
    let hint = |a: &str| shortcut(bindings, a, mac);
    let mut out = Vec::new();
    if in_tab {
        out.push(Item::run(i18n::t("menu.split_right"), Some(PANEL_RIGHT), "pane.split", json!({ "direction": "right", "pane": pane })).keys(hint("pane.split:right")));
        out.push(Item::run(i18n::t("menu.split_down"), Some(PANEL_RIGHT), "pane.split", json!({ "direction": "down", "pane": pane })).keys(hint("pane.split:down")));
        out.push(Item::run(i18n::t("menu.zoom"), Some(SQUARE), "pane.zoom", json!({ "pane": pane })).keys(hint("pane.zoom")));
    }
    if term.is_some() {
        out.push(if to_program {
            Item::run(i18n::t("menu.use_herdr_right_click_menu"), Some(TERMINAL), "terminal.right_click", json!({ "terminal": address, "to": "menu" }))
        } else {
            Item::run(i18n::t("menu.send_right_clicks_to_pane"), Some(TERMINAL), "terminal.right_click", json!({ "terminal": address, "to": "program" }))
        });
    }
    if let Some(p) = parent {
        let name = [str_of(p, "name"), str_of(p, "address")].into_iter().find(|n| !n.is_empty()).unwrap_or_default();
        out.push(Item::run(i18n::tf("menu.send_back_to_name", &[("name", name)]), Some(TERMINAL), "terminal.park", json!({ "terminal": address })));
    }
    if in_tab {
        out.push(Item::Sep);
        out.push(Item::run(i18n::t("menu.close_pane"), Some(X), "pane.close", json!({ "pane": pane })).keys(hint("pane.close")));
    }
    out
}

/// A tab's menu (`id`, and the space it is in).
pub(super) fn tab_items(id: &str, space: &str, bindings: &Bindings, mac: bool) -> Vec<Item> {
    vec![
        Item::run(i18n::t("menu.rename"), Some(PEN), "tab.rename", json!({ "id": id })),
        Item::run(i18n::t("menu.new_tab"), Some(PLUS), "tab.new", json!({ "space": space })).keys(shortcut(bindings, "tab.new", mac)),
        Item::Sep,
        Item::run(i18n::t("menu.close"), Some(X), "tab.close", json!({ "id": id })).keys(shortcut(bindings, "tab.close", mac)),
    ]
}

/// The window's own menu (a layout can open it with `{"mod": "menu"}`).
pub(super) fn global_items(bindings: &Bindings, mac: bool) -> Vec<Item> {
    let mut out = vec![
        Item::run(i18n::t("menu.new_space"), Some(PLUS), "space.new", json!({})).keys(shortcut(bindings, "space.new", mac)),
        Item::Sep,
        Item::run(i18n::t("menu.keybinds"), Some(TERMINAL), "app.help", json!({})).keys(shortcut(bindings, "app.help", mac)),
        Item::Sep,
    ];
    for (key, name) in [("menu.theme_graphite", "graphite"), ("menu.theme_ink", "ink"), ("menu.theme_moss", "moss"), ("menu.theme_atom_one_dark", "atom-one-dark")] {
        out.push(Item::run(i18n::t(key), Some(SETTINGS), "app.theme", json!({ "name": name })));
    }
    out
}

/// The menu of the «+» of the tab bar. `agents` is what `knownAgents` lists for the machine (`installed` says
/// which can start); `failed` when looking them up failed. The installed ones answer to their digit.
pub(super) fn new_items(agents: &[Value], failed: bool, space: Option<&str>, bindings: &Bindings, mac: bool) -> Vec<Item> {
    let installed: Vec<&Value> = agents.iter().filter(|a| a["installed"] == true).collect();
    let missing: Vec<&Value> = if failed { Vec::new() } else { agents.iter().filter(|a| a["installed"] != true).take(2).collect() };
    let start = |a: &Value| {
        let mut args = json!({ "name": str_of(a, "name"), "argv": a["run"] });
        if let Some(s) = space {
            args["space"] = json!(s);
        }
        args
    };
    let mut out = vec![
        Item::run("Terminal", Some(SHELL), "tab.new", space.map_or(json!({}), |s| json!({ "space": s }))).keys(shortcut(bindings, "tab.new", mac)),
        Item::Head(i18n::t("new-menu.start_an_agent")),
    ];
    for (i, a) in installed.iter().enumerate() {
        out.push(Item::run(str_of(a, "name"), None, "tab.new", start(a)).lead((*a).clone()).keys(if i < 9 { (i + 1).to_string() } else { String::new() }));
    }
    for a in missing {
        out.push(Item::run(str_of(a, "name"), None, "tab.new", start(a)).lead(a.clone()).keys(i18n::t("new-menu.not_installed")).disabled(true));
    }
    if failed {
        out.push(Item::Head(i18n::t("new-menu.could_not_check_installed_agents")));
    } else if agents.is_empty() {
        out.push(Item::Head(i18n::t("new-menu.checking_installed_agents")));
    }
    out.push(Item::Sep);
    out.push(Item::run(i18n::t("new-menu.new_space"), Some(PLUS), "space.new", json!({})).keys(shortcut(bindings, "space.new", mac)));
    out.push(Item::run(i18n::t("folders.open"), Some(FOLDER), "project.add", json!({ "step": "browse" })).keys(shortcut(bindings, "mod.open:folders", mac)));
    out.push(Item::run(i18n::t("new-menu.new_worktree"), Some(BRANCH), "worktree.new", json!({})).disabled(space.is_none()));
    out.push(Item::run(i18n::t("new-menu.open_worktree"), Some(BRANCH), "worktree.open", json!({})).disabled(space.is_none()));
    out
}

/// The coffee cup's menu: the three modes, the current one checked.
pub(super) fn awake_items(mode: crate::awake::Mode) -> Vec<Item> {
    use crate::awake::Mode;
    [("on", "awake.menu.on", Mode::On), ("auto", "awake.menu.auto", Mode::Auto), ("off", "awake.menu.off", Mode::Off)]
        .into_iter()
        .map(|(id, key, m)| Item::run(i18n::t(key), None, "app.keep_awake", json!({ "mode": id })).checked(m == mode))
        .collect()
}

// The window -----------------------------------------------------------------------------------

impl AppWindow {
    fn mac() -> bool {
        cfg!(target_os = "macos")
    }

    /// A pane's (`tab: "pane"`), tab's (`"tab"`) or the global menu at the pointer.
    pub(super) fn open_native_menu(&mut self, tab: &str, target: &Value, window: &mut Window, cx: &mut Context<Self>) {
        let bindings = crate::keys::listing();
        let mac = Self::mac();
        let items = match tab {
            "pane" => mods::snapshot_with(|s| pane_items(target, s.terminals.as_array().map_or(&[], Vec::as_slice), s.panes.as_array().map_or(&[], Vec::as_slice), &bindings, mac)),
            "tab" => {
                let id = str_of(target, "id").to_owned();
                let space = [str_of(target, "space")].into_iter().find(|s| !s.is_empty()).map(str::to_owned).or_else(|| self.model.active_space().map(|s| s.id.clone())).unwrap_or_default();
                tab_items(&id, &space, &bindings, mac)
            }
            _ => global_items(&bindings, mac),
        };
        self.open_context_menu(items, window, cx);
    }

    /// The «+» menu: the machine's agents come from the last look (shown at once) and are asked again in the background.
    pub(super) fn open_new_menu(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let space = self.model.active_space();
        let machine = space.and_then(|s| s.machine.clone());
        let space_id = space.map(|s| s.id.clone());
        let key = machine.clone().unwrap_or_default();
        let cached = mods::shared("new-menu.agents").and_then(|v| v.get(&key).and_then(Value::as_array).cloned());
        let asked = cx.background_executor().spawn(async move { mods::known_agents(machine, crate::programs::startable()) });
        let bindings = crate::keys::listing();
        match cached {
            Some(list) => {
                let items = new_items(&list, false, space_id.as_deref(), &bindings, Self::mac());
                self.open_context_menu(items, window, cx);
                cx.spawn(async move |_, _| {
                    if let Ok(fresh) = asked.await {
                        let mut all = mods::shared("new-menu.agents").unwrap_or(json!({}));
                        all[&key] = fresh;
                        mods::set_shared("new-menu.agents".into(), all);
                    }
                })
                .detach();
            }
            None => {
                cx.spawn_in(window, async move |this, cx| {
                    let found = asked.await;
                    let _ = this.update_in(cx, |this, window, cx| {
                        let (list, failed) = match found {
                            Ok(Value::Array(list)) => (list, false),
                            _ => (Vec::new(), true),
                        };
                        if !failed {
                            let mut all = mods::shared("new-menu.agents").unwrap_or(json!({}));
                            all[&key] = json!(list);
                            mods::set_shared("new-menu.agents".into(), all);
                        }
                        let items = new_items(&list, failed, space_id.as_deref(), &bindings, Self::mac());
                        this.open_context_menu(items, window, cx);
                    });
                })
                .detach();
            }
        }
    }

    /// The coffee cup's menu.
    pub(super) fn open_awake_menu(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.open_context_menu(awake_items(settings::get().keep_awake), window, cx);
    }

    /// `worktree.new`, `worktree.open` and `worktree.cleanup`: the dialog for the active space (or `space`).
    pub(super) fn worktree_command(&mut self, name: &str, a: &Value, window: &mut Window, cx: &mut Context<Self>) -> Result<(), String> {
        let space = a.get("space").and_then(Value::as_str).map(str::to_owned).or_else(|| self.model.active_space().map(|s| s.id.clone())).ok_or("no space to work from")?;
        let action = name.trim_start_matches("worktree.");
        self.open_worktree(json!({ "space": space, "action": action }), window, cx);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::awake::Mode;
    use core::prelude::v1::test;

    fn b(list: &[(&str, &str, &str)]) -> Vec<(String, String, String)> {
        list.iter().map(|(s, k, a)| ((*s).into(), (*k).into(), (*a).into())).collect()
    }
    fn labels(items: &[Item]) -> Vec<String> {
        items.iter().map(|i| match i {
            Item::Entry(e) => e.label.clone(),
            Item::Sub { label, .. } => label.clone(),
            Item::Head(t) => format!("# {t}"),
            Item::Sep => "-".into(),
        }).collect()
    }
    fn commands(items: &[Item]) -> Vec<String> {
        items.iter().filter_map(|i| if let Item::Entry(e) = i { Some(e.command.0.clone()) } else { None }).collect()
    }

    #[test]
    fn keys_read_like_the_platform_writes_them() {
        assert_eq!(key_label("cmd-shift-o", true), "⇧⌘O");
        assert_eq!(key_label("ctrl-alt-t", false), "Ctrl+Alt+T");
        assert_eq!(key_label("cmd-t", false), "Super+T");
        assert_eq!(key_label("secondary-n", true), "⌘N");
        assert_eq!(key_label("secondary-n", false), "Ctrl+N");
        assert_eq!(key_label("cmd--", true), "⌘-");
        assert_eq!(key_label("ctrl-a c", false), "Ctrl+A C");
        assert_eq!(key_label("cmd-down", true), "⌘Down");
        assert_eq!(key_label("cmd-T", true), "⇧⌘T");
        assert_eq!(key_label("", true), "");
    }

    #[test]
    fn a_shortcut_is_looked_up_by_action_or_action_with_its_argument() {
        let keys = b(&[("global", "cmd-t", "tab.new"), ("global", "cmd-d", "pane.split:right"), ("terminal", "x", "pane.zoom")]);
        assert_eq!(shortcut(&keys, "tab.new", true), "⌘T");
        assert_eq!(shortcut(&keys, "pane.split:right", true), "⌘D");
        assert_eq!(shortcut(&keys, "pane.zoom", true), "");
        assert_eq!(shortcut(&keys, "pane.close", true), "");
    }

    #[test]
    fn the_pane_menu_depends_on_the_terminal_and_whether_the_pane_is_in_a_tab() {
        let target = json!({ "pane": "p1", "terminal": "t1" });
        let panes = vec![json!({ "pane": "p1" })];
        let terms = vec![json!({ "address": "t1" })];
        let full = pane_items(&target, &terms, &panes, &[], false);
        assert_eq!(commands(&full), ["pane.split", "pane.split", "pane.zoom", "terminal.right_click", "pane.close"]);
        // A terminal outside a tab has nothing to split or close.
        assert_eq!(commands(&pane_items(&target, &terms, &[], &[], false)), ["terminal.right_click"]);
        // Right-clicks already go to the program: the menu offers to take them back.
        let sent = vec![json!({ "address": "t1", "labels": { "herdr.right-click": "program" } })];
        let Item::Entry(e) = &pane_items(&target, &sent, &[], &[], false)[0] else { panic!() };
        assert_eq!(e.command.1["to"], "menu");
        // A kept sub-agent can go back under its parent.
        let kid = vec![json!({ "address": "t1", "parent": "t0" }), json!({ "address": "t0", "name": "lead" })];
        assert!(labels(&pane_items(&target, &kid, &[], &[], false)).iter().any(|l| l.contains("lead")));
        assert!(pane_items(&target, &[], &[], &[], false).is_empty());
    }

    #[test]
    fn the_tab_and_global_menus_run_the_expected_commands() {
        assert_eq!(commands(&tab_items("t9", "s1", &[], false)), ["tab.rename", "tab.new", "tab.close"]);
        let Item::Entry(close) = &tab_items("t9", "s1", &[], false)[3] else { panic!() };
        assert_eq!(close.command.1, json!({ "id": "t9" }));
        let global = commands(&global_items(&[], false));
        assert_eq!(global.iter().filter(|c| *c == "app.theme").count(), 4);
        assert!(global.contains(&"app.help".to_owned()));
    }

    #[test]
    fn the_new_menu_lists_installed_agents_with_digits_and_two_missing_ones_off() {
        let agents: Vec<Value> = (0..5).map(|i| json!({ "name": format!("a{i}"), "run": [format!("a{i}")], "installed": i < 2 })).collect();
        let items = new_items(&agents, false, Some("s1"), &[], false);
        let entries: Vec<&crate::window::ctxmenu::Entry> = items.iter().filter_map(|i| if let Item::Entry(e) = i { Some(e) } else { None }).collect();
        assert_eq!(entries[1].keys.as_deref(), Some("1"));
        assert_eq!(entries[2].keys.as_deref(), Some("2"));
        assert!(entries[3].disabled && entries[4].disabled);
        assert_eq!(entries[1].command.1, json!({ "name": "a0", "argv": ["a0"], "space": "s1" }));
        // Without a space the worktree lines are off and a new tab names no space.
        let none = new_items(&agents, false, None, &[], false);
        let off: Vec<bool> = none.iter().rev().take(2).map(|i| matches!(i, Item::Entry(e) if e.disabled)).collect();
        assert_eq!(off, [true, true]);
        assert_eq!(commands(&none)[0], "tab.new");
        // A failed look shows only the installed ones and says so; an empty one says it is checking.
        let failed = new_items(&agents, true, Some("s1"), &[], false);
        assert!(labels(&failed).iter().any(|l| l.starts_with('#') && l.contains("check")) || labels(&failed).iter().filter(|l| l.starts_with('#')).count() == 2);
        assert_eq!(labels(&failed).iter().filter(|l| l.starts_with('a')).count(), 2);
        assert_eq!(labels(&new_items(&[], false, Some("s"), &[], false)).iter().filter(|l| l.starts_with('#')).count(), 2);
    }

    #[test]
    fn the_awake_menu_checks_only_the_current_mode() {
        for (mode, at) in [(Mode::On, 0), (Mode::Auto, 1), (Mode::Off, 2)] {
            let checked: Vec<bool> = awake_items(mode).iter().map(|i| matches!(i, Item::Entry(e) if e.on)).collect();
            assert_eq!(checked.iter().filter(|c| **c).count(), 1);
            assert!(checked[at]);
        }
        let Item::Entry(e) = &awake_items(Mode::On)[1] else { panic!() };
        assert_eq!(e.command, ("app.keep_awake".to_owned(), json!({ "mode": "auto" })));
    }
}
