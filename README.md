# Varde

A private, local-only desktop app for tracking personal wealth. It covers **employment income** and **investments**, and more areas will follow as their specs are approved.

## What it does today

- **Dashboard:** your income this year to date and last month (total gross, gross salary, holiday pay, bonus and number of entries), summed over all jobs. An annual statement counts in full in this year and not in last month.
- **Income:** any number of jobs. A monthly job has one paycheck per month, a yearly job has one annual statement per year, both with gross salary, holiday pay and bonus. Yearly totals over all jobs and a list of all paychecks and annual statements sit next to the jobs. An **Insights** tab charts income per year, with the year-over-year growth.
- **Investments:** any number of investment accounts, each recorded monthly or yearly. A statement is the closing balance of one period, the net deposits of that period (money put in minus money taken out) and an optional note. The capital growth of each statement is derived from the balance, the previous statement and the net deposits, and the form calculates the balance or the growth from the other. Each account shows its latest balance, the total of the latest balances is shown over all accounts, and yearly totals show the closing balance, net deposits and capital growth per year. An **Insights** tab charts each year as opening balance, net deposits and capital growth with a closing-balance marker and the growth as a percentage of the opening balance. Individual stocks, prices and trades are not tracked.
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

The frontend only displays what Rust returns. It never computes financial figures. How the code is organised, and why, is in [docs/architecture.md](docs/architecture.md).

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
ui/                 frontend (HTML, CSS, JS; no build step)
  index.html
  main.js           entry point: wires the pages and reloads them
  navigation.js     left-pane navigation
  pages/            one file per page or view (income-overview, income-insights, ...)
  shared/           helpers pages share (ipc, dom, format, chart, yearly-chart, tabs)
  styles/           CSS by area (base, layout, components, pages, charts)
src-tauri/
  src/lib.rs        app setup, command registration
  src/menu.rs       native macOS menu bar (File menu, Settings…)
  src/commands/     thin #[tauri::command] handlers
  src/domain/       pure business logic (money, income, investments, settings), unit-tested
  src/storage/      project file persistence (JSON, atomic write)
  src/error.rs      app error type
  capabilities/     Tauri permission files
docs/
  architecture.md   how the code is organised and the conventions it follows
  csd/              capability specifications
```

### Specifications

Behaviour is defined in Capability Specification Documents in [`docs/csd/`](docs/csd/). Start with [CSD-000](docs/csd/CSD-000-capability-specifications.md), which describes the format (EARS requirements, Gherkin acceptance criteria).

| CSD | Capability |
|---|---|
| [CSD-001](docs/csd/CSD-001-employment-income.md) | Employment income tracking |
| [CSD-002](docs/csd/CSD-002-project-file.md) | Project file (save, open, new, reopen last project) |
| [CSD-003](docs/csd/CSD-003-project-settings.md) | Project settings (currency) |
| [CSD-004](docs/csd/CSD-004-income-dashboard.md) | Income dashboard |
| [CSD-005](docs/csd/CSD-005-investments.md) | Investments (accounts, statements, net deposits, capital growth, yearly totals) |
| [CSD-006](docs/csd/CSD-006-income-insights.md) | Income insights chart |
| [CSD-007](docs/csd/CSD-007-investment-insights.md) | Investment insights chart |
| [CSD-008](docs/csd/CSD-008-application-preferences.md) | Application preferences (theme) |

Only specs with `status: approved` are implemented; check a spec's status before relying on it. Tests reference acceptance criteria in their names, for example `test_csd002_ac_1_1_...`.

### Contributing

Work on the `development` branch and open a PR from it into `main`; nothing is pushed to `main` directly. Use Conventional Commit messages (`feat:`, `fix:`, `refactor:`, `chore:`). See [CLAUDE.md](CLAUDE.md) for the full project rules.

The repo is public, so fixtures and test data must be obviously fake (for example "Example Bank", round numbers).

## License

No license has been chosen yet.
