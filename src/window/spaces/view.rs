//! How the sidebar's list is shown (Orca's «Group by / Sort / Filters»): the choices, what they are saved as
//! in `shared()` (`sidebar.group_by`, `sidebar.sort_by`, `sidebar.project_order`, `sidebar.filters`, plus
//! `space.status` and `space.pinned` for each space), and the pure rules that filter, sort and group rows.
//! Nothing here draws or touches the window, so all of it is tested at the end.

use super::{Group, Row, changed_at, project_key, shown_name, text};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet};

/// A space with no running agent and no tab output for this long is «at rest».
pub(super) const REST_AFTER_MS: u64 = 30 * 60 * 1000;
/// Branch names the «hide default branch» filter takes for the repo's default.
const DEFAULT_BRANCHES: [&str; 3] = ["main", "master", "trunk"];

macro_rules! choice {
    ($(#[$m:meta])* $name:ident { $($var:ident = $id:literal, $key:literal;)+ } default $def:ident) => {
        $(#[$m])*
        #[derive(Clone, Copy, Debug, PartialEq, Eq)]
        pub(super) enum $name { $($var),+ }
        impl Default for $name {
            fn default() -> Self { Self::$def }
        }
        impl $name {
            pub(super) const ALL: &[Self] = &[$(Self::$var),+];
            pub(super) fn id(self) -> &'static str { match self { $(Self::$var => $id),+ } }
            /// The locale key that names it.
            pub(super) fn label(self) -> &'static str { match self { $(Self::$var => $key),+ } }
            pub(super) fn parse(s: &str) -> Option<Self> { match s { $($id => Some(Self::$var),)+ _ => None } }
            /// The saved value `v` (a string), or the default for anything else.
            #[allow(dead_code)]
            pub(super) fn read(v: Option<&Value>) -> Self { v.and_then(Value::as_str).and_then(Self::parse).unwrap_or_default() }
        }
    };
}

choice! {
    /// What the list is split under.
    GroupBy {
        None = "none", "sidebar.group_none";
        Status = "status", "sidebar.group_status";
        Pr = "pr", "sidebar.group_pr";
        Project = "project", "sidebar.group_project";
    } default Project
}

choice! {
    /// The order spaces take inside a group.
    SortBy {
        Name = "name", "sidebar.sort_name";
        Agents = "agents", "sidebar.sort_agents";
        Recent = "recent", "sidebar.sort_recent";
        Project = "project", "sidebar.sort_project";
        Manual = "manual", "sidebar.sort_manual";
    } default Manual
}

choice! {
    /// The order of the project headers.
    ProjectOrder {
        Manual = "manual", "sidebar.order_manual";
        Recent = "recent", "sidebar.order_recent";
    } default Manual
}

choice! {
    /// Where a space is in its work (set by hand: `space.status`).
    Status {
        Todo = "todo", "sidebar.status_todo";
        Progress = "in_progress", "sidebar.status_progress";
        Review = "in_review", "sidebar.status_review";
        Done = "done", "sidebar.status_done";
    } default Todo
}

/// What hides spaces. All off (and no project picked) shows everything.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub(super) struct Filters {
    /// Hide spaces at rest (no agent working or blocked, nothing written to a tab for `REST_AFTER_MS`).
    pub hide_idle: bool,
    /// Hide spaces on the repo's default branch (main, master, trunk).
    pub hide_default_branch: bool,
    /// Hide spaces a bot made (`space.bot_made`).
    pub hide_bots: bool,
    /// Only these projects (their keys); empty: all.
    pub projects: Vec<String>,
}

impl Filters {
    /// The saved filters; anything unreadable is no filters.
    pub(super) fn read(v: Option<&Value>) -> Self {
        v.and_then(|v| serde_json::from_value(v.clone()).ok()).unwrap_or_default()
    }
    pub(super) fn to_value(&self) -> Value {
        serde_json::to_value(self).unwrap_or_else(|_| json!({}))
    }
    /// The number the badge shows: one per switch on, one for the project filter.
    pub(super) fn count(&self) -> usize {
        usize::from(self.hide_idle) + usize::from(self.hide_default_branch) + usize::from(self.hide_bots) + usize::from(!self.projects.is_empty())
    }
}

/// The saved view of the list.
#[derive(Clone, Debug, Default, PartialEq)]
pub(super) struct View {
    pub group_by: GroupBy,
    pub sort_by: SortBy,
    pub project_order: ProjectOrder,
    pub filters: Filters,
}

/// The saved spaces' states: an unknown name or a non-map is «todo» for all.
pub(super) fn read_statuses(v: Option<&Value>) -> BTreeMap<String, Status> {
    v.and_then(Value::as_object).map(|m| m.iter().filter_map(|(id, s)| Some((id.clone(), Status::parse(s.as_str()?)?))).collect()).unwrap_or_default()
}

pub(super) fn read_ids(v: Option<&Value>) -> BTreeSet<String> {
    v.and_then(Value::as_array).map(|a| a.iter().filter_map(|k| k.as_str().map(str::to_owned)).collect()).unwrap_or_default()
}

/// What the rules need to know besides the rows.
pub(super) struct Ctx<'a> {
    pub agents: &'a [Value],
    pub now: u64,
    pub status: &'a BTreeMap<String, Status>,
    pub bot_made: &'a BTreeSet<String>,
    pub branch_names: bool,
}

