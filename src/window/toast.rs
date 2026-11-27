//! Notifications (toasts), native: «terminó», «te necesita», errors and successes. A card
//! at the window's bottom right with a level dot (or the sender's icon), a title, a body and
//! a visible ✕ (32 px) that dismisses it; a click on the card runs its `action` (go to the
//! agent) and dismisses it. Anything can raise one with `notify({ title, body, level, icon,
//! action })` or `app.notify`.
//!
//! At most `MAX_SHOWN` are on screen, the oldest first; the rest wait in a queue and start
//! their auto-close timer only when they appear (`next_to_start`). The `toast` mod it
//! replaced is gone, but a layout's own `"toast": "<mod>"` still wins when that mod exists.

use super::native::{icon, keyed, r, tip, tone, z};
use super::*;
use crate::i18n;

const CLOSE_ICON: &[u8] = include_bytes!("../../defaults/mods/workspace-chrome/x.svg");
pub(super) const WIDTH: f32 = 340.0;
pub(super) const HEIGHT: f32 = 72.0;
/// Toasts on screen at once; the others wait.
pub(super) const MAX_SHOWN: usize = 4;

/// What a notification says and does.
#[derive(Clone, Debug, PartialEq)]
pub(super) struct Note {
    pub title: String,
    pub body: String,
    pub level: String,
    /// An image path (relative ones to the app's folder).
    pub icon: Option<String>,
    pub action: Option<(String, Value)>,
    pub timeout_ms: u64,
}

impl Note {
    pub fn from_props(p: &Value) -> Self {
        let text = |k: &str| p.get(k).and_then(Value::as_str).unwrap_or_default().to_owned();
        let level = text("level");
        let action = p.get("action").and_then(|a| Some((a.get("name")?.as_str()?.to_owned(), a.get("args").filter(|v| v.is_object()).cloned().unwrap_or_else(|| json!({})))));
        Self {
            title: text("title"),
            body: text("body"),
            icon: p.get("icon").and_then(Value::as_str).filter(|s| !s.is_empty()).map(str::to_owned),
            action,
            timeout_ms: p.get("timeout").and_then(Value::as_f64).map_or_else(|| default_timeout(&level), |t| t.max(0.0) as u64),
            level,
        }
    }
}

/// Errors and warnings stay longer than the rest.
pub(super) fn default_timeout(level: &str) -> u64 {
    if matches!(level, "error" | "warning") { 8000 } else { 5000 }
}

/// The theme color a level's dot takes.
pub(super) fn level_color(level: &str) -> &'static str {
    match level {
        "error" => "blocked",
        "warning" => "working",
        "success" => "done",
        _ => "accent",
    }
}

/// `<name> needs login; run hc machine login <name>`: the machine the app could not reach.
pub(super) fn login_machine(body: &str) -> Option<&str> {
    let (name, rest) = body.split_once(" needs login; run hc machine login ")?;
    let valid = !name.is_empty() && name.len() <= 32 && name.starts_with(|c: char| c.is_ascii_lowercase() || c.is_ascii_digit()) && name.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || matches!(c, '_' | '.' | '-'));
    (valid && rest == name).then_some(name)
}

/// Which of the toasts (in the order they were opened) start their timer now: the unstarted ones
/// among the first `max`.
pub(super) fn next_to_start(started: &[bool], max: usize) -> Vec<usize> {
    started.iter().enumerate().take(max).filter(|(_, s)| !**s).map(|(i, _)| i).collect()
}

pub(super) struct ToastView {
    app: WeakEntity<AppWindow>,
    id: String,
    note: Note,
    copied: bool,
}

impl ToastView {
    fn close(&self, window: &mut Window, cx: &mut App) {
        let id = self.id.clone();
        let _ = self.app.update(cx, |app, cx| app.close_overlay(&id, window, cx));
    }
}

