//! Calls the real commands through Tauri's IPC layer on the mock runtime, the
//! same way the frontend does: command names, camelCase argument names and
//! the JSON shapes of the responses are the contract with `ui/`.

use serde_json::{json, Value};
use tauri::test::{get_ipc_response, mock_builder, mock_context, noop_assets, MockRuntime};
use tauri::webview::InvokeRequest;
use tauri::{ipc::InvokeBody, App, Manager, WebviewWindow, WebviewWindowBuilder};
use time::UtcOffset;

use crate::configure;
use crate::storage::Preferences;

struct Harness {
    window: WebviewWindow<MockRuntime>,
    _app: App<MockRuntime>,
}

fn harness() -> Harness {
    let app = configure(mock_builder(), UtcOffset::UTC)
        .build(mock_context(noop_assets()))
        .unwrap();
    let window = WebviewWindowBuilder::new(&app, "main", tauri::WebviewUrl::default())
        .build()
        .unwrap();
    Harness { window, _app: app }
}

/// An app whose preference file lives in `config_dir`, as `run` sets it up.
fn harness_in(config_dir: &std::path::Path) -> Harness {
    let app = configure(
        mock_builder().manage(Preferences::in_dir(config_dir)),
        UtcOffset::UTC,
    )
    .build(mock_context(noop_assets()))
    .unwrap();
    let window = WebviewWindowBuilder::new(&app, "main", tauri::WebviewUrl::default())
        .build()
        .unwrap();
    Harness { window, _app: app }
}

impl Harness {
    fn call(&self, command: &str, args: Value) -> Result<Value, Value> {
        let request = InvokeRequest {
            cmd: command.into(),
            callback: tauri::ipc::CallbackFn(0),
            error: tauri::ipc::CallbackFn(1),
            url: "tauri://localhost".parse().unwrap(),
            body: InvokeBody::Json(args),
            headers: tauri::http::HeaderMap::default(),
            invoke_key: tauri::test::INVOKE_KEY.to_string(),
        };
        get_ipc_response(&self.window, request).map(|body| body.deserialize::<Value>().unwrap())
    }

    fn ok(&self, command: &str, args: Value) -> Value {
        self.call(command, args)
            .unwrap_or_else(|error| panic!("{command} failed: {error}"))
    }

    fn err(&self, command: &str, args: Value) -> Value {
        self.call(command, args)
            .expect_err(&format!("{command} should have failed"))
    }
}

fn job_input() -> Value {
    json!({
        "employer": "Example Employer",
        "title": "",
        "start_date": "01/03/2024",
        "end_date": ""
    })
}

fn amounts(gross: &str, holiday: &str, bonus: &str) -> Value {
    json!({ "gross_salary": gross, "holiday_pay": holiday, "bonus": bonus })
}

#[test]
fn income_flow_through_ipc_matches_what_the_frontend_expects() {
    let app = harness();
    assert_eq!(app.ok("list_jobs", json!({})), json!([]));
    assert_eq!(
        app.ok("project_status", json!({})),
        json!({ "unsaved_changes": false, "has_path": false })
    );

    // Create a job exactly as ui/pages/income-overview.js submits it (empty strings for blanks).
    let job = app.ok("create_job", json!({ "input": job_input() }));
    let job_id = job["id"].as_str().unwrap().to_owned();
    assert_eq!(job["employer"], "Example Employer");
    assert_eq!(job["title"], Value::Null);
    assert_eq!(job["start_date"], "2024-03-01");
    assert_eq!(job["end_date"], Value::Null);
    // Jobs carry no currency: the project has one (CSD-003).
    assert!(job.get("currency").is_none());
    assert_eq!(job["entry_count"], 0);
    assert_eq!(app.ok("project_status", json!({}))["unsaved_changes"], true);

    // Paychecks: arguments use camelCase `jobId`, amounts are strings.
    let added = app.ok(
        "add_paycheck",
        json!({ "jobId": job_id, "month": "2025-12", "amounts": amounts("3000.00", "", "1000.00") }),
    );
    assert_eq!(added["period"], "2025-12");
    assert_eq!(added["kind"], "paycheck");
    assert_eq!(added["gross_salary"], "3000.00");
    assert_eq!(added["holiday_pay"], "0.00");
    assert_eq!(added["bonus"], "1000.00");
    assert_eq!(added["total_gross"], "4000.00");
    app.ok(
        "add_paycheck",
        json!({ "jobId": job_id, "month": "2026-05", "amounts": amounts("3000", "2400", "") }),
    );

    let paychecks = app.ok("list_entries", json!({}));
    let months: Vec<&str> = paychecks
        .as_array()
        .unwrap()
        .iter()
        .map(|p| p["period"].as_str().unwrap())
        .collect();
    assert_eq!(months, ["2026-05", "2025-12"], "newest first");
    assert_eq!(paychecks[0]["total_gross"], "5400.00");
    // The paychecks table needs the job without another round trip.
    assert_eq!(paychecks[0]["employer"], "Example Employer");
    assert!(paychecks[0].get("currency").is_none());
    assert_eq!(paychecks[0]["job_id"], job_id);
    assert_eq!(added["employer"], "Example Employer");

    let totals = app.ok("yearly_totals", json!({}));
    assert_eq!(totals[0]["year"], 2026);
    assert_eq!(totals[0]["total_gross"], "5400.00");
    assert_eq!(totals[0]["entry_count"], 1);
    assert_eq!(totals[1]["year"], 2025);
    assert_eq!(totals[1]["bonus"], "1000.00");

    let prefill = app.ok("copy_previous_entry", json!({ "jobId": job_id }));
    assert_eq!(
        prefill,
        json!({ "gross_salary": "3000.00", "holiday_pay": "0.00", "bonus": "0.00" })
    );

    let updated = app.ok(
        "update_paycheck",
        json!({ "jobId": job_id, "month": "2026-05", "newMonth": "2026-05", "amounts": amounts("3100.00", "2400.00", "") }),
    );
    assert_eq!(updated["gross_salary"], "3100.00");

    let jobs = app.ok("list_jobs", json!({}));
    assert_eq!(jobs[0]["entry_count"], 2);

    app.ok(
        "delete_paycheck",
        json!({ "jobId": job_id, "month": "2025-12" }),
    );
    assert_eq!(
        app.ok("list_entries", json!({})).as_array().unwrap().len(),
        1
    );

    app.ok("delete_job", json!({ "jobId": job_id }));
    assert_eq!(app.ok("list_jobs", json!({})), json!([]));
}

#[test]
fn errors_reach_the_frontend_as_code_and_safe_message() {
    let app = harness();
    let error = app.err(
        "create_job",
        json!({ "input": { "employer": "", "start_date": "01/03/2024" } }),
    );
    assert_eq!(error["code"], "JOB_EMPLOYER_REQUIRED");
    assert!(error["message"].as_str().unwrap().contains("employer"));

    // REQ-22: ISO dates are not accepted as typed input.
    let iso = app.err(
        "create_job",
        json!({ "input": { "employer": "Example Employer", "start_date": "2024-03-01" } }),
    );
    assert_eq!(iso["code"], "JOB_DATE_INVALID");
    assert!(iso["message"].as_str().unwrap().contains("dd/mm/yyyy"));

    let job = app.ok("create_job", json!({ "input": job_input() }));
    let job_id = job["id"].as_str().unwrap().to_owned();

    let future = app.err(
        "add_paycheck",
        json!({ "jobId": job_id, "month": "9999-12", "amounts": amounts("3000.00", "", "") }),
    );
    assert_eq!(future["code"], "PAYCHECK_MONTH_IN_FUTURE");

    app.ok(
        "add_paycheck",
        json!({ "jobId": job_id, "month": "2026-09", "amounts": amounts("3000.00", "", "") }),
    );
    let duplicate = app.err(
        "add_paycheck",
        json!({ "jobId": job_id, "month": "2026-09", "amounts": amounts("3100.00", "", "") }),
    );
    assert_eq!(duplicate["code"], "PAYCHECK_DUPLICATE_MONTH");

    let negative = app.err(
        "add_paycheck",
        json!({ "jobId": job_id, "month": "2026-08", "amounts": amounts("-1", "", "") }),
    );
    assert_eq!(negative["code"], "PAYCHECK_AMOUNT_NEGATIVE");
    let letters = app.err(
        "add_paycheck",
        json!({ "jobId": job_id, "month": "2026-08", "amounts": amounts("abc", "", "") }),
    );
    assert_eq!(letters["code"], "PAYCHECK_AMOUNT_INVALID");

    // A job can be renamed even though it has paychecks.
    let renamed = app.ok(
        "update_job",
        json!({ "jobId": job_id, "input": { "employer": "Renamed", "start_date": "01/03/2024" } }),
    );
    assert_eq!(renamed["employer"], "Renamed");

    let missing = app.err("copy_previous_entry", json!({ "jobId": "not-a-job" }));
    assert_eq!(missing["code"], "JOB_NOT_FOUND");
}

#[test]
fn new_project_clears_data_and_status() {
    let app = harness();
    app.ok("create_job", json!({ "input": job_input() }));
    assert_eq!(app.ok("project_status", json!({}))["unsaved_changes"], true);
    let status = app.ok("new_project", json!({}));
    assert_eq!(
        status,
        json!({ "unsaved_changes": false, "has_path": false })
    );
    assert_eq!(app.ok("list_jobs", json!({})), json!([]));
}

#[test]
fn close_is_prevented_only_while_there_are_unsaved_changes() {
    // REQ-4: the window handler asks the frontend instead of closing.
    let app = harness();
    assert!(app
        .window
        .try_state::<crate::commands::AppState>()
        .is_some());
    let state = app.window.state::<crate::commands::AppState>();
    assert!(!crate::has_unsaved_changes(&state));
    app.ok("create_job", json!({ "input": job_input() }));
    assert!(crate::has_unsaved_changes(&state));
    app.ok("new_project", json!({}));
    assert!(!crate::has_unsaved_changes(&state));
}

