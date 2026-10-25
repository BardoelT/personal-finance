# CLAUDE.md

Instructions for Claude when working in this repository. Keep this file short and current.
If you (Claude) make the same mistake twice, propose an addition here.

## Project

Personal wealth tracking desktop app: accounts, assets, transactions and net-worth over time.

- **Stack:** Rust + Tauri v2. Frontend: vanilla HTML + CSS + JS (no framework, no build step) in `ui/`, Rust backend in `src-tauri/`.
- **Platform:** macOS only (Apple Silicon + Intel). Do not add Windows/Linux code paths or CI targets unless asked.
- **Data:** stored **locally only** (SQLite via `rusqlite` or `sqlx` — not yet wired up; see Ask first). No cloud sync, no backend server.
- **Repo:** public on GitHub. Assume everything committed is visible to the world.

## Commands

```bash
cargo tauri dev                          # run the app locally
cargo tauri build                        # release build (.app / .dmg)
cargo fmt --all                          # format
cargo clippy --all-targets --all-features -- -D warnings   # lint (must be clean)
cargo test --all-features                # all tests
cargo test <name>                        # single test
cargo deny check                         # licenses, advisories, banned crates
```

No npm / frontend build step — `ui/` is served as-is by Tauri.

Before saying a task is done: fmt, clippy, and tests must all pass.

## Layout

```
ui/                   # frontend (UI only — HTML + CSS + JS, no business logic)
  index.html
  main.js
  style.css
src-tauri/
  src/main.rs         # thin entry point
  src/lib.rs          # app setup, command registration
  src/commands/       # #[tauri::command] handlers: validate input, call domain, map errors  (not yet created)
  src/domain/         # pure business logic (money, portfolios, net worth): no Tauri, no IO  (not yet created)
  src/storage/        # persistence + migrations                                              (not yet created)
  src/error.rs        # app error type                                                        (not yet created)
  capabilities/       # Tauri permission files
  tauri.conf.json
docs/                 # architecture notes and ADRs                                           (not yet created)
```

Rule: `domain/` is pure and unit-tested. Commands stay thin. The frontend never computes financial figures; it only displays what Rust returns.

## Rust rules

### Safety
- **No `unsafe`.** Enforced via `Cargo.toml`:
  ```toml
  [lints.rust]
  unsafe_code = "forbid"
  ```
  Do not work around this (no `unsafe` in macros, no new dependencies whose main purpose is FFI without asking).
- No `unwrap()`, `expect()`, `panic!`, `todo!`, `unimplemented!` or indexing that can panic in non-test code. Return `Result`.
- No `as` casts between numeric types where truncation is possible; use `TryFrom`/`try_into()`.

### Lints
Keep this in `Cargo.toml` and don't silence lints without a comment explaining why:
```toml
[lints.clippy]
pedantic = { level = "warn", priority = -1 }
unwrap_used = "deny"
expect_used = "deny"
panic = "deny"
indexing_slicing = "warn"
float_arithmetic = "warn"
module_name_repetitions = "allow"
```
`#[allow(...)]` must be scoped as narrowly as possible and include a reason: `#[allow(clippy::x, reason = "...")]`.

### Money and numbers
- **Never use `f32`/`f64` for money.** Use `rust_decimal::Decimal` (or integer minor units) everywhere: domain, storage, and IPC.
- Every amount carries its currency (`Money { amount: Decimal, currency: Currency }`). Never add amounts with different currencies without an explicit conversion.
- Rounding is explicit and documented (banker's rounding unless stated otherwise).
- Dates: use `time` or `chrono` with explicit time zones; store UTC, display local.

### Errors
- Library-style errors with `thiserror` in `domain/` and `storage/`; one `AppError` in `error.rs`.
- Errors returned to the frontend are serialized to a safe message and code. Never leak file paths, SQL or stack traces to the UI.

### Style
- Prefer newtypes over primitive obsession (`AccountId(Uuid)`, not `String`).
- Make invalid states unrepresentable (enums over boolean flags).
- Small functions; doc comments (`///`) on all public items in `domain/`.
- Async only where IO requires it; don't block the main thread in commands.

### Dependencies
- Ask before adding a new crate. Prefer well-maintained, widely used crates.
- Must pass `cargo deny check` (licenses: MIT/Apache-2.0/BSD-compatible).
- Commit `Cargo.lock`.

## Tauri security
- **Least privilege capabilities.** Only enable the permissions a feature needs in `src-tauri/capabilities/`. Do not add `shell`, `fs` with broad scopes, or `http` plugins without asking.
- The CSP in `tauri.conf.json` is currently `null` — set a strict policy before any release build (no `unsafe-eval`, no remote scripts). Load no remote content.
- Treat every `#[tauri::command]` input as untrusted: validate and bound it before use.
- No network calls at all unless explicitly requested (e.g. a future exchange-rate fetch must be opt-in and documented).
- Don't use `dangerousDisableAssetCspModification` or similar `dangerous*` options.

## Privacy and the public repo
This app handles personal financial data and the repo is public.
- **Never commit real financial data.** Fixtures and test data must be obviously fake (e.g. "Example Bank", round numbers).
- **No secrets in the repo.** Signing certificates, notarization credentials and API keys live only in GitHub Actions secrets. Never print them in logs.
- No telemetry, analytics or crash reporting that sends data off the machine.
- Don't log amounts, account names or balances at `info` level or above.
- Don't read or modify `.env*`, keychains, or anything outside the project directory.

## Testing
- Unit tests for all `domain/` logic, especially money math, currency handling and edge cases (zero, negative, very large values, rounding).
- Use property tests (`proptest`) for arithmetic invariants where useful.
- Storage tests run against a temporary database, never the real app data dir.
- Bug fix = failing test first, then the fix.
- Don't delete or weaken existing tests to make a change pass.

## Git and PRs
- Work on a branch; never push to `main`. All changes go through a PR.
- Small, focused PRs with Conventional Commit messages (`feat:`, `fix:`, `refactor:`, `chore:`).
- PR description: what changed, why, and how it was tested.
- CI (GitHub Actions, `macos-latest`) must be green: fmt, clippy, tests, `cargo deny`.

## Specifications (CSD)
- All capability specs (CSDs) live in `docs/csd/`. Read `docs/csd/CSD-000-capability-specifications.md` before reading, writing, or implementing any CSD.
- Requirements use EARS, acceptance criteria use Gherkin (tagged @AC-n.m), and flows use Mermaid.
- Before changing behaviour, find and read the governing CSD; only implement CSDs with `status: approved`.
- Never build what a CSD lists as a non-goal. Ask when the spec is ambiguous about behaviour or interfaces.
- Reference REQ-/AC- IDs in tests (test name: `test_ac_n_m_…`), commits, and PR descriptions.

## Ask first
Stop and ask before:
- adding dependencies or Tauri plugins/permissions
- choosing the SQLite crate and writing the first migration / schema
- touching `.github/workflows/`, signing/notarization config, or `tauri.conf.json` security settings
- adding any network access
- large refactors across modules
