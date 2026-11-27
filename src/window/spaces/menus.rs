//! What the sidebar's menus offer (right-click on a space, a project header or a machine
//! header, and the «⋯» button) and the commands behind each line. Each line is a command
//! (`space.read`, `space.open_in`, `project.icon`, `machine.check`, ...), so `sunna do` can do
//! all of it; the menu itself is `ctxmenu::MenuView`. What is chosen, what is built per
//! context (a main checkout has no «Remove worktree», a remote space no «Open in») and what is
//! launched are plain functions, tested at the end.

use super::super::ctxmenu::Item;
use super::*;
use super::view::Status;

const MAIL: &[u8] = include_bytes!("../../../assets/icons/menu/mail.svg");
const MAIL_OPEN: &[u8] = include_bytes!("../../../assets/icons/menu/mail-open.svg");
const KANBAN: &[u8] = include_bytes!("../../../assets/icons/menu/kanban.svg");
const COPY: &[u8] = include_bytes!("../../../assets/icons/menu/copy.svg");
const EXTERNAL: &[u8] = include_bytes!("../../../assets/icons/menu/external.svg");
const TRASH: &[u8] = include_bytes!("../../../assets/icons/menu/trash.svg");
const IMAGE: &[u8] = include_bytes!("../../../assets/icons/menu/image.svg");
const REFRESH: &[u8] = include_bytes!("../../../assets/icons/menu/refresh.svg");
const BUG: &[u8] = include_bytes!("../../../assets/icons/menu/bug.svg");
pub(super) const ELLIPSIS: &[u8] = include_bytes!("../../../assets/icons/menu/ellipsis.svg");
const PEN: &[u8] = include_bytes!("../../../defaults/mods/workspace-chrome/square-pen.svg");
const FOLDER: &[u8] = include_bytes!("../../../defaults/mods/workspace-chrome/folder.svg");
const TERMINAL: &[u8] = include_bytes!("../../../defaults/mods/workspace-chrome/terminal.svg");
const X: &[u8] = include_bytes!("../../../defaults/mods/workspace-chrome/x.svg");
const PIN_OFF: &[u8] = include_bytes!("../../../defaults/mods/workspace-chrome/pin-off.svg");
const SETTINGS: &[u8] = include_bytes!("../../../defaults/mods/workspace-chrome/settings.svg");
const MINUS: &[u8] = include_bytes!("../../../defaults/mods/workspace-chrome/minus.svg");

/// The commands this file runs (the docs and the tests name each one).
pub(crate) const COMMANDS: [&str; 15] = [
    "space.read", "space.open_in", "space.copy", "space.details", "space.remove", "project.icon", "project.hide", "project.open_in", "project.copy", "project.remove", "project.worktree", "machine.rename", "machine.check", "machine.remove", "sidebar.menu",
];

// What to open a folder with -----------------------------------------------------

/// An editor Sunna looks for: its name, the macOS app, the Linux command, and the Windows program folder and exe (under Programs or Program Files).
struct Editor {
    label: &'static str,
    mac: &'static str,
    linux: &'static str,
    windows: &'static str,
}

const EDITORS: [Editor; 8] = [
    Editor { label: "Visual Studio Code", mac: "Visual Studio Code", linux: "code", windows: "Microsoft VS Code\\Code.exe" },
    Editor { label: "Cursor", mac: "Cursor", linux: "cursor", windows: "cursor\\Cursor.exe" },
    Editor { label: "Zed", mac: "Zed", linux: "zed", windows: "Zed\\Zed.exe" },
    Editor { label: "Windsurf", mac: "Windsurf", linux: "windsurf", windows: "Windsurf\\Windsurf.exe" },
    Editor { label: "Sublime Text", mac: "Sublime Text", linux: "subl", windows: "Sublime Text\\sublime_text.exe" },
    Editor { label: "Xcode", mac: "Xcode", linux: "", windows: "" },
    Editor { label: "IntelliJ IDEA", mac: "IntelliJ IDEA", linux: "idea", windows: "JetBrains\\IntelliJ IDEA\\bin\\idea64.exe" },
    Editor { label: "WebStorm", mac: "WebStorm", linux: "webstorm", windows: "JetBrains\\WebStorm\\bin\\webstorm64.exe" },
];

/// The editors found on this computer: `(name, what to launch it by)`. `os` is "macos", "linux" or "windows";
/// `exists` says whether a path is there, `on_path` whether a command is.
pub(super) fn find_editors(os: &str, exists: &dyn Fn(&str) -> bool, on_path: &dyn Fn(&str) -> bool, roots: &[String], home: &str) -> Vec<(String, String)> {
    EDITORS
        .iter()
        .filter_map(|e| {
            let found = match os {
                "macos" => [format!("/Applications/{}.app", e.mac), format!("{home}/Applications/{}.app", e.mac)].iter().any(|p| exists(p)).then(|| e.mac.to_owned()),
                "windows" if !e.windows.is_empty() => roots.iter().map(|r| format!("{r}\\{}", e.windows)).find(|p| exists(p)),
                "linux" if !e.linux.is_empty() => on_path(e.linux).then(|| e.linux.to_owned()),
                _ => None,
            };
            found.map(|launch| (e.label.to_owned(), launch))
        })
        .collect()
}

fn on_path(program: &str) -> bool {
    std::env::var_os("PATH").is_some_and(|paths| std::env::split_paths(&paths).any(|dir| dir.join(program).is_file()))
}

fn this_os() -> &'static str {
    if cfg!(target_os = "macos") {
        "macos"
    } else if cfg!(windows) {
        "windows"
    } else {
        "linux"
    }
}

/// The editors of this computer, looked for once.
fn editors() -> &'static [(String, String)] {
    static FOUND: std::sync::OnceLock<Vec<(String, String)>> = std::sync::OnceLock::new();
    FOUND.get_or_init(|| {
        let home = mods::home().to_string_lossy().into_owned();
        let mut roots = Vec::new();
        if let Ok(r) = std::env::var("LOCALAPPDATA") {
            roots.push(format!("{r}\\Programs"));
        }
        if let Ok(r) = std::env::var("ProgramFiles") {
            roots.push(r);
        }
        find_editors(this_os(), &|p| std::path::Path::new(p).exists(), &on_path, &roots, &home)
    })
}

/// «Open in» apps: the editors found, then the ones added in Settings > General (`menu.open_in`) that were not.
pub(super) fn open_apps() -> Vec<(String, String)> {
    let mut apps: Vec<(String, String)> = editors().to_vec();
    for app in shared_list("menu.open_in") {
        if !apps.iter().any(|(label, launch)| *label == app || *launch == app) {
            apps.push((app.clone(), app));
        }
    }
    apps
}

