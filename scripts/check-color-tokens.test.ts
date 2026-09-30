// @vitest-environment node
import { execFileSync, spawnSync } from "node:child_process";
import { mkdtempSync, mkdirSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join } from "node:path";
import { afterEach, describe, expect, it } from "vitest";

const fixtures: string[] = [];
const checker = readFileSync(new URL("./check-color-tokens.mjs", import.meta.url), "utf8");

afterEach(() => {
  for (const fixture of fixtures.splice(0)) rmSync(fixture, { recursive: true, force: true });
});

describe("color token enforcement", () => {
  it("allows palette literals, variables, currentColor and test data", () => {
    const root = fixture({
      "src/color-scheme.css":
        ":root { --color-accent: #8bd6bf; --color-transparent: transparent; }",
      "src/component.css":
        "/* red #fff rgb(0, 0, 0) */\n.card { color: var(--color-accent); background: var(--color-transparent); border: 1px solid currentColor; }",
      "src/component.tsx": '<svg fill="var(--color-accent)" stroke="currentColor" />',
      "src/component.test.tsx": 'const sample = "#fff";',
      "src/nested/component.spec.ts": 'const sample = "red";',
      "index.html": "<!-- #fff --> <html></html>",
    });

    expect(
      execFileSync(process.execPath, [join(root, "scripts/check-color-tokens.mjs")], {
        encoding: "utf8",
      }),
    ).toContain("all frontend colors use the shared palette");
  });

  it.each([
    ["src/nested/component.css", ".card { color: #ffb3a6; }", "#ffb3a6"],
    ["src/component.css", ".card { box-shadow: 0 1px 3px rgba(0, 0, 0, .2); }", "rgba("],
    ["src/component.css", ".card { background: transparent; }", "transparent"],
    ["src/component.css", ".card { color: rebeccapurple; }", "rebeccapurple"],
    ["src/component.tsx", '<svg fill="white" />', "white"],
    ["src/component.tsx", '<QrDisplay fgColor="black" />', "black"],
    ["src/component.tsx", '<QrDisplay fgColor={"black"} />', "black"],
    ["src/component.ts", 'const style = { backgroundColor: "red" };', "red"],
    ["src/component.svg", '<svg stroke="#fff" />', "#fff"],
    ["index.html", '<meta name="theme-color" content="#fff" />', "#fff"],
  ])("rejects %s literal %s with a line diagnostic", (file, source, literal) => {
    const root = fixture({ [file]: `/* ignored #000 */\n${source}` });

    const result = spawnSync(process.execPath, [join(root, "scripts/check-color-tokens.mjs")], {
      encoding: "utf8",
    });

    expect(result.status).toBe(1);
    expect(result.stderr.replace(/\\/g, "/")).toContain(`${file}:2: literal color ${literal}`);
  });
});

function fixture(files: Record<string, string>) {
  const root = mkdtempSync(join(tmpdir(), "skopos-color-tokens-"));
  fixtures.push(root);
  const sources = {
    "scripts/check-color-tokens.mjs": checker,
    "index.html": "<html></html>",
    ...files,
  };
  mkdirSync(join(root, "src"), { recursive: true });

  for (const [file, content] of Object.entries(sources)) {
    const target = join(root, file);
    mkdirSync(dirname(target), { recursive: true });
    writeFileSync(target, content);
  }

  return root;
}
