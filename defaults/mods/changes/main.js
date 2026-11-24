import { View, div } from "gpui-kit";
import { v_flex, h_flex, InputState, Input, Scrollbar, v_virtual_list } from "gpui-base";
import { spaces, panes, focused, exec, readTextFile, watchFolder, notify, theme, settings, shared, tr, style } from "herdr";

// Radius scale: the theme's `style.radius` (6 = the original sizes).
const R = (n) => Math.round(n * style().radius / 6);

function location() {
  const space = spaces().find((s) => s.active), f = focused();
  const pane = f?.pane ? panes().find(p => p.pane === f.pane) : null;
  const on = f?.pane === null ? (f.terminal?.includes("/") ? f.terminal.split("/")[0] : null) : space?.machine ?? null;
  return { cwd: pane?.cwd || f?.cwd || space?.folder, on };
}
const key = () => JSON.stringify(location());
// Reads write nothing under .git (no index refresh or lock, no fsmonitor cookie):
// the folder watch would see it, and every refresh would start the next one.
const READ = ["--no-optional-locks", "-c", "core.fsmonitor=false", "-c", "diff.autoRefreshIndex=false"];
const countLines = (text) => text ? text.split("\n").length - Number(text.endsWith("\n")) : 0;

// Text draws a tab as nothing: pad each to the next 4-column stop.
const untab = (text) => text.replace(/[^\t]*\t/g, (s) => s.slice(0, -1).padEnd((s.length - 1) - (s.length - 1) % 4 + 4));

// Numbers each diff line once, so a frame only draws the rows on screen.
function diffRows(lines) {
  let old = 0, next = 0;
  return lines.map((line) => {
    const match = /^@@ -(\d+)(?:,\d+)? \+(\d+)/.exec(line);
    if (match) { old = +match[1]; next = +match[2]; return { kind: "@", text: line }; }
    const kind = line[0];
    if (kind === "-") return { kind, number: old++, text: untab(line.slice(1)) || " " };
    if (kind === "+") return { kind, number: next++, text: untab(line.slice(1)) || " " };
    if (kind === " ") { old++; return { kind, number: next++, text: untab(line.slice(1)) || " " }; }
    return { kind: "", text: line };
  });
}

// A patch to diff rows: file headers stay, git's own header lines go.
const patchLines = (text) => text.replace(/\n$/, "").split("\n").filter((l) => !/^(index |--- |\+\+\+ |new file mode|deleted file mode|old mode|new mode|similarity |rename |Binary files)/.test(l));

// A preference Settings wrote with setShared; unset (or unreadable) means the default.
const pref = (key) => { try { return shared(key); } catch { return undefined; } };

// Settings > Git: what the list compares the working tree with. "head": uncommitted changes (the
// default); "default" or "upstream": the branch's whole work since it left the repo's default branch
// or its upstream. Names the base and returns its merge-base with HEAD, or null when there is none.
async function mergeBase(git, mode) {
  let ref = null;
  if (mode === "upstream") ref = (await git(["rev-parse", "--verify", "--quiet", "@{u}"])).code === 0 ? "@{u}" : null;
  else {
    const head = await git(["symbolic-ref", "--short", "refs/remotes/origin/HEAD"]);
    for (const name of [head.code === 0 ? head.stdout.trim() : null, "origin/main", "origin/master", "main", "master"]) {
      if (name && (await git(["rev-parse", "--verify", "--quiet", name])).code === 0) { ref = name; break; }
    }
  }
  if (!ref) return null;
  const base = await git(["merge-base", "HEAD", ref]);
  return base.code === 0 ? { ref: ref === "@{u}" ? "upstream" : ref, commit: base.stdout.trim() } : null;
}

// Settings > Git: the order of the change groups (status letters); null: one flat list.
const GROUP_ORDER = { status: ["U", "M", "A", "D"], new_first: ["U", "A", "M", "D"] };

