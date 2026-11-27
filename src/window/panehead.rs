//! The pane header, native: what a layout that frames its panes puts above (or below) each
//! one: the program's icon, the agent's name or the terminal's title, the folder, the agent's
//! status, and buttons that split, zoom and close the pane (shown while the pointer is on the
//! header). A click on the header focuses the pane.
//!
//! Layouts say `{"header": {"content": {"mod": "pane"}}}` (the shipped `main`) or
//! `{"mod": "terminals-pane"}` (the shipped `terminals`, a bigger variant); the app draws both
//! unless the person keeps a mod folder of that name.

use super::native::{icon, keyed, r, tip, tone, z};
use super::*;
use crate::i18n;

const SPLIT_RIGHT: &[u8] = include_bytes!("../../assets/icons/pane/split-right.svg");
const SPLIT_DOWN: &[u8] = include_bytes!("../../assets/icons/pane/split-down.svg");
const ZOOM: &[u8] = include_bytes!("../../assets/icons/pane/zoom.svg");
const UNZOOM: &[u8] = include_bytes!("../../assets/icons/pane/unzoom.svg");
const CLOSE: &[u8] = include_bytes!("../../assets/icons/pane/close.svg");
const SPINNER: &[u8] = include_bytes!("../../defaults/mods/workspace-chrome/spinner.svg");

/// The two shipped headers: `pane` and the roomier `terminals-pane`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) enum Style {
    Pane,
    Terminals,
}

/// The header a mod name stands for (the caller checks that no mod folder has that name).
pub(super) fn style_of(name: &str) -> Option<Style> {
    match name {
        "pane" => Some(Style::Pane),
        "terminals-pane" => Some(Style::Terminals),
        _ => None,
    }
}

/// `/Users/me/dev` is `~/dev` under the home folder.
pub(super) fn short_folder(cwd: &str, home: &str) -> String {
    if cwd.is_empty() {
        String::new()
    } else if cwd == home {
        "~".to_owned()
    } else {
        match cwd.strip_prefix(home).filter(|rest| rest.starts_with('/')) {
            Some(rest) => format!("~{rest}"),
            None => cwd.to_owned(),
        }
    }
}

/// The header's name: an agent's or shell's program name, else the terminal's title, else the program.
pub(super) fn header_name(agent_or_shell: bool, program: &str, title: &str) -> String {
    if agent_or_shell && !program.is_empty() {
        program.to_owned()
    } else if !title.is_empty() {
        title.to_owned()
    } else if !program.is_empty() {
        program.to_owned()
    } else {
        "shell".to_owned()
    }
}

/// The folder next to the name, unless the shell already put it in its title.
pub(super) fn folder_beside(title: &str, folder: &str) -> String {
    if folder.is_empty() || title.contains(folder) || title.starts_with(['~', '/']) { String::new() } else { folder.to_owned() }
}

/// Which edge of the header faces the terminal: the one away from the frame's side.
pub(super) fn faces_terminal(bottom: bool) -> &'static str {
    if bottom { "top" } else { "bottom" }
}

impl AppWindow {
    pub(super) fn native_pane_header_of(&self, name: &str) -> Option<Style> {
        style_of(name).filter(|_| !self.folders.contains_key(name))
    }

