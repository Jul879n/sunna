//! Add project, native: the menu with its three ways (browse a folder, clone a URL, create a new one) and
//! the clone and create dialogs. Browse and every success end in `space.new {folder}`, so the new project
//! shows up the way any other space does.
//!
//! The menu is the sidebar's context menu; its entries run `project.add {step}` (`browse`, `clone`,
//! `create`). A mod that opens an overlay named `add-project` (`props.step`) gets the same (see
//! `open_overlay`), unless the person keeps a mod folder of that name. The rules (URL, names, folders)
//! are plain functions at the top, tested at the bottom.

use super::ctxmenu::Item;
use super::dialog::{self, Act, Dialog};
use super::native::z;
use super::*;
use crate::i18n;
use gpui_base::input::{InputEvent, InputState};

const FOLDER: &[u8] = include_bytes!("../../defaults/mods/workspace-chrome/folder.svg");
const BRANCH: &[u8] = include_bytes!("../../defaults/mods/workspace-chrome/branch.svg");
const PLUS: &[u8] = include_bytes!("../../defaults/mods/workspace-chrome/plus.svg");

// Rules ------------------------------------------------------------------------------

/// The folder name a clone lands in, from its URL (`owner/repo`, `…/repo.git`, `git@host:owner/repo`), or "".
pub(super) fn repo_name(url: &str) -> String {
    let last = url.trim().trim_end_matches(['/', '\\']).rsplit(['/', '\\', ':']).next().unwrap_or_default();
    last.strip_suffix(".git").unwrap_or(last).to_owned()
}

fn word(c: char) -> bool {
    c.is_ascii_alphanumeric() || matches!(c, '_' | '.' | '-')
}

/// What `git clone` takes from `text`: a URL, an scp-style address, or `owner/repo` (GitHub); None otherwise.
pub(super) fn clone_url(text: &str) -> Option<String> {
    let s = text.trim();
    if s.is_empty() || s.chars().any(char::is_whitespace) {
        return None;
    }
    if ["https://", "http://", "ssh://", "git://"].iter().any(|scheme| s.strip_prefix(scheme).is_some_and(|rest| !rest.is_empty())) {
        return Some(s.to_owned());
    }
    if let Some((user, rest)) = s.split_once('@')
        && let Some((host, path)) = rest.split_once(':')
        && !user.is_empty()
        && user.chars().all(word)
        && !host.is_empty()
        && host.chars().all(word)
        && !path.is_empty()
    {
        return Some(s.to_owned());
    }
    match s.split_once('/') {
        Some((owner, repo)) if !owner.is_empty() && !repo.is_empty() && owner.chars().all(word) && repo.chars().all(word) => Some(format!("https://github.com/{s}.git")),
        _ => None,
    }
}

/// Why a project name can't be used, as a locale key.
pub(super) fn name_problem(name: &str) -> Option<&'static str> {
    let n = name.trim();
    if n.is_empty() {
        return Some("add-project.name_empty");
    }
    if n.contains(['/', '\\']) || n == "." || n == ".." {
        return Some("add-project.name_invalid");
    }
    None
}

/// Why a parent folder can't be used, as a locale key.
pub(super) fn parent_problem(parent: &str) -> Option<&'static str> {
    let p = parent.trim();
    if p.is_empty() {
        return Some("add-project.parent_required");
    }
    let drive = p.len() >= 3 && p.as_bytes()[0].is_ascii_alphabetic() && p.as_bytes()[1] == b':' && matches!(p.as_bytes()[2], b'/' | b'\\');
    if !p.starts_with('/') && !drive {
        return Some("add-project.parent_not_absolute");
    }
    None
}

/// `parent/child` with the separator the parent uses.
pub(super) fn join_path(parent: &str, child: &str) -> String {
    let p = parent.trim().trim_end_matches(['/', '\\']);
    let sep = if parent.contains('\\') && !parent.contains('/') { '\\' } else { '/' };
    format!("{p}{sep}{}", child.trim())
}

/// `~/sunna/projects`: where new projects and clones go unless the person picks another folder.
pub(super) fn default_parent(home: &str) -> String {
    join_path(home.trim_end_matches(['/', '\\']), "sunna/projects")
}

