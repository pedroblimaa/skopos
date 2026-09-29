import { spawnSync } from "node:child_process";
import { existsSync, readdirSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

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
  measureCoverage();
} catch (error) {
  console.error(error instanceof Error ? error.message : String(error));
  process.exitCode = 1;
}

function measureCoverage() {
  if (windows && !env.RC) {
    const compiler = findResourceCompiler();
    if (compiler) env.RC = compiler;
  }

  // Force a stable format regardless of the terminal; never evaluate shell output.
  const instrumentation = run("cargo", ["llvm-cov", "show-env", ...manifest, "--cmd"], true);

  for (const assignment of instrumentation.trimEnd().split(/\r?\n/)) {
    const match = /^set ([A-Za-z_][A-Za-z_0-9]*)=(.*)$/.exec(assignment);
    if (!match) throw new Error(`Invalid coverage environment assignment: ${assignment}`);

    env[match[1]] = match[2];
  }

  // Remove stale workspace coverage artifacts while retaining compiled dependencies.
  run("cargo", ["llvm-cov", "clean", ...manifest, "--workspace"]);
  run("cargo", ["test", ...manifest, "--locked"]);

  env.VITE_E2E = "1";
  runPnpm([
    "tauri",
    "build",
    "--debug",
    "--no-bundle",
    "--features",
    "e2e",
    "--config",
    "src-tauri/tauri.e2e.conf.json",
  ]);

  env.SKOPOS_E2E_BINARY = join(env.CARGO_TARGET_DIR, "debug", windows ? "skopos.exe" : "skopos");
  runPnpm(["test:e2e"]);

  // Exclude the Telegram fixture only; keep all production Rust in the report.
  const report = [
    "llvm-cov",
    "report",
    ...manifest,
    "--ignore-filename-regex",
    "[/\\\\]telegram[/\\\\]e2e[/\\\\]",
  ];
  run("cargo", [...report, "--html"]);
  run("cargo", [...report, "--fail-under-lines", "96", "--show-missing-lines", "--summary-only"]);
}

function run(program, args, capture = false) {
  const result = spawnSync(program, args, {
    cwd: repo,
    env,
    stdio: capture ? ["inherit", "pipe", "inherit"] : "inherit",
    encoding: "utf8",
    windowsHide: true,
  });

  if (result.error) throw result.error;
  if (result.status !== 0) {
    throw new Error(`${program} failed (${result.signal ?? result.status})`);
  }

  return result.stdout;
}

function runPnpm(args) {
  const launcher = process.env.npm_execpath;
  if (!launcher) throw new Error("Run this script with pnpm test:rust:coverage.");

  if (/\.(?:cjs|mjs|js)$/.test(launcher)) run(process.execPath, [launcher, ...args]);
  else run(launcher, args);
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
