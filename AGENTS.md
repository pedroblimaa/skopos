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

This repository starts as a blank Tauri foundation. Do not add product-specific behavior until a task explicitly requests it.