impl Render for ToastView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let n = self.note.clone();
        let login = login_machine(&n.body).map(str::to_owned);
        let command = login.as_ref().map(|m| format!("hc machine login {m}"));
        let muted = tone("muted");
        let ring = c("accent");
        let title = login.as_ref().map_or_else(|| n.title.clone(), |m| i18n::tf("toast.name_needs_a_login_run", &[("name", m.as_str())]));
        let has_action = n.action.is_some();
        let lead: AnyElement = match n.icon.as_deref() {
            Some(path) => {
                let path = if std::path::Path::new(path).is_absolute() { PathBuf::from(path) } else { mods::app_dir().join(path) };
                img(path).size(z(18.0)).flex_none().into_any_element()
            }
            None => div().size(z(8.0)).flex_none().rounded(r(4.0)).bg(c(level_color(&n.level))).into_any_element(),
        };
        let copy = command.clone().map(|command| {
            let label = if self.copied { i18n::t("toast.copied") } else { i18n::t("toast.copy") };
            let this = cx.entity();
            keyed(
                div()
                    .id("toast-copy")
                    .flex_none()
                    .h(z(32.0))
                    .px(z(10.0))
                    .flex()
                    .items_center()
                    .rounded(r(6.0))
                    .border_1()
                    .border_color(c("border"))
                    .bg(c("raised"))
                    .text_size(z(12.0))
                    .text_color(c("foreground"))
                    .cursor_pointer()
                    .hover(|s| s.bg(c("hover")))
                    .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation()),
                ring,
                move |_, cx| {
                    cx.write_to_clipboard(ClipboardItem::new_string(command.clone()));
                    this.update(cx, |v, cx| {
                        v.copied = true;
                        cx.notify();
                    });
                },
            )
            .child(label)
        });
        let text = div()
            .flex_1()
            .min_w_0()
            .flex()
            .flex_col()
            .child(div().truncate().font_weight(FontWeight::SEMIBOLD).text_size(z(13.0)).text_color(c("foreground")).child(title))
            .when(login.is_none() && !n.body.is_empty(), |d| d.child(div().id("toast-body").line_clamp(2).text_size(z(12.0)).text_color(muted).tooltip(tip(n.body.clone())).child(n.body.clone())))
            .when_some(command.clone().zip(copy), |d, (command, copy)| {
                d.child(div().flex().items_center().gap(z(8.0)).child(div().id("toast-command").flex_1().min_w_0().truncate().text_size(z(12.0)).text_color(muted).tooltip(tip(command.clone())).child(command)).child(copy))
            });
        let this = cx.entity();
        let close = keyed(
            div()
                .id("toast-close")
                .flex_none()
                .size(z(32.0))
                .flex()
                .items_center()
                .justify_center()
                .rounded(r(8.0))
                .border_1()
                .border_color(Hsla { a: 0.0, ..ring })
                .cursor_pointer()
                .hover(|s| s.bg(c("hover")))
                .tooltip(tip(i18n::t("menu.close")))
                .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation()),
            ring,
            move |window, cx| this.update(cx, |v, cx| v.close(window, cx)),
        )
        .child(icon(CLOSE_ICON, 14.0, muted));
        let action = n.action.clone();
        // Close it, then run its action (when it has one).
        let press = {
            let me = cx.entity();
            move |window: &mut Window, cx: &mut App| {
                me.update(cx, |this, cx| {
                    let (id, action) = (this.id.clone(), action.clone());
                    let _ = this.app.update(cx, |app, cx| {
                        app.close_overlay(&id, window, cx);
                        if let Some((name, args)) = action {
                            app.run(&name, args, window, cx);
                        }
                    });
                })
            }
        };
        let base = div()
            .id("toast")
            .w_full()
            .h(z(HEIGHT))
            .pl(z(16.0))
            .pr(z(8.0))
            .flex()
            .items_center()
            .gap(z(12.0))
            .rounded(r(8.0))
            .border_2()
            .border_color(Hsla { a: 0.0, ..c("accent") });
        // A toast with an action is a button the keyboard reaches too (Enter or Space); one without only closes on a click.
        let el = if has_action {
            keyed(base.cursor_pointer().hover(|s| s.bg(c("hover"))), c("accent"), press)
        } else {
            base.on_click(cx.listener(move |_, _: &ClickEvent, window, cx| press(window, cx)))
        };
        el
            .child(lead)
            .child(text)
            .child(close)
    }
}

impl AppWindow {
    /// The layout's own notification mod (`"toast": "<mod>"`), when that folder exists.
    pub(super) fn custom_toast_mod(&self) -> Option<String> {
        self.chrome.toast.as_deref().filter(|n| self.folders.contains_key(*n) || mods::mods_dir().join(n).is_dir()).map(str::to_owned)
    }

