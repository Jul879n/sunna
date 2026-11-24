import { expect, test } from "bun:test";
import { readFile } from "node:fs/promises";
import { tr } from "./tr-stub";

class Element {
  nodes: any[] = [];
  calls: Record<string, any[]> = {};
  constructor() {
    const proxy: any = new Proxy(this, {
      get: (target, key: string) => key in target ? Reflect.get(target, key) : (...args: any[]) => {
        if (key === "child") target.nodes.push(args[0]);
        else if (key === "children") target.nodes.push(...args[0]);
        else if (key === "when") { if (args[0]) args[1](proxy); }
        else target.calls[key] = args;
        return proxy;
      },
    });
    return proxy;
  }
}
const el = () => new Element();
const ids = (e: any, out: string[] = []): string[] => {
  if (!e || typeof e !== "object") return out;
  if (typeof e.calls?.id?.[0] === "string") out.push(e.calls.id[0]);
  e.nodes.forEach((n: any) => ids(n, out));
  return out;
};
const byId = (e: any, id: string): any => {
  if (!e || typeof e !== "object") return null;
  if (e.calls?.id?.[0] === id) return e;
  for (const n of e.nodes) { const r = byId(n, id); if (r) return r; }
  return null;
};

const source = await readFile(new URL("../defaults/mods/search/main.js", import.meta.url), "utf8");
const calls: any[] = [];
const shared: Record<string, any> = {};
const runs: any[] = [];
let engine: (cmd: string, args: string[]) => Promise<any>;
const values: Record<string, unknown> = {
  tr, View: class {}, div: el, svg: el, v_flex: el, h_flex: el, Scrollbar: { vertical: el },
  InputState: { new: () => ({ on() {}, value: () => "", set_value() {} }) }, Input: { new: el },
  spaces: () => [{ id: "s1", active: true, folder: "/repo" }], panes: () => [], focused: () => null,
  exec: (cmd: string, args: string[], o: any) => { calls.push([cmd, args, o]); return engine(cmd, args); },
  setShared: (k: string, v: any) => { shared[k] = v; }, run: (...a: any[]) => runs.push(a),
  window: () => ({ platform: "linux" }), theme: () => new Proxy({}, { get: () => "#000000" }), style: () => ({ radius: 6 }),
};
const Search = new Function(...Object.keys(values), source.replace(/^import .*;\n/gm, "").replace("export default class Search", "return class Search"))(...Object.values(values));

const match = (path: string, line: number, text: string, start = 0, end = 3) =>
  JSON.stringify({ type: "match", data: { path: { text: path }, line_number: line, lines: { text: text + "\n" }, submatches: [{ start, end }] } });
const ok = (stdout: string) => ({ code: 0, stdout, stderr: "" });
const view = (query = "fn p") => {
  const v = new Search();
  v.init({}, {});
  v.query = query;
  return v;
};
const cx = { notify() {}, spawn: (f: any) => (cx.last = f({ notify() {} })), last: null as any };

test("ripgrep matches are grouped by file, one row each", async () => {
  calls.length = 0;
  engine = async (cmd) => cmd === "git" ? ok("/repo\n") : ok([match("src/a.rs", 3, "fn pane_frame()"), match("src/a.rs", 9, "fn pane_gap()"), JSON.stringify({ type: "summary" })].join("\n"));
  const v = view();
  v.search(cx);
  await cx.last;
  expect(v.results.groups).toHaveLength(1);
  expect(v.results.groups[0].hits).toHaveLength(2);
  const tree = v.render(cx);
  expect(ids(tree).filter((i) => i.startsWith("search-file-"))).toEqual(["search-file-src/a.rs"]);
  expect(ids(tree).filter((i) => i.startsWith("search-hit-"))).toEqual(["search-hit-src/a.rs-3", "search-hit-src/a.rs-9"]);
  const rg = calls.find(([c]) => c === "rg")!;
  expect(rg[1]).toEqual(["--json", "--max-count", "50", "--max-columns", "300", "-S", "-F", "--", "fn p", "."]);
});

test("without ripgrep it falls back to git grep", async () => {
  calls.length = 0;
  engine = async (cmd) => {
    if (cmd === "rg") throw new Error("spawn rg: not found");
    if (cmd === "git" && calls.some(([, a]) => a[0] === "grep")) return ok("src/b.rs:4:let x = fn p;\n");
    return ok("/repo\n");
  };
  const v = view();
  v.search(cx);
  await cx.last;
  expect(calls.some(([c, a]) => c === "git" && a[0] === "grep")).toBe(true);
  expect(v.results.groups[0].hits[0]).toMatchObject({ path: "src/b.rs", line: 4 });
});

test("a click on a result opens it in the editor on its line", async () => {
  engine = async (cmd) => cmd === "git" ? ok("/repo\n") : ok(match("src/a.rs", 3, "fn pane_frame()"));
  const v = view();
  v.search(cx);
  await cx.last;
  runs.length = 0;
  byId(v.render(cx), "search-hit-src/a.rs-3").calls.on_click[0]();
  expect(runs).toContainEqual(["file.open", { path: "/repo/src/a.rs", line: 3, on: "" }]);
});
