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
        appBinaryPath: "./src-tauri/target/debug/skopos.exe",
        driverProvider: "embedded",
      },
    ],
  ],
  capabilities: [{ browserName: "tauri" }],
};
