//! Settings > Updates: this app, the fork's releases and herdr-core: what is
//! waiting, when it was last looked for, and the buttons that act on it.
//! The release notes themselves are the What's New window (`app.whats_new`).

use super::*;
use controls::{button, primary, row};
use std::time::{Duration, Instant};

/// How long "Up to date" stays after pressing Check now.
const ANSWER: Duration = Duration::from_secs(3);

/// What the Check now button looks like: the press, the look, the answer.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Check {
    Button,
    Checking,
    UpToDate,
}

/// `asked`: when Check now was pressed. It follows the look (the answer comes
/// from the app), then says "Up to date" for a moment.
pub fn check_state(asked: Option<Instant>, now: Instant, checking: bool, checked: bool) -> Check {
    match asked {
        Some(_) if checking => Check::Checking,
        Some(at) if checked && now.duration_since(at) < ANSWER => Check::UpToDate,
        _ => Check::Button,
    }
}

/// The text under "Sunna x.y.z" or "herdr-core x.y.z" when nothing is waiting.
pub fn result_text(u: &Value) -> String {
    if u["checking"] == json!(true) {
        i18n::t("settings.checking")
    } else if let Some(at) = u["checked"].as_i64().filter(|t| *t > 0) {
        format!("{}. {}", i18n::t("settings.up_to_date"), i18n::tf("settings.checked_at_time", &[("time", &timefmt::clock(at))]))
    } else {
        i18n::t("settings.no_successful_check_check_your")
    }
}

pub fn fork_text(f: &Value) -> String {
    let latest = f["latest"].as_str().unwrap_or_default();
    if let Some(e) = f["error"].as_str().filter(|e| !e.is_empty()) {
        i18n::tf("settings.fork_error", &[("error", e)])
    } else if f["newer"] == json!(true) {
        i18n::tf("settings.fork_new", &[("version", latest)])
    } else if !latest.is_empty() {
        i18n::tf("settings.fork_current", &[("version", latest)])
    } else {
        i18n::t("settings.fork_none")
    }
}

impl SettingsView {
    /// The Check now control of one row, in whichever of its three states.
    fn check_control(&self, id: &'static str, u: &Value, cx: &mut Context<Self>) -> Div {
        let state = check_state(self.asked, Instant::now(), u["checking"] == json!(true), !u["checked"].is_null());
        let z = settings::z;
        match state {
            Check::Checking => div().h(z(32.0)).px(z(12.0)).flex().items_center().text_size(z(13.0)).text_color(c("muted")).child(i18n::t("settings.checking")),
            Check::UpToDate => div().h(z(32.0)).px(z(12.0)).flex().items_center().text_size(z(13.0)).text_color(c("ansi2")).child(format!("\u{2713} {}", i18n::t("settings.up_to_date"))),
            Check::Button => div().child(button(id, i18n::t("settings.check_now"), self.act(cx, |this, _, _| {
                this.asked = Some(Instant::now());
                crate::update::check_now();
            }))),
        }
    }