/// What runs to open `folder` with `app`: "folder" (the file manager), "terminal", or an app (macOS: its name; elsewhere its command).
pub(super) fn open_command(os: &str, folder: &str, app: &str, has: &dyn Fn(&str) -> bool) -> Option<(String, Vec<String>)> {
    let s = |v: &[&str]| v.iter().map(|x| (*x).to_owned()).collect::<Vec<String>>();
    Some(match (app, os) {
        ("folder", "macos") => ("open".into(), s(&[folder])),
        ("folder", "windows") => ("explorer".into(), s(&[folder])),
        ("folder", _) => ("xdg-open".into(), s(&[folder])),
        ("terminal", "macos") => ("open".into(), s(&["-a", "Terminal", folder])),
        ("terminal", "windows") => ("cmd".into(), s(&["/c", "start", "", "/D", folder, "cmd"])),
        ("terminal", _) => {
            let (program, args): (&str, Vec<String>) = if has("gnome-terminal") {
                ("gnome-terminal", vec![format!("--working-directory={folder}")])
            } else if has("konsole") {
                ("konsole", s(&["--workdir", folder]))
            } else if has("xfce4-terminal") {
                ("xfce4-terminal", vec![format!("--working-directory={folder}")])
            } else {
                return None;
            };
            (program.into(), args)
        }
        (app, "macos") => ("open".into(), s(&["-a", app, folder])),
        (app, _) => (app.into(), s(&[folder])),
    })
}

/// Starts a program and leaves it running (its exit is collected so it never lingers).
fn launch(program: &str, args: &[String]) -> Result<(), String> {
    let mut child = std::process::Command::new(program).args(args).stdin(std::process::Stdio::null()).stdout(std::process::Stdio::null()).stderr(std::process::Stdio::null()).spawn().map_err(|e| format!("{program}: {e}"))?;
    std::thread::spawn(move || {
        let _ = child.wait();
    });
    Ok(())
}

// What each menu holds ------------------------------------------------------------

/// What a space's menu needs to know.
pub(super) struct SpaceCtx {
    pub id: String,
    /// Where it is now (its first terminal's folder, else where it was made).
    pub folder: String,
    pub machine: Option<String>,
    pub branch: Option<String>,
    /// A git worktree that is not the repo's main checkout.
    pub linked: bool,
    pub pinned: bool,
    pub unread: bool,
    pub status: Status,
    pub apps: Vec<(String, String)>,
    /// Alt was held: the Developer submenu.
    pub dev: bool,
    /// A space in a git repo can start a worktree from itself.
    pub repo: bool,
    /// Its parent and its children in the lineage: `(id, name)`.
    pub parent: Option<(String, String)>,
    pub children: Vec<(String, String)>,
}

fn open_in_items(command: &str, who: Value, apps: &[(String, String)]) -> Item {
    let with_app = |app: &str| {
        let mut args = who.clone();
        args["app"] = json!(app);
        args
    };
    let mut items = vec![Item::run(i18n::t("menu.reveal_folder"), Some(FOLDER), command, with_app("folder")), Item::run(i18n::t("ctx.terminal"), Some(TERMINAL), command, with_app("terminal"))];
    if !apps.is_empty() {
        items.push(Item::Sep);
    }
    items.extend(apps.iter().map(|(label, launch)| Item::run(label.clone(), None, command, with_app(launch))));
    Item::sub(i18n::t("ctx.open_in"), Some(EXTERNAL), items)
}

pub(super) fn space_items(c: &SpaceCtx) -> Vec<Item> {
    let id = json!({ "id": c.id });
    let local = c.machine.is_none() && !c.folder.is_empty();
    let name = c.folder.trim_end_matches('/').rsplit(['/', '\\']).next().unwrap_or_default();
    let mut out = vec![
        Item::run(i18n::t(if c.unread { "ctx.mark_read" } else { "ctx.mark_unread" }), Some(if c.unread { MAIL_OPEN } else { MAIL }), "space.read", json!({ "id": c.id, "read": c.unread })),
        Item::sub(
            i18n::t("ctx.move_to"),
            Some(KANBAN),
            Status::ALL.iter().map(|s| Item::run(i18n::t(s.label()), None, "space.status", json!({ "id": c.id, "status": s.id() })).checked(*s == c.status)).collect(),
        ),
        Item::run(i18n::t("ctx.edit_details"), Some(PEN), "space.details", id.clone()),
        Item::run(i18n::t(if c.pinned { "ctx.unpin" } else { "ctx.pin" }), Some(if c.pinned { PIN_OFF } else { PIN }), "space.pin", json!({ "id": c.id, "pinned": !c.pinned })),
        Item::Sep,
    ];
    if c.repo {
        out.push(Item::run(i18n::t("lineage.new_here"), Some(BRANCH), "space.worktree", id.clone()));
    }
    if c.parent.is_some() || !c.children.is_empty() {
        let mut lineage = Vec::new();
        if let Some((parent, name)) = &c.parent {
            lineage.push(Item::run(i18n::tf("lineage.open_parent", &[("name", name)]), None, "space.select", json!({ "id": parent })));
            lineage.push(Item::run(i18n::t("lineage.remove_parent"), None, "space.parent", json!({ "id": c.id, "parent": "" })));
        }
        if c.parent.is_some() && !c.children.is_empty() {
            lineage.push(Item::Sep);
        }
        lineage.extend(c.children.iter().map(|(child, name)| Item::run(i18n::tf("lineage.open_child", &[("name", name)]), None, "space.select", json!({ "id": child }))));
        out.push(Item::sub(i18n::t("lineage.title"), Some(GIT_BRANCH), lineage));
    }
    if local {
        out.push(open_in_items("space.open_in", id.clone(), &c.apps));
    }
    let mut copy = vec![Item::run(i18n::t("ctx.path"), None, "space.copy", json!({ "id": c.id, "what": "path" }))];
    if !name.is_empty() {
        copy.push(Item::run(i18n::t("ctx.worktree_name"), None, "space.copy", json!({ "id": c.id, "what": "name" })));
    }
    if c.branch.is_some() {
        copy.push(Item::run(i18n::t("ctx.branch"), None, "space.copy", json!({ "id": c.id, "what": "branch" })));
    }
    out.push(Item::sub(i18n::t("ctx.copy"), Some(COPY), copy));
    out.push(Item::Sep);
    out.push(Item::run(i18n::t("ctx.close_space"), Some(X), "space.close", id.clone()));
    if c.linked {
        out.push(Item::run(i18n::t("ctx.remove_worktree"), Some(TRASH), "space.remove", id.clone()).danger());
    }
    if c.dev {
        out.push(Item::Sep);
        out.push(Item::sub(
            i18n::t("ctx.developer"),
            Some(BUG),
            vec![Item::run(i18n::t("ctx.copy_space_id"), None, "space.copy", json!({ "id": c.id, "what": "id" })), Item::run(i18n::t("ctx.copy_core_id"), None, "space.copy", json!({ "id": c.id, "what": "core" }))],
        ));
    }
    out
}

/// What a project's menu needs to know.
pub(super) struct ProjectCtx {
    pub key: String,
    /// The repo's main checkout, else the first space's folder.
    pub folder: String,
    pub machine: Option<String>,
    pub repo: bool,
    pub custom_icon: bool,
    pub apps: Vec<(String, String)>,
    /// The groups there are, `(id, name)`, and the one the project is in.
    pub groups: Vec<(String, String)>,
    pub in_group: Option<String>,
}

