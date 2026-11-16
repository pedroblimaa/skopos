import assert from "node:assert/strict";
import { execFile } from "node:child_process";
import { readFile, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join, resolve } from "node:path";
import { promisify } from "node:util";
import { browser } from "@wdio/globals";
import { describe, it } from "mocha";

const execute = promisify(execFile);
const binary = process.env.SKOPOS_E2E_BINARY ?? resolve("src-tauri/target/debug/skopos.exe");

describe("Desktop process lifecycle", () => {
  it("a second background instance preserves the hidden window and a foreground launch reopens it", async () => {
    await command("close");
    await browser.waitUntil(async () => !(await command("inspect")).visible);

    await execute(binary, ["--background"], { windowsHide: true });

    assert.equal((await command("inspect")).visible, false);

    await execute(binary, [], { windowsHide: true });
    await browser.waitUntil(async () => (await command("inspect")).visible);

    assert.equal((await command("inspect")).stopping, false);
  });

  it("graceful quit waits for active work and duplicate quit requests exit successfully", async () => {
    const child = execFile(binary, ["--background"], {
      windowsHide: true,
      timeout: 10_000,
      env: { ...process.env, SKOPOS_E2E_EXIT_PROBE: "1" },
    });
    assert.ok(child.pid);
    const directory = join(tmpdir(), `skopos-e2e-${String(child.pid)}`);
    const exited = new Promise<void>((resolveExit, reject) => {
      child.once("error", reject);
      child.once("exit", (code, signal) => {
        if (code === 0 && signal === null) {
          resolveExit();
        } else {
          reject(
            new Error(`Shutdown probe exited with code ${String(code)}, signal ${String(signal)}`),
          );
        }
      });
    });

    try {
      await exited;

      assert.equal(
        await readFile(join(directory, "shutdown-verified"), "utf8"),
        "operations drained before exit",
      );
    } finally {
      await rm(directory, { recursive: true, force: true });
    }
  });
});

async function command(action: string) {
  const serialized = await browser.execute(async (requestedAction) => {
    const api = (
      window as Window & {
        __TAURI__: {
          core: {
            invoke: (
              name: string,
              args: Record<string, unknown>,
            ) => Promise<{ visible: boolean; stopping: boolean }>;
          };
        };
      }
    ).__TAURI__;

    return JSON.stringify(await api.core.invoke("desktop_fixture", { action: requestedAction }));
  }, action);

  return JSON.parse(serialized) as { visible: boolean; stopping: boolean };
}
