//! Automatic space cleanup (Orca's «Workspace Cleanup»), native: worktree checkouts that are no longer
//! needed are removed on a schedule, or by «Clean up now» (Settings > Git and spaces, or `cleanup.run`).
//! Three rules, each switched on by the person (everything starts off):
//!   * merged: the branch had commits of its own and they all landed in the default branch;
//!     * unchanged: no commits over the default branch for N days (the checkout folder's age);
//!     * bots: a `bot: <name>` space whose last run's agent is not working any more.
//!
//! A checkout is never touched when it is the repo's main one, its space is pinned, an agent is working in it
//! or it has uncommitted changes: those are kept and the log says why. Only checkouts on this computer are
//! looked at. The spaces' terminals close first, then `git worktree remove` (without --force) runs.
//!
//! `shared()` keys: `cleanup.auto`, `cleanup.every_hours`, `cleanup.merged`, `cleanup.unchanged`,
//! `cleanup.unchanged_days`, `cleanup.bots`, `cleanup.last` (unix ms of the last automatic run) and
//! `cleanup.log` (the last entries, newest first). The rules are plain functions, tested at the end.

use super::*;
use crate::i18n;

const LOG_KEEP: usize = 30;

/// What the person switched on.
#[derive(Clone, Debug, PartialEq)]
pub(super) struct Prefs {
    pub auto: bool,
    pub every_hours: u64,
    pub merged: bool,
    pub unchanged: bool,
    pub unchanged_days: u64,
    pub bots: bool,
}

pub(super) fn prefs() -> Prefs {
    let get = |k: &str| mods::shared(&format!("cleanup.{k}"));
    let flag = |k: &str, d: bool| get(k).and_then(|v| v.as_bool()).unwrap_or(d);
    let num = |k: &str, d: u64| get(k).and_then(|v| v.as_u64()).filter(|n| *n > 0).unwrap_or(d);
    Prefs { auto: flag("auto", false), every_hours: num("every_hours", 24), merged: flag("merged", true), unchanged: flag("unchanged", false), unchanged_days: num("unchanged_days", 1), bots: flag("bots", false) }
}

/// A checkout with the evidence the rules need.
#[derive(Clone, Debug, Default, PartialEq)]
pub(super) struct Cand {
    pub path: String,
    /// The repo's main checkout: where `git worktree remove` runs.
    pub repo: String,
    pub branch: Option<String>,
    /// The space showing it, to close first.
    pub space: Option<String>,
    pub main: bool,
    pub pinned: bool,
    pub busy: bool,
    pub dirty: bool,
    /// The branch has commits of its own and all of them are in the default branch.
    pub merged: bool,
    /// Commits over the default branch (None: not known).
    pub ahead: Option<u64>,
    /// Days since the folder was made (None: not known).
    pub age_days: Option<f64>,
    /// A bot's space whose last run is over.
    pub bot_done: bool,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) enum Reason {
    Merged,
    Unchanged,
    Bot,
}

