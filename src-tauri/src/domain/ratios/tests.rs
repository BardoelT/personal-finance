use time::macros::date;

use super::*;
use crate::domain::assets::{AssetInput, AssetValue};
use crate::domain::bank_accounts::{AccountInput as BankInput, BankBalance};
use crate::domain::debts::{DebtBalance, DebtInput};
use crate::domain::expenses::ExpenseInput;
use crate::domain::income::JobInput;
use crate::domain::investments::{
    AccountInput as InvestmentInput, Balance, NetDeposits, Note, Period,
};
use crate::domain::money::Amounts;
use crate::domain::ProjectData;

fn today() -> YearMonth {
    YearMonth::parse("2026-10").unwrap()
}

fn dec(text: &str) -> Decimal {
    text.parse().unwrap()
}

fn period(text: &str) -> Period {
    Period::parse(text).unwrap()
}

fn add_income(data: &mut ProjectData, year: &str, gross: &str) {
    let job = data
        .income
        .create_job(&JobInput {
            employer: "Example Employer".to_owned(),
            title: None,
            start_date: Some("01/01/2020".to_owned()),
            end_date: None,
            pay_period: Some("Yearly".to_owned()),
        })
        .unwrap()
        .id;
    data.income
        .add_statement(
            job,
            crate::domain::income::Year::parse(year).unwrap(),
            Amounts::parse(gross, "", "").unwrap(),
            today(),
        )
        .unwrap();
}

fn add_expense(data: &mut ProjectData, on: &str, amount: &str) {
    data.expenses
        .add_expense(
            &ExpenseInput {
                name: "Example Bike".to_owned(),
                amount: amount.to_owned(),
                date: on.to_owned(),
                category: "Other".to_owned(),
                note: None,
            },
            date!(2026 - 10 - 29),
        )
        .unwrap();
}

/// A yearly bank account with the given (year, balance) entries.
fn add_bank(data: &mut ProjectData, name: &str, entries: &[(&str, &str)]) {
    let id = data
        .bank_accounts
        .create_account(&BankInput {
            name: name.to_owned(),
            entry_period: Some("Yearly".to_owned()),
        })
        .unwrap()
        .id;
    for (at, balance) in entries {
        data.bank_accounts
            .add_entry(
                id,
                crate::domain::bank_accounts::parse_period(at).unwrap(),
                BankBalance::parse(balance).unwrap(),
                Note::default(),
                today(),
            )
            .unwrap();
    }
}

/// A yearly investment account with (year, balance, net deposits) statements.
fn add_investment(data: &mut ProjectData, statements: &[(&str, &str, &str)]) {
    let id = data
        .investments
        .create_account(&InvestmentInput {
            name: "Example Brokerage".to_owned(),
            statement_period: Some("Yearly".to_owned()),
        })
        .unwrap()
        .id;
    for (at, balance, deposits) in statements {
        data.investments
            .add_statement(
                id,
                period(at),
                Balance::parse(balance).unwrap(),
                NetDeposits::parse(deposits).unwrap(),
                Note::default(),
                today(),
            )
            .unwrap();
    }
}

/// A yearly debt with the given (year, balance) statements.
fn add_debt(data: &mut ProjectData, statements: &[(&str, &str)]) {
    let id = data
        .debts
        .create_debt(&DebtInput {
            name: "Example Loan".to_owned(),
            statement_period: Some("Yearly".to_owned()),
        })
        .unwrap()
        .id;
    for (at, balance) in statements {
        data.debts
            .add_statement(
                id,
                crate::domain::debts::parse_period(at).unwrap(),
                DebtBalance::parse(balance).unwrap(),
                Note::default(),
                today(),
            )
            .unwrap();
    }
}

fn add_asset(data: &mut ProjectData, at: &str, value: &str) {
    let id = data
        .assets
        .create_asset(
            &AssetInput {
                name: "Example House".to_owned(),
                valuation_period: Some("Yearly".to_owned()),
                purchase_date: Some("01/01/1900".to_owned()),
                purchase_amount: Some("0.00".to_owned()),
                ..AssetInput::default()
            },
            time::macros::date!(2026 - 10 - 29),
        )
        .unwrap()
        .id;
    data.assets
        .add_valuation(
            id,
            crate::domain::assets::parse_period(at).unwrap(),
            AssetValue::parse(value).unwrap(),
            Note::default(),
            today(),
        )
        .unwrap();
}

fn series(data: &ProjectData) -> Vec<RatiosYear> {
    ratios(
        &RatioSources {
            income: &data.income,
            expenses: &data.expenses,
            assets: &data.assets,
            investments: &data.investments,
            bank_accounts: &data.bank_accounts,
            debts: &data.debts,
        },
        today(),
    )
    .years
}

fn year(data: &ProjectData, wanted: i32) -> RatiosYear {
    series(data)
        .into_iter()
        .find(|row| row.year == wanted)
        .unwrap()
}

// REQ-2: year series

