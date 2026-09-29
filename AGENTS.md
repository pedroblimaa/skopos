# Skopos engineering context

## Product

Skopos is a local desktop app that monitors selected Telegram promotion groups and channels and alerts the user when a product matching their criteria appears.

The core flow is:

```text
Connect Telegram → choose promotion chats → define product and maximum price → find matches → notify
```

The user creates watches such as `RTX 5070` with a maximum price of `R$ 4,000`. Skopos inspects selected Telegram messages, extracts product and price information, compares them with enabled watches, and notifies the user when a promotion matches.

## Technical constraints

- Desktop shell: Tauri
- Frontend: React + TypeScript + Vite
- Backend: Rust
- Persistence: SQLite
- Telegram client: MTProto user client, preferably `grammers`
- The app runs entirely locally. Do not add a hosted backend or HTTP API without a concrete technical reason.
- Frontend/backend communication uses Tauri commands and events.
- Telegram session and authentication data remain local.

## Planned capabilities

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

## Repository conventions

- Follow [Rust code guidelines](docs/rust-code-guidelines.md) when writing or reviewing `src-tauri` code.
- Keep Rust module entry files focused on declarations and selective re-exports; put Tauri commands and workflows in responsibility-named files, following the [Rust module guide](https://doc.rust-lang.org/book/ch07-05-separating-modules-into-different-files.html) and [Tauri plugin layout](https://v2.tauri.app/develop/plugins/).
- Keep the frontend and Rust host separately testable.
- Prefer typed interfaces and small pure functions for parsing and matching.
- Keep secrets, Telegram sessions, databases, build output, and local environment files out of Git.
- Use pnpm scripts for frontend checks and Cargo commands for Rust checks.
- Run `pnpm check` before handing off implementation work.
- Run `pnpm tauri build --no-bundle` when native build verification is relevant.
- Keep both production TypeScript and Rust at or above 96% measured line coverage. Run `pnpm check` before handoff; never lower the thresholds or exclude production code to make a change pass.
- Add or update a desktop E2E scenario for every implemented feature flow; run the E2E-only build and `pnpm test:e2e` for changed user flows.
- Keep E2E mocking plugins and permissions out of production builds. Changes to authentication, session storage, dependencies, and capabilities need explicit human review.

## Project structure and organization

- Keep React pages and their hooks under `src/pages/<PageName>`, reusable UI under `src/components/<ComponentName>`, and component styles beside their components. Keep global styles in `src/global.css`.
- Keep the typed Tauri bridge in `src/telegram.ts`. Shared Telegram payloads belong in a nearby domain model file; local types stay beside their use until reuse or readability warrants moving them.
- Keep Rust startup in `src-tauri/src/lib.rs`, feature declarations in `telegram.rs`, and implementation grouped by responsibility inside `telegram/`. Commands own the Tauri boundary, adapters own Telegram calls, workflows own orchestration, and state owns login transitions.
- Colocate frontend unit tests and Rust `tests.rs` files with the code they exercise. Group native desktop E2E tests by feature under `e2e/`; keep fixtures behind test or E2E feature gates.
- Split files for distinct responsibilities, not arbitrary length limits. Avoid a new layer, trait, or helper that only forwards calls without resolving concrete coupling, testing, or readability needs.

## Code style and maintainability

- Follow existing conventions first. Use PascalCase for React components and their folders, `useCamelCase` for hooks, kebab-case for other TypeScript modules, and standard Rust naming.
- Organize functions in reading order: entry points first, direct helpers in call order, and low-level utilities last. Keep closely related types near their implementation.
- Prefer clear domain names, focused functions, explicit boundary types, flat control flow, isolated side effects, and predictable errors. Keep structured Rust errors until the command or event boundary translates them for the UI.
- Name booleans for the positive state they represent, such as `isBusy` or `hasSession`. Prefer boolean `&&` rendering to a ternary returning `null`; use an explicitly boolean condition where necessary.
- Separate distinct logical blocks inside functions with blank lines; keep related statements together and single-block functions compact.
- Use `void` on promises only for intentional fire-and-forget work or lint requirements. Every background operation needs an owner, cleanup, and an explicit failure policy.
- Search for reusable components and styles before adding UI. Feature CSS may arrange shared controls but should not redefine their visual contract. Extend a shared component with a deliberate variant when needed.
- Keep work limited to the requested flow. Avoid speculative fallbacks, compatibility for unreleased formats, generic scaffolding, needless indirection, and forced symmetry.
- Remove temporary diagnostics. Comments should explain a constraint or non-obvious decision, not narrate the code or preserve conversation history.
- Review the entire changed implementation before handoff for clarity, duplication, ownership, error paths, and obvious bugs. Improve concrete problems without rewriting sound code for personal preference.

## Test discipline

- Test observable behavior and failure paths. Keep automated tests isolated from real Telegram accounts, credentials, and production session files.
- Run focused checks first, then the required `pnpm check`. Keep assertions meaningful when refactoring; do not weaken tests or exclude production code to satisfy coverage.

Do not add product-specific behavior until a task explicitly requests it.
