use std::fs;

use tempfile::TempDir;

use super::*;
use crate::domain::bank_accounts::{bank_note, parse_period, AccountInput};
use crate::domain::{BankAccountId, BankBalance, YearMonth};

fn account(project: &mut Project, name: &str, period: &str) -> BankAccountId {
    project
        .bank_accounts_mut()
        .create_account(&AccountInput {
            name: name.to_owned(),
            entry_period: Some(period.to_owned()),
        })
        .unwrap()
        .id
}

#[test]
fn test_csd009_ac_23_1_bank_accounts_survive_save_and_open_including_notes_and_slots() {
    let dir = TempDir::new().unwrap();
    let mut project = Project::new();
    account(&mut project, "Example Current Account", "Monthly");
    let id = account(&mut project, "Example Savings", "Yearly");
    project
        .bank_accounts_mut()
        .add_entry(
            id,
            parse_period("2025").unwrap(),
            BankBalance::parse("8000.00").unwrap(),
            bank_note("Line one\nLine two").unwrap(),
            YearMonth::parse("2026-10").unwrap(),
        )
        .unwrap();
    let path = dir.path().join("example-a.wealth");
    project.save_as(path.clone()).unwrap();

    let mut fresh = Project::new();
    fresh.open(&path).unwrap();
    assert_eq!(fresh.bank_accounts(), project.bank_accounts());
    let rows = fresh.bank_accounts().all_entries();
    assert_eq!(rows[0].1.note.as_str(), "Line one\nLine two");
    assert_eq!(rows[0].0.colour_slot, 1);
    assert!(!fresh.status().unsaved_changes);

    // The section sits next to the others; the version stays 1.
    let value: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&path).unwrap()).unwrap();
    assert_eq!(value["format_version"], 1);
    assert_eq!(
        value["bank_accounts"]["accounts"][1]["name"],
        "Example Savings"
    );
    assert_eq!(value["bank_accounts"]["accounts"][1]["colour_slot"], 1);
    assert_eq!(value["bank_accounts"]["entries"][0]["balance"], "8000.00");
}

#[test]
fn test_csd009_ac_23_2_a_file_without_the_section_opens_without_accounts() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("example-a.wealth");
    fs::write(
        &path,
        r#"{"format_version":1,"income":{"jobs":[],"paychecks":[]}}"#,
    )
    .unwrap();
    let mut project = Project::new();
    project.open(&path).unwrap();
    assert_eq!(project.bank_accounts().accounts().len(), 0);
    assert!(!project.status().unsaved_changes);
}

#[test]
fn test_csd009_ac_23_3_a_negative_balance_survives_the_round_trip() {
    let dir = TempDir::new().unwrap();
    let mut project = Project::new();
    let id = account(&mut project, "Example Current Account", "Monthly");
    project
        .bank_accounts_mut()
        .add_entry(
            id,
            parse_period("2026-09").unwrap(),
            BankBalance::parse("-150.25").unwrap(),
            bank_note("").unwrap(),
            YearMonth::parse("2026-10").unwrap(),
        )
        .unwrap();
    let path = dir.path().join("example-a.wealth");
    project.save_as(path.clone()).unwrap();
    let mut fresh = Project::new();
    fresh.open(&path).unwrap();
    let rows = fresh.bank_accounts().all_entries();
    assert_eq!(rows[0].1.balance.value().to_string(), "-150.25");
}

#[test]
fn test_csd009_ac_23_4_the_other_sections_are_untouched() {
    let dir = TempDir::new().unwrap();
    let mut project = Project::new();
    account(&mut project, "Example Current Account", "Monthly");
    project
        .investments_mut()
        .create_account(&crate::domain::AccountInput {
            name: "Example Brokerage".to_owned(),
            statement_period: None,
        })
        .unwrap();
    let path = dir.path().join("example-a.wealth");
    project.save_as(path.clone()).unwrap();
    let mut fresh = Project::new();
    fresh.open(&path).unwrap();
    assert_eq!(fresh.bank_accounts(), project.bank_accounts());
    assert_eq!(fresh.investments(), project.investments());
}

#[test]
fn test_csd009_an_inconsistent_bank_section_is_an_invalid_file() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("example-a.wealth");
    // An entry of an account that does not exist.
    fs::write(
        &path,
        r#"{"format_version":1,"income":{"jobs":[],"paychecks":[]},
            "bank_accounts":{"accounts":[],"entries":[
            {"account_id":"671166f4-5827-4794-bcdc-54d7824fb650","period":"2026-09","balance":"1.00"}]}}"#,
    )
    .unwrap();
    let mut project = Project::new();
    let error = project.open(&path).unwrap_err();
    assert_eq!(error.code(), "PROJECT_INVALID_FILE");
}

#[test]
fn test_csd009_a_bank_change_is_an_unsaved_change() {
    let dir = TempDir::new().unwrap();
    let mut project = Project::new();
    project
        .save_as(dir.path().join("example-a.wealth"))
        .unwrap();
    assert!(!project.status().unsaved_changes);
    account(&mut project, "Example Current Account", "Monthly");
    assert!(project.status().unsaved_changes);
}