export default class Changes extends View {
  init(_props, cx) {
    this.view = "changes"; // "changes" | "history"
    this.branch = null; this.upstream = false; this.ahead = 0; this.behind = 0; this.gitBusy = null;
    this.branches = null; this.branchFilter = ""; this.commits = []; this.commit_open = null;
    this.branchInput = InputState.new({ placeholder: tr("changes.filter_branches") });
    this.branchInput.on("change", (_e, cx) => { this.branchFilter = this.branchInput.value().toLowerCase(); cx.notify(); });
    this.files = []; this.rows = []; this.diffWidth = 0; this.message = ""; this.handle = cx.focus_handle();
    this.input = InputState.new({ placeholder: tr("changes.commit_message") });
    this.input.on("change", (_e, cx) => {
      const hadMessage = Boolean(this.message.trim());
      this.message = this.input.value();
      if (hadMessage !== Boolean(this.message.trim())) cx.notify();
    });
    this.input.on("submit", (_e, cx) => this.commit(cx));
    this.input.on("focus", (_e, cx) => { this.typing = true; cx.notify(); });
    this.input.on("blur", (_e, cx) => { this.typing = false; cx.notify(); });
    this.refresh(cx);
    cx.timer.every(10000, (cx) => this.refresh(cx));
  }

  watch(root, cx) {
    if (this.on || this.watched === root) return;
    this.watched = root;
    cx.spawn(async (cx) => {
      try {
        while (this.watched === root) {
          await watchFolder(root);
          if (this.watched === root) this.refresh(cx);
        }
      } catch { this.watched = null; }
    });
  }

