import { spawnSync } from "node:child_process";

export function runPnpm(args, options) {
  const launcher = process.env.npm_execpath;
  if (!launcher) throw new Error("Run this command through a pnpm script.");

  const commandOptions = { ...options, label: `pnpm ${args.join(" ")}` };

  return /\.(?:cjs|mjs|js)$/.test(launcher)
    ? runCommand(process.execPath, [launcher, ...args], commandOptions)
    : runCommand(launcher, args, commandOptions);
}

export function runCommand(
  program,
  args,
  { env = process.env, capture = false, label = `${program} ${args.join(" ")}` } = {},
) {
  const started = performance.now();
  const result = spawnSync(program, args, {
    env,
    stdio: capture ? ["inherit", "pipe", "inherit"] : "inherit",
    encoding: "utf8",
    windowsHide: true,
  });
  const seconds = ((performance.now() - started) / 1000).toFixed(1);
  console.log(`[${seconds}s] ${label}: ${result.status === 0 ? "passed" : "failed"}`);

  if (result.error) throw result.error;
  if (result.status !== 0) throw new Error(`${program} failed (${result.signal ?? result.status})`);

  return result.stdout;
}