pub(super) fn project_items(c: &ProjectCtx) -> Vec<Item> {
    let who = json!({ "key": c.key });
    let mut out = vec![Item::run(i18n::t("ctx.project_settings"), Some(SETTINGS), "app.settings", json!({ "section": "git" }))];
    if c.repo {
        out.push(Item::run(i18n::t("menu.new_worktree"), Some(BRANCH), "project.worktree", who.clone()));
    }
    out.push(Item::Sep);
    if c.machine.is_none() && !c.folder.is_empty() {
        out.push(open_in_items("project.open_in", who.clone(), &c.apps));
    }
    out.push(Item::run(i18n::t("menu.copy_path"), Some(COPY), "project.copy", json!({ "key": c.key })));
    out.push(Item::run(i18n::t("ctx.change_icon"), Some(IMAGE), "project.icon", who.clone()));
    if c.custom_icon {
        out.push(Item::run(i18n::t("ctx.reset_icon"), Some(REFRESH), "project.icon", json!({ "key": c.key, "clear": true })));
    }
    out.push(Item::Sep);
    out.push(Item::run(i18n::t("groups.new_from_project"), Some(FOLDER), "group.new", who.clone()));
    if !c.groups.is_empty() {
        out.push(Item::sub(i18n::t("groups.move_to"), Some(FOLDER), c.groups.iter().map(|(id, name)| Item::run(name.clone(), None, "group.move", json!({ "key": c.key, "group": id })).checked(c.in_group.as_deref() == Some(id.as_str()))).collect()));
    }
    if c.in_group.is_some() {
        out.push(Item::run(i18n::t("groups.remove_from"), Some(X), "group.move", json!({ "key": c.key, "group": "" })));
    }
    out.push(Item::Sep);
    out.push(Item::run(i18n::t("ctx.hide_sidebar"), Some(MINUS), "project.hide", json!({ "key": c.key, "hidden": true })));
    out.push(Item::run(i18n::t("ctx.remove_project"), Some(TRASH), "project.remove", who).danger());
    out
}

/// A machine's menu (this computer has none): what `hc machine` can do. There is no disconnect without forgetting it.
pub(super) fn machine_items(name: &str) -> Vec<Item> {
    vec![
        Item::run(i18n::t("ctx.rename"), Some(PEN), "machine.rename", json!({ "machine": name })),
        Item::run(i18n::t("ctx.check_connection"), Some(REFRESH), "machine.check", json!({ "machine": name })),
        Item::run(i18n::t("ctx.manage_machines"), Some(SETTINGS), "app.settings", json!({ "section": "machines" })),
        Item::Sep,
        Item::run(i18n::t("ctx.remove_machine"), Some(TRASH), "machine.remove", json!({ "machine": name })).danger(),
    ]
}

/// A group's menu: rename, its color, look at its projects again, delete it (its projects stay).
pub(super) fn group_items(id: &str) -> Vec<Item> {
    vec![
        Item::run(i18n::t("ctx.rename"), Some(PEN), "group.rename", json!({ "id": id })),
        Item::sub(i18n::t("groups.color"), Some(IMAGE), PALETTE.iter().map(|c| Item::run(i18n::t(&format!("groups.color_{c}")), None, "group.color", json!({ "id": id, "color": c }))).collect()),
        Item::run(i18n::t("groups.refresh"), Some(REFRESH), "group.refresh", json!({ "id": id })),
        Item::Sep,
        Item::run(i18n::t("groups.delete"), Some(TRASH), "group.delete", json!({ "id": id })).danger(),
    ]
}

/// What `space.copy` puts on the clipboard.
pub(super) fn copy_text(c: &SpaceCtx, what: &str, core: &str) -> Result<String, String> {
    match what {
        "path" if !c.folder.is_empty() => Ok(c.folder.clone()),
        "name" if !c.folder.is_empty() => Ok(base(&c.folder)),
        "branch" => c.branch.clone().ok_or_else(|| "this space has no branch".to_owned()),
        "id" => Ok(c.id.clone()),
        "core" => Ok(core.to_owned()),
        "path" | "name" => Err("this space has no folder".into()),
        _ => Err("what must be path, name, branch, id or core".into()),
    }
}

/// The notes with `id`'s changed: an empty note removes it.
pub(super) fn notes_with(mut notes: serde_json::Map<String, Value>, id: &str, note: &str) -> serde_json::Map<String, Value> {
    if note.trim().is_empty() {
        notes.remove(id);
    } else {
        notes.insert(id.to_owned(), json!(note.trim()));
    }
    notes
}

/// The list with `id` in it (`on`) or out of it, once.
pub(super) fn listed(mut list: Vec<String>, id: &str, on: bool) -> Vec<String> {
    list.retain(|k| k != id);
    if on {
        list.push(id.to_owned());
    }
    list
}

/// A file name made from the project's key that stays the same for the same key.
fn icon_name(key: &str) -> String {
    let name: String = key.chars().map(|ch| if ch.is_alphanumeric() || matches!(ch, '_' | '.' | '-') { ch } else { '_' }).collect();
    name.chars().rev().take(80).collect::<Vec<_>>().into_iter().rev().collect()
}

/// An image Sunna can draw as an icon.
pub(super) fn icon_ext(path: &str) -> Option<String> {
    let ext = std::path::Path::new(path).extension()?.to_string_lossy().to_lowercase();
    ["png", "jpg", "jpeg", "svg", "webp", "gif"].contains(&ext.as_str()).then_some(ext)
}

// The commands --------------------------------------------------------------------

impl AppWindow {
    fn find_space(&self, a: &Value, d: &Data) -> Result<Value, String> {
        let id = a.get("id").and_then(Value::as_str).ok_or("id is the space's id")?;
        d.spaces.iter().find(|w| text(w, "id") == id).cloned().ok_or_else(|| "no such space".to_owned())
    }

    /// The context of a space for its menu or a command.
    fn space_ctx(&self, d: &Data, w: &Value, dev: bool) -> SpaceCtx {
        let id = text(w, "id").to_owned();
        let g = with(|s| s.git.get(&where_key(w, &d.panes)).cloned().flatten());
        let lists = Lists::load();
        let parents = groups::read_parents(mods::shared("space.parents").as_ref());
        let parent = parents.get(&id);
        SpaceCtx {
            folder: place(w, &d.panes),
            machine: opt_text(w, "machine").map(str::to_owned),
            branch: g.as_ref().map(|g| g.branch.clone()).filter(|b| !b.is_empty() && b != "(detached)"),
            linked: g.as_ref().is_some_and(|g| g.linked),
            pinned: lists.pinned.contains(&id),
            unread: view::read_ids(mods::shared("space.unread").as_ref()).contains(&id),
            status: lists.status.get(&id).copied().unwrap_or_default(),
            apps: open_apps(),
            dev,
            repo: g.is_some(),
            parent: parent.and_then(|p| d.spaces.iter().find(|w| text(w, "id") == p).map(|w| (p.clone(), text(w, "name").to_owned()))),
            children: groups::children_of(&parents, &d.spaces.iter().map(|w| text(w, "id").to_owned()).collect::<Vec<_>>(), &id).into_iter().filter_map(|c| d.spaces.iter().find(|w| text(w, "id") == c).map(|w| (c.clone(), text(w, "name").to_owned()))).collect(),
            id,
        }
    }

