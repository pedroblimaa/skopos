# Skopos engineering guide

## Product and scope

Skopos is a local desktop app for finding Telegram promotions that match a product name and optional maximum BRL price. Use Tauri, React, TypeScript, Vite, Rust, SQLite, and the `grammers` MTProto client. Communicate through Tauri commands and events; keep credentials, sessions, and data local.

Implemented: QR/phone/password login, session restoration and sign-out, chat selection, product CRUD, manual search of the last 24 hours, saved results, promotion previews, optional photos, external web links, Telegram Saved Messages delivery, and desktop notification summaries. Automatic monitoring, tray/background operation, and startup integration remain planned; plans do not authorize implementation.

Keep work within the requested flow. Do not add hosted services or HTTP APIs without a concrete technical reason. Skopos accounts, email infrastructure, LLM matching, complex rules, scraping, and synchronization are outside the MVP. Add modules and abstractions only for needed behavior; avoid speculative fallbacks and unreleased-format compatibility.

## Architecture and data

```text
Telegram adapter → generic source message → parser/matcher → product match
```

- Keep matching independent of Telegram types in `promotion/`; prices use integer cents. Minimums default to 20% of the maximum (rounded down), or zero without a maximum; a stored custom value overrides this, and zero disables the lower limit. Match both bounds inclusively and require a known price when either bound constrains it. Preserve token matching, alternative names, price ceilings, and rejection of ambiguous, installment, shipping, and old prices.
- Products are app-wide. Selected chats and saved messages are scoped by Telegram account. Results accumulate across searches, deduplicate by chat/message ID, and are rematched against current products. Clearing results does not delete Telegram messages or prevent rediscovery.
- `AppShell` owns chat, product, and search caches across page navigation. Update product caches after successful writes and invalidate matches when criteria change. Prevent stale async completions from overwriting current data; dispose authenticated caches on sign-out.
- Saved-result reads use local storage and cached/persisted account identity; a missing identity may require one Telegram lookup. Missing optional media must not fail results or imply a broken login. Keep photo downloads bounded and preserve cached photos when a later download fails.
- Render Telegram text as text. Open promotion links through the native HTTP/HTTPS validation boundary in `links.rs`; do not enable arbitrary protocols or render message HTML.
- Serialize search and cleanup; preserve cancellation and generation checks so sign-out or superseded operations cannot publish stale results.
- Notification settings and delivery history are account-scoped; both switches default on. Use the existing Telegram session and only send to the current account’s self peer. Save results before enqueueing delivery; keep notification history independent of result cleanup. Preserve uncertain sends across restarts and require explicit retry confirmation. Gate fake Telegram and desktop adapters out of production.

## Code organization

- Pages and hooks: `src/pages/<PageName>`. Shared UI: `src/components/<ComponentName>`. Colocate styles; global tokens belong in `src/global.css`, colors in `src/color-scheme.css`.
- Typed Tauri bridge: `src/telegram.ts`. Shared payloads: nearby domain model files. Keep local types beside their use until reuse warrants moving them. Translate structured `AppMessage` codes in the frontend; keep both supported languages aligned.
- Rust startup: `src-tauri/src/lib.rs`. Feature entry files declare modules and exports. Commands own the Tauri boundary, adapters own Telegram calls, workflows coordinate operations, state owns login transitions, and repositories own persistence.
- Colocate unit tests and Rust `tests.rs`; group desktop scenarios by feature in `e2e/`. Gate fixtures, mocking plugins, and test permissions out of production builds.
- Follow existing naming: PascalCase components/folders, `useCamelCase` hooks, kebab-case TypeScript modules, standard Rust names. Split by responsibility, not line count; avoid forwarding layers without a concrete benefit.

## Required references

Read the applicable guide before implementation or review; its rules apply within that scope.

| Scope                                                 | Guide                                           |
| ----------------------------------------------------- | ----------------------------------------------- |
| Rust host                                             | [Rust guidelines](docs/rust-code-guidelines.md) |
| UI, styles, controls, tooltips, motion, accessibility | [UI guidelines](docs/ui-design-guidelines.md)   |
| Implementation, bugs, tests, fixtures, verification   | [Quality checks](docs/quality-checks.md)        |

## Safeguards and verification

- Keep frontend and native logic separately testable. Isolate tests from real accounts, credentials, sessions, and databases. Keep secrets, local environment files, databases, sessions, and build output out of Git.
- Before code handoff, run focused tests, the desktop scenario for each changed flow, and `pnpm check:local`. Follow the linked local budget and stage-reuse rules. Documentation changes need formatting, link, and diff checks.
- CI owns full frontend/native coverage, desktop E2E, and production-build verification. Report unrun gates as pending. Both production TypeScript and Rust require **96% measured line coverage**; never lower thresholds or exclude production code to pass.
- Authentication, session storage, dependencies, capabilities, quality rules, and thresholds need explicit human review. Fix lint findings; any necessary local suppression must explain its concrete constraint.

## Maintainability

- Prefer focused functions, explicit boundary types, guard clauses, isolated side effects, predictable errors, and positive booleans such as `isBusy`. Keep structured Rust errors until the command/event boundary. Prefer boolean `&&` rendering over ternaries returning `null`.
- Order functions by reading flow: entry points, direct helpers, low-level utilities. Keep related types nearby. Use `void` only for intentional background work or lint requirements; each background operation needs ownership, cleanup, and a failure policy.
- Separate logical steps with one blank line, including setup/action/assertions and repeated test steps. Keep related statements together; formatting does not replace this review.
- Before handoff, review every changed function and test for clarity, grouping, duplication, ownership, failure paths, and bugs. Remove temporary diagnostics; comments explain constraints, not obvious code. Improve concrete problems without rewriting sound code for preference.