/// The folder a parent field names: typed text, with `~` as the home.
pub(super) fn expand_home(text: &str, home: &str) -> String {
    let t = text.trim();
    if t == "~" {
        home.to_owned()
    } else if let Some(rest) = t.strip_prefix("~/") {
        join_path(home, rest)
    } else {
        t.to_owned()
    }
}

fn last_lines(text: &str, n: usize) -> String {
    let lines: Vec<&str> = text.trim().lines().collect();
    lines[lines.len().saturating_sub(n)..].join("\n")
}

/// A failure the dialog shows: a locale key and its variables.
type Failure = (&'static str, Vec<(&'static str, String)>);

fn fail(key: &'static str) -> Failure {
    (key, Vec::new())
}

/// Clones `url` into `parent/<repo>`; the folder to open.
fn clone_into(url: &str, parent: &str) -> Result<String, Failure> {
    let Some(url) = clone_url(url) else { return Err(fail("add-project.url_invalid")) };
    if let Some(bad) = parent_problem(parent) {
        return Err(fail(bad));
    }
    let name = repo_name(&url);
    let dest = join_path(parent, &name);
    if std::fs::read_dir(&dest).is_ok() {
        return Err(("add-project.target_exists", vec![("name", name)]));
    }
    std::fs::create_dir_all(parent).map_err(|e| ("add-project.failed", vec![("detail", e.to_string())]))?;
    let args: Vec<String> = ["clone", "--", &url, &dest].iter().map(|s| (*s).to_owned()).collect();
    let (code, _, err) = mods::run_blocking("git", &args, None, Duration::from_secs(600)).map_err(|e| ("add-project.failed", vec![("detail", last_lines(&e, 4))]))?;
    if code != 0 {
        return Err(("add-project.clone_failed", vec![("detail", last_lines(&err, 4))]));
    }
    Ok(dest)
}

/// Creates `parent/name` as a git repository, with a first commit when git knows who the person is.
/// The folder to open, and whether the first commit was skipped for want of an identity.
fn create_in(name: &str, parent: &str) -> Result<(String, bool), Failure> {
    if let Some(bad) = name_problem(name).or_else(|| parent_problem(parent)) {
        return Err(fail(bad));
    }
    let git = |args: &[&str], cwd: Option<&str>| mods::run_blocking("git", &args.iter().map(|s| (*s).to_owned()).collect::<Vec<_>>(), cwd.map(str::to_owned), Duration::from_secs(60));
    if !matches!(git(&["--version"], None), Ok((0, _, _))) {
        return Err(fail("add-project.git_missing"));
    }
    let name = name.trim();
    let dest = join_path(parent, name);
    if std::fs::read_dir(&dest).is_ok_and(|mut d| d.next().is_some()) {
        return Err(("add-project.target_not_empty", vec![("name", name.to_owned())]));
    }
    std::fs::create_dir_all(&dest).map_err(|e| ("add-project.failed", vec![("detail", e.to_string())]))?;
    match git(&["init"], Some(&dest)) {
        Ok((0, _, _)) => {}
        Ok((_, _, err)) => return Err(("add-project.init_failed", vec![("detail", last_lines(&err, 4))])),
        Err(e) => return Err(("add-project.init_failed", vec![("detail", last_lines(&e, 4))])),
    }
    // The first commit needs an identity; without one the repo stays empty and says so.
    let known = ["user.name", "user.email"].iter().all(|k| matches!(git(&["config", k], Some(&dest)), Ok((0, out, _)) if !out.trim().is_empty()));
    if known {
        let _ = git(&["commit", "--allow-empty", "-m", "Initial commit"], Some(&dest));
    }
    Ok((dest, !known))
}

// The dialogs ------------------------------------------------------------------------

/// The field a failure is about (its outline turns red next to the message).
#[derive(Clone, Copy, PartialEq, Debug)]
enum Culprit {
    First,
    Parent,
}

