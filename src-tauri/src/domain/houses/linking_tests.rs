//! Tests for CSD-020: debts linked to a house and settled on its sale.

use rust_decimal::Decimal;
use time::macros::date;

use crate::domain::debts::{debt_note, parse_period, DebtBalance, DebtId, DebtInput, Outcome};
use crate::domain::houses::linking::{self, DebtLinksInput, SettlementChoice};
use crate::domain::houses::{HouseId, HouseInput, HouseValue};
use crate::domain::income::YearMonth;
use crate::domain::investments::Note;
use crate::domain::net_worth::net_worth_insights;
use crate::domain::ratios::{ratios, RatioSources};
use crate::domain::ProjectData;

fn today_date() -> time::Date {
    date!(2026 - 10 - 29)
}

fn today() -> YearMonth {
    YearMonth::parse("2026-10").unwrap()
}

fn dec(text: &str) -> Decimal {
    text.parse().unwrap()
}

fn debt(data: &mut ProjectData, name: &str, kind: &str) -> DebtId {
    data.debts
        .create_debt(&DebtInput {
            name: name.to_owned(),
            statement_period: Some(kind.to_owned()),
        })
        .unwrap()
        .id
}

fn statement(data: &mut ProjectData, id: DebtId, at: &str, balance: &str, note: &str) {
    data.debts
        .add_statement(
            id,
            parse_period(at).unwrap(),
            DebtBalance::parse(balance).unwrap(),
            debt_note(note).unwrap(),
            today(),
        )
        .unwrap();
}

fn balance_of(data: &ProjectData, id: DebtId, at: &str) -> Option<(String, String)> {
    data.debts
        .all_statements()
        .into_iter()
        .find(|(d, s)| d.id == id && s.period == parse_period(at).unwrap())
        .map(|(_, s)| (s.balance.value().to_string(), s.note.as_str().to_owned()))
}

fn links(ids: &[DebtId], choices: &[(DebtId, &str)], confirm_undo: bool) -> DebtLinksInput {
    DebtLinksInput {
        debt_ids: ids.iter().map(ToString::to_string).collect(),
        settlements: choices
            .iter()
            .map(|(id, outcome)| SettlementChoice {
                debt_id: id.to_string(),
                outcome: (*outcome).to_owned(),
            })
            .collect(),
        confirm_undo,
    }
}

fn input(name: &str, sold: (&str, &str)) -> HouseInput {
    HouseInput {
        name: name.to_owned(),
        valuation_period: Some("Monthly".to_owned()),
        purchase_date: Some("15/03/2024".to_owned()),
        purchase_amount: Some("250000.00".to_owned()),
        sell_date: Some(sold.0.to_owned()),
        sell_amount: Some(sold.1.to_owned()),
    }
}

fn add_house(data: &mut ProjectData, name: &str, ids: &[DebtId]) -> HouseId {
    linking::create_house(
        &mut data.houses,
        &mut data.debts,
        &input(name, ("", "")),
        &links(ids, &[], false),
        today_date(),
    )
    .unwrap()
    .id
}

fn save(
    data: &mut ProjectData,
    id: HouseId,
    name: &str,
    sold: (&str, &str),
    links: &DebtLinksInput,
) -> Result<(), crate::domain::DomainError> {
    linking::update_house(
        &mut data.houses,
        &mut data.debts,
        id,
        &input(name, sold),
        links,
        today_date(),
    )
    .map(|_| ())
}

fn code(result: Result<(), crate::domain::DomainError>) -> &'static str {
    result.unwrap_err().code()
}

/// "Example House" with the mortgage (190000.00 in 2026-03) and the loan
/// (10000.00 in 2026-03), both linked, and the two debts.
fn house_with_two_debts() -> (ProjectData, HouseId, DebtId, DebtId) {
    let mut data = ProjectData::default();
    let mortgage = debt(&mut data, "Example Mortgage", "Monthly");
    let loan = debt(&mut data, "Example Loan", "Monthly");
    statement(&mut data, mortgage, "2026-03", "190000.00", "");
    statement(&mut data, loan, "2026-03", "10000.00", "");
    let house = add_house(&mut data, "Example House", &[mortgage, loan]);
    (data, house, mortgage, loan)
}