impl Ctx<'_> {
    pub(super) fn status_of(&self, r: &Row) -> Status {
        self.status.get(r.id()).copied().unwrap_or_default()
    }
}

/// 4 needs the person (blocked), 3 finished, 2 working, 0 nothing: the best of the space's agents.
fn agent_rank(r: &Row, agents: &[Value]) -> u8 {
    agents
        .iter()
        .filter(|a| text(a, "space") == r.id())
        .map(|a| match text(a, "status") {
            "blocked" => 4,
            "done" => 3,
            "working" => 2,
            _ => 0,
        })
        .max()
        .unwrap_or(0)
}

pub(super) fn is_idle(r: &Row, agents: &[Value], now: u64) -> bool {
    let busy = agents.iter().any(|a| text(a, "space") == r.id() && matches!(text(a, "status"), "working" | "blocked"));
    let last = changed_at(&r.w);
    !busy && now.saturating_sub(last) > REST_AFTER_MS
}

/// The spaces the filters leave. The shown space is filtered like any other: «Clear filters» is always at hand.
pub(super) fn filter_rows(rows: &[Row], f: &Filters, k: &Ctx) -> Vec<Row> {
    rows.iter()
        .filter(|r| {
            !(f.hide_idle && is_idle(r, k.agents, k.now)
                || f.hide_default_branch && r.g.as_ref().is_some_and(|g| DEFAULT_BRANCHES.contains(&g.branch.as_str()))
                || f.hide_bots && k.bot_made.contains(r.id())
                || !f.projects.is_empty() && !f.projects.contains(&project_key(r)))
        })
        .cloned()
        .collect()
}

/// Puts rows in the chosen order; every order keeps the model's order among equals.
pub(super) fn sort_rows(rows: &mut [Row], by: SortBy, k: &Ctx) {
    match by {
        SortBy::Manual => rows.sort_by_key(|r| r.i),
        SortBy::Name => rows.sort_by_cached_key(|r| (shown_name(&r.w, r.g.as_ref(), k.branch_names).to_lowercase(), r.i)),
        SortBy::Agents => rows.sort_by_cached_key(|r| (std::cmp::Reverse(agent_rank(r, k.agents)), std::cmp::Reverse(changed_at(&r.w)), r.i)),
        SortBy::Recent => rows.sort_by_cached_key(|r| (std::cmp::Reverse(changed_at(&r.w)), r.i)),
        SortBy::Project => rows.sort_by_cached_key(|r| (project_label(r).to_lowercase(), r.i)),
    }
}

fn project_label(r: &Row) -> String {
    r.g.as_ref().map(|g| g.name.clone()).filter(|n| !n.is_empty()).unwrap_or_else(|| text(&r.w, "name").to_owned())
}

/// The projects in order: `Manual` follows the order the person dragged them into (`saved`, the keys), then the
/// order they were first seen in; `Recent` puts the one written to last first.
pub(super) fn order_projects(groups: &mut [Group], order: ProjectOrder, saved: &[String]) {
    match order {
        ProjectOrder::Recent => groups.sort_by_cached_key(|g| std::cmp::Reverse(g.rows.iter().map(|r| changed_at(&r.w)).max().unwrap_or(0))),
        ProjectOrder::Manual if !saved.is_empty() => groups.sort_by_cached_key(|g| saved.iter().position(|k| *k == g.key).unwrap_or(usize::MAX)),
        ProjectOrder::Manual => {}
    }
}

