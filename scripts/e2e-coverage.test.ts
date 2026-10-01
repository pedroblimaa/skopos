// @vitest-environment node
import { execFileSync } from "node:child_process";
import { resolve } from "node:path";
import { afterEach, describe, expect, it, vi } from "vitest";

afterEach(() => {
  vi.unstubAllEnvs();
  vi.resetModules();
});

describe("desktop coverage artifacts", () => {
  it("routes standalone E2E profiles into the ignored target directory", async () => {
    vi.stubEnv("LLVM_PROFILE_FILE", undefined);

    const { config } = await import("../wdio.conf");

    expect(config.services[0][1]).toMatchObject({
      env: {
        LLVM_PROFILE_FILE: resolve("src-tauri/target/e2e-profiles/skopos-%p-%m.profraw"),
      },
    });
  });

  it("preserves the profile destination supplied by cargo-llvm-cov", async () => {
    const destination = resolve("src-tauri/target/llvm-cov-target/coverage-%p-%m.profraw");
    vi.stubEnv("LLVM_PROFILE_FILE", destination);

    const { config } = await import("../wdio.conf");

    expect(config.services[0][1]).toMatchObject({
      env: { LLVM_PROFILE_FILE: destination },
    });
  });

  it("ignores raw and merged coverage artifacts even outside target", () => {
    const files = ["default_example.profraw", "coverage.profdata"];

    const ignored = execFileSync("git", ["check-ignore", "--no-index", "--stdin"], {
      cwd: resolve(import.meta.dirname, ".."),
      input: files.join("\n"),
      encoding: "utf8",
    });

    expect(ignored.trim().split(/\r?\n/)).toEqual(files);
  });
});
