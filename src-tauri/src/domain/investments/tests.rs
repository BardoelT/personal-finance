use uuid::Uuid;

use super::amounts::MAX_BALANCE;
use super::*;
use crate::domain::income::{PayPeriod, YearMonth};

fn month(text: &str) -> Period {
    Period::parse(text).unwrap()
}

fn today() -> YearMonth {
    YearMonth::parse("2026-10").unwrap()
}

fn balance(text: &str) -> Balance {
    Balance::parse(text).unwrap()
}

fn no_note() -> Note {
    Note::default()
}

fn deposits(text: &str) -> NetDeposits {
    NetDeposits::parse(text).unwrap()
}

fn no_deposits() -> NetDeposits {
    NetDeposits::default()
}

fn add_with_deposits(
    investments: &mut Investments,
    id: InvestmentAccountId,
    period: &str,
    amount: &str,
    net_deposits: &str,
) {
    investments
        .add_statement(
            id,
            month(period),
            balance(amount),
            deposits(net_deposits),
            no_note(),
            today(),
        )
        .unwrap();
}

/// The capital growth of the statement of `period`, as text.
fn growth(investments: &Investments, id: InvestmentAccountId, period: &str) -> String {
    let (_, statement) = investments
        .all_statements()
        .into_iter()
        .find(|(account, statement)| account.id == id && statement.period == month(period))
        .unwrap();
    investments.capital_growth(statement).to_string()
}

fn calculate(
    investments: &Investments,
    id: InvestmentAccountId,
    period: &str,
    net_deposits: &str,
    input: CalculationInput,
    value: &str,
) -> Result<StatementCalculation, DomainError> {
    investments.calculate(id, month(period), net_deposits, input, value, None)
}

/// A monthly account with a statement for 2026-08 of 12000.00.
fn account_with_august() -> (Investments, InvestmentAccountId) {
    let mut investments = Investments::default();
    let id = add_account(&mut investments, "Example Brokerage", "Monthly");
    add(&mut investments, id, "2026-08", "12000.00");
    (investments, id)
}

fn input(name: &str, period: Option<&str>) -> AccountInput {
    AccountInput {
        name: name.to_owned(),
        statement_period: period.map(str::to_owned),
    }
}

fn add_account(investments: &mut Investments, name: &str, period: &str) -> InvestmentAccountId {
    investments
        .create_account(&input(name, Some(period)))
        .unwrap()
        .id
}

fn add(investments: &mut Investments, id: InvestmentAccountId, period: &str, amount: &str) {
    investments
        .add_statement(
            id,
            month(period),
            balance(amount),
            no_deposits(),
            no_note(),
            today(),
        )
        .unwrap();
}

fn periods(investments: &Investments) -> Vec<String> {
    investments
        .all_statements()
        .into_iter()
        .map(|(account, statement)| format!("{} {}", statement.period, account.name))
        .collect()
}

// ---- REQ-1 to REQ-5: accounts ----

#[test]
fn test_csd005_ac_1_1_an_account_is_monthly_by_default() {
    let mut investments = Investments::default();
    let account = investments
        .create_account(&input("Example Brokerage", None))
        .unwrap();
    assert_eq!(account.name, "Example Brokerage");
    assert_eq!(account.statement_period, PayPeriod::Monthly);
    assert_eq!(investments.statement_count(account.id), 0);
}

#[test]
fn test_csd005_ac_1_2_a_yearly_account() {
    let mut investments = Investments::default();
    let account = investments
        .create_account(&input("Example Pension", Some("Yearly")))
        .unwrap();
    assert_eq!(account.statement_period, PayPeriod::Yearly);
}

#[test]
fn test_csd005_ac_1_3_an_unknown_period_is_rejected() {
    let mut investments = Investments::default();
    let error = investments
        .create_account(&input("Example Brokerage", Some("Weekly")))
        .unwrap_err();
    assert_eq!(error.code(), "INVESTMENT_PERIOD_UNSUPPORTED");
    assert_eq!(investments.accounts().len(), 0);
}

#[test]
fn test_csd005_ac_2_1_invalid_names_are_rejected() {
    for (name, code) in [
        ("", "INVESTMENT_ACCOUNT_NAME_REQUIRED"),
        ("   ", "INVESTMENT_ACCOUNT_NAME_REQUIRED"),
        ("EXAMPLE BROKERAGE", "INVESTMENT_ACCOUNT_NAME_DUPLICATE"),
        ("example brokerage", "INVESTMENT_ACCOUNT_NAME_DUPLICATE"),
    ] {
        let mut investments = Investments::default();
        add_account(&mut investments, "Example Brokerage", "Monthly");
        let error = investments.create_account(&input(name, None)).unwrap_err();
        assert_eq!(error.code(), code, "{name:?}");
        assert_eq!(investments.accounts().len(), 1, "{name:?}");
    }
}

#[test]
fn test_csd005_ac_2_2_the_name_length_limit_counts_characters() {
    let mut investments = Investments::default();
    investments
        .create_account(&input(&"a".repeat(100), None))
        .unwrap();
    let error = investments
        .create_account(&input(&"b".repeat(101), None))
        .unwrap_err();
    assert_eq!(error.code(), "INVESTMENT_ACCOUNT_NAME_TOO_LONG");
    // 100 multi-byte characters are within the limit (CON-8).
    investments
        .create_account(&input(&"é".repeat(100), None))
        .unwrap();
}

#[test]
fn test_csd005_ac_2_3_the_name_is_stored_trimmed() {
    let mut investments = Investments::default();
    let account = investments
        .create_account(&input("  Example Brokerage  ", None))
        .unwrap();
    assert_eq!(account.name, "Example Brokerage");
}

#[test]
fn test_csd005_ac_2_4_renaming_to_an_existing_name_is_rejected() {
    let mut investments = Investments::default();
    add_account(&mut investments, "Example Brokerage", "Monthly");
    let pension = add_account(&mut investments, "Example Pension", "Yearly");
    let error = investments
        .update_account(pension, &input("example brokerage", None))
        .unwrap_err();
    assert_eq!(error.code(), "INVESTMENT_ACCOUNT_NAME_DUPLICATE");
    assert_eq!(investments.accounts().len(), 2);
}

#[test]
fn test_csd005_ac_2_5_changing_only_the_case_of_its_own_name_is_allowed() {
    let mut investments = Investments::default();
    let id = add_account(&mut investments, "Example Brokerage", "Monthly");
    let account = investments
        .update_account(id, &input("EXAMPLE BROKERAGE", None))
        .unwrap();
    assert_eq!(account.name, "EXAMPLE BROKERAGE");
}

#[test]
fn test_csd005_ac_3_1_renaming_keeps_the_statements() {
    let mut investments = Investments::default();
    let id = add_account(&mut investments, "Example Brokerage", "Monthly");
    add(&mut investments, id, "2026-08", "1000.00");
    add(&mut investments, id, "2026-09", "1100.00");
    let account = investments
        .update_account(id, &input("Example Broker", None))
        .unwrap();
    assert_eq!(account.name, "Example Broker");
    assert_eq!(account.statement_period, PayPeriod::Monthly);
    assert_eq!(investments.statement_count(id), 2);
}

#[test]
fn test_csd005_ac_3_2_the_period_of_an_account_without_statements_can_change() {
    let mut investments = Investments::default();
    let id = add_account(&mut investments, "Example Brokerage", "Monthly");
    let account = investments
        .update_account(id, &input("Example Brokerage", Some("Yearly")))
        .unwrap();
    assert_eq!(account.statement_period, PayPeriod::Yearly);
}

#[test]
fn test_csd005_ac_4_1_the_period_is_locked_once_statements_exist() {
    let mut investments = Investments::default();
    let id = add_account(&mut investments, "Example Brokerage", "Monthly");
    add(&mut investments, id, "2026-09", "12500.00");
    let error = investments
        .update_account(id, &input("Example Brokerage", Some("Yearly")))
        .unwrap_err();
    assert_eq!(error.code(), "INVESTMENT_PERIOD_LOCKED");
    assert_eq!(
        investments.accounts()[0].statement_period,
        PayPeriod::Monthly
    );
    // Keeping the same period is not a change.
    investments
        .update_account(id, &input("Example Brokerage", Some("Monthly")))
        .unwrap();
}

#[test]
fn test_csd005_ac_4_2_the_period_can_change_after_deleting_the_last_statement() {
    let mut investments = Investments::default();
    let id = add_account(&mut investments, "Example Brokerage", "Monthly");
    add(&mut investments, id, "2026-09", "12500.00");
    investments.delete_statement(id, month("2026-09")).unwrap();
    let account = investments
        .update_account(id, &input("Example Brokerage", Some("Yearly")))
        .unwrap();
    assert_eq!(account.statement_period, PayPeriod::Yearly);
}

#[test]
fn test_csd005_ac_5_1_deleting_an_account_deletes_its_statements() {
    let mut investments = Investments::default();
    let id = add_account(&mut investments, "Example Brokerage", "Monthly");
    let other = add_account(&mut investments, "Example Crypto", "Monthly");
    for period in ["2026-07", "2026-08", "2026-09"] {
        add(&mut investments, id, period, "1000.00");
    }
    add(&mut investments, other, "2026-09", "50.00");
    investments.delete_account(id).unwrap();
    assert_eq!(investments.accounts().len(), 1);
    assert_eq!(investments.total_statement_count(), 1);
    assert_eq!(
        investments.delete_account(id).unwrap_err().code(),
        "INVESTMENT_ACCOUNT_NOT_FOUND"
    );
}

// ---- REQ-6 to REQ-14: statements ----

#[test]
fn test_csd005_ac_6_1_a_monthly_statement_with_a_note() {
    let mut investments = Investments::default();
    let id = add_account(&mut investments, "Example Brokerage", "Monthly");
    let note = Note::new("Bought example fund").unwrap();
    let statement = investments
        .add_statement(
            id,
            month("2026-09"),
            balance("12500.00"),
            no_deposits(),
            note,
            today(),
        )
        .unwrap();
    assert_eq!(statement.balance.value().to_string(), "12500.00");
    assert_eq!(statement.note.as_str(), "Bought example fund");
}