    /// The project with this key, as the sidebar groups it now.
    fn find_project(&self, key: &str, d: &Data) -> Result<(Vec<Row>, String), String> {
        let rows = with(|s| ordered_all(&d.spaces, &d.panes, &s.git));
        let group = group_projects(&rows).into_iter().find(|g| g.key == key).ok_or("no such project")?;
        let lead = &group.rows[0];
        // A repo's project is its main checkout; a folder's, its folder.
        let folder = lead
            .g
            .as_ref()
            .and_then(|g| serde_json::from_str::<Value>(&g.repo).ok())
            .and_then(|v| v.get(1).and_then(Value::as_str).map(str::to_owned))
            .and_then(|common| common.strip_suffix("/.git").map(str::to_owned))
            .unwrap_or_else(|| place(&lead.w, &d.panes));
        Ok((group.rows, folder))
    }

    pub(super) fn space_menu(&mut self, id: &str, dev: bool, window: &mut Window, cx: &mut Context<Self>) {
        let d = data();
        let Some(w) = d.spaces.iter().find(|w| text(w, "id") == id).cloned() else { return };
        let items = space_items(&self.space_ctx(&d, &w, dev));
        self.open_context_menu(items, window, cx);
    }

    pub(super) fn project_menu_for(&mut self, key: &str, window: &mut Window, cx: &mut Context<Self>) {
        let d = data();
        let Ok((rows, folder)) = self.find_project(key, &d) else { return };
        let lead = &rows[0];
        let custom_icon = mods::shared("project.icons").is_some_and(|m| m.get(key).is_some());
        let pgs = groups::read_groups(mods::shared("sidebar.groups").as_ref());
        let ctx = ProjectCtx {
            key: key.to_owned(),
            folder,
            machine: lead.machine().map(str::to_owned),
            repo: lead.g.is_some(),
            custom_icon,
            apps: open_apps(),
            groups: pgs.iter().map(|g| (g.id.clone(), g.name.clone())).collect(),
            in_group: groups::group_of(&pgs, key).map(|g| g.id.clone()),
        };
        self.open_context_menu(project_items(&ctx), window, cx);
    }

    pub(super) fn group_menu(&mut self, id: &str, window: &mut Window, cx: &mut Context<Self>) {
        if groups::read_groups(mods::shared("sidebar.groups").as_ref()).iter().any(|g| g.id == id) {
            self.open_context_menu(group_items(id), window, cx);
        } else {
            with(|s| s.list_return = false);
        }
    }

    pub(super) fn machine_menu(&mut self, name: &str, window: &mut Window, cx: &mut Context<Self>) {
        self.open_context_menu(machine_items(name), window, cx);
    }

    pub(in crate::window) fn toast(&mut self, title: String, body: Option<String>, level: &str, window: &mut Window, cx: &mut Context<Self>) {
        let mut n = json!({ "title": title, "level": level });
        if let Some(body) = body {
            n["body"] = json!(body);
        }
        self.open_overlay(mods::new_overlay_id(), json!({ "kind": "toast", "notification": n }), window, cx);
    }

    fn confirmed(a: &Value) -> bool {
        a.get("confirmed").and_then(Value::as_bool) == Some(true)
    }

    /// Asks first (a person) or insists on `confirmed: true` (a script); true when it can go on.
    pub(super) fn consent(&mut self, a: &Value, name: &str, ask: (String, String, Option<String>), person: bool, window: &mut Window, cx: &mut Context<Self>) -> Result<bool, String> {
        if Self::confirmed(a) {
            return Ok(true);
        }
        if !person {
            return Err(format!("{name} is not undoable: pass confirmed: true"));
        }
        let mut args = a.clone();
        args["confirmed"] = json!(true);
        let mut dialog = confirm::Ask::new(ask.0, ask.1, name, args);
        dialog.confirm_label = ask.2;
        self.open_confirm(dialog, window, cx);
        Ok(false)
    }

