use super::*;
use crate::domain::assets::{AssetInput, AssetValue};
use crate::domain::bank_accounts::{AccountInput as BankInput, BankBalance};
use crate::domain::debts::{DebtBalance, DebtInput};
use crate::domain::investments::{
    AccountInput as InvestmentInput, Balance, NetDeposits, Note, Period,
};
use crate::domain::ProjectData;

fn today() -> YearMonth {
    YearMonth::parse("2026-10").unwrap()
}

fn period(text: &str) -> Period {
    Period::parse(text).unwrap()
}

fn add_asset(data: &mut ProjectData, at: &str, amount: &str) {
    let id = data
        .assets
        .create_asset(&AssetInput {
            name: "Example House".to_owned(),
            valuation_period: Some("Yearly".to_owned()),
        })
        .unwrap()
        .id;
    data.assets
        .add_valuation(
            id,
            crate::domain::assets::parse_period(at).unwrap(),
            AssetValue::parse(amount).unwrap(),
            Note::default(),
            today(),
        )
        .unwrap();
}

fn add_investment(data: &mut ProjectData, at: &str, amount: &str) {
    let id = match data.investments.accounts().first() {
        Some(account) => account.id,
        None => {
            data.investments
                .create_account(&InvestmentInput {
                    name: "Example Brokerage".to_owned(),
                    statement_period: Some("Yearly".to_owned()),
                })
                .unwrap()
                .id
        }
    };
    data.investments
        .add_statement(
            id,
            period(at),
            Balance::parse(amount).unwrap(),
            NetDeposits::parse(amount).unwrap(),
            Note::default(),
            today(),
        )
        .unwrap();
}

fn add_cash(data: &mut ProjectData, at: &str, amount: &str) {
    let id = data
        .bank_accounts
        .create_account(&BankInput {
            name: "Example Bank".to_owned(),
            entry_period: Some("Yearly".to_owned()),
        })
        .unwrap()
        .id;
    data.bank_accounts
        .add_entry(
            id,
            crate::domain::bank_accounts::parse_period(at).unwrap(),
            BankBalance::parse(amount).unwrap(),
            Note::default(),
            today(),
        )
        .unwrap();
}

fn add_debt(data: &mut ProjectData, at: &str, amount: &str) {
    let id = data
        .debts
        .create_debt(&DebtInput {
            name: "Example Loan".to_owned(),
            statement_period: Some("Yearly".to_owned()),
        })
        .unwrap()
        .id;
    data.debts
        .add_statement(
            id,
            crate::domain::debts::parse_period(at).unwrap(),
            DebtBalance::parse(amount).unwrap(),
            Note::default(),
            today(),
        )
        .unwrap();
}

fn series(data: &ProjectData) -> Vec<NetWorthYear> {
    net_worth_insights(
        &data.assets,
        &data.investments,
        &data.bank_accounts,
        &data.debts,
        today(),
    )
    .years
}

fn years(data: &ProjectData) -> Vec<i32> {
    series(data).iter().map(|row| row.year).collect()
}

fn year(data: &ProjectData, wanted: i32) -> NetWorthYear {
    series(data)
        .into_iter()
        .find(|row| row.year == wanted)
        .unwrap()
}

// REQ-2: year series

#[test]
fn test_csd017_ac_2_1_the_range_covers_all_four_areas() {
    let mut data = ProjectData::default();
    add_cash(&mut data, "2024", "100.00");
    add_asset(&mut data, "2025", "200.00");
    assert_eq!(years(&data), [2024, 2025, 2026]);
}

#[test]
fn test_csd017_ac_2_1_a_debt_can_start_the_range() {
    let mut data = ProjectData::default();
    add_debt(&mut data, "2023", "100.00");
    add_asset(&mut data, "2025", "200.00");
    assert_eq!(years(&data), [2023, 2024, 2025, 2026]);
}

#[test]
fn test_csd017_ac_2_2_no_records_gives_no_years() {
    assert_eq!(series(&ProjectData::default()).len(), 0);
}

#[test]
fn test_csd017_ac_2_2_accounts_without_records_give_no_years() {
    let mut data = ProjectData::default();
    data.bank_accounts
        .create_account(&BankInput {
            name: "Example Bank".to_owned(),
            entry_period: None,
        })
        .unwrap();
    assert_eq!(series(&data).len(), 0);
}

// REQ-3: figures of a year

