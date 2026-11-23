// Installing mods, themes and layouts from GitHub (`owner/repo[@ref]`), a git URL or a folder,
// pinned to the commit that was read; and the GitHub catalog (topics herdr-mod, herdr-theme).

import * as fs from "fs/promises";
import { appHome, home, exec } from "herdr";
import { copyDir, removeDir, listDirs, listFiles, readJson } from "./fsx.js";
import { slug } from "./theme-import.js";

const fail = (key, body) => Object.assign(new Error(key), { body });

/** `owner/repo[@ref]`, a git URL, or a folder. */
export function parseSource(input) {
  const s = String(input ?? "").trim();
  const gh = s.match(/^([\w.-]+)\/([\w.-]+)(@[\w./-]+)?$/);
  if (gh) return { kind: "github", url: `https://github.com/${gh[1]}/${gh[2]}.git`, ref: gh[3] ? gh[3].slice(1) : null, label: `${gh[1]}/${gh[2]}`, name: slug(gh[2]) };
  if (/^(https?:\/\/|ssh:\/\/|git@)/.test(s) || s.endsWith(".git")) {
    const base = s.replace(/\.git$/, "").split(/[/:]/).filter(Boolean).pop() ?? "mod";
    return { kind: "git", url: s, ref: null, label: s, name: slug(base) };
  }
  if (s.startsWith("/") || s.startsWith("~/") || /^[A-Za-z]:[\\/]/.test(s)) {
    return { kind: "folder", path: s, label: s, name: slug(s.replace(/[\\/]+$/, "").split(/[\\/]/).pop()) };
  }
  throw fail("settings.mods_bad_source");
}

/** Brings the source's files to a folder: a shallow clone, or the folder itself. */
export async function fetchSource(src) {
  if (src.kind === "folder") {
    const dir = src.path.startsWith("~/") ? `${home()}/${src.path.slice(2)}` : src.path;
    if (!(await fs.exists(dir))) throw fail("settings.mods_nothing_found");
    return { dir, commit: null, staging: null };
  }
  const staging = `${appHome()}/mod-store/staging/${src.name || "src"}-${Date.now()}`;
  await fs.mkdir(`${appHome()}/mod-store/staging`, { recursive: true });
  const r = await exec("git", ["clone", "--depth", "1", ...(src.ref ? ["--branch", src.ref] : []), src.url, staging], { timeout: 120000 }).catch((e) => ({ code: 1, stderr: String(e?.message ?? e) }));
  if (r.code !== 0) throw fail("settings.mods_clone_failed", r.stderr);
  const rev = await exec("git", ["-C", staging, "rev-parse", "HEAD"]);
  return { dir: staging, commit: rev.stdout.trim(), staging };
}

const isTheme = (j) => j && typeof j.background === "string" && typeof j.foreground === "string" && typeof j.selection === "string" && Array.isArray(j.ansi) && j.ansi.length === 16;

/** What a source holds: [{ type: "mod" | "theme" | "layout", name, path }]. */
export async function scanSource(dir, hint = "") {
  const items = [];
  if (await fs.exists(`${dir}/main.js`)) items.push({ type: "mod", name: slug(hint || dir.split("/").pop()), path: dir });
  for (const name of await listDirs(`${dir}/mods`)) if (await fs.exists(`${dir}/mods/${name}/main.js`)) items.push({ type: "mod", name, path: `${dir}/mods/${name}` });
  for (const f of await listFiles(`${dir}/themes`)) {
    if (f.endsWith(".json") && isTheme(await readJson(`${dir}/themes/${f}`))) items.push({ type: "theme", name: f.slice(0, -5), path: `${dir}/themes/${f}` });
  }
  for (const f of await listFiles(`${dir}/layouts`)) if (f.endsWith(".json")) items.push({ type: "layout", name: f.slice(0, -5), path: `${dir}/layouts/${f}` });
  if (!items.length) throw fail("settings.mods_nothing_found");
  return items;
}

export const destination = (item) =>
  item.type === "mod" ? `${appHome()}/mods/${item.name}` : item.type === "theme" ? `${appHome()}/themes/${item.name}.json` : `${appHome()}/layouts/${item.name}.json`;

