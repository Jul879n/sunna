// Worktree dialog: { space, action: "new" | "open" | "remove" } or
// { folder, machine } for a repo without a space; optional { branch, tab, argv }.

import { View, div, svg } from "gpui-kit";
import { v_flex, h_flex, InputState, Input, Scrollbar } from "gpui-base";
import { run, props, closeOverlay, openOverlay, theme, spaces, panes, agents, exec, home, appHome, readTextFile, shared, tr } from "herdr";

// Where a space is now: its first terminal's live folder, else where it was made.
function place(s, all = panes()) {
  if (!s) return null;
  const tabs = new Set((s.tabs ?? []).map((c) => c.id));
  return all.find((p) => tabs.has(p.tab) && p.cwd)?.cwd || s.folder || null;
}

// The mod owns this preference; paths belong to the space's machine, not the app.
async function root(userHome) {
  const raw = await readTextFile(`${appHome()}/settings.json`);
  const directory = (raw === null ? {} : JSON.parse(raw))?.worktrees?.directory ?? "~/.herdr/worktrees";
  if (typeof directory !== "string" || !directory.trim()) throw new Error(tr("worktree.worktrees_directory_must_be_a_non_empty"));
  let path = directory.replace(/\\/g, "/");
  if (path === "~" || path.startsWith("~/")) path = userHome.replace(/\\/g, "/").replace(/\/$/, "") + path.slice(1);
  if (!path.startsWith("/") && !/^[A-Za-z]:\//.test(path)) throw new Error(tr("worktree.worktrees_directory_must_be_an_absolute"));
  return path.replace(/\/+$/, "");
}
// Home as ~ (git says C:/Users/x where Windows says C:\Users\x).
const tilde = (path, userHome) => {
  const [p, h] = [path, userHome].map((s) => s.replace(/\\/g, "/"));
  return p.startsWith(h) ? `~${p.slice(h.length)}` : path;
};
const slug = (branch) => {
  const normalized = branch.normalize("NFC");
  const name = normalized.replace(/[^\p{L}\p{M}\p{N}._-]+/gu, "-").replace(/^-+|-+$/g, "");
  if (name) return name;
  // Hash the original name so symbol-only branches do not share a folder.
  let hash = 2166136261;
  for (const char of normalized) hash = Math.imul(hash ^ char.codePointAt(0), 16777619);
  return `branch-${(hash >>> 0).toString(16).padStart(8, "0")}`;
};
// A preference other mods' Settings page wrote with setShared; unset (or unreadable) means the default.
const pref = (key) => { try { return shared(key); } catch { return undefined; } };

// Settings > Git: the branch a typed name becomes: the template ({name}, {date}, {repo}), then the prefix.
// A name that is already a branch is used as typed.
async function branchName(typed, repo, on) {
  const template = String(pref("worktree.template") ?? "").trim() || "{name}";
  const mode = pref("worktree.prefix_mode");
  if (template === "{name}" && (!mode || mode === "none")) return typed;
  if ((await git(repo.main, ["show-ref", "--verify", "--quiet", `refs/heads/${typed}`], on)).ok) return typed;
  const date = new Date(), pad = (n) => String(n).padStart(2, "0");
  const filled = template.replace(/\{date\}/g, `${date.getFullYear()}-${pad(date.getMonth() + 1)}-${pad(date.getDate())}`).replace(/\{repo\}/g, repo.name);
  let name = filled.includes("{name}") ? filled.replace(/\{name\}/g, typed) : `${filled}-${typed}`;
  let prefix = "";
  if (mode === "custom") prefix = String(pref("worktree.prefix") ?? "").trim();
  else if (mode === "git") {
    const user = await git(repo.top, ["config", "user.name"], on);
    prefix = user.ok ? user.out.toLowerCase().replace(/[^\p{L}\p{N}._-]+/gu, "-").replace(/^-+|-+$/g, "") : "";
  }
  if (prefix && !/[/_-]$/.test(prefix)) prefix += "/";
  return prefix && !name.startsWith(prefix) ? prefix + name : name;
}

// Settings > Git: where a new branch starts: the remote's default branch, brought up to date
// (and the local copy of it, when it is not checked out), else HEAD.
async function freshStart(repo, on) {
  const head = await git(repo.main, ["symbolic-ref", "--short", "refs/remotes/origin/HEAD"], on);
  let def = head.ok ? head.out.replace(/^origin\//, "") : "";
  if (!def) for (const name of ["main", "master"]) if ((await git(repo.main, ["rev-parse", "--verify", "--quiet", `refs/remotes/origin/${name}`], on)).ok) { def = name; break; }
  if (!def) return "HEAD";
  if (!(await git(repo.main, ["fetch", "origin", def], on, 120_000)).ok) return "HEAD";
  await git(repo.main, ["fetch", "origin", `${def}:${def}`], on, 120_000); // refused while checked out: fine
  return `origin/${def}`;
}

const ago = (s) => {
  const d = Date.now() / 1000 - s;
  return d < 3600 ? tr("worktree.age_minutes", { n: Math.max(1, Math.round(d / 60)) }) : d < 86400 ? `${Math.round(d / 3600)}h` : d < 86400 * 60 ? `${Math.round(d / 86400)}d` : tr("worktree.age_months", { n: Math.round(d / 86400 / 30) });
};

// Every checkout of a repo (`git worktree list`), the main one first, then
// the most recently committed: { path, branch, main, at }.
async function checkouts(repo, on) {
  const list = await git(repo.main, ["worktree", "list", "--porcelain"], on);
  if (!list.ok) return [];
  const refs = await git(repo.main, ["for-each-ref", "--format=%(refname:short)%09%(committerdate:unix)", "refs/heads"], on);
  const when = new Map(refs.out.split("\n").map((l) => l.split("\t")).map(([b, at]) => [b, Number(at) || 0]));
  const out = [];
  for (const block of list.out.split(/\n\n+/)) {
    const lines = block.split("\n");
    const path = lines.find((l) => l.startsWith("worktree "))?.slice(9);
    if (!path || lines.some((l) => l === "bare" || l.startsWith("prunable"))) continue;
    const branch = lines.find((l) => l.startsWith("branch "))?.slice(7).replace(/^refs\/heads\//, "") ?? null;
    out.push({ path, branch, main: out.length === 0, at: branch ? (when.get(branch) ?? 0) : 0 });
  }
  return out.sort((a, b) => b.main - a.main || b.at - a.at);
}

// The space showing a checkout: its first terminal is in it (the deepest checkout wins).
function spaceOf(path, all, on, spaceList, paneList) {
  const inside = (p, dir) => p === dir || p.startsWith(dir + "/");
  return spaceList.find((s) => (s.machine ?? null) === (on ?? null) && (() => {
    const at = place(s, paneList)?.replace(/\\/g, "/");
    if (!at || !inside(at, path)) return false;
    return !all.some((c) => c.path !== path && c.path.length > path.length && inside(at, c.path));
  })());
}

async function git(cwd, args, on, timeout) {
  const r = await exec("git", args, { cwd, on, ...(timeout === undefined ? {} : { timeout }) });
  return { ok: r.code === 0, out: (r.stdout ?? "").trim(), err: (r.stderr ?? r.stdout ?? "").trim() };
}

// The submodule repos git keeps for a checkout (nested ones too, checked out
// or not): { name, dir, pinned }. `config` is the args that read its
// .gitmodules; `pinned` is the commit its parent records ("" if none).
async function modules(gitDir, config, on, out = [], parent = "") {
  const keys = await git(gitDir, ["--git-dir", gitDir, "config", ...config, "-z", "--get-regexp", "^submodule\\..*\\.path$"], on);
  if (!keys.ok) return out; // no .gitmodules, or no submodules in it
  // -z: "key\nvalue" records ended by NUL, so names and paths may hold spaces.
  for (const record of keys.out.split("\0").filter(Boolean)) {
    const at = record.indexOf("\n");
    const name = record.slice("submodule.".length, at - ".path".length);
    const dir = `${gitDir}/modules/${name}`;
    if (!(await git(gitDir, ["--git-dir", dir, "rev-parse", "--git-dir"], on)).ok) continue; // never checked out
    const pinned = await git(gitDir, ["--git-dir", gitDir, "rev-parse", `HEAD:${record.slice(at + 1)}`], on);
    // Names are only unique within their parent: nested ones carry its name.
    out.push({ name: parent + name, dir, pinned: pinned.ok ? pinned.out : "" });
    if ((await git(gitDir, ["--git-dir", dir, "cat-file", "-e", "HEAD:.gitmodules"], on)).ok) await modules(dir, ["--blob", "HEAD:.gitmodules"], on, out, `${parent + name}/`);
  }
  return out;
}

// The worktree's submodules, and the names of those with commits that exist
// nowhere else: not on a remote, and not the commit the parent pins (git
// fetches that by hash, so it is often on no remote branch here).
async function submodules(top, on) {
  const gitDir = await git(top, ["rev-parse", "--absolute-git-dir"], on);
  if (!gitDir.ok) throw new Error(gitDir.err || tr("worktree.git_rev_parse_failed"));
  const mods = await modules(gitDir.out, ["--file", `${top}/.gitmodules`], on);
  const unpushed = [];
  for (const m of mods) {
    const local = await git(gitDir.out, ["--git-dir", m.dir, "rev-list", "-n1", "--all", "--not", "--remotes", ...(m.pinned ? [m.pinned] : [])], on);
    if (!local.ok) throw new Error(local.err || tr("worktree.couldn_t_check_submodule_name", { name: m.name }));
    if (local.out) unpushed.push(m.name);
  }
  return { mods, unpushed };
}

// The repo a folder belongs to: its main checkout and name, and whether
// the folder is itself a linked worktree. null outside git.
async function repoOf(folder, on) {
  const top = await git(folder, ["rev-parse", "--show-toplevel"], on);
  const common = await git(folder, ["rev-parse", "--path-format=absolute", "--git-common-dir"], on);
  if (!top.ok || !common.ok) return null;
  const main = common.out.endsWith("/.git") ? common.out.slice(0, -5) : common.out;
  return { top: top.out, main, name: main.split("/").pop(), linked: main !== top.out };
}

export default class Worktree extends View {
  init(_props, cx) {
    this.on = this.space()?.machine;
    this.userHome = this.on ? null : home();
    this.opening = props()?.action === "open";
    this.cleaning = props()?.action === "cleanup"; // the checkouts that can go, picked and removed together
    this.cands = undefined; this.sel = new Set(); this.armed = false;
    this.text = props()?.branch ?? "";
    this.list = undefined; // open: the checkouts, once read
    this.at = 0;
    this.input = InputState.new({ value: this.text, placeholder: this.opening ? tr("worktree.filter_worktrees") : tr("worktree.branch_name") });
    this.input.on("change", (_e, cx) => { this.text = this.input.value(); this.error = ""; this.at = 0; cx.notify(); });
    this.input.on("submit", (_e, cx) => this.go(cx));
    this.inputFocused = true;
    this.input.on("focus", (_e, cx) => { this.inputFocused = true; cx.notify(); });
    this.input.on("blur", (_e, cx) => { this.inputFocused = false; cx.notify(); });
    this.handle = cx.focus_handle();
    this.handle.focus(); // the root is mounted from the first frame: keep the keyboard here while loading
    if (props()?.action !== "remove" && !this.cleaning) this.input.focus();
    this.busy = false;
    this.error = "";
    this.repo = undefined; // undefined: looking, null: not in git
    this.dirty = false;
    this.unpushed = [];
    cx.spawn(async (cx) => {
      try {
        if (this.on) {
          let result;
          try { result = await exec("sh", ["-c", 'printf %s "$HOME"'], { on: this.on }); } catch {}
          if (!result || result.code !== 0 || !result.stdout.trim()) result = await exec("powershell.exe", ["-NoProfile", "-Command", "[Console]::Write($HOME)"], { on: this.on });
          if (result.code !== 0 || !result.stdout.trim()) throw new Error(tr("worktree.could_not_find_the_remote"));
          this.userHome = result.stdout.trim().replace(/\\/g, "/");
        }
        if (!this.cleaning && props()?.action !== "remove") {
          try { this.root = await root(this.userHome); } catch (error) { if (!this.opening) throw error; }
        }
        this.folder = props()?.folder || place(this.space()) || this.userHome;
        // The repo is set last: Enter does nothing until the warnings it consents to are known.
        const repo = await repoOf(this.folder, this.on);
        if (repo && props().action === "remove") {
          this.dirty = (await git(repo.top, ["--no-optional-locks", "-c", "core.fsmonitor=false", "status", "--porcelain"], this.on)).out !== "";
          this.unpushed = (await submodules(repo.top, this.on)).unpushed;
        }
        if (repo && (this.opening || this.cleaning)) this.list = this.outside(await checkouts(repo, this.on));
        if (repo && this.cleaning) {
          this.cands = await this.inspect(repo);
          this.sel = new Set(this.cands.filter((x) => x.merged && !x.dirty && !x.busy).map((x) => x.c.path));
        }
        this.repo = repo;
        cx.notify();
        // The list (and with it the filter input) only mounts once loading is
        // done, so the keyboard must move there now — a focus call made while
        // loading lands on nothing and the dialog stays keyboard-dead.
        this.handle.focus();
        if (props().action !== "remove" && !this.cleaning) cx.timer.after(100, () => this.input.focus());
        if (props().branch) await this.work(cx);
      } catch (error) { this.repo = null; this.error = String(error); cx.notify(); }
    });
  }

  space() {
    const p = props() ?? {};
    return spaces().find((s) => s.id === p.space) ?? (p.folder ? { folder: p.folder, machine: p.machine ?? null, name: p.name ?? "" } : undefined);
  }

  close() {
    closeOverlay(props().overlay);
  }

  go(cx) {
    if (this.busy || !this.repo) return;
    if (this.cleaning) { if (this.sel.size === 0) return; if (!this.armed) { this.armed = true; cx.notify(); return; } }
    cx.spawn(async (cx) => this.work(cx));
  }

  async work(cx) {
    if (this.busy || !this.repo) return;
    this.busy = true;
    this.error = "";
    cx.notify();
    try { this.error = props().action === "remove" ? await this.remove() : this.cleaning ? await this.cleanup() : await this.create(); }
    catch (error) { this.error = String(error); }
    this.busy = false;
    if (!this.error) this.close();
    cx.notify();
  }

  async create() {
    const typed = this.text.trim();
    if (!typed) return tr("worktree.type_a_branch_name");
    const branch = await branchName(typed, this.repo, this.on);
    // Settings > General > Folder per project: off, worktrees sit straight in the worktrees folder.
    const path = `${this.root}/${pref("worktree.nest") === false ? "" : `${this.repo.name}/`}${slug(branch)}`;
    const exists = (await git(this.repo.main, ["show-ref", "--verify", "--quiet", `refs/heads/${branch}`], this.on)).ok;
    const start = !exists && pref("worktree.fresh") === true ? await freshStart(this.repo, this.on) : "HEAD";
    const add = exists ? ["worktree", "add", path, branch] : ["worktree", "add", "-b", branch, path, start];
    const r = await git(this.repo.top, add, this.on, 600_000);
    if (!r.ok) return r.err || tr("worktree.git_worktree_add_failed");
    const p = props();
    // Leave spaces unnamed so they follow the checkout folder and nest under the repo.
    run("space.new", { folder: path, machine: this.on ?? "", tab: p.tab, argv: p.argv });
    return "";
  }

  // Settings > General > External worktrees: off, the open list keeps the main checkout and those under the worktrees folder.
  outside(list) {
    if (!this.opening || pref("worktree.external") !== false || !this.root) return list;
    return list.filter((c) => c.main || c.path.replace(/\\/g, "/").startsWith(`${this.root}/`));
  }

  shown() {
    const f = this.text.trim().toLowerCase();
    return (this.list ?? []).filter((c) => !f || `${c.branch ?? ""} ${c.path}`.toLowerCase().includes(f));
  }

  open(c) {
    if (!c) return;
    this.close();
    const s = spaceOf(c.path, this.list, this.on, spaces(), panes());
    if (s) run("space.select", { id: s.id });
    else run("space.new", { folder: c.path, machine: this.on ?? "" });
  }

  // Each linked checkout with the evidence for removing it: merged into the main one, uncommitted changes, an agent working in it.
  async inspect(repo) {
    const merged = new Set((await git(repo.main, ["branch", "--merged"], this.on)).out.split("\n").map((l) => l.replace(/^[*+]?\s+/, "").trim()).filter(Boolean));
    const spaceList = spaces(), paneList = panes(), live = agents();
    const out = [];
    for (const c of this.list ?? []) {
      if (c.main) continue;
      const dirty = (await git(c.path, ["--no-optional-locks", "-c", "core.fsmonitor=false", "status", "--porcelain"], this.on)).out !== "";
      const there = spaceOf(c.path, this.list, this.on, spaceList, paneList);
      const busy = Boolean(there && live.some((a) => a.space === there.id && a.status === "working"));
      out.push({ c, dirty, busy, merged: Boolean(c.branch && merged.has(c.branch)), space: there?.id ?? null });
    }
    return out.sort((a, b) => b.merged - a.merged || b.c.at - a.c.at);
  }

  toggle(path, cx) {
    this.sel.has(path) ? this.sel.delete(path) : this.sel.add(path);
    this.armed = false;
    cx.notify();
  }

  // Removes the picked checkouts, closing their spaces first. Git itself refuses one with changes (no --force).
  async cleanup() {
    const failed = [];
    for (const x of this.cands ?? []) {
      if (!this.sel.has(x.c.path) || x.dirty || x.busy) continue;
      if (x.space) run("space.close", { id: x.space, confirmed: true });
      const r = await git(this.repo.main, ["worktree", "remove", x.c.path], this.on, 600_000);
      if (!r.ok) failed.push(`${x.c.branch ?? x.c.path}: ${r.err || tr("worktree.git_worktree_remove_failed")}`);
    }
    return failed.join("\n");
  }

  // A checkout's own remove dialog, from its row: the same checks and warnings as from the space's menu.
  removeRow(c) {
    const there = spaceOf(c.path, this.list ?? [], this.on, spaces(), panes());
    this.close();
    openOverlay({ mod: "worktree", kind: "dialog", width: 460, height: 280, props: { action: "remove", space: there?.id, folder: c.path, machine: this.on ?? null, name: c.branch ?? c.path.split("/").pop() } });
  }

  // The same dialog's other doors: a new worktree for this repo.
  newFromList() {
    const p = props() ?? {};
    this.close();
    openOverlay({ mod: "worktree", kind: "dialog", width: 460, height: 280, props: { action: "new", space: p.space, folder: this.folder, machine: this.on ?? null } });
  }

  async remove() {
    const s = this.space();
    const status = await git(this.repo.top, ["--no-optional-locks", "-c", "core.fsmonitor=false", "status", "--porcelain"], this.on);
    if (!status.ok) return status.err || tr("worktree.git_status_failed");
    // git refuses a worktree with submodules unless forced, because their
    // history goes with it. The popup already showed what's at risk; only
    // risk that appeared since it opened gets one more warning.
    const { mods, unpushed } = await submodules(this.repo.top, this.on);
    const changed = (status.out && !this.dirty) || unpushed.some((name) => !this.unpushed.includes(name));
    this.dirty ||= status.out !== "";
    this.unpushed = [...new Set([...this.unpushed, ...unpushed])];
    if (changed) return tr("worktree.it_changed_since_this_opened");
    // Terminals first: a shell sitting in the folder keeps it busy on Windows.
    if (s?.id) run("space.close", { id: s.id, confirmed: true });
    const force = this.dirty || mods.length > 0;
    const r = await git(this.repo.main, ["worktree", "remove", ...(force ? ["--force"] : []), this.repo.top], this.on, 600_000);
    return r.ok ? "" : r.err || tr("worktree.git_worktree_remove_failed");
  }

  key(e, cx) {
    if (e.key === "escape") return this.close();
    if (this.opening) {
      const n = this.shown().length;
      if (e.key === "enter") this.open(this.shown()[this.at]);
      else if ((e.key === "down" || e.key === "up") && n) this.at = (this.at + (e.key === "down" ? 1 : n - 1)) % n;
      else return;
      cx.stop_propagation();
      return cx.notify();
    }
    if (e.key === "enter" && props()?.action === "remove") return this.go(cx);
  }

  renderOpen(t) {
    const r = this.repo;
    const rows = this.shown();
    const all = this.list ?? [];
    const spaceList = spaces();
    const paneList = panes();
    const row = (c, i) => {
      const on = i === this.at;
      const there = spaceOf(c.path, all, this.on, spaceList, paneList);
      return v_flex()
        .id(`wt-${i}`)
        .flex_none()
        .h(44)
        .px(10)
        .justify_center()
        .rounded(7)
        .cursor_pointer()
        .when(on, (el) => el.bg(t.selected))
        .when(!on, (el) => el.hover((s) => s.bg(t.hover)))
        .on_click(() => this.open(c))
        .child(
          h_flex()
            .gap(8)
            .items_center()
            .child(div().flex_1().min_w(0).truncate().text_color(t.foreground).child(c.branch ?? tr("worktree.detached")))
            .when(c.main, (el) => el.child(div().flex_none().text_size(11).text_color(t.dim).child(tr("worktree.main_checkout"))))
            .when(Boolean(there), (el) => el.child(div().flex_none().text_size(11).text_color(t.accent).child(tr("worktree.open"))))
            .when(!there && c.at > 0, (el) => el.child(div().flex_none().text_size(11).text_color(t.dim).child(ago(c.at))))
            // The main checkout cannot be removed: git refuses it, so there is no button.
            .when(!c.main, (el) => el.child(div().id(`wt-remove-${i}`).size(24).flex_none().flex().items_center().justify_center().rounded(6).text_color(t.dim)
              .hover((s) => s.bg(t.hover).text_color(t.foreground)).tooltip(tr("menu.remove_worktree"))
              .on_mouse_down("left", (_e, cx) => cx.stop_propagation())
              .on_click((_e, cx) => { cx.stop_propagation(); this.removeRow(c); })
              .child(svg("mods/workspace-chrome/x.svg").size(12)))),
        )
        .child(div().truncate().text_size(11.5).text_color(t.dim).child(tilde(c.path, this.userHome ?? "")));
    };
    const count = rows.length === all.length ? `${all.length} checkouts` : `${rows.length}/${all.length} checkouts`;
    return v_flex()
      .id("worktree")
      .track_focus(this.handle)
      .on_key_down((e, cx) => this.key(e, cx))
      .size_full()
      .p(16)
      .gap(10)
      .bg(t.popover)
      .text_size(13)
      .child(h_flex().child(div().flex_1().font_semibold().text_color(t.foreground).child((r ? tr("worktree.open_worktree_of_name", { name: r.name }) : tr("worktree.open_worktree")))).child(div().text_size(11).text_color(t.dim).child(r ? count : "")))
      .when(r === undefined || (r && this.list === undefined), (el) => el.child(div().text_color(t.muted).child("\u2026")))
      .when(r === null, (el) => el.child(div().text_color(t.muted).child(tr("worktree.folder_is_not_in_a", { folder: this.folder ? tilde(this.folder, this.userHome ?? "") : tr("worktree.this_folder") }))))
      .when(Boolean(r && this.list), (el) =>
        el
          .child(Input.new(this.input).h(32).px(10).items_center().rounded(6).border(1).border_color(this.inputFocused ? t.accent : t.input).bg(t.background).text_color(t.foreground))
          .child(
            div()
              .relative()
              .flex_1()
              .min_h(0)
              .child(
                v_flex()
                  .id("wt-list")
                  .size_full()
                  .overflow_y_scroll()
                  .children(rows.map(row))
                  .when(rows.length === 0, (el) => el.child(div().p(10).text_color(t.dim).child(tr("worktree.no_matching_worktrees"))))
                  // Only the main checkout exists: say what a worktree is for, and offer the first one.
                  .when(all.length <= 1 && this.text.trim() === "", (el) => el.child(v_flex().id("wt-empty").flex_none().gap(8).p(10).items_start()
                    .child(div().text_color(t.muted).child(tr("worktree.only_main_hint")))
                    .child(div().id("wt-empty-new").h(28).px(12).flex().items_center().rounded(7).bg(t.raised).text_color(t.foreground).cursor_pointer()
                      .hover((s) => s.bg(t.hover)).on_click(() => this.newFromList()).child(tr("menu.new_worktree"))))),
              )
              .child(Scrollbar.vertical("wt-list").absolute().inset_0()),
          ),
      )
      .child(h_flex().justify_end().text_size(11).text_color(t.dim).child(tr("worktree.choose_open")));
  }

  renderCleanup(t) {
    const r = this.repo, list = this.cands;
    const n = [...this.sel].filter((p) => list?.some((x) => x.c.path === p && !x.dirty && !x.busy)).length;
    const row = (x, i) => {
      const locked = x.dirty || x.busy, on = this.sel.has(x.c.path) && !locked;
      return h_flex().id(`wt-clean-${i}`).flex_none().h(44).px(10).gap(10).items_center().rounded(7)
        .when(!locked, (el) => el.cursor_pointer().hover((s) => s.bg(t.hover)).on_click((_e, cx) => this.toggle(x.c.path, cx)))
        .child(div().size(18).flex_none().rounded(5).border_1().border_color(locked ? t.dim : t.border).bg(on ? t.accent : t.background).flex().items_center().justify_center()
          .when(on, (el) => el.child(svg("mods/workspace-chrome/check.svg").size(12).text_color(t.accent_foreground ?? t.background))))
        .child(v_flex().flex_1().min_w(0)
          .child(div().truncate().text_color(locked ? t.dim : t.foreground).child(x.c.branch ?? tr("worktree.detached")))
          .child(div().truncate().text_size(11.5).text_color(t.dim).child(tilde(x.c.path, this.userHome ?? ""))))
        .child(div().flex_none().text_size(11).text_color(x.dirty || x.busy ? t.blocked : x.merged ? t.accent : t.dim)
          .child(tr(x.busy ? "worktree.clean_busy" : x.dirty ? "worktree.clean_dirty" : x.merged ? "worktree.clean_merged" : "worktree.clean_unmerged")));
    };
    return v_flex().id("worktree").track_focus(this.handle).on_key_down((e, cx) => this.key(e, cx)).size_full().p(16).gap(10).bg(t.popover).text_size(13)
      .child(div().font_semibold().text_color(t.foreground).child(r ? tr("worktree.clean_title", { name: r.name }) : tr("worktree.new_worktree")))
      .when(r === undefined || (r && list === undefined), (el) => el.child(div().text_color(t.muted).child("\u2026")))
      .when(r === null, (el) => el.child(div().text_color(t.muted).child(tr("worktree.folder_is_not_in_a", { folder: this.folder ? tilde(this.folder, this.userHome ?? "") : tr("worktree.this_folder") }))))
      .when(Boolean(r && list && list.length === 0), (el) => el.child(div().text_color(t.dim).child(tr("worktree.clean_none"))))
      .when(Boolean(r && list && list.length > 0), (el) => el.child(div().relative().flex_1().min_h(0)
        .child(v_flex().id("wt-clean-list").size_full().overflow_y_scroll().children((list ?? []).map(row)))
        .child(Scrollbar.vertical("wt-clean-list").absolute().inset_0())))
      .when(Boolean(this.error), (el) => el.child(div().text_color(t.blocked).child(this.error)))
      .child(h_flex().gap(8).items_center().justify_end()
        .child(div().flex_1().text_size(11).text_color(t.dim).child(tr("worktree.clean_note")))
        .child(div().id("wt-clean-go").h(28).px(12).flex().items_center().rounded(7).text_size(12).font_semibold()
          .when(n > 0, (el) => el.bg(this.armed ? t.blocked : t.raised).text_color(this.armed ? (t.accent_foreground ?? t.background) : t.foreground).cursor_pointer().on_click((_e, cx) => this.go(cx)))
          .when(n === 0, (el) => el.bg(t.raised).text_color(t.dim))
          .child(this.busy ? tr("worktree.working") : this.armed ? tr("worktree.clean_confirm", { count: n }) : tr("worktree.clean_go", { count: n }))));
  }

  render(cx) {
    const t = theme();
    if (this.cleaning) return this.renderCleanup(t);
    if (this.opening) return this.renderOpen(t);
    const remove = props()?.action === "remove";
    const s = this.space();
    const r = this.repo;
    const title = remove ? tr("worktree.remove_worktree_name", { name: s?.name ?? "" }) : r ? tr("worktree.new_worktree_of_name", { name: r.name }) : tr("worktree.new_worktree");
    const detail =
      r === undefined
        ? "…"
        : r === null
          ? tr("worktree.folder_is_not_in_a", { folder: this.folder ? tilde(this.folder, this.userHome) : tr("worktree.this_folder") })
          : remove
            ? !r.linked
              ? tr("worktree.this_is_the_repository_s_main")
              : tr("worktree.closes_the_space_and_deletes", { path: tilde(r.top, this.userHome) })
            : tr("worktree.a_new_space_on_a", { path: `${tilde(this.root, this.userHome)}/${pref("worktree.nest") === false ? "" : `${r.name}/`}` });
    const blocked = !r || (remove && !r.linked);
    return v_flex()
      .id("worktree")
      .track_focus(this.handle)
      .on_key_down((e, cx) => this.key(e, cx))
      .size_full()
      .p(16)
      .gap(10)
      .bg(t.popover)
      .text_size(13)
      .child(div().font_semibold().text_color(t.foreground).child(title))
      .child(div().text_color(t.muted).child(detail))
      .when(remove && this.dirty, (el) => el.child(div().text_color(t.blocked).child(tr("worktree.it_has_uncommitted_changes_they"))))
      .when(remove && this.unpushed.length > 0, (el) => el.child(div().text_color(t.blocked).child(tr("worktree.submodule_name_has_commits_that", { name: this.unpushed.join(", ") }))))
      .when(!remove && !blocked, (el) =>
        el.child(
          Input.new(this.input)
            .h(32)
            .px(10)
            .items_center()
            .rounded(6)
            .border(1)
            .border_color(this.inputFocused ? t.accent : t.input)
            .bg(t.background)
            .text_color(t.foreground),
        ),
      )
      .when(Boolean(this.error), (el) => el.child(div().text_color(t.blocked).child(this.error)))
      .child(div().flex_1())
      .child(
        h_flex()
          .justify_end()
          .text_size(11)
          .text_color(t.dim)
          .gap(6)
          .when(this.busy, (el) => el.child(div().frames(["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"], 90)))
          .child(this.busy ? tr("worktree.working") : blocked ? tr("worktree.esc_close") : remove ? tr("worktree.remove_esc_cancel") : tr("worktree.create_esc_cancel")),
      );
  }
}