fn settlement_of(data: &ProjectData, id: DebtId) -> Option<(Outcome, String)> {
    data.debts
        .debts()
        .into_iter()
        .find(|d| d.id == id)
        .and_then(|d| d.settlement.as_ref())
        .map(|s| (s.outcome, s.amount.value().to_string()))
}

// REQ-1: link debts

#[test]
fn test_csd020_ac_1_1_link_two_debts() {
    let mut data = ProjectData::default();
    let mortgage = debt(&mut data, "Example Mortgage", "Monthly");
    let loan = debt(&mut data, "Example Loan", "Monthly");
    let house = add_house(&mut data, "Example House", &[]);
    save(
        &mut data,
        house,
        "Example House",
        ("", ""),
        &links(&[mortgage, loan], &[], false),
    )
    .unwrap();
    assert_eq!(data.debts.house_of(mortgage), Some(house));
    assert_eq!(data.debts.house_of(loan), Some(house));
}

#[test]
fn test_csd020_ac_1_2_unlink_a_debt() {
    let (mut data, house, mortgage, loan) = house_with_two_debts();
    save(
        &mut data,
        house,
        "Example House",
        ("", ""),
        &links(&[mortgage], &[], false),
    )
    .unwrap();
    assert_eq!(data.debts.house_of(loan), None);
    assert_eq!(data.debts.house_of(mortgage), Some(house));
    assert!(balance_of(&data, loan, "2026-03").is_some());
}

#[test]
fn test_csd020_ac_1_3_link_while_adding_a_house() {
    let mut data = ProjectData::default();
    let mortgage = debt(&mut data, "Example Mortgage", "Monthly");
    let house = add_house(&mut data, "Example House", &[mortgage]);
    assert_eq!(data.debts.house_of(mortgage), Some(house));
}

// REQ-2: link rules

#[test]
fn test_csd020_ac_2_1_a_debt_cannot_have_two_houses() {
    let (mut data, _house, mortgage, _loan) = house_with_two_debts();
    let other = add_house(&mut data, "Example Lodge", &[]);
    let result = save(
        &mut data,
        other,
        "Example Lodge",
        ("", ""),
        &links(&[mortgage], &[], false),
    );
    assert_eq!(code(result), "HOUSE_DEBT_LINKED_ELSEWHERE");
}

#[test]
fn test_csd020_ac_2_1_an_unknown_debt_is_rejected() {
    let mut data = ProjectData::default();
    let ghost = debt(&mut data, "Example Ghost", "Monthly");
    let house = add_house(&mut data, "Example House", &[]);
    data.debts.delete_debt(ghost).unwrap();
    let result = save(
        &mut data,
        house,
        "Example House",
        ("", ""),
        &links(&[ghost], &[], false),
    );
    assert_eq!(code(result), "DEBT_NOT_FOUND");
}

#[test]
fn test_csd020_ac_2_1_the_links_of_a_sold_house_are_locked() {
    let mut data = ProjectData::default();
    let loan = debt(&mut data, "Example Loan", "Monthly");
    let house = add_house(&mut data, "Example Guesthouse", &[]);
    save(
        &mut data,
        house,
        "Example Guesthouse",
        ("10/06/2026", "9000.00"),
        &links(&[], &[], false),
    )
    .unwrap();
    let result = save(
        &mut data,
        house,
        "Example Guesthouse",
        ("10/06/2026", "9000.00"),
        &links(&[loan], &[], false),
    );
    assert_eq!(code(result), "HOUSE_LINKS_LOCKED_SOLD");
    assert_eq!(data.debts.house_of(loan), None);
}

// REQ-3, REQ-4: the sale asks for a choice per open debt

#[test]
fn test_csd020_ac_3_1_sell_with_two_outcomes() {
    let (mut data, house, mortgage, loan) = house_with_two_debts();
    save(
        &mut data,
        house,
        "Example House",
        ("10/06/2026", "300000.00"),
        &links(
            &[mortgage, loan],
            &[(mortgage, "Repaid"), (loan, "Cancelled")],
            false,
        ),
    )
    .unwrap();
    assert_eq!(
        settlement_of(&data, mortgage),
        Some((Outcome::Repaid, "190000.00".to_owned()))
    );
    assert_eq!(
        settlement_of(&data, loan),
        Some((Outcome::Cancelled, "10000.00".to_owned()))
    );
    assert!(!data.houses.get(house).unwrap().is_held());
}

