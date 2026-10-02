# Skopos engineering context

## Product and constraints

Skopos is a local desktop app that monitors selected Telegram promotion groups and channels and alerts the user when a product matches their criteria.

```text
Connect Telegram → choose promotion chats → define product and maximum price → find matches → notify
```

- Use Tauri, React + TypeScript + Vite, Rust, SQLite, and an MTProto Telegram user client (preferably `grammers`).
- Run entirely locally. Do not add a hosted backend or HTTP API without a concrete technical reason. Frontend/backend communication uses Tauri commands and events; Telegram sessions and authentication data remain local.
- Do not add product-specific behavior until a task explicitly requests it. Introduce modules and abstractions only when their behavior is needed.

The user creates watches such as `RTX 5070` with a maximum price of `R$ 4,000`. Skopos inspects selected Telegram messages, extracts product and price information, compares them with enabled watches, and notifies the user when a promotion matches.

## Planned capabilities

Planned capabilities are context, not authorization to implement them.

- Telegram QR login, phone verification code, two-step verification password, persistent session, and logout/disconnect.
- Watch creation, editing, enable/disable, removal, product/search text, and maximum price.
- Retrieval and searchable filtering of chats accessible to the authenticated account, with selected sources persisted locally.
- Manual recent-message search and real-time monitoring using the same generic parser and matcher pipeline.
- Native desktop notifications with an original-message or promotion link when available.
- Duplicate-alert prevention for the same Telegram message.
- Background monitoring while the window is closed, system tray support, optional Windows startup, reconnection, and restart restoration.
- Local storage for watches, selected chats, seen message IDs, promotion history, settings, and Telegram session state.

## Architecture direction

Keep business logic independent of Telegram-specific types. The intended boundary is:

```text
Telegram adapter → generic message/promotion data → parser → matcher → watch match → notification
```

Suggested Rust areas are `telegram/` (auth, chats, history, listener), `watch/` (model, repository), `promotion/` (parser, matcher), `notification/`, and `storage/`. Introduce these areas only when their behavior is needed.

The matcher must operate on generic promotion/message data so future sources such as websites, Discord, or RSS can reuse it.

## MVP boundaries

Keep the first release intentionally small. Do not introduce Skopos accounts, hosted services, email infrastructure, LLM-based matching, complex rule engines, browser/headless scraping, multi-device synchronization, or unnecessary abstractions.

Initial matching may be simple text/product detection plus price extraction and comparison. Exclusions, aliases, fuzzy matching, and LLM assistance are future enhancements.

## Project structure and organization

- Keep React pages and their hooks under `src/pages/<PageName>`, reusable UI under `src/components/<ComponentName>`, and component styles beside their components. Keep global styles in `src/global.css`.
- Keep the typed Tauri bridge in `src/telegram.ts`. Shared Telegram payloads belong in a nearby domain model file; local types stay beside their use until reuse or readability warrants moving them.
- Keep Rust startup in `src-tauri/src/lib.rs`, feature declarations in `telegram.rs`, and implementation grouped by responsibility inside `telegram/`. Commands own the Tauri boundary, adapters own Telegram calls, workflows own orchestration, and state owns login transitions.
- Colocate frontend unit tests and Rust `tests.rs` files with the code they exercise. Group native desktop E2E tests by feature under `e2e/`; keep fixtures behind test or E2E feature gates.
- Split files for distinct responsibilities, not arbitrary length limits. Avoid a new layer, trait, or helper that only forwards calls without resolving concrete coupling, testing, or readability needs.

## Task-specific references

Read the applicable reference before implementing or reviewing the corresponding area. Load only references relevant to the task; their rules are required within that scope.

| When the task involves                                                   | Read                                                 |
| ------------------------------------------------------------------------ | ---------------------------------------------------- |
| Writing or reviewing `src-tauri` code                                    | [Rust code guidelines](docs/rust-code-guidelines.md) |
| UI, styling, shared controls, tokens, tooltips, motion, or accessibility | [UI design guidelines](docs/ui-design-guidelines.md) |
| Code implementation, bug fixes, tests, fixtures, or verification         | [Quality checks and testing](docs/quality-checks.md) |

## Repository safeguards

- Keep the frontend and Rust host separately testable. Prefer typed interfaces and small pure functions for parsing and matching.
- Keep secrets, Telegram sessions, databases, build output, and local environment files out of Git. Isolate automated tests from real Telegram accounts, credentials, and production session files.
- Use focused local tests and `pnpm check:local` before local handoff; report full verification as pending CI. CI runs the full frontend, Rust coverage, desktop E2E, and production build gates. Keep both production TypeScript and Rust at or above 96% measured line coverage; never lower thresholds or exclude production code to make a change pass. See `docs/quality-checks.md` for the local budget and stage commands.
- Add or update a desktop E2E scenario for every implemented feature flow. Keep E2E mocking plugins and permissions out of production builds.
- Changes to authentication, session storage, dependencies, capabilities, quality rules, or thresholds need explicit human review. Fix lint findings rather than weakening rules or adding blanket suppressions; any necessary local suppression must explain its concrete constraint.

## Code style and maintainability

- Follow existing conventions first. Use PascalCase for React components and their folders, `useCamelCase` for hooks, kebab-case for other TypeScript modules, and standard Rust naming.
- Organize functions in reading order: entry points first, direct helpers in call order, and low-level utilities last. Keep closely related types near their implementation.
- Prefer clear domain names, focused functions, explicit boundary types, flat control flow, isolated side effects, and predictable errors. Keep structured Rust errors until the command or event boundary translates them for the UI.
- Name booleans for the positive state they represent, such as `isBusy` or `hasSession`. Prefer boolean `&&` rendering to a ternary returning `null`; use an explicitly boolean condition where necessary.
- Do not write cramped code. Separate distinct logical steps with one blank line: setup, validation, side effects, result handling, and return. Keep statements for the same step together; do not add a blank line after every statement. This applies to implementation, helpers, fixtures, and tests in every language.
- Use `void` on promises only for intentional fire-and-forget work or lint requirements. Every background operation needs an owner, cleanup, and an explicit failure policy.
- Keep work limited to the requested flow. Avoid speculative fallbacks, compatibility for unreleased formats, generic scaffolding, needless indirection, and forced symmetry.
- Remove temporary diagnostics. Comments should explain a constraint or non-obvious decision, not narrate the code or preserve conversation history.
- Review the entire changed implementation before handoff for clarity, duplication, ownership, error paths, and obvious bugs. Improve concrete problems without rewriting sound code for personal preference.
- Before handoff, inspect every changed function and test for visual grouping. Split uninterrupted runs of statements that perform different steps, including repeated action/assertion sequences. Formatter and lint success do not replace this readability review.