/// Which field a failure's locale key points at; none when it is about neither (git itself failed).
fn culprit(key: &str) -> Option<Culprit> {
    match key {
        "add-project.url_invalid" | "add-project.name_empty" | "add-project.name_invalid" | "add-project.target_exists" | "add-project.target_not_empty" => Some(Culprit::First),
        "add-project.parent_required" | "add-project.parent_not_absolute" => Some(Culprit::Parent),
        _ => None,
    }
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub(super) enum Step {
    Clone,
    Create,
}

pub(super) struct AddView {
    app: WeakEntity<AppWindow>,
    id: String,
    step: Step,
    /// The URL (clone) or the name (create).
    first: Entity<InputState>,
    parent: Entity<InputState>,
    busy: bool,
    error: Option<String>,
    culprit: Option<Culprit>,
    focus: dialog::Focus,
}

impl AddView {
    fn new(app: WeakEntity<AppWindow>, id: String, step: Step, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let on = |this: &mut Self, ev: &InputEvent, window: &mut Window, cx: &mut Context<Self>| match ev {
            InputEvent::PressEnter { .. } => this.submit(window, cx),
            InputEvent::Change => {
                this.error = None;
                this.culprit = None;
                cx.notify();
            }
            _ => {}
        };
        let hint = if step == Step::Clone { "https://github.com/owner/repo.git".to_owned() } else { i18n::t("add-project.name_hint") };
        let first = dialog::input("", &hint, window, cx, on);
        let parent = dialog::input(&default_parent(&mods::home().to_string_lossy()), "~/sunna/projects", window, cx, on);
        first.update(cx, |s, cx| s.focus(window, cx));
        Self { app, id, step, first, parent, busy: false, error: None, culprit: None, focus: dialog::Focus::new(cx) }
    }

    fn close(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let id = self.id.clone();
        let _ = self.app.update(cx, |app, cx| app.close_overlay(&id, window, cx));
    }

    fn parent_path(&self, cx: &App) -> String {
        expand_home(&self.parent.read(cx).value(), &mods::home().to_string_lossy())
    }

    fn pick_parent(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let paths = cx.prompt_for_paths(PathPromptOptions { files: false, directories: true, multiple: true, prompt: Some(i18n::t("add-project.choose").into()) });
        cx.spawn_in(window, async move |this, cx| {
            if let Ok(Ok(Some(paths))) = paths.await
                && let Some(folder) = paths.first()
            {
                let folder = folder.to_string_lossy().into_owned();
                let _ = this.update_in(cx, |this, window, cx| this.parent.update(cx, |s, cx| s.set_value(folder, window, cx)));
            }
        })
        .detach();
    }

    fn submit(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.busy {
            return;
        }
        self.busy = true;
        self.error = None;
        self.culprit = None;
        cx.notify();
        let (step, first, parent) = (self.step, self.first.read(cx).value().to_string(), self.parent_path(cx));
        cx.spawn_in(window, async move |this, cx| {
            let done = cx
                .background_executor()
                .spawn(async move {
                    match step {
                        Step::Clone => clone_into(&first, &parent).map(|dest| (dest, false)),
                        Step::Create => create_in(&first, &parent),
                    }
                })
                .await;
            let _ = this.update_in(cx, |this, window, cx| {
                this.busy = false;
                match done {
                    Ok((folder, no_identity)) => {
                        let app = this.app.clone();
                        let id = this.id.clone();
                        let _ = app.update(cx, |app, cx| {
                            app.close_overlay(&id, window, cx);
                            if no_identity {
                                app.toast(i18n::t("add-project.no_identity"), None, "warning", window, cx);
                            }
                            app.run("space.new", json!({ "folder": folder }), window, cx);
                        });
                    }
                    Err((key, vars)) => {
                        let vars: Vec<(&str, &str)> = vars.iter().map(|(k, v)| (*k, v.as_str())).collect();
                        this.error = Some(i18n::tf(key, &vars));
                        this.culprit = culprit(key);
                    }
                }
                cx.notify();
            });
        })
        .detach();
    }
}

impl Render for AddView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let this = cx.entity();
        let clone = self.step == Step::Clone;
        let t = |clone_key: &str, create_key: &str| i18n::t(if clone { clone_key } else { create_key });
        let body = div()
            .flex()
            .flex_col()
            .gap(z(12.0))
            .child(dialog::field(t("add-project.url", "add-project.name"), &self.first, self.culprit == Some(Culprit::First), window, cx))
            .child(self.parent_field(&this, window, cx))
            .when_some(self.error.clone(), |d, e| d.child(dialog::message(e, true)));
        let go = if self.busy { t("add-project.cloning", "add-project.creating") } else { t("add-project.clone_go", "add-project.create_go") };
        Dialog::new(t("add-project.clone_title", "add-project.create_title"), &self.focus, dialog::run(&this, Self::close))
            .wide()
            .fields(2)
            .description(t("add-project.clone_intro", "add-project.create_intro"))
            .body(body)
            .cancel_label(i18n::t("settings.cancel"))
            .action(Act::primary(go, dialog::run(&this, Self::submit)).enabled(!self.busy))
            .render("add-project")
    }
}