#[test]
fn test_csd020_ac_3_2_a_missing_choice_saves_nothing() {
    let (mut data, house, mortgage, loan) = house_with_two_debts();
    let before = data.clone();
    let result = save(
        &mut data,
        house,
        "Example House",
        ("10/06/2026", "300000.00"),
        &links(&[mortgage, loan], &[(mortgage, "Repaid")], false),
    );
    assert_eq!(code(result), "HOUSE_SETTLEMENT_REQUIRED");
    assert_eq!(data, before);
}

#[test]
fn test_csd020_ac_3_3_a_choice_for_a_debt_that_needs_none_is_rejected() {
    let mut data = ProjectData::default();
    let loan = debt(&mut data, "Example Loan", "Monthly");
    let house = add_house(&mut data, "Example House", &[loan]);
    let before = data.clone();
    let result = save(
        &mut data,
        house,
        "Example House",
        ("10/06/2026", "300000.00"),
        &links(&[loan], &[(loan, "Repaid")], false),
    );
    assert_eq!(code(result), "HOUSE_SETTLEMENT_INVALID");
    assert_eq!(data, before);
}

#[test]
fn test_csd020_ac_3_4_a_house_without_open_debts_is_sold_directly() {
    let mut data = ProjectData::default();
    let house = add_house(&mut data, "Example House", &[]);
    save(
        &mut data,
        house,
        "Example House",
        ("10/06/2026", "300000.00"),
        &links(&[], &[], false),
    )
    .unwrap();
    assert!(!data.houses.get(house).unwrap().is_held());
}

#[test]
fn test_csd020_ac_4_1_which_debts_need_a_choice() {
    type Case<'a> = (&'a [(&'a str, &'a str)], Option<&'a str>);
    let cases: [Case; 4] = [
        (
            &[("2026-02", "5000.00"), ("2026-05", "4000.00")],
            Some("4000.00"),
        ),
        (&[("2026-06", "3500.00")], Some("3500.00")),
        (&[], None),
        (&[("2026-04", "0.00")], None),
    ];
    for (statements, expected) in cases {
        let mut data = ProjectData::default();
        let loan = debt(&mut data, "Example Loan", "Monthly");
        for (at, balance) in statements {
            statement(&mut data, loan, at, balance, "");
        }
        let open =
            linking::open_debts(&data.debts, &[loan.to_string()], "10/06/2026", today_date())
                .unwrap();
        assert_eq!(
            open.first().map(|row| row.amount.to_string()).as_deref(),
            expected
        );
    }
}

#[test]
fn test_csd020_ac_4_2_a_statement_after_the_sale_period_blocks_the_sale() {
    let mut data = ProjectData::default();
    let loan = debt(&mut data, "Example Loan", "Monthly");
    statement(&mut data, loan, "2026-04", "4000.00", "");
    statement(&mut data, loan, "2026-08", "3000.00", "");
    let house = add_house(&mut data, "Example House", &[loan]);
    let before = data.clone();
    let result = save(
        &mut data,
        house,
        "Example House",
        ("10/06/2026", "300000.00"),
        &links(&[loan], &[(loan, "Repaid")], false),
    );
    assert_eq!(code(result), "DEBT_STATEMENT_AFTER_SALE");
    assert_eq!(data, before);
}

// REQ-5: the settlement

#[test]
fn test_csd020_ac_5_1_the_settlement_statement() {
    let (mut data, house, mortgage, loan) = house_with_two_debts();
    save(
        &mut data,
        house,
        "Example House",
        ("10/06/2026", "300000.00"),
        &links(
            &[mortgage, loan],
            &[(mortgage, "Repaid"), (loan, "Repaid")],
            false,
        ),
    )
    .unwrap();
    assert_eq!(
        balance_of(&data, mortgage, "2026-06"),
        Some((
            "0.00".to_owned(),
            "Repaid on sale of Example House".to_owned()
        ))
    );
    assert_eq!(
        data.debts.overview().debts[1]
            .latest_balance
            .map(|b| b.to_string()),
        Some("0.00".to_owned())
    );
}

