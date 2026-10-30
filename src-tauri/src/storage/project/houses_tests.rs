use std::fs;

use tempfile::TempDir;

use super::*;
use crate::domain::houses::linking::{self, DebtFormInput, DebtLinksInput, SettlementChoice};
use crate::domain::houses::{house_note, parse_period, HouseInput, HouseValue};
use crate::domain::{HouseId, YearMonth};

fn house(project: &mut Project, name: &str, period: &str) -> HouseId {
    project
        .houses_mut()
        .create_house(
            &HouseInput {
                name: name.to_owned(),
                valuation_period: Some(period.to_owned()),
                purchase_date: Some("01/01/1900".to_owned()),
                purchase_amount: Some("0.00".to_owned()),
                ..HouseInput::default()
            },
            time::macros::date!(2026 - 10 - 29),
        )
        .unwrap()
        .id
}

#[test]
fn test_csd015_ac_24_1_houses_survive_save_and_open_including_notes_and_slots() {
    let dir = TempDir::new().unwrap();
    let mut project = Project::new();
    house(&mut project, "Example House", "Monthly");
    let id = house(&mut project, "Example Lodge", "Yearly");
    project
        .houses_mut()
        .add_valuation(
            id,
            parse_period("2025").unwrap(),
            HouseValue::parse("8000.00").unwrap(),
            house_note("Line one\nLine two").unwrap(),
            YearMonth::parse("2026-10").unwrap(),
        )
        .unwrap();
    let path = dir.path().join("example-a.wealth");
    project.save_as(path.clone()).unwrap();

    let mut fresh = Project::new();
    fresh.open(&path).unwrap();
    assert_eq!(fresh.houses(), project.houses());
    let rows = fresh.houses().all_valuations();
    assert_eq!(rows[0].1.note.as_str(), "Line one\nLine two");
    assert_eq!(rows[0].0.colour_slot, 1);
    assert!(!fresh.status().unsaved_changes);

    // The section sits next to the others; the version stays 1.
    let value: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&path).unwrap()).unwrap();
    assert_eq!(value["format_version"], 1);
    assert_eq!(value["houses"]["houses"][1]["name"], "Example Lodge");
    assert_eq!(value["houses"]["houses"][1]["colour_slot"], 1);
    assert_eq!(value["houses"]["valuations"][0]["value"], "8000.00");
}

#[test]
fn test_csd015_ac_24_2_a_file_without_the_section_opens_without_houses() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("example-a.wealth");
    fs::write(
        &path,
        r#"{"format_version":1,"income":{"jobs":[],"paychecks":[]}}"#,
    )
    .unwrap();
    let mut project = Project::new();
    project.open(&path).unwrap();
    assert_eq!(project.houses().houses().len(), 0);
    assert!(!project.status().unsaved_changes);
}

#[test]
fn test_csd015_ac_24_6_a_file_with_the_former_section_opens_as_houses() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("example-a.wealth");
    fs::write(
        &path,
        r#"{"format_version":1,"income":{"jobs":[],"paychecks":[]},
            "assets":{"assets":[{"id":"671166f4-5827-4794-bcdc-54d7824fb650",
            "name":"Example House","valuation_period":"Monthly","colour_slot":0}],
            "valuations":[{"asset_id":"671166f4-5827-4794-bcdc-54d7824fb650",
            "period":"2026-09","value":"300000.00"}]}}"#,
    )
    .unwrap();
    let mut project = Project::new();
    project.open(&path).unwrap();
    assert_eq!(project.houses().houses().len(), 1);
    project
        .save_as(dir.path().join("example-b.wealth"))
        .unwrap();
    let text = fs::read_to_string(dir.path().join("example-b.wealth")).unwrap();
    let value: serde_json::Value = serde_json::from_str(&text).unwrap();
    assert!(value.get("assets").is_none());
    assert_eq!(value["houses"]["houses"][0]["name"], "Example House");
    assert_eq!(value["houses"]["valuations"][0]["value"], "300000.00");
}

#[test]
fn test_csd015_ac_24_3_a_zero_value_survives_the_round_trip() {
    let dir = TempDir::new().unwrap();
    let mut project = Project::new();
    let id = house(&mut project, "Example House", "Monthly");
    project
        .houses_mut()
        .add_valuation(
            id,
            parse_period("2026-09").unwrap(),
            HouseValue::parse("0.00").unwrap(),
            house_note("").unwrap(),
            YearMonth::parse("2026-10").unwrap(),
        )
        .unwrap();
    let path = dir.path().join("example-a.wealth");
    project.save_as(path.clone()).unwrap();
    let mut fresh = Project::new();
    fresh.open(&path).unwrap();
    let rows = fresh.houses().all_valuations();
    assert_eq!(rows[0].1.value.value().to_string(), "0.00");
}

#[test]
fn test_csd015_ac_24_4_the_other_sections_are_untouched() {
    let dir = TempDir::new().unwrap();
    let mut project = Project::new();
    house(&mut project, "Example House", "Monthly");
    project
        .bank_accounts_mut()
        .create_account(&crate::domain::bank_accounts::AccountInput {
            name: "Example Current Account".to_owned(),
            entry_period: None,
        })
        .unwrap();
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
    assert_eq!(fresh.houses(), project.houses());
    assert_eq!(fresh.bank_accounts(), project.bank_accounts());
    assert_eq!(fresh.investments(), project.investments());
}