  refresh(cx) {
    if (this.loading || this.busy) return;
    const { cwd, on } = location(), context = key();
    if (context !== this.context) {
      this.context = context; this.root = null; this.files = []; this.open = null;
      this.setLines([]); this.message = ""; this.input.set_value(""); this.error = null; this.note = null; this.watched = null; this.closed = false; this.limited = false;
    }
    if (!cwd) return;
    this.loading = true;
    this.baseMode = pref("changes.base") ?? "head";
    cx.spawn(async (cx) => {
      try {
        const top = await exec("git", ["rev-parse", "--show-toplevel"], { cwd, on });
        if (key() !== context) return;
        if (top.code !== 0) { this.root = null; this.files = []; this.setLines([]); this.open = null; // Only git's own "not a repository" gets the short text: a permission or PATH failure says what it is.
          const why = (top.stderr ?? "").trim().split("\n")[0];
          this.error = !why || /not a git repository/i.test(why) ? tr("changes.not_a_git_repository") : why; return; }
        const root = top.stdout.trim(), git = (args) => exec("git", [...READ, ...args], { cwd: root, on });
        const status = await git(["status", "--porcelain=v1", "-z", "--no-renames", "--untracked-files=all"]);
        if (status.code !== 0) throw new Error(status.stderr);
        const head = await git(["rev-parse", "--verify", "HEAD"]);
        const branch = await git(["branch", "--show-current"]);
        const counts = await git(["rev-list", "--left-right", "--count", "@{u}...HEAD"]);
        const stats = {};
        const base = head.code === 0 && (pref("changes.base") ?? "head") !== "head" ? await mergeBase(git, pref("changes.base")) : null;
        const against = base?.commit ?? "HEAD";
        const results = head.code === 0 ? [await git(["diff", against, "--numstat", "-z", "--no-renames"])] : [await git(["diff", "--cached", "--numstat", "-z", "--no-renames"]), await git(["diff", "--numstat", "-z", "--no-renames"])];
        for (const result of results) {
          if (result.code !== 0) throw new Error(result.stderr);
          for (const entry of result.stdout.split("\0")) {
            const [a, d, ...parts] = entry.split("\t");
            if (!parts.length) continue;
            const path = parts.join("\t"), old = stats[path] ?? { add: 0, del: 0 };
            stats[path] = { add: old.add + (Number(a) || 0), del: old.del + (Number(d) || 0), binary: a === "-" };
          }
        }
        const files = [];
        const entries = status.stdout.split("\0").filter(Boolean);
        // Against a base, the files are those that differ from it (committed or not); untracked ones come from status.
        if (base) {
          const named = await git(["diff", "--name-status", "-z", "--no-renames", against]);
          if (named.code !== 0) throw new Error(named.stderr);
          const parts = named.stdout.split("\0").filter(Boolean);
          for (let i = 0; i + 1 < parts.length; i += 2) {
            const letter = parts[i][0], path = parts[i + 1], stat = stats[path] ?? { add: 0, del: 0 };
            files.push({ path, status: letter === "D" ? "D" : letter === "A" ? "A" : letter === "U" ? "U" : "M", fresh: false, ...stat });
          }
          for (const entry of entries) if (entry.slice(0, 2) === "??" && !files.some((f) => f.path === entry.slice(3))) files.push({ path: entry.slice(3), status: "A", fresh: true, add: 0, del: 0 });
          files.splice(1000);
        }
        for (const entry of base ? [] : entries.slice(0, 1000)) {
          const xy = entry.slice(0, 2), path = entry.slice(3), fresh = xy === "??";
          const conflict = xy.includes("U") || xy === "AA" || xy === "DD";
          const s = conflict ? "U" : xy.includes("D") ? "D" : fresh || xy.includes("A") ? "A" : "M";
          const stat = stats[path] ?? { add: 0, del: 0 };
          files.push({ path, status: s, fresh, ...stat });
        }
        if (key() !== context) return;
        this.branch = branch.code === 0 ? branch.stdout.trim() || null : null;
        this.upstream = counts.code === 0;
        const [behind, ahead] = this.upstream ? counts.stdout.trim().split(/\s+/).map(Number) : [0, 0];
        this.behind = behind || 0; this.ahead = ahead || 0;
        this.root = root; this.on = on; this.hasHead = head.code === 0; this.files = files; this.diffBase = against; this.baseRef = base?.ref ?? null; this.limited = entries.length > 1000; this.error = null;
        this.watch(root, cx);
        if (this.open && !files.some((f) => f.path === this.open)) { this.open = null; this.setLines([]); }
        // The first file's diff shows until you pick another.
        if (!this.open && !this.closed && files.length && this.view === "changes") this.open = files[0].path;
        if (this.open) {
          try { await this.load(this.open, context); }
          catch { if (key() === context) this.setLines([tr("changes.cannot_preview_this_diff_too")]); }
        }
      } catch (e) { if (key() === context) this.error = String(e); }
      finally { this.loading = false; cx.notify(); }
    });
  }

  async load(path, context) {
    const file = this.files.find((f) => f.path === path), root = this.root, on = this.on;
    if (!file) return;
    let lines;
    if (file.fresh || (!this.hasHead && file.status === "A")) {
      const text = await readTextFile(`${root}/${path}`, { on }).catch(() => null);
      if (text === null) lines = [tr("changes.cannot_preview_this_file_binary")];
      else if (text.includes("\0")) lines = [tr("changes.binary_file_no_text")];
      else {
        file.add = countLines(text);
        lines = [`@@ -0,0 +1,${file.add} @@`, ...text.slice(0, 400000).replace(/\n$/, "").split("\n").slice(0, 1500).map(l => `+${l}`)];
      }
    } else {
      const r = await exec("git", [...READ, "diff", ...(this.hasHead ? [this.diffBase ?? "HEAD"] : []), "--no-ext-diff", "--no-textconv", "--no-color", "--", path], { cwd: root, on });
      if (r.code !== 0) throw new Error(r.stderr);
      const all = r.stdout.replace(/\n$/, "").split("\n"), first = all.findIndex((l) => l.startsWith("@@"));
      lines = first < 0 ? [file.binary ? tr("changes.binary_file_changed") : tr("changes.no_text_diff_available")] : all.slice(first);
    }
    if (key() === context && this.open === path) this.setLines(lines.slice(0, 1500).map(line => line.length > 4000 ? line.slice(0, 4000) + tr("changes.line_truncated") : line));
  }