#[test]
fn test_csd020_ac_5_2_a_statement_in_the_sale_period_is_replaced() {
    let mut data = ProjectData::default();
    let loan = debt(&mut data, "Example Loan", "Monthly");
    statement(&mut data, loan, "2026-06", "3500.00", "Mid-June");
    let house = add_house(&mut data, "Example House", &[loan]);
    save(
        &mut data,
        house,
        "Example House",
        ("10/06/2026", "300000.00"),
        &links(&[loan], &[(loan, "Cancelled")], false),
    )
    .unwrap();
    assert_eq!(
        balance_of(&data, loan, "2026-06"),
        Some((
            "0.00".to_owned(),
            "Cancelled on sale of Example House".to_owned()
        ))
    );
    assert_eq!(
        settlement_of(&data, loan),
        Some((Outcome::Cancelled, "3500.00".to_owned()))
    );
    let replaced = data.debts.debts()[0]
        .settlement
        .as_ref()
        .and_then(|s| s.replaced.as_ref())
        .map(|r| (r.balance.value().to_string(), r.note.as_str().to_owned()));
    assert_eq!(
        replaced,
        Some(("3500.00".to_owned(), "Mid-June".to_owned()))
    );
}

#[test]
fn test_csd020_ac_5_3_a_yearly_debt_settles_in_the_sale_year() {
    let mut data = ProjectData::default();
    let mortgage = debt(&mut data, "Example Mortgage", "Yearly");
    statement(&mut data, mortgage, "2025", "200000.00", "");
    let house = add_house(&mut data, "Example House", &[mortgage]);
    save(
        &mut data,
        house,
        "Example House",
        ("10/06/2026", "300000.00"),
        &links(&[mortgage], &[(mortgage, "Repaid")], false),
    )
    .unwrap();
    assert_eq!(balance_of(&data, mortgage, "2026").unwrap().0, "0.00");
    assert_eq!(
        settlement_of(&data, mortgage),
        Some((Outcome::Repaid, "200000.00".to_owned()))
    );
}

// REQ-6: undo

#[test]
fn test_csd020_ac_6_1_undo_restores_the_debts() {
    let mut data = ProjectData::default();
    let loan = debt(&mut data, "Example Loan", "Monthly");
    statement(&mut data, loan, "2026-06", "3500.00", "Mid-June");
    let house = add_house(&mut data, "Example House", &[loan]);
    save(
        &mut data,
        house,
        "Example House",
        ("10/06/2026", "300000.00"),
        &links(&[loan], &[(loan, "Cancelled")], false),
    )
    .unwrap();
    save(
        &mut data,
        house,
        "Example House",
        ("", ""),
        &links(&[loan], &[], true),
    )
    .unwrap();
    assert!(data.houses.get(house).unwrap().is_held());
    assert_eq!(settlement_of(&data, loan), None);
    assert_eq!(data.debts.house_of(loan), Some(house));
    assert_eq!(
        balance_of(&data, loan, "2026-06"),
        Some(("3500.00".to_owned(), "Mid-June".to_owned()))
    );
}

#[test]
fn test_csd020_ac_6_2_undo_without_confirmation() {
    let (mut data, house, mortgage, loan) = house_with_two_debts();
    save(
        &mut data,
        house,
        "Example House",
        ("10/06/2026", "300000.00"),
        &links(
            &[mortgage, loan],
            &[(mortgage, "Repaid"), (loan, "Repaid")],
            false,
        ),
    )
    .unwrap();
    let before = data.clone();
    let result = save(
        &mut data,
        house,
        "Example House",
        ("", ""),
        &links(&[mortgage, loan], &[], false),
    );
    assert_eq!(code(result), "HOUSE_SALE_UNDO_UNCONFIRMED");
    assert_eq!(data, before);
}

// REQ-7, REQ-8: locks

fn sold_house() -> (ProjectData, HouseId, DebtId) {
    let mut data = ProjectData::default();
    let mortgage = debt(&mut data, "Example Mortgage", "Monthly");
    statement(&mut data, mortgage, "2026-03", "190000.00", "");
    let house = add_house(&mut data, "Example House", &[mortgage]);
    save(
        &mut data,
        house,
        "Example House",
        ("10/06/2026", "300000.00"),
        &links(&[mortgage], &[(mortgage, "Repaid")], false),
    )
    .unwrap();
    (data, house, mortgage)
}

#[test]
fn test_csd020_ac_7_1_the_sell_date_is_locked() {
    let (mut data, house, mortgage) = sold_house();
    let result = save(
        &mut data,
        house,
        "Example House",
        ("11/06/2026", "300000.00"),
        &links(&[mortgage], &[], false),
    );
    assert_eq!(code(result), "HOUSE_SELL_DATE_LOCKED");
}