#[test]
fn test_csd001_ac_23_3_paychecks_and_totals_span_all_jobs_over_ipc() {
    let app = harness();
    let recent = app.ok("create_job", json!({ "input": job_input() }));
    let older = app.ok(
        "create_job",
        json!({ "input": { "employer": "Older Example Employer", "start_date": "01/01/2020" } }),
    );
    let (eur_id, usd_id) = (
        recent["id"].as_str().unwrap(),
        older["id"].as_str().unwrap(),
    );
    app.ok(
        "add_paycheck",
        json!({ "jobId": eur_id, "month": "2026-09", "amounts": amounts("3000.00", "", "") }),
    );
    app.ok(
        "add_paycheck",
        json!({ "jobId": usd_id, "month": "2026-09", "amounts": amounts("1000.00", "", "500") }),
    );
    app.ok(
        "add_paycheck",
        json!({ "jobId": usd_id, "month": "2026-08", "amounts": amounts("1000.00", "", "") }),
    );

    let rows = app.ok("list_entries", json!({}));
    let table: Vec<(&str, &str)> = rows
        .as_array()
        .unwrap()
        .iter()
        .map(|row| {
            (
                row["period"].as_str().unwrap(),
                row["employer"].as_str().unwrap(),
            )
        })
        .collect();
    assert_eq!(
        table,
        [
            ("2026-09", "Example Employer"),
            ("2026-09", "Older Example Employer"),
            ("2026-08", "Older Example Employer"),
        ]
    );

    // One row for the year, all jobs added together.
    let totals = app.ok("yearly_totals", json!({}));
    assert_eq!(totals.as_array().unwrap().len(), 1);
    assert_eq!(totals[0]["year"], 2026);
    assert_eq!(totals[0]["gross_salary"], "5000.00");
    assert_eq!(totals[0]["bonus"], "500.00");
    assert_eq!(totals[0]["total_gross"], "5500.00");
    assert_eq!(totals[0]["entry_count"], 3);
}

#[test]
fn test_csd003_settings_over_ipc() {
    let app = harness();
    // A new project uses EUR and has no paychecks.
    assert_eq!(
        app.ok("get_settings", json!({})),
        json!({ "currency": "EUR", "entry_count": 0 })
    );
    assert_eq!(
        app.ok("project_status", json!({}))["unsaved_changes"],
        false
    );

    let changed = app.ok("set_currency", json!({ "currency": "USD" }));
    assert_eq!(changed, json!({ "currency": "USD", "entry_count": 0 }));
    assert_eq!(app.ok("get_settings", json!({}))["currency"], "USD");
    // AC-4.2: the change is an unsaved change.
    assert_eq!(app.ok("project_status", json!({}))["unsaved_changes"], true);

    // AC-6.1: unsupported currencies are rejected and the currency is kept.
    for bad in ["CHF", "euro", ""] {
        let error = app.err("set_currency", json!({ "currency": bad }));
        assert_eq!(error["code"], "PROJECT_CURRENCY_UNSUPPORTED", "{bad:?}");
    }
    assert_eq!(app.ok("get_settings", json!({}))["currency"], "USD");

    // The paycheck count tells the frontend whether to ask for a confirmation (AC-5).
    let job = app.ok("create_job", json!({ "input": job_input() }));
    let job_id = job["id"].as_str().unwrap();
    app.ok(
        "add_paycheck",
        json!({ "jobId": job_id, "month": "2026-09", "amounts": amounts("3000.00", "", "") }),
    );
    let settings = app.ok("get_settings", json!({}));
    assert_eq!(settings["entry_count"], 1);

    // AC-4.1: changing the currency converts nothing.
    app.ok("set_currency", json!({ "currency": "GBP" }));
    let paychecks = app.ok("list_entries", json!({}));
    assert_eq!(paychecks[0]["gross_salary"], "3000.00");

    // AC-3.1: a new project is back to EUR.
    app.ok("new_project", json!({}));
    assert_eq!(app.ok("get_settings", json!({}))["currency"], "EUR");
}

#[test]
fn test_csd004_dashboard_over_ipc() {
    use crate::domain::YearMonth;

    let app = harness();
    let now = time::OffsetDateTime::now_utc();
    let this_month = YearMonth::from_date(now.date());
    let last_month = this_month.previous().unwrap();

    // AC-5.1: an empty project says there are no jobs and has no figures.
    let empty = app.ok("get_dashboard", json!({}));
    assert_eq!(empty["has_jobs"], false);
    assert!(empty.get("currency").is_none());
    assert_eq!(empty["year"], this_month.year());
    assert_eq!(empty["year_to_date"], Value::Null);
    assert_eq!(empty["last_month_totals"], Value::Null);

    // A job without paychecks: jobs exist, still no amounts (AC-4).
    let job = app.ok("create_job", json!({ "input": job_input() }));
    let job_id = job["id"].as_str().unwrap();
    let no_paychecks = app.ok("get_dashboard", json!({}));
    assert_eq!(no_paychecks["has_jobs"], true);
    assert_eq!(no_paychecks["year_to_date"], Value::Null);
    assert_eq!(no_paychecks["last_month"], last_month.to_string());

    // AC-7.1: figures follow the project as it is now, saved or not.
    app.ok(
        "add_paycheck",
        json!({ "jobId": job_id, "month": last_month.to_string(), "amounts": amounts("3000.00", "", "500") }),
    );
    let last = app.ok("get_dashboard", json!({}));
    assert_eq!(last["last_month"], last_month.to_string());
    assert_eq!(last["last_month_totals"]["gross_salary"], "3000.00");
    assert_eq!(last["last_month_totals"]["bonus"], "500.00");
    assert_eq!(last["last_month_totals"]["total_gross"], "3500.00");
    assert_eq!(last["last_month_totals"]["entry_count"], 1);

    // This year to date counts the current month, and last month too unless
    // that is still the previous year (January).
    app.ok(
        "add_paycheck",
        json!({ "jobId": job_id, "month": this_month.to_string(), "amounts": amounts("1000.00", "", "") }),
    );
    let year = app.ok("get_dashboard", json!({}));
    let same_year = last_month.year() == this_month.year();
    assert_eq!(
        year["year_to_date"]["entry_count"],
        if same_year { 2 } else { 1 }
    );
    assert_eq!(
        year["year_to_date"]["total_gross"],
        if same_year { "4500.00" } else { "1000.00" }
    );

    // AC-6.1: the dashboard names no currency.
    app.ok("set_currency", json!({ "currency": "USD" }));
    assert!(app.ok("get_dashboard", json!({})).get("currency").is_none());

    // AC-7.2: a new project goes back to the empty state.
    app.ok("new_project", json!({}));
    let fresh = app.ok("get_dashboard", json!({}));
    assert_eq!(fresh["has_jobs"], false);
}

/// A monthly and a yearly job, as their ids.
fn monthly_and_yearly_job(app: &Harness) -> (String, String) {
    let monthly = app.ok("create_job", json!({ "input": job_input() }));
    // AC-1.1, AC-1.5: Monthly by default, Yearly when chosen.
    assert_eq!(monthly["pay_period"], "Monthly");
    let yearly = app.ok(
        "create_job",
        json!({ "input": {
            "employer": "Older Example Employer",
            "start_date": "01/01/2020",
            "pay_period": "Yearly"
        } }),
    );
    assert_eq!(yearly["pay_period"], "Yearly");
    (
        monthly["id"].as_str().unwrap().to_owned(),
        yearly["id"].as_str().unwrap().to_owned(),
    )
}

#[test]
fn test_csd001_annual_statements_are_listed_and_totalled_over_ipc() {
    let app = harness();
    let (monthly_id, yearly_id) = monthly_and_yearly_job(&app);
    let unsupported = app.err(
        "create_job",
        json!({ "input": { "employer": "Example Employer", "start_date": "01/03/2024", "pay_period": "Weekly" } }),
    );
    assert_eq!(unsupported["code"], "JOB_PERIOD_UNSUPPORTED");

    // AC-25.1: add a statement; the view carries kind and period.
    let statement = app.ok(
        "add_statement",
        json!({ "jobId": yearly_id, "year": "2025", "amounts": amounts("40000.00", "", "3000") }),
    );
    assert_eq!(statement["kind"], "statement");
    assert_eq!(statement["period"], "2025");
    assert_eq!(statement["employer"], "Older Example Employer");
    assert_eq!(statement["total_gross"], "43000.00");
    app.ok(
        "add_paycheck",
        json!({ "jobId": monthly_id, "month": "2025-12", "amounts": amounts("3000.00", "", "") }),
    );

    // Entries of both kinds in one list, newest first.
    let entries = app.ok("list_entries", json!({}));
    let rows: Vec<(&str, &str)> = entries
        .as_array()
        .unwrap()
        .iter()
        .map(|e| (e["kind"].as_str().unwrap(), e["period"].as_str().unwrap()))
        .collect();
    assert_eq!(rows, [("paycheck", "2025-12"), ("statement", "2025")]);
    assert_eq!(app.ok("list_jobs", json!({}))[0]["entry_count"], 1);

    // AC-11.5: the statement counts in its own year, next to the paychecks.
    let totals = app.ok("yearly_totals", json!({}));
    assert_eq!(totals.as_array().unwrap().len(), 1);
    assert_eq!(totals[0]["year"], 2025);
    assert_eq!(totals[0]["gross_salary"], "43000.00");
    assert_eq!(totals[0]["bonus"], "3000.00");
    assert_eq!(totals[0]["total_gross"], "46000.00");
    assert_eq!(totals[0]["entry_count"], 2);
}

#[test]
fn test_csd001_annual_statement_errors_over_ipc() {
    let app = harness();
    let (monthly_id, yearly_id) = monthly_and_yearly_job(&app);
    app.ok(
        "add_statement",
        json!({ "jobId": yearly_id, "year": "2025", "amounts": amounts("40000.00", "", "") }),
    );
    let code = |command: &str, args: Value| app.err(command, args)["code"].clone();
    let one = amounts("1", "", "");

    // REQ-26, REQ-27, REQ-25.
    assert_eq!(
        code(
            "add_statement",
            json!({ "jobId": yearly_id, "year": "2025", "amounts": one })
        ),
        "STATEMENT_DUPLICATE_YEAR"
    );
    assert_eq!(
        code(
            "add_paycheck",
            json!({ "jobId": yearly_id, "month": "2025-11", "amounts": one })
        ),
        "JOB_PERIOD_MISMATCH"
    );
    assert_eq!(
        code(
            "add_statement",
            json!({ "jobId": monthly_id, "year": "2024", "amounts": one })
        ),
        "JOB_PERIOD_MISMATCH"
    );
    assert_eq!(
        code(
            "add_statement",
            json!({ "jobId": yearly_id, "year": "9999", "amounts": one })
        ),
        "STATEMENT_YEAR_IN_FUTURE"
    );
    for bad in ["26", "2026-05", "abcd", "1899"] {
        assert_eq!(
            code(
                "add_statement",
                json!({ "jobId": yearly_id, "year": bad, "amounts": one })
            ),
            "STATEMENT_YEAR_INVALID",
            "{bad:?}"
        );
    }
    // REQ-7 applies to statements too.
    assert_eq!(
        code(
            "add_statement",
            json!({ "jobId": yearly_id, "year": "2024", "amounts": amounts("-1", "", "") })
        ),
        "PAYCHECK_AMOUNT_NEGATIVE"
    );

    // REQ-24: the pay period is locked while the job has entries.
    let job_input = |name: &str, period: &str| json!({ "employer": name, "start_date": "01/01/2020", "pay_period": period });
    assert_eq!(
        code(
            "update_job",
            json!({ "jobId": yearly_id, "input": job_input("Older Example Employer", "Monthly") })
        ),
        "JOB_PERIOD_LOCKED"
    );
    let renamed = app.ok(
        "update_job",
        json!({ "jobId": yearly_id, "input": job_input("Renamed Example Employer", "Yearly") }),
    );
    assert_eq!(renamed["pay_period"], "Yearly");
}