  setLines(lines) {
    this.rows = diffRows(lines);
    this.diffWidth = Math.max(300, ...this.rows.map(row => (row.text.length + 1) * 7 + 50));
  }

  pick(path, cx) {
    this.open = this.open === path ? null : path; this.setLines([]); this.note = null;
    this.closed = !this.open;
    const context = this.context;
    if (this.open) cx.spawn(async (cx) => {
      try { await this.load(path, context); }
      catch (e) { if (key() === context && this.open === path) this.note = tr("changes.cannot_preview_this_diff_too_2", { e }); }
      cx.notify();
    });
    cx.notify();
  }

  commit(cx) {
    if (!this.message.trim() || !this.root || this.busy || this.loading || this.context !== key()) return;
    const root = this.root, on = this.on, context = this.context, message = this.message.trim();
    this.busy = true; this.note = null;
    cx.spawn(async (cx) => {
      try {
        for (const args of [["add", "-A"], ["commit", "-m", message]]) {
          const r = await exec("git", args, { cwd: root, on });
          if (r.code !== 0) throw new Error((r.stderr || r.stdout).trim());
        }
        if (key() === context) { this.message = ""; this.input.set_value(""); }
      } catch (e) { if (key() === context) this.note = tr("changes.commit_failed_e", { e }); }
      finally { this.busy = false; this.refresh(cx); cx.notify(); }
    });
    cx.notify();
  }

  // fetch, pull and push: one at a time, from the repository root.
  git(args, cx) {
    if (this.gitBusy || !this.root || this.context !== key()) return;
    const root = this.root, on = this.on, command = `git ${args.join(" ")}`;
    this.gitBusy = args[0]; cx.notify();
    cx.spawn(async (cx) => {
      try {
        const r = await exec("git", args, { cwd: root, on, timeout: 120000 });
        if (r.code !== 0) throw new Error((r.stderr || r.stdout).trim());
      } catch (e) {
        notify({ level: "error", title: tr("changes.git_failed", { command }), body: String(e?.message ?? e).split("\n").slice(-5).join("\n") });
      } finally { this.gitBusy = null; this.refresh(cx); cx.notify(); }
    });
  }

  push(cx) { this.git(this.upstream ? ["push"] : ["push", "-u", "origin", this.branch ?? "HEAD"], cx); }

  openBranches(cx) {
    if (!this.root) return;
    if (this.branches) { this.branches = null; cx.notify(); return; }
    this.branches = []; this.branchFilter = ""; this.branchInput.set_value("");
    const root = this.root, on = this.on;
    cx.spawn(async (cx) => {
      const r = await exec("git", [...READ, "branch", "--format=%(refname:short)", "--sort=-committerdate"], { cwd: root, on });
      if (this.branches) this.branches = r.code === 0 ? r.stdout.split("\n").filter(Boolean) : [];
      cx.notify();
    });
    cx.notify();
  }

  switchBranch(name, cx) { this.branches = null; this.git(["switch", name], cx); }

  showView(view, cx) {
    this.view = view; this.open = null; this.commit_open = null; this.setLines([]); this.note = null;
    if (view === "history") this.loadHistory(cx);
    else this.refresh(cx);
    cx.notify();
  }

  loadHistory(cx) {
    const root = this.root, on = this.on;
    if (!root) return;
    cx.spawn(async (cx) => {
      const r = await exec("git", [...READ, "log", "-n", "50", "--format=%h%x1f%s%x1f%an%x1f%ar"], { cwd: root, on });
      this.commits = r.code === 0 ? r.stdout.split("\n").filter(Boolean).map((l) => { const [hash, subject, author, ago] = l.split("\x1f"); return { hash, subject, author, ago }; }) : [];
      cx.notify();
    });
  }

