# Varde

A private, local-only desktop app for tracking personal wealth. It starts with **employment income**, and more areas will follow as their specs are approved.

## What it does today

- **Dashboard:** your income this year to date and last month (total gross, gross salary, holiday pay, bonus and number of entries), summed over all jobs. An annual statement counts in full in this year and not in last month.
- **Income:** any number of jobs. A monthly job has one paycheck per month, a yearly job has one annual statement per year, both with gross salary, holiday pay and bonus. Yearly totals over all jobs and a list of all paychecks and annual statements sit next to the jobs.
- **Settings:** project settings. The first one is the currency (EUR, USD or GBP) of the whole project. Changing it does not convert any amount.
- **Project file:** everything is saved in one file you choose, and the app reopens the last one you used.

The Reports page is a placeholder for a later feature.

## Principles

- **Local only.** All data lives in one project file you choose. There is no cloud sync, no backend server, no telemetry and no network access.
- **macOS only** (Apple Silicon and Intel).
- **Exact money math.** Amounts use `rust_decimal`, never floats, and are in the one currency of the project.
- **Safe saving.** Files are written to a temporary file and renamed over the target, so a failed save never damages an existing project.

## Tech stack

- Rust + [Tauri v2](https://tauri.app/) backend in `src-tauri/`
- Vanilla HTML, CSS and JS frontend in `ui/` (no framework, no build step)

The frontend only displays what Rust returns. It never computes financial figures.

## Getting started

Prerequisites: a recent stable [Rust toolchain](https://rustup.rs/) and the Tauri CLI.

```bash
cargo install tauri-cli --version "^2"
```

Run the app in development:

```bash
cargo tauri dev
```

Build a release bundle (`.app` / `.dmg`):

```bash
cargo tauri build
```

There is no npm or frontend build step. `ui/` is served as-is.

## Using the app

- **Project actions** live in the native macOS **File** menu: New (⌘N), Open… (⌘O), Save (⌘S) and Save As… (⇧⌘S). An indicator at the bottom of the left pane shows unsaved changes, and the app asks before you quit or replace a project with unsaved edits. On start, the app reopens the last project you used.
- **Settings** open from the Settings item at the bottom of the left pane, or from the app menu with ⌘,.
- **Dates** are shown and typed as `dd/mm/yyyy`, and months as `YYYY-MM`.

> **Project files hold real financial data** and are plain, unencrypted JSON. `*.wealth` is git-ignored. Never commit one.

## Development

```bash
cargo fmt --all
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-features
cargo deny check
```

Format, clippy and tests must pass before a change is done. CI runs the same checks on `macos-latest`.

### Layout

```
ui/                 frontend (HTML, CSS, JS)
src-tauri/
  src/lib.rs        app setup, command registration
  src/menu.rs       native macOS menu bar (File menu, Settings…)
  src/commands/     thin #[tauri::command] handlers
  src/domain/       pure business logic (money, income, settings), unit-tested
  src/storage/      project file persistence (JSON, atomic write)
  src/error.rs      app error type
  capabilities/     Tauri permission files
docs/csd/           capability specifications
```

### Specifications

Behaviour is defined in Capability Specification Documents in [`docs/csd/`](docs/csd/). Start with [CSD-000](docs/csd/CSD-000-capability-specifications.md), which describes the format (EARS requirements, Gherkin acceptance criteria).

| CSD | Capability |
|---|---|
| [CSD-001](docs/csd/CSD-001-employment-income.md) | Employment income tracking |
| [CSD-002](docs/csd/CSD-002-project-file.md) | Project file (save, open, new, reopen last project) |
| [CSD-003](docs/csd/CSD-003-project-settings.md) | Project settings (currency) |
| [CSD-004](docs/csd/CSD-004-income-dashboard.md) | Income dashboard |

Only specs with `status: approved` are implemented; check a spec's status before relying on it. Tests reference acceptance criteria in their names, for example `test_csd002_ac_1_1_...`.

### Contributing

Work on a branch and open a PR; nothing is pushed to `main` directly. Use Conventional Commit messages (`feat:`, `fix:`, `refactor:`, `chore:`). See [CLAUDE.md](CLAUDE.md) for the full project rules.

The repo is public, so fixtures and test data must be obviously fake (for example "Example Bank", round numbers).

## License

No license has been chosen yet.