#[test]
fn test_csd005_ac_6_2_a_yearly_statement_without_a_note() {
    let mut investments = Investments::default();
    let id = add_account(&mut investments, "Example Pension", "Yearly");
    let statement = investments
        .add_statement(
            id,
            month("2025"),
            balance("80000.00"),
            no_deposits(),
            no_note(),
            today(),
        )
        .unwrap();
    assert_eq!(statement.balance.value().to_string(), "80000.00");
    assert_eq!(statement.note.as_str(), "");
}

#[test]
fn test_csd005_ac_6_3_old_periods_are_accepted() {
    let mut investments = Investments::default();
    let id = add_account(&mut investments, "Example Brokerage", "Monthly");
    add(&mut investments, id, "2019-01", "1000.00");
    assert_eq!(investments.statement_count(id), 1);
}

#[test]
fn test_csd005_ac_6_4_the_current_period_is_accepted_and_later_ones_are_not() {
    for (kind, period, ok) in [
        ("Monthly", "2026-10", true),
        ("Monthly", "2026-11", false),
        ("Yearly", "2026", true),
        ("Yearly", "2027", false),
    ] {
        let mut investments = Investments::default();
        let id = add_account(&mut investments, "Example Account", kind);
        let result = investments.add_statement(
            id,
            month(period),
            balance("1000.00"),
            no_deposits(),
            no_note(),
            today(),
        );
        match (ok, result) {
            (true, Ok(_)) => {}
            (false, Err(error)) => {
                assert_eq!(error.code(), "INVESTMENT_STATEMENT_IN_FUTURE");
                assert_eq!(investments.statement_count(id), 0);
            }
            (ok, result) => panic!("{kind} {period}: expected ok={ok}, got {result:?}"),
        }
    }
}

#[test]
fn test_csd005_ac_6_5_an_invalid_period_is_rejected() {
    for text in [
        "2026-13", "2026-9", "abcd", "26", "1899", "", "2026-", "-2026",
    ] {
        assert_eq!(
            Period::parse(text).unwrap_err().code(),
            "INVESTMENT_STATEMENT_PERIOD_INVALID",
            "{text:?}"
        );
    }
    assert_eq!(month("2026-09").to_string(), "2026-09");
    assert_eq!(month(" 2025 ").to_string(), "2025");
}

#[test]
fn test_csd005_ac_7_1_a_duplicate_period_is_rejected_and_changes_nothing() {
    let mut investments = Investments::default();
    let id = add_account(&mut investments, "Example Brokerage", "Monthly");
    let first = Note::new("First").unwrap();
    investments
        .add_statement(
            id,
            month("2026-09"),
            balance("12500.00"),
            no_deposits(),
            first,
            today(),
        )
        .unwrap();
    let error = investments
        .add_statement(
            id,
            month("2026-09"),
            balance("13000.00"),
            no_deposits(),
            no_note(),
            today(),
        )
        .unwrap_err();
    assert_eq!(error.code(), "INVESTMENT_STATEMENT_DUPLICATE");
    let rows = investments.all_statements();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].1.balance.value().to_string(), "12500.00");
    assert_eq!(rows[0].1.note.as_str(), "First");
}

#[test]
fn test_csd005_ac_7_2_the_same_period_on_another_account_is_allowed() {
    let mut investments = Investments::default();
    let first = add_account(&mut investments, "Example Brokerage", "Monthly");
    let second = add_account(&mut investments, "Example Crypto", "Monthly");
    add(&mut investments, first, "2026-09", "1000.00");
    add(&mut investments, second, "2026-09", "50.00");
    assert_eq!(investments.total_statement_count(), 2);
}

#[test]
fn test_csd005_ac_8_1_a_monthly_statement_for_a_yearly_account_is_rejected() {
    let mut investments = Investments::default();
    let id = add_account(&mut investments, "Example Pension", "Yearly");
    let error = investments
        .add_statement(
            id,
            month("2026-09"),
            balance("1000.00"),
            no_deposits(),
            no_note(),
            today(),
        )
        .unwrap_err();
    assert_eq!(error.code(), "INVESTMENT_PERIOD_MISMATCH");
}

#[test]
fn test_csd005_ac_8_2_a_yearly_statement_for_a_monthly_account_is_rejected() {
    let mut investments = Investments::default();
    let id = add_account(&mut investments, "Example Brokerage", "Monthly");
    let error = investments
        .add_statement(
            id,
            month("2025"),
            balance("1000.00"),
            no_deposits(),
            no_note(),
            today(),
        )
        .unwrap_err();
    assert_eq!(error.code(), "INVESTMENT_PERIOD_MISMATCH");
}

#[test]
fn test_csd005_ac_9_1_invalid_balances_are_rejected() {
    for (text, code) in [
        ("-0.01", "INVESTMENT_BALANCE_NEGATIVE"),
        ("1000.999", "INVESTMENT_BALANCE_PRECISION"),
        ("1000000000.01", "INVESTMENT_BALANCE_TOO_LARGE"),
    ] {
        assert_eq!(Balance::parse(text).unwrap_err().code(), code, "{text}");
    }
}

#[test]
fn test_csd005_ac_9_2_boundary_balances_are_accepted() {
    assert_eq!(balance("0.00").value().to_string(), "0.00");
    assert_eq!(balance("0").value().to_string(), "0.00");
    assert_eq!(balance("-0.00").value().to_string(), "0.00");
    assert_eq!(
        balance("1000000000.00").value().to_string(),
        "1000000000.00"
    );
    assert_eq!(balance("12500.5").value().to_string(), "12500.50");
    assert_eq!(MAX_BALANCE.to_string(), "1000000000.00");
}

#[test]
fn test_csd005_ac_9_3_an_invalid_balance_in_an_edit_changes_nothing() {
    // The command parses the balance before the domain is called, so an
    // invalid one never reaches `update_statement`.
    let mut investments = Investments::default();
    let id = add_account(&mut investments, "Example Brokerage", "Monthly");
    add(&mut investments, id, "2026-09", "12500.00");
    assert_eq!(
        Balance::parse("-1.00").unwrap_err().code(),
        "INVESTMENT_BALANCE_NEGATIVE"
    );
    assert_eq!(
        investments.all_statements()[0]
            .1
            .balance
            .value()
            .to_string(),
        "12500.00"
    );
}

#[test]
fn test_csd005_ac_9_4_text_that_is_not_a_plain_number_is_rejected() {
    for text in [
        "", "  ", "abc", "1e3", "1_000", "12500,50", "--1", "1.", ".5", "+5",
    ] {
        assert_eq!(
            Balance::parse(text).unwrap_err().code(),
            "INVESTMENT_BALANCE_INVALID",
            "{text:?}"
        );
    }
}

#[test]
fn test_csd005_ac_10_1_the_note_length_limit_counts_characters() {
    assert!(Note::new(&"a".repeat(2000)).is_ok());
    assert!(Note::new(&"é".repeat(2000)).is_ok());
    assert_eq!(
        Note::new(&"a".repeat(2001)).unwrap_err().code(),
        "INVESTMENT_NOTE_TOO_LONG"
    );
}

#[test]
fn test_csd005_ac_10_2_line_breaks_and_spaces_are_kept() {
    let note = Note::new(" line one\nline two ").unwrap();
    assert_eq!(note.as_str(), " line one\nline two ");
    assert_eq!(note.as_str().lines().count(), 2);
}

#[test]
fn test_csd005_ac_11_1_editing_changes_balance_and_note() {
    let mut investments = Investments::default();
    let id = add_account(&mut investments, "Example Brokerage", "Monthly");
    let first = Note::new("First").unwrap();
    investments
        .add_statement(
            id,
            month("2026-09"),
            balance("12500.00"),
            no_deposits(),
            first,
            today(),
        )
        .unwrap();
    let corrected = Note::new("Corrected").unwrap();
    let statement = investments
        .update_statement(
            id,
            month("2026-09"),
            month("2026-09"),
            balance("12750.00"),
            no_deposits(),
            corrected,
            today(),
        )
        .unwrap();
    assert_eq!(statement.balance.value().to_string(), "12750.00");
    assert_eq!(statement.note.as_str(), "Corrected");
    assert_eq!(investments.total_statement_count(), 1);
}

#[test]
fn test_csd005_ac_11_2_a_note_can_be_cleared() {
    let mut investments = Investments::default();
    let id = add_account(&mut investments, "Example Brokerage", "Monthly");
    let first = Note::new("First").unwrap();
    investments
        .add_statement(
            id,
            month("2026-09"),
            balance("12500.00"),
            no_deposits(),
            first,
            today(),
        )
        .unwrap();
    let statement = investments
        .update_statement(
            id,
            month("2026-09"),
            month("2026-09"),
            balance("12500.00"),
            no_deposits(),
            no_note(),
            today(),
        )
        .unwrap();
    assert_eq!(statement.note.as_str(), "");
}

#[test]
fn test_csd005_ac_11_4_editing_a_missing_statement_is_an_error() {
    let mut investments = Investments::default();
    let id = add_account(&mut investments, "Example Brokerage", "Monthly");
    let error = investments
        .update_statement(
            id,
            month("2026-09"),
            month("2026-09"),
            balance("1.00"),
            no_deposits(),
            no_note(),
            today(),
        )
        .unwrap_err();
    assert_eq!(error.code(), "INVESTMENT_STATEMENT_NOT_FOUND");
}

#[test]
fn test_csd005_ac_12_1_deleting_a_statement_keeps_the_others() {
    let mut investments = Investments::default();
    let id = add_account(&mut investments, "Example Brokerage", "Monthly");
    add(&mut investments, id, "2026-08", "1000.00");
    add(&mut investments, id, "2026-09", "1100.00");
    investments.delete_statement(id, month("2026-09")).unwrap();
    assert_eq!(periods(&investments), ["2026-08 Example Brokerage"]);
    assert_eq!(
        investments
            .delete_statement(id, month("2026-09"))
            .unwrap_err()
            .code(),
        "INVESTMENT_STATEMENT_NOT_FOUND"
    );
}