  openCommit(hash, cx) {
    this.commit_open = this.commit_open === hash ? null : hash; this.setLines([]);
    if (!this.commit_open) { cx.notify(); return; }
    const root = this.root, on = this.on;
    cx.spawn(async (cx) => {
      const r = await exec("git", [...READ, "show", "--format=", "--patch", "--no-ext-diff", "--no-textconv", "--no-color", hash], { cwd: root, on });
      if (this.commit_open === hash) this.setLines(r.code === 0 ? patchLines(r.stdout).slice(0, 1500) : [r.stderr.trim()]);
      cx.notify();
    });
    cx.notify();
  }

  diffRow({ kind, number, text }, t) {
    if (kind === "@") return div().w_full().px(12).h(20).flex_none().truncate().bg(t.raised).text_color(t.dim).child(text);
    return h_flex().w_full().h(20).flex_none().gap(10).when(kind === "+", (e) => e.bg(t.ansi2 + "18")).when(kind === "-", (e) => e.bg(t.ansi1 + "18"))
      .child(div().w(30).flex_none().text_right().text_color(t.dim).child(number === undefined ? "" : String(number)))
      .child(div().flex_1().min_w(0).truncate().child(text));
  }

  // The files as the list draws them: grouped by status in the order Settings > Git picked, or flat.
  groupRows() {
    const order = GROUP_ORDER[pref("changes.groups")];
    if (!order) return this.files;
    return order.flatMap((k) => {
      const group = this.files.filter((f) => f.status === k);
      return group.length ? [{ head: k, n: group.length }, ...group] : [];
    });
  }

  gitBar(t) {
    const busy = Boolean(this.gitBusy), ready = Boolean(this.root);
    const btn = (id, label, action) => div().id(id).h(24).px(8).flex().items_center().rounded(R(6)).border_1().border_color(t.border).text_size(11)
      .bg(t.background).text_color(busy || !ready ? t.dim : t.foreground).when(!busy && ready, (e) => e.cursor_pointer().hover((s) => s.bg(t.hover))).on_click((_e, cx) => action(cx)).child(label);
    const tab = (id, label, view) => div().id(id).h(24).px(8).flex().items_center().rounded(R(6)).text_size(11).cursor_pointer()
      .bg(this.view === view ? t.selected : t.background).text_color(this.view === view ? t.foreground : t.muted).on_click((_e, cx) => this.showView(view, cx)).child(label);
    return h_flex().flex_none().h(36).px(10).gap(6).items_center().border_b(1).border_color(t.border)
      .child(div().id("git-branch").max_w(120).h(24).px(8).flex().items_center().rounded(R(6)).border_1().border_color(this.branches ? t.accent : t.border).text_size(11).truncate().cursor_pointer()
        .tooltip(tr("changes.switch_branch")).on_click((_e, cx) => this.openBranches(cx)).child(this.branch ?? (ready ? "HEAD" : "–")))
      .child(div().text_size(11).text_color(t.dim).child(this.upstream ? `↑${this.ahead} ↓${this.behind}` : ready ? tr("changes.no_upstream") : ""))
      .child(div().flex_1())
      .child(btn("git-fetch", tr("changes.fetch"), (cx) => this.git(["fetch", "--prune"], cx)))
      .child(btn("git-pull", tr("changes.pull"), (cx) => this.git(["pull", "--ff-only"], cx)))
      .child(btn("git-push", tr("changes.push"), (cx) => this.push(cx)))
      .child(div().w(6))
      .child(tab("git-tab-changes", tr("changes.tab_changes"), "changes"))
      .child(tab("git-tab-history", tr("changes.tab_history"), "history"));
  }

