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
pnpm check
pnpm tauri build --no-bundle
```

The check command runs Prettier, ESLint, TypeScript, Vitest with a 96% line-coverage gate, the frontend production build, Rust formatting, Clippy, Rust tests, and a 96% Rust line-coverage gate combining unit tests with native desktop E2E scenarios. Install `cargo-llvm-cov` 0.9.1 and the Rust `llvm-tools-preview` component to run that gate. `pnpm test:rust:coverage` uses a Node.js runner with no PowerShell or Bash dependency. The runner handles Windows, Linux, and macOS paths; native desktop E2E is currently verified on Windows only. Desktop scenarios use a test-only Telegram API fixture and are documented in `docs/e2e-coverage.md`.

## Project layout

- `src/`: React + TypeScript frontend
- `src-tauri/`: Rust/Tauri desktop host
- `AGENTS.md`: product context and engineering constraints for future work