#[test]
fn test_csd001_annual_statements_can_be_copied_edited_and_deleted_over_ipc() {
    use crate::domain::YearMonth;

    let app = harness();
    let this_year = YearMonth::from_date(time::OffsetDateTime::now_utc().date()).year();
    let (_, yearly_id) = monthly_and_yearly_job(&app);
    app.ok(
        "add_statement",
        json!({ "jobId": yearly_id, "year": "2025", "amounts": amounts("40000.00", "", "3000") }),
    );

    // REQ-9: copy previous year.
    let prefill = app.ok("copy_previous_entry", json!({ "jobId": yearly_id }));
    assert_eq!(
        prefill,
        json!({ "gross_salary": "40000.00", "holiday_pay": "0.00", "bonus": "0.00" })
    );

    // REQ-8: edit and delete a statement.
    let edited = app.ok(
        "update_statement",
        json!({ "jobId": yearly_id, "year": "2025", "newYear": "2025", "amounts": amounts("40000.00", "2000", "3000") }),
    );
    assert_eq!(edited["total_gross"], "45000.00");
    app.ok(
        "delete_statement",
        json!({ "jobId": yearly_id, "year": "2025" }),
    );
    assert_eq!(
        app.ok("list_entries", json!({})).as_array().unwrap().len(),
        0
    );
    assert_eq!(
        app.err(
            "delete_statement",
            json!({ "jobId": yearly_id, "year": "2025" })
        )["code"],
        "STATEMENT_NOT_FOUND"
    );

    // AC-24.2: without entries the pay period can change.
    let changed = app.ok(
        "update_job",
        json!({ "jobId": yearly_id, "input": {
            "employer": "Older Example Employer",
            "start_date": "01/01/2020",
            "pay_period": "Monthly"
        } }),
    );
    assert_eq!(changed["pay_period"], "Monthly");

    // CSD-004 AC-2.4, AC-3.4: this year's statement counts in "This year" in full
    // and never in "Last month".
    let other = app.ok(
        "create_job",
        json!({ "input": { "employer": "Third Example Employer", "start_date": "01/01/2019", "pay_period": "Yearly" } }),
    );
    app.ok(
        "add_statement",
        json!({ "jobId": other["id"], "year": this_year.to_string(), "amounts": amounts("40000.00", "", "3000") }),
    );
    let dashboard = app.ok("get_dashboard", json!({}));
    assert_eq!(dashboard["year_to_date"]["total_gross"], "43000.00");
    assert_eq!(dashboard["year_to_date"]["entry_count"], 1);
    assert_eq!(dashboard["last_month_totals"], Value::Null);

    // AC-23.2 data: the job view counts statements for a yearly job.
    let jobs = app.ok("list_jobs", json!({}));
    let third = jobs
        .as_array()
        .unwrap()
        .iter()
        .find(|j| j["employer"] == "Third Example Employer")
        .unwrap();
    assert_eq!(third["pay_period"], "Yearly");
    assert_eq!(third["entry_count"], 1);

    // AC-4.1: deleting a job removes its statements too.
    app.ok("delete_job", json!({ "jobId": other["id"] }));
    assert_eq!(
        app.ok("get_dashboard", json!({}))["year_to_date"],
        Value::Null
    );
}

fn account_input(name: &str, period: &str) -> Value {
    json!({ "name": name, "statement_period": period })
}

#[test]
fn test_csd005_ac_17_2_investment_accounts_and_the_total_over_ipc() {
    let app = harness();
    let empty = app.ok("list_investment_accounts", json!({}));
    assert_eq!(empty, json!({ "accounts": [], "total": null }));

    let brokerage = app.ok(
        "create_investment_account",
        json!({ "input": account_input("Example Brokerage", "Monthly") }),
    );
    let pension = app.ok(
        "create_investment_account",
        json!({ "input": account_input("Example Pension", "Yearly") }),
    );
    let brokerage_id = brokerage["id"].as_str().unwrap();
    let pension_id = pension["id"].as_str().unwrap();
    assert_eq!(brokerage["statement_period"], "Monthly");
    assert_eq!(app.ok("project_status", json!({}))["unsaved_changes"], true);

    app.ok(
        "add_investment_statement",
        json!({ "accountId": brokerage_id, "period": "2026-08", "balance": "12000.00", "note": null }),
    );
    app.ok(
        "add_investment_statement",
        json!({ "accountId": brokerage_id, "period": "2026-09", "balance": "12500.50", "note": "Bought example fund" }),
    );
    app.ok(
        "add_investment_statement",
        json!({ "accountId": pension_id, "period": "2025", "balance": "80000.00", "note": "" }),
    );

    let overview = app.ok("list_investment_accounts", json!({}));
    assert_eq!(overview["total"], "92500.50");
    let accounts = overview["accounts"].as_array().unwrap();
    assert_eq!(accounts[0]["name"], "Example Brokerage");
    assert_eq!(accounts[0]["statement_period"], "Monthly");
    assert_eq!(accounts[0]["statement_count"], 2);
    assert_eq!(accounts[0]["latest_period"], "2026-09");
    assert_eq!(accounts[0]["latest_balance"], "12500.50");
    assert_eq!(accounts[1]["name"], "Example Pension");
    assert_eq!(accounts[1]["statement_count"], 1);
    assert_eq!(accounts[1]["latest_period"], "2025");
    assert_eq!(accounts[1]["latest_balance"], "80000.00");
}

#[test]
fn test_csd005_ac_17_3_investment_statements_span_all_accounts_over_ipc() {
    let app = harness();
    let brokerage = app.ok(
        "create_investment_account",
        json!({ "input": account_input("Example Brokerage", "Monthly") }),
    );
    let pension = app.ok(
        "create_investment_account",
        json!({ "input": account_input("Example Pension", "Yearly") }),
    );
    app.ok(
        "add_investment_statement",
        json!({ "accountId": brokerage["id"], "period": "2026-09", "balance": "12500.00", "note": "Bought\nexample fund" }),
    );
    app.ok(
        "add_investment_statement",
        json!({ "accountId": pension["id"], "period": "2025", "balance": "80000.00" }),
    );

    let rows = app.ok("list_investment_statements", json!({}));
    let rows = rows.as_array().unwrap();
    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0]["period"], "2026-09");
    assert_eq!(rows[0]["account_name"], "Example Brokerage");
    assert_eq!(rows[0]["account_id"], brokerage["id"]);
    assert_eq!(rows[0]["balance"], "12500.00");
    assert_eq!(rows[0]["note"], "Bought\nexample fund");
    assert_eq!(rows[1]["period"], "2025");
    assert_eq!(rows[1]["account_name"], "Example Pension");
    assert_eq!(rows[1]["note"], "");
}

#[test]
fn test_csd005_investment_errors_reach_the_frontend_as_codes() {
    let app = harness();
    let account = app.ok(
        "create_investment_account",
        json!({ "input": account_input("Example Brokerage", "Monthly") }),
    );
    let id = account["id"].as_str().unwrap();
    let add = |period: &str, balance: &str, note: &str| {
        app.call(
            "add_investment_statement",
            json!({ "accountId": id, "period": period, "balance": balance, "note": note }),
        )
        .unwrap_err()
    };
    assert_eq!(
        add("2026-09", "-1.00", "")["code"],
        "INVESTMENT_BALANCE_NEGATIVE"
    );
    assert_eq!(
        add("2026-09", "abc", "")["code"],
        "INVESTMENT_BALANCE_INVALID"
    );
    assert_eq!(
        add("2026-13", "1.00", "")["code"],
        "INVESTMENT_STATEMENT_PERIOD_INVALID"
    );
    assert_eq!(
        add("2025", "1.00", "")["code"],
        "INVESTMENT_PERIOD_MISMATCH"
    );
    assert_eq!(
        add("2999-01", "1.00", "")["code"],
        "INVESTMENT_STATEMENT_IN_FUTURE"
    );
    assert_eq!(
        add("2026-09", "1.00", &"x".repeat(2001))["code"],
        "INVESTMENT_NOTE_TOO_LONG"
    );
    app.ok(
        "add_investment_statement",
        json!({ "accountId": id, "period": "2026-09", "balance": "1.00" }),
    );
    let duplicate = add("2026-09", "2.00", "");
    assert_eq!(duplicate["code"], "INVESTMENT_STATEMENT_DUPLICATE");
    assert!(!duplicate["message"].as_str().unwrap().contains(id));

    let error = app.err(
        "create_investment_account",
        json!({ "input": account_input("example brokerage", "Monthly") }),
    );
    assert_eq!(error["code"], "INVESTMENT_ACCOUNT_NAME_DUPLICATE");
    let error = app.err(
        "update_investment_account",
        json!({ "accountId": id, "input": account_input("Example Brokerage", "Yearly") }),
    );
    assert_eq!(error["code"], "INVESTMENT_PERIOD_LOCKED");
    let error = app.err(
        "delete_investment_account",
        json!({ "accountId": "not-an-id" }),
    );
    assert_eq!(error["code"], "INVESTMENT_ACCOUNT_NOT_FOUND");
}

