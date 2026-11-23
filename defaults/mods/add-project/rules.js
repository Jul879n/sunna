// Rules of the "add project" dialogs, apart from the view so they can be tested.

/** The folder name a clone lands in, from its URL (`owner/repo`, `…/repo.git`, `git@host:owner/repo`), or "". */
export function repoName(url) {
  const last = String(url ?? "").trim().replace(/[\\/]+$/, "").split(/[\\/:]/).pop() ?? "";
  return last.replace(/\.git$/, "");
}

/** Whether `text` looks like something `git clone` takes: a URL, an scp-style address, or `owner/repo` (GitHub). */
export function cloneUrl(text) {
  const s = String(text ?? "").trim();
  if (!s || /\s/.test(s)) return null;
  if (/^(https?|ssh|git):\/\/\S+$/.test(s) || /^[\w.-]+@[\w.-]+:\S+$/.test(s)) return s;
  if (/^[\w.-]+\/[\w.-]+$/.test(s)) return `https://github.com/${s}.git`;
  return null;
}

/** Why a project name can't be used, as a locale key, or null. */
export function nameProblem(name) {
  const n = String(name ?? "").trim();
  if (!n) return "add-project.name_empty";
  if (/[\\/]/.test(n) || n === "." || n === "..") return "add-project.name_invalid";
  return null;
}

/** Why a parent folder can't be used, as a locale key, or null. */
export function parentProblem(parent) {
  const p = String(parent ?? "").trim();
  if (!p) return "add-project.parent_required";
  if (!p.startsWith("/") && !/^[A-Za-z]:[\\/]/.test(p)) return "add-project.parent_not_absolute";
  return null;
}

/** `parent/child` with the separator the parent uses. */
export function joinPath(parent, child) {
  const p = String(parent).trim().replace(/[\\/]+$/, "");
  return `${p}${String(parent).includes("\\") && !String(parent).includes("/") ? "\\" : "/"}${String(child).trim()}`;
}

/** `~/sunna/projects`: where new projects and clones go unless the person picks another folder. */
export const defaultParent = (home) => joinPath(String(home).replace(/[\\/]+$/, ""), "sunna/projects");