    pub(super) fn updates_page(&mut self, cx: &mut Context<Self>) -> Div {
        let u = crate::update::status();
        let z = settings::z;
        let checks = u["checks"] == json!(true);
        let none = || div();

        let (app_about, app_control) = if u["app"]["updates"] != json!(true) {
            (i18n::t("settings.a_build_from_source_it"), none())
        } else if let Some(v) = u["app"]["ready"].as_str() {
            (
                i18n::tf("settings.version_is_ready_restart_to", &[("version", v)]),
                div().child(primary("st-app-update", i18n::t("settings.restart_to_update"), self.act(cx, |this, w, cx| this.run("app.update", json!({}), w, cx)))),
            )
        } else {
            (result_text(&u), self.check_control("st-check-app", &u, cx))
        };

        let core = &u["core"];
        let ready = core["ready"].as_str();
        let (core_about, core_control) = if let Some(manager) = core["managed"].as_str() {
            let mut text = i18n::tf("settings.managed_by_manager_update_it", &[("manager", manager)]);
            if let Some(v) = ready {
                text = format!("{text} {}", i18n::tf("settings.version_is_available", &[("version", v)]));
            }
            (text, if checks { self.check_control("st-check-core", &u, cx) } else { none() })
        } else if core["updating"] == json!(true) {
            (i18n::t("settings.updating_your_terminals_keep_running"), none())
        } else if let Some(e) = core["error"].as_str() {
            (
                i18n::tf("settings.the_update_failed_error", &[("error", e)]),
                div().child(primary("st-core-retry", i18n::t("settings.try_again"), self.act(cx, |this, w, cx| this.run("app.update_core", json!({}), w, cx)))),
            )
        } else if let Some(v) = ready {
            (
                i18n::tf("settings.version_is_ready_your_terminals", &[("version", v)]),
                div().child(primary("st-core-update", i18n::t("settings.update_herdr_core"), self.act(cx, |this, w, cx| this.run("app.update_core", json!({}), w, cx)))),
            )
        } else if checks {
            (result_text(&u), self.check_control("st-check-core", &u, cx))
        } else {
            (i18n::t("settings.not_checked_a_build_from"), none())
        };

        let whats_new = button(
            "st-whats-new",
            i18n::t("settings.what_s_new"),
            self.act(cx, |this, w, cx| {
                let _ = this.app.update(cx, |app, cx| {
                    app.close_overlay(NATIVE_ID, w, cx);
                    app.run("app.whats_new", json!({}), w, cx);
                });
            }),
        );
        let mut page = div().w_full().child(controls::heading("Sunna")).child(row(
            format!("Sunna {}", u["app"]["version"].as_str().unwrap_or_default()),
            Some(app_about),
            div().flex().flex_row().items_center().gap(z(8.0)).child(whats_new).child(app_control),
        ));
        let fork = &u["fork"];
        if !fork.is_null() {
            let open = match (fork["newer"] == json!(true), fork["url"].as_str()) {
                (true, Some(url)) => {
                    let url = url.to_owned();
                    div().child(button("st-fork-open", i18n::t("settings.fork_open"), controls::act(move |_, _| crate::links::open(&crate::links::Target::Url(url.clone())))))
                }
                _ => none(),
            };
            page = page.child(row(i18n::t("settings.fork_release"), Some(fork_text(fork)), open));
        }
        page.child(controls::gap())
            .child(controls::heading("herdr-core"))
            .child(row(format!("herdr-core {}", core["version"].as_str().unwrap_or_default()), Some(core_about), core_control))
            .child(div().pt(z(12.0)).text_size(z(12.5)).line_height(z(18.0)).text_color(c("dim")).child(i18n::t("settings.herdr_looks_for_both_every")))
    }
}

#[cfg(test)]
mod tests {
    use super::{Check, ANSWER, check_state, fork_text, result_text};
    use crate::i18n;
    use serde_json::json;
    use std::time::{Duration, Instant};
    const _: Duration = ANSWER;

    #[test]
    fn check_now_goes_from_button_to_look_to_answer_to_button() {
        let t0 = Instant::now();
        assert_eq!(check_state(None, t0, false, true), Check::Button);
        assert_eq!(check_state(Some(t0), t0, true, false), Check::Checking);
        assert_eq!(check_state(Some(t0), t0 + Duration::from_secs(1), false, true), Check::UpToDate);
        assert_eq!(check_state(Some(t0), t0 + Duration::from_secs(4), false, true), Check::Button);
        // Never checked successfully: nothing to answer.
        assert_eq!(check_state(Some(t0), t0, false, false), Check::Button);
    }

    #[test]
    fn the_result_line_follows_the_status() {
        assert_eq!(result_text(&json!({ "checking": true })), i18n::t("settings.checking"));
        assert_eq!(result_text(&json!({ "checking": false, "checked": null })), i18n::t("settings.no_successful_check_check_your"));
        assert!(result_text(&json!({ "checked": 1_795_000_000 })).starts_with(&i18n::t("settings.up_to_date")));
    }

    #[test]
    fn the_fork_line_says_which_case_it_is() {
        assert!(fork_text(&json!({ "error": "offline" })).contains("offline"));
        assert!(fork_text(&json!({ "newer": true, "latest": "0.2.0" })).contains("0.2.0"));
        assert!(fork_text(&json!({ "newer": false, "latest": "0.1.0" })).contains("0.1.0"));
        assert_eq!(fork_text(&json!({ "latest": null })), i18n::t("settings.fork_none"));
    }
}