/// The rows under each state, in the order Todo, In progress, In review, Done; a state with none is left out.
pub(super) fn by_status(rows: &[Row], k: &Ctx) -> Vec<(Status, Vec<Row>)> {
    Status::ALL.iter().map(|s| (*s, rows.iter().filter(|r| k.status_of(r) == *s).cloned().collect::<Vec<_>>())).filter(|(_, v)| !v.is_empty()).collect()
}

/// The pinned rows (in `rows`' order) and the rest.
pub(super) fn split_pinned(rows: Vec<Row>, pinned: &BTreeSet<String>) -> (Vec<Row>, Vec<Row>) {
    rows.into_iter().partition(|r| pinned.contains(r.id()))
}

/// A project's header keys, for folding all of them.
pub(super) fn all_folded(keys: &[String], folded: &[String]) -> bool {
    !keys.is_empty() && keys.iter().all(|k| folded.contains(k))
}

/// What `sidebar.collapse_all` saves: every key folded, or (when they all were) none of these.
pub(super) fn toggle_folded(keys: &[String], folded: &[String]) -> Vec<String> {
    if all_folded(keys, folded) {
        folded.iter().filter(|f| !keys.contains(f)).cloned().collect()
    } else {
        folded.iter().chain(keys.iter().filter(|k| !folded.contains(k))).cloned().collect()
    }
}

/// One thing the options menu does.
#[derive(Clone, Debug, PartialEq)]
pub(super) enum Act {
    Group(GroupBy),
    Sort(SortBy),
    Order(ProjectOrder),
    HideIdle,
    HideDefault,
    HideBots,
    /// Show (or stop showing) only this project.
    Project(String),
    Clear,
    /// Not a view choice: scroll to the shown space.
    Reveal,
}

/// The view after `act`.
pub(super) fn apply(v: &View, act: &Act) -> View {
    let mut v = v.clone();
    match act {
        Act::Group(g) => v.group_by = *g,
        Act::Sort(s) => v.sort_by = *s,
        Act::Order(o) => v.project_order = *o,
        Act::HideIdle => v.filters.hide_idle = !v.filters.hide_idle,
        Act::HideDefault => v.filters.hide_default_branch = !v.filters.hide_default_branch,
        Act::HideBots => v.filters.hide_bots = !v.filters.hide_bots,
        Act::Project(key) => match v.filters.projects.iter().position(|k| k == key) {
            Some(at) => {
                v.filters.projects.remove(at);
            }
            None => v.filters.projects.push(key.clone()),
        },
        Act::Clear => v.filters = Filters::default(),
        Act::Reveal => {}
    }
    v
}

/// A line of the options menu.
#[derive(Clone, Debug, PartialEq)]
pub(super) enum Entry {
    Heading(String),
    /// `radio`: one of a group (a dot-less check); else a switch.
    Item { act: Act, label: String, on: bool, radio: bool },
    /// The project search: what was typed so far.
    Search(String),
    Note(String),
}

impl Entry {
    pub(super) fn act(&self) -> Option<&Act> {
        match self {
            Self::Item { act, .. } => Some(act),
            _ => None,
        }
    }
}