impl Reason {
    pub fn key(self) -> &'static str {
        match self {
            Reason::Merged => "cleanup.reason_merged",
            Reason::Unchanged => "cleanup.reason_unchanged",
            Reason::Bot => "cleanup.reason_bot",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) enum Verdict {
    Remove(Reason),
    /// A rule matched but it is protected: the locale key of why.
    Keep(Reason, &'static str),
    /// No rule matches: nothing to say.
    Skip,
}

/// What the rules say about one checkout.
pub(super) fn judge(c: &Cand, p: &Prefs) -> Verdict {
    let reason = if p.merged && c.merged {
        Reason::Merged
    } else if p.bots && c.bot_done {
        Reason::Bot
    } else if p.unchanged && c.ahead == Some(0) && c.age_days.is_some_and(|d| d >= p.unchanged_days as f64) {
        Reason::Unchanged
    } else {
        return Verdict::Skip;
    };
    let why = if c.main {
        "cleanup.keep_main"
    } else if c.pinned {
        "cleanup.keep_pinned"
    } else if c.busy {
        "cleanup.keep_busy"
    } else if c.dirty {
        "cleanup.keep_dirty"
    } else {
        return Verdict::Remove(reason);
    };
    Verdict::Keep(reason, why)
}

/// Is an automatic run due? Off, or run less than `every_hours` ago: no.
pub(super) fn due(p: &Prefs, last_ms: u64, now_ms: u64) -> bool {
    p.auto && (p.merged || p.unchanged || p.bots) && now_ms.saturating_sub(last_ms) >= p.every_hours * 3_600_000
}

/// The log with new entries first, capped.
pub(super) fn with_entries(old: &[Value], new: Vec<Value>) -> Vec<Value> {
    new.into_iter().chain(old.iter().cloned()).take(LOG_KEEP).collect()
}

/// One log line: what was done to a checkout and why.
pub(super) fn entry(at: u64, c: &Cand, outcome: &str, reason: Reason, note: &str) -> Value {
    json!({ "at": at, "path": c.path, "branch": c.branch, "reason": reason.key(), "outcome": outcome, "note": note })
}

/// The detail a confirmation lists: the branch (or folder) of each checkout and why.
pub(super) fn describe(found: &[(Cand, Reason)]) -> String {
    found.iter().map(|(c, r)| format!("{} ({})", c.branch.clone().unwrap_or_else(|| c.path.clone()), i18n::t(r.key()))).collect::<Vec<_>>().join("\n")
}

// The window ---------------------------------------------------------------------------------------------

impl AppWindow {
    /// Looks every minute whether the automatic cleanup is due.
    pub(super) fn cleanup_tick(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let p = prefs();
        let last = mods::shared("cleanup.last").and_then(|v| v.as_u64()).unwrap_or(0);
        if due(&p, last, model::now_ms()) {
            mods::set_shared("cleanup.last".into(), json!(model::now_ms()));
            self.cleanup_run(true, window, cx);
        }
    }

    /// `cleanup.run`: scans the repos of the open spaces and removes what the rules allow. A person is shown the
    /// list first (`confirmed` true skips the question); `auto` runs (the schedule) do not ask.
    pub(super) fn cleanup_run(&mut self, confirmed: bool, window: &mut Window, cx: &mut Context<Self>) {
        let (p, d) = (prefs(), mods::snapshot_with(|s| (s.spaces.as_array().cloned().unwrap_or_default(), s.panes.as_array().cloned().unwrap_or_default(), s.agents.as_array().cloned().unwrap_or_default())));
        let pinned: Vec<String> = mods::shared("space.pinned").and_then(|v| serde_json::from_value(v).ok()).unwrap_or_default();
        let done = bots_done(&d.2);
        cx.spawn_in(window, async move |this, cx| {
            let found = cx.background_executor().spawn(async move { super::spaces::cleanup_scan(&d.0, &d.1, &d.2, &pinned, &done, &p) }).await;
            let _ = this.update_in(cx, |this, window, cx| {
                let now = model::now_ms();
                let (go, keep): (Vec<_>, Vec<_>) = found.into_iter().partition(|(_, v)| matches!(v, Verdict::Remove(_)));
                let remove: Vec<(Cand, Reason)> = go.into_iter().filter_map(|(c, v)| if let Verdict::Remove(r) = v { Some((c, r)) } else { None }).collect();
                let kept: Vec<Value> = keep.into_iter().filter_map(|(c, v)| if let Verdict::Keep(r, why) = v { Some(entry(now, &c, "kept", r, &i18n::t(why))) } else { None }).collect();
                if !kept.is_empty() {
                    let old: Vec<Value> = mods::shared("cleanup.log").and_then(|v| v.as_array().cloned()).unwrap_or_default();
                    mods::set_shared("cleanup.log".into(), json!(with_entries(&old, kept)));
                }
                if remove.is_empty() {
                    if !confirmed {
                        let spec = json!({ "kind": "toast", "notification": { "title": i18n::t("cleanup.nothing"), "level": "info" } });
                        this.open_overlay(mods::new_overlay_id(), spec, window, cx);
                    }
                    return;
                }
                if !confirmed {
                    let title = i18n::tn("cleanup.confirm_title", remove.len(), &[("count", &remove.len().to_string())]);
                    this.open_confirm(super::confirm::Ask::new(title, describe(&remove), "cleanup.run", json!({ "confirmed": true })), window, cx);
                    return;
                }
                this.cleanup_remove(remove, window, cx);
            });
        })
        .detach();
    }

    /// Closes each checkout's space, removes the checkout off the UI thread and writes the log.
    fn cleanup_remove(&mut self, remove: Vec<(Cand, Reason)>, window: &mut Window, cx: &mut Context<Self>) {
        for (c, _) in &remove {
            if let Some(id) = &c.space {
                self.run("space.close", json!({ "id": id, "confirmed": true }), window, cx);
            }
        }
        cx.spawn_in(window, async move |this, cx| {
            let done = cx.background_executor().spawn(async move { super::spaces::cleanup_delete(remove) }).await;
            let _ = this.update(cx, |_, cx| {
                let now = model::now_ms();
                let lines: Vec<Value> = done.iter().map(|(c, r, result)| entry(now, c, if result.is_ok() { "removed" } else { "failed" }, *r, result.as_ref().err().map_or("", String::as_str))).collect();
                let old: Vec<Value> = mods::shared("cleanup.log").and_then(|v| v.as_array().cloned()).unwrap_or_default();
                mods::set_shared("cleanup.log".into(), json!(with_entries(&old, lines)));
                mods::set_shared("cleanup.last_run".into(), json!(now));
                cx.notify();
            });
        })
        .detach();
    }

    pub(super) fn cleanup_command(&mut self, a: &Value, window: &mut Window, cx: &mut Context<Self>) {
        self.cleanup_run(a.get("confirmed").and_then(Value::as_bool) == Some(true), window, cx);
    }
}

/// The names of the bots whose last run is over: the tabs of runs whose agent is not working.
fn bots_done(agents: &[Value]) -> Vec<String> {
    super::bots::load_bots()
        .into_iter()
        .filter(|b| !super::bots::live_state(b, agents).eq("working"))
        .map(|b| format!("bot: {}", b.name))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use core::prelude::v1::test;

    fn on() -> Prefs {
        Prefs { auto: true, every_hours: 24, merged: true, unchanged: true, unchanged_days: 2, bots: true }
    }
    fn cand() -> Cand {
        Cand { path: "/w/a".into(), branch: Some("a".into()), ..Cand::default() }
    }

    #[test]
    fn a_checkout_no_rule_matches_is_left_alone_silently() {
        assert_eq!(judge(&cand(), &on()), Verdict::Skip);
        let off = Prefs { merged: false, unchanged: false, bots: false, ..on() };
        assert_eq!(judge(&Cand { merged: true, ..cand() }, &off), Verdict::Skip);
    }

    #[test]
    fn merged_unchanged_and_bot_checkouts_are_removed() {
        assert_eq!(judge(&Cand { merged: true, ..cand() }, &on()), Verdict::Remove(Reason::Merged));
        assert_eq!(judge(&Cand { bot_done: true, ..cand() }, &on()), Verdict::Remove(Reason::Bot));
        let fresh = Cand { ahead: Some(0), age_days: Some(2.5), ..cand() };
        assert_eq!(judge(&fresh, &on()), Verdict::Remove(Reason::Unchanged));
        // Too young, or with commits of its own, or its age unknown: kept without a word.
        assert_eq!(judge(&Cand { age_days: Some(0.5), ..fresh.clone() }, &on()), Verdict::Skip);
        assert_eq!(judge(&Cand { ahead: Some(3), ..fresh.clone() }, &on()), Verdict::Skip);
        assert_eq!(judge(&Cand { age_days: None, ..fresh }, &on()), Verdict::Skip);
    }

    #[test]
    fn what_is_protected_is_kept_with_its_reason() {
        let merged = Cand { merged: true, ..cand() };
        for (c, key) in [
            (Cand { main: true, ..merged.clone() }, "cleanup.keep_main"),
            (Cand { pinned: true, ..merged.clone() }, "cleanup.keep_pinned"),
            (Cand { busy: true, ..merged.clone() }, "cleanup.keep_busy"),
            (Cand { dirty: true, ..merged.clone() }, "cleanup.keep_dirty"),
        ] {
            assert_eq!(judge(&c, &on()), Verdict::Keep(Reason::Merged, key));
        }
    }

    #[test]
    fn the_schedule_is_due_only_when_on_with_a_rule_and_the_interval_passed() {
        let hour = 3_600_000;
        assert!(due(&on(), 0, 24 * hour));
        assert!(!due(&on(), 1, 24 * hour));
        assert!(!due(&Prefs { auto: false, ..on() }, 0, 99 * hour));
        assert!(!due(&Prefs { merged: false, unchanged: false, bots: false, ..on() }, 0, 99 * hour));
        assert!(due(&Prefs { every_hours: 1, ..on() }, 0, hour));
    }

    #[test]
    fn the_log_keeps_the_newest_thirty() {
        let old: Vec<Value> = (0..30).map(|i| json!({ "at": i })).collect();
        let out = with_entries(&old, vec![json!({ "at": 99 })]);
        assert_eq!(out.len(), 30);
        assert_eq!(out[0]["at"], json!(99));
        assert_eq!(out[29]["at"], json!(28));
    }

    #[test]
    fn the_defaults_run_nothing_until_the_person_asks() {
        let p = prefs();
        assert!(!p.auto && p.merged && !p.unchanged && !p.bots);
        assert_eq!((p.every_hours, p.unchanged_days), (24, 1));
        assert!(!due(&p, 0, u64::MAX / 2));
    }
}
