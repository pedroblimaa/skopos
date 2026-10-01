# Skopos

Skopos is a local-first desktop application for monitoring Telegram promotion chats. Telegram QR login, phone-code login, two-step verification, session restoration, and disconnect are implemented. Watches, promotion matching, and notifications are planned for later tasks.

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

```text
pnpm check:local
```

Local handoff uses focused feature tests plus `pnpm check:local` for formatting, lint, types, and frontend unit tests. Full verification remains pending CI, which enforces 96% line coverage for both TypeScript and Rust, runs all desktop scenarios, and verifies the production native build.

`pnpm check` is still available for a complete local frontend/native run. It prints stage durations and supports `--from=<stage>` for explicit resumption. Install `cargo-llvm-cov` 0.9.1 and the Rust `llvm-tools-preview` component for native coverage. `pnpm test:rust:coverage` combines unit and native desktop E2E measurements and supports independent unit, build, E2E, and report stages. Desktop scenarios use a test-only Telegram API fixture and are currently verified on Windows. See [Quality checks and testing](docs/quality-checks.md) for the 10–20 minute local budget, stage commands, and binary/profile reuse requirements.

## Project layout

- `src/`: React + TypeScript frontend
- `src-tauri/`: Rust/Tauri desktop host
- `AGENTS.md`: product context and engineering constraints for future work