impl AddView {
    /// The parent folder's label, input and the Choose folder button on one line.
    fn parent_field(&self, this: &Entity<Self>, window: &Window, cx: &App) -> Div {
        let focused = self.parent.read(cx).focus_handle(cx).is_focused(window);
        div().flex().flex_col().gap(z(6.0)).child(dialog::label(i18n::t("add-project.parent"))).child(
            div()
                .flex()
                .flex_row()
                .gap(z(8.0))
                .child(div().flex_1().min_w_0().child(dialog::field_box(focused, self.culprit == Some(Culprit::Parent)).child(div().flex_1().h_full().child(gpui_base::input::Input::new(&self.parent)))))
                .child(dialog::side_button("add-project-pick", i18n::t("add-project.choose"), dialog::run(this, Self::pick_parent))),
        )
    }
}

impl AppWindow {
    pub(super) fn open_add_project(&mut self, step: Step, window: &mut Window, cx: &mut Context<Self>) {
        let id = "add-project".to_owned();
        if self.overlays.iter().any(|o| o.id == id) {
            self.close_overlay(&id, window, cx);
        }
        let app = cx.weak_entity();
        let view = cx.new(|cx| AddView::new(app, id.clone(), step, window, cx));
        self.push_dialog_overlay(id, dialog::WIDE + 48.0, view.into(), window, cx);
    }

    /// The three ways to add a project, as the sidebar's menu at the pointer.
    pub(super) fn open_add_project_menu(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let entry = |icon: &'static [u8], label: String, step: &str| Item::run(label, Some(icon), "project.add", json!({ "step": step }));
        // Where a new project lands: the active space's machine (what `space.new` uses for a person); none on this computer.
        let heading = match self.model.active_space().and_then(|s| s.machine.as_deref()).filter(|m| !m.is_empty()) {
            Some(machine) => i18n::tf("add-project.menu_title_on", &[("machine", machine)]),
            None => i18n::t("add-project.menu_title"),
        };
        let items = vec![
            Item::heading(heading),
            entry(FOLDER, i18n::t("add-project.browse"), "browse"),
            entry(BRANCH, i18n::t("add-project.clone"), "clone"),
            entry(PLUS, i18n::t("add-project.create"), "create"),
        ];
        self.open_context_menu(items, window, cx);
    }

