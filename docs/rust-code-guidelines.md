# Rust code guidelines for Skopos

Use this guide when adding or reviewing code in `src-tauri`. It applies to the current Telegram login code and to future Rust features. Prefer code that makes state changes, network calls, and failure paths easy to follow. These are review rules, not a mandate to create a layer for every function.

## Organize by responsibility

- Keep `lib.rs` focused on Tauri setup: state registration, command registration, and application startup.
- Use a feature entry file such as `telegram.rs` for module declarations and selective re-exports. Rust also supports `telegram/mod.rs`, but the named-file form makes module tabs easier to distinguish. Keep session, protocol, and command workflows in responsibility-named files rather than the entry file.
- Follow the separation shown by Tauri's official plugin layout when useful: setup and exports in the entry file, commands at the Tauri boundary, and implementation in feature modules. Apply the responsibilities, not a fixed file template for every feature.
- Keep Tauri commands thin. A command should validate its input, call the feature operation, and translate the result into a stable value for the frontend. It should not own a long Telegram protocol workflow.
- Put Telegram calls and Telegram-specific types in the Telegram adapter. Keep future parsing and matching code independent of `grammers` types, as described in `AGENTS.md`.
- Put session file access in one small storage or client-initialization area. Avoid opening a database, starting a client, emitting events, and changing login state in the same function.
- Split a module when it contains distinct responsibilities or several workflows that can be understood separately. Do not split files just to satisfy a line-count target. Use narrow visibility (`pub(super)` or private) until another module needs an item.
- Name modules for the feature or responsibility (`auth`, `session`, `qr`, `phone`), and functions for actions (`request_code`, `complete_login`). Follow Rust's usual `snake_case` and `UpperCamelCase` conventions.

For the current login code, a reasonable _direction_ is a small command boundary, a login coordinator/state type, Telegram protocol operations, and session persistence. Add these boundaries as a refactor needs them; do not create empty modules or generic frameworks in advance.

## Keep control flow flat

- Return early for invalid input, missing state, cancellation, and unsupported Telegram responses. Let the successful path remain visible.
- Use `?` to propagate errors when the caller can handle them. Match explicitly when a Telegram response changes the login step, requires migration, or merits a distinct user message.
- Extract a helper when it names a meaningful operation or isolates a complex branch. A helper that merely hides three lines without clarifying ownership is unnecessary.
- Model mutually exclusive login steps with an enum when several `Option` fields or booleans can represent impossible combinations. Make transitions explicit, including cleanup after failure, cancellation, sign-out, and successful login.
- Keep protocol retries bounded and visible. For example, a data-center migration should have an explicit retry path rather than recursive calls with an unclear stopping condition.

Example of a command boundary (illustrative, not a required new abstraction):

```rust
#[tauri::command]
async fn submit_code(
    state: tauri::State<'_, LoginService>,
    code: String,
) -> Result<LoginOutcome, CommandError> {
    let code = code.trim();
    if code.is_empty() {
        return Err(CommandError::MissingCode);
    }

    state.submit_code(code).await.map_err(CommandError::from)
}
```

The service owns the transition; the command does not reproduce it. Use project types and Tauri serialization requirements when implementing this pattern.

## Handle errors at the right boundary

- Use `Result` for expected failures: invalid code or password, expired token, migration failure, network failure, rate limit, missing credentials, and storage failure. Reserve `panic!`, `unwrap`, and `expect` for invariants that truly cannot fail in a running app.
- Preserve structured errors inside Rust. Convert them to user-facing messages at the Tauri boundary. Avoid passing raw Telegram or SQLite errors to the UI, and avoid turning every internal error into `String` immediately.
- Match typed `grammers` error variants or error codes when available. Avoid broad substring checks of `Display` text for decisions; that text can change and can accidentally match an unrelated error.
- Provide context for operational failures without logging login codes, passwords, QR tokens, session contents, or the API hash.
- Keep errors actionable: say whether the user should retry, request a new code, wait for a rate limit, check connectivity, or configure credentials and rebuild.

## Make async state and ownership clear

- Keep lock scopes short. Clone a cheap client handle or take an owned value while locked, release the guard, then perform network or file I/O. Reacquire the lock only to commit the result, checking that the operation is still current.
- Use a Tokio mutex when a lock genuinely must span an `.await`; do not use it as a shortcut for an unclear state transition. Consider a dedicated task with messages if one long-running workflow owns the Telegram client.
- Do not emit events while holding the login-state lock. A command and a background QR task may race; use a generation ID or another explicit cancellation mechanism so an old task cannot publish a new token or overwrite a newer login step.
- Give every spawned task a clear owner and stop condition. Handle task errors instead of silently discarding them, except for explicitly best-effort notifications.
- Keep `async` for I/O and orchestration. Keep validation, response classification, and state-transition decisions in small synchronous functions where possible; those are easier to test.

## Test behavior and review changes

- Unit test pure validation, Telegram response classification, error mapping, and state transitions. Test the boundaries between QR, phone code, password, cancellation, restoration, and sign-out.
- For code that uses Telegram, isolate the protocol boundary enough to test outcomes without a real account. Add an abstraction only when it removes a concrete testing or coupling problem.
- Review the unhappy paths: missing `.env` credentials, expired QR, interrupted request, concurrent login attempts, network loss, migration, invalid code/password, rate limit, failed persistence, and sign-out.
- Run `cargo fmt`, Clippy, and tests through the repository's `pnpm check` script. Run `pnpm tauri build --no-bundle` when a change affects native build behavior.
- Treat Clippy's complexity and length warnings as prompts to inspect a function, not as hard architecture rules. Avoid suppressing a lint without a local reason.

## Avoid these patterns

- A feature entry file (`feature.rs` or `feature/mod.rs`) that mixes state, session setup, protocol calls, Tauri commands, and UI error text.
- A command that holds the shared login lock while awaiting Telegram, SQLite, a timer, or a Tauri event.
- Nested `if` and `match` blocks that obscure the normal path when a guard clause or a named operation would clarify it.
- Several booleans and optional tokens whose combinations are ambiguous.
- `Result<T, String>` throughout the internal implementation, raw upstream errors shown directly to the user, or `expect` on recoverable state.
- Fire-and-forget tasks with no cancellation or error reporting; ignored event-emission failures without an explicit best-effort decision.
- Extra traits, crates, or layers introduced solely because an architecture diagram suggests them.

## Sources

These rules adapt primary documentation to Skopos; they are not verbatim requirements from the sources.

- [The Rust Programming Language: refactoring for modularity and error handling](https://doc.rust-lang.org/book/ch12-03-improving-error-handling-and-modularity.html)
- [The Rust Programming Language: modules in separate files](https://doc.rust-lang.org/book/ch07-05-separating-modules-into-different-files.html)
- [The Rust Reference: module file naming](https://doc.rust-lang.org/reference/items/modules.html)
- [The Rust Programming Language: recoverable errors with `Result`](https://doc.rust-lang.org/book/ch09-02-recoverable-errors-with-result.html)
- [Rust API Guidelines: naming](https://rust-lang.github.io/api-guidelines/naming.html) and [error types](https://rust-lang.github.io/api-guidelines/interoperability.html)
- [Tokio tutorial: shared state and lock scopes](https://tokio.rs/tokio/tutorial/shared-state)
- [Tauri: calling Rust from the frontend](https://v2.tauri.app/develop/calling-rust/)
- [Tauri: official plugin layout](https://v2.tauri.app/develop/plugins/)
- [Clippy lint configuration](https://doc.rust-lang.org/clippy/lint_configuration.html)