#[test]
fn test_csd005_ac_13_1_copy_previous_takes_the_latest_balance_and_no_note() {
    let mut investments = Investments::default();
    let id = add_account(&mut investments, "Example Brokerage", "Monthly");
    add(&mut investments, id, "2026-07", "12000.00");
    let note = Note::new("Bought example fund").unwrap();
    investments
        .add_statement(
            id,
            month("2026-08"),
            balance("12500.00"),
            no_deposits(),
            note,
            today(),
        )
        .unwrap();
    let prefill = investments.prefill_from_latest(id).unwrap();
    assert_eq!(prefill.balance.unwrap().to_string(), "12500.00");
    assert_eq!(investments.total_statement_count(), 2);
}

#[test]
fn test_csd005_ac_13_2_copy_previous_without_statements_is_empty() {
    let mut investments = Investments::default();
    let id = add_account(&mut investments, "Example Brokerage", "Monthly");
    assert_eq!(investments.prefill_from_latest(id).unwrap().balance, None);
}

// ---- REQ-15, REQ-16: latest balance, total, ordering ----

#[test]
fn test_csd005_ac_15_1_each_account_shows_its_latest_balance() {
    let mut investments = Investments::default();
    let id = add_account(&mut investments, "Example Brokerage", "Monthly");
    add(&mut investments, id, "2026-07", "12000.00");
    add(&mut investments, id, "2026-09", "12500.00");
    let overview = investments.overview();
    let summary = &overview.accounts[0];
    assert_eq!(summary.latest_period, Some(month("2026-09")));
    assert_eq!(summary.latest_balance.unwrap().to_string(), "12500.00");
    assert_eq!(summary.statement_count, 2);
}

#[test]
fn test_csd005_ac_15_2_the_total_adds_latest_balances_of_different_periods() {
    let mut investments = Investments::default();
    let brokerage = add_account(&mut investments, "Example Brokerage", "Monthly");
    let pension = add_account(&mut investments, "Example Pension", "Yearly");
    add(&mut investments, brokerage, "2026-09", "12500.50");
    add(&mut investments, pension, "2025", "80000.00");
    let overview = investments.overview();
    assert_eq!(overview.total.unwrap().to_string(), "92500.50");
    let latest: Vec<String> = overview
        .accounts
        .iter()
        .filter_map(|summary| summary.latest_period.map(|period| period.to_string()))
        .collect();
    assert_eq!(latest, ["2026-09", "2025"]);
}

#[test]
fn test_csd005_ac_15_3_accounts_without_statements_do_not_count() {
    let mut investments = Investments::default();
    let brokerage = add_account(&mut investments, "Example Brokerage", "Monthly");
    add_account(&mut investments, "Example Crypto", "Monthly");
    add(&mut investments, brokerage, "2026-09", "12500.00");
    let overview = investments.overview();
    assert_eq!(overview.total.unwrap().to_string(), "12500.00");
    assert_eq!(overview.accounts[1].latest_balance, None);
    assert_eq!(overview.accounts[1].latest_period, None);
}

#[test]
fn test_csd005_ac_15_3_without_any_statement_there_is_no_total() {
    let mut investments = Investments::default();
    assert_eq!(investments.overview().total, None);
    add_account(&mut investments, "Example Brokerage", "Monthly");
    assert_eq!(investments.overview().total, None);
}

#[test]
fn test_csd005_ac_15_4_a_new_latest_statement_updates_the_total() {
    let mut investments = Investments::default();
    let id = add_account(&mut investments, "Example Brokerage", "Monthly");
    add(&mut investments, id, "2026-09", "12500.00");
    add(&mut investments, id, "2026-10", "13000.00");
    let overview = investments.overview();
    assert_eq!(overview.accounts[0].latest_period, Some(month("2026-10")));
    assert_eq!(overview.total.unwrap().to_string(), "13000.00");
}

#[test]
fn test_csd005_ac_15_5_an_older_statement_does_not_change_the_latest_balance() {
    let mut investments = Investments::default();
    let id = add_account(&mut investments, "Example Brokerage", "Monthly");
    add(&mut investments, id, "2026-09", "12500.00");
    add(&mut investments, id, "2026-01", "9000.00");
    let overview = investments.overview();
    assert_eq!(overview.accounts[0].latest_period, Some(month("2026-09")));
    assert_eq!(overview.total.unwrap().to_string(), "12500.00");
}

#[test]
fn test_csd005_ac_16_1_accounts_are_alphabetical_ignoring_case() {
    let mut investments = Investments::default();
    add_account(&mut investments, "example pension", "Yearly");
    add_account(&mut investments, "Example Brokerage", "Monthly");
    add_account(&mut investments, "Example Crypto", "Monthly");
    let names: Vec<&str> = investments
        .accounts()
        .into_iter()
        .map(|account| account.name.as_str())
        .collect();
    assert_eq!(
        names,
        ["Example Brokerage", "Example Crypto", "example pension"]
    );
}

#[test]
fn test_csd005_ac_16_2_the_newest_statement_is_first() {
    let mut investments = Investments::default();
    let id = add_account(&mut investments, "Example Brokerage", "Monthly");
    add(&mut investments, id, "2026-07", "1000.00");
    add(&mut investments, id, "2026-09", "1100.00");
    assert_eq!(
        periods(&investments),
        ["2026-09 Example Brokerage", "2026-07 Example Brokerage"]
    );
}

#[test]
fn test_csd005_ac_16_3_a_yearly_statement_counts_as_december() {
    let mut investments = Investments::default();
    let brokerage = add_account(&mut investments, "Example Brokerage", "Monthly");
    let pension = add_account(&mut investments, "Example Pension", "Yearly");
    add(&mut investments, brokerage, "2025-11", "1000.00");
    add(&mut investments, brokerage, "2025-12", "1100.00");
    add(&mut investments, pension, "2025", "80000.00");
    assert_eq!(
        periods(&investments),
        [
            "2025-12 Example Brokerage",
            "2025 Example Pension",
            "2025-11 Example Brokerage"
        ]
    );
}

#[test]
fn test_csd005_ac_16_4_the_same_period_is_ordered_by_account() {
    let mut investments = Investments::default();
    let crypto = add_account(&mut investments, "Example Crypto", "Monthly");
    let brokerage = add_account(&mut investments, "Example Brokerage", "Monthly");
    add(&mut investments, crypto, "2026-09", "50.00");
    add(&mut investments, brokerage, "2026-09", "1000.00");
    assert_eq!(
        periods(&investments),
        ["2026-09 Example Brokerage", "2026-09 Example Crypto"]
    );
}

// ---- REQ-19: data read from a file ----

#[test]
fn test_csd005_ac_19_1_serialising_and_reading_back_gives_the_same_data() {
    let mut investments = Investments::default();
    let id = add_account(&mut investments, "Example Pension", "Yearly");
    let note = Note::new("Line one\nLine two").unwrap();
    investments
        .add_statement(
            id,
            month("2025"),
            balance("80000.00"),
            no_deposits(),
            note,
            today(),
        )
        .unwrap();
    let json = serde_json::to_string(&investments).unwrap();
    assert!(json.contains("\"balance\":\"80000.00\""), "{json}");
    let back: Investments = serde_json::from_str(&json).unwrap();
    assert_eq!(back, investments);
    back.validate().unwrap();
}

#[test]
fn test_csd005_ac_19_2_a_missing_section_is_empty() {
    let investments: Investments = serde_json::from_str("{}").unwrap();
    assert_eq!(investments, Investments::default());
    investments.validate().unwrap();
}

// ---- REQ-13: net deposits are not copied ----

#[test]
fn test_csd005_ac_13_3_net_deposits_are_not_copied() {
    let mut investments = Investments::default();
    let id = add_account(&mut investments, "Example Brokerage", "Monthly");
    add_with_deposits(&mut investments, id, "2026-08", "12500.00", "300.00");
    let prefill = investments.prefill_from_latest(id).unwrap();
    assert_eq!(prefill.balance.unwrap().to_string(), "12500.00");
    assert_eq!(prefill.net_deposits.to_string(), "0.00");
}

// ---- REQ-19: net deposits in the file ----

#[test]
fn test_csd005_ac_19_3_net_deposits_round_trip() {
    let mut investments = Investments::default();
    let id = add_account(&mut investments, "Example Brokerage", "Monthly");
    add_with_deposits(&mut investments, id, "2026-09", "12500.00", "-250.00");
    let json = serde_json::to_string(&investments).unwrap();
    assert!(json.contains("\"net_deposits\":\"-250.00\""), "{json}");
    let back: Investments = serde_json::from_str(&json).unwrap();
    assert_eq!(back, investments);
    back.validate().unwrap();
}

#[test]
fn test_csd005_ac_19_4_a_statement_without_net_deposits_gets_zero() {
    let id = Uuid::new_v4();
    let json = format!(
        r#"{{"accounts":[{{"id":"{id}","name":"Example","statement_period":"Monthly"}}],
            "statements":[{{"account_id":"{id}","period":"2026-09","balance":"12500.00"}}]}}"#
    );
    let investments: Investments = serde_json::from_str(&json).unwrap();
    investments.validate().unwrap();
    let rows = investments.all_statements();
    assert_eq!(rows[0].1.net_deposits.value().to_string(), "0.00");
}

#[test]
fn validate_rejects_net_deposits_out_of_range() {
    let id = Uuid::new_v4();
    let json = format!(
        r#"{{"accounts":[{{"id":"{id}","name":"Example","statement_period":"Monthly"}}],
            "statements":[{{"account_id":"{id}","period":"2026-09","balance":"1.00",
            "net_deposits":"1000000000.01"}}]}}"#
    );
    let investments: Investments = serde_json::from_str(&json).unwrap();
    assert_eq!(
        investments.validate().unwrap_err().code(),
        "INVESTMENT_DATA_INVALID"
    );
}

