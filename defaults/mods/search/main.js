import { View, div, svg } from "gpui-kit";
import { v_flex, h_flex, InputState, Input, Scrollbar } from "gpui-base";
import { spaces, panes, focused, exec, setShared, run, window, theme, tr, style } from "herdr";

// Radius scale: the theme's `style.radius` (6 = the original sizes).
const R = (n) => Math.round(n * style().radius / 6);

// The folder the focused pane is in (the same rule as the Files and Changes panels).
function location() {
  const space = spaces().find((s) => s.active), f = focused();
  const pane = f?.pane ? panes().find(p => p.pane === f.pane) : null;
  const on = f?.pane === null ? (f.terminal?.includes("/") ? f.terminal.split("/")[0] : null) : space?.machine ?? null;
  return { cwd: pane?.cwd || f?.cwd || space?.folder, on };
}
const key = () => JSON.stringify(location());

const MAX_HITS = 500;
const missing = (r) => r.code === 127 || /not found|no such file|ENOENT/i.test(r.stderr ?? "");

// One matched line: its text split around the match, when the offsets are known.
function split(text, start, end) {
  if (start === undefined) return [text, "", ""];
  if (/^[\x00-\x7f]*$/.test(text)) return [text.slice(0, start), text.slice(start, end), text.slice(end)];
  if (typeof TextEncoder === "undefined") return [text, "", ""];
  const bytes = new TextEncoder().encode(text), dec = new TextDecoder();
  return [dec.decode(bytes.slice(0, start)), dec.decode(bytes.slice(start, end)), dec.decode(bytes.slice(end))];
}

/** ripgrep --json output to hits. */
function parseRg(stdout) {
  const hits = [];
  for (const line of stdout.split("\n")) {
    if (!line) continue;
    let o;
    try { o = JSON.parse(line); } catch { continue; }
    if (o.type !== "match") continue;
    const text = (o.data.lines?.text ?? "").replace(/\r?\n$/, ""), sub = o.data.submatches?.[0];
    hits.push({ path: o.data.path.text, line: o.data.line_number, text, start: sub?.start, end: sub?.end });
  }
  return hits;
}

