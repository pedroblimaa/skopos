# Skopos

Skopos is a local desktop application for finding Telegram promotions. Connect Telegram, select promotion chats, add products with optional maximum BRL prices, and search messages from the last 24 hours. Login, session restoration, product editing, saved results, promotion photos, clickable web links, Telegram Saved Messages, and desktop notification summaries are implemented. Automatic monitoring remains planned.

Saved results accumulate across searches and survive restarts. Products are shared across Telegram accounts; chat selections and promotion history are stored separately for each account. Editing a product rematches its saved messages. Clearing results removes local history; a later search can find those messages again.

## Prerequisites

- Windows with Microsoft C++ Build Tools and the **Desktop development with C++** workload
- Rust stable using the `x86_64-pc-windows-msvc` toolchain
- WebView2 Runtime (included with supported Windows versions)
- Node.js 22.12+ and pnpm

## Development

```text
pnpm install
pnpm dev
pnpm tauri dev
```

`pnpm dev` starts the Vite frontend. `pnpm tauri dev` starts the native desktop application.

## Telegram login configuration

Create a Telegram application at [my.telegram.org](https://my.telegram.org/), copy `.env.base` to `.env`, and fill in its API credentials:

```text
TG_ID=your-api-id
TG_HASH=your-api-hash
```

Run `pnpm tauri dev` or `pnpm tauri build --no-bundle` after saving `.env`. The Rust build reads the repository's `.env`; shell variables with the same names take precedence. The credentials are compiled into the native app, so rebuild after changing them. Builds without credentials still compile, but login displays a configuration error. `.env` is ignored by Git. Telegram sessions are stored in Skopos's local application data directory.

## Quality checks

See [Saved Messages and desktop alerts](docs/notification-setup.md). Both switches initially default on; Telegram delivery uses the connected account without bot setup. Delivery history remains separate from saved results, so clearing results does not resend successful notifications.

```text
pnpm check:local
```

Local handoff uses focused feature tests plus `pnpm check:local` for formatting, lint, types, and frontend unit tests. Full verification remains pending CI, which enforces 96% line coverage for both TypeScript and Rust, runs all desktop scenarios, and verifies the production native build.

`pnpm check` is still available for a complete local frontend/native run. It prints stage durations and supports `--from=<stage>` for explicit resumption. Install `cargo-llvm-cov` 0.9.1 and the Rust `llvm-tools-preview` component for native coverage. `pnpm test:rust:coverage` combines unit and native desktop E2E measurements and supports independent unit, build, E2E, and report stages. Desktop scenarios use a test-only Telegram API fixture and are currently verified on Windows. See [Quality checks and testing](docs/quality-checks.md) for the 10–20 minute local budget, stage commands, and binary/profile reuse requirements.

## Project layout

- `src/`: React + TypeScript frontend
- `src-tauri/`: Rust/Tauri desktop host
- `AGENTS.md`: product context and engineering constraints for future work
