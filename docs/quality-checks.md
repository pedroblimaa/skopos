# Quality checks and testing

Read when implementing code changes, fixing bugs, changing tests or fixtures, or running verification. Documentation-only changes need focused formatting, link, and diff checks; the full implementation checks apply to code changes.

## Required checks

- Use pnpm scripts for frontend checks and Cargo commands for Rust checks.
- Local handoff requires focused tests for the changed behavior and `pnpm check:local` (formatting, lint, types, and the fast frontend unit suite). Run focused Rust tests for changed native behavior and the relevant desktop scenario for changed user flows. The full verification gate runs in CI; local handoff must state that CI is pending, not claim full verification.
- CI runs `pnpm check:frontend`, `pnpm check:native`, and `pnpm tauri build --no-bundle`. Combined native coverage runs the complete Rust unit and desktop E2E suites. The desktop E2E status depends on that job instead of rebuilding and running the same suite again.
- Keep both production TypeScript and Rust at or above 96% measured line coverage; never lower thresholds or exclude production code to make a change pass. Merge readiness requires the full CI gates to pass.
- Add or update a desktop E2E scenario for every implemented feature flow. Reuse a compatible E2E binary for test-only edits; rebuild after application, asset, configuration, dependency, or toolchain changes. Run a local production build when diagnosing packaging or production-only behavior; routine production build verification stays in CI.
- Keep E2E mocking plugins and permissions out of production builds. Changes to authentication, session storage, dependencies, and capabilities need explicit human review.

## Formatting and lint rules

- `pnpm format` formats frontend/configuration files with Prettier and Rust with stable rustfmt; `pnpm format:check` enforces both locally and in CI. Let Prettier wrap JSX attributes according to line width, use spaces and LF line endings, and preserve deliberately multiline objects. Use rustfmt's default layout for Rust structs, enum variants, and match arms. Formatters preserve existing logical blank lines but cannot decide where separate steps need them; review that grouping explicitly.
- ESLint enforces modified cyclomatic complexity up to 20, control-flow nesting up to 3, at most 4 parameters, and no nested ternaries or redundant `else` after a return. Selected Clippy lints enforce simpler Rust control flow and iterator usage, with cognitive complexity capped at 20, block nesting capped at 4, and at most 5 arguments. Both run in local checks and GitHub CI. These metrics do not replace readability review; avoid extracting forwarding helpers merely to pass a limit.
- Fix lint findings rather than weakening rules or adding blanket suppressions. Any necessary local suppression must explain the concrete constraint; changes to quality rules or thresholds need human review.

## Test discipline

- Test observable behavior and failure paths. Keep automated tests isolated from real Telegram accounts, credentials, and production session files.
- Make each test's setup, action, and assertions visually distinct with blank lines. For multi-step flows, separate each new action and its resulting assertions from the previous step. Keep related assertions together; use descriptive test names instead of comments labeling obvious phases.
- Validate changed test files with formatting, lint, and types before launching expensive builds. Run focused tests next, then the local handoff checks. Keep assertions meaningful; do not weaken tests or exclude production code to satisfy coverage.
- When fixing a bug, confirm it with the smallest useful regression test. On the first unexpected desktop failure, capture the error, screenshot/DOM state, focus, and runner environment needed to distinguish a product bug from a test setup problem. Reproduce using the same binary and launch environment. Change the test or implementation only when evidence supports the change; a passing retry does not establish that an intermittent failure is fixed.
- Keep native builds and heavy checks sequential on this machine. After an edit, repeat only stages whose inputs changed. Do not restart the full pipeline for an E2E assertion, formatting correction, or test type error.

## Local testing budget

For a small feature taking about ten minutes to implement, target 10–20 minutes after acceptance: 4–6 minutes for tests, 3–5 for focused verification, and the remainder for local checks and corrections. This is a planning budget, not permission to skip required tests or declare failures resolved.

- At five minutes investigating one unexplained failure, stop speculative changes and collect decisive evidence. If it occurs only in the full runner, reproduce that environment without rebuilding unrelated stages.
- At 15 minutes, report any blocker and the specific remaining work. At 20 minutes, end the bounded local testing attempt with passed/failed/pending checks and a concrete follow-up; do not silently continue a retry loop or label the feature fully verified. Continue beyond that budget only when the user requests it.
- Record stage durations from the command output when evaluating this workflow. Distinguish time writing tests, executing checks, and debugging. CI duration is separate from local handoff time.

## Independent verification stages

`pnpm check` remains available for a complete local frontend/native run. It prints durations and stops on the first failed stage. `pnpm check --from=rust-coverage` resumes at a named stage; earlier stages are not rerun or represented as freshly passed. Use this only when their relevant inputs have not changed. CI always starts its groups from the beginning.

Combined coverage can also be run in stages:

```text
pnpm test:rust:coverage --stage=unit
pnpm test:rust:coverage --stage=build
pnpm test:rust:coverage --stage=e2e --spec e2e/language.spec.ts
pnpm test:rust:coverage --stage=e2e
pnpm test:rust:coverage --stage=report
```

The unit stage resets raw measurements without deleting compiled instrumented binaries. The build stage includes current frontend assets. E2E and report stages retain measurements from the same source snapshot. After production code or build inputs change, start again with unit and build stages; do not combine profiles from different snapshots. A focused E2E run is diagnostic and does not replace the full E2E stage required for combined coverage. A report alone does not certify that the required suites passed. CI uses the default `all` stage, including a fresh profile reset and all suites.

Available full-check stages are `format`, `lint`, `types`, `frontend-coverage`, `frontend-build`, `rust-lint`, and `rust-coverage`. `build:frontend` only bundles assets; it is used after the independent static checks. The normal `pnpm build` still checks types and color tokens for standalone builds.
