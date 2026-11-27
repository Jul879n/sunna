//! Every command the app runs, defined once. Keys, menus, mods and
//! `sunna do` all run these; key checks, `sunna commands` and the docs come
//! from this list. Mods' own commands are `<mod>.<command>`.

use serde_json::{Value, json};

pub struct Arg {
    pub name: &'static str,
    pub required: bool,
}

pub struct Command {
    pub name: &'static str,
    pub args: &'static [Arg],
    pub about: &'static str,
}

const fn req(name: &'static str) -> Arg {
    Arg { name, required: true }
}

const fn opt(name: &'static str) -> Arg {
    Arg { name, required: false }
}

const fn cmd(name: &'static str, args: &'static [Arg], about: &'static str) -> Command {
    Command { name, args, about }
}

pub const COMMANDS: &[Command] = &[
    // What exists.
    cmd("space.new", &[opt("name"), opt("folder"), opt("tab"), opt("argv"), opt("task"), opt("machine"), opt("focus")],
        "Make a space: its name (default: none, so it's called after where its first terminal is), the folder its terminals start in (default: the focused terminal's), its first tab's name (default: none, so it's called by its number), optionally an agent (argv) and its first message (task), and the machine it lives on (`hc machine ls`; default the focused space's machine for keys and clicks, this computer for scripts; empty string: this computer; its terminals start in that machine's home unless folder). It is shown at once; focus false keeps the shown space and tab (with no space shown, the new space is shown)."),
    cmd("space.close", &[opt("id"), opt("confirmed")], "Close a space and end its terminals (asks first unless confirmed)."),
    cmd("space.rename", &[opt("id"), opt("name")], "Rename a space (asks the person when no name; an empty name: called after where its first terminal is again)."),
    cmd("space.move", &[req("id"), req("index")], "Move a space to a position (1 = first)."),
    cmd("space.status", &[req("id"), req("status")], "Set where a space is in its work: todo, in_progress, in_review or done (todo is the default). The sidebar groups by it when Group by is Status; it is kept in shared() (space.status)."),
    cmd("space.pin", &[req("id"), req("pinned")], "Pin a space to the Pinned section at the top of the sidebar (pinned true) or unpin it (false); kept in shared() (space.pinned)."),
    cmd("space.read", &[req("id"), req("read")], "Mark a space read (read true) or unread (false): an unread space shows a dot on its row until it is shown. Kept in shared() (space.unread)."),
    cmd("space.open_in", &[req("id"), opt("app")], "Open a space's folder (this computer's only) with app: \"folder\" (the file manager, default), \"terminal\", or one of the apps its Open in menu lists (editors found on this computer and Settings > General > Open in apps; macOS: the app's name, elsewhere its command)."),
    cmd("space.copy", &[req("id"), opt("what")], "Copy to the clipboard what of a space: path (default; its folder now), name (that folder's name), branch, id (the space's) or core (the core session that keeps it)."),
    cmd("space.details", &[req("id"), opt("name"), opt("note")], "Edit a space's details: its name (as space.rename) and a note shown as the tooltip of its row (empty: no note; kept in shared() space.notes). With neither, a person gets the Edit details dialog."),
    cmd("space.remove", &[req("id"), opt("confirmed")], "Remove a git worktree's space and its folder (git worktree remove; uncommitted changes go with it; the branch is kept). Never the repo's main checkout or a plain folder (space.close). Asks first unless confirmed."),
    cmd("project.add", &[opt("step"), opt("folders")], "Add projects. With folders (a list of paths), a space is opened for each one, no picker. For a person, step menu (default; the three ways), browse (a folder picker that takes several folders, then space.new for each), clone (the Clone from URL dialog) or create (the Create a new project dialog). A script adds a project with space.new and a folder."),
    cmd("project.icon", &[req("key"), opt("file"), opt("clear")], "Set a project's icon in the sidebar from an image file (png, jpg, svg, webp or gif; copied to project-icons/ and kept in shared() project.icons), or clear true for the automatic one. A person without file picks one."),
    cmd("project.hide", &[req("key"), req("hidden")], "Hide a project from the sidebar (hidden true) or show it again (false); kept in shared() (space.hidden). The project of the shown space never hides."),
    cmd("project.open_in", &[req("key"), opt("app")], "Open a project's folder (its repo's main checkout) with app, as space.open_in."),
    cmd("project.copy", &[req("key")], "Copy a project's folder (its repo's main checkout) to the clipboard."),
    cmd("project.remove", &[req("key"), opt("confirmed")], "Close every space of a project and end their terminals; the folders stay. Asks first unless confirmed."),
    cmd("worktree.new", &[opt("space")], "Open the New worktree dialog for a space (the active one by default)."),
    cmd("worktree.open", &[opt("space")], "Open the Open worktree dialog (the repo's checkouts) for a space (the active one by default)."),
    cmd("worktree.cleanup", &[opt("space")], "Open the worktree cleanup dialog (merged, unchanged, bot ones) for a space's repo (the active one by default)."),
    cmd("cleanup.run", &[opt("confirmed")], "Clean up spaces now by the rules of Settings > Git and spaces (merged branches, unchanged ones, finished bot runs): removes the worktree checkouts on this computer that the rules allow, never the main checkout, a pinned space, one with a working agent or uncommitted changes. A person is shown the list first unless confirmed; the result goes to shared() cleanup.log."),
    cmd("bot.new", &[req("name"), opt("template"), opt("agent"), opt("folder"), opt("yolo")], "Make a bot (a folder bots/<name> with SOUL.md and bot.json) from a template (pr-review, ci-failures, release-notes, issue-triage, docs-sync, daily-summary, day-plan or blank, the default); schedules and GitHub bots start switched off."),
    cmd("bot.run", &[req("name")], "Start a bot's agent now (the precheck is ignored; refused at its daily limit or while its last run is still working)."),
    cmd("bot.enable", &[req("name"), opt("on")], "Switch a bot on (on true, default) or off (false); a switched-off bot runs only with bot.run."),
    cmd("bot.delete", &[req("name"), opt("confirmed")], "Delete a bot's folder (its SOUL.md, bot.json and history). Asks first unless confirmed."),
    cmd("project.worktree", &[req("key")], "Open the New worktree dialog for a git project."),
    cmd("project.move", &[req("key"), req("index"), opt("group")], "Put a project at a position (1 = first) of the group `group` (a group id) or, without it, of the projects that are in no group; it leaves the group it was in, and the sidebar's project order becomes Manual. Kept in shared() (sidebar.groups, sidebar.project_keys)."),
    cmd("group.new", &[req("key"), opt("name"), opt("color")], "Make a project group (a folder in the sidebar) holding the project `key`; name (the project's by default; a person is asked) and color (ansi6, ansi3, ansi4, ansi5, ansi2 or ansi1; the least used by default). Answers {group}. Kept in shared() (sidebar.groups)."),
    cmd("group.rename", &[req("id"), req("name")], "Rename a project group (a person is asked); names are unique ignoring case."),
    cmd("group.color", &[req("id"), req("color")], "Set a project group's color: ansi6, ansi3, ansi4, ansi5, ansi2 or ansi1."),
    cmd("group.delete", &[req("id"), opt("confirmed")], "Delete a project group; its projects go back to the list and no space is closed. Asks first unless confirmed."),
    cmd("group.move", &[req("key"), opt("group")], "Put a project last in the group `group`, or out of every group (group \"\" or missing)."),
    cmd("group.reorder", &[req("id"), req("index")], "Put a project group at a position (1 = first) among the groups."),
    cmd("group.refresh", &[req("id")], "Read the git state and the icons of a group's projects again."),
    cmd("space.parent", &[req("id"), opt("parent")], "Make a space the child of another (parent, a space id) so the sidebar nests it under it, or free it (parent \"\" or missing). A space can't descend from itself. Kept in shared() (space.parents)."),
    cmd("space.worktree", &[req("id")], "Open the New worktree dialog from a git space: the worktree it makes starts from that space's branch and becomes its child (the dialog's «Create from this space» switch)."),
    cmd("machine.rename", &[req("machine"), opt("name")], "Give a machine (hc machine ls) a new name here (asks the person when no name)."),
    cmd("machine.check", &[req("machine")], "Look at the machines again and say how this one is (connected, unreachable, needs login...)."),
    cmd("machine.remove", &[req("machine"), opt("confirmed")], "Forget a machine (hc machine rm): its terminals keep running there. Asks first unless confirmed."),
    cmd("sidebar.menu", &[req("kind"), opt("id"), opt("key"), opt("machine"), opt("dev")], "Open a sidebar context menu at the pointer: kind space (id; dev true adds the Developer submenu), project (key) or machine (machine)."),
    cmd("tab.new", &[opt("name"), opt("space"), opt("folder"), opt("argv"), opt("task"), opt("mod"), opt("props"), opt("focus")],
        "Make a tab in a space (default: the selected one; name default: none, so it's called by its number) with a terminal in folder (default: the focused terminal's), optionally an agent (argv) and its first message (task); or with a mod as its only pane (`mod`, its `props`: a file, a board). It is shown at once; focus false adds it last in its space and keeps the shown space and the target space's selected tab. With no space open, a person's new tab starts a space (as space.new)."),
    cmd("tab.close", &[opt("id"), opt("confirmed")], "Close a tab and end its terminals. A space's last tab: from a key or click it is replaced by a new terminal tab (the space stays); from a script it closes the space (asks first unless confirmed). With `confirm_close_running` a person is asked before ending a running program."),
    cmd("tab.rename", &[opt("id"), opt("name")], "Rename a tab (asks the person when no name; an empty name: called by its number again)."),
    cmd("tab.move", &[opt("id"), opt("index"), opt("by")], "Move a tab (the active one if no id) to a position in its space (1 = first), or `by` places left (-) or right (+), wrapping."),
    cmd("pane.split", &[req("direction"), opt("pane"), opt("argv"), opt("mod"), opt("props")], "Split a pane (default: the focused one) \"right\" or \"down\" with a new terminal, or with a mod (`mod`, its `props`) beside it."),
    cmd("pane.close", &[opt("pane"), opt("confirmed")], "Close a pane and end its terminal (a file editor with unsaved changes asks first unless confirmed)."),
    // Files.
    cmd("file.open", &[req("path"), opt("line"), opt("on"), opt("space"), opt("focus")],
        "Open a file in the editor, in its own tab (or show the tab that already has it). path: absolute, or relative to the space's folder; line (1 is the first) puts the cursor there; on: the machine the file is on (default the space's machine; empty string: this computer; files on another machine open read only); space: default the shown one. Binary and over 1 MiB files show why they can't be edited. focus false opens the tab without showing it."),
    cmd("file.goto", &[], "Toggle Go to file: fuzzy search by name over the files of the focused pane's project (ripgrep, else git ls-files, else find; .git, node_modules and target are never listed; at most 60000 files), recent ones first; Enter opens the file with file.open, Esc closes; calling again closes it."),
    cmd("editor.save", &[opt("pane"), opt("force")], "Write a file editor's text to its file (default: the focused pane). It refuses when the file changed on disk since it was read, unless force is true (overwrite). Files on another machine are read only."),
    cmd("editor.save_close", &[req("then"), opt("panes")], "Save every file editor with unsaved changes (only `panes` when given), then run `then` ({name, args}, as confirmed). If a file can't be saved (it changed on disk, a write error) nothing runs and that file is shown. What the unsaved-changes dialog's Save and close does."),
    cmd("editor.preview", &[opt("pane"), opt("on")], "Show a markdown file in the editor as formatted text or as source again (default: the focused pane; on true/false sets it instead of switching)."),
    cmd("editor.reload", &[opt("pane"), opt("confirmed")], "Read a file editor's file again, dropping what wasn't saved (asks first unless confirmed)."),
    cmd("editor.goto", &[req("line"), opt("pane")], "Put the cursor of a file editor (default: the focused pane) on a line (1 is the first)."),
    cmd("terminal.open", &[opt("argv"), opt("folder"), opt("place"), opt("name")], "Start a terminal: as a new tab (place \"tab\", default), or \"right\"/\"down\" of the focused pane."),
    cmd("terminal.quick", &[req("name")], "Type a quick command (Settings > Terminal > Quick commands, kept in shared() quick_commands) into the focused terminal; it runs at once unless the command has enter off."),
    cmd("quick.set", &[req("name"), req("command"), opt("enter"), opt("old")], "Make or replace a quick command: name, command, enter (default true: press Enter after typing it) and old (the name it had, to rename it)."),
    cmd("quick.edit", &[opt("name")], "Open the dialog that makes a quick command, or edits the one called name."),
    cmd("quick.delete", &[req("name"), opt("confirmed")], "Delete a quick command. Asks first unless confirmed."),
    cmd("terminal.float", &[], "Show or hide the floating terminal: a shell in a window over the work area that keeps its session while hidden (it starts in the focused terminal's folder the first time)."),
    cmd("terminal.keep", &[req("terminal"), opt("place")], "Give a running terminal without a pane (a sub-agent) a pane: a new tab, or \"right\"/\"down\"."),
    cmd("terminal.park", &[req("terminal")], "Take a terminal's pane away and disable core keep; it keeps running (a sub-agent shows under its parent again)."),
    cmd("terminal.right_click", &[req("to"), opt("terminal")], "Where right-clicks in a terminal (default: the focused one) go when its program takes the mouse: \"program\" or \"menu\" (Herdr's, the default). Shift+right-click is always the menu."),
    cmd("terminal.pin", &[req("terminal"), req("pinned")], "Pin an agent to the top of the agent lists (pinned true) or unpin it (false). Pins are kept with the terminal, so every Herdr client shows them, and end with it."),
    cmd("terminal.kill", &[req("terminal")], "End a terminal and its sub-agents."),
    // What you see.
    cmd("sidebar.collapse_all", &[], "Fold every project (and Pinned or state heading) of the sidebar; when they are all folded, open them all."),
    cmd("sidebar.reveal_current", &[], "Show the selected space in the sidebar: open what folds it and scroll to it."),
    cmd("sidebar.focus", &[], "Give the keyboard to the sidebar's list of spaces (the sidebar must be showing): arrows, Home and End move its cursor, Enter shows the space, the menu key or Shift+F10 opens its menu."),
    cmd("sidebar.view", &[opt("group_by"), opt("sort_by"), opt("project_order")], "How the sidebar lists spaces: group_by none, status, pr or project (default); sort_by name, agents, recent, project or manual (default); project_order manual (default) or recent. Kept in shared() (sidebar.group_by, sidebar.sort_by, sidebar.project_order)."),
    cmd("sidebar.filter", &[opt("hide_idle"), opt("hide_default_branch"), opt("hide_bots"), opt("projects"), opt("clear")], "Filter the sidebar's spaces: hide_idle (nothing running or written for 30 minutes), hide_default_branch (main, master, trunk), hide_bots (spaces listed in shared() space.bot_made), projects (a list of project keys to show), clear true (no filters). Kept in shared() (sidebar.filters)."),
    cmd("space.select", &[opt("index"), opt("id")], "Show a space (by position or id)."),
    cmd("space.next", &[], "Show the next space."),
    cmd("space.prev", &[], "Show the previous space."),
    cmd("tab.select", &[opt("index"), opt("id")], "Show a tab (by position in the space, or id)."),
    cmd("tab.settle", &[opt("id")], "Put a tab away on this screen (default: the shown one): out of the lists, still running. It comes back when shown or when an agent in it needs you."),
    cmd("tab.unsettle", &[opt("id")], "Bring a settled tab back to the lists."),
    cmd("tab.next", &[], "Show the next tab."),
    cmd("tab.prev", &[], "Show the previous tab."),
    cmd("pane.zoom", &[opt("pane")], "Show only this pane of its tab, or all again."),
    cmd("pane.focus", &[opt("pane"), opt("terminal")], "Focus a pane (or the pane showing a terminal)."),
    cmd("pane.go", &[req("direction")], "Focus the pane or focusable mod to the \"left\", \"right\", \"up\" or \"down\"."),
    cmd("pane.swap", &[opt("direction"), opt("pane"), opt("target")], "Swap a pane (default: the focused one) with its neighbor in a direction, or with another pane of its tab (target). Give exactly one of direction or target."),
    cmd("terminal.peek", &[req("terminal")], "Peek at a running terminal over the tab, with keep-as-tab and close controls."),
    cmd("terminal.show", &[opt("terminal"), opt("pane")], "Show a terminal in a pane for now, in place of its own (no terminal: back)."),
    cmd("terminal.select_all", &[opt("terminal")], "Select a terminal's whole screen and scrollback (default: the focused one), ready to copy. On the alternate screen, select only that screen."),
    cmd("terminal.select", &[opt("terminal"), opt("at"), opt("mode")], "Start a terminal selection (default: the focused one), replacing the old selection. at defaults to \"cursor\" or takes {row, col}: zero-based cells, rows from the top of the history including the active screen. mode is \"cell\" (default), \"word\" or \"line\" (like double/triple click). Mods read the inclusive endpoints from focused().selection or terminals(); selection.end is the moving end."),
    cmd("terminal.adjust_selection", &[opt("terminal"), req("direction")], "Move the selection's moving end: left, right, up, down, page_up, page_down, home, end, line_start or line_end. The view follows the moving end. Fails if there is no selection. Defaults to the focused terminal."),
    cmd("terminal.clear_selection", &[opt("terminal")], "Clear a terminal's selection (default: the focused one)."),
    cmd("terminal.copy", &[], "Copy the focused terminal's selection. With nothing selected, that key goes to the program."),
    cmd("terminal.scroll", &[opt("lines"), opt("pages"), opt("to"), opt("terminal")], "Scroll a terminal's view of its history (default: the focused one). Give exactly one of: lines (a whole number; negative goes up, toward history), pages (a number, fractions allowed; one page is the terminal's rows), to: \"top\" (oldest history) or \"bottom\" (the live screen). This scrolls Herdr's view, not the program."),
    cmd("terminal.paste", &[], "Paste into the focused terminal (copied files as if dropped; an image alone as terminal.paste_image pastes it)."),
    cmd("terminal.paste_image", &[], "Ctrl+V: the clipboard's image as a file on the focused terminal's machine, when that's another one; on Windows, paste clipboard text; otherwise Ctrl+V goes to the terminal."),
    cmd("terminal.search", &[], "Search the focused terminal's screen and scrollback; the find bar and query stay with that terminal."),
    cmd("agent.focus", &[req("terminal")], "Show an agent (sub-agents too)."),
    cmd("agent.next", &[], "Show the next agent, in the order the agents list shows them."),
    cmd("agent.prev", &[], "Show the previous agent."),
    cmd("layout.switch", &[opt("name")], "Show a layout (layouts/<name>.json), or the next one."),
    cmd("layout.clone", &[req("name"), opt("as")], "Duplicate a layout as <name>-copy (or as), then show it."),
    cmd("mod.clone", &[req("name"), opt("as")], "Duplicate a mod as <name>-copy (or as)."),
    cmd("layout.set", &[req("layout")], "Replace a personal layout tree; shipped layouts accept only click/look fields (not structure)."),
    cmd("layout.toggle", &[req("id")], "Hide or show a layout node."),
    cmd("layout.show", &[req("id")], "Show a layout node."),
    cmd("layout.hide", &[req("id")], "Hide a layout node."),
    cmd("layout.move", &[req("id"), opt("before"), opt("after")], "Move a layout node before or after another (a Herdr layout is copied to my-<name> first)."),
    cmd("layout.edit", &[], "Toggle layout edit mode: drag the blocks of the screen to rearrange them (Esc leaves)."),
    cmd("layout.resize", &[req("id"), opt("px"), opt("weight")], "Size a layout node."),
    cmd("layout.select", &[req("stack"), req("child")], "Show one child of a stack."),
    cmd("layout.reset", &[opt("name")], "Reset a shipped layout and its clicks (the old file is kept aside)."),
    cmd("mod.open", &[req("name")], "Toggle a mod as the overlay its mod.json describes; calling again closes it."),
    cmd("overlay.close", &[opt("id")], "Close an overlay (default: the top dialog or popup)."),
    // The app.
    cmd("app.help", &[], "Toggle the keys dialog."),
    cmd("app.notify", &[req("title"), opt("body"), opt("level"), opt("action"), opt("timeout")], "Show a notification (a toast at the bottom right with a close button): title, body, level (info, success, warning or error), action ({name, args}, run when it is clicked) and timeout in ms. At most four show at once; the others wait."),
    cmd("app.welcome", &[], "Show the welcome again."),
    cmd("app.whats_new", &[], "Show what this release brought (it opens once by itself after an update)."),
    cmd("app.feedback", &[opt("message"), opt("from")], "Draft feedback for the person to review and send (from: person or agent)."),
    cmd("app.palette", &[], "Toggle the command palette: every command by name, with its key; calling again closes it."),
    cmd("app.settings", &[opt("section")], "Open the settings (section: look, general, git, agents, terminal, editor, keyboard, machines, alerts, updates, privacy, mods; an unknown one opens the first)."),
    cmd("app.check_updates", &[], "Look for a newer Herdr and herdr-core now."),
    cmd("app.update_core", &[], "Update herdr-core to its latest release of this generation; terminals keep running."),
    cmd("app.reset", &[], "The escape hatch: set the shown layout and keys aside, back to the default screen."),
    cmd("app.theme", &[req("name")], "Use a theme (a preset or themes/<name>.json)."),
    cmd("app.keep_awake", &[opt("mode")], "Keep the computer awake: \"on\" (always), \"auto\" (while an agent works) or \"off\"; with no mode, the next one."),
    cmd("app.zoom_in", &[], "Make everything bigger."),
    cmd("app.zoom_out", &[], "Make everything smaller."),
    cmd("app.zoom_reset", &[], "Back to normal size."),
    cmd("app.update", &[], "Restart into the update Herdr downloaded (your terminals keep running)."),
    cmd("app.quit", &[opt("confirmed")], "Quit (macOS). Files edited and not saved ask first unless confirmed."),
    cmd("app.hide", &[], "Hide the app (macOS)."),
    cmd("app.hide_others", &[], "Hide other apps (macOS)."),
    cmd("app.show_all", &[], "Show all apps (macOS)."),
    cmd("window.close", &[opt("confirmed")], "Close the window (on Linux, quits). Files edited and not saved ask first unless confirmed."),
    cmd("window.minimize", &[], "Minimize the window."),
    cmd("window.maximize", &[], "Maximize the window, or back."),
];