// ---- REQ-21, REQ-22: net deposits ----

#[test]
fn test_csd005_ac_21_1_net_deposits_default_to_zero() {
    let mut investments = Investments::default();
    let id = add_account(&mut investments, "Example Brokerage", "Monthly");
    let statement = investments
        .add_statement(
            id,
            month("2026-09"),
            balance("12500.00"),
            NetDeposits::parse("").unwrap(),
            no_note(),
            today(),
        )
        .unwrap();
    assert_eq!(statement.net_deposits.value().to_string(), "0.00");
    assert_eq!(
        NetDeposits::parse("  ").unwrap().value().to_string(),
        "0.00"
    );
}

#[test]
fn test_csd005_ac_21_2_a_deposit_is_stored() {
    let mut investments = Investments::default();
    let id = add_account(&mut investments, "Example Brokerage", "Monthly");
    add_with_deposits(&mut investments, id, "2026-09", "12500.00", "300.00");
    let rows = investments.all_statements();
    assert_eq!(rows[0].1.net_deposits.value().to_string(), "300.00");
}

#[test]
fn test_csd005_ac_21_3_a_net_withdrawal_is_stored() {
    let mut investments = Investments::default();
    let id = add_account(&mut investments, "Example Brokerage", "Monthly");
    add_with_deposits(&mut investments, id, "2026-09", "12500.00", "-250.00");
    let rows = investments.all_statements();
    assert_eq!(rows[0].1.net_deposits.value().to_string(), "-250.00");
}

#[test]
fn test_csd005_ac_21_4_editing_changes_the_net_deposits() {
    let mut investments = Investments::default();
    let id = add_account(&mut investments, "Example Brokerage", "Monthly");
    add_with_deposits(&mut investments, id, "2026-09", "12500.00", "300.00");
    let statement = investments
        .update_statement(
            id,
            month("2026-09"),
            month("2026-09"),
            balance("12500.00"),
            deposits("0.00"),
            no_note(),
            today(),
        )
        .unwrap();
    assert_eq!(statement.balance.value().to_string(), "12500.00");
    assert_eq!(statement.net_deposits.value().to_string(), "0.00");
}

#[test]
fn test_csd005_ac_22_1_invalid_net_deposits_are_rejected() {
    for (text, code) in [
        ("abc", "INVESTMENT_NET_DEPOSITS_INVALID"),
        ("1e3", "INVESTMENT_NET_DEPOSITS_INVALID"),
        ("300,50", "INVESTMENT_NET_DEPOSITS_INVALID"),
        ("100.999", "INVESTMENT_NET_DEPOSITS_PRECISION"),
        ("-100.999", "INVESTMENT_NET_DEPOSITS_PRECISION"),
        ("1000000000.01", "INVESTMENT_NET_DEPOSITS_TOO_LARGE"),
        ("-1000000000.01", "INVESTMENT_NET_DEPOSITS_TOO_LARGE"),
    ] {
        assert_eq!(NetDeposits::parse(text).unwrap_err().code(), code, "{text}");
    }
}

#[test]
fn test_csd005_ac_22_2_boundary_net_deposits_are_accepted() {
    for (text, shown) in [
        ("0.00", "0.00"),
        ("-0.00", "0.00"),
        ("1000000000.00", "1000000000.00"),
        ("-1000000000.00", "-1000000000.00"),
        ("300.5", "300.50"),
    ] {
        assert_eq!(deposits(text).value().to_string(), shown, "{text}");
    }
}

#[test]
fn test_csd005_ac_22_3_invalid_net_deposits_in_an_edit_change_nothing() {
    // The command parses the net deposits before the domain is called.
    let mut investments = Investments::default();
    let id = add_account(&mut investments, "Example Brokerage", "Monthly");
    add_with_deposits(&mut investments, id, "2026-09", "12500.00", "300.00");
    assert_eq!(
        NetDeposits::parse("100.999").unwrap_err().code(),
        "INVESTMENT_NET_DEPOSITS_PRECISION"
    );
    let rows = investments.all_statements();
    assert_eq!(rows[0].1.net_deposits.value().to_string(), "300.00");
}

// ---- REQ-23: capital growth ----

#[test]
fn test_csd005_ac_23_1_growth_with_a_deposit() {
    let (mut investments, id) = account_with_august();
    add_with_deposits(&mut investments, id, "2026-09", "12500.00", "300.00");
    assert_eq!(growth(&investments, id, "2026-09"), "200.00");
}

#[test]
fn test_csd005_ac_23_2_a_loss_is_a_negative_growth() {
    let (mut investments, id) = account_with_august();
    add(&mut investments, id, "2026-09", "11000.00");
    assert_eq!(growth(&investments, id, "2026-09"), "-1000.00");
}

#[test]
fn test_csd005_ac_23_3_the_first_statement_of_an_account_has_a_growth() {
    let mut investments = Investments::default();
    let id = add_account(&mut investments, "Example Brokerage", "Monthly");
    add_with_deposits(&mut investments, id, "2026-09", "1020.00", "1000.00");
    assert_eq!(growth(&investments, id, "2026-09"), "20.00");
}

#[test]
fn test_csd005_ac_23_9_a_first_statement_with_an_existing_balance_and_no_net_deposits() {
    let mut investments = Investments::default();
    let id = add_account(&mut investments, "Example Brokerage", "Monthly");
    add(&mut investments, id, "2026-09", "12500.00");
    assert_eq!(growth(&investments, id, "2026-09"), "12500.00");
}

#[test]
fn test_csd005_ac_23_10_an_existing_balance_entered_as_net_deposits_gives_no_growth() {
    let mut investments = Investments::default();
    let id = add_account(&mut investments, "Example Brokerage", "Monthly");
    add_with_deposits(&mut investments, id, "2026-09", "12500.00", "12500.00");
    assert_eq!(growth(&investments, id, "2026-09"), "0.00");
}

#[test]
fn test_csd005_ac_23_4_skipped_periods_are_bridged() {
    let mut investments = Investments::default();
    let id = add_account(&mut investments, "Example Brokerage", "Monthly");
    add(&mut investments, id, "2026-06", "10000.00");
    add_with_deposits(&mut investments, id, "2026-09", "10900.00", "600.00");
    assert_eq!(growth(&investments, id, "2026-09"), "300.00");
}

#[test]
fn test_csd005_ac_23_5_a_yearly_account() {
    let mut investments = Investments::default();
    let id = add_account(&mut investments, "Example Pension", "Yearly");
    add(&mut investments, id, "2024", "70000.00");
    add_with_deposits(&mut investments, id, "2025", "80000.00", "6000.00");
    assert_eq!(growth(&investments, id, "2025"), "4000.00");
}

#[test]
fn test_csd005_ac_23_5_the_previous_statement_is_of_the_same_account() {
    let mut investments = Investments::default();
    let brokerage = add_account(&mut investments, "Example Brokerage", "Monthly");
    let crypto = add_account(&mut investments, "Example Crypto", "Monthly");
    add(&mut investments, crypto, "2026-08", "50.00");
    add(&mut investments, brokerage, "2026-09", "12500.00");
    // The other account's statement is not the previous one: the account starts at 0.00.
    assert_eq!(growth(&investments, brokerage, "2026-09"), "12500.00");
}

#[test]
fn test_csd005_ac_23_6_editing_an_earlier_balance_changes_the_later_growth() {
    let (mut investments, id) = account_with_august();
    add_with_deposits(&mut investments, id, "2026-09", "12500.00", "300.00");
    investments
        .update_statement(
            id,
            month("2026-08"),
            month("2026-08"),
            balance("12100.00"),
            no_deposits(),
            no_note(),
            today(),
        )
        .unwrap();
    assert_eq!(growth(&investments, id, "2026-09"), "100.00");
}

#[test]
fn test_csd005_ac_23_7_a_statement_added_before_the_first_gives_it_a_previous_statement() {
    let mut investments = Investments::default();
    let id = add_account(&mut investments, "Example Brokerage", "Monthly");
    add(&mut investments, id, "2026-09", "12500.00");
    add_with_deposits(&mut investments, id, "2026-07", "12000.00", "12000.00");
    assert_eq!(growth(&investments, id, "2026-07"), "0.00");
    assert_eq!(growth(&investments, id, "2026-09"), "500.00");
}

#[test]
fn test_csd005_ac_23_8_deleting_the_previous_statement_changes_the_growth() {
    let mut investments = Investments::default();
    let id = add_account(&mut investments, "Example Brokerage", "Monthly");
    add(&mut investments, id, "2026-07", "12000.00");
    add(&mut investments, id, "2026-08", "12200.00");
    add(&mut investments, id, "2026-09", "12500.00");
    investments.delete_statement(id, month("2026-08")).unwrap();
    assert_eq!(growth(&investments, id, "2026-09"), "500.00");
}

#[test]
fn test_csd005_ac_24_9_growth_is_never_stored_in_the_file() {
    let (mut investments, id) = account_with_august();
    add_with_deposits(&mut investments, id, "2026-09", "12500.00", "300.00");
    let json = serde_json::to_string(&investments).unwrap();
    assert!(!json.contains("growth"), "{json}");
}

// ---- REQ-24, REQ-25: the form calculation ----

#[test]
fn test_csd005_ac_24_1_a_balance_gives_the_growth() {
    let (investments, id) = account_with_august();
    let result = calculate(
        &investments,
        id,
        "2026-09",
        "",
        CalculationInput::Balance,
        "12500.00",
    )
    .unwrap();
    assert_eq!(result.balance.to_string(), "12500.00");
    assert_eq!(result.capital_growth.to_string(), "500.00");
}

#[test]
fn test_csd005_ac_24_2_a_growth_gives_the_balance() {
    let (investments, id) = account_with_august();
    let result = calculate(
        &investments,
        id,
        "2026-09",
        "300.00",
        CalculationInput::Growth,
        "200.00",
    )
    .unwrap();
    assert_eq!(result.balance.to_string(), "12500.00");
    assert_eq!(result.capital_growth.to_string(), "200.00");
}

