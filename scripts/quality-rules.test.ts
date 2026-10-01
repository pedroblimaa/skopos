// @vitest-environment node
import { ESLint } from "eslint";
import { describe, expect, it } from "vitest";

const eslint = new ESLint();

describe("maintainability lint enforcement", () => {
  it.each([
    {
      rule: "complexity",
      source: `export function accepts(values) { return ${Array.from(
        { length: 21 },
        (_, index) => `values[${String(index)}]`,
      ).join(" || ")}; }`,
    },
    {
      rule: "max-depth",
      source: `export function inspect(value) {
        if (value) { while (value.next) { for (const item of value.items) {
          if (item.ready) { return item; }
        } } }
      }`,
    },
    {
      rule: "max-params",
      source: "export function collect(a, b, c, d, e) { return [a, b, c, d, e]; }",
    },
    {
      rule: "no-else-return",
      source: "export function label(value) { if (value) { return 'yes'; } else { return 'no'; } }",
    },
    {
      rule: "no-nested-ternary",
      source:
        "export function label(value) { return value.ready ? 'ready' : value.busy ? 'busy' : 'idle'; }",
    },
    {
      rule: "no-lonely-if",
      source:
        "export function inspect(value) { if (value.ready) { value.start(); } else { if (value.busy) { value.stop(); } } }",
    },
    {
      rule: "eqeqeq",
      source: "export function matches(value) { return value == 1; }",
    },
    {
      rule: "curly",
      source: "export function inspect(value) { if (value)\nreturn value; }",
    },
    {
      rule: null,
      source: "// eslint-disable-next-line no-nested-ternary\nexport const name = 'product';",
    },
  ])("rejects code violating $rule", async ({ rule, source }) => {
    const [result] = await eslint.lintText(source, {
      filePath: "scripts/quality-fixture.mjs",
    });

    expect(result.messages).toEqual(
      expect.arrayContaining([expect.objectContaining({ ruleId: rule, severity: 2 })]),
    );
  });

  it("accepts a small function with a guard clause and explicit comparison", async () => {
    const [result] = await eslint.lintText(
      `export function label(product) {
        if (!product) return 'Missing';

        return product.enabled === true ? product.name : 'Disabled';
      }`,
      { filePath: "scripts/quality-fixture.mjs" },
    );

    expect(result.messages).toEqual([]);
  });
});
