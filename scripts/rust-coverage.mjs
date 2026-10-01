import { existsSync, readdirSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { runCommand, runPnpm } from "./run-command.mjs";

const repo = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const manifest = ["--manifest-path", "src-tauri/Cargo.toml"];
const windows = process.platform === "win32";
const env = {
  ...process.env,
  TG_ID: "1",
  TG_HASH: "coverage",
  CARGO_TARGET_DIR: join(repo, "src-tauri/target/llvm-cov-target"),
};

try {
  const [option = "--stage=all", ...args] = process.argv.slice(2);
  if (!option.startsWith("--stage=")) throw new Error("Use --stage=all|unit|build|e2e|report.");
  measureCoverage(option.slice(8), args);
} catch (error) {
  console.error(error instanceof Error ? error.message : String(error));
  process.exitCode = 1;
}

function measureCoverage(stage, args) {
  if (!["all", "unit", "build", "e2e", "report"].includes(stage)) {
    throw new Error(`Unknown coverage stage: ${stage}`);
  }
  if (args.length && stage !== "e2e") throw new Error("WDIO arguments require --stage=e2e.");
  if (windows && !env.RC) {
    const compiler = findResourceCompiler();
    if (compiler) env.RC = compiler;
  }

  // Force a stable format regardless of the terminal; never evaluate shell output.
  const instrumentation = runCommand("cargo", ["llvm-cov", "show-env", ...manifest, "--cmd"], {
    env,
    capture: true,
  });

  for (const assignment of instrumentation.trimEnd().split(/\r?\n/)) {
    const match = /^set ([A-Za-z_][A-Za-z_0-9]*)=(.*)$/.exec(assignment);
    if (!match) throw new Error(`Invalid coverage environment assignment: ${assignment}`);

    env[match[1]] = match[2];
  }

  if (stage === "all" || stage === "unit") {
    // Reset measurements while retaining instrumented compilation artifacts.
    runCommand("cargo", ["llvm-cov", "clean", ...manifest, "--profraw-only"], { env });
    runCommand("cargo", ["test", ...manifest, "--locked"], { env });
  }

  env.VITE_E2E = "1";
  env.SKOPOS_E2E_BINARY = join(env.CARGO_TARGET_DIR, "debug", windows ? "skopos.exe" : "skopos");

  if (stage === "all" || stage === "build") {
    runPnpm(
      [
        "tauri",
        "build",
        "--debug",
        "--no-bundle",
        "--features",
        "e2e",
        "--config",
        "src-tauri/tauri.e2e.conf.json",
      ],
      { env },
    );
  }

  if (stage === "all" || stage === "e2e") {
    if (!existsSync(env.SKOPOS_E2E_BINARY)) {
      throw new Error("Build the coverage E2E binary first with --stage=build.");
    }
    runPnpm(["test:e2e", ...args], { env });
  }

  if (stage === "all" || stage === "report") {
    // Exclude the Telegram fixture only; keep all production Rust in the report.
    const report = [
      "llvm-cov",
      "report",
      ...manifest,
      "--ignore-filename-regex",
      "[/\\\\]telegram[/\\\\]e2e[/\\\\]",
    ];
    runCommand("cargo", [...report, "--html"], { env });
    runCommand(
      "cargo",
      [...report, "--fail-under-lines", "96", "--show-missing-lines", "--summary-only"],
      { env },
    );
  }
}

function findResourceCompiler() {
  const sdk = join(env["ProgramFiles(x86)"] ?? "C:/Program Files (x86)", "Windows Kits/10/bin");
  if (!existsSync(sdk)) return;

  const versions = readdirSync(sdk).sort((a, b) =>
    b.localeCompare(a, undefined, { numeric: true }),
  );

  for (const version of versions) {
    const compiler = join(sdk, version, "x64/rc.exe");
    if (existsSync(compiler)) return compiler;
  }
}