    /// `space.*` of the menus, `project.*`, `machine.*` and `sidebar.menu`.
    pub(in crate::window) fn menu_command(&mut self, name: &str, a: &Value, person: bool, window: &mut Window, cx: &mut Context<Self>) -> Result<(), String> {
        let d = data();
        let flag = |k: &str| a.get(k).and_then(Value::as_bool).ok_or_else(|| format!("{k} must be true or false"));
        match name {
            "sidebar.menu" => {
                let kind = a.get("kind").and_then(Value::as_str).ok_or("kind is space, project, machine or group")?;
                match kind {
                    "space" => {
                        let w = self.find_space(a, &d)?;
                        self.space_menu(text(&w, "id"), a.get("dev").and_then(Value::as_bool) == Some(true), window, cx);
                    }
                    "project" => {
                        let key = a.get("key").and_then(Value::as_str).ok_or("key is the project's key")?;
                        self.find_project(key, &d)?;
                        self.project_menu_for(key, window, cx);
                    }
                    "machine" => {
                        let m = self.known_machine(a, &d)?;
                        self.machine_menu(&m, window, cx);
                    }
                    "group" => {
                        let id = a.get("id").and_then(Value::as_str).ok_or("id is the group's id")?;
                        if !groups::read_groups(mods::shared("sidebar.groups").as_ref()).iter().any(|g| g.id == id) {
                            return Err(format!("no group {id:?}"));
                        }
                        self.group_menu(id, window, cx);
                    }
                    _ => return Err("kind is space, project, machine or group".into()),
                }
            }
            "space.read" => {
                let w = self.find_space(a, &d)?;
                let list = listed(shared_list("space.unread"), text(&w, "id"), !flag("read")?);
                mods::set_shared("space.unread".into(), json!(list));
            }
            "space.copy" => {
                let w = self.find_space(a, &d)?;
                let ctx = self.space_ctx(&d, &w, false);
                let text = copy_text(&ctx, a.get("what").and_then(Value::as_str).unwrap_or("path"), &std::env::var("HC_SESSION").unwrap_or_default())?;
                cx.write_to_clipboard(ClipboardItem::new_string(text));
            }
            "space.open_in" => {
                let w = self.find_space(a, &d)?;
                let ctx = self.space_ctx(&d, &w, false);
                if ctx.machine.is_some() || ctx.folder.is_empty() {
                    return Err("that space is on another machine: its folder can't be opened here".into());
                }
                self.open_folder(&ctx.folder, a.get("app").and_then(Value::as_str).unwrap_or("folder"))?;
            }
            "space.details" => {
                let w = self.find_space(a, &d)?;
                let id = text(&w, "id").to_owned();
                if a.get("name").is_none() && a.get("note").is_none() {
                    if !person {
                        return Err("space.details needs name or note".into());
                    }
                    self.open_details(&id, window, cx);
                    return Ok(());
                }
                if let Some(n) = a.get("name") {
                    let name = n.as_str().ok_or("name must be text")?;
                    if let Some(error) = self.run_as("space.rename", json!({ "id": id, "name": name }), false, window, cx).get("error").and_then(Value::as_str) {
                        return Err(error.to_owned());
                    }
                }
                if let Some(n) = a.get("note") {
                    let notes = mods::shared("space.notes").and_then(|v| v.as_object().cloned()).unwrap_or_default();
                    mods::set_shared("space.notes".into(), Value::Object(notes_with(notes, &id, n.as_str().ok_or("note must be text")?)));
                }
            }
            "space.remove" => {
                let w = self.find_space(a, &d)?;
                let ctx = self.space_ctx(&d, &w, false);
                let g = with(|s| s.git.get(&where_key(&w, &d.panes)).cloned().flatten()).filter(|g| g.linked).ok_or("only a git worktree can be removed: the main checkout is never removed, and a plain folder is closed with space.close")?;
                let wt = base(&ctx.folder);
                let dirty = g.dirty;
                let mut detail = i18n::tf("ctx.remove_worktree_detail", &[("path", &ctx.folder), ("branch", &g.branch)]);
                if dirty {
                    detail.push(' ');
                    detail.push_str(&i18n::t("ctx.remove_worktree_dirty"));
                }
                if !self.consent(a, name, (i18n::tf("ctx.remove_worktree_title", &[("name", &wt)]), detail, Some(i18n::t("ctx.remove"))), person, window, cx)? {
                    return Ok(());
                }
                let repo: Value = serde_json::from_str(&g.repo).map_err(|e| e.to_string())?;
                let common = repo.get(1).and_then(Value::as_str).ok_or("this worktree's repo is unknown")?.to_owned();
                let main = common.strip_suffix("/.git").unwrap_or(&common).to_owned();
                let (on, folder, id) = (ctx.machine.clone(), ctx.folder.clone(), ctx.id.clone());
                // Terminals first: a shell sitting in the folder keeps it busy on Windows.
                let closed = self.run_as("space.close", json!({ "id": id, "confirmed": true }), false, window, cx);
                if let Some(error) = closed.get("error").and_then(Value::as_str) {
                    return Err(error.to_owned());
                }
                cx.spawn_in(window, async move |this, cx| {
                    let result = cx
                        .background_executor()
                        .spawn(async move {
                            let top = run_on(&on, "git", &["rev-parse".into(), "--show-toplevel".into()], &folder).ok().filter(|(code, _, _)| *code == 0).map(|(_, out, _)| out.trim().to_owned()).filter(|t| !t.is_empty()).unwrap_or(folder);
                            let mut args: Vec<String> = vec!["worktree".into(), "remove".into()];
                            if dirty {
                                args.push("--force".into());
                            }
                            args.push(top);
                            match run_on(&on, "git", &args, &main) {
                                Ok((0, _, _)) => Ok(()),
                                Ok((_, out, err)) => Err(if err.trim().is_empty() { out } else { err }.trim().to_owned()),
                                Err(e) => Err(e),
                            }
                        })
                        .await;
                    let _ = this.update_in(cx, |this, window, cx| {
                        if let Err(error) = result {
                            this.command_error("space.remove", &error, window, cx);
                        }
                        cx.notify();
                    });
                })
                .detach();
            }
            "project.hide" => {
                let key = a.get("key").and_then(Value::as_str).ok_or("key is the project's key")?;
                let hidden = flag("hidden")?;
                mods::set_shared("space.hidden".into(), json!(listed(shared_list("space.hidden"), key, hidden)));
            }
            "project.copy" => {
                let key = a.get("key").and_then(Value::as_str).ok_or("key is the project's key")?;
                let (_, folder) = self.find_project(key, &d)?;
                if folder.is_empty() {
                    return Err("this project has no folder".into());
                }
                cx.write_to_clipboard(ClipboardItem::new_string(folder));
            }
            "project.open_in" => {
                let key = a.get("key").and_then(Value::as_str).ok_or("key is the project's key")?;
                let (rows, folder) = self.find_project(key, &d)?;
                if rows[0].machine().is_some() || folder.is_empty() {
                    return Err("that project is on another machine: its folder can't be opened here".into());
                }
                self.open_folder(&folder, a.get("app").and_then(Value::as_str).unwrap_or("folder"))?;
            }
            "project.worktree" => {
                let key = a.get("key").and_then(Value::as_str).ok_or("key is the project's key")?;
                let (rows, folder) = self.find_project(key, &d)?;
                if rows[0].g.is_none() {
                    return Err("only a git project can have worktrees".into());
                }
                self.open_worktree(json!({ "space": rows[0].id(), "folder": folder, "action": "new" }), window, cx);
            }
            "project.icon" => {
                let key = a.get("key").and_then(Value::as_str).ok_or("key is the project's key")?.to_owned();
                self.find_project(&key, &d)?;
                if a.get("clear").and_then(Value::as_bool) == Some(true) {
                    let mut map = mods::shared("project.icons").and_then(|v| v.as_object().cloned()).unwrap_or_default();
                    map.remove(&key);
                    mods::set_shared("project.icons".into(), Value::Object(map));
                    with(|s| s.icons.remove(&key));
                } else if let Some(file) = a.get("file").and_then(Value::as_str) {
                    self.set_project_icon(&key, file)?;
                } else if person {
                    let picked = cx.prompt_for_paths(PathPromptOptions { files: true, directories: false, multiple: false, prompt: Some(i18n::t("ctx.pick_icon").into()) });
                    cx.spawn_in(window, async move |this, cx| {
                        let Ok(Ok(Some(paths))) = picked.await else { return };
                        let Some(file) = paths.into_iter().next() else { return };
                        let _ = this.update_in(cx, |this, window, cx| {
                            if let Err(error) = this.set_project_icon(&key, &file.to_string_lossy()) {
                                this.command_error("project.icon", &error, window, cx);
                            }
                            cx.notify();
                        });
                    })
                    .detach();
                } else {
                    return Err("project.icon needs file (an image), or clear: true".into());
                }
            }
            "project.remove" => {
                let key = a.get("key").and_then(Value::as_str).ok_or("key is the project's key")?;
                let (rows, _) = self.find_project(key, &d)?;
                let label = group_projects(&rows).first().map(|g| g.label.clone()).unwrap_or_default();
                if !self.consent(a, name, (i18n::tf("ctx.remove_project_title", &[("name", &label)]), i18n::tf("ctx.remove_project_detail", &[("n", &rows.len().to_string())]), Some(i18n::t("ctx.remove_from_list"))), person, window, cx)? {
                    return Ok(());
                }
                for row in &rows {
                    if let Some(error) = self.run_as("space.close", json!({ "id": row.id(), "confirmed": true }), false, window, cx).get("error").and_then(Value::as_str) {
                        return Err(error.to_owned());
                    }
                }
            }
            "machine.check" => {
                let m = self.known_machine(a, &d)?;
                self.check_machine(m, window, cx);
            }
            "machine.rename" => {
                let m = self.known_machine(a, &d)?;
                let new = a.get("name").and_then(Value::as_str).map(str::trim).filter(|n| !n.is_empty()).ok_or("machine.rename needs name: the new name")?.to_owned();
                self.hc_machine(vec!["rename".into(), m.clone(), new.clone()], "machine.rename", Some(i18n::tf("ctx.machine_renamed", &[("name", &new)])), window, cx);
            }
            "machine.remove" => {
                let m = self.known_machine(a, &d)?;
                if !self.consent(a, name, (i18n::tf("ctx.remove_machine_title", &[("name", &m)]), i18n::t("ctx.remove_machine_detail"), Some(i18n::t("ctx.remove_from_list"))), person, window, cx)? {
                    return Ok(());
                }
                self.hc_machine(vec!["rm".into(), m.clone()], "machine.remove", None, window, cx);
            }
            n if groups::handles(n) => self.group_command(n, a, person, window, cx)?,
            _ => return Err(format!("unknown command {name:?}")),
        }
        cx.notify();
        // A picture chosen just now is read off the disk a moment later: draw again then.
        if name == "project.icon" {
            cx.spawn(async move |this, cx| {
                for ms in [300, 1200] {
                    cx.background_executor().timer(Duration::from_millis(ms)).await;
                    let _ = this.update(cx, |_, cx| cx.notify());
                }
            })
            .detach();
        }
        Ok(())
    }

