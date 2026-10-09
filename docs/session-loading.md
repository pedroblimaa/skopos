# Session validation and local preferences

## Implementation plan

1. Share one session validation when restoring the app session; keep the resulting status in memory. Refresh it when QR, phone, or password login completes.
2. Resolve account-scoped local reads from the known session and cached/persisted account identity. Remove repeated authorization requests from notification settings and automatic monitoring polls.
3. Detect session revocation in the Telegram adapter from real operation responses. Invalidate the session snapshot, cancel searches and notifications, suspend monitoring, and emit the existing authentication event. Preserve generation checks so an old response cannot revoke a newer session.
4. Preload notification settings, delivery status, monitoring status, and startup preferences in AppShell after authentication. Preserve these caches across dialog closure and navigation; dispose them when authentication ends.
5. Load notification preferences independently of delivery status. Keep monitoring and delivery status current through events, rejecting initial reads superseded by a newer event.

## Review and verification

Authentication changes require human review under AGENTS.md. The user authorized this implementation plan; review the resulting login, sign-out, and revocation behavior before handoff.

After acceptance, regression tests were added for concurrent restoration, login refresh, session revocation, stale RPC responses, cache disposal, late completions, account filtering, independent preference loading, dialog reopening, animated exit, and reduced motion.

Local verification on 2026-10-08:

- `pnpm check:local` passed formatting, lint, TypeScript, and all 362 frontend tests.
- The isolated desktop fixture build passed. Authentication (20 scenarios), monitoring (2 scenarios), and 11 notification scenarios passed in the first targeted desktop run.
- The notification UI scenario failed because the embedded WebDriver dispatches a synthetic `mousemove` instead of moving the native pointer. This cannot activate CSS `:hover`. The scenario now focuses the info trigger and then the Close button, asserting both focus transitions and tooltip visibility. The focused scenario passes, including independent switches, narrow layout, and reopening without loading. Native pointer hover remains a manual visual check.
- The Windows loader failure was reproduced in CI and isolated locally: the test harness imported `TaskDialogIndirect` without a manifest selecting Common Controls v6. Embedding the app's existing manifest in a copy changed `--list` from `STATUS_ENTRYPOINT_NOT_FOUND` (`0xc0000139`) to successful enumeration of 126 tests. The build now embeds that same manifest through MSVC linker arguments for application and test targets. All 126 instrumented Rust tests pass after updating the sign-out fixture to refresh the cached session when simulating a new login.
- After the loader correction, `pnpm lint:rust` passed in normal and E2E configurations, the fixture app rebuilt in 36 seconds, and all 22 authentication/monitoring desktop scenarios passed (44.9 seconds including runner setup). `pnpm check:local` also passed again. The manifest matches Tauri's existing Common Controls v6 dependency; see [Microsoft's TaskDialogIndirect requirements](https://learn.microsoft.com/en-us/windows/win32/api/commctrl/nf-commctrl-taskdialogindirect) and [Cargo linker directives](https://doc.rust-lang.org/cargo/reference/build-scripts.html#rustc-link-arg).
- Full measured TypeScript/Rust coverage and production-build verification remain CI gates. No thresholds were changed.

The first desktop build took 84.6 seconds; the first targeted desktop run took 56 seconds. The original local checks took about 23 seconds. After the manifest correction, the instrumented Rust unit stage passed in 51 seconds. Full combined coverage requires rebuilding the fixture app and running the complete desktop suite before reporting coverage.

The first opening can still show loading if it happens before the initial local preload completes. Subsequent openings reuse the loaded preferences. Temporary network failures do not invalidate an already validated session; explicit authorization rejection does.