#[test]
fn test_csd005_investment_statements_can_be_copied_edited_and_deleted_over_ipc() {
    let app = harness();
    let account = app.ok(
        "create_investment_account",
        json!({ "input": account_input("Example Brokerage", "Monthly") }),
    );
    let id = account["id"].as_str().unwrap();
    // AC-13.2: nothing to copy yet.
    assert_eq!(
        app.ok(
            "copy_previous_investment_statement",
            json!({ "accountId": id })
        ),
        json!({ "balance": null, "net_deposits": "0.00" })
    );
    app.ok(
        "add_investment_statement",
        json!({ "accountId": id, "period": "2026-08", "balance": "12500.00", "note": "First" }),
    );
    // AC-13.1: the balance is copied and the note is not part of the prefill.
    assert_eq!(
        app.ok(
            "copy_previous_investment_statement",
            json!({ "accountId": id })
        ),
        json!({ "balance": "12500.00", "net_deposits": "0.00" })
    );
    // AC-11.1: edit the balance and the note.
    let edited = app.ok(
        "update_investment_statement",
        json!({ "accountId": id, "period": "2026-08", "newPeriod": "2026-08", "balance": "12750.00", "note": "Corrected" }),
    );
    assert_eq!(edited["balance"], "12750.00");
    assert_eq!(edited["note"], "Corrected");
    // AC-12.1: delete.
    app.ok(
        "delete_investment_statement",
        json!({ "accountId": id, "period": "2026-08" }),
    );
    assert_eq!(app.ok("list_investment_statements", json!({})), json!([]));
    let error = app.err(
        "delete_investment_statement",
        json!({ "accountId": id, "period": "2026-08" }),
    );
    assert_eq!(error["code"], "INVESTMENT_STATEMENT_NOT_FOUND");
}

#[test]
fn test_csd003_ac_5_5_an_investment_statement_counts_for_the_currency_confirmation() {
    let app = harness();
    let account = app.ok(
        "create_investment_account",
        json!({ "input": account_input("Example Brokerage", "Monthly") }),
    );
    // AC-5.6: an account without statements needs no confirmation.
    assert_eq!(app.ok("get_settings", json!({}))["entry_count"], 0);
    app.ok(
        "add_investment_statement",
        json!({ "accountId": account["id"], "period": "2026-09", "balance": "12500.00" }),
    );
    assert_eq!(app.ok("get_settings", json!({}))["entry_count"], 1);
    // The balance is kept as it is when the currency changes (CSD-003 CON-1).
    app.ok("set_currency", json!({ "currency": "USD" }));
    let rows = app.ok("list_investment_statements", json!({}));
    assert_eq!(rows[0]["balance"], "12500.00");
}

#[test]
fn test_csd005_new_project_clears_the_investments() {
    let app = harness();
    app.ok(
        "create_investment_account",
        json!({ "input": account_input("Example Brokerage", "Monthly") }),
    );
    app.ok("new_project", json!({}));
    assert_eq!(
        app.ok("list_investment_accounts", json!({})),
        json!({ "accounts": [], "total": null })
    );
}

#[test]
fn test_csd005_ac_17_8_net_deposits_and_capital_growth_over_ipc() {
    let app = harness();
    let account = app.ok(
        "create_investment_account",
        json!({ "input": account_input("Example Brokerage", "Monthly") }),
    );
    let id = account["id"].as_str().unwrap();
    app.ok(
        "add_investment_statement",
        json!({ "accountId": id, "period": "2026-08", "balance": "12000.00" }),
    );
    let added = app.ok(
        "add_investment_statement",
        json!({ "accountId": id, "period": "2026-09", "balance": "12500.00", "netDeposits": "300.00" }),
    );
    assert_eq!(added["net_deposits"], "300.00");
    assert_eq!(added["capital_growth"], "200.00");

    let rows = app.ok("list_investment_statements", json!({}));
    let rows = rows.as_array().unwrap();
    assert_eq!(rows[0]["period"], "2026-09");
    assert_eq!(rows[0]["net_deposits"], "300.00");
    assert_eq!(rows[0]["capital_growth"], "200.00");
    assert_eq!(rows[1]["period"], "2026-08");
    assert_eq!(rows[1]["net_deposits"], "0.00");
    assert_eq!(rows[1]["capital_growth"], "12000.00");

    // AC-21.4: edit the net deposits; the growth follows.
    let edited = app.ok(
        "update_investment_statement",
        json!({ "accountId": id, "period": "2026-09", "newPeriod": "2026-09", "balance": "12500.00", "netDeposits": "0.00" }),
    );
    assert_eq!(edited["capital_growth"], "500.00");

    // AC-22.1 and AC-22.3: invalid net deposits are rejected and change nothing.
    let error = app.err(
        "update_investment_statement",
        json!({ "accountId": id, "period": "2026-09", "newPeriod": "2026-09", "balance": "12500.00", "netDeposits": "100.999" }),
    );
    assert_eq!(error["code"], "INVESTMENT_NET_DEPOSITS_PRECISION");
    let rows = app.ok("list_investment_statements", json!({}));
    assert_eq!(rows[0]["net_deposits"], "0.00");
}

#[test]
fn test_csd005_ac_24_the_form_calculation_over_ipc() {
    let app = harness();
    let account = app.ok(
        "create_investment_account",
        json!({ "input": account_input("Example Brokerage", "Monthly") }),
    );
    let id = account["id"].as_str().unwrap();
    app.ok(
        "add_investment_statement",
        json!({ "accountId": id, "period": "2026-08", "balance": "12000.00" }),
    );
    let before = app.ok("project_status", json!({}));

    // AC-24.1: a balance gives the growth.
    let result = app.ok(
        "calculate_investment_statement",
        json!({ "accountId": id, "period": "2026-09", "netDeposits": "", "input": "balance", "value": "12500.00" }),
    );
    assert_eq!(
        result,
        json!({ "balance": "12500.00", "capital_growth": "500.00" })
    );

    // AC-24.2: a growth gives the balance.
    let result = app.ok(
        "calculate_investment_statement",
        json!({ "accountId": id, "period": "2026-09", "netDeposits": "300.00", "input": "growth", "value": "200.00" }),
    );
    assert_eq!(
        result,
        json!({ "balance": "12500.00", "capital_growth": "200.00" })
    );

    // AC-24.9: nothing was stored.
    assert_eq!(
        app.ok("list_investment_statements", json!({}))
            .as_array()
            .unwrap()
            .len(),
        1
    );
    assert_eq!(app.ok("project_status", json!({})), before);

    // AC-25.1 and AC-25.2: errors reach the form as codes.
    let error = app.err(
        "calculate_investment_statement",
        json!({ "accountId": id, "period": "2026-09", "netDeposits": "0.00", "input": "growth", "value": "-12500.00" }),
    );
    assert_eq!(error["code"], "INVESTMENT_BALANCE_NEGATIVE");
    let error = app.err(
        "calculate_investment_statement",
        json!({ "accountId": id, "period": "2026-09", "netDeposits": "", "input": "growth", "value": "abc" }),
    );
    assert_eq!(error["code"], "INVESTMENT_GROWTH_INVALID");
}

#[test]
fn test_csd006_income_insights_over_ipc() {
    let app = harness();

    // An empty project has an empty series (AC-10.1).
    assert_eq!(
        app.ok("get_income_insights", json!({})),
        json!({ "years": [] })
    );

    let job = app.ok("create_job", json!({ "input": job_input() }));
    let job_id = job["id"].as_str().unwrap().to_owned();
    assert_eq!(
        app.ok("get_income_insights", json!({})),
        json!({ "years": [] }),
        "a job without entries has no years"
    );

    app.ok(
        "add_paycheck",
        json!({ "jobId": job_id, "month": "2025-12", "amounts": amounts("3000.00", "", "1000.00") }),
    );
    let insights = app.ok("get_income_insights", json!({}));
    let years = insights["years"].as_array().unwrap();
    assert_eq!(years[0]["year"], 2025);
    assert_eq!(years[0]["gross_salary"], "3000.00");
    assert_eq!(years[0]["holiday_pay"], "0.00");
    assert_eq!(years[0]["bonus"], "1000.00");
    assert_eq!(years[0]["total_gross"], "4000.00");
    assert_eq!(years[0]["entry_count"], 1);
    assert_eq!(years[0]["is_partial"], false);
    assert_eq!(years[0]["growth_percent"], Value::Null);
    assert_eq!(years.last().unwrap()["is_partial"], true);
}

#[test]
fn test_csd005_ac_26_yearly_totals_over_ipc() {
    let app = harness();
    // AC-26.2: nothing yet.
    assert_eq!(app.ok("yearly_investment_totals", json!({})), json!([]));
    let account = app.ok(
        "create_investment_account",
        json!({ "input": account_input("Example Brokerage", "Monthly") }),
    );
    let id = account["id"].as_str().unwrap();
    app.ok(
        "add_investment_statement",
        json!({ "accountId": id, "period": "2025-12", "balance": "11500.00", "netDeposits": "11500.00" }),
    );
    app.ok(
        "add_investment_statement",
        json!({ "accountId": id, "period": "2026-05", "balance": "12500.00", "netDeposits": "300.00" }),
    );
    // AC-26.1 and AC-26.10: one row per year, newest first, amounts as strings.
    assert_eq!(
        app.ok("yearly_investment_totals", json!({})),
        json!([
            { "year": 2026, "closing_balance": "12500.00", "net_deposits": "300.00",
              "capital_growth": "700.00", "statement_count": 1 },
            { "year": 2025, "closing_balance": "11500.00", "net_deposits": "11500.00",
              "capital_growth": "0.00", "statement_count": 1 },
        ])
    );
}

#[test]
fn test_csd007_investment_insights_over_ipc() {
    let app = harness();
    // AC-2.3: no statements, no series.
    assert_eq!(
        app.ok("get_investment_insights", json!({})),
        json!({ "years": [] })
    );
    let account = app.ok(
        "create_investment_account",
        json!({ "input": account_input("Example Brokerage", "Monthly") }),
    );
    let id = account["id"].as_str().unwrap();
    app.ok(
        "add_investment_statement",
        json!({ "accountId": id, "period": "2025-11", "balance": "11000.00", "netDeposits": "11000.00" }),
    );
    app.ok(
        "add_investment_statement",
        json!({ "accountId": id, "period": "2025-12", "balance": "11500.00", "netDeposits": "200.00" }),
    );
    let insights = app.ok("get_investment_insights", json!({}));
    let years = insights["years"].as_array().unwrap();
    // The series runs up to the real current year, so look at 2025 only.
    assert_eq!(
        years[0],
        json!({
            "year": 2025,
            "opening_balance": "0.00",
            "net_deposits": "11200.00",
            "capital_growth": "300.00",
            "growth_percent": null,
            "closing_balance": "11500.00",
            "statement_count": 2,
            "is_partial": false
        })
    );
    assert!(years.len() >= 2);
    assert_eq!(years.last().unwrap()["is_partial"], true);
}