#[test]
fn test_csd018_ac_2_1_the_range_covers_all_areas() {
    let mut data = ProjectData::default();
    add_expense(&mut data, "05/03/2024", "10.00");
    add_bank(&mut data, "Example Bank", &[("2025", "100.00")]);
    let years: Vec<i32> = series(&data).iter().map(|row| row.year).collect();
    assert_eq!(years, [2024, 2025, 2026]);
}

#[test]
fn test_csd018_ac_2_1_income_can_start_the_range() {
    let mut data = ProjectData::default();
    add_income(&mut data, "2023", "100.00");
    add_bank(&mut data, "Example Bank", &[("2025", "100.00")]);
    assert_eq!(series(&data)[0].year, 2023);
}

#[test]
fn test_csd018_ac_2_3_no_records_gives_no_years() {
    assert_eq!(series(&ProjectData::default()).len(), 0);
}

#[test]
fn test_csd018_ac_2_2_only_the_current_year_is_partial() {
    let mut data = ProjectData::default();
    add_expense(&mut data, "05/03/2025", "10.00");
    let partial: Vec<bool> = series(&data).iter().map(|row| row.is_partial).collect();
    assert_eq!(partial, [false, true]);
}

// REQ-4: total income

#[test]
fn test_csd018_ac_4_1_income_of_a_year() {
    let mut data = ProjectData::default();
    add_income(&mut data, "2025", "48000.00");
    assert_eq!(year(&data, 2025).total_income, dec("48000.00"));
}

#[test]
fn test_csd018_ac_4_2_a_year_without_income_is_zero() {
    let mut data = ProjectData::default();
    add_expense(&mut data, "05/03/2024", "10.00");
    assert_eq!(year(&data, 2024).total_income.to_string(), "0.00");
}

// REQ-5: wealth accumulation

/// 2025: banks +2000, net deposits 3000, repaid 1000.
fn saving_year() -> ProjectData {
    let mut data = ProjectData::default();
    add_bank(
        &mut data,
        "Example Bank",
        &[("2024", "1000.00"), ("2025", "3000.00")],
    );
    add_investment(&mut data, &[("2025", "3000.00", "3000.00")]);
    add_debt(&mut data, &[("2024", "5000.00"), ("2025", "4000.00")]);
    data
}

#[test]
fn test_csd018_ac_5_1_the_three_parts() {
    let data = saving_year();
    assert_eq!(
        year(&data, 2025).wealth_accumulation.unwrap().to_string(),
        "6000.00"
    );
}

#[test]
fn test_csd018_ac_5_2_parts_can_be_negative() {
    let mut data = ProjectData::default();
    add_bank(
        &mut data,
        "Example Bank",
        &[("2024", "1000.00"), ("2025", "500.00")],
    );
    add_investment(&mut data, &[("2025", "1000.00", "-1000.00")]);
    add_debt(&mut data, &[("2024", "5000.00"), ("2025", "5200.00")]);
    assert_eq!(
        year(&data, 2025).wealth_accumulation.unwrap().to_string(),
        "-1700.00"
    );
}

#[test]
fn test_csd018_ac_5_3_the_bank_increase_compares_with_the_last_earlier_entry() {
    let mut data = ProjectData::default();
    add_bank(
        &mut data,
        "Example Bank",
        &[("2023", "1000.00"), ("2025", "1500.00")],
    );
    assert_eq!(data.bank_accounts.increase(2025), Some(dec("500.00")));
    assert_eq!(data.bank_accounts.increase(2024), None);
}

#[test]
fn test_csd018_ac_5_4_a_new_bank_account_adds_nothing_in_its_first_year() {
    let mut data = ProjectData::default();
    add_bank(&mut data, "Example Bank", &[("2024", "1000.00")]);
    assert_eq!(data.bank_accounts.increase(2024), None);
}

#[test]
fn test_csd018_ac_5_4_only_accounts_with_an_earlier_entry_count() {
    let mut data = ProjectData::default();
    add_bank(
        &mut data,
        "Example Bank",
        &[("2024", "1000.00"), ("2025", "1200.00")],
    );
    add_bank(&mut data, "Example Savings", &[("2025", "9000.00")]);
    assert_eq!(data.bank_accounts.increase(2025), Some(dec("200.00")));
}

#[test]
fn test_csd018_ac_5_5_no_part_available_gives_none() {
    let mut data = ProjectData::default();
    add_expense(&mut data, "05/03/2024", "10.00");
    let row = year(&data, 2024);
    assert_eq!(row.wealth_accumulation, None);
    assert_eq!(row.disposable_income, None);
}

// REQ-6: percentage of income to wealth

#[test]
fn test_csd018_ac_6_1_a_share_of_the_income() {
    let mut data = saving_year();
    add_income(&mut data, "2025", "48000.00");
    assert_eq!(
        year(&data, 2025).wealth_percent.unwrap().to_string(),
        "12.5"
    );
}