#[test]
fn test_csd005_ac_24_3_changing_net_deposits_recalculates_the_growth() {
    let (investments, id) = account_with_august();
    let result = calculate(
        &investments,
        id,
        "2026-09",
        "300.00",
        CalculationInput::Balance,
        "12500.00",
    )
    .unwrap();
    assert_eq!(result.balance.to_string(), "12500.00");
    assert_eq!(result.capital_growth.to_string(), "200.00");
}

#[test]
fn test_csd005_ac_24_4_changing_net_deposits_recalculates_the_balance() {
    let (investments, id) = account_with_august();
    let result = calculate(
        &investments,
        id,
        "2026-09",
        "300.00",
        CalculationInput::Growth,
        "200.00",
    )
    .unwrap();
    assert_eq!(result.balance.to_string(), "12500.00");
    assert_eq!(result.capital_growth.to_string(), "200.00");
}

#[test]
fn test_csd005_ac_24_5_a_negative_growth_and_net_withdrawal() {
    let (investments, id) = account_with_august();
    let result = calculate(
        &investments,
        id,
        "2026-09",
        "-500.00",
        CalculationInput::Growth,
        "-250.50",
    )
    .unwrap();
    assert_eq!(result.balance.to_string(), "11249.50");
}

#[test]
fn test_csd005_ac_24_7_the_first_statement_gets_a_calculated_growth() {
    let mut investments = Investments::default();
    let id = add_account(&mut investments, "Example Brokerage", "Monthly");
    let result = calculate(
        &investments,
        id,
        "2026-09",
        "1000.00",
        CalculationInput::Balance,
        "1020.00",
    )
    .unwrap();
    assert_eq!(result.capital_growth.to_string(), "20.00");
}

#[test]
fn test_csd005_ac_24_12_a_growth_for_a_first_statement_gives_the_balance() {
    let mut investments = Investments::default();
    let id = add_account(&mut investments, "Example Brokerage", "Monthly");
    let result = calculate(
        &investments,
        id,
        "2026-09",
        "1000.00",
        CalculationInput::Growth,
        "200.00",
    )
    .unwrap();
    assert_eq!(result.balance.to_string(), "1200.00");
    assert_eq!(result.capital_growth.to_string(), "200.00");
}

#[test]
fn test_csd005_ac_24_8_a_period_before_all_statements_calculates_from_zero() {
    let mut investments = Investments::default();
    let id = add_account(&mut investments, "Example Brokerage", "Monthly");
    add(&mut investments, id, "2026-09", "12500.00");
    let result = calculate(
        &investments,
        id,
        "2026-07",
        "1000.00",
        CalculationInput::Growth,
        "20.00",
    )
    .unwrap();
    assert_eq!(result.balance.to_string(), "1020.00");
}

#[test]
fn test_csd005_ac_24_9_calculating_stores_nothing() {
    let (investments, id) = account_with_august();
    let before = investments.clone();
    calculate(
        &investments,
        id,
        "2026-09",
        "300.00",
        CalculationInput::Growth,
        "200.00",
    )
    .unwrap();
    assert_eq!(investments, before);
}

#[test]
fn test_csd005_ac_24_10_the_edit_form_shows_the_calculated_growth() {
    let (mut investments, id) = account_with_august();
    add_with_deposits(&mut investments, id, "2026-09", "12500.00", "300.00");
    // Opening the statement calculates the growth from its stored balance
    // and net deposits, without the statement itself being the previous one.
    let result = calculate(
        &investments,
        id,
        "2026-09",
        "300.00",
        CalculationInput::Balance,
        "12500.00",
    )
    .unwrap();
    assert_eq!(result.capital_growth.to_string(), "200.00");
}

#[test]
fn test_csd005_ac_24_11_a_yearly_account_calculates_against_the_previous_year() {
    let mut investments = Investments::default();
    let id = add_account(&mut investments, "Example Pension", "Yearly");
    add(&mut investments, id, "2024", "70000.00");
    let result = investments
        .calculate(
            id,
            month("2025"),
            "6000.00",
            CalculationInput::Balance,
            "80000.00",
            None,
        )
        .unwrap();
    assert_eq!(result.capital_growth.to_string(), "4000.00");
}

#[test]
fn test_csd005_ac_25_1_a_growth_that_gives_a_negative_balance_is_rejected() {
    let (investments, id) = account_with_august();
    let error = calculate(
        &investments,
        id,
        "2026-09",
        "0.00",
        CalculationInput::Growth,
        "-12500.00",
    )
    .unwrap_err();
    assert_eq!(error.code(), "INVESTMENT_BALANCE_NEGATIVE");
}

#[test]
fn test_csd005_ac_25_2_invalid_growth_is_rejected() {
    let (investments, id) = account_with_august();
    for (text, code) in [
        ("abc", "INVESTMENT_GROWTH_INVALID"),
        ("", "INVESTMENT_GROWTH_INVALID"),
        ("100.999", "INVESTMENT_GROWTH_PRECISION"),
        ("1000000000.01", "INVESTMENT_GROWTH_TOO_LARGE"),
        ("-1000000000.01", "INVESTMENT_GROWTH_TOO_LARGE"),
    ] {
        let error = calculate(
            &investments,
            id,
            "2026-09",
            "",
            CalculationInput::Growth,
            text,
        )
        .unwrap_err();
        assert_eq!(error.code(), code, "{text:?}");
    }
}

#[test]
fn test_csd005_ac_25_3_a_growth_that_gives_a_balance_above_the_limit_is_rejected() {
    let mut investments = Investments::default();
    let id = add_account(&mut investments, "Example Brokerage", "Monthly");
    add(&mut investments, id, "2026-08", "999999999.00");
    let error = calculate(
        &investments,
        id,
        "2026-09",
        "0.00",
        CalculationInput::Growth,
        "2.00",
    )
    .unwrap_err();
    assert_eq!(error.code(), "INVESTMENT_BALANCE_TOO_LARGE");
}

#[test]
fn test_csd005_ac_25_4_invalid_net_deposits_in_the_form_are_rejected() {
    let (investments, id) = account_with_august();
    for (text, code) in [
        ("100.999", "INVESTMENT_NET_DEPOSITS_PRECISION"),
        ("abc", "INVESTMENT_NET_DEPOSITS_INVALID"),
        ("1000000000.01", "INVESTMENT_NET_DEPOSITS_TOO_LARGE"),
    ] {
        let error = calculate(
            &investments,
            id,
            "2026-09",
            text,
            CalculationInput::Balance,
            "12500.00",
        )
        .unwrap_err();
        assert_eq!(error.code(), code, "{text}");
    }
}

#[test]
fn test_csd005_ac_25_5_an_invalid_balance_in_the_form_is_rejected() {
    let (investments, id) = account_with_august();
    let error = calculate(
        &investments,
        id,
        "2026-09",
        "",
        CalculationInput::Balance,
        "-1.00",
    )
    .unwrap_err();
    assert_eq!(error.code(), "INVESTMENT_BALANCE_NEGATIVE");
}

#[test]
fn calculating_needs_an_existing_account_and_a_matching_period() {
    let (investments, id) = account_with_august();
    let missing = InvestmentAccountId::parse(&Uuid::new_v4().to_string()).unwrap();
    assert_eq!(
        calculate(
            &investments,
            missing,
            "2026-09",
            "",
            CalculationInput::Balance,
            "1.00"
        )
        .unwrap_err()
        .code(),
        "INVESTMENT_ACCOUNT_NOT_FOUND"
    );
    assert_eq!(
        investments
            .calculate(
                id,
                month("2026"),
                "",
                CalculationInput::Balance,
                "1.00",
                None
            )
            .unwrap_err()
            .code(),
        "INVESTMENT_PERIOD_MISMATCH"
    );
}

// ---- REQ-26: yearly totals ----

/// One row per year as `(year, closing balance, net deposits, growth, statements)`.
fn yearly(investments: &Investments) -> Vec<(i32, String, String, String, usize)> {
    investments
        .yearly_totals()
        .into_iter()
        .map(|total| {
            (
                total.year,
                total.closing_balance.to_string(),
                total.net_deposits.to_string(),
                total.capital_growth.to_string(),
                total.statement_count,
            )
        })
        .collect()
}

fn row(
    year: i32,
    closing: &str,
    net_deposits: &str,
    growth: &str,
    count: usize,
) -> (i32, String, String, String, usize) {
    (
        year,
        closing.to_owned(),
        net_deposits.to_owned(),
        growth.to_owned(),
        count,
    )
}

#[test]
fn test_csd005_ac_26_1_totals_per_calendar_year() {
    let mut investments = Investments::default();
    let id = add_account(&mut investments, "Example Brokerage", "Monthly");
    add_with_deposits(&mut investments, id, "2025-11", "11000.00", "11000.00");
    add_with_deposits(&mut investments, id, "2025-12", "11500.00", "200.00");
    add_with_deposits(&mut investments, id, "2026-05", "12500.00", "300.00");
    assert_eq!(
        yearly(&investments),
        [
            row(2026, "12500.00", "300.00", "700.00", 1),
            row(2025, "11500.00", "11200.00", "300.00", 2),
        ]
    );
}

#[test]
fn test_csd005_ac_26_2_no_statements_give_no_totals() {
    let mut investments = Investments::default();
    assert_eq!(yearly(&investments), Vec::new());
    add_account(&mut investments, "Example Brokerage", "Monthly");
    assert_eq!(yearly(&investments), Vec::new());
}

#[test]
fn test_csd005_ac_26_3_totals_add_up_all_accounts() {
    let mut investments = Investments::default();
    let brokerage = add_account(&mut investments, "Example Brokerage", "Monthly");
    let crypto = add_account(&mut investments, "Example Crypto", "Monthly");
    add_with_deposits(
        &mut investments,
        brokerage,
        "2026-08",
        "12000.00",
        "12000.00",
    );
    add_with_deposits(&mut investments, brokerage, "2026-09", "12500.00", "300.00");
    add_with_deposits(&mut investments, crypto, "2026-09", "500.00", "500.00");
    assert_eq!(
        yearly(&investments),
        [row(2026, "13000.00", "12800.00", "200.00", 3)]
    );
}

