# Sunna app

<!-- Written by the Sunna app on every start. Edits here are overwritten. -->

This folder is **the Sunna app's whole interface, as files**. The Sunna app
is a desktop app for terminals and coding agents, and it redraws itself from
these files while it runs: change a layout, a key or a panel here and the
window follows within a second. Shipped layouts and mods are Sunna's:
change settings for look, or clone before editing structure/code (see Layouts).
When someone asks for something ("a sidebar
of my PRs", "give ctrl+1 back to the shell", "make it warmer", "move the
agents list to the right"), you do it by editing these files or writing a
mod. Nothing in the default interface is special: the spaces sidebar, the
agents list, the tab bar, menus, dialogs and notifications are built
from the same parts you have.

## Two programs

- **`sunna`**, the window: spaces, tabs, layouts, panels.
- **Sunna Core (`hc`)**, which runs every terminal and agent. They keep
  running when the window closes or updates. You can use `hc` yourself:
  start a terminal, read its screen, send it text, wait for an agent
  (`hc --help`). The Sunna app is one client of it.

`sunna` controls this app; `hc` controls Sunna Core.

## What you can and can't do

You can: rearrange and restyle the whole window (layouts), rebind any key,
write panels (mods) that list, filter and act on spaces, tabs, agents
and terminals, open dialogs, popups and floating blocks, show a live
terminal inside a block, run any app command, run programs (`exec`) and
fetch URLs, keep state per mod or shared between mods, animate natively.

You can't: use HTML, CSS, a web view, Node APIs or `setTimeout`; draw
outside your block except through overlays; change how a terminal itself
draws text; read or change other mods' code at run time.

## What mods are written in

Mods use **gpui-kit** (its gpui-shell): JavaScript that describes **GPUI**
elements, a native GPU UI (`div()`, flex, children,
`.on_click`). There's no DOM: the script describes, Rust draws. Background:
https://gpui-kit.com. The Sunna app carries its own copy with additions
(`frames`, `loop`), so **the type files here are the truth**:
`gpui-kit.d.ts` for elements and `sunna.d.ts` for the app's data and
commands. They're rewritten on every start to match this version.

## Changing things safely

- Edits apply at once. If a file is invalid, the app keeps the last good
  version and writes why to `<name>.error` (`layout.error`, `keys.error`,
  `settings.error`, `programs.error`). Check after every edit.
- `sunna check <mod folder>` loads a mod once and checks its initial
  description and eager native elements, including virtual lists and motion.
  It does not lay out the mod or run deferred rows, nested renders, or later
  async states. Never call a mod done without a passing check; also try it
  in the app.
- **Escape hatch:** `ctrl-alt-shift-escape` always works: it sets the shown
  layout and `keys.json` aside (`*.reset-<time>`) and goes back to the
  default `main` layout. Nothing can rebind it.

## Seeing the window

`sunna shot [file.png]` saves the current window frame as a PNG
and prints its absolute path (default: `herdr-shot.png` in your current
folder). Use it when debugging mod UIs: inspect spacing, colors, terminal
images, overlays, popups and floats. It captures the app, not the desktop,
at the window’s physical pixel size, even when covered or minimized. No
screen-recording permission is needed. Sunna must be running; before its
first frame or during GPU recovery it reports an error instead of a PNG.
`SUNNA_HOME` selects which running window to ask, as with `status`.

## The files

The home is `~/.config/sunna` (`SUNNA_HOME` picks another; the older
`HERDR_APP_HOME` still works). A home from when the app was called herdr-app
(`~/.config/herdr-app`) is copied there once, on the first start, and left as is.

Terminals, spaces and layouts live in Sunna Core, in the session `sunna` (the app
sets `HC_SESSION=sunna` unless it is already set), so Sunna and the original Sunna
don't see each other's. To reach Sunna's terminals with `hc`, run it with `HC_SESSION=sunna`.

| File | What it decides | Applied |
|---|---|---|
| `layouts/<name>.json` | the window and its whole look: blocks, theme, fonts, backgrounds, pane frames; one file per layout (`main`, `terminals`, `win98`, any you add) | live |
| `keys.json` | what every key does | live |
| `settings.json` | the person: zoom, sounds, fonts for every layout, worktree directory | live |
| `mods/<name>/` | panels: small JavaScript programs | live (hot reload) |
| `programs.json` | how programs show (name, icon, tile), on top of the built-in table | live |
| `icons/` | the shipped program icons | written by Sunna (put your own elsewhere) |
| `frame/` | (empty now: the app's own screens are native) | rewritten by Sunna on every start: don't edit, it won't stick |
| `view.json` | this screen's selected space/tab/pane, zoom, settled tabs, recent spaces and seen attention | written by Sunna |
| `shared.json`, `data/`, `seen.json`, `installed.json`, `window.json`, `windows.json` | mod state, bookkeeping, where the window was (size, maximized), mods' windows (spots, kept ones) | written by Sunna |

Shared state is saved atomically in the background, with the latest change winning.
If `shared.json` is invalid, Sunna logs a warning and keeps it as
`shared.json.corrupt-<time>` before starting empty.
`sunna check` uses empty shared state in memory and leaves this file untouched.

## Dialogs

Every dialog of the app (confirmations, the palette, Go to..., Go to file, What's new, the welcome, Send feedback, Add project/machine, worktrees, prompt) is native and built from one component: a centered card, a ✕ of 32 px, Esc closes, Tab stays inside, the focus ring is always visible. A click on the dimmed area closes a dialog only while nothing is typed in it. The names `palette`, `picker`, `frame-palette`, `frame-whatsnew`, `frame-welcome` and `frame-feedback` still work in `mod.open` and `openOverlay` and open the native dialog; a mod folder of your own with one of those names takes over. `folders` (the retired folder search) opens the same folder picker as Add project > Browse. `overlay.close {id: "frame-welcome"}` closes the welcome.
Notifications (toasts) have a 32 px ✕ and also close with Esc while the keyboard is outside a terminal (Tab and Shift+Tab walk the controls there; `sidebar.focus` brings the keyboard out of the terminal); bind `overlay.close` to close one from a terminal.

## The model: spaces, tabs, terminals

Every layout shows the same world; a layout may show less, never a
different one. Each machine keeps its spaces, tabs and splits in its core's
`sunna` document. Connected screens share that layout and live terminals,
not their focus, zoom or attention. Use commands to change structure; there
is no `spaces.json`. On simultaneous edits the newest accepted document wins
(no merging); structural commands fail while that machine's layout is unavailable.
Mod pane props are stored as given, never machine-qualified. Space order is local
machine first, then machines by name; `space.move` stays inside its machine's block.
In the expanded Spaces sidebar, drag a space to reorder it within its machine;
the highlighted line shows where it will land.
Click a project header to fold it to just that row (a dot shows when one of its
agents works or is blocked); the folded keys are `shared("space.folded")`.
Right-click a project header for its menu (new and open worktree in a repo, show
in the file manager on this machine, copy path, hide). Hidden projects leave the list
and sit under a folded "Hidden projects" row with Show; the project holding the
active space is never hidden. The hidden keys are `shared("space.hidden")`.
A project's tile shows the icon the repo ships (the first of `assets/icon.png`,
`icon.png`, `logo.svg`, `public/favicon.png`, and similar, local folders only),
copied into `project-icons/` in the home; without one it is the first letter.
The sidebar has no narrow rail: its button (and ctrl-shift-b / cmd-b, `layout.toggle`
on `sidebar`) shows or hides it whole, and the top bar looks the same either way.
A saved `sidebar.collapsed` or a sidebar under 160 px counts as hidden.
The Projects `+` opens the native Add project menu (`project.add`): browse folders (the system picker
takes several; one space per folder), clone from a URL, or create a new project (`git init`); each ends in
`space.new {folder}`. The folder button of the sidebar, the palette and the menus open the same picker. The old
`{"mod": "add-project"}` overlay and `mod.open add-project` still open it.

- A **space** is a place to work, on one machine: this one, or another
  from `hc machine ls` (`spaces()[i].machine`). Sunna leaves out machines
  added with `--temp` (an agent's short-lived sandbox): no spaces, not in
  `machines()`, and mods listing machines skip `temporary`. Names work as in Sunna:
  a space nobody named is called after where its first terminal is now
  (its repo, else its folder, `~` at home) and follows it; once named
  (`space.rename`), the name stays; renaming to "" goes back.
  `spaces()[i].named` says which. Where a space is now is its terminals'
  live folders (`panes()[i].cwd`), not the folder it was made in: new
  tabs start where its focused pane is, and the default mods take a
  space's repo from its first terminal.
  If starting in an inherited folder fails, new spaces, tabs and splits retry in
  that machine's home. Explicit invalid folders still fail.
- A **tab** is a piece of work in a space: its terminals and how they're
  split right and down. One nobody named is called by its place (1, 2,
  ...), as in Sunna; `tab.rename` names it ("fix login"). A space
  has one or more; closing the last one closes the space (it stays in
  `recent`).
- A tab can be **settled** (`tab.settle`): put away on this screen, still running;
  `settled` on each tab. The shipped layouts don't use it (they list every
  tab); a mod that does can hide or fold settled tabs. Showing one, or an
  agent in it needing the user, brings it back.
- A **pane** is a terminal on screen, or a **mod** beside them (a markdown
  viewer, a board, a git panel): `pane.split {direction, mod, props}`. A
  mod pane splits, zooms, closes and gets a header like any pane, keeps
  the keyboard when clicked, and is saved with its tab; its `props` tell
  it what to show (a file, a folder). Closing a terminal pane ends its
  terminal, and a program exiting closes its pane, even if Sunna was closed.
  Terminal panes are kept by their machine's core (`keep: true`). A lost
  terminal (core crash, reboot, logout) gets a shell in its last folder,
  with the same address; an agent whose programs.json entry has `resume` is started again picking up its conversation (unless `resume_agents` is false); any other command is never run again. Splits
  and mod panes stay. An exited or killed terminal is closed, not restarted.
  Focus, zoom, recent spaces, settled tabs and seen attention stay on this screen.
- Every terminal is a core terminal. Sunna labels the ones it starts with
  `sunna.space=<id>` and `sunna.tab=<id>`. **A terminal with those
  labels shows up there**, even one you start yourself:
  `hc run --keep --label sunna.space=<id> --label sunna.tab=<id> -- claude`.
  Labels use ids local to that machine; remote ids shown by the app have a
  `<machine>/` prefix (strip it when writing labels on that machine). Adoption
  also enables core keep; `terminal.park` removes the pane and disables keep.
  When a core change is needed, `terminal.keep` / `terminal.park` acknowledge
  with `pending: true`, then apply after the machine replies; failures show a toast.
  Normal quit waits for pending terminal changes and layout writes; remote waits are bounded.
  An id Sunna doesn't know becomes a new space or tab (unnamed: Sunna's names).
  Terminals without the labels (another app's) stay out of Sunna's
  screens, and so do other machines' terminals, unless Sunna started them
  for a space on that machine.
- **Agents** are terminals whose program reports a status (OSC 7501):
  `working`, `blocked`, `idle`, or `done` = finished while the user wasn't
  looking (it becomes `idle` once they look). A space's and a tab's
  status is the most urgent of its agents: blocked > done > working > idle.
- **Sub-agents** are terminals an agent started with `hc run`: the core
  records the starter as `parent` (see `terminals()`). Parent addresses use
  the app's machine names, or a bare ID for this computer, when they can be
  mapped; otherwise they stay as the core reported them. They belong to their
  parent's tab without a pane; the native `agents` block shows them,
  `terminal.peek` opens one, `terminal.keep` gives one a pane.
  Ctrl-click (Cmd-click on macOS) a live terminal address in terminal text to
  peek at it. Bare addresses refer to the machine of the terminal showing them;
  use `machine/address` for another machine. Names and non-live addresses aren't links.
- **Agent navigation order** belongs to the mods. A mod displaying agents calls
  `setAgentOrder(list.map(a => a.address))` on render so `agent.next` /
  `agent.prev` follow its list.

## Layouts: `layouts/<name>.json`

Each file is one whole window. `layout.switch` (`ctrl-shift-l`) cycles
through them, `layout.switch {name}` picks one; `shared("layout")` is the
shown one's name. Sunna ships `main` ("Sunna": Search and Bots, then the
projects (spaces by machine, worktrees under their checkout, each with a
fold-away list of its agents and their status) on the left, `spaces`, and the native `sidebar-footer`; the top bar's left part is the
native `brand`; the native `tabs` bar (tabs as pills with a + menu
that starts a terminal or any installed agent: native, as are the terminal, tab, awake and help menus); panes as cards
with a header; native Files and Changes panels on the right (`{"native": "files"}`, `"changes"`), opened from the tab bar and remembered per space (which ones:
shared `"workspace.panels"`, set by the welcome); a native status bar,
`status`), and as examples `terminals` ("Terminals": tabs in
the title bar, panes as cards, a space picker) and `win98`
("Sunna 98": a whole desktop in one mod, `win98`). Add a layout by
writing a new file. The native Files panel (`{"mod": "files"}` still shows it; settings `files.ignored`, `files.open`) supports Windows folder paths with spaces,
parentheses and apostrophes, and preserves Unicode file and folder names
(including Chinese). Directory links expand as folders; their contents load
only when expanded.

**Sunna's own layouts and mods are read-only.** They are replaced whole
on every start. Use settings.json for look (fonts, sizes, theme, colors,
sounds), where the layout leaves them unset. Sunna (main) has no look of
its own, so your settings decide it; Sunna 98 and Terminals bring their own
look. The theme picker overrides any layout. To change a shipped layout's
look beyond that, or its structure/code, clone it and edit the clone:

```sh
sunna do layout.clone '{"name":"main","as":"my-layout"}'
sunna do mod.clone '{"name":"search","as":"my-search"}'
```

The default clone name is `<name>-copy`; an existing name is refused.
`layout.clone` switches to the clone. In the palette: Duplicate layout
or Duplicate mod. Clones are yours and never updated. They record
`cloned_from: {name, version}` and copy the layout and its clicks as-is,
including its look. A cloned layout keeps its mod references:
clone a mod too and point your layout's `content.mod` to it to change code.
Mod clones rewrite their own `mods/<name>/` asset paths and self-opening
overlay names to the new name.

If you edit a shipped file anyway, the next start moves it to
`<name>-edited` (a mod folder or layout JSON; a number is added if taken),
copies its layout clicks, restores Sunna's version and shows a toast.
No-longer-shipped things go if
untouched, otherwise they stay.

Clicks (sizes, shown/hidden, stack child, floating spots, theme picks,
background, text size, pane header height and border/focus look) live
in `layouts/.state/` and survive restarts. Don't edit those files.
A deliberate layout-file edit beats a click on that field.
`layout.reset {name}` resets a shipped layout and its clicks, keeping its
previous file aside as `<name>.json.old-<time>`.

A layout is a tree of nodes. Four types:

- `block`: shows content. `"content"` is `{ "mod": "<folder>", "props": {...} }`,
  `{ "native": "<name>" }` (a block the app draws itself, in Rust: `status`
  (bottom bar: machine, branch, agents, awake cup, commands key; reads
  `status.items` / `status.format`), `brand` (top bar's left part: logo,
  search, sidebar button; it keeps the width of the block `sidebar` and
  makes room for the macOS window buttons), `spaces` (the sidebar's list: search, bots, projects with
  their spaces), `sidebar-footer` (Settings, Help and
  Add machine), `welcome` (no space open) and `tabs` (the tab bar: the active
  space's tabs, `+`, one button per side panel, the window buttons where the
  layout draws them); an unknown name is an empty
  block, so a layout written for a newer version still loads),
  `{ "terminal": "t7" }`, `{ "tab": "active" }` (the shown tab's
  terminals as it arranges them), or `{ "agent": "focused" }` (one agent's
  terminal, full size: the one chosen with `agent.focus` / `agent.next`, else
  the focused pane's agent, else the first agent).
- `row` / `column`: children side by side / stacked.
- `stack`: shows one child (`"active": "<child id>"`), with an optional
  `"header"` (a mod that draws tabs, or `"tabs"` for a plain built-in strip).

Properties any node can have:
- `id` (unique; commands use it), `size`: `{ "px": 240 }` or `{ "weight": 2 }`
  (shares of what fixed siblings leave), `min` / `max` (px), `hidden`.
- Paint: `background` (a color; `{ "gradient": ["#2a6cbd", "#0b2a57"], "angle": 165 }`;
  or `{ "image": "backgrounds/sky.jpg", "fit": "cover" | "contain" }`, a path in
  the home folder), `border` (`{ "width": 1, "color": "border" }`), `radius`,
  `shadow` (`"sm" | "md" | "lg"`), `focus` (`{ "border": {...} }` when the
  focused pane is inside).
- `padding`: space inside the node's edge, px: `16` or `[vertical, horizontal]`.
- row/column: `gap` (px between children), `divider` (`{ "width": 1, "color": "border" }`),
  `resizable` (default true: drag the boundary). This applies to that row/column only.
- `movable` (default true): panes can be dragged by the app's top-center grip within a split tab.
  Unlike `resizable`, it inherits: the nearest explicit ancestor wins, through the
  tab tree and then the layout's tab block and its ancestors. Root `"movable": false`
  opts the layout out. Composition terminal blocks aren't tab panes and never drag.
  Center drop swaps panes; an edge drop places beside it. Escape or dropping outside
  another pane cancels. Zoomed tabs and one-pane tabs don't drag. The grip appears
  on top-edge hover, regardless of the header's position or presence. Only the grip
  starts dragging; header mods don't need drag handlers. `panes()` exposes `movable`.
- `floating`: blocks over this node (below).
- `window_drag: true`: dragging the node moves the window, a double-click
  maximizes it. A mod's own control inside opts out by stopping the press:
  `.on_mouse_down("left", (_e, cx) => cx.stop_propagation())`.
- Root only: `"titlebar": "none"` hides the system title bar so the layout
  draws its own (a bar mod in a `window_drag` block). macOS keeps its window
  buttons over the layout at `"traffic_lights": [x, y]` (px); leave them
  room. Where the system draws no buttons (`window().buttons`, Linux and
  Windows), the bar draws them with `window.minimize`, `window.maximize`, `window.close`.
  Windows' shipped tab bar and system strip use vector window icons, with a
  restore icon when maximized, rather than font glyphs.
- Root only: `"window_blur": true` blurs what's behind the window where the
  system can (macOS, Windows, some Wayland compositors). Colors with alpha
  then show the desktop through frosted glass: make `background` and
  `canvas` transparent and give the sidebar a translucent `chrome`
  (`"#0b102466"`). Nodes with solid colors stay solid.
- Root only: `"preview": "layouts/<name>.svg"` (or png): the picture the
  settings screen shows for this layout. Draw one when you make a layout;
  without it the person sees a plain frame. New layouts and `themes/*.json`
  show up in settings by themselves.
- Root only: `"terminal_menu": "my-menu"` chooses the mod for terminal
  right-clicks (including Shift+right-click). Omit it for the shipped `menu`;
  an unknown mod logs a warning and falls back to `menu`. Clone a shipped
  layout before changing this field. Other menus are unchanged.
- Root only: `"toast": "my-toast"` chooses the mod that shows notifications
  (Sunna's own and every `notify()`), in place of the native toast (the app draws it; there is no `toast` mod
  any more); an unknown mod logs a warning and uses the native one. It gets the same
  `props()`: `{title, body, level, icon, action, timeout, overlay}` (`timeout`
  is optional). A mod that hands them to the system instead
  (`exec("notify-send", ...)`) closes itself at once
  with `closeOverlay(props().overlay)`.
- Colors are theme names (`border`, `accent`, `surface`, ...) or `#rrggbb[aa]`.

A personal terminal menu replaces the whole menu; it doesn't merge items.
Create `mods/my-menu/main.js` and `mod.json` (e.g.
`{"place":"none","overlay":{"width":280,"height":100}}`), then set the layout root's
`"terminal_menu": "my-menu"`. It opens as a pointer popup, default 220×260 px;
its `mod.json` overlay width/height override those defaults. It gets
`props()` = `{tab: "pane", target: {pane, terminal}, overlay}`. Always use
`target.terminal` or `target.pane` in actions, not the focused terminal:
right-clicking a different terminal must act on the one clicked. A terminal
shown outside a tab (a layout terminal block, a window) has no pane:
`target.pane` is its block's id, not in `panes()`, so leave out pane items
there, as the shipped menu does. For example:

```js
import { View, div } from "gpui-kit";
import { props, run, closeOverlay } from "sunna";
export default class MyMenu extends View {
  render() {
    return div().id("split-target").child("Split this pane").on_click(() => {
      const {target, overlay} = props();
      closeOverlay(overlay);
      run("pane.split", {pane: target.pane, direction: "right"});
    });
  }
}
```

### The look: on the layout, cascading

A layout's own look wins; settings.json fills what it leaves unset.
The theme picker overrides both. These go
on any node, and the nearest node up the tree wins, so the root sets the
layout's look and a block can differ:

```json
{ "id": "root", "type": "row", "theme": "graphite", "colors": { "accent": "#a7b4c6" },
  "font": "Geist Mono", "font_size": 15, "ui_font": "Geist",
  "terminal_padding": [10, 6], "terminal_opacity": 1, ... }
```

- `theme`: `graphite`, `ink`, `moss` (dark), `paper` (light), or the name
  of a file in `themes/`; or one for each appearance:
  `"theme": { "light": "paper", "dark": "graphite" }` follows the system's
  light or dark mode (or the person's `appearance`). `app.theme {name}`
  sets the half of the name's lightness. A layout without a theme shows
  settings.json's theme, else graphite or paper. `colors` overrides any theme color by name (see
  `theme()` below), for terminals and panels alike: overrides apply in
  both appearances, so name theme colors (`chrome`, `canvas`) rather than
  writing hex when a layout should work light and dark.
- `themes/<name>.json`: `{ "background", "foreground", "selection",
  "cursor"?, "ansi": [16 colors], "colors"?: {...} }`, all `#rrggbb`; panel
  colors are derived from these, `colors` overrides them. Sunna ships
  `atom-one-dark` and twelve more (Catppuccin, Dracula, Nord, Tokyo Night,
  Gruvbox, Rosé Pine, GitHub). The original unedited Atom One Dark and Win98 theme files
  are upgraded to readable terminal colors; edited theme files are kept.
  A theme named like a layout (`themes/win98.json`) is
  that layout's own: Settings offers it only there.
  A theme may also carry `"style": {...}`, how the interface is drawn: `radius`
  (0..24, base radius of the mods' controls, 6 = the usual), `pane_radius`
  (0..24) and `pane_gap` (0..32) and `border_width` (0..4; 0 removes the
  pane border and focus ring) of the pane frame, `shadow` (`none`, `sm`, `md`,
  `lg`), and `font`, `font_size` (6..72), `ui_font`. The chosen theme wins over
  the layout and settings.json, as its colors do. Mods read it with `style()`
  and scale their corners with `Math.round(n * style().radius / 6)`.
- `terminal_line_height`: row height multiplier (default 1, clamped to 0.8..2).
  Text stays centred; joining characters, cursors and images follow the rows.
- `terminal_cell_width`: column width multiplier (default 1, clamped to 0.8..2),
  like Ghostty's `adjust-cell-width` as a percentage (`-5%` is 0.95). Glyphs
  are placed in their cells as Ghostty places them.
- `terminal_scroll_speed`: wheel and trackpad multiplier (default 1, clamped
  to 0.1..5), including mouse reports to programs. Small moves accumulate.
- `font` / `font_size`: the terminals' (px at zoom 1); `ui_font`: the mods'
  (their text sizes are their own, in px, and follow zoom). Optional: without
  them a layout uses settings.json's, else Sunna's (JetBrains Mono 14, Geist).
  Sunna (main) leaves these unset; Terminals sets text size 15, and Sunna 98
  brings its own fonts.
  JetBrains Mono, Geist Mono and Geist ship with Sunna; any installed font
  works (a terminal font the machine lacks falls back to JetBrains Mono).
  A layout brings its own fonts: `.ttf`/`.otf` files in a mod's `fonts/`
  folder (or the home's `fonts/`) load at start and on the next look change,
  so a layout and its mod folder travel together (Sunna 98's do: its pixel
  fonts are in `mods/win98/fonts/`).
- `font_weight` (100..900): the terminals' text weight. Default 500
  (Medium); 400 is Regular, 300 Light, using the family's nearest face.
  Bold text stays at 700.
- `pane_scrollbars` in settings.json (default `true`): show terminal scrollback
  with a slim thumb and track. Drag the thumb or click the track to jump there.
  With little or no terminal padding, the hit lane overlays the last 8px;
  terminal columns do not change. Hidden in alternate-screen programs.
- `terminal_opacity` (0..1): below 1, the background behind a terminal
  (a pane frame's, the layout's gradient) shows through its text.
- Switching layouts switches the look. The theme menu changes the shown
  layout's root `theme`.
- Today only the root's look applies; a block's own look is coming.

### Pane frames

The tab block says how each pane of the shown tab is wrapped:

```json
{ "tab": "active",
  "pane": { "gap": 6, "radius": 6, "background": "surface",
            "border": { "width": 1, "color": "border" }, "focus": { "width": 1, "color": "accent" },
            "header": { "content": { "mod": "pane" }, "height": 30, "position": "top", "reveal": "always" },
            "lone": false } }
```

- The look applies to every pane; `focus` replaces the border while the
  pane has the keyboard; `gap` is the space between panes.
- `header`: a mod drawn on each pane, one copy per pane, with
  `props().pane` and `props().terminal`. It reads its live facts from
  `panes()` (title, folder, branch, agent status, focused, zoomed, scroll)
  and its buttons run pane commands with that pane (`pane.split`, `pane.zoom`,
  `pane.close`). `scroll` is `{ total, offset, len, from_bottom }` for a
  terminal pane's history viewport (`from_bottom` is 0 at the end; `scroll`
  is null on the alternate screen). Reads get current numbers. Mods refresh
  on user scrolling and transitions between the end, history, and the
  alternate screen; ordinary output within one of those states does not
  refresh them just because the numbers changed. `position`: `top` or
  `bottom`; `reveal: "hover"` shows it only while the pointer is on the pane,
  drawn over the terminal so the terminal never resizes. The native header
  (`{"mod": "pane"}` or `{"mod": "terminals-pane"}`, drawn by the app unless you keep a mod
  folder of that name) keeps its divider on the terminal-facing edge and reads the frame
  position from `layout()`.
- `lone: true` frames a tab's only pane too (default: only when split).
- Each layout has its own, so one can stay bare while another makes
  every pane a card.

### Floating blocks

```json
"floating": [ { "id": "board", "content": { "mod": "board" }, "edge": "left", "align": "center", "peek": 52 } ]
```

A floating block sits over its node, `edge` (`top`, `right`, `bottom`,
`left`) and `align` along it (`start`, `center`, `end`), 12 px in. It sizes
to its content (render nothing to hide it). People drag it anywhere; on
release it springs to the nearest edge and third, and Sunna saves the new
spot. Clicks and hovers on it never reach what's underneath.

`peek` (side edges only) makes it a drawer: only that many px next to the
edge show until it's hovered, then the rest slides out; it slides back a
moment after the pointer leaves. The mod draws one full layout and never
changes it on hover (that's what keeps it smooth): put what should peek
(status, icons) on the edge side of each row.

Sunna draws the frame around a floating mod (theme background, border,
rounded corners, shadow; accent border while dragged), like a dialog's,
with 5 px above and below. The mod draws only its content, no panel: on a
side, rows can run edge to edge. It reads `placement()`:
`{ edge, align, vertical, hovered, dragging }`. Lay out vertically when
`vertical` (left/right edges), horizontally otherwise; on the right edge,
mirror rows so their edge side is on the right.

The default window:

```json
{ "id": "root", "type": "row", "divider": { "width": 1, "color": "border" },
  "children": [
    { "id": "sidebar", "type": "column", "size": { "px": 240 }, "min": 40, "max": 480,
      "divider": { "width": 1, "color": "border" },
      "children": [
        { "id": "spaces", "type": "block", "content": { "mod": "spaces" } },
        { "id": "agents", "type": "block", "content": { "native": "agents" } } ] },
    { "id": "work", "type": "column",
      "children": [
        { "id": "tabs", "type": "block", "size": { "px": 30 }, "content": { "mod": "tabs" } },
        { "id": "main", "type": "block", "content": { "tab": "active",
          "pane": { "gap": 6, "radius": 6, "border": { "width": 1, "color": "border" },
                    "focus": { "width": 1, "color": "accent" } } } } ] } ] }
```

The `main` layout puts a fixed top bar (`topbar`: `brand` and `tabs`) above `body` (`sidebar` and `work`). Sidebar on the right: swap the two children of `body`. No sidebar: set `"hidden": true` on `sidebar` (the user can toggle it: `ctrl-shift-b`). Narrower than 160px, the sidebar mods draw a compact rail; folding resizes `sidebar` to 44. `brand` holds the window buttons on macOS and follows the sidebar's width (dragged or folded; 178 px on macOS and 108 elsewhere while folded or hidden, room for the logo, search and the toggle. There are no edge-to-edge lines: the areas split by background and short faded dividers). Layouts copied from `main` before this keep their old shape.

### Themes, mods and panels people add from Settings

- Themes: Settings > Look shows each theme as a card with a preview. *Import*
  reads Ghostty, Warp and VS Code/Cursor themes into `themes/<name>.json`
  (never over an existing one); *New theme* opens an editor (colors, `style`)
  that saves the same file. Edit those files directly just as well.
- Mods: Settings > Mods lists `mods/<name>/` with the optional `mod.json`
  fields `name`, `description`, `author`, `version`, `homepage`, `preview` and
  `panel: { "title", "icon" }`. A mod can be turned off (its folder moves to
  `mod-store/disabled/<name>/`) and back, or deleted. Sunna's own mods can only
  be duplicated (`mod.clone`). The installed list has a search and an
  All/Active/Disabled/Updates filter; installed mods are checked for updates
  once per session. Settings has a search box that jumps to the section
  holding a setting.
- Install: from `owner/repo[@ref]`, a git URL or a folder. Sunna shows what
  it found and what each mod can do (programs, network, files) before
  copying; nothing existing is overwritten. An installed mod keeps
  `mods/<name>/.herdr-install.json` (`source`, `ref`, `commit`) and can be
  checked for updates. The *Explore* tab lists GitHub repos with the topic
  `sunna-mod` or `sunna-theme`.
- Right sidebar: the stack `panel-body` of the `main` layout holds the panels
  (`files`, `changes`, `search`, `bots`, `activity`). A mod whose `mod.json` has `panel`
  can be added as a child `panel-<mod>` (`{"content":{"mod":"<mod>"}}`);
  Settings > Layout > Right sidebar adds, orders and removes them, and
  `shared("workspace.panels")` lists the ids shown in the tab bar. Editing a
  Sunna layout copies it to `my-<name>` first.
- Search panel (native block `search`; `{"mod":"search"}` still works): text search in the focused folder with `rg`,
  else `git grep`, else `grep`. A result opens in the file editor on its line (`file.open`).
- Changes panel: branch, ahead/behind, fetch, pull (`--ff-only`), push, a
  branch picker and a History tab, run with `git` in the repository.

### Settings that live in `shared()` (Look, General, Git and spaces)

Settings writes these with `setShared`; the mod that owns each one reads them, and a
missing value means the default. You may set them from a mod too.

| Key | Value (default) | Read by |
|---|---|---|
| `space.card` | `{git, ago, agents}` booleans (all `true`): git line, time ago, agents under a space | `spaces` |
| `space.branch_names` | `true`: an unnamed space shows its branch (`false`) | `spaces` |
| `sidebar.buttons` | `{search, bots}` (both `true`) | `spaces` |
| `sidebar.group_by` | `"none"`, `"status"`, `"pr"` or `"project"` (default): what the sidebar groups spaces under | native `spaces` |
| `sidebar.sort_by` | `"name"`, `"agents"` (blocked, finished, working, then latest), `"recent"`, `"project"` or `"manual"` (default): the order inside a group | native `spaces` |
| `sidebar.project_order` | `"manual"` (default, as first seen) or `"recent"` (latest activity first) | native `spaces` |
| `sidebar.filters` | `{hide_idle, hide_default_branch, hide_bots, projects}` (all off; `projects`: project keys to show, `[]` all) | native `spaces` |
| `space.status` | `{<space id>: "todo" \| "in_progress" \| "in_review" \| "done"}`; a missing space is `todo` | native `spaces` |
| `space.pinned` | list of space ids shown in the Pinned section on top | native `spaces` |
| `sidebar.groups` | `[{id, name, color, projects}]` (`[]`): the project groups, in order; `projects` are project keys; `color` is `ansi6`, `ansi3`, `ansi4`, `ansi5`, `ansi2` or `ansi1` | native `spaces` |
| `sidebar.project_keys` | project keys in the order dragged (`[]`: as first seen); read when the project order is Manual | native `spaces` |
| `space.parents` | `{<child space id>: <parent space id>}` (`{}`): the sidebar nests a child under its parent | native `spaces` |
| `space.bot_made` | list of space ids a bot made, what «hide made by bots» hides (nothing writes it yet) | native `spaces` |
| `status.items` | `{machine, branch, agents, awake, hint}` (all `true`) | native `status` |
| `status.format` | `"total"` or `"working"`: which agent count shows | native `status` |
| `tabs.order` | `"strip"` or `"recent"`: the wheel on the tab bar moves by position or by last use | native `tabs` |
| `files.ignored` | `false` hides what `.gitignore` names (`true`) | `files` |
| `files.open` | `"click"` or `"double"`: a double click opens a file in the editor | `files` |
| `space.unread` | list of space ids marked unread (a dot on the row until shown) | native `spaces` |
| `space.notes` | `{<space id>: note}`, the tooltip of a space's row | native `spaces` |
| `project.icons` | `{<project key>: file in project-icons/}`, a project's chosen or found icon | native `spaces` |
| `menu.open_in` | app names (`[]`): an "Open in" item per app on local project and space menus (merged with the editors found on this computer); macOS `open -a <app>`, elsewhere the command | `menu` |
| `cleanup.auto`, `cleanup.every_hours` | `false` / `24`: automatic space cleanup (Settings > Git and spaces) runs every N hours while the window is open | native cleanup |
| `cleanup.merged`, `cleanup.unchanged`, `cleanup.unchanged_days`, `cleanup.bots` | `true` / `false` / `1` / `false`: the rules: merged branches, checkouts with no commits over the default branch for N days, spaces of finished bot runs | native cleanup |
| `cleanup.last`, `cleanup.last_run`, `cleanup.log` | unix ms of the last automatic check and last removal; the last 30 entries `{at, path, branch, reason, outcome: removed|failed|kept, note}`, newest first (written by the app) | native cleanup |
| `worktree.nest` | `false`: worktrees go in `<dir>/<branch>`, not `<dir>/<repo>/<branch>` (`true`) | native worktree dialog |
| `worktree.external` | `false`: Open worktree lists only the main checkout and those under the folder (`true`) | native worktree dialog |
| `worktree.prefix_mode`, `worktree.prefix` | `"none"` / `"git"` (the git user name) / `"custom"` with the text in `worktree.prefix` | native worktree dialog |
| `worktree.template` | branch name with `{name}`, `{date}`, `{repo}` (`"{name}"`); an existing branch is used as typed | native worktree dialog |
| `worktree.fresh` | `true`: a new branch starts from `origin`'s default branch after a fetch, and the local copy is fast-forwarded when free (`false`) | native worktree dialog |
| `changes.base` | `"head"` (uncommitted), `"default"` (since the repo's default branch) or `"upstream"` | `changes` |
| `changes.groups` | `"flat"`, `"status"` or `"new_first"`: group the file list by status | `changes` |

The worktrees folder is still `worktrees.directory` in `settings.json`; Settings > General
edits it.

The `activity` panel lists every agent with filters (status, program, text) and
counts, and keeps a log of when agents finished or needed you. The native `agents`
block (the sidebar list: groups, sort, pins) and the `activity` panel are Rust blocks
(`{"native":"agents"}`, `{"native":"activity"}`; the old `{"mod":"agents"}` and
`{"mod":"activity"}` names still map to them). The window writes that log to `shared("activity.events")` (newest first, up to 100:
`{kind: "blocked" | "done", address, name, at}`); `shared("activity.seen")` is
the time it was last looked at.

The native `welcome` block is the screen shown in the main area while no space is open
(and only then): the logo and name, a subtitle, the buttons *Add project* (the
Add project menu, `project.add`) and *Create workspace* (`space.new`), and the shortcuts for
`space.new`, `space.prev` and `space.next` read from `keys()`, so a rebound key
shows as rebound (an unbound action is left out). The app draws it in that
branch of the layout's `tab: "active"` block, and a layout may place it with
`{"native": "welcome"}`; it takes no props. These three blocks used to be the mods
`workspace-status`, `welcome` and `workspace-chrome`: a layout that still says
`{"mod": "workspace-status"}` (or `welcome`, or `workspace-chrome`, with
`props.footer` for the sidebar footer) gets the native block, unless you keep a
mod folder of that name, which wins. To change one, make your own mod and
point your layout's block at it (texts are `welcome.empty_*` and
`workspace-status.*` in `locales/`). `mods/workspace-chrome/` keeps only its
`*.svg` icons (no code), which mods may draw as `svg("mods/workspace-chrome/x.svg")`.
The native `tabs` block was the mod `tabs`: `{"mod": "tabs"}` in a layout gets the
native bar (unless you keep a mod folder of that name). It lists every tab of the
active space (caption: the name you gave, else the program's or the agent's live
title; a dot for an unsaved file, a ring for a working agent, a dot for blocked or
done; click selects, double click renames, drag moves, right click opens the
`menu` popup, the X closes the active one), and a button for each panel in the
`panel-body` stack that `workspace.panels` turns on (title and icon from the
panel mod's `mod.json` `panel`). The panel each space had open is remembered in
`data/tabs/store.json`. The wheel moves one tab (by `tabs.order`).
The Files and Changes panels show *Select a workspace to see ...* with no space.

### Bots: `bots/<name>/`

A bot is an agent that starts by itself (native panel *Bots*, `{"native": "bots"}`; a scheduler
checks every minute while the window is open, even with the panel closed). The sidebar's *Bots*
section starts folded; it and the hidden-projects section remember whether the user opened them
(`data/spaces/store.json`). Commands:
`bot.new {name, template?, agent?, folder?, yolo?}`, `bot.run {name}`, `bot.enable {name, on?}`,
`bot.delete {name, confirmed?}`. Files in `bots/<name>/`:

- `SOUL.md`: first paragraph is the purpose, the rest the instructions; it is
  the agent's first message.
- `bot.json`: `{ "agent": "claude", "folder": "~/dev/app", "enabled": true,
  "yolo": false, "max_runs_per_day": 10, "triggers": [...] }`. Triggers:
  `{"type":"manual"}`, `{"type":"interval","minutes":60}`,
  `{"type":"daily","at":"09:00"}`, `{"type":"github","query":"is:pr is:open
  review-requested:@me","every_minutes":10}` (needs `gh`).
  A `daily` trigger may add `"days": [1,2,3,4,5]` (0 = Sunday) and
  `"grace_minutes": 30` (a slot missed by longer is dropped). `bot.json` may
  hold `"precheck": "<shell command>"`: when it exits non-zero an automatic
  run is skipped (recorded with `"status": "skipped"`; *Ejecutar ahora*
  ignores it). A run that could not start is recorded `"status": "failed"`.
- `runs.json`: written by the app (last 100 runs, items already seen).

The panel's *Plantillas* button opens a gallery (search, categories: releases,
support, planning, personal) of starting points; each has the text it starts
with and its triggers. A template with a schedule or GitHub trigger creates its
bot switched off (`"enabled": false`) until the person turns it on; a manual
one is ready. With no bots the panel shows the templates as cards. Each bot's
card shows its schedule, when it fires next and how long ago it last ran
(*Pausar*/*Reanudar* switches `enabled`; *Eliminar* asks in a dialog, then deletes the folder), and *Ejecutar ahora* says why it would be refused (daily cap,
previous run still working). A bot's *Detalle* shows where it runs, what
permissions it has (it acts as the person) and its last 8 runs. The sidebar asks the panel for the gallery or a new
bot with the shared keys `bots.templates` and `bots.new` (a timestamp).

Each run is a tab `bot <name> #n` in the space `bot: <name>`, started without
taking focus. `yolo` is ignored for github triggers (their text comes from
outside), the first github poll only learns what exists, and a bot does not
start while its previous run is still working.

### Layouts and mods other people made

People share layouts and mods as GitHub repositories (topics `sunna-layout`, `sunna-mod`, `sunna-theme`), listed in Settings › Mods. Each
is a folder laid out like this home: a layout has `layouts/<name>.json` and
the `mods/<name>/` it needs; a mod is a piece for the person's own window,
so it has only `mods/<name>/` and maybe a `keys.json` with the keys it
expects. Both have a `RECIPE.md` (the brief to build it from scratch) and a
`preview.png`. The files are one build of the recipe. Asked to add one
(`<owner>/<name>` on GitHub or a
folder):

1. Read the recipe and every file before writing anything. Before adding,
   tell the person if a mod reaches the network, runs programs (`exec`,
   `hc`) or reads files outside its own folder.
2. Copy the layout file to `layouts/` and each mod folder to `mods/`. Leave
   the recipe and preview out. Never overwrite: if a name is taken (yours
   or Sunna's), rename the new one: its file or folder, the `mod` names in
   the layout, and the mod's own `mods/<name>/` paths.
3. Fit it to this setup only where its assumptions don't hold (a missing
   program, a name clash) or the person asks. If the files don't work on
   this version, or the person wants it different, build it from the
   recipe instead.
4. A mod's `keys.json`: merge it into the person's. Add its sections and
   keys, and ask before replacing a key they already use.
5. `sunna check` each mod; for a layout, then `layout.switch {name}`.
   The person's own layout stays as it was; `ctrl-shift-l` switches back,
   and deleting the files removes it.

To share one: put that folder in a public GitHub repo (one folder each if
there are several), write its `RECIPE.md`, save a screenshot of it as
`preview.png` and add the repo topic `sunna-layout` for a layout or
`sunna-mod` for a mod. It shows up at the Sunna catalog in Settings › Mods
within the hour.

A `RECIPE.md` is a brief: another agent with an empty folder builds the
layout or mod from it. Write it from the real files, in this order:

1. `# Name`, one paragraph of what the person sees, `![Name](preview.png)`.
2. **What you see:** the regions top to bottom or left to right, their
   sizes, colors and type; what each one shows.
3. **How it behaves:** every agent status (working, blocked, done, idle)
   and how it looks; what clicks, keys and hovers do; what moves.
4. **Built from:** the layout's node tree and blocks, each mod and its
   `props` roles, the shipped mods it uses, the `herdr` functions and
   commands it calls, any programs it runs, its art.
5. **Watch out:** what tripped the build up and what a fresh build would
   get wrong; limits (sizes, counts); controls that do nothing yet; what
   it assumes about the setup.
6. **Done when:** checks that prove a build works, each one something the
   build really does.

The person and what they see come first; code only in Built from and
Watch out. Exact over vague (`#ab7748`, 29px, `agent.focus`), and nothing
the build doesn't do.

## Keys: `keys.json`

On upgrade, an untouched keys file from an earlier release gets the current
shortcuts. Any edit, even whitespace, keeps the entire file as yours; new default
shortcuts must then be added by hand.

```json
{
  "global":   { "ctrl-shift-t": "tab.new", "ctrl-shift-o": "pane.split:right", "alt-1": "space.select:1" },
  "terminal": { "ctrl-shift-c": "terminal.copy", "ctrl-left": "send:\u001bb", "ctrl-1": "terminal" },
  "layout:terminals": { "ctrl-shift-pagedown": "space.next", "ctrl-shift-pageup": "space.prev" },
  "mod:editor": { "ctrl-s": "editor.save" }
}
```

- Sections: `global`, `layout:<name>` (while `layouts/<name>.json` shows),
  `terminal` (a terminal has focus), `mod:<name>`. The more specific section
  wins: `layout:terminals` over `global`, `terminal` over both.
- An action is a command (`name`, or `name:value`: the value fills the
  command's first argument), `send:<text>` (types into the terminal),
  `<mod>.<command>` (delivered to that mod's `nextCommand()`), or `terminal`
  (the program gets the key).
- Named, typed command arguments use an object instead of a string:
  `"ctrl-shift-r": {"command": "layout.resize", "args": {"id": "sidebar", "px": 240}}`.
  `command` names an app command; `args` is an object (optional, defaults to `{}`).
  It can pass several arguments, numbers, booleans, arrays or nested objects.
  String actions keep their literal first-argument shorthand, including numeric
  indexes: `space.new:{work}` names a space `{work}`, and `space.new:{}` names it `{}`.
  Invalid binding objects or rejected arguments show an error toast when pressed;
  the key does not go to the terminal program.
- Plain ctrl+letters go to the program unless bound. Programs rely on
  ctrl+c, ctrl+d, ctrl+l, ctrl+r, ctrl+w, ctrl+a, ctrl+e, ctrl+u, ctrl+k,
  ctrl+z and ctrl+[ (Escape); ctrl+alt is AltGr on Windows; option+digits
  and letters type characters on many Mac keyboards (`[`, `@`). Don't pick
  these on your own. If the person asks for one, bind it and tell them what
  it takes from typing.
- Windows AltGr and dead-key compositions type text in terminals. macOS
  dead keys compose accents through native text input; Option keeps its
  keyboard-layout characters rather than acting as Alt.
- On macOS, shifted punctuation shortcuts accept both spellings from the
  current layout: `cmd-shift-]` and `cmd-}` match the same US-layout keypress.
  Existing `keys.json` files don't need to change.
- The macOS menu bar runs the same commands and shows the key that keys.json
  binds to each (string actions only). Rebinding updates the menus live.
- Terminal keys honor the program's negotiated Kitty mode, including on Windows.
- Windows terminal Escape stays separate from the following key.
- Search: `ctrl-shift-f` (Cmd-F on macOS) opens a find bar inside the focused terminal;
  Enter/Shift-Enter moves through matches and Escape closes it while the find field
  has focus. Clicking the terminal puts typing, including Escape, back in the program;
  the bar and its matches stay.
  Each terminal keeps its own query, and reopening selects it so typing replaces it. Search covers the screen and scrollback without replacing text selection.

## Commands

The one way to change anything; keys, mods (`run(name, args)`), menus and
you use the same list. `sunna commands` prints every one with its arguments
(which are required) and what it does. Unknown argument names are errors and list
valid names; they never fall back to the current target. For example, closing a
space uses `sunna do space.close --id <id> --confirmed true`, not `--space`.
Target IDs (`id`, `pane`, `space`, `terminal`) must be strings. Other non-null
types are rejected before a current-target default applies; null means omitted.
The main ones:

- Spaces: `space.new {name?, folder?, tab?, argv?, task?, machine?, focus?}`
  (machine: another machine from `hc machine ls`; omitted: the focused space's
  machine for keys and clicks, this computer for scripts; `""`: this computer.
  The space lives there, its
  terminals start in that machine's home unless folder, and its tabs and
  splits stay there; `spaces()` gives each space's `machine`, null here),
  `space.close {id?, confirmed?}`, `space.rename {id?, name}` ("": Sunna's name again),
  `space.move {id, index}`, `space.select {index | id}`, `space.next`, `space.prev`.
  The sidebar's list: `space.status {id, status}` (todo, in_progress, in_review, done),
  `space.pin {id, pinned}`, `sidebar.view {group_by?, sort_by?, project_order?}`,
  `sidebar.filter {hide_idle?, hide_default_branch?, hide_bots?, projects?, clear?}`,
  The sidebar's menus are native (right click, or the "⋯" button; Alt+right click adds a Developer
  submenu to a space) and every item is a command: `space.read {id, read}`, `space.open_in {id, app?}`,
  `space.copy {id, what?}`, `space.details {id, name?, note?}`, `space.remove {id, confirmed?}` (a git
  worktree only, never the main checkout), `project.icon {key, file?, clear?}`, `project.hide {key, hidden}`,
  `project.open_in {key, app?}`, `project.copy {key}`, `project.remove {key, confirmed?}`,
  `project.worktree {key}`, `project.move {key, index, group?}`, `project.add {step?: menu|browse|clone|create, folders?}` (a person's Add project
  menu and dialogs; `folders: [paths]` opens a space for each, no picker, also from a script), `machine.rename {machine, name?}`, `machine.check {machine}`,
  `machine.remove {machine, confirmed?}` and `sidebar.menu {kind: space|project|machine|group, id?, key?, machine?, dev?}`.
  Their shared keys are listed above (`space.unread`, `space.notes`, `project.icons`).
  Project groups (Group by project): `group.new {key, name?, color?}` makes a group (a folder in the list) from a
  project and answers `{group}`; `group.rename {id, name}`, `group.color {id, color}` (ansi6, ansi3, ansi4, ansi5,
  ansi2 or ansi1), `group.delete {id, confirmed?}` (its projects go back to the list; no space is closed),
  `group.move {key, group?}` (a project last in a group, or out of every group with `group: ""`),
  `group.reorder {id, index}` and `group.refresh {id}` (git and icons of its projects again). `project.move
  {key, index, group?}` puts a project at a position (1 = first) of a group or of the loose projects, and makes
  the project order Manual. Lineage: `space.parent {id, parent?}` nests a space under another (`parent: ""`
  frees it; no loops) and `space.worktree {id}` opens the New worktree dialog from a space, whose
  «Create from this space» switch makes the new worktree its child. Spaces move with `space.move`, and drop on
  a state heading with `space.status`. In the list, dragging a space, a project or a group header does the same,
  and Alt+Up / Alt+Down move the row the keyboard cursor is on. Their shared keys: `sidebar.groups`,
  `sidebar.project_keys`, `space.parents`.
  `sidebar.collapse_all` (fold every heading; again opens them, ctrl-shift-h on Linux and Windows) and
  `sidebar.reveal_current` (open what folds the shown space and scroll to it). `sidebar.focus` gives the
  keyboard to the list (ctrl-shift-e, cmd-shift-e on Mac; Tab also reaches it): arrows, Home, End, PageUp and
  PageDown move a ringed cursor, Enter or Space shows the space (or folds a header, or focuses an agent), Right
  and Left unfold and fold a header, and the menu key or Shift+F10 opens the row's menu (Esc gives the keys back).
  The other commands only write the
  `shared()` keys above, so a mod can do the same. «At rest» is no agent working or blocked and nothing
  written to a tab for 30 minutes; «default branch» is main, master or trunk; the pull request's state
  needs `gh`, so grouping by PR has one group, «No pull request».
  With no `argv`, the target core chooses that machine's default shell (PowerShell 7 when available on Windows).
- Tabs: `tab.new {name?, space?, folder?, argv?, task?, focus?}` (argv: an
  agent to run, task: its first message). Agents run in your interactive
  shell (including fish), which stays open after the agent exits. A terminal
  that exits with a nonzero code within two seconds of launch shows an error
  toast with its exit code and last output lines. Or `tab.new {name?, space?, mod,
  props?, focus?}` for a tab whose only pane is a mod (a file, a board; split a shell
  in beside it like any tab), `tab.close {id?, confirmed?}` (the person's Cmd+W / Ctrl+W
  on a space's last tab closes only that tab and leaves the space with a new terminal tab;
  to close the space use `space.close` or the space's menu. From a script, closing the
  last tab still closes its space and asks first unless confirmed),
  `tab.rename {id?, name}` ("": its number again), `tab.move {id?, index | by}` (by: places left (-) or right (+), skipping settled tabs as `tab.next` does, wrapping),
  `tab.select {index | id}`, `tab.next`, `tab.prev`,
  `tab.settle {id?}`, `tab.unsettle {id?}` (index, next and prev
  skip settled tabs).
- New spaces and tabs are shown at once. `focus: false` on `space.new` or
  `tab.new` keeps the shown space and the target space's selected tab (with no
  space shown, the new space is shown), for work the person looks at later;
  such tabs go last in their space, in the order made. `focus` is a boolean.
- Panes: `pane.split {direction: "right" | "down", pane?, argv? | mod?, props?}` (a new
  terminal, or a mod pane), `pane.close {pane?, confirmed?}`,
  `pane.zoom {pane?}`, `pane.focus {pane | terminal}`, `pane.go {direction}`,
  `pane.swap {pane?, direction | target}`.
  `pane.zoom` also makes the pane its tab's focused one (the tab isn't
  selected); on the zoomed pane it unzooms and focus stays.
  `pane.go` prefers the most recently focused pane along the nearest edge in that
  direction; unvisited panes fall back to overlap, then top/left layout order.
  `pane.go` also reaches visible mods whose `mod.json` says `"focusable": true`, and
  starts from that mod while it holds the keyboard.
  `pane.swap {direction}` uses geometry only. Focus history lasts until the window closes.
  `pane.swap {target}` trades the places of `pane` (default: the focused one) and
  `target`, another pane of the same tab; give `direction` or `target`, not both.
  A swap moves panes, not focus: the focused pane stays focused in its new place.
  `pane.focus {terminal}` prefers the first visible layout terminal block showing that
  address, then its model pane. Clicking a terminal block focuses it too.
  While a layout terminal block is focused, implicit split/close/zoom/go/swap and
  `terminal.show` fail, as do `terminal.open`/`terminal.keep` with place right/down.
  They never act on the terminal's source pane. An explicit `pane` still targets that
  model pane; composition edits belong to the mod (edit its layout instead).
- Terminals: `terminal.open {argv?, folder?, place?, name?}` (`folder` sets the
  starting directory for tabs and right/down splits, with or without `argv`), `terminal.kill {terminal}`,
  `terminal.keep {terminal, place?}`, `terminal.park {terminal}`,
  `terminal.peek {terminal}` (peek over the tab, with keep-as-tab and close),
  `terminal.float` (shows or hides the floating terminal, one shell that keeps its session
  while hidden; `cmd-alt-a` on Mac, `ctrl-alt-a` elsewhere, in the default keys only when
  `keys.json` is installed fresh), `terminal.quick {name}` (types the quick command `name`
  into the focused terminal, and presses Enter if it says so), `quick.set {name, command,
  enter?, old?}` (creates or replaces one), `quick.edit {name?}` (opens its dialog),
  `quick.delete {name, confirmed?}`, `app.notify {title, body?, level?, action?, timeout?}`
  (posts a notification). Quick commands live in `shared("quick_commands")` (a list of
  `{name, command, enter}`; edit them in Settings > Terminal or the palette «Run: name»);
  the floating terminal's address is in `shared("float.terminal")`.
  `terminal.show {terminal?, pane?}`, `terminal.search` (screen and scrollback;
  find bar state stays with that terminal), `terminal.select_all {terminal?}` (selects the whole
  screen and scrollback for copying; only the alternate screen in vim and similar apps;
  Cmd+A on macOS, no default on Linux/Windows), `terminal.copy`, `terminal.paste` (copied
  files, such as Finder's or a clipboard manager's image, as if dropped; an image
  alone as `terminal.paste_image` pastes it), `terminal.paste_image` (Ctrl+V: an
  image, into a terminal on another machine: copied there, its path pasted; on
  Windows, clipboard text is pasted; otherwise Ctrl+V goes to the program, which
  reads the clipboard itself). TIFF and BMP clipboard
  images are converted to PNG before sending,
  `terminal.right_click {to: "program" | "menu", terminal?}` (where right-clicks go
  when the program takes the mouse; kept as its label `sunna.right-click=program`;
  Shift+right-click is always Sunna's menu),
  `terminal.scroll {lines | pages | to: "top" | "bottom", terminal?}` (moves Sunna's view of the
  history, not the program: negative lines or pages go up, toward history; pages may be
  fractional, one page is the terminal's rows; on the alternate screen, such as vim or less,
  there is no history, so nothing moves),
  `terminal.select {terminal?, at?: {row, col} | "cursor", mode?: "cell" | "word" | "line"}`
  (replaces the selection, at the program's cursor by default; word/line act like double/triple click),
  `terminal.adjust_selection {terminal?, direction}` (left, right, up, down, page_up, page_down,
  home, end, line_start, line_end; moves the selection's end and follows it in the view;
  fails with no selection), `terminal.clear_selection {terminal?}`.
  These default to the focused terminal, like scroll, and add no default keys or copy mode.
  `terminal.copy` copies the focused terminal's selection. `terminals()` and `focused()` expose
  `selection: {start: {row, col}, end: {row, col}} | null` when the selection changes.
  Start at the cursor with `terminal.select`; use `selection.end` as a copy-mode cursor.
  Cells are zero-based; rows count from the oldest retained history through the active screen,
  so output doesn't move an existing position unless history is pruned or reflowed.
  Selection endpoints are inclusive: start is the anchor, end is the moving end, even when reversed.
  Selection is null when absent or when no terminal view is attached.
  `terminal.pin {terminal, pinned}` (pins an agent to the top of the agent lists;
  kept on the terminal as `sunna.pinned`, so every Sunna client shows it, and
  gone when the terminal ends; read it as `agents()` `pinned`). Copy, paste, paste-image and implicit
  right-click targeting follow keyboard focus, including peek and floating terminals;
  with no terminal holding keyboard focus, they use the focused pane or block.
  Dragging a text selection past the terminal's top or bottom scrolls while held;
  releasing stops it. Programs that track the mouse still get their mouse events
  (hold Shift to select text instead). A drag stays with the terminal where it
  started, including its release outside the pane; other panes don't receive it.
  Terminals display Kitty graphics, including Unicode placeholder images used by
  programs such as yazi; partial images and scrolling follow the placeholder cells.
- Agents: `agent.next`, `agent.prev`, `agent.focus {terminal}`.
- Layout: `layout.switch {name?}`, `layout.toggle {id}`, `layout.show {id}`, `layout.hide {id}`,
  `layout.resize {id, px | weight}`, `layout.edit` (drag mode, Esc leaves), `layout.move {id, before | after}` (a Sunna layout is copied to `my-<name>` first), `layout.select {stack, child}`, `layout.set {layout}`
  (personal tree edits; on shipped layouts only click/look fields change),
  `layout.reset {name?}` (reset a shipped layout and its clicks),
  `layout.clone {name, as?}` (duplicate, then switch; default `<name>-copy`).
- App: `mod.clone {name, as?}` (duplicate; default `<name>-copy`), `mod.open {name}` (again: closes it), `overlay.close {id?}`, `app.help` (again: closes it), `app.palette` (again: closes it), `app.settings {section?}` (section: look, general, git, agents, terminal, editor, keyboard, machines, alerts, updates, privacy, mods; an unknown one opens the first), `app.keep_awake {mode?}`
  (sections: `look`, `machines`, `alerts`, `updates`, `privacy`; Machines lists, updates, renames and removes ssh machines; settings edit the same files you do).
  `app.check_updates` (only a successful check reports Up to date; a failed check clears the checked time and can be retried), `app.update_core` (a newer Sunna Core of this generation, terminals kept;
  never one a package manager owns), `app.welcome`, `app.whats_new`, `app.feedback {message?, from?}` (from `"person"` or `"agent"`; opens a review window, where the person can paste up to 3 screenshots; never sends until the person clicks Send), `app.reset`,
  `app.theme {name}`, `app.zoom_in`, `app.zoom_out`, `app.zoom_reset`,
  `app.update` (restart into a downloaded update).
  Add machine lists all named ssh-config hosts in a scrollable list. Machines
  that need a password, a new host key or Sunna Core installed show what to run
  in your own terminal; the picker updates once it is added. Other machines are
  added quietly. If a Linux server reports that programs stop at logout, its
  added card warns that agents stop when you disconnect and offers the administrator's
  `sudo loginctl enable-linger -- <user>` command with Copy. A warning mark in
  Settings > Machines and the sidebar opens that card; it clears automatically
  after the administrator enables lingering. Older cores show no warning.
  Login guidance includes Copy. A machine that needs a login shows
  `hc machine login <name>` with Copy in Settings > Machines, its sidebar group,
  the welcome screen and the failed New space toast. If a remote machine was rebuilt
  or replaced and a different core answers there, its sidebar offers Connect:
  explicitly forget the old identity and pair again with the same name and target.
  Other unreachable machines don't offer this.
- Window: `window.minimize`, `window.maximize` (or back), `window.close {confirmed?}`.
- macOS: `app.quit {confirmed?}`, `app.hide`, `app.hide_others`, `app.show_all`.
- Files (the editor, below): `file.open {path, line?, on?, space?, focus?}`, `file.goto` (Go to file; again: closes it),
  `editor.save {pane?, force?}`, `editor.preview {pane?, on?}`,
  `editor.reload {pane?, confirmed?}`, `editor.goto {line, pane?}`,
  `editor.save_close {then, panes?}`.

### The file editor

Sunna edits files itself, as Orca does: a native code editor (syntax
highlighting by extension, line numbers, folding, find and replace), one tab
per file.

- `file.open {path, line?, on?}` opens a file in its own tab, or shows the tab
  that already has it. `path` is absolute or relative to the space's folder;
  `line` (1 is the first) puts the cursor there; `on` is the machine the file is
  on (default: the shown space's machine; `""`: this computer). The Files panel
  (click or double click, by `files.open`), a Search result, a bot's SOUL.md and a
  file dropped on the window (where no terminal takes it) all run it. Folders are
  not files.
- An editor pane is a mod pane named `editor` with `props: { file, on? }`, so
  `tab.new {mod: "editor", props: {file: "/path/a.rs"}}` and
  `pane.split {direction, mod: "editor", props: {file}}` (a file beside a
  terminal) work and the layout document keeps them like any mod pane. It needs
  no folder in `mods/`. `spaces()[].tabs[].dirty` and `panes()[].file/dirty` say
  which have unsaved changes (the tab bar shows a dot).
- Keys in the `mod:editor` section of keys.json: `cmd-s` (`ctrl-s`) `editor.save`,
  `cmd-shift-v` (`ctrl-shift-v`) `editor.preview` (markdown as formatted text).
  The editor itself has find (`cmd-f`, `ctrl-f`) and replace (`cmd-alt-f` or
  `cmd-shift-f` on Mac, `ctrl-h` elsewhere), undo and redo.
- Saving never overwrites what changed on disk since the file was read: the pane
  says so with Overwrite and Reload from disk, `editor.save` fails, and `force:
  true` overwrites. A file that changes on disk while it has no unsaved edits is
  shown again by itself (checked every 2 seconds).
- Closing a tab, pane or space, `window.close` and `app.quit` with unsaved
  changes ask first; scripts must pass `confirmed: true` (they get an error
  naming the files otherwise). The dialog offers Save and close (Enter),
  Discard and Cancel; Save and close is `editor.save_close {then: {name, args},
  panes?}`: it saves each file with unsaved changes, then runs `then` as
  confirmed. If a file can't be saved (it changed on disk, a write error) nothing
  closes, that file is shown and an error toast says which.
- `cmd-p` (`ctrl-p`) is Go to file, the native command `file.goto` (again: closes it):
  fuzzy search by name over the files of the focused pane's project (`rg --files`,
  else `git ls-files`, else `find`; `.gitignore` is followed with rg and git;
  `.git`, `node_modules` and `target` are never listed; the first 60000 files, and
  the dialog says so). The files opened last come first. Enter opens the file
  with `file.open`. The tab and agent selector is now `cmd-alt-p` (`ctrl-alt-p`)
  and the palette's Go to space or tab; the palette also lists Go to file.
  A `keys.json` that still says `mod.open:goto-file` keeps working: it runs
  `file.goto` unless you have a mod folder of that name.
- The confirmation dialog (close a space, tab or pane, quit, discard unsaved
  changes, reload a file) is native too. A mod that opens an overlay named
  `confirm` with `props { title, detail, confirm_label?, command: {name, args},
  extra?: {label, command} }` still gets it (unless you keep your own `confirm`
  mod folder); Enter runs `extra` when there is one, else `command`.
- Files on another machine open read only (the local API can't write there).
  A file that is binary, not UTF-8 or over 1 MiB shows why it isn't opened. Line
  ends of Windows files (all `\r\n`) are kept on save.
- Settings > Editor (`shared()` keys `editor.font_size`, `editor.line_numbers`,
  `editor.folding`, `editor.wrap`) applies to every open file at once.

When the user runs a command without a required name (a key, a click), Sunna
asks them. **You never get a dialog**: pass every required argument, or the
command says what's missing.

From a shell:

```sh
sunna do tab.new --name "fix login" --argv '["claude"]' --task "look at the login bug"
sunna do pane.split:right
sunna ls          # spaces, their tabs and terminals (sub-agents under their parent)
sunna status      # what's on screen: layout, focus, overlays, zoom, errors from files and mods
sunna commands    # every command and its arguments
```

`sunna do` prints what the command made (`{ "space", "tab",
"terminal" }`, `{ "pane", "terminal" }`) or `{ "error": "..." }` and exits 1.

## The person: `settings.json`

What's about the user, never a look:

```json
{ "zoom": 1, "sounds": { "enabled": true } }
```

- `zoom` scales the whole app: terminals, mods, the layout's px sizes
  (0.5 to 3; the zoom keys change it). Write sizes as designed at zoom 1.
- `sounds`: `{ "enabled": true, "done": "<file>", "blocked": "<file>" }`.
  When an agent needs the person, or finishes while they aren't looking at
  it, Sunna plays a sound and (unless they're looking) posts a toast that
  goes there when clicked. `done` / `blocked` replace the bundled MP3s
  (path absolute or relative to `~/.config/sunna`). Changes apply live,
  including the Play buttons in Settings > Alerts.
- `language`: `"es"` or `"en"`. Absent: the system's language. Applies live;
  macOS menus change on restart.
- `folders`: `{ "roots": [...] }`: no effect. The folder search dialog was retired (Open folder
  now uses the system picker); the key is still read without error and ignored.
- `keep_awake`: `"off"` (default), `"auto"` (awake while an agent works) or
  `"on"`: stops the computer from sleeping. The status bar's cup changes it;
  `app.keep_awake {mode?}` too. Not on Windows.
- `resume_agents`: `true` (default). When Sunna finds an agent's terminal lost
  (reboot, core crash), it starts the agent again with its `resume` arguments
  from programs.json. `false` leaves the shell alone.
- `yolo`: `{ "enabled": true, "agents": { "claude": false } }`. Agents started by
  Sunna get the flag that drops their permission prompts (`yolo` in
  programs.json); `agents` overrides it per programs.json key. On by default.
- `core`: the Sunna Core binary to use, when it isn't where Sunna looks
  (PATH, the login shell's PATH, `~/.local/bin`, Homebrew, mise, Nix,
  cargo). Read at start.
- `worktrees`: `{ "directory": "~/Developer/worktrees" }` sets where the shipped
  New worktree dialog creates checkouts: `<directory>/<repo>/<branch>` (branch
  slashes become dashes). Default: `~/.herdr/worktrees`. Use an absolute path or
  `~` / `~/...`; Windows paths work too. The setting lives in this app's
  settings.json; the path and `~` refer to the space's machine, including remote
  spaces. Read when New worktree opens; existing checkouts are not moved, and
  Open / Remove worktree still use Git's checkout list. No mod clones needed.
  The app does not read the TUI's config.toml; set its `[worktrees] directory`
  to the same place if both should create checkouts together.
- `terminal_line_height` (default 1, 0.8..2), `terminal_cell_width`
  (default 1, 0.8..2) and `terminal_scroll_speed` (default 1, 0.1..5):
  terminal row height, column width and wheel/trackpad speed, used where the
  layout leaves them unset.
- `font`, `font_size`, `font_weight`, `ui_font`: used where the layout doesn't set its
  own. Sunna (main) leaves these unset. Bigger panels: `zoom`.
- `font_ligatures`: programming ligatures are off by default; set `true` to
  enable them in terminal fonts such as JetBrains Mono and Fira Code. The cursor
  and background changes (including selections) break ligatures so individual cells stay clear.
- `terminal_padding` (`[x, y]` px) and `terminal_opacity` (0..1): the space
  around terminal text and its background opacity. Unlike fonts, the person's
  own value wins over the layout's; absent, the layout's. `scroll_on_input`
  (default `true`): typing scrolls a terminal back to the bottom.
- `notifications`: `{ "enabled": true, "done": true, "blocked": true }`. `enabled`
  off posts no toasts; `done` / `blocked` off silence that kind of agent alert
  (toast and sound). Settings > Alerts.
- Settings > Terminal gathers the terminal keys (fonts, line height, cell
  width, scroll speed, `copy_on_select`, `clipboard_write`, `minimum_contrast`,
  `pane_scrollbars`, the three above). Settings > Editor writes `shared()` keys
  the file editor reads: `editor.font_size` (default 12, the terminal's
  font), `editor.line_numbers`, `editor.folding` (both default true) and
  `editor.wrap` (default false).
- `theme`: a theme name or `{light, dark}`; used where the layout sets none.
  The theme picker overrides any layout and stays on that layout.
- `colors`: theme color overrides (`{"accent":"#ff8800"}`), with the same
  precedence as fonts.
- `appearance`: `"light"` or `"dark"` always; absent, the system's. A layout
  with a light and a dark theme shows the matching one.
- Updates: an installed app downloads them by itself and offers Restart and
  What changed. If the app crashes or is killed with a verified newer update
  waiting, the next launch installs it before opening a window. The first
  start after an app update also updates Sunna Core
  to the latest release of the same generation, keeping terminals running.
  Package-managed cores stay with their package manager; Settings shows available releases.
  A core update error can be dismissed with its × or `overlay.close {id:"core-update-error"}`
  until the error changes or the update recovers.
  Settings → Updates keeps app and core actions together;
  its What's new button opens the waiting update's release range, or this
  version's notes, in the What's new window. Reading never clears the waiting-update dot.
  Rare spotlight releases open What’s new after restart, once; otherwise a
  quiet Updated pill lasts for that launch. A build from source never updates.
  Curated notes come from the bundled root `CHANGELOG.md`; remote notes and
  media are optional. `update-notes.json` records the last successful app
  version and presented spotlights; `update-notes-cache.md` caches remote
  notes; `update-notes-media/` caches HTTPS images (including animated WebP clips),
  referenced below its item in the changelog as `![label](https://...)`.
  Each version uses `### New` with `- **Headline.** detail` highlights and
  `### Fixed` with short bullet items. Unsectioned bullets are plain items.
  Notes expose `items` with `headline`, `detail`, `fixed`, and cached `media`
  objects (`path`, pixel `width`, pixel `height`). Highlights use matching cards:
  edge-to-edge media at its original aspect ratio, then headline and detail.
  For local release-media previews only, launch with
  `HERDR_NOTES_MEDIA_DIR=/var/tmp/notes-media`: URLs under
  `https://github.com/Jul879n/sunna/releases/download/<version>/<scene>.webp` use that directory's
  `<version>/<scene>.webp` instead of downloading. Without the variable,
  media uses the normal HTTPS cache.
  `updates().fork` is null in official builds, else `{repo, latest, url, newer, notes, error}` for the fork's
  GitHub releases (`latest` null without `error`: no releases yet); Settings > Updates shows it.
  Mods use `updates().notes` and `presentUpdateNotes([version])` only
  for spotlight entries actually shown expanded, not collapsed headlines.
  macOS releases require Apple Silicon; Intel Macs must build from source.
  Keep signed macOS bundles unchanged: don't edit their Info.plist or
  re-sign them. On launch, an installed macOS bundle creates or repairs its
  own `~/.local/bin/sunna` symlink, leaving personal files and other
  apps' links alone. Disk-image and translocated launches don't create it.
  Finder launches load the login shell's PATH, with the usual install
  folders as a fallback.
- `option_as_alt`: macOS only, `"auto"` (default), `"on"`, `"off"`, `"left"`, or `"right"`.
  Available in Settings → Keyboard.
  `"auto"` sends Option as Alt on US, US International, ABC and ABC-India;
  other layouts keep native Option text and dead keys. `"on"` enables terminal
  Alt shortcuts (including Option+E without composition); `"off"` keeps native
  Option input on every layout. `"left"` / `"right"` makes only that Option
  key act as Alt; the other types characters. Changes apply immediately, including layout
  switches. Linux and Windows are unchanged.
- `minimum_contrast`: terminal text contrast ratio. Unset, it is `1` (leaves
  program colors unchanged) on dark themes and `1.6` on light ones, where white
  text would vanish; write `1` to turn it off there. Set `1.6` on a dark theme
  to enable the same readability adjustment, or another number from `1` to `21`. Text below that ratio is
  replaced with near-white or near-black, whichever contrasts more with its
  background; this is a threshold, not a guaranteed resulting ratio.
  Box/block, Powerline and sprite glyph colors are never adjusted.
  Changes apply immediately, including text already on screen.
- `clipboard_write`: `false` blocks terminal programs from writing the local
  clipboard (default `true`, including remote terminals attached in this window).
  OSC 52 accepts base64 UTF-8 text up to 1 MiB: `c` writes the system clipboard,
  `p` writes Linux primary selection. `s` is ignored; cut buffers `0`–`7`
  are treated as the system clipboard.
  Clipboard reads (`?`) are never allowed.
- `copy_on_select`: copy a mouse selection to the clipboard on release (default
  `true`). Set `false` to leave the clipboard untouched while keeping the selection
  visible; `terminal.copy` (Cmd+C/Ctrl+Shift+C) still copies it. On Linux, terminal
  selections always set PRIMARY, independently of this setting. Middle-click
  pastes PRIMARY on Linux, does nothing on macOS, and pastes the clipboard on
  Windows (see `middle_click_paste`). Programs that capture the mouse still
  receive middle-clicks. Hold Shift while dragging in programs that capture the mouse.
- Terminal cursor, panes and behavior (Settings > Terminal; all optional, absent =
  today's behavior): `cursor_shape` (`"block"`, `"bar"` or `"underline"`; absent: the
  program's), `cursor_blink` (default `false`; solid while typing), `cursor_opacity`
  (0.1..1, default 1), `inactive_pane_opacity` (0.3..1, default 1: panes without the
  keyboard fade, only in a split tab), `divider_width` (0..16 px between split
  panes; absent: the layout's), `focus_follows_mouse` (default `false`),
  `hide_mouse_on_type` (default `false`; macOS only, other systems have no call for it),
  `confirm_close_running` (default `false`: `tab.close` / `pane.close` from a key or
  click ask first when a program or agent runs there; `confirmed: true` skips it),
  `shell` (path of the program new terminals run, e.g. `/bin/zsh`; absent: the
  login shell; only for terminals the app starts on this machine, and a layout or
  command that names its own program wins), `middle_click_paste` (absent: on except
  macOS), `trim_copy_margin` (default `false`: copying drops the indent all copied
  lines share).
- Window (Settings > Appearance): `reduce_effects` (default `false`: no shadows,
  window blur or translucent terminals), `show_app_name` (default `false`: "Sunna"
  in the macOS system title strip). Not available, by design of the toolkit:
  a tray or menu bar icon and GPU acceleration.
- `notifications` also takes `bell` (default `false`: a terminal's BEL raises a toast
  that goes there and the "needs you" sound) and `suppress_when_focused` (default
  `true`: no toast for the terminal in front; `false` shows it anyway).
- `pane_scrollbars`: `false` hides the scroll position thumb at a terminal
  pane's right edge (default `true`). It shows only when the pane has
  scrollback — never on the alternate screen (editors, pagers) — and
  drags to move through it.
- `telemetry`: `false` stops anonymous usage data and deletes what wasn't
  sent (on when absent; Settings → Privacy shows each report word for
  word, and `telemetry()` in mods). Once a day the app sends counts:
  version, OS, which known agents ran, how many spaces, tabs and
  terminals, which layouts, shipped mods and commands were used. Never
  terminal content, names of files, folders, spaces or the person's own
  mods; nothing from Sunna Core. `DO_NOT_TRACK=1` or `SUNNA_TELEMETRY=0` (`HERDR_TELEMETRY=0` still works)
  turns it off for one run. A build from source never sends.

## Programs: `programs.json`

Every pane, agent and terminal carries `program`: its name, icon and tile,
from one table, so every mod shows the same, and the app's own toasts
("Kiro finished") use the same names. An agent's own report wins;
else the program the terminal runs (an interpreter shows what it runs:
`node .../vite` is Vite, `npx vitest` is Vitest); shells are `shell`,
anything unlisted is `program` with a color of its own. The built-in table
has 124 programs (agents, editors, runtimes, tools, databases, infra);
`programs.json` here adds to it or changes entries field by field:

```json
{ "programs": {
    "lazygit": { "name": "Git", "tile": "#f05032" },
    "mytool": { "name": "My Tool", "icon": "my-icons/mytool.svg", "tint": true,
                "tile": "#334155", "color": "#ffffff", "match": ["mytool-dev"] } },
  "shells": ["xonsh"], "interpreters": ["pipx"] }
```

- `icon`: a path in the home folder. `tint: true` for a one-color glyph
  (drawn in `color` on a tile, or the text color), false for a picture.
- `match`: other process names (or agents' `app` names) for the entry.
- `run`: how to start it (`["agent"]` for Cursor), for agents people start
  from a menu: `knownAgents()` lists the entries with one, and whether this
  machine (or another) has them installed. For example, override
  `"claude": { "run": ["my-claude"] }` or add
  `"claude-alt": { "name": "Claude (alt)", "icon": "icons/claude.svg", "run": ["my-claude-alt"] }`
  inside `programs`. Use programs or shell functions, not aliases (the launcher
  does not expand them). On macOS both checks and the menu launcher load login profiles before
  running the agent and leaves an interactive login shell when it ends.
  `yolo`: flags that drop the agent's permission prompts, added to `run`
  while settings.json `yolo` allows it. `resume`: arguments added to `run` to
  pick up the agent's last conversation in the folder (`["--continue"]`);
  without it the agent isn't resumed. `knownAgents()` returns `base` (the
  plain `run`) and `yolo` too.
  Codex starts as `codex --no-daemon`: a shared Codex daemon would make later
  sessions use the first terminal's identity and stop with it.
  An explicit key still wins over another entry's `match`, so a personal
  `"python3": { "name": "…" }` keeps its name; only the generic `agent`
  fallback yields to a `match` (Cursor's process is `agent`).
  On remote Unix machines, both the installed-agent check and launcher load
  that user's login/interactive shell for sh, bash, zsh, dash, ksh and fish, so
  its PATH includes shell-configured tools. Other shells keep the sh launcher.
  Local Linux launches keep interactive non-login shells. Local and remote checks
  include functions in bash and zsh; fish checks find executables only. Installation
  checks are cached for up to a minute; changing the commands checks again.
  Invalid JSON is reported in `programs.error` (a valid file has no error).
- It follows what runs in the terminal now (lazygit, nvim), else what it
  started with.

## Mods: `mods/<name>/`

```
~/.config/sunna/mods/<name>/
  main.js       the mod (required)
  mod.json    where it first appears: { "place": "right" | "left" | "top" | "bottom" | "none", "size": 320 }
  *.js, *.svg   more modules ("./ui.js"), its own icons
```

- `"focusable": true` in `mod.json` lets `pane.go` give a shown copy the keyboard.
  It gets `"focus"` from `nextCommand()`: focus your own handle then
  (`this.handle.focus()`) and handle keys with `on_key_down`. The copy gets `"focus"`
  again after a popup over it closes. Leave with `run("pane.go", {direction})`.
  Without this opt-in, navigation and clicks stay unchanged.

- The first time Sunna sees a folder, it docks one copy per `mod.json` into
  the shown layout (`"none"`: only where a layout or another mod puts it).
  After that, the layouts decide. Saving any file reloads every copy in place
  (not files the app writes: `jsconfig.json`, `.d.ts`, hidden ones,
  `node_modules`). The old build's tasks and timers stop: a loop ends at its
  next `cx` use. A broken save keeps the old version running and shows in
  `sunna status`; its top-level code has already run, so keep side effects
  on `globalThis` out of the module's top level.
- A copy starts the first time it shows, then stays alive while its block is in
  the layout, shown or not (a hidden column, the other child of a stack, a pane
  in another tab or behind a zoom): switching back finds it as it was. Its
  timers keep running while hidden, so poll gently. Another layout starts its
  mods fresh (pane mods belong to their tab and stay); a block removed or given
  another mod or props starts over. `<mod>.<command>` reaches shown copies only.
- Images and SVGs are referenced from `~/.config/sunna`:
  `image("mods/<name>/logo.svg")` (full color), `svg("mods/<name>/icon.svg")`
  (tinted with the text color). Saving an asset under `mods/` updates every
  mod showing it, including images that were missing or invalid on first load.
- The defaults are mods too, read them as examples: `spaces`, `agents`,
  `tabs` (the tab bar) and `terminals-bar`; `win98` is a skin. Toasts, the pane header
  (`pane`, `terminals-pane`), the peek bar (`peek`), menus and help are native. Every dialog of the app is native
  and shares one look (a 420 px card, 480 px for forms, a visible ✕ at the top right, Esc,
  Tab inside the card, Enter on the primary button): confirmations (`confirm`), Edit details,
  Go to file, the text prompt (`prompt`), Add project (`add-project`), Add a machine
  (`machine-add`) and the worktree dialog (`worktree`). A mod or layout that still
  opens `openOverlay({ mod: "<name>", ... })` or `mod.open` with one of those names gets the
  native dialog with the same props, unless you keep a mod folder of that name.
  `prompt` takes `{title, value?, hint?, command: {name, args?}}` and runs the command with
  `name` set to the text. `worktree` (git worktrees as spaces: the space menu's and the +
  menu's New worktree,
  Open worktree (the repo's checkouts, filtered; one already shown is gone
  to; each linked one has a remove button, the main one never), Clean up
  worktrees (project menu, `action: "cleanup"`: the linked checkouts with
  merged / changes / agent-working evidence, merged clean ones preselected,
  removed together after a second click, never forced) and Remove
  worktree) takes props `{space, folder?, machine?, name?,
  action: "new" | "open" | "remove" | "cleanup"}` (the repo: folder, else where the
  space's first terminal is; a checkout with no open space is removed from
  its folder, closing nothing; a repo with only its main checkout offers a
  first worktree), or `{branch, tab?, argv?}` to make one at once with
  that first tab; checkouts go to
  `<worktrees.directory>/<repo>/<branch>` on the space's machine (default
  `~/.herdr/worktrees`; see settings.json above); the
  `spaces` sidebar nests worktree spaces under their repo's and shows each
  space's repo, branch (when the name does not say it), ahead/behind and a dot
  for uncommitted changes on a second line). Shipped mods
  are Sunna's; duplicate with `mod.clone` before editing.

### Check before you finish

```sh
sunna check ~/.config/sunna/mods/<name>
```

Loads the mod, renders it once, exits nonzero with the exact error. Run it
after every change. Never call a mod done without a passing check.

`init()` runs for real during a check. Import `checking` from `"herdr"` and
use `if (checking()) return;` before starting helpers or writing files.
`checking()` is true throughout a check and false in a live window.
`exec`, `hc` and `writeTextFile` still run during checks and warn when called.

### A mod

```js
import { View, div } from "gpui-kit";
import { v_flex, h_flex } from "gpui-base";
import { exec, spaces, theme, run } from "sunna";

export default class Branches extends View {
  init(_props, cx) {
    this.branch = {};
    cx.timer.every(3000, (cx) => this.refresh(cx));
    this.refresh(cx);
  }

  refresh(cx) {
    cx.spawn(async (cx) => {
      for (const p of spaces()) {
        // on: a space on another machine runs git there.
        const r = await exec("git", ["branch", "--show-current"], { cwd: p.folder, on: p.machine });
        this.branch[p.id] = r.stdout.trim();
      }
      cx.notify();
    });
  }

  render(cx) {
    const t = theme();
    return v_flex().size_full().bg(t.surface).p(10).gap(4).text_size(13).children(
      spaces().map((p) =>
        h_flex().id(`b-${p.id}`).gap(8).hover((s) => s.bg(t.raised))
          .on_click(() => run("space.select", { id: p.id }))
          .child(div().text_color(p.active ? t.foreground : t.muted).child(p.name))
          .child(div().text_color(t.dim).child(this.branch[p.id] ?? ""))));
  }
}
```

Rules:
- `main.js` default-exports a class extending `View`. `init(props, cx)` runs
  once; `render(cx)` describes the UI and runs again after `cx.notify()` and
  whenever Sunna's live data changes (spaces, agents, terminals, focus,
  theme, shared state). Don't poll those.
- Module code runs once per source version and is shared by copies, even after all
  copies close. Keep instance state and host calls in constructor/init, not module
  scope. Each copy still gets its own instance, tasks and localStorage.
- **Never keep a `cx` and use it later.** Every callback gets its own:
  `on_click((event, cx) => …)`, `cx.spawn(async (cx) => …)`,
  `cx.timer.every(ms, (cx) => …)`.
- No `setTimeout`/`setInterval` (use `cx.timer`), no top-level `await` (use
  `cx.spawn` in `init`). Anything slow goes through `exec` or `fetch`.
- Size: put `.id("root").on_size_change((size, cx) => { this.size = size; cx.notify(); })`
  on the root `div()`, `h_flex()` or `v_flex()`. It reports `{width, height}`
  in logical pixels after the first layout and whenever the size changes,
  even with the pointer elsewhere. It does not change `props()`.
- Cleanup: an optional `async close(cx)` on the exported `View` gets one final
  call when its pane, tab, layout-owned slot or overlay goes, and on app quit.
  It is not called for a hidden copy or an inactive tab. Start cleanup inside
  `close()`; don't await work started earlier. Await cleanup directly,
  for example `async close(cx) { await exec("hc", ["kill", this.browser]); }`.
  Closing stays responsive: cleanup has at most 250 ms (shared across copies
  on quit), then unfinished tasks are cancelled. On quit, use `exec(...)` and
  `run("terminal.kill", { terminal: address })`: those go out before the event
  loop stops; other commands may not run. An `exec` already started keeps
  running; don't start new long-lived work in `close`. This is best-effort,
  not a guarantee after a crash or forced termination.
- Use `theme()` colors so the mod follows the user's theme.
- **Animate natively, never with a timer.** Every redraw of a mod redraws
  the whole window, so `cx.timer.every(90, () => cx.notify())` for a
  spinner costs real battery. Use `.frames(["⠋", "⠙", …], 90)` (shows
  each string in turn as the element's text), `svg(arc).spin(900)` (turns
  an icon once every 900 ms: a ring spinner, e.g.
  `mods/workspace-chrome/spinner.svg`), or `.loop("opacity", { from:
  0.4, to: 1, period: 1200 })` (also width, height, left, top), and
  `.transition(...)` for changes: they run without your script and stop
  when not shown. Timers are for data (a clock, a fetch), and only as often
  as what they show changes.

### UI

Chained builders with GPUI's snake_case names: `div()`, `h_flex()`,
`v_flex()`; `.size_full()`, `.flex_1()`, `.gap(n)`, `.p(n)`, `.px(n)`,
`.w(n)`, `.h(n)`, `.items_center()`, `.justify_between()`,
`.overflow_y_scrollbar()`; `.child("text")`, `.text_size(n)`, `.font_semibold()`,
`.truncate()`, `.text_color(c)`; `.bg(c)`, `.border(1)`, `.border_color(c)`,
`.rounded(n)`; `.id("unique")` then `.on_click(...)`, `.on_mouse_down("right", ...)`,
`.hover((s) => s.bg(c))`, `.on_scroll_wheel(...)`, `.on_pinch(...)`; keyboard: `this.h = cx.focus_handle()`
in init, `.track_focus(this.h).on_key_down((e, cx) => …)`, `this.h.focus()`.
`ScrollHandle` from `gpui-base` scrolls a plain `overflow_*_scroll()` or
`overflow_*_scrollbar()` area from code. Create `this.scroll = ScrollHandle.new()`
in `init`, attach `.track_scroll(this.scroll)`, then call
`this.scroll.scroll_to_item(index)` to reveal a zero-based direct child with
only the movement needed. Requests run once after layout; visible children stay put.
Lazy lists: `list` and `uniform_list` come from `gpui-kit`, where `div` is;
`v_virtual_list` and `h_virtual_list` come from `gpui-base`. They need a
definite height from their container: a `flex_1()` chain with no explicit
`h(n)` anywhere lays out at zero and draws nothing. `track_scroll` works on a
plain scroll area and the virtual-list pair; `list` and `uniform_list` keep
their own position, so pair a `Scrollbar` of the same id instead.
`.on_drop((event, cx) => …)` on a plain layout element receives external files as
`event.paths` (absolute paths). The feedback form accepts dropped images with
the same limits, previews and removal as picked images.
`.id("file-row").drag_paths([absolutePath], { on: machine })` makes a plain layout
element a file/folder drag source for terminals. Omit `on` (or use null)
for local paths; remote paths retain their source machine. Ordinary clicks
still work; dropping outside a terminal cancels without pasting.
A terminal on the source machine receives quoted paths without Enter.
Local files dropped onto a remote terminal are copied there like an OS file
drop; local folders and remote paths dropped onto another machine are refused.
Paths containing control characters are rejected, including clipboard files.
`.on_hover((hovered, cx) => …)` reports pointer entry and exit on buttons
and other interactive controls too.
`.when(cond, (el) => …)`, `.children([...])`. Controls from `gpui-base`
(`Button`, `Checkbox`, `Switch`, `InputState` + `Input`) are unstyled: style
them. Text: `TextView.markdown(id, text)` renders markdown; `EditorState.new({
language, value, readonly, font_family: settings().font_family, syntax: {
keyword: theme().ansi5, ... } })` + `Editor.new(state)` is a code view (readonly)
or editor with syntax highlighting, line numbers and folding. To show a file
as a pane, don't draw it: `file.open {path}` opens the app's own editor, and
`pane.split {direction, mod: "editor", props: { file }}` puts it beside the shell.
The full surface is in `gpui-kit.d.ts` next to this file.

### The `herdr` module

All of it is in `sunna.d.ts`. In short: `tr(key, vars)` gives interface text in
the person's language and `locale()` names it.

- Live data: `spaces()` (each with its tabs, status, name, named, folder,
  machine, branch), `machines()` (name, null for this computer; layout:
  `loading`, `ready`, or `unavailable` on this connection; loading ends on
  the first read even if it fails, and offline machines are unavailable),
  `agents()` (status, app, space, tab, pinned, focused; "done" turns "idle"
  once someone looks at it on any Sunna client), `terminals()`
  (every core terminal), `focused()`, `layout()`, `theme()` and `style()` (the theme's `style` block, `radius` 6 if unset; every color by
  name: background (the terminals', cards), canvas (behind cards), chrome
  (sidebars, bars), surface, popover, raised, hover, selected, border,
  input (a stronger border), foreground, muted, dim, accent,
  accent_foreground, solid and solid_foreground (a primary button),
  selection, working, blocked, done, idle, unknown, ansi0..15; light and
  dark themes name the same ones), `settings()` (with `dark`: the
  appearance now), `keys()`, `commands()` (every app
  command, its arguments and what it does), `layouts()` (the layout names).
  `focused().agent` is the agent an agent block shows.
  `focused().space`/`tab` describe the current window context. With a layout terminal
  block focused, `pane` and `shown` are null; `terminal` and `cwd` describe that block,
  not its source pane. A terminal in a window (`openOverlay` with a `layout`) that has
  the keyboard is reported the same way; pane commands without a `pane` still act on
  the focused pane under the window. Check `focused()?.pane` before offering pane controls.
  `panes()` still lists only model panes; none is focused while a terminal block is.
  For composition navigation: `run("pane.focus", {terminal: nextAddress})`.
  Terminal copy/paste and implicit right-click routing follow the focused block.
- Floating: `placement()` (null unless this copy floats).
- Act: `run(command, args)`.
- Overlays: `openOverlay({ mod, props, kind: "dialog" | "popup" | "window", width, height, at, frame })`
  returns an id; the mod gets `props().overlay` and closes itself with
  `closeOverlay(id)`; Esc closes the top dialog or popup. `frame: "none"`
  drops Sunna's border, corners and shadow so the mod draws its own.
  `notify({ title, body, level, icon, timeout })` posts a notification, drawn
  by the layout-chosen mod (default: `toast`).
- Windows (`kind: "window"`): stay open until closed, over the layout and
  under dialogs; a press the mod doesn't stop drags it (stop it on your
  controls, like a title bar's buttons), it stays where dropped, and the one
  pressed last is on top. Sized by content unless `width`/`height`; at `at`
  or centered. `key: "name"` remembers its spot (opening that key again
  replaces it); `keep: true` with a key reopens it on the next start. A
  layout switch closes windows; kept ones come back with their layout.
  A window can hold a layout, terminals too: a terminal keeps its presses
  (selecting text), so give it a mod block to drag by, a title bar:
  `openOverlay({ kind: "window", key: "t7", width: 600, height: 400, layout: { type: "column", children: [{ type: "block", size: { px: 28 }, content: { mod: "my-titlebar" } }, { type: "block", content: { terminal: "t7" } }] } })`.
- Machine: `exec(cmd, args, { cwd, on, timeout })` (the user's real environment; use this,
  not `process.run`; `on: space.machine` runs there, omitted means local).
  `timeout` is milliseconds, defaults to 30,000 and is capped at 600,000 (10 minutes),
  locally and remotely. It rejects on timeout, if the program cannot start or the machine cannot be reached.
  `hc(args)` (`hc read <t>` gives a terminal's text),
  `home()`, `appHome()`, `playSound("done" | "blocked" | file)`.
  `knownAgents({ on })`: the agents people can start (programs.json's
  `run`), most picked first, each `installed` or not there (asked of the
  shell agents start in; a remote Unix machine's login/interactive shell when supported);
  rejects if the check fails (failed checks are not cached); start one with `argv: agent.run`.
  `pickFolder({ prompt })`: the system's folder picker (here only),
  resolves with the folder or null.
- Files: `readTextFile(path, { on })` (here, or on that machine),
  `writeTextFile(path, text, expected)` (here only), and
  `watchFolder(dir)` (here only; poll a remote space's folder): resolves with the changed paths the next time anything
  under `dir` changes. On Linux, Git-ignored directories and Git objects are skipped,
  and trees above 4096 non-ignored directories reject the watch; use a timer fallback.
  macOS and Windows watch recursively, including ignored directories.
  A returned root path means refresh everything (the pending paths are bounded).
  At most 64 folders can have pending watches at once; further folders reject.
  Idle watches can be reclaimed to make room, so reload when starting a watch.
  The returned promise also has `cancel()`: keep it and cancel when you stop
  showing the folder. Cancellation resolves with `[]` and releases this call
  without stopping other callers: `const watch = watchFolder(dir); watch.cancel();`.
  Closing a mod copy cancels its pending calls.
  Files also refreshes on its 10-second timer:
  `cx.spawn(async (cx) => { for (;;) { await watchFolder(dir); this.reload(); cx.notify(); } })`.
- Window title: `setWindowTitle(title)` sets the OS window's title, which window switchers,
  taskbars and time trackers read. The last call wins; `null` restores `Sunna`. Call it when the
  title changes, not on every render.
- App icon badge: `setBadge(text)` puts `text` on the app's Dock icon (macOS; other platforms
  ignore it), e.g. `setBadge(blocked ? String(blocked) : null)`. The last call wins; `null` or `""`
  clears it.
- `notify({ ..., action: { name, args } })`: the toast runs the command when clicked.
- This copy: `props()`, `shared(key)` / `setShared(key, value)` (every mod
  sees it; setting re-renders all), `nextCommand()` (key commands for this
  mod: `cx.spawn(async (cx) => { for (;;) { const c = await nextCommand(); …; cx.notify(); } })`).

Mods are trusted code: commands and network access (including `fetch`) are unrestricted.
Other built-ins: `localStorage` (per mod copy), `console.log`, `fetch`,
`import * as fs from "fs/promises"` (read in home, write under `~/.config/sunna`).

## Sunna Core (`hc`)

`sunna --skill` prints the manual for driving this window;
`hc --skill` prints the manual for running and coordinating programs and agents,
including a small Sunna Core skill you can install with the user's permission.
`sunna commands` lists app commands and arguments without needing an open window.

- `hc ls --json`, `hc read t3 --lines 50`, `hc run --json --cwd <dir> --label k=v -- <cmd>`,
  `hc label t3 k=v`, `hc send t3 "text"`, `hc wait t3`. `hc --help` for all.
- Other machines: `hc machine ls`; `hc run --on <machine> ...` starts a
  terminal there (`vps/t7`), `hc exec --on <machine> git status` runs one
  command there and prints exactly its output.

## Keep in mind

- A task's `cx` belongs to the copy that started it: `cx.notify()` redraws
  that copy, even when another copy woke it (a shared promise).
- One mod per idea. Don't edit `AGENTS.md` or the `.d.ts` files (the app
  rewrites them). Don't touch other mods unless the request
  needs it.

### Space cleanup and worktree commands

`cleanup.run {confirmed?}` removes the worktree checkouts of the open spaces' repos (this computer only) that
the rules of Settings > Git and spaces allow (merged, unchanged, finished bot run): never the main checkout, a
pinned space, one with a working agent or uncommitted changes. It shows the list first unless `confirmed`;
the result goes to `shared("cleanup.log")`. `worktree.new {space?}`, `worktree.open {space?}` and
`worktree.cleanup {space?}` open the native worktree dialog in that action (the project menu's items).
