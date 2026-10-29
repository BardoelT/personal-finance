# Varde

A private, local-only desktop app for tracking personal wealth: what comes in, what goes out, what you own and what you owe. Each area has a table view and an **Insights** tab with a yearly chart.

## What it does today

The left pane groups the pages as **Overview**, **Finance**, **Wealth**, **Liabilities** and **Insights** (the Net worth page), with Settings and the project status at the bottom.

- **Dashboard:** your income this year to date and last month (total gross, gross salary, holiday pay and bonus), summed over all jobs. An annual statement counts in full in this year and not in last month.
- **Income:** any number of jobs. A monthly job has one paycheck per month, a yearly job has one annual statement per year, both with gross salary, holiday pay and bonus. Yearly totals and a list of all paychecks and annual statements sit next to the jobs. Insights charts income per year with the year-over-year growth.
- **Expenses:** dated expenses with a name, amount, category (a fixed list) and a note, with yearly totals and the largest expense of each year. Insights charts the spending per year, stacked by category.
- **Bank accounts:** any number of accounts, each recorded monthly or yearly as a closing balance (negative when overdrawn) and a note. Yearly totals and the total of the latest balances. Insights charts the balance per bank per year with a closing-balance marker.
- **Investments:** any number of investment accounts, each recorded monthly or yearly. A statement holds the closing balance, the net deposits (money put in minus money taken out) and a note; the capital growth is derived from them, and the form calculates the balance or the growth from the other. Yearly totals show closing balance, net deposits and capital growth. Insights charts opening balance, net deposits and capital growth with a closing-balance marker and the growth percentage. Individual stocks, prices and trades are not tracked.
- **Assets:** things you own (a house, a car), each valued monthly or yearly, with the change since the previous valuation and yearly values. Insights charts the value per asset per year.
- **Debts:** what you owe, each recorded monthly or yearly as a remaining balance, with the amount repaid per year. Insights charts the debt per debt per year.
- **Net worth:** one chart of your overall financial state: a stacked bar per year with assets, investments and cash above the zero line and debts below it, and a marker at the net worth. Every figure is the one the chart of its own area shows.
- **Settings:** the currency (EUR, USD or GBP) of the whole project, and the theme (System, Light or Dark). Changing the currency does not convert any amount.
- **Project file:** everything is saved in one file you choose, and the app reopens the last one you used.

All insights charts look and behave alike: hovering a legend item highlights that series in every year, and the tooltip lists its rows in the same order as the bar.


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
  src/domain/       pure business logic (money, income, expenses, bank accounts, investments, assets, debts, net worth, settings), unit-tested
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
| [CSD-001](docs/csd/CSD-001-employment-income.md) | Income tracking (employment income) |
| [CSD-002](docs/csd/CSD-002-project-file.md) | Project file (save, open, new, reopen last project) |
| [CSD-003](docs/csd/CSD-003-project-settings.md) | Project settings (currency) |
| [CSD-004](docs/csd/CSD-004-income-dashboard.md) | Income dashboard and the structure of the left pane |
| [CSD-005](docs/csd/CSD-005-investments.md) | Investments (accounts, statements, net deposits, capital growth, yearly totals) |
| [CSD-006](docs/csd/CSD-006-income-insights.md) | Income insights chart |
| [CSD-007](docs/csd/CSD-007-investment-insights.md) | Investment insights chart |
| [CSD-008](docs/csd/CSD-008-application-preferences.md) | Application preferences (theme) |
| [CSD-009](docs/csd/CSD-009-bank-accounts.md) | Bank accounts |
| [CSD-010](docs/csd/CSD-010-bank-account-insights.md) | Bank account insights chart |
| [CSD-011](docs/csd/CSD-011-debts.md) | Debts |
| [CSD-012](docs/csd/CSD-012-debt-insights.md) | Debt insights chart |
| [CSD-013](docs/csd/CSD-013-expenses.md) | Expenses |
| [CSD-014](docs/csd/CSD-014-expense-insights.md) | Expense insights chart |
| [CSD-015](docs/csd/CSD-015-assets.md) | Assets |
| [CSD-016](docs/csd/CSD-016-asset-insights.md) | Asset insights chart |
| [CSD-017](docs/csd/CSD-017-net-worth-insights.md) | Net worth insights (the Net worth page) |

Only specs with `status: approved` are implemented; check a spec's status before relying on it. Tests reference acceptance criteria in their names, for example `test_csd002_ac_1_1_...`.

### Contributing

Work on the `development` branch and open a PR from it into `main`; nothing is pushed to `main` directly. Use Conventional Commit messages (`feat:`, `fix:`, `refactor:`, `chore:`). See [CLAUDE.md](CLAUDE.md) for the full project rules.

The repo is public, so fixtures and test data must be obviously fake (for example "Example Bank", round numbers).

## License

No license has been chosen yet.