#[test]
fn test_csd018_ac_6_2_a_negative_share() {
    let mut data = ProjectData::default();
    add_bank(
        &mut data,
        "Example Bank",
        &[("2024", "1000.00"), ("2025", "500.00")],
    );
    add_investment(&mut data, &[("2025", "1000.00", "-1000.00")]);
    add_debt(&mut data, &[("2024", "5000.00"), ("2025", "5200.00")]);
    add_income(&mut data, "2025", "48000.00");
    assert_eq!(
        year(&data, 2025).wealth_percent.unwrap().to_string(),
        "-3.5"
    );
}

#[test]
fn test_csd018_ac_6_3_no_income_gives_no_percentage() {
    let data = saving_year();
    let row = year(&data, 2025);
    assert_eq!(row.total_income.to_string(), "0.00");
    assert_eq!(row.wealth_percent, None);
    assert_eq!(row.disposable_percent, None);
}

// REQ-7: growth of investments

#[test]
fn test_csd018_ac_7_1_same_figure_as_the_investments_chart() {
    let mut data = ProjectData::default();
    add_investment(
        &mut data,
        &[
            ("2025", "12000.00", "12000.00"),
            ("2026", "12600.00", "100.00"),
        ],
    );
    let chart = data.investments.insights(today());
    let expected = chart
        .iter()
        .find(|r| r.year == 2026)
        .unwrap()
        .growth_percent;
    assert_eq!(year(&data, 2026).investment_growth_percent, expected);
    assert!(expected.is_some());
}

#[test]
fn test_csd018_ac_7_2_the_first_year_has_none() {
    let mut data = ProjectData::default();
    add_investment(&mut data, &[("2025", "12000.00", "12000.00")]);
    assert_eq!(year(&data, 2025).investment_growth_percent, None);
}

// REQ-8: growth of total wealth

fn net_worth_of(first: &str, second: &str) -> ProjectData {
    let mut data = ProjectData::default();
    add_asset(&mut data, "2025", first);
    let id = data.assets.assets()[0].id;
    data.assets
        .add_valuation(
            id,
            crate::domain::assets::parse_period("2026").unwrap(),
            AssetValue::parse(second).unwrap(),
            Note::default(),
            today(),
        )
        .unwrap();
    data
}

#[test]
fn test_csd018_ac_8_1_growth_of_the_net_worth() {
    let data = net_worth_of("100000.00", "110000.00");
    assert_eq!(
        year(&data, 2026).wealth_growth_percent.unwrap().to_string(),
        "10.0"
    );
}

#[test]
fn test_csd018_ac_8_2_a_shrinking_net_worth() {
    let data = net_worth_of("100000.00", "95000.00");
    assert_eq!(
        year(&data, 2026).wealth_growth_percent.unwrap().to_string(),
        "-5.0"
    );
}

#[test]
fn test_csd018_ac_8_3_a_year_before_that_is_not_positive_gives_none() {
    let mut data = ProjectData::default();
    add_debt(&mut data, &[("2025", "5000.00")]);
    add_bank(&mut data, "Example Bank", &[("2026", "1000.00")]);
    assert_eq!(year(&data, 2026).wealth_growth_percent, None);
}

#[test]
fn test_csd018_ac_8_4_a_year_before_without_records_gives_none() {
    let mut data = ProjectData::default();
    add_asset(&mut data, "2024", "100.00");
    add_bank(&mut data, "Example Bank", &[("2026", "1000.00")]);
    assert_eq!(year(&data, 2026).wealth_growth_percent, None);
}

#[test]
fn test_csd018_ac_8_4_a_year_without_records_gives_none() {
    let mut data = ProjectData::default();
    add_asset(&mut data, "2024", "100.00");
    assert_eq!(year(&data, 2025).wealth_growth_percent, None);
}

// REQ-9: free income

#[test]
fn test_csd018_ac_9_1_set_aside_plus_spent() {
    let mut data = saving_year();
    add_expense(&mut data, "10/05/2025", "4000.00");
    assert_eq!(
        year(&data, 2025).disposable_income.unwrap().to_string(),
        "10000.00"
    );
}

#[test]
fn test_csd018_ac_9_2_the_percentage_of_the_income() {
    let mut data = saving_year();
    add_expense(&mut data, "10/05/2025", "4000.00");
    add_income(&mut data, "2025", "48000.00");
    assert_eq!(
        year(&data, 2025).disposable_percent.unwrap().to_string(),
        "20.8"
    );
}

#[test]
fn test_csd018_ac_9_3_no_expenses_counts_zero() {
    let data = saving_year();
    assert_eq!(
        year(&data, 2025).disposable_income.unwrap().to_string(),
        "6000.00"
    );
}

// Section 5: the payload

#[test]
fn test_csd018_section_5_figures_cross_ipc_as_strings_and_none_as_null() {
    let mut data = saving_year();
    add_income(&mut data, "2025", "48000.00");
    let json = serde_json::to_value(Ratios {
        years: series(&data),
    })
    .unwrap();
    let row = json["years"]
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["year"] == 2025)
        .unwrap();
    assert_eq!(row["total_income"], "48000.00");
    assert_eq!(row["wealth_percent"], "12.5");
    assert_eq!(row["wealth_growth_percent"], serde_json::Value::Null);
}
