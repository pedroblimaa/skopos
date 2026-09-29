# Desktop E2E coverage

Every implemented user flow needs a deterministic desktop E2E test in `e2e/auth.spec.ts` or a new feature spec. Tests run the production Tauri commands, login state machine, response handling, and events in a native Windows app. Only calls to Telegram receive fixture responses; no real Telegram credentials are required. The E2E app uses a separate application identifier and real SQLite session storage, so it cannot touch the production session.

| Feature flow                  | E2E scenario                                                                    |
| ----------------------------- | ------------------------------------------------------------------------------- |
| QR login and token refresh    | Shows a QR token, updates it, and opens the connected page after authentication |
| Phone login                   | Requests a code, submits it, and opens the connected page                       |
| Immediate phone authorization | Opens the connected page when Telegram authorizes without a code                |
| Two-step verification         | Completes password challenges after phone or QR login                           |
| Change phone number           | Returns from code entry to phone entry                                          |
| Persistent session            | Opens the connected page from an authorized session                             |
| Unauthorized saved session    | Redirects the connected route to login                                          |
| Session lookup failure        | Shows the error and keeps login available                                       |
| Logout                        | Disconnects and returns to login                                                |
| Telegram failure and retry    | Shows an error and retries QR login                                             |
| QR cancellation               | Switches to phone login and stops QR polling                                    |
| Phone code failure            | Shows request and invalid-code errors while keeping the form available          |
| Logout failure                | Shows the error and keeps the connected page                                    |

Run locally with `VITE_E2E=1 pnpm tauri build --debug --no-bundle --features e2e --config src-tauri/tauri.e2e.conf.json` (set the variable using your shell syntax), then `pnpm test:e2e`. The E2E feature and capabilities are omitted from production builds.

`pnpm test:rust:coverage` uses `scripts/rust-coverage.mjs` to combine unit tests and an instrumented native E2E run, then enforces 96% measured production Rust line coverage. The Node.js runner sets build variables itself and works independently of the terminal shell. It handles executable names on Windows, Linux, and macOS; native desktop E2E remains verified on Windows only. `pnpm check` includes that gate. The report excludes test files and the feature-gated Telegram response fixture; all production Rust files remain included. Native profiles are flushed before WebDriver teardown. HTML reports are written under `src-tauri/target/llvm-cov-target/llvm-cov/html`.

The saved-session desktop scenario verifies startup routing from Telegram's authorization response. A Rust test separately verifies SQLite session data survives closing and reopening the file; deterministic tests do not establish a live Telegram connection.
