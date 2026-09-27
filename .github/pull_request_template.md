## Behavior

- [ ] Describe the user behavior changed and relevant failure paths.
- [ ] Add or update a desktop E2E test for each affected feature flow; update `docs/e2e-coverage.md`.
- [ ] Add unit or integration tests for new frontend and Rust behavior.

## Verification

- [ ] `pnpm check` passes, including both 96% line coverage gates.
- [ ] `pnpm test:e2e` passes against the E2E desktop build.
- [ ] Review changes to authentication, local session data, dependencies, and Tauri permissions for security impact.
