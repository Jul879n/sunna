//! The keybind list (`app.help`), native: every binding of keys.json grouped (spaces, tabs, panes, agents,
//! terminal, window) with a line to filter by typing. Esc, the ✕ and Cancel close it. Grouping, naming and
//! filtering are plain functions, tested at the end.

use super::native::tone;
use super::popmenus::key_label;
use super::*;
use crate::i18n;
use gpui_base::input::{Input, InputEvent, InputState};

pub(super) const ID: &str = "help";
const ROW_H: f32 = 32.0;

/// `[locale key, tests prefix]` in the order the groups are listed; the last takes whatever is left.
const GROUPS: [(&str, &[&str]); 6] = [
    ("help.spaces", &["space."]),
    ("help.tabs", &["tab."]),
    ("help.panes", &["pane."]),
    ("help.agents", &["agent."]),
    ("rs.help.terminal", &["terminal.", "send:"]),
    ("help.window", &[]),
];

/// An action in words: `pane.split:right` is «pane split right», `send:x` is «type "x"».
pub(super) fn label(action: &str) -> String {
    if let Some(text) = action.strip_prefix("send:") {
        return format!("type {}", serde_json::to_string(text).unwrap_or_default());
    }
    let (name, arg) = action.split_once(':').map_or((action, None), |(n, a)| (n, Some(a)));
    let words = name.replace(['.', '_'], " ");
    arg.map_or(words.clone(), |a| format!("{words} {a}"))
}

/// The bindings that match the typed text (in the keys or the action's words), as `(section, keys, action)`.
pub(super) fn matching<'a>(all: &'a [(String, String, String)], filter: &str) -> Vec<&'a (String, String, String)> {
    let f = filter.trim().to_lowercase();
    all.iter().filter(|(_, keys, action)| f.is_empty() || keys.to_lowercase().contains(&f) || label(action).to_lowercase().contains(&f)).collect()
}

/// The matching bindings in their groups; an empty group is left out and each binding appears once.
pub(super) fn groups<'a>(rows: &[&'a (String, String, String)]) -> Vec<(&'static str, Vec<&'a (String, String, String)>)> {
    let mut left: Vec<&(String, String, String)> = rows.to_vec();
    let mut out = Vec::new();
    for (name, prefixes) in GROUPS {
        let (taken, rest): (Vec<_>, Vec<_>) = left.into_iter().partition(|(_, _, a)| prefixes.is_empty() || prefixes.iter().any(|p| a.starts_with(p)));
        left = rest;
        if !taken.is_empty() {
            out.push((name, taken));
        }
    }
    out
}

pub(super) struct HelpView {
    app: WeakEntity<AppWindow>,
    input: Entity<InputState>,
    focus: dialog::Focus,
    filter: String,
    all: Vec<(String, String, String)>,
}

impl HelpView {
    fn new(app: WeakEntity<AppWindow>, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let input = cx.new(|cx| InputState::new(window, cx).placeholder(i18n::t("help.type_to_filter")));
        input.update(cx, |s, _| s.set_editor_style(dialog::input_style()));
        cx.subscribe_in(&input, window, |this, input, ev: &InputEvent, _, cx| {
            if matches!(ev, InputEvent::Change) {
                this.filter = input.read(cx).value().to_string();
                cx.notify();
            }
        })
        .detach();
        input.update(cx, |s, cx| s.focus(window, cx));
        Self { app, input, focus: dialog::Focus::new(cx), filter: String::new(), all: crate::keys::listing() }
    }

    fn close(&self, window: &mut Window, cx: &mut Context<Self>) {
        let _ = self.app.update(cx, |app, cx| app.close_overlay(ID, window, cx));
    }
}

