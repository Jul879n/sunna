# sunna: drive the person's window

Herdr shows spaces, tabs and panes. herdr-core (`hc`) runs their terminals
and keeps them alive when the window closes. Use `sunna` only to change
what the person sees. For starting, reading, messaging, waiting on or cleaning
up agents and programs, read `hc --skill` and use hc.
Your terminal address is `$HC_TERMINAL_ID`; it is not a pane ID.

## Inspect before acting

```sh
sunna ls          # spaces > tabs > terminals, including pane IDs
sunna status      # current layout, focus, overlays, zoom and file/mod errors
sunna commands    # available commands, arguments and descriptions
```

`ls` and `status` talk to the running window, not a new app. `commands`
prints the built-in command list even without an open window. Do not launch bare
`sunna` to inspect it: that opens or raises the window. Do not stop,
signal or restart the person's app or core. A window closing does not kill
its terminals.

`app.check_updates` checks for newer releases. Settings reports "Up to date"
only after a successful check; on failure, check the connection and retry.
Package-managed cores are checked but updated through their package manager.
Dismiss a core update error with its × or `overlay.close {id:"core-update-error"}`;
it stays dismissed until the error changes or the update recovers.

## Show something when asked

New spaces and tabs are shown immediately: only create them when asked.
To start one the person will look at later without moving them there, pass
`--focus false`:

```sh
sunna do space.new --name eng-123 --folder ~/code/app/eng-123 --argv '["claude"]' --task "Implement ENG-123" --focus false
```

Pick an explicit, useful name for every new space and tab; don't rely on
default names. Use IDs from `ls` rather than guessing or acting on the wrong
focused pane.

```sh
sunna do space.new --name api --folder /path/to/api
sunna do tab.new --name "fix login" --argv '["claude"]' --task "Look at the login bug"
sunna do pane.split --direction right --pane p12
sunna do agent.focus --terminal t7
```

To show a file to the person (or to edit one together), open it in the app's
own editor instead of printing it: it gets its own tab, with highlighting, find
and save.

```sh
sunna do file.open --path src/main.rs --line 40
sunna do editor.save                  # the focused file; fails if the disk changed under it (add --force true to overwrite)
```