    /// `project.add {step}`: browse (a folder picker, then `space.new`), clone or create (their dialogs).
    pub(super) fn project_add(&mut self, step: &str, window: &mut Window, cx: &mut Context<Self>) -> Result<(), String> {
        match step {
            "" | "menu" => self.open_add_project_menu(window, cx),
            "clone" => self.open_add_project(Step::Clone, window, cx),
            "create" => self.open_add_project(Step::Create, window, cx),
            "browse" => {
                let paths = cx.prompt_for_paths(PathPromptOptions { files: false, directories: true, multiple: false, prompt: Some(i18n::t("add-project.browse").into()) });
                cx.spawn_in(window, async move |this, cx| {
                    if let Ok(Ok(Some(paths))) = paths.await {
                        // One space per folder, in the order picked (the last one stays active).
                        let folders: Vec<String> = paths.iter().map(|p| p.to_string_lossy().into_owned()).collect();
                        let _ = this.update_in(cx, |this, window, cx| this.run("project.add", json!({ "folders": folders }), window, cx));
                    }
                })
                .detach();
            }
            other => return Err(format!("project.add step is menu, browse, clone or create, not {other:?}")),
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    // `super::*` brings gpui's own `test` macro: the plain one is the std attribute.
    use core::prelude::v1::test;

    #[test]
    fn a_failure_marks_the_field_it_is_about() {
        assert_eq!(culprit("add-project.url_invalid"), Some(Culprit::First));
        assert_eq!(culprit("add-project.target_exists"), Some(Culprit::First));
        assert_eq!(culprit("add-project.parent_not_absolute"), Some(Culprit::Parent));
        // Every problem the validators name points at a field; git's own failures at none.
        for key in [name_problem("").unwrap(), name_problem("a/b").unwrap()] {
            assert_eq!(culprit(key), Some(Culprit::First), "{key}");
        }
        for key in [parent_problem("").unwrap(), parent_problem("relative/dir").unwrap()] {
            assert_eq!(culprit(key), Some(Culprit::Parent), "{key}");
        }
        assert_eq!(culprit("add-project.clone_failed"), None);
    }

    #[test]
    fn a_clone_lands_in_a_folder_named_after_the_repository() {
        assert_eq!(repo_name("https://github.com/owner/repo.git"), "repo");
        assert_eq!(repo_name("git@github.com:owner/repo.git"), "repo");
        assert_eq!(repo_name("owner/repo"), "repo");
        assert_eq!(repo_name("https://host/a/b/"), "b");
        assert_eq!(repo_name(""), "");
    }

    #[test]
    fn only_something_git_clone_takes_is_a_clone_url_and_owner_repo_means_github() {
        assert_eq!(clone_url("https://github.com/o/r.git").as_deref(), Some("https://github.com/o/r.git"));
        assert_eq!(clone_url("ssh://git@host/o/r").as_deref(), Some("ssh://git@host/o/r"));
        assert_eq!(clone_url("git@host:o/r.git").as_deref(), Some("git@host:o/r.git"));
        assert_eq!(clone_url("o/r").as_deref(), Some("https://github.com/o/r.git"));
        assert_eq!(clone_url("not a url"), None);
        assert_eq!(clone_url(""), None);
        assert_eq!(clone_url("justaword"), None);
        assert_eq!(clone_url("https://"), None);
    }

    #[test]
    fn project_names_reject_empty_values_and_path_separators() {
        assert_eq!(name_problem(""), Some("add-project.name_empty"));
        assert_eq!(name_problem("  "), Some("add-project.name_empty"));
        assert_eq!(name_problem("a/b"), Some("add-project.name_invalid"));
        assert_eq!(name_problem("a\\b"), Some("add-project.name_invalid"));
        assert_eq!(name_problem(".."), Some("add-project.name_invalid"));
        assert_eq!(name_problem("my-app"), None);
    }

    #[test]
    fn the_parent_folder_must_be_given_and_absolute() {
        assert_eq!(parent_problem(""), Some("add-project.parent_required"));
        assert_eq!(parent_problem("projects"), Some("add-project.parent_not_absolute"));
        assert_eq!(parent_problem("/home/u/projects"), None);
        assert_eq!(parent_problem("C:\\Users\\u"), None);
    }

    #[test]
    fn paths_join_with_the_parents_separator_and_projects_default_under_sunna_projects() {
        assert_eq!(join_path("/a/b/", "c"), "/a/b/c");
        assert_eq!(join_path("C:\\a", "c"), "C:\\a\\c");
        assert_eq!(default_parent("/Users/me/"), "/Users/me/sunna/projects");
        assert_eq!(expand_home("~", "/h"), "/h");
        assert_eq!(expand_home("~/x", "/h"), "/h/x");
        assert_eq!(expand_home(" /abs ", "/h"), "/abs");
    }

    #[test]
    fn create_makes_a_git_repository_and_refuses_a_folder_with_files() {
        let tmp = std::env::temp_dir().join(format!("sunna-addproject-{}", std::process::id()));
        let parent = tmp.to_string_lossy().into_owned();
        let (dest, _) = create_in("proj", &parent).expect("created");
        assert!(std::path::Path::new(&dest).join(".git").is_dir());
        assert_eq!(create_in("proj", &parent).unwrap_err().0, "add-project.target_not_empty");
        assert_eq!(create_in("a/b", &parent).unwrap_err().0, "add-project.name_invalid");
        assert_eq!(clone_into("not a url", &parent).unwrap_err().0, "add-project.url_invalid");
        let _ = std::fs::remove_dir_all(&tmp);
    }
}
