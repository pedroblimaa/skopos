// @vitest-environment node
import { afterEach, beforeEach, expect, it, vi } from "vitest";

const mocks = vi.hoisted(() => ({ runCommand: vi.fn(), runPnpm: vi.fn(), existsSync: vi.fn() }));
vi.mock("./run-command.mjs", () => mocks);
vi.mock("node:fs", async (importOriginal) => ({
  ...(await importOriginal()),
  existsSync: mocks.existsSync,
}));
const originalArgs = process.argv;
const originalExitCode = process.exitCode;

beforeEach(() => {
  vi.resetModules();
  mocks.runCommand.mockReset().mockReturnValue("set LLVM_PROFILE_FILE=coverage-%p-%m.profraw");
  mocks.runPnpm.mockReset();
  mocks.existsSync.mockReset().mockReturnValue(true);
  vi.stubEnv("RC", "test-rc");
  vi.spyOn(console, "error").mockImplementation(() => {});
});
afterEach(() => {
  process.argv = originalArgs;
  process.exitCode = originalExitCode;
  vi.unstubAllEnvs();
  vi.restoreAllMocks();
});

it("resets raw profiles without deleting compiled binaries for a fresh full run", async () => {
  process.argv = ["node", "rust-coverage"];

  await import("./rust-coverage.mjs");

  const commands = mocks.runCommand.mock.calls.map(([, args]) => args);
  expect(commands[1]).toEqual([
    "llvm-cov",
    "clean",
    "--manifest-path",
    "src-tauri/Cargo.toml",
    "--profraw-only",
  ]);
  expect(commands[2]).toEqual(["test", "--manifest-path", "src-tauri/Cargo.toml", "--locked"]);
  expect(commands.at(-1)).toContain("--fail-under-lines");
  expect(commands.at(-1)).toContain("96");
  expect(mocks.runPnpm.mock.calls[1][0]).toEqual(["test:e2e"]);
});

it("reruns only selected E2E scenarios with the same binary and coverage destination", async () => {
  process.argv = ["node", "rust-coverage", "--stage=e2e", "--spec", "e2e/language.spec.ts"];

  await import("./rust-coverage.mjs");

  expect(mocks.runCommand).toHaveBeenCalledTimes(1);
  expect(mocks.runPnpm).toHaveBeenCalledExactlyOnceWith(
    ["test:e2e", "--spec", "e2e/language.spec.ts"],
    {
      env: expect.objectContaining({
        LLVM_PROFILE_FILE: "coverage-%p-%m.profraw",
        VITE_E2E: "1",
        SKOPOS_E2E_BINARY: expect.stringMatching(/llvm-cov-target.*debug.*skopos/),
      }),
    },
  );
});

it("refuses to run E2E without a previously built binary", async () => {
  process.argv = ["node", "rust-coverage", "--stage=e2e"];
  mocks.existsSync.mockReturnValue(false);

  await import("./rust-coverage.mjs");

  expect(mocks.runPnpm).not.toHaveBeenCalled();
  expect(process.exitCode).toBe(1);
});

it("does not generate a passing report after an E2E failure", async () => {
  process.argv = ["node", "rust-coverage"];
  mocks.runPnpm.mockImplementation((args) => {
    if (args[0] === "test:e2e") throw new Error("E2E failure");
  });

  await import("./rust-coverage.mjs");

  expect(mocks.runCommand.mock.calls.some(([, args]) => args.includes("report"))).toBe(false);
  expect(process.exitCode).toBe(1);
});

it.each([["--stage=unknown"], ["--stage=report", "--spec", "e2e/watch.spec.ts"]])(
  "rejects unsupported stages or misplaced E2E filters: %j",
  async (args) => {
    process.argv = ["node", "rust-coverage", ...args];

    await import("./rust-coverage.mjs");

    expect(mocks.runCommand).not.toHaveBeenCalled();
    expect(process.exitCode).toBe(1);
  },
);