#[test]
fn test_csd008_the_theme_is_chosen_saved_and_restored_over_ipc() {
    let config = tempfile::TempDir::new().unwrap();
    let app = harness_in(config.path());

    // AC-2.1: nothing saved yet means System.
    assert_eq!(
        app.ok("get_preferences", json!({})),
        json!({ "theme": "system" })
    );

    // AC-3.2: choosing a theme is not an unsaved project change.
    assert_eq!(
        app.ok("set_theme", json!({ "theme": "dark" })),
        json!({ "theme": "dark" })
    );
    assert_eq!(app.ok("get_preferences", json!({}))["theme"], "dark");
    assert_eq!(
        app.ok("project_status", json!({}))["unsaved_changes"],
        false
    );

    // AC-3.3: choosing the current theme changes nothing.
    app.ok("set_theme", json!({ "theme": "dark" }));
    assert_eq!(app.ok("get_preferences", json!({}))["theme"], "dark");

    // AC-5.1: a fresh session reads the saved theme.
    let restarted = harness_in(config.path());
    assert_eq!(restarted.ok("get_preferences", json!({}))["theme"], "dark");

    // AC-5.2: the theme belongs to the person, not to the project.
    app.ok("new_project", json!({}));
    assert_eq!(app.ok("get_preferences", json!({}))["theme"], "dark");
}

#[test]
fn test_csd008_ac_9_1_unsupported_themes_are_rejected_over_ipc() {
    let config = tempfile::TempDir::new().unwrap();
    let app = harness_in(config.path());
    app.ok("set_theme", json!({ "theme": "light" }));

    for bad in ["sepia", "Dark", ""] {
        let error = app.err("set_theme", json!({ "theme": bad }));
        assert_eq!(error["code"], "PREFERENCES_THEME_UNSUPPORTED", "{bad:?}");
        assert_eq!(app.ok("get_preferences", json!({}))["theme"], "light");
    }
}

#[test]
fn test_csd008_ac_10_1_a_failed_save_keeps_the_theme_over_ipc() {
    use std::os::unix::fs::PermissionsExt;
    let config = tempfile::TempDir::new().unwrap();
    let app = harness_in(config.path());
    app.ok("set_theme", json!({ "theme": "light" }));

    std::fs::set_permissions(config.path(), std::fs::Permissions::from_mode(0o500)).unwrap();
    let error = app.err("set_theme", json!({ "theme": "dark" }));
    std::fs::set_permissions(config.path(), std::fs::Permissions::from_mode(0o700)).unwrap();

    assert_eq!(error["code"], "PREFERENCES_SAVE_FAILED");
    let message = error["message"].as_str().unwrap();
    assert!(
        !message.contains(config.path().to_str().unwrap()),
        "{message}"
    );
    assert_eq!(app.ok("get_preferences", json!({}))["theme"], "light");
}

#[test]
fn test_csd001_ac_29_the_month_of_a_paycheck_changes_over_ipc() {
    let app = harness();
    let job = app.ok("create_job", json!({ "input": job_input() }));
    let job_id = job["id"].as_str().unwrap().to_owned();
    app.ok(
        "add_paycheck",
        json!({ "jobId": job_id, "month": "2026-04", "amounts": amounts("3000.00", "", "") }),
    );
    app.ok(
        "add_paycheck",
        json!({ "jobId": job_id, "month": "2026-05", "amounts": amounts("3500.00", "", "") }),
    );

    // AC-29.1 and AC-29.2: the paycheck of 2026-05 moves to 2026-03 with an edited amount.
    let moved = app.ok(
        "update_paycheck",
        json!({
            "jobId": job_id, "month": "2026-05", "newMonth": "2026-03",
            "amounts": amounts("3500.00", "100.00", "")
        }),
    );
    assert_eq!(moved["period"], "2026-03");
    assert_eq!(moved["total_gross"], "3600.00");
    let periods: Vec<Value> = app
        .ok("list_entries", json!({}))
        .as_array()
        .unwrap()
        .iter()
        .map(|entry| entry["period"].clone())
        .collect();
    assert_eq!(periods, [json!("2026-04"), json!("2026-03")]);
}

#[test]
fn test_csd001_ac_30_a_rejected_edit_changes_nothing_over_ipc() {
    let app = harness();
    let job = app.ok("create_job", json!({ "input": job_input() }));
    let job_id = job["id"].as_str().unwrap().to_owned();
    app.ok(
        "add_paycheck",
        json!({ "jobId": job_id, "month": "2026-05", "amounts": amounts("3000.00", "", "") }),
    );
    let before = app.ok("list_entries", json!({}));

    // AC-30.1: in the future. AC-30.2 is covered in the domain; here the codes cross IPC.
    let error = app.err(
        "update_paycheck",
        json!({
            "jobId": job_id, "month": "2026-05", "newMonth": "2099-01",
            "amounts": amounts("3000.00", "", "")
        }),
    );
    assert_eq!(error["code"], "PAYCHECK_MONTH_IN_FUTURE");
    // AC-30.4: an invalid amount together with a new month rejects the whole edit.
    let error = app.err(
        "update_paycheck",
        json!({
            "jobId": job_id, "month": "2026-05", "newMonth": "2026-04",
            "amounts": amounts("-1.00", "", "")
        }),
    );
    assert_eq!(error["code"], "PAYCHECK_AMOUNT_NEGATIVE");
    assert_eq!(app.ok("list_entries", json!({})), before);
}

#[test]
fn test_csd001_ac_29_3_the_year_of_an_annual_statement_changes_over_ipc() {
    let app = harness();
    let yearly = app.ok(
        "create_job",
        json!({ "input": { "employer": "Example Employer", "title": "", "start_date": "01/03/2024", "end_date": "", "pay_period": "Yearly" } }),
    );
    let job_id = yearly["id"].as_str().unwrap().to_owned();
    app.ok(
        "add_statement",
        json!({ "jobId": job_id, "year": "2025", "amounts": amounts("40000.00", "", "") }),
    );
    app.ok(
        "add_statement",
        json!({ "jobId": job_id, "year": "2024", "amounts": amounts("38000.00", "", "") }),
    );
    let moved = app.ok(
        "update_statement",
        json!({ "jobId": job_id, "year": "2025", "newYear": "2023", "amounts": amounts("40000.00", "", "") }),
    );
    assert_eq!(moved["period"], "2023");
    let error = app.err(
        "update_statement",
        json!({ "jobId": job_id, "year": "2024", "newYear": "2023", "amounts": amounts("38000.00", "", "") }),
    );
    assert_eq!(error["code"], "STATEMENT_DUPLICATE_YEAR");
    let error = app.err(
        "update_statement",
        json!({ "jobId": job_id, "year": "2024", "newYear": "2099", "amounts": amounts("38000.00", "", "") }),
    );
    assert_eq!(error["code"], "STATEMENT_YEAR_IN_FUTURE");
}

#[test]
fn test_csd005_ac_28_the_period_of_a_statement_changes_over_ipc() {
    let app = harness();
    let account = app.ok(
        "create_investment_account",
        json!({ "input": account_input("Example Brokerage", "Monthly") }),
    );
    let id = account["id"].as_str().unwrap().to_owned();
    for (period, balance) in [("2026-07", "10000.00"), ("2026-08", "10500.00")] {
        app.ok(
            "add_investment_statement",
            json!({ "accountId": id, "period": period, "balance": balance, "netDeposits": "0.00" }),
        );
    }

    // AC-29.4: an invalid balance with a new period rejects the whole edit.
    let error = app.err(
        "update_investment_statement",
        json!({ "accountId": id, "period": "2026-08", "newPeriod": "2026-09", "balance": "-1.00" }),
    );
    assert_eq!(error["code"], "INVESTMENT_BALANCE_NEGATIVE");
    // AC-29.1 to AC-29.3: the codes of the add rules cross IPC.
    for (new_period, code) in [
        ("2026-12", "INVESTMENT_STATEMENT_IN_FUTURE"),
        ("2026-07", "INVESTMENT_STATEMENT_DUPLICATE"),
        ("2026", "INVESTMENT_PERIOD_MISMATCH"),
    ] {
        let error = app.err(
            "update_investment_statement",
            json!({ "accountId": id, "period": "2026-08", "newPeriod": new_period, "balance": "10500.00" }),
        );
        assert_eq!(error["code"], code, "{new_period}");
    }
    let rows = app.ok("list_investment_statements", json!({}));
    assert_eq!(rows[0]["period"], "2026-08");
    assert_eq!(rows[0]["balance"], "10500.00");

    // AC-24.13: the form calculation does not use the statement being edited.
    let result = app.ok(
        "calculate_investment_statement",
        json!({
            "accountId": id, "period": "2026-09", "netDeposits": "", "input": "balance",
            "value": "11000.00", "ignorePeriod": "2026-08"
        }),
    );
    assert_eq!(result["capital_growth"], "1000.00");

    // AC-28.1 and AC-28.4: the statement moves and its growth follows.
    let moved = app.ok(
        "update_investment_statement",
        json!({ "accountId": id, "period": "2026-08", "newPeriod": "2026-09", "balance": "11000.00" }),
    );
    assert_eq!(moved["period"], "2026-09");
    assert_eq!(moved["capital_growth"], "1000.00");
}

fn bank_input(name: &str, period: &str) -> Value {
    json!({ "name": name, "entry_period": period })
}

#[test]
fn test_csd009_ac_17_2_bank_accounts_and_the_total_over_ipc() {
    let app = harness();
    let empty = app.ok("list_bank_accounts", json!({}));
    assert_eq!(empty, json!({ "accounts": [], "total": null }));

    let current = app.ok(
        "create_bank_account",
        json!({ "input": bank_input("Example Current Account", "Monthly") }),
    );
    assert_eq!(current["entry_period"], "Monthly");
    assert_eq!(current["colour_slot"], 0);
    let savings = app.ok(
        "create_bank_account",
        json!({ "input": bank_input("Example Savings", "Yearly") }),
    );
    assert_eq!(savings["colour_slot"], 1);
    app.ok(
        "add_bank_entry",
        json!({ "accountId": current["id"], "period": "2026-09", "balance": "2500.50" }),
    );
    app.ok(
        "add_bank_entry",
        json!({ "accountId": savings["id"], "period": "2025", "balance": "-400.00", "note": "Overdrawn" }),
    );
    let overview = app.ok("list_bank_accounts", json!({}));
    assert_eq!(overview["total"], "2100.50");
    assert_eq!(overview["accounts"][0]["latest_balance"], "2500.50");
    assert_eq!(overview["accounts"][0]["latest_period"], "2026-09");
    assert_eq!(overview["accounts"][0]["entry_count"], 1);
    assert_eq!(overview["accounts"][1]["latest_balance"], "-400.00");

    let rows = app.ok("list_bank_entries", json!({}));
    assert_eq!(rows[0]["account_name"], "Example Current Account");
    assert_eq!(rows[1]["note"], "Overdrawn");
    let totals = app.ok("yearly_bank_totals", json!({}));
    assert_eq!(
        totals[0],
        json!({ "year": 2026, "closing_balance": "2500.50", "entry_count": 1 })
    );
    assert_eq!(totals[1]["closing_balance"], "-400.00");
}

