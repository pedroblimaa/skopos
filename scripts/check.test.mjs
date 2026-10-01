// @vitest-environment node
import { afterEach, beforeEach, expect, it, vi } from "vitest";

const { runPnpm } = vi.hoisted(() => ({ runPnpm: vi.fn() }));
vi.mock("./run-command.mjs", () => ({ runPnpm }));
const originalArgs = process.argv;
const originalExitCode = process.exitCode;

beforeEach(() => {
  vi.resetModules();
  runPnpm.mockReset();
  vi.spyOn(console, "log").mockImplementation(() => {});
  vi.spyOn(console, "error").mockImplementation(() => {});
});
afterEach(() => {
  process.argv = originalArgs;
  process.exitCode = originalExitCode;
  vi.restoreAllMocks();
});

it("runs inexpensive local checks without native builds or coverage", async () => {
  process.argv = ["node", "check", "--group=local"];

  await import("./check.mjs");

  expect(runPnpm.mock.calls.map(([args]) => args)).toEqual([
    ["format:check"],
    ["lint"],
    ["typecheck"],
    ["test"],
  ]);
});

it("resumes at the selected stage without rerunning earlier successful checks", async () => {
  process.argv = ["node", "check", "--from=rust-coverage"];

  await import("./check.mjs");

  expect(runPnpm).toHaveBeenCalledExactlyOnceWith(["test:rust:coverage"]);
});

it("stops immediately on failure instead of running later stages or retrying", async () => {
  process.argv = ["node", "check", "--group=native"];
  runPnpm.mockImplementation(() => {
    throw new Error("Clippy failure");
  });

  await import("./check.mjs");

  expect(runPnpm).toHaveBeenCalledExactlyOnceWith(["lint:rust"]);
  expect(process.exitCode).toBe(1);
});

it.each(["--group=unknown", "--from=missing", "--skip-everything"])(
  "rejects invalid arguments without executing checks: %s",
  async (arg) => {
    process.argv = ["node", "check", arg];

    await import("./check.mjs");

    expect(runPnpm).not.toHaveBeenCalled();
    expect(process.exitCode).toBe(1);
  },
);
