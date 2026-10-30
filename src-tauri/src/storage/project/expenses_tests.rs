use std::fs;

use tempfile::TempDir;
use time::macros::date;

use super::*;
use crate::domain::expenses::ExpenseInput;

fn add(project: &mut Project, name: &str, amount: &str, date: &str, note: &str) {
    project
        .expenses_mut()
        .add_expense(
            &ExpenseInput {
                name: name.to_owned(),
                amount: amount.to_owned(),
                date: date.to_owned(),
                category: "Home".to_owned(),
                note: Some(note.to_owned()),
            },
            date!(2026 - 10 - 29),
        )
        .unwrap();
}

#[test]
fn test_csd013_ac_16_1_expenses_survive_save_and_open_including_notes() {
    let dir = TempDir::new().unwrap();
    let mut project = Project::new();
    add(
        &mut project,
        "Example Roof Repair",
        "4500.00",
        "15/09/2026",
        "Line one\nLine two",
    );
    let path = dir.path().join("example-a.wealth");
    project.save_as(path.clone()).unwrap();

    let mut fresh = Project::new();
    fresh.open(&path).unwrap();
    assert_eq!(fresh.expenses(), project.expenses());
    let list = fresh.expenses().list();
    assert_eq!(list.expenses[0].note.as_str(), "Line one\nLine two");
    assert!(!fresh.status().unsaved_changes);

    // The section sits next to the others; the version stays 1.
    let value: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&path).unwrap()).unwrap();
    assert_eq!(value["format_version"], 1);
    assert_eq!(value["expenses"]["expenses"][0]["date"], "2026-09-15");
    assert_eq!(value["expenses"]["expenses"][0]["amount"], "4500.00");
}

#[test]
fn test_csd013_ac_16_2_a_file_without_the_section_opens_without_expenses() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("example-a.wealth");
    fs::write(
        &path,
        r#"{"format_version":1,"income":{"jobs":[],"paychecks":[]}}"#,
    )
    .unwrap();
    let mut project = Project::new();
    project.open(&path).unwrap();
    assert_eq!(project.expenses().total_count(), 0);
    assert!(!project.status().unsaved_changes);
}

#[test]
fn test_csd013_ac_16_3_the_other_sections_are_untouched() {
    let dir = TempDir::new().unwrap();
    let mut project = Project::new();
    add(&mut project, "Example Bike", "900.00", "10/09/2026", "");
    project
        .debts_mut()
        .create_debt(&crate::domain::debts::DebtInput {
            name: "Example Mortgage".to_owned(),
            statement_period: None,
        })
        .unwrap();
    project
        .bank_accounts_mut()
        .create_account(&crate::domain::bank_accounts::AccountInput {
            name: "Example Current Account".to_owned(),
            entry_period: None,
        })
        .unwrap();
    let path = dir.path().join("example-a.wealth");
    project.save_as(path.clone()).unwrap();
    let mut fresh = Project::new();
    fresh.open(&path).unwrap();
    assert_eq!(fresh.expenses(), project.expenses());
    assert_eq!(fresh.debts(), project.debts());
    assert_eq!(fresh.bank_accounts(), project.bank_accounts());
}

#[test]
fn test_csd013_ac_16_4_a_damaged_section_is_an_invalid_file() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("example-a.wealth");
    let id = "671166f4-5827-4794-bcdc-54d7824fb650";
    let expense = |amount: &str, date: &str, category: &str| {
        format!(
            r#"{{"format_version":1,"income":{{"jobs":[],"paychecks":[]}},"expenses":{{"expenses":[
            {{"id":"{id}","name":"Example","amount":"{amount}","date":"{date}","category":"{category}"}}]}}}}"#
        )
    };
    let duplicate = format!(
        r#"{{"format_version":1,"income":{{"jobs":[],"paychecks":[]}},"expenses":{{"expenses":[
        {{"id":"{id}","name":"A","amount":"1.00","date":"2026-09-15","category":"Home"}},
        {{"id":"{id}","name":"B","amount":"1.00","date":"2026-09-15","category":"Home"}}]}}}}"#
    );
    for text in [
        expense("1.00", "2026-09-15", "Health"),
        expense("0.00", "2026-09-15", "Home"),
        expense("1.00", "2026-02-30", "Home"),
        duplicate,
    ] {
        fs::write(&path, text).unwrap();
        let mut project = Project::new();
        assert_eq!(
            project.open(&path).unwrap_err().code(),
            "PROJECT_INVALID_FILE"
        );
    }
}

#[test]
fn test_csd013_an_expense_change_is_an_unsaved_change() {
    let dir = TempDir::new().unwrap();
    let mut project = Project::new();
    project
        .save_as(dir.path().join("example-a.wealth"))
        .unwrap();
    assert!(!project.status().unsaved_changes);
    add(&mut project, "Example Bike", "900.00", "10/09/2026", "");
    assert!(project.status().unsaved_changes);
}