    /// Opens a native toast.
    pub(super) fn open_toast(&mut self, id: String, props: &Value, window: &mut Window, cx: &mut Context<Self>) {
        let note = Note::from_props(props);
        let ttl = Duration::from_millis(note.timeout_ms);
        let app = cx.weak_entity();
        let view = cx.new(|_| ToastView { app, id: id.clone(), note, copied: false });
        let focus = cx.focus_handle();
        self.push_view_overlay(id.clone(), OverlayKind::Toast, size(WIDTH, HEIGHT), None, view.into(), focus, window, cx);
        if let Some(o) = self.overlays.iter_mut().find(|o| o.id == id) {
            o.ttl = Some(ttl);
        }
        self.start_toasts();
    }

    /// Starts the auto-close timer of the toasts that are on screen now (the oldest `MAX_SHOWN`).
    pub(super) fn start_toasts(&mut self) {
        let toasts: Vec<usize> = (0..self.overlays.len()).filter(|i| self.overlays[*i].kind == OverlayKind::Toast).collect();
        let started: Vec<bool> = toasts.iter().map(|i| self.overlays[*i].expires.is_some()).collect();
        let now = Instant::now();
        for k in next_to_start(&started, MAX_SHOWN) {
            let o = &mut self.overlays[toasts[k]];
            o.expires = o.ttl.map(|t| now + t);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    // `super::*` brings in gpui's own `test` attribute, which would shadow the standard one.
    use std::prelude::v1::test;

    #[test]
    fn a_notification_reads_what_mods_pass_to_notify() {
        let n = Note::from_props(&json!({ "title": "Claude terminó", "body": "repo", "level": "success", "action": { "name": "agent.focus", "args": { "terminal": "t1" } } }));
        assert_eq!((n.title.as_str(), n.body.as_str(), n.level.as_str()), ("Claude terminó", "repo", "success"));
        assert_eq!(n.action, Some(("agent.focus".to_owned(), json!({ "terminal": "t1" }))));
        assert_eq!(n.timeout_ms, 5000);
        let bare = Note::from_props(&json!({ "title": "x", "action": {} }));
        assert!(bare.action.is_none() && bare.icon.is_none());
    }

    #[test]
    fn errors_and_warnings_stay_longer_and_a_timeout_wins() {
        assert_eq!(Note::from_props(&json!({ "level": "error" })).timeout_ms, 8000);
        assert_eq!(Note::from_props(&json!({ "level": "warning" })).timeout_ms, 8000);
        assert_eq!(Note::from_props(&json!({ "level": "info", "timeout": 12000 })).timeout_ms, 12000);
    }

    #[test]
    fn each_level_has_its_color() {
        assert_eq!(level_color("error"), "blocked");
        assert_eq!(level_color("warning"), "working");
        assert_eq!(level_color("success"), "done");
        assert_eq!(level_color("info"), "accent");
        assert_eq!(level_color(""), "accent");
    }

    #[test]
    fn the_login_notice_names_its_machine_only_when_it_is_that_exact_sentence() {
        assert_eq!(login_machine("dev needs login; run hc machine login dev"), Some("dev"));
        assert_eq!(login_machine("a.b-1_c needs login; run hc machine login a.b-1_c"), Some("a.b-1_c"));
        assert_eq!(login_machine("dev needs login; run hc machine login other"), None);
        assert_eq!(login_machine("Dev needs login; run hc machine login Dev"), None);
        assert_eq!(login_machine("dev needs login; run hc machine login dev; rm -rf /"), None);
        assert_eq!(login_machine("nothing"), None);
    }

    #[test]
    fn only_the_first_toasts_start_their_timer_and_the_rest_queue() {
        assert_eq!(next_to_start(&[false; 6], 4), vec![0, 1, 2, 3]);
        assert_eq!(next_to_start(&[true, true, false, false, false, false], 4), vec![2, 3]);
        assert_eq!(next_to_start(&[true; 4], 4), Vec::<usize>::new());
        assert_eq!(next_to_start(&[], 4), Vec::<usize>::new());
    }
}