    fn known_machine(&self, a: &Value, d: &Data) -> Result<String, String> {
        let m = a.get("machine").and_then(Value::as_str).filter(|m| !m.is_empty()).ok_or("machine is the machine's name")?;
        let known = with(|s| s.machines.clone());
        if d.machines.iter().chain(known.iter()).any(|k| text(k, "name") == m) {
            Ok(m.to_owned())
        } else {
            Err(format!("no machine named {m:?}"))
        }
    }

    /// `hc machine <args>` in the background; then the machines are read again.
    fn hc_machine(&mut self, args: Vec<String>, command: &'static str, done: Option<String>, window: &mut Window, cx: &mut Context<Self>) {
        cx.spawn_in(window, async move |this, cx| {
            let mut full: Vec<String> = vec!["machine".into()];
            full.extend(args);
            full.push("--json".into());
            let result = cx
                .background_executor()
                .spawn(async move {
                    let (code, out, err) = mods::run_blocking("hc", &full, None, Duration::from_secs(30))?;
                    if code == 0 {
                        return Ok(());
                    }
                    let body = if out.trim().is_empty() { &err } else { &out };
                    let message = serde_json::from_str::<Value>(body).ok().and_then(|v| v["error"]["message"].as_str().map(str::to_owned));
                    Err(message.filter(|m| !m.is_empty()).unwrap_or_else(|| body.trim().to_owned()))
                })
                .await;
            let _ = this.update_in(cx, |this, window, cx| {
                match result {
                    Ok(()) => {
                        if let Some(done) = done {
                            this.toast(done, None, "info", window, cx);
                        }
                    }
                    Err(error) => this.command_error(command, &error, window, cx),
                }
                this.refresh_machines(cx);
                cx.notify();
            });
        })
        .detach();
    }

    /// Looks at the machines again and says how this one is.
    fn check_machine(&mut self, name: String, window: &mut Window, cx: &mut Context<Self>) {
        cx.spawn_in(window, async move |this, cx| {
            let listed = cx.background_executor().spawn(async { mods::run_blocking("hc", &["machine".into(), "ls".into(), "--json".into()], None, Duration::from_secs(30)) }).await;
            let machines: Option<Vec<Value>> = listed.ok().filter(|(code, _, _)| *code == 0).and_then(|(_, out, _)| serde_json::from_str::<Value>(&out).ok()).map(|v| items(&v, "machines").to_vec());
            let _ = this.update_in(cx, |this, window, cx| {
                match machines {
                    Some(all) => {
                        let state = all.iter().find(|m| text(m, "name") == name).map(|m| state_word(opt_text(m, "state"))).unwrap_or_else(|| i18n::t("spaces.unreachable"));
                        let up = all.iter().any(|m| text(m, "name") == name && text(m, "state") == "connected");
                        this.toast(i18n::tf("ctx.machine_state", &[("name", &name), ("state", &state)]), None, if up { "info" } else { "error" }, window, cx);
                        with(|s| s.machines = all.into_iter().filter(|m| m["temporary"] != true).collect());
                    }
                    None => this.toast(i18n::tf("ctx.machine_state", &[("name", &name), ("state", &i18n::t("spaces.unreachable"))]), None, "error", window, cx),
                }
                cx.notify();
            });
        })
        .detach();
    }

    /// Opens a local folder with "folder", "terminal" or one of the apps the menu lists.
    fn open_folder(&self, folder: &str, app: &str) -> Result<(), String> {
        if !matches!(app, "folder" | "terminal") && !open_apps().iter().any(|(_, launch)| launch == app) {
            return Err(format!("{app:?} is not one of the apps Open in lists (Settings > General > Open in apps)"));
        }
        let (program, args) = open_command(this_os(), folder, app, &on_path).ok_or_else(|| i18n::t("ctx.no_terminal"))?;
        launch(&program, &args)
    }

