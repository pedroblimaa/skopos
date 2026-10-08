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
  afterTest: async (
    test: { title: string },
    _context: unknown,
    result: { passed: boolean; error?: Error },
  ) => {
    if (result.passed) return;

    const { browser } = await import("@wdio/globals");

    try {
      const state = await browser.execute(() => ({
        documentFocused: document.hasFocus(),
        active: document.activeElement?.outerHTML,
        text: document.body.innerText,
        inputs: Array.from(document.querySelectorAll("input"), (input) => ({
          id: input.id,
          value: input.value,
          checked: input.checked,
          disabled: input.disabled,
        })),
        dialogs: Array.from(document.querySelectorAll("dialog"), (dialog) => ({
          open: dialog.open,
          closing: dialog.dataset.closing,
          resizing: dialog.dataset.resizing,
        })),
      }));
      const message = ` Desktop state: ${JSON.stringify(state)}`;

      if (result.error) result.error.message += message;
      console.error(message);

      const name = test.title.replace(/[^a-z0-9]/gi, "-");
      await browser.saveScreenshot(`src-tauri/target/e2e-failure-${name}.png`);
    } catch (error) {
      console.error(`Desktop failure diagnostics unavailable: ${String(error)}`);
    }
  },
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