/** Marks the items whose destination is taken (`replace`: a mod name being updated is allowed). */
export async function withExisting(items, replace = null) {
  return Promise.all(items.map(async (i) => ({ ...i, exists: !(i.type === "mod" && i.name === replace) && (await fs.exists(destination(i))) })));
}

/** Copies the chosen items; never writes over what exists, except the mod being updated. */
export async function installItems(items, meta, replace = null) {
  for (const item of items) {
    const dest = destination(item);
    if (item.type === "mod") {
      if (item.name === replace && (await fs.exists(dest))) await removeDir(dest);
      else if (await fs.exists(dest)) throw fail("settings.mods_exists");
      await copyDir(item.path, dest);
      await fs.writeFile(`${dest}/.herdr-install.json`, JSON.stringify({ source: meta.source, ref: meta.ref ?? null, commit: meta.commit ?? null, installed_at: new Date().toISOString() }, null, 2) + "\n");
    } else {
      if (await fs.exists(dest)) throw fail("settings.mods_exists");
      await fs.mkdir(dest.slice(0, dest.lastIndexOf("/")), { recursive: true });
      await fs.writeFile(dest, await fs.readFile(item.path, "utf8"));
    }
  }
  return items.length;
}

/** The newest commit of an installed mod's source, if it isn't the installed one. */
export async function latestCommit(install) {
  let src;
  try { src = parseSource(install.ref ? `${install.source}@${install.ref}` : install.source); } catch { return null; }
  if (src.kind === "folder") return null;
  const r = await exec("git", ["ls-remote", src.url, src.ref ?? "HEAD"]);
  return r.code === 0 ? r.stdout.trim().split(/\s+/)[0] || null : null;
}

// Catalog ---------------------------------------------------------------------

const TTL = 10 * 60 * 1000;

/** Repositories with the GitHub topic `kind` ("herdr-mod" or "herdr-theme"), most starred first. */
export async function catalog(kind) {
  const key = `catalog:${kind}`;
  try {
    const hit = JSON.parse(localStorage.getItem(key) ?? "null");
    if (hit && Date.now() - hit.at < TTL) return hit.items;
  } catch {}
  const path = `search/repositories?q=topic:${kind}&sort=stars&per_page=50`;
  let data = null;
  let limited = false;
  try {
    const r = await fetch(`https://api.github.com/${path}`, { headers: { Accept: "application/vnd.github+json" } });
    if (r.status === 200) data = await r.json();
    else limited = r.status === 403;
  } catch {}
  if (!data) {
    try {
      const r = await exec("gh", ["api", path]);
      if (r.code === 0) data = JSON.parse(r.stdout);
    } catch {}
  }
  if (!data) throw fail(limited ? "settings.mods_rate_limited" : "settings.mods_catalog_failed");
  const items = (data.items ?? []).map((r) => ({ full_name: r.full_name, description: r.description ?? "", stargazers_count: r.stargazers_count ?? 0, pushed_at: r.pushed_at, html_url: r.html_url }));
  try { localStorage.setItem(key, JSON.stringify({ at: Date.now(), items })); } catch {}
  return items;
}

/** Whether a GitHub repo holds something `scanSource` can install (true/false), or null when GitHub could not say. */
export async function compatible(fullName) {
  const path = `repos/${fullName}/git/trees/HEAD?recursive=1`;
  let data = null;
  try {
    const r = await fetch(`https://api.github.com/${path}`, { headers: { Accept: "application/vnd.github+json" } });
    if (r.status === 200) data = await r.json();
  } catch {}
  if (!data) {
    try {
      const r = await exec("gh", ["api", path]);
      if (r.code === 0) data = JSON.parse(r.stdout);
    } catch {}
  }
  if (!data || !Array.isArray(data.tree)) return null;
  return data.tree.some((n) => /^(main\.js|mods\/[^/]+\/main\.js|themes\/[^/]+\.json|layouts\/[^/]+\.json)$/.test(n.path));
}

/** "3 days ago"-style age of an ISO date, in days (the card shows a short phrase). */
export function ageDays(iso, now = Date.now()) {
  const t = Date.parse(iso);
  return Number.isNaN(t) ? null : Math.max(0, Math.floor((now - t) / 86400000));
}