#[test]
fn test_csd015_ac_24_5_a_damaged_section_is_an_invalid_file() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("example-a.wealth");
    // A valuation of a house that does not exist.
    fs::write(
        &path,
        r#"{"format_version":1,"income":{"jobs":[],"paychecks":[]},
            "houses":{"houses":[],"valuations":[
            {"house_id":"671166f4-5827-4794-bcdc-54d7824fb650","period":"2026-09","value":"1.00"}]}}"#,
    )
    .unwrap();
    let mut project = Project::new();
    let error = project.open(&path).unwrap_err();
    assert_eq!(error.code(), "PROJECT_INVALID_FILE");
}

#[test]
fn test_csd020_ac_16_1_links_and_settlements_survive_the_round_trip() {
    let dir = TempDir::new().unwrap();
    let mut project = Project::new();
    let today = time::macros::date!(2026 - 10 - 29);
    let month = YearMonth::parse("2026-10").unwrap();
    let loan = project
        .debts_mut()
        .create_debt(&crate::domain::debts::DebtInput {
            name: "Example Loan".to_owned(),
            statement_period: None,
        })
        .unwrap()
        .id;
    project
        .debts_mut()
        .add_statement(
            loan,
            crate::domain::debts::parse_period("2026-06").unwrap(),
            crate::domain::debts::DebtBalance::parse("3500.00").unwrap(),
            crate::domain::debts::debt_note("Mid-June").unwrap(),
            month,
        )
        .unwrap();
    let input = HouseInput {
        name: "Example House".to_owned(),
        purchase_date: Some("15/03/2024".to_owned()),
        purchase_amount: Some("250000.00".to_owned()),
        ..HouseInput::default()
    };
    let (houses, debts) = project.houses_and_debts_mut();
    let house = linking::create_house(houses, debts, &input, &DebtLinksInput::default(), today)
        .unwrap()
        .id;
    let (houses, debts) = project.houses_and_debts_mut();
    linking::update_debt(
        houses,
        debts,
        loan,
        &DebtFormInput {
            debt: crate::domain::debts::DebtInput {
                name: "Example Loan".to_owned(),
                statement_period: None,
            },
            category: Some("Loan".to_owned()),
            house_id: Some(house.to_string()),
            settlement: None,
        },
    )
    .unwrap();
    let sold = HouseInput {
        sell_date: Some("10/06/2026".to_owned()),
        sell_amount: Some("300000.00".to_owned()),
        ..input
    };
    let settle = DebtLinksInput {
        settlements: vec![SettlementChoice {
            debt_id: loan.to_string(),
            outcome: "Cancelled".to_owned(),
        }],
        ..DebtLinksInput::default()
    };
    let (houses, debts) = project.houses_and_debts_mut();
    linking::update_house(houses, debts, house, &sold, &settle, today).unwrap();
    let path = dir.path().join("example-a.wealth");
    project.save_as(path.clone()).unwrap();
    let mut reopened = Project::new();
    reopened.open(&path).unwrap();
    assert_eq!(reopened.debts(), project.debts());
    assert_eq!(reopened.debts().house_of(loan), Some(house));
    assert!(reopened.debts().is_settled(loan));
}

#[test]
fn test_csd020_ac_16_2_a_file_without_links_opens_without_links() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("example-a.wealth");
    fs::write(
        &path,
        r#"{"format_version":1,"income":{"jobs":[],"paychecks":[]},
            "debts":{"debts":[{"id":"671166f4-5827-4794-bcdc-54d7824fb650",
            "name":"Example Loan","statement_period":"Monthly","colour_slot":0}],
            "statements":[]}}"#,
    )
    .unwrap();
    let mut project = Project::new();
    project.open(&path).unwrap();
    let debt_id = project.debts().debts()[0].id;
    assert_eq!(project.debts().house_of(debt_id), None);
    assert!(!project.debts().is_settled(debt_id));
}

#[test]
fn test_csd020_ac_16_3_a_damaged_link_is_an_invalid_file() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("example-a.wealth");
    fs::write(
        &path,
        r#"{"format_version":1,"income":{"jobs":[],"paychecks":[]},
            "debts":{"debts":[{"id":"671166f4-5827-4794-bcdc-54d7824fb650",
            "name":"Example Loan","statement_period":"Monthly","colour_slot":0,
            "house_id":"11111111-1111-4111-8111-111111111111"}],
            "statements":[]}}"#,
    )
    .unwrap();
    let mut project = Project::new();
    let error = project.open(&path).unwrap_err();
    assert_eq!(error.code(), "PROJECT_INVALID_FILE");
}

#[test]
fn test_csd020_ac_16_4_a_link_on_a_debt_of_category_other_is_an_invalid_file() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("example-a.wealth");
    fs::write(
        &path,
        r#"{"format_version":1,"income":{"jobs":[],"paychecks":[]},
            "houses":{"houses":[{"id":"11111111-1111-4111-8111-111111111111",
            "name":"Example House","valuation_period":"Monthly","colour_slot":0}],
            "valuations":[]},
            "debts":{"debts":[{"id":"671166f4-5827-4794-bcdc-54d7824fb650",
            "name":"Example Loan","statement_period":"Monthly","colour_slot":0,
            "category":"Other","house_id":"11111111-1111-4111-8111-111111111111"}],
            "statements":[]}}"#,
    )
    .unwrap();
    let mut project = Project::new();
    let error = project.open(&path).unwrap_err();
    assert_eq!(error.code(), "PROJECT_INVALID_FILE");
}

#[test]
fn test_csd015_a_house_change_is_an_unsaved_change() {
    let dir = TempDir::new().unwrap();
    let mut project = Project::new();
    project
        .save_as(dir.path().join("example-a.wealth"))
        .unwrap();
    assert!(!project.status().unsaved_changes);
    house(&mut project, "Example House", "Monthly");
    assert!(project.status().unsaved_changes);
}
