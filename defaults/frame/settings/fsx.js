// Folder helpers over fs/promises (it has no recursive copy or remove).
import * as fs from "fs/promises";

export async function copyDir(from, to) {
  await fs.mkdir(to, { recursive: true });
  for (const e of await fs.readdir(from, { withFileTypes: true })) {
    if (e.name === ".git") continue; // a clone's history is not part of what is installed
    const a = `${from}/${e.name}`;
    const b = `${to}/${e.name}`;
    if (e.isDirectory()) await copyDir(a, b);
    else await fs.writeFile(b, await fs.readFile(a));
  }
}

export async function removeDir(dir) {
  for (const e of await fs.readdir(dir, { withFileTypes: true })) {
    const p = `${dir}/${e.name}`;
    if (e.isDirectory()) await removeDir(p);
    else await fs.unlink(p);
  }
  await fs.rmdir(dir);
}

/** Moves a folder; never writes over `to`. */
export async function moveDir(from, to) {
  if (await fs.exists(to)) throw new Error("settings.mods_name_taken");
  await copyDir(from, to);
  await removeDir(from);
}

/** Names of the folders inside `dir` (none, if it doesn't exist), hidden ones left out. */
export async function listDirs(dir) {
  try {
    return (await fs.readdir(dir, { withFileTypes: true })).filter((e) => e.isDirectory() && !e.name.startsWith(".")).map((e) => e.name).sort();
  } catch {
    return [];
  }
}

async function jsFiles(dir) {
  const out = [];
  let entries = [];
  try { entries = await fs.readdir(dir, { withFileTypes: true }); } catch { return out; }
  for (const e of entries) {
    const p = `${dir}/${e.name}`;
    if (e.isDirectory()) out.push(...(await jsFiles(p)));
    else if (e.name.endsWith(".js")) out.push(p);
  }
  return out;
}

/** What a mod's code can do, read from its source: it's trusted code, so say so before installing. */
export async function scanCapabilities(dir) {
  const caps = { programs: false, network: false, files: false };
  for (const file of await jsFiles(dir)) {
    let src;
    try { src = await fs.readFile(file, "utf8"); } catch { continue; }
    if (/\b(exec|hc)\(/.test(src)) caps.programs = true;
    if (/\bfetch\(|from\s+"(net|websocket)"/.test(src)) caps.network = true;
    if (/\bwriteTextFile\(|from\s+"fs\/promises"/.test(src)) caps.files = true;
  }
  return caps;
}

/** Names of the files inside `dir` (none, if it doesn't exist). */
export async function listFiles(dir) {
  try {
    return (await fs.readdir(dir, { withFileTypes: true })).filter((e) => !e.isDirectory()).map((e) => e.name).sort();
  } catch {
    return [];
  }
}

/** The JSON file at `path`, or null. */
export async function readJson(path) {
  try {
    return JSON.parse(await fs.readFile(path, "utf8"));
  } catch {
    return null;
  }
}
