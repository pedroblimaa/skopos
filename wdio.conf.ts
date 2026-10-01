import { resolve } from "node:path";

export const config = {
  runner: "local",
  logLevel: "error",
  specs: ["./e2e/**/*.spec.ts"],
  maxInstances: 1,
  framework: "mocha",
  reporters: ["spec"],
  mochaOpts: { timeout: 30_000 },
  services: [
    [
      "@wdio/tauri-service",
      {
        appBinaryPath:
          process.env.SKOPOS_E2E_BINARY ??
          `./src-tauri/target/debug/skopos${process.platform === "win32" ? ".exe" : ""}`,
        driverProvider: "embedded",
        env: {
          LLVM_PROFILE_FILE:
            process.env.LLVM_PROFILE_FILE ??
            resolve(import.meta.dirname, "src-tauri/target/e2e-profiles/skopos-%p-%m.profraw"),
        },
      },
    ],
  ],
  capabilities: [{ browserName: "tauri" }],
  after: async () => {
    if (process.env.SKOPOS_E2E_BINARY) {
      const { browser } = await import("@wdio/globals");

      await browser.execute(async () => {
        const api = (
          window as Window & { __TAURI__: { core: { invoke: (name: string) => Promise<void> } } }
        ).__TAURI__;

        await api.core.invoke("flush_coverage");
      });
    }
  },
};