/// The options menu's lines for a view. `projects` are `(key, name)`; the search narrows them.
pub(super) fn entries(v: &View, projects: &[(String, String)], query: &str) -> Vec<Entry> {
    use crate::i18n::t;
    let mut out = vec![Entry::Heading(t("sidebar.group_by_title"))];
    out.extend(GroupBy::ALL.iter().map(|g| Entry::Item { act: Act::Group(*g), label: t(g.label()), on: v.group_by == *g, radio: true }));
    out.push(Entry::Heading(t("sidebar.sort_by_title")));
    out.extend(SortBy::ALL.iter().map(|o| Entry::Item { act: Act::Sort(*o), label: t(o.label()), on: v.sort_by == *o, radio: true }));
    out.push(Entry::Heading(t("sidebar.order_title")));
    out.extend(ProjectOrder::ALL.iter().map(|o| Entry::Item { act: Act::Order(*o), label: t(o.label()), on: v.project_order == *o, radio: true }));
    out.push(Entry::Heading(t("sidebar.filters_title")));
    let f = &v.filters;
    out.push(Entry::Item { act: Act::HideIdle, label: t("sidebar.filter_idle"), on: f.hide_idle, radio: false });
    out.push(Entry::Item { act: Act::HideDefault, label: t("sidebar.filter_default"), on: f.hide_default_branch, radio: false });
    out.push(Entry::Item { act: Act::HideBots, label: t("sidebar.filter_bots"), on: f.hide_bots, radio: false });
    if !projects.is_empty() {
        out.push(Entry::Heading(t("sidebar.filter_project")));
        out.push(Entry::Search(query.to_owned()));
        let shown: Vec<Entry> = projects
            .iter()
            .filter(|(_, name)| matches_query(name, query))
            .map(|(key, name)| Entry::Item { act: Act::Project(key.clone()), label: name.clone(), on: f.projects.contains(key), radio: false })
            .collect();
        if shown.is_empty() {
            out.push(Entry::Note(t("sidebar.no_projects")));
        }
        out.extend(shown);
    }
    if f.count() > 0 {
        out.push(Entry::Item { act: Act::Clear, label: t("sidebar.clear_filters"), on: false, radio: false });
    }
    out.push(Entry::Heading(String::new()));
    out.push(Entry::Item { act: Act::Reveal, label: t("sidebar.reveal"), on: false, radio: false });
    out
}

/// Whether a project's name has what was typed in the menu's search (case aside).
pub(super) fn matches_query(label: &str, query: &str) -> bool {
    let q = query.trim().to_lowercase();
    q.is_empty() || label.to_lowercase().contains(&q)
}

// The keyboard cursor of the list ---------------------------------------------------

/// What a row of the list is, for the keyboard.
#[derive(Clone, Debug, PartialEq)]
pub(super) enum What {
    /// A space (its id): Enter shows it.
    Space(String),
    /// An agent under a space (its pane): Enter focuses it.
    Agent(String),
    /// A project header (its key): Enter folds it.
    Project(String),
    /// A heading over a set of spaces (Pinned, a state): Enter folds it.
    Section(String),
    /// A machine header (`None` is this computer): Enter folds it.
    Machine(Option<String>),
    /// A project group's header (its id): Enter folds it.
    Group(String),
}

impl What {
    /// A key no other row shares, whatever order the list is in.
    pub(super) fn key(&self) -> String {
        match self {
            What::Space(id) => format!("space:{id}"),
            What::Agent(pane) => format!("agent:{pane}"),
            What::Project(key) => format!("project:{key}"),
            What::Section(key) => format!("section:{key}"),
            What::Machine(name) => format!("machine:{}", name.as_deref().unwrap_or("")),
            What::Group(id) => format!("group:{id}"),
        }
    }

    /// Can Left and Right fold it?
    pub(super) fn folds(&self) -> bool {
        matches!(self, What::Project(_) | What::Section(_) | What::Machine(_) | What::Group(_))
    }
}

/// Where the cursor is: the row it was on if the list still has it, else the shown space's, else the first.
pub(super) fn settle(stops: &[What], cursor: Option<&str>, active: Option<&str>) -> Option<usize> {
    let find = |key: &str| stops.iter().position(|w| w.key() == key);
    cursor.and_then(find).or_else(|| active.and_then(|id| find(&What::Space(id.to_owned()).key()))).or_else(|| (!stops.is_empty()).then_some(0))
}

/// The row the cursor moves to on `key` (arrows, Home, End, PageUp, PageDown), by `native::step_cursor`.
pub(super) fn cursor_to(stops: &[What], cursor: Option<&str>, active: Option<&str>, key: &str) -> Option<usize> {
    crate::window::native::step_cursor(settle(stops, cursor, active)?, key, stops.len())
}

