// Add a project: a popup with three ways (browse a folder, clone a URL, create a new one), and the
// clone and create dialogs. Props: { step: "menu" | "clone" | "create" }.
import { View, div, svg } from "gpui-kit";
import { v_flex, h_flex, InputState, Input } from "gpui-base";
import { run, props, closeOverlay, openOverlay, theme, exec, home, pickFolder, notify, tr, style } from "herdr";
import { readdir, mkdir } from "fs/promises";
import { repoName, cloneUrl, nameProblem, parentProblem, joinPath, defaultParent } from "./rules.js";

// Radius scale: the theme's `style.radius` (6 = the original sizes).
const R = (n) => Math.round(n * style().radius / 6);

const lastLines = (text, n = 4) => String(text ?? "").trim().split("\n").slice(-n).join("\n");

export default class AddProject extends View {
  init(_props, cx) {
    this.step = props()?.step ?? "menu";
    this.busy = false; this.error = null;
    this.handle = cx.focus_handle();
    this.parent = defaultParent(home());
    const field = (placeholder, value = "") => {
      const input = InputState.new({ placeholder, value });
      input.on("change", (_e, cx) => { this.error = null; cx.notify(); });
      input.on("submit", () => this.submit(cx));
      return input;
    };
    this.url = field("https://github.com/owner/repo.git");
    this.name = field("project-name");
    this.parentInput = field("~/sunna/projects", this.parent);
    if (this.step === "clone") this.url.focus();
    if (this.step === "create") this.name.focus();
  }

  close() { closeOverlay(props().overlay); }

  // Browse: any folder, a repo or not, becomes a space.
  async browse() {
    this.close();
    const folder = await pickFolder({ prompt: tr("add-project.browse") });
    if (folder) run("space.new", { folder });
  }

  open(step, size) {
    this.close();
    openOverlay({ mod: "add-project", kind: "dialog", width: 460, height: size, props: { step } });
  }

  // The folder the dialog's parent field names: typed text, `~` as the home.
  parentPath() {
    const text = this.parentInput.value().trim();
    return text === "~" ? home() : text.startsWith("~/") ? joinPath(home(), text.slice(2)) : text;
  }

  fail(key, vars) { this.error = tr(key, vars); this.busy = false; }

  submit(cx) {
    if (this.busy) return;
    this.busy = true; this.error = null; cx.notify();
    cx.spawn(async (cx) => {
      try { await (this.step === "clone" ? this.clone() : this.create()); }
      catch (e) { this.fail("add-project.failed", { detail: lastLines(e?.message ?? e) }); }
      cx.notify();
    });
  }

  async clone() {
    const url = cloneUrl(this.url.value());
    if (!url) return this.fail("add-project.url_invalid");
    const bad = parentProblem(this.parentPath());
    if (bad) return this.fail(bad);
    const dest = joinPath(this.parentPath(), repoName(url));
    if (await exists(dest)) return this.fail("add-project.target_exists", { name: repoName(url) });
    await mkdir(this.parentPath(), { recursive: true });
    const r = await exec("git", ["clone", "--", url, dest], { timeout: 600000 });
    if (r.code !== 0) return this.fail("add-project.clone_failed", { detail: lastLines(r.stderr) });
    this.close();
    run("space.new", { folder: dest });
  }

  async create() {
    const name = this.name.value().trim();
    const bad = nameProblem(name) ?? parentProblem(this.parentPath());
    if (bad) return this.fail(bad);
    const git = await exec("git", ["--version"]).catch(() => ({ code: 1 }));
    if (git.code !== 0) return this.fail("add-project.git_missing");
    const dest = joinPath(this.parentPath(), name);
    if ((await readdir(dest).catch(() => []))?.length) return this.fail("add-project.target_not_empty", { name });
    await mkdir(dest, { recursive: true });
    const init = await exec("git", ["init"], { cwd: dest });
    if (init.code !== 0) return this.fail("add-project.init_failed", { detail: lastLines(init.stderr) });
    // The first commit needs an identity; without one the repo stays empty and says so.
    const who = await Promise.all(["user.name", "user.email"].map((k) => exec("git", ["config", k], { cwd: dest }).then((r) => r.stdout.trim(), () => "")));
    if (who.every(Boolean)) await exec("git", ["commit", "--allow-empty", "-m", "Initial commit"], { cwd: dest });
    else notify({ level: "warning", title: tr("add-project.no_identity") });
    this.close();
    run("space.new", { folder: dest });
  }