    pub(super) fn native_pane_header(&self, n: &Node, style: Style, bottom: bool, cx: &mut Context<Self>) -> AnyElement {
        let pane = n.id.clone();
        let addr = self.model.address_of(&pane);
        let big = style == Style::Terminals;
        let (text, pic) = if big { (14.0, 16.0) } else { (12.0, 14.0) };
        let program = addr.as_deref().map(|a| self.program(a)).unwrap_or(Value::Null);
        let str_of = |k: &str| program.get(k).and_then(Value::as_str).unwrap_or_default().to_owned();
        let flag = |k: &str| program.get(k).and_then(Value::as_bool).unwrap_or(false);
        let title = match (self.terms.get(&pane), self.editors.get(&pane)) {
            (Some(t), _) => t.view.read(cx).title.clone(),
            (None, Some(e)) => crate::editor::file_name(e.path()).to_owned(),
            _ => String::new(),
        };
        let name = header_name(flag("agent") || flag("shell"), &str_of("name"), &title);
        let folder = short_folder(addr.as_deref().and_then(|a| self.cwds.get(a)).map_or("", String::as_str), &home());
        let folder = if big { folder_beside(&title, &folder) } else { folder };
        let status = addr.as_deref().and_then(|a| self.model.status(a));
        let zoomed = self.model.active_tab().is_some_and(|t| t.zoomed.as_deref() == Some(pane.as_str()));
        let focused = self.focused_pane.as_deref() == Some(pane.as_str());
        let muted = tone("muted");
        let group = SharedString::from(format!("pane-head-{pane}"));
        let program_icon = program.get("icon").and_then(Value::as_str).map(|p| mods::app_dir().join(p));
        let tint = program.get("tint").and_then(Value::as_bool).unwrap_or(true);

        let app = cx.entity();
        let button = |id: &str, bytes: &'static [u8], label: String, command: &'static str, args: Value| {
            let ring = c("accent");
            let app = app.clone();
            keyed(
                div()
                    .id(SharedString::from(format!("{id}-{pane}")))
                    .flex_none()
                    .size(z(32.0))
                    .flex()
                    .items_center()
                    .justify_center()
                    .rounded(r(6.0))
                    .border_1()
                    .border_color(Hsla { a: 0.0, ..ring })
                    .text_color(muted)
                    .cursor_pointer()
                    .hover(|s| s.bg(c("hover")))
                    .tooltip(tip(label))
                    .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation()),
                ring,
                move |window, cx| app.update(cx, |this, cx| this.run(command, args.clone(), window, cx)),
            )
            .child(icon(bytes, 14.0, muted))
        };
        let focus_pane = json!({ "pane": pane });
        let mut head = div()
            .id(SharedString::from(format!("pane-head-{pane}")))
            .group(group.clone())
            .size_full()
            .pl(z(10.0))
            .pr(z(2.0))
            .gap(z(8.0))
            .flex()
            .items_center()
            .bg(c("surface"))
            .border_color(c("border"))
            .text_size(z(text))
            .on_click(cx.listener(move |this, _: &ClickEvent, window, cx| this.run("pane.focus", focus_pane.clone(), window, cx)));
        head = if faces_terminal(bottom) == "top" { head.border_t_1() } else { head.border_b_1() };
        head = head.when_some(program_icon, |el, path| {
            if tint {
                el.child(svg().external_path(path.to_string_lossy().into_owned()).size(z(pic)).flex_none().text_color(muted))
            } else {
                el.child(img(path).size(z(pic)).flex_none())
            }
        });
        head = head
            .child(div().flex_none().max_w(z(if big { 320.0 } else { 260.0 })).truncate().font_weight(FontWeight::MEDIUM).text_color(if big && !focused { muted } else { c("foreground") }).child(name))
            .child(div().flex_1().min_w_0().truncate().text_color(muted).child(folder));
        head = match status {
            Some("working") => head.child(icon(SPINNER, 12.0, c("working")).with_animation(
                SharedString::from(format!("pane-spin-{pane}")),
                Animation::new(Duration::from_millis(900)).repeat(),
                |s, delta| s.with_transformation(Transformation::rotate(percentage(delta))),
            )),
            Some("blocked") => {
                let blocked = tone("blocked");
                head.child(div().flex_none().px(z(6.0)).h(z(18.0)).flex().items_center().rounded(r(5.0)).bg(Hsla { a: 0.15, ..c("blocked") }).text_size(z(10.5)).font_weight(FontWeight::SEMIBOLD).text_color(blocked).child(i18n::t("pane.needs_you")))
            }
            Some("done") => head.child(div().size(z(7.0)).flex_none().rounded(r(4.0)).bg(c("done"))),
            _ => head,
        };
        let buttons = div()
            .flex_none()
            .flex()
            .items_center()
            .opacity(if focused { 1.0 } else { 0.0 })
            .group_hover(group, |s| s.opacity(1.0))
            .child(button("ph-r", SPLIT_RIGHT, i18n::t("menu.split_right"), "pane.split", json!({ "direction": "right", "pane": pane })))
            .child(button("ph-d", SPLIT_DOWN, i18n::t("menu.split_down"), "pane.split", json!({ "direction": "down", "pane": pane })))
            .child(button("ph-z", if zoomed { UNZOOM } else { ZOOM }, i18n::t(if zoomed { "pane.unzoom" } else { "menu.zoom" }), "pane.zoom", json!({ "pane": pane })))
            .child(button("ph-x", CLOSE, i18n::t("menu.close"), "pane.close", json!({ "pane": pane })));
        head.child(buttons).into_any_element()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    // `super::*` brings in gpui's own `test` attribute, which would shadow the standard one.
    use std::prelude::v1::test;

    #[test]
    fn both_shipped_header_names_are_known_and_others_are_not() {
        assert_eq!(style_of("pane"), Some(Style::Pane));
        assert_eq!(style_of("terminals-pane"), Some(Style::Terminals));
        assert_eq!(style_of("mine"), None);
    }

    #[test]
    fn the_folder_is_short_under_home_only_at_a_folder_boundary() {
        assert_eq!(short_folder("/Users/a", "/Users/a"), "~");
        assert_eq!(short_folder("/Users/a/dev/x", "/Users/a"), "~/dev/x");
        assert_eq!(short_folder("/Users/ab/dev", "/Users/a"), "/Users/ab/dev");
        assert_eq!(short_folder("", "/Users/a"), "");
    }

    #[test]
    fn the_name_is_the_program_for_agents_and_shells_else_the_title() {
        assert_eq!(header_name(true, "Claude Code", "✳ fixing"), "Claude Code");
        assert_eq!(header_name(false, "vim", "a.rs"), "a.rs");
        assert_eq!(header_name(false, "vim", ""), "vim");
        assert_eq!(header_name(false, "", ""), "shell");
    }

    #[test]
    fn a_title_that_already_holds_the_folder_hides_it_beside() {
        assert_eq!(folder_beside("zsh", "~/dev"), "~/dev");
        assert_eq!(folder_beside("~/dev", "~/dev"), "");
        assert_eq!(folder_beside("user@host: ~/dev", "~/dev"), "");
        assert_eq!(folder_beside("/tmp/x", "~/dev"), "");
        assert_eq!(folder_beside("zsh", ""), "");
    }

    #[test]
    fn the_divider_is_on_the_edge_facing_the_terminal() {
        assert_eq!(faces_terminal(true), "top");
        assert_eq!(faces_terminal(false), "bottom");
    }
}