  branchList(t) {
    const q = this.branchFilter, list = (this.branches ?? []).filter((b) => !q || b.toLowerCase().includes(q));
    return v_flex().flex_none().max_h(200).p(8).gap(6).border_b(1).border_color(t.border).bg(t.surface)
      .child(div().h(28).px(8).flex().items_center().rounded(R(6)).border_1().border_color(t.border).on_mouse_down("left", (_e, cx) => cx.stop_propagation()).child(Input.new(this.branchInput).h_full().text_size(12).text_color(t.foreground)))
      .child(v_flex().id("git-branches").overflow_y_scroll().children(list.map((b, i) =>
        div().id(`git-b-${i}`).h(24).px(8).flex().items_center().flex_none().rounded(R(6)).text_size(12).cursor_pointer()
          .when(b === this.branch, (e) => e.text_color(t.accent)).hover((s) => s.bg(t.hover)).on_click((_e, cx) => this.switchBranch(b, cx)).child(b))));
  }

  historyList(t) {
    return div().flex_1().min_h(44).max_h(this.commits.length * 40 + 12).relative().border_b(1).border_color(t.border)
      .child(v_flex().id("git-history").absolute().inset_0().overflow_y_scroll().p(6).children(this.commits.map((c, i) =>
        v_flex().id(`git-c-${i}`).flex_none().px(8).py(4).rounded(R(7)).cursor_pointer().when(this.commit_open === c.hash, (e) => e.bg(t.selected)).hover((s) => s.bg(t.hover))
          .on_click((_e, cx) => this.openCommit(c.hash, cx))
          .child(h_flex().gap(8).child(div().text_size(11).text_color(t.dim).child(c.hash)).child(div().flex_1().min_w(0).truncate().child(c.subject)))
          .child(div().text_size(11).text_color(t.dim).child(`${c.author} · ${c.ago}`))))
        .when(!this.commits.length, (e) => e.child(div().p(8).text_color(t.dim).child(this.root ? "…" : tr("welcome.empty_changes")))));
  }