#[test]
fn test_csd009_bank_errors_reach_the_frontend_as_codes() {
    let app = harness();
    let account = app.ok(
        "create_bank_account",
        json!({ "input": bank_input("Example Current Account", "Monthly") }),
    );
    let id = account["id"].clone();
    for (period, balance, code) in [
        ("2026-09", "1000.999", "BANK_ENTRY_BALANCE_PRECISION"),
        ("2026-09", "abc", "BANK_ENTRY_BALANCE_INVALID"),
        ("2026-09", "1000000000.01", "BANK_ENTRY_BALANCE_TOO_LARGE"),
        ("2026-13", "1.00", "BANK_ENTRY_PERIOD_INVALID"),
        ("2025", "1.00", "BANK_ENTRY_PERIOD_MISMATCH"),
        ("2999-01", "1.00", "BANK_ENTRY_IN_FUTURE"),
    ] {
        let error = app.err(
            "add_bank_entry",
            json!({ "accountId": id, "period": period, "balance": balance }),
        );
        assert_eq!(error["code"], code, "{period} {balance}");
    }
    let note = "x".repeat(2001);
    let error = app.err(
        "add_bank_entry",
        json!({ "accountId": id, "period": "2026-09", "balance": "1.00", "note": note }),
    );
    assert_eq!(error["code"], "BANK_ENTRY_NOTE_TOO_LONG");
    app.ok(
        "add_bank_entry",
        json!({ "accountId": id, "period": "2026-09", "balance": "1.00" }),
    );
    let error = app.err(
        "add_bank_entry",
        json!({ "accountId": id, "period": "2026-09", "balance": "2.00" }),
    );
    assert_eq!(error["code"], "BANK_ENTRY_DUPLICATE");
    let error = app.err(
        "create_bank_account",
        json!({ "input": bank_input("example current account", "Monthly") }),
    );
    assert_eq!(error["code"], "BANK_ACCOUNT_NAME_DUPLICATE");
    let error = app.err(
        "update_bank_account",
        json!({ "accountId": id, "input": bank_input("Example Current Account", "Yearly") }),
    );
    assert_eq!(error["code"], "BANK_ACCOUNT_PERIOD_LOCKED");
    let error = app.err(
        "delete_bank_account",
        json!({ "accountId": "671166f4-5827-4794-bcdc-54d7824fb650" }),
    );
    assert_eq!(error["code"], "BANK_ACCOUNT_NOT_FOUND");
}

#[test]
fn test_csd009_bank_entries_can_be_copied_edited_moved_and_deleted_over_ipc() {
    let app = harness();
    let account = app.ok(
        "create_bank_account",
        json!({ "input": bank_input("Example Current Account", "Monthly") }),
    );
    let id = account["id"].clone();
    // AC-15.2: nothing to copy yet.
    assert_eq!(
        app.ok("copy_previous_bank_entry", json!({ "accountId": id })),
        json!({ "balance": null })
    );
    app.ok(
        "add_bank_entry",
        json!({ "accountId": id, "period": "2026-09", "balance": "-150.25", "note": "First" }),
    );
    // AC-15.3: a negative balance is copied, the note is not.
    assert_eq!(
        app.ok("copy_previous_bank_entry", json!({ "accountId": id })),
        json!({ "balance": "-150.25" })
    );
    // AC-12.3 and AC-12.4: the entry moves and changes together.
    let moved = app.ok(
        "update_bank_entry",
        json!({ "accountId": id, "period": "2026-09", "newPeriod": "2026-08", "balance": "200.00", "note": "Moved" }),
    );
    assert_eq!(moved["period"], "2026-08");
    assert_eq!(moved["balance"], "200.00");
    // AC-13.1: a later period is rejected and nothing changes.
    let error = app.err(
        "update_bank_entry",
        json!({ "accountId": id, "period": "2026-08", "newPeriod": "2999-01", "balance": "1.00" }),
    );
    assert_eq!(error["code"], "BANK_ENTRY_IN_FUTURE");
    assert_eq!(
        app.ok("list_bank_entries", json!({}))[0]["balance"],
        "200.00"
    );
    app.ok(
        "delete_bank_entry",
        json!({ "accountId": id, "period": "2026-08" }),
    );
    assert_eq!(app.ok("list_bank_entries", json!({})), json!([]));
    let error = app.err(
        "delete_bank_entry",
        json!({ "accountId": id, "period": "2026-08" }),
    );
    assert_eq!(error["code"], "BANK_ENTRY_NOT_FOUND");
}

#[test]
fn test_csd003_ac_5_7_a_bank_entry_counts_for_the_currency_confirmation() {
    let app = harness();
    let account = app.ok(
        "create_bank_account",
        json!({ "input": bank_input("Example Current Account", "Monthly") }),
    );
    // AC-5.8: an account without entries needs no confirmation.
    assert_eq!(app.ok("get_settings", json!({}))["entry_count"], 0);
    app.ok(
        "add_bank_entry",
        json!({ "accountId": account["id"], "period": "2026-09", "balance": "2500.00" }),
    );
    assert_eq!(app.ok("get_settings", json!({}))["entry_count"], 1);
    // The balance is kept as it is when the currency changes (CSD-003 CON-1).
    app.ok("set_currency", json!({ "currency": "USD" }));
    let rows = app.ok("list_bank_entries", json!({}));
    assert_eq!(rows[0]["balance"], "2500.00");
}

#[test]
fn test_csd010_the_insights_reach_the_frontend_with_banks_and_segments() {
    let app = harness();
    let empty = app.ok("get_bank_insights", json!({}));
    assert_eq!(empty, json!({ "banks": [], "years": [] }));
    let current = app.ok(
        "create_bank_account",
        json!({ "input": bank_input("Example Current Account", "Monthly") }),
    );
    let overdraft = app.ok(
        "create_bank_account",
        json!({ "input": bank_input("Example Overdraft", "Monthly") }),
    );
    app.ok(
        "add_bank_entry",
        json!({ "accountId": current["id"], "period": "2025-09", "balance": "5000.00" }),
    );
    app.ok(
        "add_bank_entry",
        json!({ "accountId": overdraft["id"], "period": "2025-09", "balance": "-1000.00" }),
    );
    let insights = app.ok("get_bank_insights", json!({}));
    assert_eq!(insights["banks"][0]["name"], "Example Current Account");
    assert_eq!(insights["banks"][1]["colour_slot"], 1);
    let first = &insights["years"][0];
    assert_eq!(first["year"], 2025);
    assert_eq!(first["closing_balance"], "4000.00");
    assert_eq!(first["entry_count"], 2);
    assert_eq!(first["is_partial"], false);
    assert_eq!(first["segments"][0]["account_id"], current["id"]);
    assert_eq!(first["segments"][1]["balance"], "-1000.00");
    // The current year is in the series even without entries (CSD-010 AC-4.2).
    let last = insights["years"]
        .as_array()
        .unwrap()
        .last()
        .unwrap()
        .clone();
    assert_eq!(last["is_partial"], true);
}

#[test]
fn test_csd009_new_project_clears_the_bank_accounts() {
    let app = harness();
    app.ok(
        "create_bank_account",
        json!({ "input": bank_input("Example Current Account", "Monthly") }),
    );
    app.ok("new_project", json!({}));
    assert_eq!(
        app.ok("list_bank_accounts", json!({})),
        json!({ "accounts": [], "total": null })
    );
}

fn debt_input(name: &str, period: &str) -> Value {
    json!({ "name": name, "statement_period": period })
}

#[test]
fn test_csd011_ac_17_2_debts_and_the_total_over_ipc() {
    let app = harness();
    assert_eq!(
        app.ok("list_debts", json!({})),
        json!({ "debts": [], "total": null })
    );
    let mortgage = app.ok(
        "create_debt",
        json!({ "input": debt_input("Example Mortgage", "Monthly") }),
    );
    assert_eq!(mortgage["statement_period"], "Monthly");
    assert_eq!(mortgage["colour_slot"], 0);
    let loan = app.ok(
        "create_debt",
        json!({ "input": debt_input("Example Student Loan", "Yearly") }),
    );
    assert_eq!(loan["colour_slot"], 1);
    app.ok(
        "add_debt_statement",
        json!({ "debtId": mortgage["id"], "period": "2025-12", "balance": "150000.00" }),
    );
    app.ok(
        "add_debt_statement",
        json!({ "debtId": mortgage["id"], "period": "2026-09", "balance": "149000.50", "note": "Paid extra" }),
    );
    app.ok(
        "add_debt_statement",
        json!({ "debtId": loan["id"], "period": "2025", "balance": "8000.00" }),
    );
    let overview = app.ok("list_debts", json!({}));
    assert_eq!(overview["total"], "157000.50");
    assert_eq!(overview["debts"][0]["latest_balance"], "149000.50");
    assert_eq!(overview["debts"][0]["latest_period"], "2026-09");
    assert_eq!(overview["debts"][0]["statement_count"], 2);

    // AC-25.1 and AC-20.3: the change reaches the frontend, null for a first statement.
    let rows = app.ok("list_debt_statements", json!({}));
    assert_eq!(rows[0]["debt_name"], "Example Mortgage");
    assert_eq!(rows[0]["change"], "999.50");
    assert_eq!(rows[0]["note"], "Paid extra");
    assert_eq!(rows[2]["change"], Value::Null);

    // AC-19.1: closing balance and repaid, null when no debt has an opening balance.
    let totals = app.ok("yearly_debt_totals", json!({}));
    assert_eq!(
        totals[0],
        json!({ "year": 2026, "closing_balance": "149000.50", "repaid": "999.50", "statement_count": 1 })
    );
    assert_eq!(totals[1]["repaid"], Value::Null);
}