#[test]
fn test_csd020_ac_7_2_the_sell_amount_can_change() {
    let (mut data, house, mortgage) = sold_house();
    save(
        &mut data,
        house,
        "Example House",
        ("10/06/2026", "310000.00"),
        &links(&[mortgage], &[], false),
    )
    .unwrap();
    assert_eq!(
        data.houses
            .get(house)
            .unwrap()
            .sell_amount
            .unwrap()
            .value()
            .to_string(),
        "310000.00"
    );
    assert!(settlement_of(&data, mortgage).is_some());
}

#[test]
fn test_csd020_ac_8_1_a_settled_debt_is_locked() {
    let (mut data, house, mortgage) = sold_house();
    let add = data.debts.add_statement(
        mortgage,
        parse_period("2026-07").unwrap(),
        DebtBalance::parse("0.00").unwrap(),
        Note::default(),
        today(),
    );
    assert_eq!(add.unwrap_err().code(), "DEBT_SETTLED_LOCKED");
    let edit = data.debts.update_statement(
        mortgage,
        parse_period("2026-06").unwrap(),
        parse_period("2026-06").unwrap(),
        DebtBalance::parse("5.00").unwrap(),
        Note::default(),
        today(),
    );
    assert_eq!(edit.unwrap_err().code(), "DEBT_SETTLED_LOCKED");
    let delete = data
        .debts
        .delete_statement(mortgage, parse_period("2026-06").unwrap());
    assert_eq!(delete.unwrap_err().code(), "DEBT_SETTLED_LOCKED");
    let result = save(
        &mut data,
        house,
        "Example House",
        ("10/06/2026", "300000.00"),
        &links(&[], &[], false),
    );
    assert_eq!(code(result), "HOUSE_LINKS_LOCKED_SOLD");
}

#[test]
fn test_csd020_ac_8_2_earlier_statements_stay_editable() {
    let (mut data, _house, mortgage) = sold_house();
    data.debts
        .update_statement(
            mortgage,
            parse_period("2026-03").unwrap(),
            parse_period("2026-03").unwrap(),
            DebtBalance::parse("189000.00").unwrap(),
            Note::default(),
            today(),
        )
        .unwrap();
    assert_eq!(
        settlement_of(&data, mortgage),
        Some((Outcome::Repaid, "190000.00".to_owned()))
    );
}

// REQ-9: deleting

#[test]
fn test_csd020_ac_9_1_a_house_with_linked_debts_cannot_be_deleted() {
    let (mut data, house, _mortgage, _loan) = house_with_two_debts();
    let result = linking::delete_house(&mut data.houses, &data.debts, house);
    assert_eq!(result.unwrap_err().code(), "HOUSE_HAS_LINKED_DEBTS");
    assert!(data.houses.get(house).is_some());
}

#[test]
fn test_csd020_ac_9_2_deleting_a_settled_debt_removes_its_settlement() {
    let (mut data, house, mortgage) = sold_house();
    data.debts.delete_debt(mortgage).unwrap();
    assert_eq!(data.debts.debts().len(), 0);
    assert!(!data.houses.get(house).unwrap().is_held());
}

// REQ-10: net worth and total debt

#[test]
fn test_csd020_ac_10_1_both_outcomes_drop_to_zero() {
    for outcome in ["Repaid", "Cancelled"] {
        let mut data = ProjectData::default();
        let mortgage = debt(&mut data, "Example Mortgage", "Monthly");
        statement(&mut data, mortgage, "2026-03", "190000.00", "");
        let house = add_house(&mut data, "Example House", &[mortgage]);
        data.houses
            .add_valuation(
                house,
                crate::domain::houses::parse_period("2026-03").unwrap(),
                HouseValue::parse("300000.00").unwrap(),
                Note::default(),
                today(),
            )
            .unwrap();
        save(
            &mut data,
            house,
            "Example House",
            ("10/06/2026", "300000.00"),
            &links(&[mortgage], &[(mortgage, outcome)], false),
        )
        .unwrap();
        assert_eq!(
            data.debts.overview().total.map(|t| t.to_string()),
            Some("0.00".to_owned())
        );
        let net = net_worth_insights(
            &data.houses,
            &data.investments,
            &data.bank_accounts,
            &data.debts,
            today(),
        );
        let row = net.years.iter().find(|row| row.year == 2026).unwrap();
        assert_eq!(row.debts, Decimal::ZERO);
        assert_eq!(row.houses, Decimal::ZERO);
        assert_eq!(row.net_worth, Decimal::ZERO);
    }
}