#[test]
fn test_csd017_ac_3_1_net_worth_adds_the_three_and_subtracts_debts() {
    let mut data = ProjectData::default();
    add_asset(&mut data, "2025", "300000.00");
    add_investment(&mut data, "2025", "20000.00");
    add_cash(&mut data, "2025", "5000.00");
    add_debt(&mut data, "2025", "9000.00");
    let row = year(&data, 2025);
    assert_eq!(row.assets.to_string(), "300000.00");
    assert_eq!(row.investments.to_string(), "20000.00");
    assert_eq!(row.cash.to_string(), "5000.00");
    assert_eq!(row.debts.to_string(), "9000.00");
    assert_eq!(row.net_worth.to_string(), "316000.00");
}

#[test]
fn test_csd017_ac_3_2_figures_equal_those_of_the_area_charts() {
    let mut data = ProjectData::default();
    add_asset(&mut data, "2025", "300000.00");
    add_debt(&mut data, "2025", "9000.00");
    let row = year(&data, 2025);
    let asset_row = data
        .assets
        .insights(today())
        .years
        .into_iter()
        .find(|r| r.year == 2025)
        .unwrap();
    let debt_row = data
        .debts
        .insights(today())
        .years
        .into_iter()
        .find(|r| r.year == 2025)
        .unwrap();
    assert_eq!(row.assets, asset_row.closing_value);
    assert_eq!(row.debts, debt_row.closing_balance);
}

#[test]
fn test_csd017_ac_3_3_debts_can_make_the_net_worth_negative() {
    let mut data = ProjectData::default();
    add_cash(&mut data, "2025", "1000.00");
    add_debt(&mut data, "2025", "9000.00");
    assert_eq!(year(&data, 2025).net_worth.to_string(), "-8000.00");
}

#[test]
fn test_csd017_ac_3_3_an_overdrawn_bank_reduces_cash() {
    let mut data = ProjectData::default();
    add_cash(&mut data, "2025", "-500.00");
    assert_eq!(year(&data, 2025).cash.to_string(), "-500.00");
    assert_eq!(year(&data, 2025).net_worth.to_string(), "-500.00");
}

// REQ-4: years without records

#[test]
fn test_csd017_ac_4_1_a_gap_year_is_zero_and_nothing_is_carried_forward() {
    let mut data = ProjectData::default();
    add_investment(&mut data, "2024", "1000.00");
    add_investment(&mut data, "2026", "1500.00");
    let gap = year(&data, 2025);
    assert_eq!(gap.investments.to_string(), "0.00");
    assert!(!gap.has_records);
    assert_eq!(gap.net_worth.to_string(), "0.00");
}

#[test]
fn test_csd017_ac_4_2_before_the_first_record_of_an_area_it_is_zero() {
    let mut data = ProjectData::default();
    add_cash(&mut data, "2024", "100.00");
    add_debt(&mut data, "2026", "50.00");
    assert_eq!(year(&data, 2024).debts.to_string(), "0.00");
    assert_eq!(year(&data, 2025).debts.to_string(), "0.00");
    assert_eq!(year(&data, 2026).debts.to_string(), "50.00");
}

// REQ-5: current year is partial

#[test]
fn test_csd017_ac_5_1_only_the_current_year_is_partial() {
    let mut data = ProjectData::default();
    add_cash(&mut data, "2025", "100.00");
    let partial: Vec<bool> = series(&data).iter().map(|row| row.is_partial).collect();
    assert_eq!(partial, [false, true]);
}

// REQ-12: only some areas have records

#[test]
fn test_csd017_ac_12_2_areas_without_records_contribute_zero() {
    let mut data = ProjectData::default();
    add_cash(&mut data, "2026", "100.00");
    let row = year(&data, 2026);
    assert_eq!(
        [
            row.assets.to_string(),
            row.investments.to_string(),
            row.debts.to_string()
        ],
        ["0.00", "0.00", "0.00"]
    );
    assert!(row.has_records);
}

// Section 5: the payload

#[test]
fn test_csd017_section_5_amounts_cross_ipc_as_decimal_strings() {
    let mut data = ProjectData::default();
    add_asset(&mut data, "2026", "300000.00");
    let json = serde_json::to_value(NetWorthInsights {
        years: series(&data),
    })
    .unwrap();
    assert_eq!(json["years"][0]["assets"], "300000.00");
    assert_eq!(json["years"][0]["debts"], "0.00");
    assert_eq!(json["years"][0]["has_records"], true);
}
