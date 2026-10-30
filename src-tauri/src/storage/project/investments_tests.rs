use std::fs;

use tempfile::TempDir;

use super::*;
use crate::domain::{AccountInput, Balance, NetDeposits, Note, Period, YearMonth};

fn account(project: &mut Project, name: &str, period: &str) -> crate::domain::InvestmentAccountId {
    project
        .investments_mut()
        .create_account(&AccountInput {
            name: name.to_owned(),
            statement_period: Some(period.to_owned()),
        })
        .unwrap()
        .id
}

#[test]
fn test_csd005_ac_19_1_investments_survive_save_and_open_including_notes() {
    let dir = TempDir::new().unwrap();
    let mut project = Project::new();
    let id = account(&mut project, "Example Pension", "Yearly");
    project
        .investments_mut()
        .add_statement(
            id,
            Period::parse("2025").unwrap(),
            Balance::parse("80000.00").unwrap(),
            NetDeposits::parse("-250.00").unwrap(),
            Note::new("Line one\nLine two").unwrap(),
            YearMonth::parse("2026-10").unwrap(),
        )
        .unwrap();
    let path = dir.path().join("example-a.wealth");
    project.save_as(path.clone()).unwrap();

    let mut fresh = Project::new();
    fresh.open(&path).unwrap();
    assert_eq!(fresh.investments(), project.investments());
    let rows = fresh.investments().all_statements();
    assert_eq!(rows[0].1.note.as_str(), "Line one\nLine two");
    // AC-19.3: net deposits survive the round trip.
    assert_eq!(rows[0].1.net_deposits.value().to_string(), "-250.00");
    assert!(!fresh.status().unsaved_changes);

    // The section sits next to income and settings; the version stays 1.
    let value: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&path).unwrap()).unwrap();
    assert_eq!(value["format_version"], 1);
    assert_eq!(
        value["investments"]["accounts"][0]["name"],
        "Example Pension"
    );
    assert_eq!(value["investments"]["statements"][0]["balance"], "80000.00");
}

#[test]
fn test_csd005_ac_19_2_a_file_without_the_section_opens_without_accounts() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("example-a.wealth");
    fs::write(
        &path,
        r#"{"format_version":1,"income":{"jobs":[],"paychecks":[]}}"#,
    )
    .unwrap();
    let mut project = Project::new();
    project.open(&path).unwrap();
    assert_eq!(project.investments().accounts().len(), 0);
    assert!(!project.status().unsaved_changes);
}

#[test]
fn test_csd005_an_inconsistent_investments_section_is_an_invalid_file() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("example-a.wealth");
    // A statement of an account that does not exist.
    fs::write(
        &path,
        r#"{"format_version":1,"income":{"jobs":[],"paychecks":[]},
            "investments":{"accounts":[],"statements":[
            {"account_id":"671166f4-5827-4794-bcdc-54d7824fb650","period":"2026-09","balance":"1.00"}]}}"#,
    )
    .unwrap();
    let mut project = Project::new();
    let error = project.open(&path).unwrap_err();
    assert_eq!(error.code(), "PROJECT_INVALID_FILE");
}

#[test]
fn test_csd005_an_investment_change_is_an_unsaved_change() {
    let dir = TempDir::new().unwrap();
    let mut project = Project::new();
    let path = dir.path().join("example-a.wealth");
    project.save_as(path).unwrap();
    assert!(!project.status().unsaved_changes);
    account(&mut project, "Example Brokerage", "Monthly");
    assert!(project.status().unsaved_changes);
}