// REQ-11, REQ-12: wealth accumulation

fn repaid_2026(data: &ProjectData) -> Option<Decimal> {
    ratios(
        &RatioSources {
            income: &data.income,
            expenses: &data.expenses,
            houses: &data.houses,
            investments: &data.investments,
            bank_accounts: &data.bank_accounts,
            debts: &data.debts,
        },
        today(),
    )
    .years
    .into_iter()
    .find(|row| row.year == 2026)
    .and_then(|row| row.wealth_accumulation)
}

fn settle_mortgage(outcome: &str, first: &[(&str, &str)], sold_on: &str) -> ProjectData {
    let mut data = ProjectData::default();
    let mortgage = debt(&mut data, "Example Mortgage", "Monthly");
    for (at, balance) in first {
        statement(&mut data, mortgage, at, balance, "");
    }
    let house = add_house(&mut data, "Example House", &[mortgage]);
    save(
        &mut data,
        house,
        "Example House",
        (sold_on, "300000.00"),
        &links(&[mortgage], &[(mortgage, outcome)], false),
    )
    .unwrap();
    data
}

#[test]
fn test_csd020_ac_11_1_a_repaid_debt_counts_in_full() {
    let data = settle_mortgage(
        "Repaid",
        &[("2025-12", "200000.00"), ("2026-03", "190000.00")],
        "10/06/2026",
    );
    assert_eq!(repaid_2026(&data), Some(dec("200000.00")));
}

#[test]
fn test_csd020_ac_12_1_a_cancelled_debt_is_taken_out_of_the_repaid_amount() {
    let data = settle_mortgage(
        "Cancelled",
        &[("2025-12", "200000.00"), ("2026-03", "190000.00")],
        "10/06/2026",
    );
    assert_eq!(repaid_2026(&data), Some(dec("10000.00")));
}

#[test]
fn test_csd020_ac_12_2_a_debt_without_an_opening_balance_adds_nothing() {
    let data = settle_mortgage("Cancelled", &[("2026-02", "5000.00")], "10/06/2026");
    assert_eq!(repaid_2026(&data), None);
}

#[test]
fn test_csd020_ac_12_3_more_cancelled_than_repaid_is_shown_as_is() {
    let data = settle_mortgage(
        "Cancelled",
        &[("2025-12", "200000.00"), ("2026-02", "200000.00")],
        "10/06/2026",
    );
    assert_eq!(repaid_2026(&data), Some(dec("0.00")));
    let grown = settle_mortgage(
        "Cancelled",
        &[("2025-12", "100000.00"), ("2026-02", "150000.00")],
        "10/06/2026",
    );
    assert_eq!(repaid_2026(&grown), Some(dec("-50000.00")));
}

// REQ-13, REQ-14: views

#[test]
fn test_csd020_ac_13_1_the_houses_view_lists_the_linked_debts() {
    let (mut data, house, mortgage, loan) = house_with_two_debts();
    save(
        &mut data,
        house,
        "Example House",
        ("10/06/2026", "300000.00"),
        &links(
            &[mortgage, loan],
            &[(mortgage, "Repaid"), (loan, "Cancelled")],
            false,
        ),
    )
    .unwrap();
    let view = linking::houses_view(&data.houses, &data.debts);
    let rows = &view.houses[0].linked_debts;
    assert_eq!(rows.len(), 2);
    assert!(rows.iter().any(|row| row.debt_name == "Example Loan"
        && row.latest_balance == Some(dec("0.00"))
        && row.settlement.as_ref().is_some_and(
            |s| s.outcome == Outcome::Cancelled && s.amount.value() == dec("10000.00")
        )));
}

#[test]
fn test_csd020_ac_14_1_the_debts_view_names_the_house() {
    let (data, _house, _mortgage, _loan) = house_with_two_debts();
    let view = linking::debts_view(&data.debts, &data.houses);
    assert!(view
        .debts
        .iter()
        .all(|row| row.house_name.as_deref() == Some("Example House")));
}