pub fn find(name: &str) -> Option<&'static Command> {
    COMMANDS.iter().find(|c| c.name == name)
}

/// A key's `name:value` shorthand: the value fills the first argument (a
/// number fills `index` when there is one).
pub fn shorthand(c: &Command, value: &str) -> Value {
    let index = value.parse::<u64>().ok().filter(|_| c.args.iter().any(|a| a.name == "index"));
    match (index, c.args.first()) {
        (Some(i), _) => json!({ "index": i }),
        (None, Some(a)) => json!({ a.name: value }),
        (None, None) => Value::Null,
    }
}

/// Reject misspelled arguments before a command can use its current-target default.
/// Null is the argument-free form used by keys and mods.
pub fn validate_args(c: &Command, args: &Value) -> Result<(), String> {
    let valid = if c.args.is_empty() { "none".into() } else { c.args.iter().map(|a| a.name).collect::<Vec<_>>().join(", ") };
    match args {
        Value::Null => Ok(()),
        Value::Object(args) => {
            let unknown: Vec<_> = args.keys().filter(|key| !c.args.iter().any(|a| a.name == key.as_str())).map(String::as_str).collect();
            if !unknown.is_empty() {
                return Err(format!("{}: unknown argument(s): {}; valid arguments: {valid}", c.name, unknown.join(", ")));
            }
            for (key, value) in args {
                if matches!(key.as_str(), "id" | "pane" | "space" | "terminal" | "target") && !value.is_string() && !value.is_null() {
                    return Err(format!("{}: argument {key} must be a string (or null to omit it)", c.name));
                }
            }
            Ok(())
        }
        _ => Err(format!("{}: arguments must be an object; valid arguments: {valid}", c.name)),
    }
}