/** `path:line:text` (git grep, grep) to hits. */
function parseGrep(stdout, query, regex) {
  const hits = [];
  for (const row of stdout.split("\n")) {
    const m = /^(.*?):(\d+):(.*)$/.exec(row);
    if (!m) continue;
    const text = m[3], at = regex ? -1 : text.toLowerCase().indexOf(query.toLowerCase());
    hits.push({ path: m[1].replace(/^\.\//, ""), line: Number(m[2]), text, start: at >= 0 ? at : undefined, end: at >= 0 ? at + query.length : undefined });
  }
  return hits;
}

/** Hits grouped by file, in order, capped. */
function group(hits) {
  const groups = [], index = new Map();
  let count = 0;
  for (const h of hits.slice(0, MAX_HITS)) {
    let g = index.get(h.path);
    if (!g) { g = { path: h.path, hits: [] }; index.set(h.path, g); groups.push(g); }
    g.hits.push(h);
    count++;
  }
  return { groups, count, files: groups.length, truncated: hits.length > MAX_HITS };
}

export default class Search extends View {
  init(_props, cx) {
    this.query = ""; this.glob = "";
    this.caseSensitive = false; this.regex = false; this.word = false;
    this.results = null; this.error = null; this.running = false; this.root = null;
    this.input = InputState.new({ placeholder: tr("search.placeholder") });
    this.input.on("change", () => { this.query = this.input.value(); });
    this.input.on("submit", (_e, cx) => this.search(cx));
    this.globInput = InputState.new({ placeholder: tr("search.glob") });
    this.globInput.on("change", () => { this.glob = this.globInput.value().trim(); });
    this.globInput.on("submit", (_e, cx) => this.search(cx));
    this.context = key();
  }

  async findRoot(cwd, on) {
    try {
      const top = await exec("git", ["rev-parse", "--show-toplevel"], { cwd, on });
      if (top.code === 0) return { root: top.stdout.trim(), repo: true };
    } catch {}
    return { root: cwd, repo: false };
  }

  /** Runs the search; ripgrep first, then git grep inside a repository, then grep. */
  async run(q, root, repo, on) {
    const opts = { cwd: root, on, timeout: 30000 };
    const { caseSensitive, regex, word, glob } = this;
    try {
      const r = await exec("rg", ["--json", "--max-count", "50", "--max-columns", "300", caseSensitive ? "-s" : "-S", ...(regex ? [] : ["-F"]), ...(word ? ["-w"] : []), ...(glob ? ["-g", glob] : []), "--", q, "."], opts);
      if (!missing(r)) {
        if (r.code > 1) throw new Error(r.stderr.trim() || "rg failed");
        return parseRg(r.stdout);
      }
    } catch (e) {
      if (!/not found|no such file|ENOENT|spawn|start|find/i.test(String(e?.message ?? e))) throw e;
    }
    const flags = [...(caseSensitive ? [] : ["-i"]), regex ? "-E" : "-F", ...(word ? ["-w"] : [])];
    if (repo) {
      const r = await exec("git", ["grep", "-n", "-I", "--full-name", ...flags, "-e", q, ...(glob ? ["--", glob] : [])], opts);
      if (r.code > 1) throw new Error(r.stderr.trim() || "git grep failed");
      return parseGrep(r.stdout, q, regex);
    }
    if (window()?.platform === "windows") throw new Error(tr("search.no_engine"));
    const r = await exec("grep", ["-rnI", "--exclude-dir=.git", "--exclude-dir=node_modules", "--exclude-dir=target", "--exclude-dir=dist", ...flags, "-e", q, "."], opts);
    if (r.code > 1) throw new Error(r.stderr.trim() || "grep failed");
    return parseGrep(r.stdout, q, regex);
  }

  search(cx) {
    const q = this.query;
    if (!q || this.running) return;
    const { cwd, on } = location();
    if (!cwd) { this.error = tr("search.no_folder"); cx.notify(); return; }
    this.running = true; this.error = null;
    cx.notify();
    cx.spawn(async (cx) => {
      try {
        const { root, repo } = await this.findRoot(cwd, on);
        this.root = root;
        this.results = group(await this.run(q, root, repo, on));
      } catch (e) {
        this.results = null; this.error = String(e?.message ?? e);
      }
      this.running = false;
      cx.notify();
    });
  }

  open(hit) {
    setShared("files.reveal", { path: `${this.root}/${hit.path}`, line: hit.line, at: Date.now() });
    run("layout.select", { stack: "panel-body", child: "files" });
  }

  toggle(t, id, label, on, flip) {
    return div().id(id).h(26).min_w(26).px(6).flex().items_center().justify_center().rounded(R(6)).cursor_pointer().text_size(12)
      .border_1().border_color(on ? t.accent : t.border).bg(on ? t.selected : t.background).text_color(on ? t.foreground : t.muted)
      .hover((s) => s.bg(t.hover)).on_click((_e, cx) => { flip(); this.search(cx); cx.notify(); }).child(label);
  }

  render(cx) {
    const t = theme(), r = this.results;
    if (key() !== this.context) { this.context = key(); this.results = null; this.error = null; }
    const field = (state, id) => div().id(id).flex_1().min_w(0).h(30).px(10).flex().items_center().rounded(R(8)).border_1().border_color(t.border)
      .on_mouse_down("left", (_e, cx) => cx.stop_propagation()).child(Input.new(state).h_full().text_size(12).text_color(t.foreground));
    return v_flex().id("search").size_full().min_h(0).rounded(R(10)).overflow_hidden().bg(t.background).border_1().border_color(t.border).text_color(t.foreground).text_size(13)
      .child(h_flex().h(42).flex_none().px(14).gap(8).items_center().border_b(1).border_color(t.border)
        .child(div().font_semibold().child(tr("tabs.search")))
        .child(div().flex_1())
        .when(r, (el) => el.child(div().text_size(11).text_color(t.dim).child(tr("search.results", { count: r.count, files: r.files }))))
        .child(div().id("panel-close").size(24).flex_none().flex().items_center().justify_center().rounded(R(6)).text_color(t.muted).cursor_pointer().hover((s) => s.bg(t.hover)).tooltip(tr("menu.close")).on_click(() => run("layout.hide", { id: "panel" })).child(svg("mods/pane/close.svg").size(13))))
      .child(v_flex().flex_none().p(10).gap(8).border_b(1).border_color(t.border)
        .child(h_flex().gap(6).items_center().child(field(this.input, "search-query"))
          .child(this.toggle(t, "search-case", "Aa", this.caseSensitive, () => { this.caseSensitive = !this.caseSensitive; }))
          .child(this.toggle(t, "search-regex", ".*", this.regex, () => { this.regex = !this.regex; }))
          .child(this.toggle(t, "search-word", "W", this.word, () => { this.word = !this.word; })))
        .child(field(this.globInput, "search-glob")))
      .child(div().flex_1().min_h(0).relative()
        .child(v_flex().id("search-list").absolute().inset_0().overflow_y_scroll().p(6)
          .when(this.running, (el) => el.child(div().p(8).text_color(t.dim).child("…")))
          .when(Boolean(this.error), (el) => el.child(div().p(8).text_color(t.blocked).child(this.error)))
          .when(Boolean(r) && r.count === 0 && !this.running, (el) => el.child(div().p(8).text_color(t.dim).child(tr("search.empty"))))
          .children((r?.groups ?? []).map((g) => {
            const parts = g.path.split("/"), name = parts.pop();
            return v_flex().id(`search-file-${g.path}`).w_full().pb(6)
              .child(h_flex().h(24).px(6).gap(6).items_center().text_size(12)
                .child(div().font_semibold().child(name)).child(div().min_w(0).truncate().text_color(t.dim).child(parts.join("/"))))
              .children(g.hits.map((h) => {
                const [a, m, b] = split(h.text.trimStart(), h.start === undefined ? undefined : Math.max(0, h.start - (h.text.length - h.text.trimStart().length)), h.end === undefined ? undefined : Math.max(0, h.end - (h.text.length - h.text.trimStart().length)));
                return h_flex().id(`search-hit-${g.path}-${h.line}`).h(22).px(6).gap(8).items_center().rounded(R(6)).cursor_pointer().hover((s) => s.bg(t.hover))
                  .on_click(() => this.open(h))
                  .child(div().w(34).flex_none().text_size(11).text_color(t.dim).child(String(h.line)))
                  .child(h_flex().flex_1().min_w(0).overflow_hidden().text_size(12)
                    .child(div().flex_none().text_color(t.muted).child(a)).child(div().flex_none().font_semibold().text_color(t.accent).child(m)).child(div().flex_none().text_color(t.muted).child(b)));
              }));
          })))
        .child(Scrollbar.vertical("search-list").absolute().inset_0()));
  }
}