    /// Copies an image into project-icons/ as this project's icon and remembers the choice.
    fn set_project_icon(&mut self, key: &str, file: &str) -> Result<(), String> {
        let ext = icon_ext(file).ok_or("the icon must be a png, jpg, svg, webp or gif image")?;
        let dir = mods::app_dir().join("project-icons");
        std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
        let stored = format!("custom-{}.{ext}", icon_name(key));
        std::fs::copy(file, dir.join(&stored)).map_err(|e| format!("{file}: {e}"))?;
        let mut map = mods::shared("project.icons").and_then(|v| v.as_object().cloned()).unwrap_or_default();
        map.insert(key.to_owned(), json!(stored));
        mods::set_shared("project.icons".into(), Value::Object(map));
        with(|s| s.icons.insert(key.to_owned(), Some(stored)));
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::{COMMANDS, ProjectCtx, SpaceCtx, group_items, Status, Value, copy_text, find_editors, icon_ext, icon_name, json, listed, machine_items, notes_with, open_command, project_items, space_items};
    use crate::window::ctxmenu::Item;

    fn commands(items: &[Item]) -> Vec<String> {
        let mut out = Vec::new();
        for item in items {
            match item {
                Item::Entry(e) => out.push(e.command.0.clone()),
                Item::Sub { items, .. } => out.extend(commands(items)),
                _ => {}
            }
        }
        out
    }

    fn space(linked: bool, machine: Option<&str>) -> SpaceCtx {
        SpaceCtx { id: "s1".into(), folder: "/r/app-wt".into(), machine: machine.map(str::to_owned), branch: Some("feat".into()), linked, pinned: false, unread: false, status: Status::Todo, apps: vec![("Cursor".into(), "Cursor".into())], dev: false, repo: true, parent: None, children: Vec::new() }
    }

    #[test]
    fn the_main_checkout_is_closed_never_removed_and_a_worktree_can_be_removed() {
        assert!(!commands(&space_items(&space(false, None))).contains(&"space.remove".to_owned()));
        assert!(commands(&space_items(&space(false, None))).contains(&"space.close".to_owned()));
        let linked = space_items(&space(true, None));
        assert!(commands(&linked).contains(&"space.remove".to_owned()));
        let Some(Item::Entry(last)) = linked.last() else { panic!("the last line is Remove") };
        assert!(last.danger && last.command.0 == "space.remove");
    }

    #[test]
    fn a_space_on_another_machine_cannot_open_its_folder_here() {
        let local = commands(&space_items(&space(false, None)));
        assert!(local.contains(&"space.open_in".to_owned()));
        let remote = commands(&space_items(&space(true, Some("box"))));
        assert!(!remote.contains(&"space.open_in".to_owned()));
        // The path can still be copied.
        assert!(remote.contains(&"space.copy".to_owned()));
        // A space without a folder has nothing to open either.
        let mut none = space(false, None);
        none.folder.clear();
        assert!(!commands(&space_items(&none)).contains(&"space.open_in".to_owned()));
    }

    #[test]
    fn only_alt_adds_the_developer_menu() {
        let ids = |dev: bool| {
            let mut c = space(false, None);
            c.dev = dev;
            space_items(&c).iter().filter_map(|i| if let Item::Sub { items, .. } = i { Some(items.len()) } else { None }).count()
        };
        assert_eq!(ids(true), ids(false) + 1);
        let mut c = space(false, None);
        c.dev = true;
        let what: Vec<Value> = space_items(&c)
            .into_iter()
            .filter_map(|i| if let Item::Sub { items, .. } = i { Some(items) } else { None })
            .flatten()
            .filter_map(|i| if let Item::Entry(e) = i { Some(e.command.1["what"].clone()) } else { None })
            .collect();
        assert!(what.contains(&json!("id")) && what.contains(&json!("core")));
    }

    #[test]
    fn the_status_submenu_checks_where_the_space_is_and_read_toggles() {
        let mut c = space(false, None);
        c.status = Status::Review;
        let items = space_items(&c);
        let Some(Item::Sub { items: states, .. }) = items.iter().find(|i| matches!(i, Item::Sub { items, .. } if commands(items).iter().all(|c| c == "space.status"))) else { panic!("no status submenu") };
        let on: Vec<&str> = states.iter().filter_map(|i| if let Item::Entry(e) = i { e.on.then(|| e.command.1["status"].as_str().unwrap_or_default()) } else { None }).collect();
        assert_eq!(on, ["in_review"]);
        assert_eq!(states.len(), 4);
        let Item::Entry(first) = &items[0] else { panic!() };
        assert_eq!(first.command, ("space.read".to_owned(), json!({ "id": "s1", "read": false })));
        c.unread = true;
        let Item::Entry(first) = &space_items(&c)[0] else { panic!() };
        assert_eq!(first.command.1["read"], json!(true));
        c.pinned = true;
        assert!(space_items(&c).iter().any(|i| matches!(i, Item::Entry(e) if e.command == ("space.pin".to_owned(), json!({ "id": "s1", "pinned": false })))));
    }

    #[test]
    fn a_plain_folder_has_no_branch_to_copy() {
        let mut c = space(false, None);
        c.branch = None;
        let copy: Vec<Value> = space_items(&c).into_iter().filter_map(|i| if let Item::Sub { items, .. } = i { Some(items) } else { None }).flatten().filter_map(|i| if let Item::Entry(e) = i { e.command.1.get("what").cloned() } else { None }).collect();
        assert!(copy.contains(&json!("path")) && !copy.contains(&json!("branch")));
    }

    #[test]
    fn a_project_menu_fits_what_it_is() {
        let ctx = |repo: bool, machine: Option<&str>, custom: bool| ProjectCtx { key: "k".into(), folder: "/r/app".into(), machine: machine.map(str::to_owned), repo, custom_icon: custom, apps: Vec::new(), groups: Vec::new(), in_group: None };
        let plain = commands(&project_items(&ctx(false, None, false)));
        assert!(!plain.contains(&"project.worktree".to_owned()) && plain.contains(&"project.open_in".to_owned()));
        assert_eq!(plain.iter().filter(|c| *c == "project.icon").count(), 1);
        let repo = commands(&project_items(&ctx(true, None, true)));
        assert!(repo.contains(&"project.worktree".to_owned()));
        assert_eq!(repo.iter().filter(|c| *c == "project.icon").count(), 2);
        let remote = commands(&project_items(&ctx(true, Some("box"), false)));
        assert!(!remote.contains(&"project.open_in".to_owned()) && remote.contains(&"project.copy".to_owned()));
        assert!(remote.contains(&"app.settings".to_owned()) && remote.contains(&"project.hide".to_owned()) && remote.contains(&"project.remove".to_owned()));
    }

    #[test]
    fn a_project_menu_offers_groups_and_a_group_has_its_own_menu() {
        let mut ctx = ProjectCtx { key: "k".into(), folder: "/r/app".into(), machine: None, repo: true, custom_icon: false, apps: Vec::new(), groups: Vec::new(), in_group: None };
        // No groups yet: only «new group from the project».
        let none = commands(&project_items(&ctx));
        assert!(none.contains(&"group.new".to_owned()) && !none.contains(&"group.move".to_owned()));
        ctx.groups = vec![("g1".into(), "Work".into()), ("g2".into(), "Home".into())];
        ctx.in_group = Some("g2".into());
        let items = project_items(&ctx);
        let moves: Vec<&crate::window::ctxmenu::Entry> = items.iter().filter_map(|i| if let Item::Sub { items, .. } = i { Some(items) } else { None }).flatten().filter_map(|i| if let Item::Entry(e) = i { Some(e) } else { None }).filter(|e| e.command.0 == "group.move").collect();
        assert_eq!(moves.len(), 2);
        // The group it is in is the one that is checked.
        assert_eq!(moves.iter().filter(|e| e.on).map(|e| e.command.1["group"].clone()).collect::<Vec<_>>(), [json!("g2")]);
        // Leaving the group: a `group.move` with no group.
        assert!(items.iter().any(|i| matches!(i, Item::Entry(e) if e.command == ("group.move".to_owned(), json!({ "key": "k", "group": "" })))));
        let g = commands(&group_items("g1"));
        assert_eq!(g, ["group.rename", "group.color", "group.color", "group.color", "group.color", "group.color", "group.color", "group.refresh", "group.delete"]);
        // Deleting is the one dangerous line.
        assert!(group_items("g1").iter().any(|i| matches!(i, Item::Entry(e) if e.command.0 == "group.delete" && e.danger)));
    }

    #[test]
    fn a_space_menu_nests_by_lineage_and_starts_a_child() {
        let mut c = space(true, None);
        let plain = commands(&space_items(&c));
        assert!(plain.contains(&"space.worktree".to_owned()) && !plain.contains(&"space.parent".to_owned()));
        c.repo = false;
        assert!(!commands(&space_items(&c)).contains(&"space.worktree".to_owned()));
        c.parent = Some(("p1".into(), "main".into()));
        c.children = vec![("k1".into(), "kid".into())];
        let all = commands(&space_items(&c));
        assert!(all.contains(&"space.parent".to_owned()));
        assert_eq!(all.iter().filter(|c| *c == "space.select").count(), 2, "open the parent and open the child");
        // Leaving the parent is `space.parent` with no parent.
        assert!(space_items(&c).iter().any(|i| matches!(i, Item::Sub { items, .. } if items.iter().any(|j| matches!(j, Item::Entry(e) if e.command == ("space.parent".to_owned(), json!({ "id": "s1", "parent": "" })))))));
    }

    #[test]
    fn a_machine_can_be_renamed_checked_and_forgotten_but_not_disconnected() {
        let list = commands(&machine_items("box"));
        assert_eq!(list, ["machine.rename", "machine.check", "app.settings", "machine.remove"]);
        assert!(!list.iter().any(|c| c.contains("disconnect")));
    }

    #[test]
    fn what_is_copied() {
        let c = space(true, None);
        assert_eq!(copy_text(&c, "path", "core").unwrap(), "/r/app-wt");
        assert_eq!(copy_text(&c, "name", "core").unwrap(), "app-wt");
        assert_eq!(copy_text(&c, "branch", "core").unwrap(), "feat");
        assert_eq!(copy_text(&c, "id", "core").unwrap(), "s1");
        assert_eq!(copy_text(&c, "core", "sunna").unwrap(), "sunna");
        assert!(copy_text(&c, "other", "").is_err());
        let mut none = space(false, None);
        none.branch = None;
        none.folder.clear();
        assert!(copy_text(&none, "branch", "").is_err() && copy_text(&none, "path", "").is_err());
    }

    #[test]
    fn folders_open_the_way_each_system_does() {
        let all = |_: &str| true;
        let none = |_: &str| false;
        assert_eq!(open_command("macos", "/p", "folder", &none), Some(("open".into(), vec!["/p".into()])));
        assert_eq!(open_command("macos", "/p", "Cursor", &none), Some(("open".into(), vec!["-a".into(), "Cursor".into(), "/p".into()])));
        assert_eq!(open_command("macos", "/p", "terminal", &none).unwrap().1, ["-a", "Terminal", "/p"]);
        assert_eq!(open_command("windows", "C:\\p", "folder", &none).unwrap().0, "explorer");
        assert_eq!(open_command("windows", "C:\\p", "C:\\x\\Code.exe", &none), Some(("C:\\x\\Code.exe".into(), vec!["C:\\p".into()])));
        assert_eq!(open_command("linux", "/p", "folder", &none).unwrap().0, "xdg-open");
        assert_eq!(open_command("linux", "/p", "code", &none), Some(("code".into(), vec!["/p".into()])));
        assert_eq!(open_command("linux", "/p", "terminal", &all).unwrap().0, "gnome-terminal");
        assert_eq!(open_command("linux", "/p", "terminal", &none), None);
    }

    #[test]
    fn editors_are_found_by_each_systems_known_places() {
        let mac = find_editors("macos", &|p| p == "/Applications/Cursor.app" || p == "/Users/me/Applications/Zed.app", &|_| false, &[], "/Users/me");
        assert_eq!(mac, [("Cursor".to_owned(), "Cursor".to_owned()), ("Zed".to_owned(), "Zed".to_owned())]);
        let linux = find_editors("linux", &|_| false, &|c| c == "code" || c == "idea", &[], "/home/me");
        assert_eq!(linux.iter().map(|(l, _)| l.as_str()).collect::<Vec<_>>(), ["Visual Studio Code", "IntelliJ IDEA"]);
        let win = find_editors("windows", &|p| p == "C:\\Users\\me\\AppData\\Local\\Programs\\cursor\\Cursor.exe", &|_| false, &["C:\\Users\\me\\AppData\\Local\\Programs".into()], "");
        assert_eq!(win, [("Cursor".to_owned(), "C:\\Users\\me\\AppData\\Local\\Programs\\cursor\\Cursor.exe".to_owned())]);
        // Xcode only exists on macOS.
        assert!(find_editors("linux", &|_| true, &|_| true, &[], "").iter().all(|(l, _)| l != "Xcode"));
    }

    #[test]
    fn notes_lists_and_icons() {
        let notes = notes_with(serde_json::Map::new(), "a", "  remember this ");
        assert_eq!(notes["a"], "remember this");
        assert!(notes_with(notes, "a", "   ").is_empty());
        assert_eq!(listed(vec!["a".into(), "b".into()], "a", true), ["b", "a"]);
        assert_eq!(listed(vec!["a".into()], "a", false), Vec::<String>::new());
        assert_eq!(icon_ext("/x/Logo.PNG").as_deref(), Some("png"));
        assert_eq!(icon_ext("/x/readme.md"), None);
        assert_eq!(icon_ext("/x/noext"), None);
        assert_eq!(icon_name("[null,\"/r/app/.git\"]"), "_null___r_app_.git__");
    }

    #[test]
    fn every_command_of_the_menus_is_defined_and_every_text_exists_in_both_locales() {
        for name in COMMANDS {
            assert!(crate::commands::find(name).is_some(), "{name}");
        }
        let keys = ["ctx.mark_read", "ctx.mark_unread", "ctx.move_to", "ctx.edit_details", "ctx.pin", "ctx.unpin", "ctx.open_in", "ctx.terminal", "ctx.copy", "ctx.path", "ctx.worktree_name", "ctx.branch", "ctx.close_space", "ctx.remove_worktree",
            "ctx.developer", "ctx.copy_space_id", "ctx.copy_core_id", "ctx.project_settings", "ctx.change_icon", "ctx.reset_icon", "ctx.hide_sidebar", "ctx.remove_project", "ctx.rename", "ctx.check_connection", "ctx.manage_machines", "ctx.remove_machine",
            "ctx.project_actions", "ctx.machine_actions", "ctx.details_title", "ctx.name", "ctx.note", "ctx.save", "ctx.remove", "ctx.remove_from_list", "ctx.remove_worktree_title", "ctx.remove_worktree_detail", "ctx.remove_worktree_dirty", "ctx.remove_project_title",
            "ctx.remove_project_detail", "ctx.remove_machine_title", "ctx.remove_machine_detail", "ctx.machine_state", "ctx.machine_renamed", "ctx.rename_machine_title", "ctx.pick_icon", "ctx.no_terminal", "ctx.unread", "ctx.note_hint",
            "menu.reveal_folder", "menu.new_worktree", "menu.copy_path", "sidebar.status_todo", "sidebar.status_progress", "sidebar.status_review", "sidebar.status_done"];
        for lang in ["en", "es"] {
            let file = format!("{}/defaults/locales/{lang}.json", env!("CARGO_MANIFEST_DIR"));
            let table: Value = serde_json::from_str(&std::fs::read_to_string(&file).unwrap()).unwrap();
            for k in keys {
                assert!(table[k].as_str().is_some_and(|s| !s.is_empty()), "{lang}:{k}");
            }
        }
    }
}
