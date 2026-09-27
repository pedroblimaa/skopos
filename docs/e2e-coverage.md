# Desktop E2E coverage

Every implemented user flow needs a deterministic desktop E2E test in `e2e/auth.spec.ts` or a new feature spec. Tests mock Telegram commands and events at the Tauri IPC boundary, run against a native Windows app, and require no real Telegram credentials.

| Feature flow               | E2E scenario                                                                    |
| -------------------------- | ------------------------------------------------------------------------------- |
| QR login and token refresh | Shows a QR token, updates it, and opens the connected page after authentication |
| Phone login                | Requests a code, submits it, and opens the connected page                       |
| Two-step verification      | Receives a password challenge and completes login                               |
| Persistent session         | Opens the connected page from an authorized session                             |
| Logout                     | Disconnects and returns to login                                                |
| Telegram failure and retry | Shows an error and retries QR login                                             |
| QR cancellation            | Switches to phone login and stops QR polling                                    |
| Phone code failure         | Shows request and invalid-code errors while keeping the form available          |
| Logout failure             | Shows the error and keeps the connected page                                    |

Run locally with `VITE_E2E=1 pnpm tauri build --debug --no-bundle --features e2e --config src-tauri/tauri.e2e.conf.json` (set the variable using your shell syntax), then `pnpm test:e2e`. The E2E feature and capabilities are omitted from production builds.