  // Render ------------------------------------------------------------------

  menuRow(t, id, icon, label, hint, action) {
    return h_flex().id(id).h(40).px(10).gap(10).items_center().flex_none().rounded(R(8)).cursor_pointer().hover((s) => s.bg(t.selected)).on_click(action)
      .child(svg(`mods/workspace-chrome/${icon}.svg`).size(16).flex_none().text_color(t.muted))
      .child(v_flex().flex_1().min_w(0)
        .child(div().text_size(12.8).child(label))
        .child(div().text_size(11).text_color(t.dim).truncate().child(hint)));
  }

  menu(t) {
    return v_flex().id("add-project").track_focus(this.handle).w_full().min_h(0).p(5).bg(t.popover).text_color(t.foreground)
      .child(div().px(10).pt(6).pb(4).text_size(11).text_color(t.dim).child(tr("add-project.title")))
      .child(this.menuRow(t, "ap-browse", "folder", tr("add-project.browse"), tr("add-project.browse_hint"), () => this.browse()))
      .child(this.menuRow(t, "ap-clone", "branch", tr("add-project.clone"), tr("add-project.clone_hint"), () => this.open("clone", 300)))
      .child(this.menuRow(t, "ap-create", "plus", tr("add-project.create"), tr("add-project.create_hint"), () => this.open("create", 300)));
  }

  button(t, id, text, primary, action) {
    return h_flex().id(id).h(32).px(14).items_center().rounded(R(8)).border_1().border_color(primary ? t.solid : t.input).bg(primary ? t.solid : t.raised)
      .text_color(primary ? t.solid_foreground : t.foreground).text_size(12.5).font_medium().cursor_pointer().hover((s) => s.opacity(0.88)).on_click(action).child(text);
  }

  input(t, state) {
    return Input.new(state).h(36).px(10).rounded(R(9)).border_1().border_color(this.error ? t.blocked : t.input).bg(t.background).text_size(12.8).text_color(t.foreground);
  }

  dialog(t, cx) {
    const clone = this.step === "clone";
    const label = (text) => div().pt(12).pb(6).text_size(11.5).text_color(t.dim).child(text);
    return v_flex().id("add-project").track_focus(this.handle).w_full().min_h(0).p(18).bg(t.popover).text_color(t.foreground)
      .on_key_down((e, cx) => { if (e.key === "escape") { this.close(); cx.stop_propagation(); } })
      .child(h_flex().items_center().child(div().flex_1().text_size(15).font_semibold().child(tr(clone ? "add-project.clone_title" : "add-project.create_title")))
        .child(div().text_size(11).text_color(t.dim).child("Esc")))
      .child(div().pt(4).text_size(11.5).text_color(t.muted).child(tr(clone ? "add-project.clone_intro" : "add-project.create_intro")))
      .child(label(tr(clone ? "add-project.url" : "add-project.name")))
      .child(this.input(t, clone ? this.url : this.name))
      .child(label(tr("add-project.parent")))
      .child(h_flex().gap(8)
        .child(div().flex_1().min_w(0).child(this.input(t, this.parentInput)))
        .child(this.button(t, "ap-pick", tr("add-project.choose"), false, () => {
          cx.spawn(async (cx) => { const p = await pickFolder({ prompt: tr("add-project.choose") }); if (p) this.parentInput.set_value(p); cx.notify(); });
        })))
      .when(Boolean(this.error), (el) => el.child(div().pt(8).text_size(11.5).line_height("16px").text_color(t.blocked).child(this.error)))
      .child(h_flex().pt(18).gap(8).justify_end()
        .child(this.button(t, "ap-cancel", tr("settings.cancel"), false, () => this.close()))
        .child(this.button(t, "ap-go", this.busy ? tr(clone ? "add-project.cloning" : "add-project.creating") : tr(clone ? "add-project.clone_go" : "add-project.create_go"), true, () => this.submit(cx))));
  }

  render(cx) {
    const t = theme();
    return this.step === "menu" ? this.menu(t) : this.dialog(t, cx);
  }
}

async function exists(path) {
  try { await readdir(path); return true; } catch { return false; }
}