Closing a tab with unsaved edits needs `--confirmed true` (a person gets Save and
close, Discard or Cancel; scripts can save first with `sunna do editor.save_close
--then '{"name":"tab.close","args":{"id":"..."}}'`); a file on another machine
opens read only. Cmd/Ctrl+P is Go to file (`file.goto`, native: fuzzy by name over the project's files, recent first);
the tab and agent selector moved to Cmd/Ctrl+Alt+P.

For a remote space:

```sh
sunna do space.new --name "remote api" --machine vps --folder /srv/api
```

The folder is on that machine; tabs and splits in the space stay there.
New-space keys and clicks follow the focused space's machine. Script commands
without `--machine` still use this computer; `--machine ''` explicitly chooses it.
The agent menu checks and starts agents in the remote Unix user's
login/interactive shell (sh, bash, zsh, dash, ksh or fish), including its
configured PATH. Other shells keep the sh launcher.
`terminal.open --folder /path --place down` starts the split in that folder,
including when `--argv` supplies a program. Without `--folder`, it follows the
focused terminal's folder. If an inherited local folder was deleted, new spaces,
tabs and splits retry in that machine's home. An explicit invalid folder
still fails.
`hc machine ls` lists machines. Use `hc run --on vps ...`, `hc exec --on vps ...`
and `hc file ... --on vps ...` for remote work, not your local file tools.

`do` accepts `--key value` arguments (JSON for arrays/objects) or one JSON
object. Unknown argument names fail and list valid names. Target IDs (`id`,
`pane`, `space`, `target`, `terminal`) must be strings; other non-null types fail before
the current-target default can apply. Null keeps its omitted-argument meaning.
Use `commands` to confirm the command's exact shape:

```sh
sunna do pane.split '{"direction":"right","pane":"p12"}'
```

`pane.swap {pane?, direction | target}` trades the places of `pane` (default:
the focused one) and its neighbor in a direction, or `target`, another pane of
the same tab. Give `direction` or `target`, not both. Focus stays with the same pane:

```sh
sunna do pane.swap '{"pane":"p12","target":"p14"}'
```

`terminal.select_all {terminal?}` selects a terminal's whole screen and scrollback,
ready for `terminal.copy`. Without a target it follows keyboard focus. In vim and
other alternate-screen apps, it selects only that screen. Cmd+A runs it on macOS;
Linux and Windows have no default, so Ctrl+A stays with the shell.

`terminal.select {terminal?, at?: {row, col} | "cursor", mode?: "cell" | "word" | "line"}`
replaces the selection, at the program cursor by default. Cells are zero-based; rows
count from the oldest retained history through the active screen. Word and line
match double/triple click. `terminal.adjust_selection {terminal?, direction}` moves
the inclusive moving end and follows it in the view: left, right, up, down, page_up,
page_down, home, end, line_start, line_end. It fails without a selection.
`terminal.clear_selection {terminal?}` clears it; `terminal.copy` copies the focused terminal's selection.
These add no default keys or built-in copy mode. Mods read the inclusive selection
endpoints from `focused().selection` or `terminals()`; null means none (or no
attached view). Use `selection.end` as a copy-mode cursor. Selection facts update
when the selection changes, without refreshing readers on ordinary terminal output.

`terminal.search` opens a find bar in the focused terminal. It searches that
terminal's screen and scrollback; Enter and Shift-Enter move between matches,
Escape closes the bar while its find field has focus, and each terminal keeps its own query (reopening selects it, so typing replaces it).
Clicking the terminal while the bar is open puts typing, including Escape, back in the program; the bar and its matches stay.

It prints the created IDs (for example `{ "pane": "...", "terminal": "..." }`)
or `{ "error": "..." }` and exits 1. Read the result, then inspect `ls` or
`status` to confirm. Commands invoked by an agent never open a missing-argument
dialog: supply all required arguments. A successful launch reply means the
terminal was created, not that its program started correctly. If it exits
with a nonzero code within two seconds, the window shows an error toast with
the exit code and last output lines.

Quick commands and the floating terminal: `sunna do terminal.quick '{"name":"test"}'` types the saved
command `test` (`shared("quick_commands")`, edited in Settings > Terminal) into the focused terminal;
`quick.set {name, command, enter?}` and `quick.delete {name, confirmed: true}` manage them.
`sunna do terminal.float` shows or hides one floating shell that keeps its session
(`shared("float.terminal")`); it is not a tab. `sunna do app.notify '{"title":"x"}'` posts a toast.

`tab.close` on a space's last tab: from a key or click (Cmd+W) it closes only the tab and leaves
the space with a new terminal; from a script (`sunna do`) it still closes the space (asks unless
`confirmed: true`). `space.close` always closes the space. New settings.json keys for terminals
(`cursor_shape`, `cursor_blink`, `cursor_opacity`, `inactive_pane_opacity`, `divider_width`,
`focus_follows_mouse`, `hide_mouse_on_type`, `confirm_close_running`, `shell`, `middle_click_paste`,
`trim_copy_margin`), alerts (`notifications.bell`, `notifications.suppress_when_focused`) and the window
(`reduce_effects`, `show_app_name`) are described in the guide in your config folder; `shell` only
affects terminals the app starts on this machine.

A terminal started with hc opts into the window with `herdr.space=<id>`.
`herdr.tab=<id>` chooses its tab; without it, Herdr creates a tab in that space.
Unlabeled workers stay hidden. Do not add labels just to run a background
worker. Use app commands when the
user wants to see a new tab or split, hc for orchestration that need not change
the screen.

## Customize layouts, keys, or mods

First read `~/.config/sunna/AGENTS.md` (or `$SUNNA_HOME/AGENTS.md` if
set). It is the installed guide to settings, layouts, commands and mods.
Read the generated `.d.ts` files beside it for full API details; do not edit
those files or the generated guide. The app rewrites them on start.

Every dialog (confirmations, Edit details, Go to file, the text prompt, Add project, Add a machine, the worktree dialog) is native with one shared look; `openOverlay({mod: "confirm" | "prompt" | "add-project" | "machine-add" | "worktree" | "goto-file"})` still opens them.

The status bar, the top bar's brand and tabs, the sidebar's spaces list and
footer, and the empty-state welcome are native blocks of the layout
(`{"native": "status"}`, `"brand"`, `"tabs"`, `"spaces"`, `"files"`, `"changes"`, `"sidebar-footer"`,
`"welcome"`), not mods; `{"mod": "workspace-status"}`, `{"mod": "tabs"}`,
`{"mod": "spaces"}`, `{"mod": "files"}`, `{"mod": "changes"}` and the other old names still show them. The sidebar is
shown or hidden whole (`layout.toggle` on `sidebar`, ctrl-shift-b / cmd-b). Settings > Look and the shared keys
`status.items` / `status.format` configure the status bar as before.
The sidebar list has a view menu (Group by, Sort by, Project order, Filters, Go to current space); `sidebar.view`,
`sidebar.filter`, `sidebar.collapse_all`, `sidebar.reveal_current`, `sidebar.focus` (the keyboard to the list: arrows move a ringed cursor, Enter shows the space, the menu key or Shift+F10 opens its menu; ctrl-shift-e, cmd-shift-e on Mac), `space.status {id, status}` and `space.pin {id, pinned}`
do the same from `sunna do` (they write the `sidebar.*`, `space.status` and `space.pinned` shared keys).
Bots are native (panel `{"native": "bots"}`): `bot.new {name, template?, agent?, folder?, yolo?}`, `bot.run {name}`, `bot.enable {name, on?}`, `bot.delete {name, confirmed?}`. `cleanup.run {confirmed?}` cleans up spaces by the rules of Settings > Git and spaces (shared `cleanup.*`); `worktree.new|open|cleanup {space?}` open the worktree dialog. The tab, pane, + (new tab), awake and help (`app.help`) menus are native; a mod's `{"mod":"menu"}` overlay of those names gets the native one.

The sidebar's context menus are native; each item is a command you can `sunna do`: `space.read`, `space.open_in`, `space.copy`, `space.details`, `space.remove {id, confirmed}`, `project.add {step?, folders?}` (a person's Add project menu; `folders: [paths]` opens a space per folder), `project.icon`, `project.hide`, `project.open_in`, `project.copy`, `project.remove`, `project.worktree`, `project.move {key, index, group?}`, `machine.rename`, `machine.check`, `machine.remove` and `sidebar.menu {kind: space|project|machine|group, id|key|machine}`.
Project groups and lineage: `group.new {key, name?, color?}`, `group.rename {id, name}`, `group.color {id, color}`, `group.delete {id, confirmed}`, `group.move {key, group?}` (`group: ""` takes it out), `group.reorder {id, index}`, `group.refresh {id}`, `space.parent {id, parent?}` and `space.worktree {id}` (the dialog's «Create from this space» nests the new worktree under the space). Their shared keys are `sidebar.groups`, `sidebar.project_keys` and `space.parents`. A person drags a space, a project or a group header to reorder (a space onto a state heading changes its state; a project onto a group heading puts it in) or moves the row under the keyboard cursor with Alt+Up and Alt+Down; each is one of the commands above.

- `terminal_line_height` (default 1, 0.8..2) adjusts terminal rows;
  `terminal_cell_width` (default 1, 0.8..2) adjusts terminal columns;
  `terminal_scroll_speed` (default 1, 0.1..5) adjusts wheel and trackpad
  scrolling, including mouse reports. Set them in settings.json or the
  layout's look; the layout wins.
- Home defaults to `~/.config/sunna`; `SUNNA_HOME` selects another.
- `worktrees.directory` in settings.json sets the shipped New worktree destination,
  e.g. `{"worktrees":{"directory":"~/Developer/worktrees"}}`. Default:
  `~/.herdr/worktrees`; checkouts use `<directory>/<repo>/<branch>` (slashes in
  branches become dashes). Use an absolute path or `~` / `~/...`, resolved on the
  space's machine. Read on dialog open; existing checkouts stay put. No clones
  needed, and the app does not read the TUI's config.toml.
- Mods live in `mods/<name>/` with `main.js` and optional `mod.json`.
  They are trusted JavaScript with command and network access.
  `"focusable": true` in `mod.json` lets `pane.go` move onto a copy; it receives
  `"focus"` from `nextCommand()` and focuses its own keyboard handle.
- A mod's `watchFolder(dir)` promise has `cancel()`; cancel it when the folder
  is no longer shown. It resolves with `[]` and releases only that call.
- Shipped mods belong to Herdr: duplicate with `mod.clone` before editing.
  Shipped layouts likewise use `layout.clone` for a personal copy.
  `layout.edit` toggles a drag mode for rearranging the blocks of the shown
  layout; `layout.move {id, before|after}` does it by command, and on a shipped
  layout first copies it to `my-<name>` (reused on later moves).
  Discover required arguments with `sunna commands`.
- Save to reload. A broken save keeps the old version running and reports
  an error in `status`; a still-working screen is not proof the edit worked.
- Make only the requested change. Don't touch unrelated mods or settings.
- Use theme colors. Do not retain a mod's `cx` across callbacks; each
  callback receives its own. Native animations, not redraw timers, keep
  the window cheap to run. Root `on_size_change((size, cx) => …)` reports
  logical pixels after layout, including resizes without pointer movement.
  Optional `View.close(cx)` gets one bounded final call; start cleanup there,
  not in an earlier task. The installed guide explains its 250 ms quit budget
  and which cleanup commands go out before shutdown.

After every mod change:

```sh
sunna check ~/.config/sunna/mods/<name>
sunna status
sunna shot /tmp/mod.png   # then look at the picture
```

`check` loads the mod and renders it once. It exits nonzero with the exact
error. Its shared state starts empty and stays in memory; it leaves shared.json
untouched. Its `init()` still runs: import `checking` from `"herdr"` and guard
process side effects with `if (checking()) return;`. It is true throughout a
check and false in a live window; `exec`, `hc` and `writeTextFile` still run
and warn during checks. Never call a mod done without a passing check.
Then confirm it is shown correctly in the running window with `shot`; don't claim a visual
check you haven't made.

`sunna shot [file.png]` saves the window's current frame and prints its
absolute path (default: `herdr-shot.png` in your current folder). It is the
app's own rendering at full pixel size: terminals, images, overlays and
floats included, even when the window is covered or minimized. No screen
capture or permissions. It needs a running window that has drawn a frame;
`SUNNA_HOME` selects the window, as for `status`.

## Problems and safety

If Herdr misbehaves, `sunna feedback "<what happened>"` drafts a report.
The person reviews and sends it; it does not silently send feedback.

Never experiment with pointer/key automation on the person's real display.
When developing the app on Linux, its `scripts/sandbox` provides an isolated
home, core and display: `up`, `do`, `status`, `shot`, then `down` when done.
Never stop a server or session you didn't start.

The palette, Go to..., What's new, the welcome and Send feedback are native dialogs (no `frame/` mods). `mod.open {name}` with `picker`, `frame-palette`, `frame-whatsnew`, `frame-welcome` or `frame-feedback` opens them. Esc or the ✕ closes; the welcome records shared `frame.welcomed` (`finished` or `skipped`).