/// How far a list scrolled by `offset` (negative when scrolled down) must move to show a row: `top..bottom` inside a
/// viewport `view_top..view_bottom`, all in window pixels; 0 when it already shows. A row taller than the viewport shows its top.
pub(super) fn scroll_delta(view_top: f32, view_bottom: f32, top: f32, bottom: f32) -> f32 {
    if top < view_top || bottom - top > view_bottom - view_top {
        view_top - top
    } else if bottom > view_bottom {
        view_bottom - bottom
    } else {
        0.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::window::spaces::Git;

    fn row(id: &str, i: usize, name: &str, repo: &str, branch: &str, changed: u64, active: bool) -> Row {
        let w = json!({ "id": id, "name": name, "folder": format!("/r/{name}"), "active": active, "tabs": [{ "id": format!("t-{id}"), "changed_at": changed }] });
        let g = Git { name: repo.into(), branch: branch.into(), ahead: 0, behind: 0, dirty: false, repo: json!([null, format!("/r/{repo}/.git")]).to_string(), linked: false };
        Row { w, i, g: Some(g) }
    }
    fn facts<'a>(agents: &'a [Value], status: &'a BTreeMap<String, Status>, _pinned: &'a BTreeSet<String>, bots: &'a BTreeSet<String>) -> Ctx<'a> {
        Ctx { agents, now: 10_000_000, status, bot_made: bots, branch_names: false }
    }
    fn ids(rows: &[Row]) -> Vec<&str> {
        rows.iter().map(Row::id).collect()
    }
    fn sample() -> Vec<Row> {
        vec![row("a", 0, "zed", "app", "main", 9_000_000, false), row("b", 1, "bob", "api", "feat", 9_900_000, false), row("c", 2, "amy", "app", "fix", 1_000, false), row("d", 3, "kim", "web", "main", 0, true)]
    }

    #[test]
    fn choices_read_their_saved_name_and_fall_back_to_today_s_behavior() {
        assert_eq!(GroupBy::read(Some(&json!("status"))), GroupBy::Status);
        assert_eq!(GroupBy::read(Some(&json!("nope"))), GroupBy::Project);
        assert_eq!(GroupBy::read(Some(&json!(7))), GroupBy::Project);
        assert_eq!(SortBy::read(None), SortBy::Manual);
        assert_eq!(SortBy::read(Some(&json!("agents"))), SortBy::Agents);
        assert_eq!(ProjectOrder::read(Some(&json!("recent"))), ProjectOrder::Recent);
        assert_eq!(ProjectOrder::read(Some(&json!(null))), ProjectOrder::Manual);
        for s in Status::ALL {
            assert_eq!(Status::parse(s.id()), Some(*s));
        }
        assert_eq!(View::default(), View { group_by: GroupBy::Project, sort_by: SortBy::Manual, project_order: ProjectOrder::Manual, filters: Filters::default() });
    }

    #[test]
    fn filters_survive_a_round_trip_and_bad_files_mean_none() {
        let f = Filters { hide_idle: true, hide_default_branch: false, hide_bots: true, projects: vec!["k".into()] };
        assert_eq!(Filters::read(Some(&f.to_value())), f);
        assert_eq!(f.count(), 3);
        assert_eq!(Filters::read(Some(&json!({ "hide_idle": true }))).count(), 1);
        for bad in [json!("x"), json!([1]), json!({ "projects": 3 }), json!(null)] {
            assert_eq!(Filters::read(Some(&bad)), Filters::default(), "{bad}");
        }
        assert_eq!(Filters::read(None).count(), 0);
    }

    #[test]
    fn statuses_and_ids_ignore_what_they_do_not_know() {
        let s = read_statuses(Some(&json!({ "a": "done", "b": "weird", "c": 4 })));
        assert_eq!(s.len(), 1);
        assert_eq!(s["a"], Status::Done);
        assert!(read_statuses(Some(&json!(["a"]))).is_empty());
        assert_eq!(read_ids(Some(&json!(["a", 1, "b"]))), BTreeSet::from(["a".to_owned(), "b".to_owned()]));
    }

    #[test]
    fn sorting_by_name_recent_project_agents_and_manual() {
        let (st, pin, bots) = (BTreeMap::new(), BTreeSet::new(), BTreeSet::new());
        let agents = [json!({ "space": "c", "status": "blocked" }), json!({ "space": "a", "status": "working" }), json!({ "space": "b", "status": "idle" })];
        let k = facts(&agents, &st, &pin, &bots);
        let mut rows = sample();
        sort_rows(&mut rows, SortBy::Name, &k);
        assert_eq!(ids(&rows), ["c", "b", "d", "a"]);
        sort_rows(&mut rows, SortBy::Recent, &k);
        assert_eq!(ids(&rows), ["b", "a", "c", "d"]);
        sort_rows(&mut rows, SortBy::Project, &k);
        assert_eq!(ids(&rows), ["b", "a", "c", "d"]);
        // Blocked first, then working, then the rest by how recently they were written to.
        sort_rows(&mut rows, SortBy::Agents, &k);
        assert_eq!(ids(&rows), ["c", "a", "b", "d"]);
        sort_rows(&mut rows, SortBy::Manual, &k);
        assert_eq!(ids(&rows), ["a", "b", "c", "d"]);
    }

    #[test]
    fn a_finished_agent_comes_before_a_working_one_and_an_idle_one_counts_for_nothing() {
        let (st, pin, bots) = (BTreeMap::new(), BTreeSet::new(), BTreeSet::new());
        let agents = [json!({ "space": "a", "status": "working" }), json!({ "space": "b", "status": "done" }), json!({ "space": "c", "status": "idle" })];
        let k = facts(&agents, &st, &pin, &bots);
        let mut rows = sample();
        sort_rows(&mut rows, SortBy::Agents, &k);
        assert_eq!(ids(&rows)[..2], ["b", "a"]);
    }

    #[test]
    fn filters_hide_rest_default_branch_bots_and_other_projects() {
        let (st, pin) = (BTreeMap::new(), BTreeSet::new());
        let bots = BTreeSet::from(["b".to_owned()]);
        let agents = [json!({ "space": "c", "status": "working" })];
        let k = facts(&agents, &st, &pin, &bots);
        let rows = sample();
        assert_eq!(ids(&filter_rows(&rows, &Filters::default(), &k)), ["a", "b", "c", "d"]);
        // a (written 1000 s ago) and b are within 30 min; c works; d was never written to.
        let idle = Filters { hide_idle: true, ..Filters::default() };
        assert_eq!(ids(&filter_rows(&rows, &idle, &k)), ["a", "b", "c"]);
        let late = Ctx { now: 9_900_000 + REST_AFTER_MS + 1, ..facts(&agents, &st, &pin, &bots) };
        assert_eq!(ids(&filter_rows(&rows, &idle, &late)), ["c"]);
        let branch = Filters { hide_default_branch: true, ..Filters::default() };
        assert_eq!(ids(&filter_rows(&rows, &branch, &k)), ["b", "c"]);
        let bot = Filters { hide_bots: true, ..Filters::default() };
        assert_eq!(ids(&filter_rows(&rows, &bot, &k)), ["a", "c", "d"]);
        let key = project_key(&rows[1]);
        let only = Filters { projects: vec![key], ..Filters::default() };
        assert_eq!(ids(&filter_rows(&rows, &only, &k)), ["b"]);
    }

    #[test]
    fn states_group_in_order_and_empty_ones_are_left_out() {
        let st = BTreeMap::from([("b".to_owned(), Status::Done), ("c".to_owned(), Status::Progress)]);
        let (pin, bots) = (BTreeSet::new(), BTreeSet::new());
        let k = facts(&[], &st, &pin, &bots);
        let groups = by_status(&sample(), &k);
        let shown: Vec<(Status, Vec<&str>)> = groups.iter().map(|(s, r)| (*s, ids(r))).collect();
        assert_eq!(shown, [(Status::Todo, vec!["a", "d"]), (Status::Progress, vec!["c"]), (Status::Done, vec!["b"])]);
        assert!(by_status(&[], &k).is_empty());
    }

    #[test]
    fn pinned_spaces_split_off_and_keep_their_order() {
        let pinned = BTreeSet::from(["c".to_owned(), "a".to_owned(), "gone".to_owned()]);
        let (top, rest) = split_pinned(sample(), &pinned);
        assert_eq!((ids(&top), ids(&rest)), (vec!["a", "c"], vec!["b", "d"]));
    }

    #[test]
    fn projects_order_by_recent_activity_or_stay_as_first_seen() {
        let mut groups = super::super::group_projects(&sample());
        let first: Vec<String> = groups.iter().map(|g| g.label.clone()).collect();
        order_projects(&mut groups, ProjectOrder::Manual, &[]);
        assert_eq!(groups.iter().map(|g| g.label.clone()).collect::<Vec<_>>(), first);
        order_projects(&mut groups, ProjectOrder::Recent, &[]);
        assert_eq!(groups.iter().map(|g| g.label.as_str()).collect::<Vec<_>>(), ["api", "app", "web"]);
        // The order a person dragged them into wins in Manual; a project it does not know stays last.
        let keys: Vec<String> = groups.iter().map(|g| g.key.clone()).collect();
        let saved = vec![keys[2].clone(), keys[0].clone()];
        order_projects(&mut groups, ProjectOrder::Manual, &saved);
        assert_eq!(groups.iter().map(|g| g.key.clone()).collect::<Vec<_>>(), [keys[2].clone(), keys[0].clone(), keys[1].clone()]);
    }

    #[test]
    fn folding_all_toggles_between_everything_folded_and_everything_open() {
        let keys: Vec<String> = ["p1", "p2"].iter().map(|s| s.to_string()).collect();
        let none: Vec<String> = Vec::new();
        assert_eq!(toggle_folded(&keys, &none), keys);
        // One already folded: the rest fold (it is not «all» yet).
        assert_eq!(toggle_folded(&keys, &["p1".to_owned()]), keys);
        // All folded (and an unrelated key): they open, the unrelated one stays.
        let all: Vec<String> = ["p1", "p2", "other"].iter().map(|s| s.to_string()).collect();
        assert_eq!(toggle_folded(&keys, &all), ["other"]);
        assert!(!all_folded(&[], &all));
    }

    #[test]
    fn menu_acts_change_one_thing_and_clear_only_the_filters() {
        let v = View::default();
        let v = apply(&v, &Act::Group(GroupBy::Status));
        let v = apply(&v, &Act::Sort(SortBy::Name));
        let v = apply(&v, &Act::Order(ProjectOrder::Recent));
        let v = apply(&v, &Act::HideIdle);
        let v = apply(&v, &Act::Project("k".into()));
        assert_eq!((v.group_by, v.sort_by, v.project_order, v.filters.count()), (GroupBy::Status, SortBy::Name, ProjectOrder::Recent, 2));
        // The same project twice takes it off again; the switch toggles.
        let off = apply(&apply(&v, &Act::Project("k".into())), &Act::HideIdle);
        assert_eq!(off.filters, Filters::default());
        let cleared = apply(&v, &Act::Clear);
        assert_eq!((cleared.filters.count(), cleared.group_by), (0, GroupBy::Status));
    }

    #[test]
    fn the_project_search_ignores_case_and_blanks() {
        assert!(matches_query("Sunna-App", "sun"));
        assert!(matches_query("Sunna-App", "  "));
        assert!(matches_query("x", ""));
        assert!(!matches_query("alfa", "zeta"));
    }

    #[test]
    fn the_menu_lists_every_choice_and_only_offers_clear_when_filters_are_on() {
        let projects = vec![("k1".to_owned(), "alfa".to_owned()), ("k2".to_owned(), "beta".to_owned())];
        let v = View::default();
        let list = entries(&v, &projects, "");
        let acts: Vec<&Act> = list.iter().filter_map(Entry::act).collect();
        assert_eq!(acts.iter().filter(|a| matches!(a, Act::Group(_))).count(), 4);
        assert_eq!(acts.iter().filter(|a| matches!(a, Act::Sort(_))).count(), 5);
        assert_eq!(acts.iter().filter(|a| matches!(a, Act::Order(_))).count(), 2);
        assert_eq!(acts.iter().filter(|a| matches!(a, Act::Project(_))).count(), 2);
        assert!(!acts.contains(&&Act::Clear) && acts.contains(&&Act::Reveal));
        // The shown choice is the one that is on.
        assert!(list.iter().any(|e| matches!(e, Entry::Item { act: Act::Group(GroupBy::Project), on: true, .. })));
        let on = apply(&v, &Act::HideBots);
        assert!(entries(&on, &projects, "").iter().any(|e| e.act() == Some(&Act::Clear)));
        // The search narrows the projects; no match says so.
        let narrow = entries(&v, &projects, "BET");
        assert_eq!(narrow.iter().filter(|e| matches!(e.act(), Some(Act::Project(_)))).count(), 1);
        assert!(entries(&v, &projects, "zzz").iter().any(|e| matches!(e, Entry::Note(_))));
        // No projects: no project section.
        assert!(!entries(&v, &[], "").iter().any(|e| matches!(e, Entry::Search(_))));
    }

    #[test]
    fn every_text_of_the_view_menu_exists_in_both_locales() {
        let mut keys: Vec<&str> = ["sidebar.options", "sidebar.group_by_title", "sidebar.sort_by_title", "sidebar.order_title", "sidebar.filters_title", "sidebar.filter_idle", "sidebar.filter_default", "sidebar.filter_bots", "sidebar.filter_project", "sidebar.search_project", "sidebar.no_projects", "sidebar.clear_filters", "sidebar.no_results", "sidebar.pinned", "sidebar.no_pr", "sidebar.collapse_all", "sidebar.expand_all", "sidebar.reveal"].to_vec();
        keys.extend(GroupBy::ALL.iter().map(|c| c.label()));
        keys.extend(SortBy::ALL.iter().map(|c| c.label()));
        keys.extend(ProjectOrder::ALL.iter().map(|c| c.label()));
        keys.extend(Status::ALL.iter().map(|c| c.label()));
        for lang in ["en", "es"] {
            let file = format!("{}/defaults/locales/{lang}.json", env!("CARGO_MANIFEST_DIR"));
            let table: Value = serde_json::from_str(&std::fs::read_to_string(&file).unwrap()).unwrap();
            for k in &keys {
                assert!(table[*k].as_str().is_some_and(|s| !s.is_empty()), "{lang}:{k}");
            }
        }
    }

    fn stops() -> Vec<What> {
        vec![What::Machine(None), What::Project("p".into()), What::Space("a".into()), What::Agent("x".into()), What::Space("b".into())]
    }

    #[test]
    fn the_cursor_starts_on_the_shown_space_else_the_first_row() {
        assert_eq!(settle(&stops(), None, Some("b")), Some(4));
        assert_eq!(settle(&stops(), None, Some("gone")), Some(0));
        assert_eq!(settle(&stops(), Some("space:a"), Some("b")), Some(2));
        // A row that left the list gives the cursor back to the shown space.
        assert_eq!(settle(&stops(), Some("space:gone"), Some("b")), Some(4));
        assert_eq!(settle(&[], None, None), None);
    }

    #[test]
    fn arrows_home_and_end_move_the_cursor_without_wrapping() {
        let s = stops();
        assert_eq!(cursor_to(&s, Some("space:a"), None, "down"), Some(3));
        assert_eq!(cursor_to(&s, Some("space:a"), None, "up"), Some(1));
        assert_eq!(cursor_to(&s, Some("machine:"), None, "up"), Some(0));
        assert_eq!(cursor_to(&s, Some("space:b"), None, "down"), Some(4));
        assert_eq!(cursor_to(&s, Some("space:a"), None, "home"), Some(0));
        assert_eq!(cursor_to(&s, Some("space:a"), None, "end"), Some(4));
        assert_eq!(cursor_to(&s, None, Some("b"), "up"), Some(3));
        assert_eq!(cursor_to(&s, Some("space:a"), None, "x"), None);
        assert_eq!(cursor_to(&[], None, None, "down"), None);
    }

    #[test]
    fn rows_have_distinct_keys_and_only_headers_fold() {
        let keys: BTreeSet<String> = stops().iter().map(What::key).collect();
        assert_eq!(keys.len(), 5);
        assert_eq!(What::Machine(Some("m".into())).key(), "machine:m");
        assert_eq!(What::Group("g1".into()).key(), "group:g1");
        assert!(What::Group("g1".into()).folds());
        assert!(What::Project("p".into()).folds() && What::Section("s".into()).folds() && What::Machine(None).folds());
        assert!(!What::Space("a".into()).folds() && !What::Agent("x".into()).folds());
    }

    #[test]
    fn a_row_out_of_view_scrolls_just_enough() {
        // Viewport 100..300.
        assert_eq!(scroll_delta(100.0, 300.0, 150.0, 182.0), 0.0);
        assert_eq!(scroll_delta(100.0, 300.0, 80.0, 112.0), 20.0);
        assert_eq!(scroll_delta(100.0, 300.0, 280.0, 312.0), -12.0);
        assert_eq!(scroll_delta(100.0, 300.0, 500.0, 532.0), -232.0);
        // Taller than the viewport: its top.
        assert_eq!(scroll_delta(100.0, 130.0, 140.0, 200.0), -40.0);
    }
}
