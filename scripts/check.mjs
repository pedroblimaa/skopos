import { runPnpm } from "./run-command.mjs";

const stages = {
  "format": "format:check",
  "lint": "lint",
  "types": "typecheck",
  "frontend-coverage": "test:coverage",
  "frontend-build": "build:frontend",
  "rust-lint": "lint:rust",
  "rust-coverage": "test:rust:coverage",
};

try {
  check(process.argv.slice(2));
} catch (error) {
  console.error(error instanceof Error ? error.message : String(error));
  process.exitCode = 1;
}

function check(args) {
  const group = args.find((arg) => arg.startsWith("--group="))?.slice(8) ?? "all";
  const from = args.find((arg) => arg.startsWith("--from="))?.slice(7);
  if (args.some((arg) => !arg.startsWith("--group=") && !arg.startsWith("--from="))) {
    throw new Error("Use --group=all|local|frontend|native and optionally --from=<stage>.");
  }

  const groups = {
    all: Object.keys(stages),
    local: ["format", "lint", "types", "frontend-unit"],
    frontend: ["format", "lint", "types", "frontend-coverage", "frontend-build"],
    native: ["rust-lint", "rust-coverage"],
  };
  if (!Object.hasOwn(groups, group)) throw new Error(`Unknown check group: ${group}`);

  const selected = groups[group];
  const start = from ? selected.indexOf(from) : 0;
  if (start < 0) throw new Error(`Stage ${from} is not in group ${group}.`);

  for (const stage of selected.slice(start)) {
    console.log(`Checking ${stage}`);
    runPnpm([stage === "frontend-unit" ? "test" : stages[stage]]);
  }

  console.log(`Passed stages: ${selected.slice(start).join(", ")}`);
}
