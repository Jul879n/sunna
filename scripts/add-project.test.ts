import { expect, test } from "bun:test";
import { loadModule } from "./mod-module";

const rules = await loadModule(new URL("../defaults/mods/add-project/rules.js", import.meta.url), {});

test("a clone lands in a folder named after the repository", () => {
  expect(rules.repoName("https://github.com/owner/repo.git")).toBe("repo");
  expect(rules.repoName("git@github.com:owner/repo.git")).toBe("repo");
  expect(rules.repoName("owner/repo")).toBe("repo");
  expect(rules.repoName("https://host/a/b/")).toBe("b");
  expect(rules.repoName("")).toBe("");
});

test("only something git clone takes is a clone URL; owner/repo means GitHub", () => {
  expect(rules.cloneUrl("https://github.com/o/r.git")).toBe("https://github.com/o/r.git");
  expect(rules.cloneUrl("ssh://git@host/o/r")).toBe("ssh://git@host/o/r");
  expect(rules.cloneUrl("git@host:o/r.git")).toBe("git@host:o/r.git");
  expect(rules.cloneUrl("o/r")).toBe("https://github.com/o/r.git");
  expect(rules.cloneUrl("not a url")).toBeNull();
  expect(rules.cloneUrl("")).toBeNull();
  expect(rules.cloneUrl("justaword")).toBeNull();
});

test("project names reject empty values and path separators", () => {
  expect(rules.nameProblem("")).toBe("add-project.name_empty");
  expect(rules.nameProblem("  ")).toBe("add-project.name_empty");
  expect(rules.nameProblem("a/b")).toBe("add-project.name_invalid");
  expect(rules.nameProblem("a\\b")).toBe("add-project.name_invalid");
  expect(rules.nameProblem("..")).toBe("add-project.name_invalid");
  expect(rules.nameProblem("my-app")).toBeNull();
});

test("the parent folder must be given and absolute", () => {
  expect(rules.parentProblem("")).toBe("add-project.parent_required");
  expect(rules.parentProblem("projects")).toBe("add-project.parent_not_absolute");
  expect(rules.parentProblem("/home/u/projects")).toBeNull();
  expect(rules.parentProblem("C:\\Users\\u")).toBeNull();
});

test("paths join with the parent's separator and projects default under ~/sunna/projects", () => {
  expect(rules.joinPath("/a/b/", "c")).toBe("/a/b/c");
  expect(rules.joinPath("C:\\a", "c")).toBe("C:\\a\\c");
  expect(rules.defaultParent("/Users/me/")).toBe("/Users/me/sunna/projects");
});