  render(cx) {
    if (key() !== this.context) this.refresh(cx);
    // The compare base changed in Settings: read the repository again.
    if ((pref("changes.base") ?? "head") !== (this.baseMode ?? "head") && this.root && !this.loading && !this.busy) this.refresh(cx);
    const t = theme(), add = this.files.reduce((n, f) => n + f.add, 0), del = this.files.reduce((n, f) => n + f.del, 0);
    const enabled = Boolean(this.message.trim() && this.root && !this.busy && !this.loading);
    return v_flex().id("changes").size_full().min_h(0).rounded(R(10)).overflow_hidden().bg(t.background).border_1().border_color(t.border).text_color(t.foreground).text_size(13).track_focus(this.handle)
      .on_key_down((e, cx) => {
        if (e.key === "escape" && this.typing) { this.typing = false; this.handle.focus(); cx.stop_propagation(); cx.notify(); }
      })
      .child(h_flex().h(42).flex_none().px(14).gap(8).items_center().border_b(1).border_color(t.border)
        .child(div().font_semibold().child(tr("changes.changes")))
        .child(div().px(5).py(1).rounded(R(4)).border_1().border_color(t.border).text_size(10).text_color(t.muted).child(tr("changes.n_files", { n: this.files.length })))
        .when(Boolean(this.baseRef), (el) => el.child(div().id("changes-base").max_w(110).truncate().text_size(10).text_color(t.dim).child(tr("changes.vs_base", { ref: this.baseRef }))))
        .child(div().flex_1()).child(div().text_size(11).text_color(t.ansi2).child(`+${add}`)).child(div().text_size(11).text_color(t.ansi1).child(`−${del}`)))
      .child(this.gitBar(t))
      .when(this.branches !== null, (el) => el.child(this.branchList(t)))
      // overflow_y_scrollbar() wraps the list, so its flex sizing never reached the panel's layout.
      .when(this.view === "history", (el) => el.child(this.historyList(t)))
      .when(this.view === "changes", (el) => el.child(div().flex_1().min_h(44).max_h(this.files.length * 28 + (this.groupRows().length - this.files.length) * 22 + 12).relative().border_b(1).border_color(t.border)
        .child(v_flex().id("changes-list").absolute().inset_0().overflow_y_scroll().p(6).children(this.groupRows().map((f) => {
        if (f.head) return div().id(`change-group-${f.head}`).h(22).flex_none().px(8).flex().items_center().text_size(10.5).font_semibold().text_color(t.dim).child(`${tr(`changes.group_${f.head}`)} · ${f.n}`);
        const parts = f.path.split("/"), name = parts.pop();
        return h_flex().id(`change-${f.path}`).h(28).flex_none().px(8).gap(8).items_center().rounded(R(7)).cursor_pointer().when(this.open === f.path, (e) => e.bg(t.selected)).hover((s) => s.bg(t.hover)).on_click((_e, cx) => this.pick(f.path, cx))
          .child(div().w(14).text_size(11).text_color(f.status === "A" ? t.ansi2 : f.status === "D" ? t.ansi1 : f.status === "U" ? t.ansi1 : t.blocked).child(f.status))
          .child(h_flex().flex_1().min_w(0).child(div().text_color(t.dim).truncate().child(parts.length ? parts.join("/") + "/" : "")).child(div().max_w(260).min_w(0).truncate().child(name)))
          .when(f.add > 0, (e) => e.child(div().text_size(11).text_color(t.ansi2).child(`+${f.add}`))).when(f.del > 0, (e) => e.child(div().text_size(11).text_color(t.ansi1).child(`−${f.del}`)));
      })).when(!this.files.length, (e) => e.child(div().p(8).text_color(t.dim).child(this.error || (this.loading ? tr("changes.loading_changes") : this.root ? tr("changes.no_uncommitted_changes") : tr("welcome.empty_changes"))))))
        .child(Scrollbar.vertical("changes-list").absolute().inset_0())))
      .when(this.limited, el => el.child(div().px(12).py(4).text_size(11).text_color(t.dim).child(tr("changes.showing_the_first_1_000_changed"))))
      .child(div().flex_1().min_h(0).relative().font_family(settings().font_family ?? "JetBrains Mono").text_size(11)
        .child(div().id("changes-diff-x").size_full().overflow_x_scrollbar()
          .child(div().w_full().min_w(this.diffWidth).h_full().child(v_virtual_list("changes-diff", this.rows.length, 20, (i) => String(i),
            (range) => this.rows.slice(range.start, range.end).map((row) => this.diffRow(row, t))).size_full())))
        .child(Scrollbar.vertical("changes-diff").viewport_from_layout().absolute().inset_0())
        .when(!this.open && this.view === "changes" && this.files.length > 0, (e) => e.child(div().absolute().top(6).px(12).text_color(t.dim).child(tr("changes.select_a_file_to_view")))))
      .when(Boolean(this.note), (e) => e.child(div().px(10).pb(8).text_size(11).text_color(t.ansi1).child(this.note)))
      .child(h_flex().flex_none().p(10).gap(8).border_t(1).border_color(t.border)
        .child(div().id("commit-message").flex_1().min_w(0).h(32).px(10).flex().items_center().rounded(R(8)).border_1().border_color(this.typing ? t.accent : t.border).cursor_text().on_mouse_down("left", (_e, cx) => cx.stop_propagation()).child(Input.new(this.input).h_full().text_size(12).text_color(t.foreground)))
        .child(div().id("commit").h(32).px(14).flex().items_center().rounded(R(8)).bg(enabled ? t.solid : t.raised).text_color(enabled ? t.solid_foreground : t.dim).text_size(12).when(enabled, (e) => e.cursor_pointer()).on_click((_e, cx) => this.commit(cx)).child(this.busy ? tr("changes.committing") : "Commit")));
  }
}
