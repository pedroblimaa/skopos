// @vitest-environment node
import { afterEach, beforeEach, expect, it, vi } from "vitest";
import { runCommand } from "./run-command.mjs";

beforeEach(() => {
  vi.spyOn(console, "log").mockImplementation(() => {});
});
afterEach(() => {
  vi.restoreAllMocks();
});

it("captures output from a subprocess using its supplied environment", () => {
  const output = runCommand(
    process.execPath,
    ["-e", "process.stdout.write(process.env.SKOPOS_TEST_VALUE)"],
    {
      env: { ...process.env, SKOPOS_TEST_VALUE: "instrumented profile destination" },
      capture: true,
    },
  );

  expect(output).toBe("instrumented profile destination");
});

it("propagates nonzero subprocess exits instead of reporting a successful stage", () => {
  expect(() => runCommand(process.execPath, ["-e", "process.exit(7)"], { capture: true })).toThrow(
    "failed (7)",
  );
});