impl Render for HelpView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let z = settings::z;
        let mac = cfg!(target_os = "macos");
        let rows = matching(&self.all, &self.filter);
        let grouped = groups(&rows);
        let none = grouped.is_empty();
        let list = div().id("help-list").flex().flex_col().gap(z(10.0)).max_h(z(340.0)).overflow_y_scroll()
            .when(none, |d| d.child(div().px(z(4.0)).py(z(8.0)).text_size(z(12.5)).text_color(tone("muted")).child(i18n::t("rs.help.none"))))
            .children(grouped.into_iter().map(|(name, items)| {
                div().flex().flex_col().gap(z(2.0))
                    .child(div().text_size(z(11.5)).font_weight(FontWeight::MEDIUM).text_color(tone("muted")).child(i18n::t(name)))
                    .children(items.into_iter().map(|(section, keys, action)| {
                        div().h(z(ROW_H)).flex_none().px(z(8.0)).flex().items_center().rounded(z(7.0)).text_size(z(12.5))
                            .child(div().w(z(190.0)).flex_none().truncate().text_color(tone("accent")).child(format!("{}{}", key_label(keys, mac), if section == "global" { String::new() } else { format!("  ({section})") })))
                            .child(div().flex_1().min_w_0().truncate().text_color(c("foreground")).child(label(action)))
                    }))
            }));
        let focused = self.input.read(cx).focus_handle(cx).is_focused(window);
        let body = div().flex().flex_col().gap(z(10.0))
            .child(dialog::field_box(focused, false).flex_none().child(div().flex_1().h_full().child(Input::new(&self.input))))
            .child(list);
        let this = cx.entity();
        let close = move |w: &mut Window, cx: &mut App| this.update(cx, |v, cx| v.close(w, cx));
        dialog::Dialog::new(i18n::t("help.keybinds"), &self.focus, close).width(640.0).no_cancel().fields(1).body(body).render("help")
    }
}

impl AppWindow {
    /// `app.help`: opens the list, or closes it when it is open.
    pub(super) fn toggle_help(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.overlays.iter().any(|o| o.id == ID) {
            return self.close_overlay(ID, window, cx);
        }
        let app = cx.weak_entity();
        let view = cx.new(|cx| HelpView::new(app, window, cx));
        self.push_dialog_overlay(ID.into(), 640.0 + 48.0, view.into(), window, cx);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use core::prelude::v1::test;

    fn rows(list: &[(&str, &str)]) -> Vec<(String, String, String)> {
        list.iter().map(|(k, a)| ("global".to_owned(), (*k).to_owned(), (*a).to_owned())).collect()
    }

    #[test]
    fn an_action_reads_as_words() {
        assert_eq!(label("pane.split:right"), "pane split right");
        assert_eq!(label("app.keep_awake"), "app keep awake");
        assert_eq!(label("send:\u{1b}x"), "type \"\\u001bx\"");
    }

    #[test]
    fn typing_filters_by_keys_or_words_ignoring_case() {
        let all = rows(&[("cmd-t", "tab.new"), ("cmd-d", "pane.split:right"), ("cmd-,", "app.settings")]);
        assert_eq!(matching(&all, "").len(), 3);
        assert_eq!(matching(&all, " SPLIT ").len(), 1);
        assert_eq!(matching(&all, "cmd-t")[0].2, "tab.new");
        assert!(matching(&all, "zzz").is_empty());
    }

    #[test]
    fn every_binding_lands_in_one_group_and_empty_groups_are_left_out() {
        let all = rows(&[("a", "space.new"), ("b", "tab.new"), ("c", "pane.zoom"), ("d", "agent.next"), ("e", "terminal.copy"), ("f", "send:x"), ("g", "app.palette")]);
        let found = matching(&all, "");
        let g = groups(&found);
        assert_eq!(g.iter().map(|(n, _)| *n).collect::<Vec<_>>(), ["help.spaces", "help.tabs", "help.panes", "help.agents", "rs.help.terminal", "help.window"]);
        assert_eq!(g.iter().map(|(_, v)| v.len()).sum::<usize>(), 7);
        let only = rows(&[("a", "tab.new")]);
        assert_eq!(groups(&matching(&only, "")).len(), 1);
        assert!(groups(&[]).is_empty());
    }
}