#[test]
fn test_csd011_debt_errors_reach_the_frontend_as_codes() {
    let app = harness();
    let debt = app.ok(
        "create_debt",
        json!({ "input": debt_input("Example Mortgage", "Monthly") }),
    );
    let id = debt["id"].clone();
    for (period, balance, code) in [
        ("2026-09", "-1.00", "DEBT_STATEMENT_BALANCE_NEGATIVE"),
        ("2026-09", "1000.999", "DEBT_STATEMENT_BALANCE_PRECISION"),
        ("2026-09", "abc", "DEBT_STATEMENT_BALANCE_INVALID"),
        (
            "2026-09",
            "1000000000.01",
            "DEBT_STATEMENT_BALANCE_TOO_LARGE",
        ),
        ("2026-13", "1.00", "DEBT_STATEMENT_PERIOD_INVALID"),
        ("2025", "1.00", "DEBT_PERIOD_MISMATCH"),
        ("2999-01", "1.00", "DEBT_STATEMENT_IN_FUTURE"),
    ] {
        let error = app.err(
            "add_debt_statement",
            json!({ "debtId": id, "period": period, "balance": balance }),
        );
        assert_eq!(error["code"], code, "{period} {balance}");
    }
    let error = app.err(
        "add_debt_statement",
        json!({ "debtId": id, "period": "2026-09", "balance": "1.00", "note": "x".repeat(2001) }),
    );
    assert_eq!(error["code"], "DEBT_STATEMENT_NOTE_TOO_LONG");
    app.ok(
        "add_debt_statement",
        json!({ "debtId": id, "period": "2026-09", "balance": "0.00" }),
    );
    let error = app.err(
        "add_debt_statement",
        json!({ "debtId": id, "period": "2026-09", "balance": "2.00" }),
    );
    assert_eq!(error["code"], "DEBT_STATEMENT_DUPLICATE");
    let error = app.err(
        "create_debt",
        json!({ "input": debt_input("example mortgage", "Monthly") }),
    );
    assert_eq!(error["code"], "DEBT_NAME_DUPLICATE");
    let error = app.err(
        "update_debt",
        json!({ "debtId": id, "input": debt_input("Example Mortgage", "Yearly") }),
    );
    assert_eq!(error["code"], "DEBT_PERIOD_LOCKED");
    let error = app.err(
        "delete_debt",
        json!({ "debtId": "671166f4-5827-4794-bcdc-54d7824fb650" }),
    );
    assert_eq!(error["code"], "DEBT_NOT_FOUND");
}

#[test]
fn test_csd011_debt_statements_can_be_copied_edited_moved_and_deleted_over_ipc() {
    let app = harness();
    let debt = app.ok(
        "create_debt",
        json!({ "input": debt_input("Example Mortgage", "Monthly") }),
    );
    let id = debt["id"].clone();
    // AC-15.2: nothing to copy yet.
    assert_eq!(
        app.ok("copy_previous_debt_statement", json!({ "debtId": id })),
        json!({ "balance": null })
    );
    app.ok(
        "add_debt_statement",
        json!({ "debtId": id, "period": "2026-09", "balance": "150000.00", "note": "First" }),
    );
    // AC-15.1: the balance is copied, the note is not.
    assert_eq!(
        app.ok("copy_previous_debt_statement", json!({ "debtId": id })),
        json!({ "balance": "150000.00" })
    );
    // AC-12.3 and AC-12.4: the statement moves and changes together.
    let moved = app.ok(
        "update_debt_statement",
        json!({ "debtId": id, "period": "2026-09", "newPeriod": "2026-08", "balance": "149000.00", "note": "Moved" }),
    );
    assert_eq!(moved["period"], "2026-08");
    assert_eq!(moved["balance"], "149000.00");
    // AC-13.1: a later period is rejected and nothing changes.
    let error = app.err(
        "update_debt_statement",
        json!({ "debtId": id, "period": "2026-08", "newPeriod": "2999-01", "balance": "1.00" }),
    );
    assert_eq!(error["code"], "DEBT_STATEMENT_IN_FUTURE");
    assert_eq!(
        app.ok("list_debt_statements", json!({}))[0]["balance"],
        "149000.00"
    );
    app.ok(
        "delete_debt_statement",
        json!({ "debtId": id, "period": "2026-08" }),
    );
    assert_eq!(app.ok("list_debt_statements", json!({})), json!([]));
    let error = app.err(
        "delete_debt_statement",
        json!({ "debtId": id, "period": "2026-08" }),
    );
    assert_eq!(error["code"], "DEBT_STATEMENT_NOT_FOUND");
}

#[test]
fn test_csd003_ac_5_9_a_debt_statement_counts_for_the_currency_confirmation() {
    let app = harness();
    let debt = app.ok(
        "create_debt",
        json!({ "input": debt_input("Example Mortgage", "Monthly") }),
    );
    // AC-5.10: a debt without statements needs no confirmation.
    assert_eq!(app.ok("get_settings", json!({}))["entry_count"], 0);
    app.ok(
        "add_debt_statement",
        json!({ "debtId": debt["id"], "period": "2026-09", "balance": "150000.00" }),
    );
    assert_eq!(app.ok("get_settings", json!({}))["entry_count"], 1);
    // The balance is kept as it is when the currency changes (CSD-003 CON-1).
    app.ok("set_currency", json!({ "currency": "USD" }));
    let rows = app.ok("list_debt_statements", json!({}));
    assert_eq!(rows[0]["balance"], "150000.00");
}

#[test]
fn test_csd012_the_debt_insights_reach_the_frontend_with_segments_and_repaid() {
    let app = harness();
    assert_eq!(
        app.ok("get_debt_insights", json!({})),
        json!({ "debts": [], "years": [] })
    );
    let mortgage = app.ok(
        "create_debt",
        json!({ "input": debt_input("Example Mortgage", "Monthly") }),
    );
    let car = app.ok(
        "create_debt",
        json!({ "input": debt_input("Example Cabin Loan", "Monthly") }),
    );
    for (period, balance) in [("2024-12", "10000.00"), ("2025-12", "9000.00")] {
        app.ok(
            "add_debt_statement",
            json!({ "debtId": mortgage["id"], "period": period, "balance": balance }),
        );
    }
    app.ok(
        "add_debt_statement",
        json!({ "debtId": car["id"], "period": "2025-12", "balance": "0.00" }),
    );
    let insights = app.ok("get_debt_insights", json!({}));
    assert_eq!(insights["debts"][0]["name"], "Example Mortgage");
    assert_eq!(insights["debts"][1]["colour_slot"], 1);
    let first = &insights["years"][0];
    assert_eq!(first["year"], 2024);
    assert_eq!(first["repaid"], Value::Null);
    let second = &insights["years"][1];
    assert_eq!(second["year"], 2025);
    assert_eq!(second["closing_balance"], "9000.00");
    assert_eq!(second["repaid"], "1000.00");
    assert_eq!(second["statement_count"], 2);
    assert_eq!(second["is_partial"], false);
    assert_eq!(second["segments"][0]["debt_id"], mortgage["id"]);
    assert_eq!(second["segments"][1]["balance"], "0.00");
    // The current year is in the series even without statements (CSD-012 AC-4.2).
    let last = insights["years"]
        .as_array()
        .unwrap()
        .last()
        .unwrap()
        .clone();
    assert_eq!(last["is_partial"], true);
}

#[test]
fn test_csd011_new_project_clears_the_debts() {
    let app = harness();
    app.ok(
        "create_debt",
        json!({ "input": debt_input("Example Mortgage", "Monthly") }),
    );
    app.ok("new_project", json!({}));
    assert_eq!(
        app.ok("list_debts", json!({})),
        json!({ "debts": [], "total": null })
    );
}

fn house_input(name: &str, period: &str) -> Value {
    json!({
        "name": name,
        "valuation_period": period,
        "purchase_date": "01/01/1900",
        "purchase_amount": "0.00",
    })
}

#[test]
fn test_csd015_ac_17_2_houses_and_the_total_over_ipc() {
    let app = harness();
    assert_eq!(
        app.ok("list_houses", json!({})),
        json!({ "houses": [], "total": null })
    );
    let mortgage = app.ok(
        "create_house",
        json!({ "input": house_input("Example House", "Monthly") }),
    );
    assert_eq!(mortgage["valuation_period"], "Monthly");
    assert_eq!(mortgage["colour_slot"], 0);
    let loan = app.ok(
        "create_house",
        json!({ "input": house_input("Example Lodge", "Yearly") }),
    );
    assert_eq!(loan["colour_slot"], 1);
    app.ok(
        "add_house_valuation",
        json!({ "houseId": mortgage["id"], "period": "2025-12", "value": "150000.00" }),
    );
    app.ok(
        "add_house_valuation",
        json!({ "houseId": mortgage["id"], "period": "2026-09", "value": "149000.50", "note": "Paid extra" }),
    );
    app.ok(
        "add_house_valuation",
        json!({ "houseId": loan["id"], "period": "2025", "value": "8000.00" }),
    );
    let overview = app.ok("list_houses", json!({}));
    assert_eq!(overview["total"], "157000.50");
    assert_eq!(overview["houses"][0]["latest_value"], "149000.50");
    assert_eq!(overview["houses"][0]["latest_period"], "2026-09");
    assert_eq!(overview["houses"][0]["valuation_count"], 2);

    // AC-25.1 and AC-20.3: the change reaches the frontend, null for a first valuation.
    let rows = app.ok("list_house_valuations", json!({}));
    assert_eq!(rows[0]["house_name"], "Example House");
    assert_eq!(rows[0]["change"], "-999.50");
    assert_eq!(rows[0]["note"], "Paid extra");
    assert_eq!(rows[2]["change"], Value::Null);

    // AC-19.1: closing value and change, null when no house has an opening value.
    let totals = app.ok("yearly_house_values", json!({}));
    assert_eq!(
        totals[0],
        json!({ "year": 2026, "closing_value": "149000.50", "change": "-999.50", "valuation_count": 1 })
    );
    assert_eq!(totals[1]["change"], Value::Null);
}