#[test]
fn test_csd005_ac_26_4_the_closing_balance_is_the_latest_balance_of_the_year() {
    let mut investments = Investments::default();
    let id = add_account(&mut investments, "Example Brokerage", "Monthly");
    add(&mut investments, id, "2026-03", "10000.00");
    add(&mut investments, id, "2026-09", "12000.00");
    assert_eq!(yearly(&investments)[0].1, "12000.00");
}

#[test]
fn test_csd005_ac_26_5_monthly_and_yearly_accounts_add_up_together() {
    let mut investments = Investments::default();
    let brokerage = add_account(&mut investments, "Example Brokerage", "Monthly");
    let pension = add_account(&mut investments, "Example Pension", "Yearly");
    add_with_deposits(
        &mut investments,
        brokerage,
        "2025-12",
        "12000.00",
        "12000.00",
    );
    add_with_deposits(&mut investments, pension, "2024", "70000.00", "70000.00");
    add_with_deposits(&mut investments, pension, "2025", "80000.00", "6000.00");
    assert_eq!(
        yearly(&investments),
        [
            row(2025, "92000.00", "18000.00", "4000.00", 2),
            row(2024, "70000.00", "70000.00", "0.00", 1),
        ]
    );
}

#[test]
fn test_csd005_ac_26_6_an_account_without_a_statement_in_a_year_does_not_count_in_it() {
    let mut investments = Investments::default();
    let brokerage = add_account(&mut investments, "Example Brokerage", "Monthly");
    let pension = add_account(&mut investments, "Example Pension", "Yearly");
    add(&mut investments, brokerage, "2025-12", "12000.00");
    add(&mut investments, brokerage, "2026-03", "12500.00");
    add(&mut investments, pension, "2025", "80000.00");
    let totals = yearly(&investments);
    assert_eq!((totals[0].1.as_str(), totals[0].4), ("12500.00", 1));
    assert_eq!(totals[1].1, "92000.00");
}

#[test]
fn test_csd005_ac_26_7_growth_after_a_gap_counts_in_the_later_year() {
    let mut investments = Investments::default();
    let id = add_account(&mut investments, "Example Brokerage", "Monthly");
    add_with_deposits(&mut investments, id, "2025-12", "12000.00", "12000.00");
    add_with_deposits(&mut investments, id, "2026-03", "12500.00", "100.00");
    let totals = yearly(&investments);
    assert_eq!(totals[0].3, "400.00");
    assert_eq!(totals[1].3, "0.00");
}

#[test]
fn test_csd005_ac_26_11_a_first_statement_adds_its_growth() {
    let mut investments = Investments::default();
    let id = add_account(&mut investments, "Example Brokerage", "Monthly");
    add_with_deposits(&mut investments, id, "2026-03", "1020.00", "1000.00");
    assert_eq!(
        yearly(&investments),
        [row(2026, "1020.00", "1000.00", "20.00", 1)]
    );
}

#[test]
fn test_csd005_ac_26_8_a_net_withdrawal_and_a_loss_are_negative() {
    let mut investments = Investments::default();
    let id = add_account(&mut investments, "Example Brokerage", "Monthly");
    add(&mut investments, id, "2025-12", "12000.00");
    add_with_deposits(&mut investments, id, "2026-03", "11000.00", "-500.00");
    let totals = yearly(&investments);
    assert_eq!(
        (totals[0].2.as_str(), totals[0].3.as_str()),
        ("-500.00", "-500.00")
    );
}

#[test]
fn test_csd005_ac_26_9_a_new_statement_updates_the_totals() {
    let mut investments = Investments::default();
    let id = add_account(&mut investments, "Example Brokerage", "Monthly");
    add_with_deposits(&mut investments, id, "2026-09", "12500.00", "12500.00");
    add_with_deposits(&mut investments, id, "2026-10", "13000.00", "100.00");
    assert_eq!(
        yearly(&investments),
        [row(2026, "13000.00", "12600.00", "400.00", 2)]
    );
}

#[test]
fn test_csd005_ac_26_10_the_newest_year_is_first() {
    let mut investments = Investments::default();
    let id = add_account(&mut investments, "Example Pension", "Yearly");
    for year in ["2025", "2024", "2026"] {
        add(&mut investments, id, year, "1000.00");
    }
    let years: Vec<i32> = yearly(&investments).iter().map(|row| row.0).collect();
    assert_eq!(years, [2026, 2025, 2024]);
}

// ---- CSD-007: investment insights ----

fn insight_years(investments: &Investments) -> Vec<i32> {
    investments
        .insights(today())
        .iter()
        .map(|row| row.year)
        .collect()
}

fn insight(investments: &Investments, year: i32) -> InvestmentInsightsYear {
    investments
        .insights(today())
        .into_iter()
        .find(|row| row.year == year)
        .unwrap()
}

/// `(opening, net deposits, growth, closing)` as text, after checking that
/// the parts add up to the closing balance (REQ-5).
fn parts(row: &InvestmentInsightsYear) -> (String, String, String, String) {
    assert_eq!(
        row.opening_balance + row.net_deposits + row.capital_growth,
        row.closing_balance,
        "the parts of {} do not add up",
        row.year
    );
    (
        row.opening_balance.to_string(),
        row.net_deposits.to_string(),
        row.capital_growth.to_string(),
        row.closing_balance.to_string(),
    )
}

fn four(
    opening: &str,
    net_deposits: &str,
    growth: &str,
    closing: &str,
) -> (String, String, String, String) {
    (
        opening.to_owned(),
        net_deposits.to_owned(),
        growth.to_owned(),
        closing.to_owned(),
    )
}

fn percent_of(investments: &Investments, year: i32) -> Option<String> {
    insight(investments, year)
        .growth_percent
        .map(|percent| percent.to_string())
}

#[test]
fn test_csd007_ac_2_1_the_series_runs_from_the_first_statement_to_the_current_year() {
    let mut investments = Investments::default();
    let id = add_account(&mut investments, "Example Brokerage", "Monthly");
    add(&mut investments, id, "2024-03", "1000.00");
    add(&mut investments, id, "2025-05", "1100.00");
    add(&mut investments, id, "2026-09", "1200.00");
    assert_eq!(insight_years(&investments), [2024, 2025, 2026]);
}

#[test]
fn test_csd007_ac_2_2_a_single_year() {
    let mut investments = Investments::default();
    let id = add_account(&mut investments, "Example Brokerage", "Monthly");
    add(&mut investments, id, "2026-09", "1000.00");
    assert_eq!(insight_years(&investments), [2026]);
}

#[test]
fn test_csd007_ac_2_3_no_statements_give_no_series() {
    let mut investments = Investments::default();
    assert_eq!(insight_years(&investments), Vec::<i32>::new());
    add_account(&mut investments, "Example Brokerage", "Monthly");
    assert_eq!(insight_years(&investments), Vec::<i32>::new());
}

#[test]
fn test_csd007_ac_3_1_a_gap_year_is_zero() {
    let mut investments = Investments::default();
    let id = add_account(&mut investments, "Example Brokerage", "Monthly");
    add(&mut investments, id, "2022-06", "1000.00");
    add(&mut investments, id, "2024-06", "1200.00");
    assert_eq!(insight_years(&investments), [2022, 2023, 2024, 2025, 2026]);
    let gap = insight(&investments, 2023);
    assert_eq!(parts(&gap), four("0.00", "0.00", "0.00", "0.00"));
    assert_eq!(gap.statement_count, 0);
}

#[test]
fn test_csd007_ac_3_2_the_year_after_a_gap_opens_with_the_balance_from_before_it() {
    let mut investments = Investments::default();
    let id = add_account(&mut investments, "Example Brokerage", "Monthly");
    add(&mut investments, id, "2024-12", "10000.00");
    add_with_deposits(&mut investments, id, "2026-06", "11000.00", "500.00");
    assert_eq!(insight(&investments, 2025).statement_count, 0);
    assert_eq!(
        parts(&insight(&investments, 2026)),
        four("10000.00", "500.00", "500.00", "11000.00")
    );
}

#[test]
fn test_csd007_ac_4_1_the_current_year_is_partial() {
    let mut investments = Investments::default();
    let id = add_account(&mut investments, "Example Brokerage", "Monthly");
    add(&mut investments, id, "2025-05", "1000.00");
    add(&mut investments, id, "2026-05", "1100.00");
    assert!(insight(&investments, 2026).is_partial);
    assert!(!insight(&investments, 2025).is_partial);
}

#[test]
fn test_csd007_ac_4_2_a_current_year_without_statements_is_still_listed() {
    let mut investments = Investments::default();
    let id = add_account(&mut investments, "Example Brokerage", "Monthly");
    add(&mut investments, id, "2025-12", "1000.00");
    assert_eq!(insight_years(&investments), [2025, 2026]);
    let current = insight(&investments, 2026);
    assert!(current.is_partial);
    assert_eq!(current.statement_count, 0);
}

#[test]
fn test_csd007_ac_5_1_the_parts_of_a_year() {
    let mut investments = Investments::default();
    let id = add_account(&mut investments, "Example Brokerage", "Monthly");
    add_with_deposits(&mut investments, id, "2025-11", "11000.00", "11000.00");
    add_with_deposits(&mut investments, id, "2025-12", "11500.00", "200.00");
    add_with_deposits(&mut investments, id, "2026-05", "12500.00", "300.00");
    assert_eq!(
        parts(&insight(&investments, 2025)),
        four("0.00", "11200.00", "300.00", "11500.00")
    );
    assert_eq!(
        parts(&insight(&investments, 2026)),
        four("11500.00", "300.00", "700.00", "12500.00")
    );
}

