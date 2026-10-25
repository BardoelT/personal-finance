//! Calls the real commands through Tauri's IPC layer on the mock runtime, the
//! same way the frontend does: command names, camelCase argument names and
//! the JSON shapes of the responses are the contract with `ui/`.

use serde_json::{json, Value};
use tauri::test::{get_ipc_response, mock_builder, mock_context, noop_assets, MockRuntime};
use tauri::webview::InvokeRequest;
use tauri::{ipc::InvokeBody, App, Manager, WebviewWindow, WebviewWindowBuilder};
use time::UtcOffset;

use crate::configure;

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

    // Create a job exactly as ui/income.js submits it (empty strings for blanks).
    let job = app.ok("create_job", json!({ "input": job_input() }));
    let job_id = job["id"].as_str().unwrap().to_owned();
    assert_eq!(job["employer"], "Example Employer");
    assert_eq!(job["title"], Value::Null);
    assert_eq!(job["start_date"], "2024-03-01");
    assert_eq!(job["end_date"], Value::Null);
    // Jobs carry no currency: the project has one (CSD-003).
    assert!(job.get("currency").is_none());
    assert_eq!(job["paycheck_count"], 0);
    assert_eq!(app.ok("project_status", json!({}))["unsaved_changes"], true);

    // Paychecks: arguments use camelCase `jobId`, amounts are strings.
    let added = app.ok(
        "add_paycheck",
        json!({ "jobId": job_id, "month": "2025-12", "amounts": amounts("3000.00", "", "1000.00") }),
    );
    assert_eq!(added["month"], "2025-12");
    assert_eq!(added["gross_salary"], "3000.00");
    assert_eq!(added["holiday_pay"], "0.00");
    assert_eq!(added["bonus"], "1000.00");
    assert_eq!(added["total_gross"], "4000.00");
    app.ok(
        "add_paycheck",
        json!({ "jobId": job_id, "month": "2026-05", "amounts": amounts("3000", "2400", "") }),
    );

    let paychecks = app.ok("list_paychecks", json!({}));
    let months: Vec<&str> = paychecks
        .as_array()
        .unwrap()
        .iter()
        .map(|p| p["month"].as_str().unwrap())
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
    assert_eq!(totals[0]["paycheck_count"], 1);
    assert_eq!(totals[1]["year"], 2025);
    assert_eq!(totals[1]["bonus"], "1000.00");

    let prefill = app.ok("copy_previous_paycheck", json!({ "jobId": job_id }));
    assert_eq!(
        prefill,
        json!({ "gross_salary": "3000.00", "holiday_pay": "0.00", "bonus": "0.00" })
    );

    let updated = app.ok(
        "update_paycheck",
        json!({ "jobId": job_id, "month": "2026-05", "amounts": amounts("3100.00", "2400.00", "") }),
    );
    assert_eq!(updated["gross_salary"], "3100.00");

    let jobs = app.ok("list_jobs", json!({}));
    assert_eq!(jobs[0]["paycheck_count"], 2);

    app.ok(
        "delete_paycheck",
        json!({ "jobId": job_id, "month": "2025-12" }),
    );
    assert_eq!(
        app.ok("list_paychecks", json!({}))
            .as_array()
            .unwrap()
            .len(),
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

    let missing = app.err("copy_previous_paycheck", json!({ "jobId": "not-a-job" }));
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

    let rows = app.ok("list_paychecks", json!({}));
    let table: Vec<(&str, &str)> = rows
        .as_array()
        .unwrap()
        .iter()
        .map(|row| {
            (
                row["month"].as_str().unwrap(),
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
    assert_eq!(totals[0]["paycheck_count"], 3);
}

#[test]
fn test_csd003_settings_over_ipc() {
    let app = harness();
    // A new project uses EUR and has no paychecks.
    assert_eq!(
        app.ok("get_settings", json!({})),
        json!({ "currency": "EUR", "paycheck_count": 0 })
    );
    assert_eq!(
        app.ok("project_status", json!({}))["unsaved_changes"],
        false
    );

    let changed = app.ok("set_currency", json!({ "currency": "USD" }));
    assert_eq!(changed, json!({ "currency": "USD", "paycheck_count": 0 }));
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
    assert_eq!(settings["paycheck_count"], 1);

    // AC-4.1: changing the currency converts nothing.
    app.ok("set_currency", json!({ "currency": "GBP" }));
    let paychecks = app.ok("list_paychecks", json!({}));
    assert_eq!(paychecks[0]["gross_salary"], "3000.00");

    // AC-3.1: a new project is back to EUR.
    app.ok("new_project", json!({}));
    assert_eq!(app.ok("get_settings", json!({}))["currency"], "EUR");
}