#[test]
fn test_csd015_house_errors_reach_the_frontend_as_codes() {
    let app = harness();
    let house = app.ok(
        "create_house",
        json!({ "input": house_input("Example House", "Monthly") }),
    );
    let id = house["id"].clone();
    for (period, value, code) in [
        ("2026-09", "-1.00", "HOUSE_VALUATION_VALUE_NEGATIVE"),
        ("2026-09", "1000.999", "HOUSE_VALUATION_VALUE_PRECISION"),
        ("2026-09", "abc", "HOUSE_VALUATION_VALUE_INVALID"),
        (
            "2026-09",
            "1000000000.01",
            "HOUSE_VALUATION_VALUE_TOO_LARGE",
        ),
        ("2026-13", "1.00", "HOUSE_VALUATION_PERIOD_INVALID"),
        ("2025", "1.00", "HOUSE_PERIOD_MISMATCH"),
        ("2999-01", "1.00", "HOUSE_VALUATION_IN_FUTURE"),
    ] {
        let error = app.err(
            "add_house_valuation",
            json!({ "houseId": id, "period": period, "value": value }),
        );
        assert_eq!(error["code"], code, "{period} {value}");
    }
    let error = app.err(
        "add_house_valuation",
        json!({ "houseId": id, "period": "2026-09", "value": "1.00", "note": "x".repeat(2001) }),
    );
    assert_eq!(error["code"], "HOUSE_VALUATION_NOTE_TOO_LONG");
    app.ok(
        "add_house_valuation",
        json!({ "houseId": id, "period": "2026-09", "value": "0.00" }),
    );
    let error = app.err(
        "add_house_valuation",
        json!({ "houseId": id, "period": "2026-09", "value": "2.00" }),
    );
    assert_eq!(error["code"], "HOUSE_VALUATION_DUPLICATE");
    let error = app.err(
        "create_house",
        json!({ "input": house_input("example house", "Monthly") }),
    );
    assert_eq!(error["code"], "HOUSE_NAME_DUPLICATE");
    let error = app.err(
        "update_house",
        json!({ "houseId": id, "input": house_input("Example House", "Yearly") }),
    );
    assert_eq!(error["code"], "HOUSE_PERIOD_LOCKED");
    let error = app.err(
        "delete_house",
        json!({ "houseId": "671166f4-5827-4794-bcdc-54d7824fb650" }),
    );
    assert_eq!(error["code"], "HOUSE_NOT_FOUND");
}

#[test]
fn test_csd015_house_valuations_can_be_copied_edited_moved_and_deleted_over_ipc() {
    let app = harness();
    let house = app.ok(
        "create_house",
        json!({ "input": house_input("Example House", "Monthly") }),
    );
    let id = house["id"].clone();
    // AC-15.2: nothing to copy yet.
    assert_eq!(
        app.ok("copy_previous_house_valuation", json!({ "houseId": id })),
        json!({ "value": null })
    );
    app.ok(
        "add_house_valuation",
        json!({ "houseId": id, "period": "2026-09", "value": "150000.00", "note": "First" }),
    );
    // AC-15.1: the value is copied, the note is not.
    assert_eq!(
        app.ok("copy_previous_house_valuation", json!({ "houseId": id })),
        json!({ "value": "150000.00" })
    );
    // AC-12.3 and AC-12.4: the valuation moves and changes together.
    let moved = app.ok(
        "update_house_valuation",
        json!({ "houseId": id, "period": "2026-09", "newPeriod": "2026-08", "value": "149000.00", "note": "Moved" }),
    );
    assert_eq!(moved["period"], "2026-08");
    assert_eq!(moved["value"], "149000.00");
    // AC-13.1: a later period is rejected and nothing changes.
    let error = app.err(
        "update_house_valuation",
        json!({ "houseId": id, "period": "2026-08", "newPeriod": "2999-01", "value": "1.00" }),
    );
    assert_eq!(error["code"], "HOUSE_VALUATION_IN_FUTURE");
    assert_eq!(
        app.ok("list_house_valuations", json!({}))[0]["value"],
        "149000.00"
    );
    app.ok(
        "delete_house_valuation",
        json!({ "houseId": id, "period": "2026-08" }),
    );
    assert_eq!(app.ok("list_house_valuations", json!({})), json!([]));
    let error = app.err(
        "delete_house_valuation",
        json!({ "houseId": id, "period": "2026-08" }),
    );
    assert_eq!(error["code"], "HOUSE_VALUATION_NOT_FOUND");
}

#[test]
fn test_csd003_ac_5_13_a_house_valuation_counts_for_the_currency_confirmation() {
    let app = harness();
    let house = app.ok(
        "create_house",
        json!({ "input": house_input("Example House", "Monthly") }),
    );
    // AC-5.14: a house without valuations needs no confirmation.
    assert_eq!(app.ok("get_settings", json!({}))["entry_count"], 0);
    app.ok(
        "add_house_valuation",
        json!({ "houseId": house["id"], "period": "2026-09", "value": "150000.00" }),
    );
    assert_eq!(app.ok("get_settings", json!({}))["entry_count"], 1);
    // The value is kept as it is when the currency changes (CSD-003 CON-1).
    app.ok("set_currency", json!({ "currency": "USD" }));
    let rows = app.ok("list_house_valuations", json!({}));
    assert_eq!(rows[0]["value"], "150000.00");
}

#[test]
fn test_csd016_the_house_insights_reach_the_frontend_with_segments_and_change() {
    let app = harness();
    assert_eq!(
        app.ok("get_house_insights", json!({})),
        json!({ "houses": [], "years": [] })
    );
    let mortgage = app.ok(
        "create_house",
        json!({ "input": house_input("Example House", "Monthly") }),
    );
    let car = app.ok(
        "create_house",
        json!({ "input": house_input("Example Cabin", "Monthly") }),
    );
    for (period, value) in [("2024-12", "10000.00"), ("2025-12", "9000.00")] {
        app.ok(
            "add_house_valuation",
            json!({ "houseId": mortgage["id"], "period": period, "value": value }),
        );
    }
    app.ok(
        "add_house_valuation",
        json!({ "houseId": car["id"], "period": "2025-12", "value": "0.00" }),
    );
    let insights = app.ok("get_house_insights", json!({}));
    assert_eq!(insights["houses"][0]["name"], "Example House");
    assert_eq!(insights["houses"][1]["colour_slot"], 1);
    let first = &insights["years"][0];
    assert_eq!(first["year"], 2024);
    assert_eq!(first["change"], Value::Null);
    let second = &insights["years"][1];
    assert_eq!(second["year"], 2025);
    assert_eq!(second["closing_value"], "9000.00");
    assert_eq!(second["change"], "-1000.00");
    assert_eq!(second["valuation_count"], 2);
    assert_eq!(second["is_partial"], false);
    assert_eq!(second["segments"][0]["house_id"], mortgage["id"]);
    assert_eq!(second["segments"][1]["value"], "0.00");
    // The current year is in the series even without valuations (CSD-016 AC-4.2).
    let last = insights["years"]
        .as_array()
        .unwrap()
        .last()
        .unwrap()
        .clone();
    assert_eq!(last["is_partial"], true);
}

#[test]
fn test_csd015_new_project_clears_the_houses() {
    let app = harness();
    app.ok(
        "create_house",
        json!({ "input": house_input("Example House", "Monthly") }),
    );
    app.ok("new_project", json!({}));
    assert_eq!(
        app.ok("list_houses", json!({})),
        json!({ "houses": [], "total": null })
    );
}

#[test]
fn test_csd017_the_net_worth_insights_reach_the_frontend_and_follow_the_project() {
    let app = harness();
    assert_eq!(
        app.ok("get_net_worth_insights", json!({})),
        json!({ "years": [] })
    );
    let bank = app.ok(
        "create_bank_account",
        json!({ "input": bank_input("Example Bank", "Yearly") }),
    );
    app.ok(
        "add_bank_entry",
        json!({ "accountId": bank["id"], "period": "2025", "balance": "5000.00" }),
    );
    let debt = app.ok(
        "create_debt",
        json!({ "input": debt_input("Example Loan", "Yearly") }),
    );
    app.ok(
        "add_debt_statement",
        json!({ "debtId": debt["id"], "period": "2025", "balance": "9000.00" }),
    );
    let years = app.ok("get_net_worth_insights", json!({}))["years"].clone();
    let first = &years[0];
    assert_eq!(first["year"], 2025);
    assert_eq!(first["cash"], "5000.00");
    assert_eq!(first["debts"], "9000.00");
    assert_eq!(first["houses"], "0.00");
    assert_eq!(first["net_worth"], "-4000.00");
    assert_eq!(first["has_records"], true);
    assert_eq!(
        years.as_array().unwrap().last().unwrap()["is_partial"],
        true
    );
    app.ok("new_project", json!({}));
    assert_eq!(
        app.ok("get_net_worth_insights", json!({})),
        json!({ "years": [] })
    );
}

#[test]
fn test_csd018_the_ratios_reach_the_frontend_and_follow_the_project() {
    let app = harness();
    assert_eq!(app.ok("get_ratios", json!({})), json!({ "years": [] }));
    let bank = app.ok(
        "create_bank_account",
        json!({ "input": bank_input("Example Bank", "Yearly") }),
    );
    for (period, balance) in [("2024", "1000.00"), ("2025", "3000.00")] {
        app.ok(
            "add_bank_entry",
            json!({ "accountId": bank["id"], "period": period, "balance": balance }),
        );
    }
    app.ok(
        "add_expense",
        json!({ "input": {
            "name": "Example Bike", "amount": "500.00", "date": "05/03/2025", "category": "Other"
        } }),
    );
    let years = app.ok("get_ratios", json!({}))["years"].clone();
    let row = years
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["year"] == 2025)
        .unwrap();
    assert_eq!(row["total_income"], "0.00");
    assert_eq!(row["wealth_accumulation"], "2000.00");
    assert_eq!(row["disposable_income"], "2500.00");
    assert_eq!(row["wealth_percent"], Value::Null);
    assert_eq!(years[0]["year"], 2024);
    assert_eq!(years[0]["wealth_accumulation"], Value::Null);
    app.ok("new_project", json!({}));
    assert_eq!(app.ok("get_ratios", json!({})), json!({ "years": [] }));
}

#[test]
fn test_csd019_ac_14_1_the_purchase_and_sale_need_no_currency_confirmation() {
    let app = harness();
    app.ok(
        "create_house",
        json!({ "input": {
            "name": "Example House",
            "valuation_period": "Monthly",
            "purchase_date": "15/03/2024",
            "purchase_amount": "250000.00",
            "sell_date": "10/06/2026",
            "sell_amount": "300000.00",
        } }),
    );
    assert_eq!(app.ok("get_settings", json!({}))["entry_count"], 0);
}

#[test]
fn test_csd019_ac_9_2_the_sale_and_its_result_reach_the_frontend() {
    let app = harness();
    let house = app.ok(
        "create_house",
        json!({ "input": {
            "name": "Example House",
            "valuation_period": "Monthly",
            "purchase_date": "15/03/2024",
            "purchase_amount": "250000.00",
            "sell_date": "10/06/2026",
            "sell_amount": "300000.00",
        } }),
    );
    assert_eq!(house["purchase_date"], "2024-03-15");
    assert_eq!(house["sell_date"], "2026-06-10");
    let overview = app.ok("list_houses", json!({}));
    assert_eq!(overview["houses"][0]["held"], false);
    assert_eq!(overview["houses"][0]["result"], "50000.00");
    let error = app.err(
        "create_house",
        json!({ "input": { "name": "Example Lodge", "valuation_period": "Monthly" } }),
    );
    assert_eq!(error["code"], "HOUSE_PURCHASE_DATE_REQUIRED");
}