/// The first required argument that's missing (an empty string counts as missing).
pub fn missing(c: &Command, args: &Value) -> Option<&'static str> {
    c.args.iter().filter(|a| a.required).map(|a| a.name).find(|k| match args.get(k) {
        None | Some(Value::Null) => true,
        Some(Value::String(s)) => s.trim().is_empty(),
        _ => false,
    })
}

/// The list as data, for `sunna commands` and mods.
pub fn listing() -> Value {
    Value::Array(
        COMMANDS
            .iter()
            .map(|c| {
                let args: Vec<Value> = c.args.iter().map(|a| json!({ "name": a.name, "required": a.required })).collect();
                json!({ "name": c.name, "args": args, "about": c.about })
            })
            .collect(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The guide agents read (written to the home folder) names every command.
    #[test]
    fn the_guide_names_every_command() {
        let guide = include_str!("../defaults/AGENTS.md");
        let missing: Vec<&str> = COMMANDS.iter().map(|c| c.name).filter(|n| !guide.contains(&format!("`{n}"))).collect();
        assert!(missing.is_empty(), "defaults/AGENTS.md (## Commands) doesn't name: {missing:?}");
    }

    #[test]
    fn unknown_arguments_are_rejected_for_every_command() {
        for c in COMMANDS {
            let error = validate_args(c, &json!({ "typo": "target" })).unwrap_err();
            assert!(error.contains(c.name), "{error}");
            assert!(error.contains("unknown argument(s): typo"), "{error}");
            assert!(error.contains("valid arguments:"), "{error}");
            assert!(validate_args(c, &Value::Null).is_ok());
            assert!(validate_args(c, &json!({})).is_ok());
            assert!(validate_args(c, &json!(["target"])).is_err());
            let args: serde_json::Map<String, Value> = c.args.iter().map(|a| (a.name.into(), json!("target"))).collect();
            assert!(validate_args(c, &Value::Object(args)).is_ok());
        }
        let close = find("space.close").unwrap();
        assert_eq!(validate_args(close, &json!({"space": "X", "confirmed": true})),
            Err("space.close: unknown argument(s): space; valid arguments: id, confirmed".into()));
        assert!(validate_args(close, &json!({"id": "X", "confirmed": true})).is_ok());
    }

    #[test]
    fn malformed_targets_cannot_fall_back_to_the_current_work() {
        for (name, arg) in [("tab.close", "id"), ("pane.close", "pane"), ("space.close", "id"), ("tab.new", "space"), ("terminal.show", "terminal"), ("pane.swap", "pane"), ("pane.swap", "target")] {
            let c = find(name).unwrap();
            for value in [json!(123), json!(false), json!([]), json!({"id": "target"})] {
                let error = validate_args(c, &json!({arg: value})).unwrap_err();
                assert!(error.contains(name) && error.contains(arg), "{error}");
            }
            // Null keeps its existing omitted-argument meaning for optional targets.
            for value in [Value::Null, json!("target")] {
                assert!(validate_args(c, &json!({arg: value})).is_ok());
            }
        }
    }

    #[test]
    fn names_are_unique_and_shorthand_fills_the_first_argument() {
        let mut names: Vec<&str> = COMMANDS.iter().map(|c| c.name).collect();
        names.sort();
        names.dedup();
        assert_eq!(names.len(), COMMANDS.len());
        assert_eq!(shorthand(find("tab.select").unwrap(), "3"), json!({ "index": 3 }));
        assert_eq!(shorthand(find("pane.split").unwrap(), "right"), json!({ "direction": "right" }));
        let move_ = find("space.move").unwrap();
        assert_eq!(missing(move_, &json!({ "id": " ", "index": 1 })), Some("id"));
        assert_eq!(missing(move_, &json!({ "id": "sp1", "index": 1 })), None);
        // Names are optional: none is Herdr's automatic one.
        assert_eq!(missing(find("space.new").unwrap(), &json!({})), None);
    }
}