#[test]
fn test_csd007_ac_5_2_an_account_that_starts_in_a_later_year() {
    let mut investments = Investments::default();
    let brokerage = add_account(&mut investments, "Example Brokerage", "Monthly");
    let crypto = add_account(&mut investments, "Example Crypto", "Monthly");
    add_with_deposits(
        &mut investments,
        brokerage,
        "2025-12",
        "12000.00",
        "12000.00",
    );
    add_with_deposits(&mut investments, brokerage, "2026-03", "12600.00", "100.00");
    add_with_deposits(&mut investments, crypto, "2026-03", "500.00", "400.00");
    assert_eq!(
        parts(&insight(&investments, 2026)),
        four("12000.00", "500.00", "600.00", "13100.00")
    );
}

#[test]
fn test_csd007_ac_5_3_an_account_without_a_statement_in_the_year_is_left_out() {
    let mut investments = Investments::default();
    let brokerage = add_account(&mut investments, "Example Brokerage", "Monthly");
    let pension = add_account(&mut investments, "Example Pension", "Yearly");
    add_with_deposits(
        &mut investments,
        brokerage,
        "2025-12",
        "12000.00",
        "12000.00",
    );
    add(&mut investments, brokerage, "2026-03", "12500.00");
    add_with_deposits(&mut investments, pension, "2025", "80000.00", "80000.00");
    assert_eq!(
        parts(&insight(&investments, 2025)),
        four("0.00", "92000.00", "0.00", "92000.00")
    );
    assert_eq!(
        parts(&insight(&investments, 2026)),
        four("12000.00", "0.00", "500.00", "12500.00")
    );
}

#[test]
fn test_csd007_ac_5_4_a_net_withdrawal_and_a_loss() {
    let mut investments = Investments::default();
    let id = add_account(&mut investments, "Example Brokerage", "Monthly");
    add_with_deposits(&mut investments, id, "2025-12", "12000.00", "12000.00");
    add_with_deposits(&mut investments, id, "2026-03", "11000.00", "-500.00");
    assert_eq!(
        parts(&insight(&investments, 2026)),
        four("12000.00", "-500.00", "-500.00", "11000.00")
    );
}

#[test]
fn test_csd007_ac_5_5_closing_balance_net_deposits_and_growth_equal_the_yearly_totals() {
    let mut investments = Investments::default();
    let brokerage = add_account(&mut investments, "Example Brokerage", "Monthly");
    let crypto = add_account(&mut investments, "Example Crypto", "Monthly");
    let pension = add_account(&mut investments, "Example Pension", "Yearly");
    add_with_deposits(
        &mut investments,
        brokerage,
        "2025-11",
        "11000.00",
        "11000.00",
    );
    add_with_deposits(&mut investments, brokerage, "2025-12", "11500.00", "200.00");
    add_with_deposits(&mut investments, brokerage, "2026-03", "12600.00", "100.00");
    add_with_deposits(&mut investments, crypto, "2026-03", "500.00", "400.00");
    add_with_deposits(&mut investments, pension, "2024", "70000.00", "70000.00");
    add_with_deposits(&mut investments, pension, "2025", "80000.00", "6000.00");
    for total in investments.yearly_totals() {
        let row = insight(&investments, total.year);
        assert_eq!(row.closing_balance, total.closing_balance, "{}", total.year);
        assert_eq!(row.net_deposits, total.net_deposits, "{}", total.year);
        assert_eq!(row.capital_growth, total.capital_growth, "{}", total.year);
        parts(&row);
    }
}

#[test]
fn test_csd007_ac_5_6_a_first_statement_has_growth_and_net_deposits() {
    let mut investments = Investments::default();
    let id = add_account(&mut investments, "Example Brokerage", "Monthly");
    add_with_deposits(&mut investments, id, "2026-03", "1020.00", "1000.00");
    assert_eq!(
        parts(&insight(&investments, 2026)),
        four("0.00", "1000.00", "20.00", "1020.00")
    );
}

#[test]
fn test_csd007_ac_6_1_the_growth_percentage_per_year() {
    for (opening, closing, percent) in [
        ("11500.00", "12200.00", "6.1"),
        ("40000.00", "39000.00", "-2.5"),
        ("20000.00", "20090.00", "0.4"),
        ("20000.00", "20000.00", "0.0"),
    ] {
        let mut investments = Investments::default();
        let id = add_account(&mut investments, "Example Brokerage", "Monthly");
        add_with_deposits(&mut investments, id, "2024-12", opening, opening);
        add(&mut investments, id, "2025-12", closing);
        assert_eq!(
            percent_of(&investments, 2025).as_deref(),
            Some(percent),
            "{opening} -> {closing}"
        );
    }
}

#[test]
fn test_csd007_ac_6_2_growth_of_an_account_without_an_opening_balance_is_left_out() {
    let mut investments = Investments::default();
    let brokerage = add_account(&mut investments, "Example Brokerage", "Monthly");
    let crypto = add_account(&mut investments, "Example Crypto", "Monthly");
    add_with_deposits(
        &mut investments,
        brokerage,
        "2025-12",
        "12000.00",
        "12000.00",
    );
    add_with_deposits(&mut investments, brokerage, "2026-03", "12600.00", "100.00");
    add_with_deposits(&mut investments, crypto, "2026-03", "500.00", "400.00");
    assert_eq!(
        insight(&investments, 2026).capital_growth.to_string(),
        "600.00"
    );
    assert_eq!(percent_of(&investments, 2026).as_deref(), Some("4.2"));
}

#[test]
fn test_csd007_ac_7_1_the_first_year_has_no_percentage() {
    let mut investments = Investments::default();
    let id = add_account(&mut investments, "Example Brokerage", "Monthly");
    add(&mut investments, id, "2025-12", "12000.00");
    add(&mut investments, id, "2026-03", "12500.00");
    assert_eq!(percent_of(&investments, 2025), None);
}

#[test]
fn test_csd007_ac_7_2_a_gap_year_has_no_percentage() {
    let mut investments = Investments::default();
    let id = add_account(&mut investments, "Example Brokerage", "Monthly");
    add(&mut investments, id, "2022-06", "1000.00");
    add(&mut investments, id, "2024-06", "1200.00");
    assert_eq!(percent_of(&investments, 2023), None);
}

#[test]
fn test_csd007_ac_7_3_the_current_year_has_a_percentage() {
    let mut investments = Investments::default();
    let id = add_account(&mut investments, "Example Brokerage", "Monthly");
    add_with_deposits(&mut investments, id, "2025-12", "12500.00", "12500.00");
    add(&mut investments, id, "2026-05", "12750.00");
    assert_eq!(percent_of(&investments, 2026).as_deref(), Some("2.0"));
    assert!(insight(&investments, 2026).is_partial);
}

#[test]
fn test_csd007_ac_7_4_a_year_with_only_new_accounts_has_no_percentage() {
    let mut investments = Investments::default();
    let brokerage = add_account(&mut investments, "Example Brokerage", "Monthly");
    let crypto = add_account(&mut investments, "Example Crypto", "Monthly");
    add(&mut investments, brokerage, "2025-12", "12000.00");
    add(&mut investments, crypto, "2026-03", "500.00");
    assert_eq!(percent_of(&investments, 2026), None);
}

#[test]
fn statements_after_the_current_year_are_not_in_the_series() {
    let id = Uuid::new_v4();
    let json = format!(
        r#"{{"accounts":[{{"id":"{id}","name":"Example","statement_period":"Monthly"}}],
            "statements":[
            {{"account_id":"{id}","period":"2026-03","balance":"1000.00"}},
            {{"account_id":"{id}","period":"2027-03","balance":"2000.00"}}]}}"#
    );
    let investments: Investments = serde_json::from_str(&json).unwrap();
    assert_eq!(insight_years(&investments), [2026]);
}

#[test]
fn validate_rejects_inconsistent_files() {
    let id = Uuid::new_v4();
    let other = Uuid::new_v4();
    let cases = [
        // duplicate names, ignoring case
        format!(
            r#"{{"accounts":[
                {{"id":"{id}","name":"Example","statement_period":"Monthly"}},
                {{"id":"{other}","name":"EXAMPLE","statement_period":"Monthly"}}]}}"#
        ),
        // statement of an unknown account
        format!(
            r#"{{"statements":[{{"account_id":"{id}","period":"2026-09","balance":"1.00"}}]}}"#
        ),
        // statement period does not match the account
        format!(
            r#"{{"accounts":[{{"id":"{id}","name":"Example","statement_period":"Yearly"}}],
                "statements":[{{"account_id":"{id}","period":"2026-09","balance":"1.00"}}]}}"#
        ),
        // duplicate period
        format!(
            r#"{{"accounts":[{{"id":"{id}","name":"Example","statement_period":"Monthly"}}],
                "statements":[
                {{"account_id":"{id}","period":"2026-09","balance":"1.00"}},
                {{"account_id":"{id}","period":"2026-09","balance":"2.00"}}]}}"#
        ),
        // negative balance
        format!(
            r#"{{"accounts":[{{"id":"{id}","name":"Example","statement_period":"Monthly"}}],
                "statements":[{{"account_id":"{id}","period":"2026-09","balance":"-1.00"}}]}}"#
        ),
        // blank name
        format!(r#"{{"accounts":[{{"id":"{id}","name":"  ","statement_period":"Monthly"}}]}}"#),
    ];
    for json in cases {
        let investments: Investments = serde_json::from_str(&json).unwrap();
        assert_eq!(
            investments.validate().unwrap_err().code(),
            "INVESTMENT_DATA_INVALID",
            "{json}"
        );
    }
}

#[test]
fn an_unparseable_period_in_a_file_fails_to_read() {
    let json = r#"{"statements":[{"account_id":"00000000-0000-0000-0000-000000000000","period":"2026-13","balance":"1.00"}]}"#;
    assert!(serde_json::from_str::<Investments>(json).is_err());
}

// ---- REQ-28 and REQ-29: change the period of a statement ----

