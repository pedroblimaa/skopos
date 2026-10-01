# Quality checks and testing

Read when implementing code changes, fixing bugs, changing tests or fixtures, or running verification. Documentation-only changes need focused formatting, link, and diff checks; the full implementation checks apply to code changes.

## Required checks

- Use pnpm scripts for frontend checks and Cargo commands for Rust checks.
- Run `pnpm check` before handing off implementation work.
- Run `pnpm tauri build --no-bundle` when native build verification is relevant.
- Keep both production TypeScript and Rust at or above 96% measured line coverage. Run `pnpm check` before handoff; never lower the thresholds or exclude production code to make a change pass.
- Add or update a desktop E2E scenario for every implemented feature flow; run the E2E-only build and `pnpm test:e2e` for changed user flows.
- Keep E2E mocking plugins and permissions out of production builds. Changes to authentication, session storage, dependencies, and capabilities need explicit human review.

## Formatting and lint rules

- `pnpm format` formats frontend/configuration files with Prettier and Rust with stable rustfmt; `pnpm format:check` enforces both locally and in CI. Let Prettier wrap JSX attributes according to line width, use spaces and LF line endings, and preserve deliberately multiline objects. Use rustfmt's default layout for Rust structs, enum variants, and match arms. Formatters preserve existing logical blank lines but cannot decide where separate steps need them; review that grouping explicitly.
- ESLint enforces modified cyclomatic complexity up to 20, control-flow nesting up to 3, at most 4 parameters, and no nested ternaries or redundant `else` after a return. Selected Clippy lints enforce simpler Rust control flow and iterator usage, with cognitive complexity capped at 20 and at most 5 arguments. Both run in local checks and GitHub CI. These metrics do not replace readability review; avoid extracting forwarding helpers merely to pass a limit.
- Fix lint findings rather than weakening rules or adding blanket suppressions. Any necessary local suppression must explain the concrete constraint; changes to quality rules or thresholds need human review.

## Test discipline

- Test observable behavior and failure paths. Keep automated tests isolated from real Telegram accounts, credentials, and production session files.
- Make each test's setup, action, and assertions visually distinct with blank lines. For multi-step flows, separate each new action and its resulting assertions from the previous step. Keep related assertions together; use descriptive test names instead of comments labeling obvious phases.
- Run focused checks first, then the required `pnpm check`. Keep assertions meaningful when refactoring; do not weaken tests or exclude production code to satisfy coverage.
- When fixing a bug, confirm it with the smallest useful regression test. Validate changed tests, fixtures, lint, and types with focused commands before running `pnpm check`. For desktop E2E changes, reuse an existing compatible E2E binary when only test code changes; rebuild when application code or build configuration changes. After focused checks pass, run the full required checks. Repeat only the checks affected by subsequent changes.