/// Moves the statement of `from` to `to` with the given balance, keeping no
/// net deposits and no note, as of `today()`.
fn edit_period(
    investments: &mut Investments,
    id: InvestmentAccountId,
    from: &str,
    to: &str,
    amount: &str,
) -> Result<InvestmentStatement, DomainError> {
    investments.update_statement(
        id,
        month(from),
        month(to),
        balance(amount),
        no_deposits(),
        no_note(),
        today(),
    )
}

fn periods_of(investments: &Investments, id: InvestmentAccountId) -> Vec<String> {
    investments
        .all_statements()
        .into_iter()
        .filter(|(account, _)| account.id == id)
        .map(|(_, statement)| statement.period.to_string())
        .collect()
}

#[test]
fn test_csd005_ac_28_1_a_statement_moves_to_another_month() {
    let mut investments = Investments::default();
    let id = add_account(&mut investments, "Example Brokerage", "Monthly");
    let first = Note::new("First").unwrap();
    investments
        .add_statement(
            id,
            month("2026-09"),
            balance("12500.00"),
            no_deposits(),
            first,
            today(),
        )
        .unwrap();
    let moved = investments
        .update_statement(
            id,
            month("2026-09"),
            month("2026-08"),
            balance("12500.00"),
            no_deposits(),
            Note::new("First").unwrap(),
            today(),
        )
        .unwrap();
    assert_eq!(moved.period.to_string(), "2026-08");
    assert_eq!(moved.balance.value().to_string(), "12500.00");
    assert_eq!(moved.note.as_str(), "First");
    assert_eq!(periods_of(&investments, id), ["2026-08"]);
}

#[test]
fn test_csd005_ac_28_2_the_period_and_the_balance_change_together() {
    let mut investments = Investments::default();
    let id = add_account(&mut investments, "Example Brokerage", "Monthly");
    add(&mut investments, id, "2026-09", "12500.00");
    let moved = edit_period(&mut investments, id, "2026-09", "2026-08", "12750.00").unwrap();
    assert_eq!(moved.balance.value().to_string(), "12750.00");
    assert_eq!(periods_of(&investments, id), ["2026-08"]);
}

#[test]
fn test_csd005_ac_28_3_a_yearly_statement_moves_to_another_year() {
    let mut investments = Investments::default();
    let id = add_account(&mut investments, "Example Pension", "Yearly");
    add(&mut investments, id, "2025", "12500.00");
    let moved = edit_period(&mut investments, id, "2025", "2024", "12500.00").unwrap();
    assert_eq!(moved.period.to_string(), "2024");
    assert_eq!(periods_of(&investments, id), ["2024"]);
}

#[test]
fn test_csd005_ac_28_4_the_capital_growth_follows_the_move() {
    let mut investments = Investments::default();
    let id = add_account(&mut investments, "Example Brokerage", "Monthly");
    add_with_deposits(&mut investments, id, "2026-07", "10000.00", "10000.00");
    add(&mut investments, id, "2026-08", "10500.00");
    add(&mut investments, id, "2026-09", "11000.00");
    edit_period(&mut investments, id, "2026-08", "2026-10", "10500.00").unwrap();
    assert_eq!(growth(&investments, id, "2026-07"), "0.00");
    assert_eq!(growth(&investments, id, "2026-09"), "1000.00");
    assert_eq!(growth(&investments, id, "2026-10"), "-500.00");
}

#[test]
fn test_csd005_ac_28_5_the_yearly_totals_follow_the_move() {
    let mut investments = Investments::default();
    let id = add_account(&mut investments, "Example Brokerage", "Monthly");
    add_with_deposits(&mut investments, id, "2025-11", "11000.00", "11000.00");
    add_with_deposits(&mut investments, id, "2025-12", "11500.00", "200.00");
    add_with_deposits(&mut investments, id, "2026-05", "12500.00", "300.00");
    investments
        .update_statement(
            id,
            month("2025-12"),
            month("2026-01"),
            balance("11500.00"),
            deposits("200.00"),
            no_note(),
            today(),
        )
        .unwrap();
    assert_eq!(
        yearly(&investments),
        [
            row(2026, "12500.00", "500.00", "1000.00", 2),
            row(2025, "11000.00", "11000.00", "0.00", 1),
        ]
    );
}

#[test]
fn test_csd005_ac_28_7_keeping_the_period_changes_only_the_values() {
    let mut investments = Investments::default();
    let id = add_account(&mut investments, "Example Brokerage", "Monthly");
    add(&mut investments, id, "2026-09", "12500.00");
    let updated = edit_period(&mut investments, id, "2026-09", "2026-09", "12750.00").unwrap();
    assert_eq!(updated.balance.value().to_string(), "12750.00");
    assert_eq!(periods_of(&investments, id), ["2026-09"]);
}

#[test]
fn test_csd005_ac_29_1_a_period_in_the_future_is_rejected() {
    let mut investments = Investments::default();
    let monthly = add_account(&mut investments, "Example Brokerage", "Monthly");
    let yearly_id = add_account(&mut investments, "Example Pension", "Yearly");
    add(&mut investments, monthly, "2026-09", "12500.00");
    add(&mut investments, yearly_id, "2025", "12500.00");

    let error = edit_period(&mut investments, monthly, "2026-09", "2026-11", "9.00").unwrap_err();
    assert_eq!(error.code(), "INVESTMENT_STATEMENT_IN_FUTURE");
    let error = edit_period(&mut investments, yearly_id, "2025", "2027", "9.00").unwrap_err();
    assert_eq!(error.code(), "INVESTMENT_STATEMENT_IN_FUTURE");
    // Nothing changed, balances included.
    assert_eq!(periods_of(&investments, monthly), ["2026-09"]);
    assert_eq!(periods_of(&investments, yearly_id), ["2025"]);
    let balances: Vec<String> = investments
        .all_statements()
        .into_iter()
        .map(|(_, s)| s.balance.value().to_string())
        .collect();
    assert_eq!(balances, ["12500.00", "12500.00"]);

    // The current month and the current year are allowed.
    edit_period(&mut investments, monthly, "2026-09", "2026-10", "12500.00").unwrap();
    edit_period(&mut investments, yearly_id, "2025", "2026", "12500.00").unwrap();
}

#[test]
fn test_csd005_ac_29_2_a_period_that_already_has_a_statement_is_rejected() {
    let mut investments = Investments::default();
    let id = add_account(&mut investments, "Example Brokerage", "Monthly");
    add(&mut investments, id, "2026-08", "12000.00");
    add(&mut investments, id, "2026-09", "12500.00");
    let error = edit_period(&mut investments, id, "2026-09", "2026-08", "13000.00").unwrap_err();
    assert_eq!(error.code(), "INVESTMENT_STATEMENT_DUPLICATE");
    let balances: Vec<String> = investments
        .all_statements()
        .into_iter()
        .map(|(_, s)| s.balance.value().to_string())
        .collect();
    // Newest first: 2026-09 and 2026-08, both as before.
    assert_eq!(balances, ["12500.00", "12000.00"]);
}

#[test]
fn test_csd005_ac_29_3_a_period_of_the_wrong_kind_is_rejected() {
    let mut investments = Investments::default();
    let monthly = add_account(&mut investments, "Example Brokerage", "Monthly");
    let yearly_id = add_account(&mut investments, "Example Pension", "Yearly");
    add(&mut investments, monthly, "2026-09", "12500.00");
    add(&mut investments, yearly_id, "2025", "12500.00");

    let error = edit_period(&mut investments, monthly, "2026-09", "2026", "12500.00").unwrap_err();
    assert_eq!(error.code(), "INVESTMENT_PERIOD_MISMATCH");
    let error =
        edit_period(&mut investments, yearly_id, "2025", "2024-12", "12500.00").unwrap_err();
    assert_eq!(error.code(), "INVESTMENT_PERIOD_MISMATCH");
    assert_eq!(periods_of(&investments, monthly), ["2026-09"]);
    assert_eq!(periods_of(&investments, yearly_id), ["2025"]);
}

#[test]
fn test_csd005_ac_29_5_another_account_may_have_the_same_period() {
    let mut investments = Investments::default();
    let first = add_account(&mut investments, "Example Brokerage", "Monthly");
    let second = add_account(&mut investments, "Example Crypto", "Monthly");
    add(&mut investments, first, "2026-08", "12000.00");
    add(&mut investments, second, "2026-09", "500.00");
    edit_period(&mut investments, second, "2026-09", "2026-08", "500.00").unwrap();
    assert_eq!(periods_of(&investments, first), ["2026-08"]);
    assert_eq!(periods_of(&investments, second), ["2026-08"]);
}

#[test]
fn test_csd005_ac_24_13_the_statement_being_edited_is_not_its_own_previous_statement() {
    let mut investments = Investments::default();
    let id = add_account(&mut investments, "Example Brokerage", "Monthly");
    add(&mut investments, id, "2026-07", "10000.00");
    add(&mut investments, id, "2026-08", "10500.00");
    // Moving 2026-08 to 2026-09: the previous statement is 2026-07, not 2026-08.
    let result = investments
        .calculate(
            id,
            month("2026-09"),
            "",
            CalculationInput::Balance,
            "11000.00",
            Some(month("2026-08")),
        )
        .unwrap();
    assert_eq!(result.capital_growth.to_string(), "1000.00");
    // Without the exception the statement at 2026-08 would count.
    let result = calculate(
        &investments,
        id,
        "2026-09",
        "",
        CalculationInput::Balance,
        "11000.00",
    )
    .unwrap();
    assert_eq!(result.capital_growth.to_string(), "500.00");
}

#[test]
fn test_csd005_ac_24_14_editing_without_a_period_change_uses_the_earlier_statement() {
    let mut investments = Investments::default();
    let id = add_account(&mut investments, "Example Brokerage", "Monthly");
    add(&mut investments, id, "2026-07", "10000.00");
    add(&mut investments, id, "2026-08", "10500.00");
    let result = investments
        .calculate(
            id,
            month("2026-08"),
            "",
            CalculationInput::Balance,
            "10700.00",
            Some(month("2026-08")),
        )
        .unwrap();
    assert_eq!(result.capital_growth.to_string(), "700.00");
}
