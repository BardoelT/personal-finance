use super::debt::COLOUR_SLOTS;
use super::*;
use crate::domain::income::PayPeriod;

fn period(text: &str) -> Period {
    parse_period(text).unwrap()
}

fn today() -> YearMonth {
    YearMonth::parse("2026-10").unwrap()
}

fn balance(text: &str) -> DebtBalance {
    DebtBalance::parse(text).unwrap()
}

fn input(name: &str, period: &str) -> DebtInput {
    DebtInput {
        name: name.to_owned(),
        statement_period: Some(period.to_owned()),
    }
}

fn debt(debts: &mut Debts, name: &str, period: &str) -> DebtId {
    debts.create_debt(&input(name, period)).unwrap().id
}

fn add(debts: &mut Debts, id: DebtId, at: &str, amount: &str) {
    debts
        .add_statement(id, period(at), balance(amount), Note::default(), today())
        .unwrap();
}

fn add_error(debts: &mut Debts, id: DebtId, at: &str, amount: &str) -> DomainError {
    match parse_period(at)
        .and_then(|p| DebtBalance::parse(amount).map(|b| (p, b)))
        .and_then(|(p, b)| debts.add_statement(id, p, b, Note::default(), today()))
    {
        Ok(_) => panic!("expected an error"),
        Err(error) => error,
    }
}

fn code<T>(result: Result<T, DomainError>) -> &'static str {
    result.err().map_or("OK", |error| error.code())
}

fn update(
    debts: &mut Debts,
    id: DebtId,
    from: &str,
    to: &str,
    amount: &str,
) -> Result<DebtStatement, DomainError> {
    debts.update_statement(
        id,
        period(from),
        period(to),
        balance(amount),
        Note::default(),
        today(),
    )
}

/// The change of the statement of `at`, as text, or "none".
fn change(debts: &Debts, id: DebtId, at: &str) -> String {
    let (_, statement) = debts
        .all_statements()
        .into_iter()
        .find(|(d, s)| d.id == id && s.period == period(at))
        .unwrap();
    debts
        .change(statement)
        .map_or_else(|| "none".to_owned(), |value| value.to_string())
}

// REQ-1, REQ-2

#[test]
fn test_csd011_ac_1_1_a_debt_is_monthly_by_default() {
    let mut debts = Debts::default();
    let created = debts
        .create_debt(&DebtInput {
            name: "Example Mortgage".to_owned(),
            statement_period: None,
        })
        .unwrap();
    assert_eq!(created.statement_period, PayPeriod::Monthly);
    assert_eq!(debts.statement_count(created.id), 0);
}

#[test]
fn test_csd011_ac_1_2_a_yearly_debt() {
    let mut debts = Debts::default();
    debt(&mut debts, "Example Student Loan", "Yearly");
    assert_eq!(debts.debts()[0].statement_period, PayPeriod::Yearly);
}

#[test]
fn test_csd011_ac_1_3_an_unknown_period_is_rejected() {
    let mut debts = Debts::default();
    let result = debts.create_debt(&input("Example Mortgage", "Weekly"));
    assert_eq!(code(result), "DEBT_PERIOD_UNSUPPORTED");
    assert_eq!(debts.debts().len(), 0);
}

#[test]
fn test_csd011_ac_2_1_invalid_names_are_rejected() {
    let mut debts = Debts::default();
    debt(&mut debts, "Example Mortgage", "Monthly");
    for (name, expected) in [
        ("", "DEBT_NAME_REQUIRED"),
        ("   ", "DEBT_NAME_REQUIRED"),
        ("EXAMPLE MORTGAGE", "DEBT_NAME_DUPLICATE"),
        ("example mortgage", "DEBT_NAME_DUPLICATE"),
    ] {
        assert_eq!(
            code(debts.create_debt(&input(name, "Monthly"))),
            expected,
            "{name:?}"
        );
    }
    assert_eq!(debts.debts().len(), 1);
}

#[test]
fn test_csd011_ac_2_2_name_length_boundary() {
    let mut debts = Debts::default();
    assert!(debts
        .create_debt(&input(&"a".repeat(100), "Monthly"))
        .is_ok());
    let result = debts.create_debt(&input(&"b".repeat(101), "Monthly"));
    assert_eq!(code(result), "DEBT_NAME_TOO_LONG");
}

#[test]
fn test_csd011_ac_2_3_the_name_is_stored_trimmed() {
    let mut debts = Debts::default();
    let created = debts
        .create_debt(&input("  Example Mortgage  ", "Monthly"))
        .unwrap();
    assert_eq!(created.name, "Example Mortgage");
}

#[test]
fn test_csd011_ac_2_4_renaming_to_an_existing_name_is_rejected() {
    let mut debts = Debts::default();
    debt(&mut debts, "Example Mortgage", "Monthly");
    let loan = debt(&mut debts, "Example Student Loan", "Monthly");
    let result = debts.update_debt(loan, &input("example mortgage", "Monthly"));
    assert_eq!(code(result), "DEBT_NAME_DUPLICATE");
}

#[test]
fn test_csd011_ac_2_5_changing_only_the_case_of_its_own_name_is_allowed() {
    let mut debts = Debts::default();
    let loan = debt(&mut debts, "Example Student Loan", "Monthly");
    let renamed = debts
        .update_debt(loan, &input("EXAMPLE STUDENT LOAN", "Monthly"))
        .unwrap();
    assert_eq!(renamed.name, "EXAMPLE STUDENT LOAN");
}

// REQ-3

#[test]
fn test_csd011_ac_3_1_slots_follow_creation_order() {
    let mut debts = Debts::default();
    let slots: Vec<u8> = ["A", "B", "C"]
        .iter()
        .map(|name| {
            debts
                .create_debt(&input(name, "Monthly"))
                .unwrap()
                .colour_slot
        })
        .collect();
    assert_eq!(slots, [0, 1, 2]);
}

#[test]
fn test_csd011_ac_3_2_slots_cycle_after_eight_debts() {
    let mut debts = Debts::default();
    for index in 0..COLOUR_SLOTS {
        debt(&mut debts, &format!("Debt {index}"), "Monthly");
    }
    assert_eq!(
        debts
            .create_debt(&input("Debt 9", "Monthly"))
            .unwrap()
            .colour_slot,
        0
    );
}

#[test]
fn test_csd011_ac_3_3_deleting_a_debt_does_not_recolour_the_others() {
    let mut debts = Debts::default();
    let first = debt(&mut debts, "A", "Monthly");
    debt(&mut debts, "B", "Monthly");
    debt(&mut debts, "C", "Monthly");
    debts.delete_debt(first).unwrap();
    let slots: Vec<u8> = debts.debts().iter().map(|d| d.colour_slot).collect();
    assert_eq!(slots, [1, 2]);
}

#[test]
fn test_csd011_ac_3_4_a_freed_slot_is_used_first() {
    let mut debts = Debts::default();
    debt(&mut debts, "A", "Monthly");
    let second = debt(&mut debts, "B", "Monthly");
    debt(&mut debts, "C", "Monthly");
    debts.delete_debt(second).unwrap();
    assert_eq!(
        debts
            .create_debt(&input("D", "Monthly"))
            .unwrap()
            .colour_slot,
        1
    );
}

#[test]
fn test_csd011_ac_3_5_renaming_keeps_the_slot() {
    let mut debts = Debts::default();
    debt(&mut debts, "A", "Monthly");
    debt(&mut debts, "B", "Monthly");
    let third = debt(&mut debts, "C", "Monthly");
    let renamed = debts
        .update_debt(third, &input("Renamed", "Monthly"))
        .unwrap();
    assert_eq!(renamed.colour_slot, 2);
}

// REQ-4 to REQ-6

#[test]
fn test_csd011_ac_4_1_rename_keeps_statements() {
    let mut debts = Debts::default();
    let id = debt(&mut debts, "Example Mortgage", "Monthly");
    add(&mut debts, id, "2026-08", "100.00");
    add(&mut debts, id, "2026-09", "90.00");
    debts
        .update_debt(id, &input("Example Home Loan", "Monthly"))
        .unwrap();
    assert_eq!(debts.statement_count(id), 2);
}

#[test]
fn test_csd011_ac_4_2_the_period_of_a_debt_without_statements_can_change() {
    let mut debts = Debts::default();
    let id = debt(&mut debts, "Example Mortgage", "Monthly");
    let changed = debts
        .update_debt(id, &input("Example Mortgage", "Yearly"))
        .unwrap();
    assert_eq!(changed.statement_period, PayPeriod::Yearly);
}

#[test]
fn test_csd011_ac_4_3_an_invalid_edit_changes_nothing() {
    let mut debts = Debts::default();
    debt(&mut debts, "Example Mortgage", "Monthly");
    let loan = debt(&mut debts, "Example Student Loan", "Monthly");
    assert_eq!(
        code(debts.update_debt(loan, &input("  ", "Monthly"))),
        "DEBT_NAME_REQUIRED"
    );
    assert!(debts
        .debts()
        .iter()
        .any(|d| d.name == "Example Student Loan"));
}

#[test]
fn test_csd011_ac_5_1_the_period_is_locked_once_statements_exist() {
    let mut debts = Debts::default();
    let id = debt(&mut debts, "Example Mortgage", "Monthly");
    add(&mut debts, id, "2026-09", "100.00");
    let result = debts.update_debt(id, &input("Example Mortgage", "Yearly"));
    assert_eq!(code(result), "DEBT_PERIOD_LOCKED");
    assert_eq!(debts.debts()[0].statement_period, PayPeriod::Monthly);
}

#[test]
fn test_csd011_ac_5_2_the_period_can_change_after_deleting_the_last_statement() {
    let mut debts = Debts::default();
    let id = debt(&mut debts, "Example Mortgage", "Monthly");
    add(&mut debts, id, "2026-09", "100.00");
    debts.delete_statement(id, period("2026-09")).unwrap();
    assert!(debts
        .update_debt(id, &input("Example Mortgage", "Yearly"))
        .is_ok());
}

#[test]
fn test_csd011_ac_6_1_deleting_a_debt_removes_its_statements() {
    let mut debts = Debts::default();
    let id = debt(&mut debts, "Example Mortgage", "Monthly");
    let other = debt(&mut debts, "Example Student Loan", "Monthly");
    for month in ["2026-07", "2026-08", "2026-09"] {
        add(&mut debts, id, month, "100.00");
    }
    add(&mut debts, other, "2026-09", "50.00");
    debts.delete_debt(id).unwrap();
    assert_eq!(debts.debts().len(), 1);
    assert_eq!(debts.total_statement_count(), 1);
    assert_eq!(code(debts.delete_debt(id)), "DEBT_NOT_FOUND");
}

#[test]
fn test_csd011_ac_6_3_a_debt_at_zero_is_deleted_like_any_other() {
    let mut debts = Debts::default();
    let id = debt(&mut debts, "Example Student Loan", "Monthly");
    add(&mut debts, id, "2026-09", "0.00");
    debts.delete_debt(id).unwrap();
    assert_eq!(debts.total_statement_count(), 0);
}

// REQ-7 to REQ-9

#[test]
fn test_csd011_ac_7_1_a_monthly_statement_with_a_note() {
    let mut debts = Debts::default();
    let id = debt(&mut debts, "Example Mortgage", "Monthly");
    let statement = debts
        .add_statement(
            id,
            period("2026-09"),
            balance("150000.00"),
            debt_note("Paid extra").unwrap(),
            today(),
        )
        .unwrap();
    assert_eq!(statement.balance.value().to_string(), "150000.00");
    assert_eq!(statement.note.as_str(), "Paid extra");
}

#[test]
fn test_csd011_ac_7_2_a_yearly_statement_without_a_note() {
    let mut debts = Debts::default();
    let id = debt(&mut debts, "Example Student Loan", "Yearly");
    add(&mut debts, id, "2025", "8000.00");
    assert_eq!(debts.all_statements()[0].1.note.as_str(), "");
}

#[test]
fn test_csd011_ac_7_3_old_periods_are_accepted() {
    let mut debts = Debts::default();
    let id = debt(&mut debts, "Example Mortgage", "Monthly");
    add(&mut debts, id, "2019-01", "200000.00");
    assert_eq!(debts.statement_count(id), 1);
}

#[test]
fn test_csd011_ac_7_4_the_current_period_is_accepted_and_a_later_one_is_not() {
    let mut debts = Debts::default();
    let monthly = debt(&mut debts, "Monthly", "Monthly");
    let yearly = debt(&mut debts, "Yearly", "Yearly");
    add(&mut debts, monthly, "2026-10", "1000.00");
    add(&mut debts, yearly, "2026", "1000.00");
    assert_eq!(
        add_error(&mut debts, monthly, "2026-11", "1000.00").code(),
        "DEBT_STATEMENT_IN_FUTURE"
    );
    assert_eq!(
        add_error(&mut debts, yearly, "2027", "1000.00").code(),
        "DEBT_STATEMENT_IN_FUTURE"
    );
}

#[test]
fn test_csd011_ac_7_5_an_invalid_period_is_rejected() {
    let mut debts = Debts::default();
    let monthly = debt(&mut debts, "Monthly", "Monthly");
    let yearly = debt(&mut debts, "Yearly", "Yearly");
    for (id, text) in [
        (monthly, "2026-13"),
        (monthly, "2026-9"),
        (monthly, "abcd"),
        (yearly, "26"),
        (yearly, "abcd"),
        (yearly, "1899"),
    ] {
        assert_eq!(
            add_error(&mut debts, id, text, "1000.00").code(),
            "DEBT_STATEMENT_PERIOD_INVALID",
            "{text}"
        );
    }
}

#[test]
fn test_csd011_ac_8_1_a_duplicate_period_is_rejected() {
    let mut debts = Debts::default();
    let id = debt(&mut debts, "Example Mortgage", "Monthly");
    debts
        .add_statement(
            id,
            period("2026-09"),
            balance("150000.00"),
            debt_note("First").unwrap(),
            today(),
        )
        .unwrap();
    assert_eq!(
        add_error(&mut debts, id, "2026-09", "149000.00").code(),
        "DEBT_STATEMENT_DUPLICATE"
    );
    let rows = debts.all_statements();
    assert_eq!(rows[0].1.balance.value().to_string(), "150000.00");
    assert_eq!(rows[0].1.note.as_str(), "First");
}

#[test]
fn test_csd011_ac_8_2_the_same_period_on_another_debt_is_allowed() {
    let mut debts = Debts::default();
    let first = debt(&mut debts, "A", "Monthly");
    let second = debt(&mut debts, "B", "Monthly");
    add(&mut debts, first, "2026-09", "100.00");
    add(&mut debts, second, "2026-09", "200.00");
    assert_eq!(debts.total_statement_count(), 2);
}

#[test]
fn test_csd011_ac_9_a_statement_must_match_the_statement_period() {
    let mut debts = Debts::default();
    let yearly = debt(&mut debts, "Yearly", "Yearly");
    let monthly = debt(&mut debts, "Monthly", "Monthly");
    assert_eq!(
        add_error(&mut debts, yearly, "2026-09", "1000.00").code(),
        "DEBT_PERIOD_MISMATCH"
    );
    assert_eq!(
        add_error(&mut debts, monthly, "2025", "1000.00").code(),
        "DEBT_PERIOD_MISMATCH"
    );
}

// REQ-10, REQ-11

#[test]
fn test_csd011_ac_10_1_invalid_balances_are_rejected() {
    for (text, expected) in [
        ("", "DEBT_STATEMENT_BALANCE_INVALID"),
        ("abc", "DEBT_STATEMENT_BALANCE_INVALID"),
        ("1e3", "DEBT_STATEMENT_BALANCE_INVALID"),
        ("2500,50", "DEBT_STATEMENT_BALANCE_INVALID"),
        ("-0.01", "DEBT_STATEMENT_BALANCE_NEGATIVE"),
        ("1000.999", "DEBT_STATEMENT_BALANCE_PRECISION"),
        ("1000000000.01", "DEBT_STATEMENT_BALANCE_TOO_LARGE"),
    ] {
        assert_eq!(code(DebtBalance::parse(text)), expected, "{text:?}");
    }
}

#[test]
fn test_csd011_ac_10_2_boundary_balances_are_accepted() {
    for text in ["0.00", "1000000000.00"] {
        assert_eq!(balance(text).value().to_string(), text);
    }
}

#[test]
fn test_csd011_ac_10_3_an_invalid_balance_in_an_edit_changes_nothing() {
    let mut debts = Debts::default();
    let id = debt(&mut debts, "Example Mortgage", "Monthly");
    add(&mut debts, id, "2026-09", "150000.00");
    assert_eq!(
        code(DebtBalance::parse("-1.00")),
        "DEBT_STATEMENT_BALANCE_NEGATIVE"
    );
    assert_eq!(
        debts.all_statements()[0].1.balance.value().to_string(),
        "150000.00"
    );
}

#[test]
fn test_csd011_ac_11_note_length_and_line_breaks() {
    assert!(debt_note(&"a".repeat(2000)).is_ok());
    assert_eq!(
        code(debt_note(&"a".repeat(2001))),
        "DEBT_STATEMENT_NOTE_TOO_LONG"
    );
    assert_eq!(
        debt_note("line one\nline two").unwrap().as_str(),
        "line one\nline two"
    );
}

// REQ-12, REQ-13

#[test]
fn test_csd011_ac_12_1_edit_balance_and_note_and_clear_the_note() {
    let mut debts = Debts::default();
    let id = debt(&mut debts, "Example Mortgage", "Monthly");
    add(&mut debts, id, "2026-09", "150000.00");
    let edited = debts
        .update_statement(
            id,
            period("2026-09"),
            period("2026-09"),
            balance("149000.00"),
            debt_note("Corrected").unwrap(),
            today(),
        )
        .unwrap();
    assert_eq!(edited.balance.value().to_string(), "149000.00");
    assert_eq!(edited.note.as_str(), "Corrected");
    update(&mut debts, id, "2026-09", "2026-09", "149000.00").unwrap();
    assert_eq!(debts.all_statements()[0].1.note.as_str(), "");
}

#[test]
fn test_csd011_ac_12_3_a_statement_moves_to_another_month() {
    let mut debts = Debts::default();
    let id = debt(&mut debts, "Example Mortgage", "Monthly");
    debts
        .add_statement(
            id,
            period("2026-09"),
            balance("150000.00"),
            debt_note("First").unwrap(),
            today(),
        )
        .unwrap();
    debts
        .update_statement(
            id,
            period("2026-09"),
            period("2026-08"),
            balance("150000.00"),
            debt_note("First").unwrap(),
            today(),
        )
        .unwrap();
    let rows = debts.all_statements();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].1.period.to_string(), "2026-08");
    assert_eq!(rows[0].1.note.as_str(), "First");
}

#[test]
fn test_csd011_ac_12_4_the_period_and_the_balance_change_together() {
    let mut debts = Debts::default();
    let id = debt(&mut debts, "Example Mortgage", "Monthly");
    add(&mut debts, id, "2026-09", "150000.00");
    update(&mut debts, id, "2026-09", "2026-08", "149000.00").unwrap();
    let rows = debts.all_statements();
    assert_eq!(rows[0].1.period.to_string(), "2026-08");
    assert_eq!(rows[0].1.balance.value().to_string(), "149000.00");
}

#[test]
fn test_csd011_ac_12_5_a_yearly_statement_moves() {
    let mut debts = Debts::default();
    let id = debt(&mut debts, "Example Student Loan", "Yearly");
    add(&mut debts, id, "2025", "8000.00");
    update(&mut debts, id, "2025", "2024", "8000.00").unwrap();
    assert_eq!(debts.all_statements()[0].1.period.to_string(), "2024");
}

#[test]
fn test_csd011_ac_12_6_the_yearly_totals_follow_the_move() {
    let mut debts = Debts::default();
    let id = debt(&mut debts, "Example Mortgage", "Monthly");
    add(&mut debts, id, "2025-11", "10000.00");
    add(&mut debts, id, "2025-12", "9800.00");
    add(&mut debts, id, "2026-05", "9000.00");
    update(&mut debts, id, "2025-12", "2026-01", "9800.00").unwrap();
    let totals = debts.yearly_totals();
    assert_eq!(totals[0].closing_balance.to_string(), "9000.00");
    assert_eq!(totals[0].statement_count, 2);
    assert_eq!(totals[1].closing_balance.to_string(), "10000.00");
    assert_eq!(totals[1].statement_count, 1);
}

#[test]
fn test_csd011_ac_12_8_keeping_the_period_is_not_a_duplicate() {
    let mut debts = Debts::default();
    let id = debt(&mut debts, "Example Mortgage", "Monthly");
    add(&mut debts, id, "2026-09", "150000.00");
    assert!(update(&mut debts, id, "2026-09", "2026-09", "149000.00").is_ok());
}

#[test]
fn test_csd011_ac_13_1_a_new_period_in_the_future_is_rejected() {
    let mut debts = Debts::default();
    let monthly = debt(&mut debts, "Monthly", "Monthly");
    let yearly = debt(&mut debts, "Yearly", "Yearly");
    add(&mut debts, monthly, "2026-09", "1000.00");
    add(&mut debts, yearly, "2025", "1000.00");
    assert_eq!(
        code(update(&mut debts, monthly, "2026-09", "2026-11", "1.00")),
        "DEBT_STATEMENT_IN_FUTURE"
    );
    assert_eq!(
        code(update(&mut debts, yearly, "2025", "2027", "1.00")),
        "DEBT_STATEMENT_IN_FUTURE"
    );
}

#[test]
fn test_csd011_ac_13_2_a_taken_period_is_rejected_and_nothing_changes() {
    let mut debts = Debts::default();
    let id = debt(&mut debts, "Example Mortgage", "Monthly");
    add(&mut debts, id, "2026-08", "151000.00");
    add(&mut debts, id, "2026-09", "150000.00");
    assert_eq!(
        code(update(&mut debts, id, "2026-09", "2026-08", "149000.00")),
        "DEBT_STATEMENT_DUPLICATE"
    );
    let rows: Vec<String> = debts
        .all_statements()
        .iter()
        .map(|(_, s)| format!("{} {}", s.period, s.balance.value()))
        .collect();
    assert_eq!(rows, ["2026-09 150000.00", "2026-08 151000.00"]);
}

#[test]
fn test_csd011_ac_13_3_a_period_of_the_wrong_kind_is_rejected() {
    let mut debts = Debts::default();
    let monthly = debt(&mut debts, "Monthly", "Monthly");
    let yearly = debt(&mut debts, "Yearly", "Yearly");
    add(&mut debts, monthly, "2026-09", "1.00");
    add(&mut debts, yearly, "2025", "1.00");
    assert_eq!(
        code(update(&mut debts, monthly, "2026-09", "2026", "1.00")),
        "DEBT_PERIOD_MISMATCH"
    );
    assert_eq!(
        code(update(&mut debts, yearly, "2025", "2024-12", "1.00")),
        "DEBT_PERIOD_MISMATCH"
    );
}

#[test]
fn test_csd011_ac_13_5_another_debt_may_have_the_same_period() {
    let mut debts = Debts::default();
    let first = debt(&mut debts, "A", "Monthly");
    let second = debt(&mut debts, "B", "Monthly");
    add(&mut debts, first, "2026-08", "1.00");
    add(&mut debts, second, "2026-09", "2.00");
    update(&mut debts, second, "2026-09", "2026-08", "2.00").unwrap();
    assert!(debts
        .all_statements()
        .iter()
        .all(|(_, s)| s.period.to_string() == "2026-08"));
}

#[test]
fn test_csd011_ac_13_6_an_invalid_period_is_rejected() {
    assert_eq!(
        code(parse_period("2026-13")),
        "DEBT_STATEMENT_PERIOD_INVALID"
    );
}

// REQ-14, REQ-15

#[test]
fn test_csd011_ac_14_1_deleting_a_statement_removes_only_that_statement() {
    let mut debts = Debts::default();
    let id = debt(&mut debts, "Example Mortgage", "Monthly");
    add(&mut debts, id, "2026-08", "1.00");
    add(&mut debts, id, "2026-09", "2.00");
    debts.delete_statement(id, period("2026-09")).unwrap();
    let rows = debts.all_statements();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].1.period.to_string(), "2026-08");
    assert_eq!(
        code(debts.delete_statement(id, period("2026-09"))),
        "DEBT_STATEMENT_NOT_FOUND"
    );
}

#[test]
fn test_csd011_ac_15_copy_previous() {
    let mut debts = Debts::default();
    let id = debt(&mut debts, "Example Mortgage", "Monthly");
    // AC-15.2: nothing to copy yet.
    assert_eq!(debts.prefill_from_latest(id).unwrap().balance, None);
    add(&mut debts, id, "2026-07", "151000.00");
    debts
        .add_statement(
            id,
            period("2026-08"),
            balance("150000.00"),
            debt_note("Paid extra").unwrap(),
            today(),
        )
        .unwrap();
    // AC-15.1: the latest balance, and nothing is stored.
    assert_eq!(
        debts
            .prefill_from_latest(id)
            .unwrap()
            .balance
            .unwrap()
            .to_string(),
        "150000.00"
    );
    assert_eq!(debts.statement_count(id), 2);
    // AC-15.3: 0.00 is copied as it is.
    add(&mut debts, id, "2026-09", "0.00");
    assert_eq!(
        debts
            .prefill_from_latest(id)
            .unwrap()
            .balance
            .unwrap()
            .to_string(),
        "0.00"
    );
}

// REQ-17, REQ-18

#[test]
fn test_csd011_ac_17_1_the_latest_balance_per_debt() {
    let mut debts = Debts::default();
    let id = debt(&mut debts, "Example Mortgage", "Monthly");
    add(&mut debts, id, "2026-07", "151000.00");
    add(&mut debts, id, "2026-09", "150000.00");
    let overview = debts.overview();
    assert_eq!(
        overview.debts[0].latest_balance.unwrap().to_string(),
        "150000.00"
    );
    assert_eq!(
        overview.debts[0].latest_period.unwrap().to_string(),
        "2026-09"
    );
}

#[test]
fn test_csd011_ac_17_2_the_total_spans_debts_with_different_periods() {
    let mut debts = Debts::default();
    let monthly = debt(&mut debts, "Example Mortgage", "Monthly");
    let yearly = debt(&mut debts, "Example Student Loan", "Yearly");
    add(&mut debts, monthly, "2026-09", "150000.50");
    add(&mut debts, yearly, "2025", "8000.00");
    let overview = debts.overview();
    assert_eq!(overview.total.unwrap().to_string(), "158000.50");
    let periods: Vec<String> = overview
        .debts
        .iter()
        .map(|d| d.latest_period.unwrap().to_string())
        .collect();
    assert_eq!(periods, ["2026-09", "2025"]);
}

#[test]
fn test_csd011_ac_17_3_debts_without_statements_do_not_count() {
    let mut debts = Debts::default();
    let id = debt(&mut debts, "Example Mortgage", "Monthly");
    debt(&mut debts, "Example Empty", "Monthly");
    add(&mut debts, id, "2026-09", "150000.00");
    assert_eq!(debts.overview().total.unwrap().to_string(), "150000.00");
    assert_eq!(Debts::default().overview().total, None);
}

#[test]
fn test_csd011_ac_17_4_a_repaid_debt_counts_as_zero() {
    let mut debts = Debts::default();
    let mortgage = debt(&mut debts, "Example Mortgage", "Monthly");
    let car = debt(&mut debts, "Example Car Loan", "Monthly");
    add(&mut debts, mortgage, "2026-09", "150000.00");
    add(&mut debts, car, "2026-09", "0.00");
    let overview = debts.overview();
    assert_eq!(overview.total.unwrap().to_string(), "150000.00");
    assert_eq!(
        overview.debts[0].latest_balance.unwrap().to_string(),
        "0.00"
    );
}

#[test]
fn test_csd011_ac_17_5_a_new_latest_statement_updates_the_total_and_an_older_one_does_not() {
    let mut debts = Debts::default();
    let id = debt(&mut debts, "Example Mortgage", "Monthly");
    add(&mut debts, id, "2026-09", "150000.00");
    add(&mut debts, id, "2026-01", "160000.00");
    assert_eq!(debts.overview().total.unwrap().to_string(), "150000.00");
    add(&mut debts, id, "2026-10", "149000.00");
    let overview = debts.overview();
    assert_eq!(overview.total.unwrap().to_string(), "149000.00");
    assert_eq!(
        overview.debts[0].latest_period.unwrap().to_string(),
        "2026-10"
    );
}

#[test]
fn test_csd011_ac_18_1_debts_are_alphabetical_ignoring_case() {
    let mut debts = Debts::default();
    for name in [
        "example student loan",
        "Example Mortgage",
        "Example Car Loan",
    ] {
        debt(&mut debts, name, "Monthly");
    }
    let names: Vec<&str> = debts.debts().iter().map(|d| d.name.as_str()).collect();
    assert_eq!(
        names,
        [
            "Example Car Loan",
            "Example Mortgage",
            "example student loan"
        ]
    );
}

#[test]
fn test_csd011_ac_18_2_statements_are_newest_first_and_a_year_counts_as_december() {
    let mut debts = Debts::default();
    let monthly = debt(&mut debts, "Monthly", "Monthly");
    let yearly = debt(&mut debts, "Yearly", "Yearly");
    add(&mut debts, monthly, "2025-11", "1.00");
    add(&mut debts, monthly, "2025-12", "1.00");
    add(&mut debts, yearly, "2025", "1.00");
    add(&mut debts, monthly, "2026-07", "1.00");
    let order: Vec<String> = debts
        .all_statements()
        .iter()
        .map(|(d, s)| format!("{} {}", d.name, s.period))
        .collect();
    assert_eq!(
        order,
        [
            "Monthly 2026-07",
            "Monthly 2025-12",
            "Yearly 2025",
            "Monthly 2025-11"
        ]
    );
}

#[test]
fn test_csd011_ac_18_4_the_same_period_is_ordered_by_debt() {
    let mut debts = Debts::default();
    let mortgage = debt(&mut debts, "Example Mortgage", "Monthly");
    let car = debt(&mut debts, "Example Car Loan", "Monthly");
    add(&mut debts, mortgage, "2026-09", "1.00");
    add(&mut debts, car, "2026-09", "1.00");
    assert_eq!(debts.all_statements()[0].0.name, "Example Car Loan");
}

// REQ-25

#[test]
fn test_csd011_ac_25_1_the_change_follows_the_previous_statement() {
    let mut debts = Debts::default();
    let id = debt(&mut debts, "Example Mortgage", "Monthly");
    add(&mut debts, id, "2026-07", "10000.00");
    add(&mut debts, id, "2026-08", "9500.00");
    add(&mut debts, id, "2026-09", "9600.00");
    assert_eq!(change(&debts, id, "2026-07"), "none");
    assert_eq!(change(&debts, id, "2026-08"), "500.00");
    assert_eq!(change(&debts, id, "2026-09"), "-100.00");
}

#[test]
fn test_csd011_ac_25_2_a_gap_does_not_matter() {
    let mut debts = Debts::default();
    let id = debt(&mut debts, "Example Mortgage", "Monthly");
    add(&mut debts, id, "2026-03", "10000.00");
    add(&mut debts, id, "2026-09", "8000.00");
    assert_eq!(change(&debts, id, "2026-09"), "2000.00");
}

#[test]
fn test_csd011_ac_25_3_yearly_statements() {
    let mut debts = Debts::default();
    let id = debt(&mut debts, "Example Student Loan", "Yearly");
    add(&mut debts, id, "2024", "8000.00");
    add(&mut debts, id, "2025", "6000.00");
    assert_eq!(change(&debts, id, "2025"), "2000.00");
    assert_eq!(change(&debts, id, "2024"), "none");
}

#[test]
fn test_csd011_ac_25_4_another_debt_does_not_count() {
    let mut debts = Debts::default();
    let mortgage = debt(&mut debts, "Example Mortgage", "Monthly");
    let car = debt(&mut debts, "Example Car Loan", "Monthly");
    add(&mut debts, mortgage, "2026-09", "150000.00");
    add(&mut debts, car, "2026-08", "5000.00");
    add(&mut debts, car, "2026-09", "4500.00");
    assert_eq!(change(&debts, mortgage, "2026-09"), "none");
    assert_eq!(change(&debts, car, "2026-09"), "500.00");
}

#[test]
fn test_csd011_ac_25_5_changes_follow_a_move() {
    let mut debts = Debts::default();
    let id = debt(&mut debts, "Example Mortgage", "Monthly");
    add(&mut debts, id, "2026-07", "10000.00");
    add(&mut debts, id, "2026-08", "9500.00");
    add(&mut debts, id, "2026-09", "9000.00");
    update(&mut debts, id, "2026-08", "2026-10", "9500.00").unwrap();
    assert_eq!(change(&debts, id, "2026-09"), "1000.00");
    assert_eq!(change(&debts, id, "2026-10"), "-500.00");
}

#[test]
fn test_csd011_ac_25_6_editing_an_earlier_statement_changes_the_later_change() {
    let mut debts = Debts::default();
    let id = debt(&mut debts, "Example Mortgage", "Monthly");
    add(&mut debts, id, "2026-08", "9500.00");
    add(&mut debts, id, "2026-09", "9000.00");
    update(&mut debts, id, "2026-08", "2026-08", "9800.00").unwrap();
    assert_eq!(change(&debts, id, "2026-09"), "800.00");
}

// REQ-19

/// The closing balance, the repaid amount ("none" for none) and the statement
/// count of `year`.
fn year(debts: &Debts, year: i32) -> (String, String, usize) {
    let total = debts
        .yearly_totals()
        .into_iter()
        .find(|total| total.year == year)
        .unwrap();
    (
        total.closing_balance.to_string(),
        total
            .repaid
            .map_or_else(|| "none".to_owned(), |value| value.to_string()),
        total.statement_count,
    )
}

fn row(closing: &str, repaid: &str, count: usize) -> (String, String, usize) {
    (closing.to_owned(), repaid.to_owned(), count)
}

#[test]
fn test_csd011_ac_19_1_totals_per_calendar_year() {
    let mut debts = Debts::default();
    let id = debt(&mut debts, "Example Mortgage", "Monthly");
    add(&mut debts, id, "2025-11", "10000.00");
    add(&mut debts, id, "2025-12", "9800.00");
    add(&mut debts, id, "2026-05", "9000.00");
    assert_eq!(year(&debts, 2026), row("9000.00", "800.00", 1));
    assert_eq!(year(&debts, 2025), row("9800.00", "none", 2));
}

#[test]
fn test_csd011_ac_19_2_no_statements_no_totals() {
    let mut debts = Debts::default();
    debt(&mut debts, "Example Mortgage", "Monthly");
    assert_eq!(debts.yearly_totals().len(), 0);
}

#[test]
fn test_csd011_ac_19_3_totals_add_up_all_debts() {
    let mut debts = Debts::default();
    let mortgage = debt(&mut debts, "Example Mortgage", "Monthly");
    let loan = debt(&mut debts, "Example Student Loan", "Monthly");
    add(&mut debts, mortgage, "2026-08", "150000.00");
    add(&mut debts, mortgage, "2026-09", "149500.00");
    add(&mut debts, loan, "2026-09", "5000.00");
    assert_eq!(debts.yearly_totals().len(), 1);
    assert_eq!(year(&debts, 2026), row("154500.00", "none", 3));
}

#[test]
fn test_csd011_ac_19_4_the_closing_balance_is_the_latest_balance_not_a_sum() {
    let mut debts = Debts::default();
    let id = debt(&mut debts, "Example Mortgage", "Monthly");
    add(&mut debts, id, "2026-03", "10000.00");
    add(&mut debts, id, "2026-09", "8000.00");
    assert_eq!(year(&debts, 2026), row("8000.00", "none", 2));
}

#[test]
fn test_csd011_ac_19_5_monthly_and_yearly_debts_add_up_together() {
    let mut debts = Debts::default();
    let mortgage = debt(&mut debts, "Example Mortgage", "Monthly");
    let loan = debt(&mut debts, "Example Student Loan", "Yearly");
    add(&mut debts, mortgage, "2025-12", "10000.00");
    add(&mut debts, loan, "2024", "7000.00");
    add(&mut debts, loan, "2025", "6000.00");
    assert_eq!(year(&debts, 2025), row("16000.00", "1000.00", 2));
    assert_eq!(year(&debts, 2024), row("7000.00", "none", 1));
}

#[test]
fn test_csd011_ac_19_6_a_debt_without_a_statement_in_a_year_does_not_count_in_it() {
    let mut debts = Debts::default();
    let mortgage = debt(&mut debts, "Example Mortgage", "Monthly");
    let loan = debt(&mut debts, "Example Student Loan", "Yearly");
    add(&mut debts, mortgage, "2025-12", "10000.00");
    add(&mut debts, mortgage, "2026-03", "9500.00");
    add(&mut debts, loan, "2025", "6000.00");
    assert_eq!(year(&debts, 2026), row("9500.00", "500.00", 1));
    assert_eq!(year(&debts, 2025), row("16000.00", "none", 2));
}

#[test]
fn test_csd011_ac_19_7_a_debt_that_grew_shows_a_negative_amount_repaid() {
    let mut debts = Debts::default();
    let id = debt(&mut debts, "Example Mortgage", "Monthly");
    add(&mut debts, id, "2025-12", "5000.00");
    add(&mut debts, id, "2026-03", "5200.00");
    assert_eq!(year(&debts, 2026).1, "-200.00");
}

#[test]
fn test_csd011_ac_19_8_a_debt_without_an_opening_balance_adds_no_repaid_amount() {
    let mut debts = Debts::default();
    let mortgage = debt(&mut debts, "Example Mortgage", "Monthly");
    let car = debt(&mut debts, "Example Car Loan", "Monthly");
    add(&mut debts, mortgage, "2025-12", "10000.00");
    add(&mut debts, mortgage, "2026-03", "9000.00");
    add(&mut debts, car, "2026-03", "4000.00");
    assert_eq!(year(&debts, 2026), row("13000.00", "1000.00", 2));
}

#[test]
fn test_csd011_ac_19_9_repaid_after_a_gap_counts_in_the_later_year() {
    let mut debts = Debts::default();
    let id = debt(&mut debts, "Example Mortgage", "Monthly");
    add(&mut debts, id, "2024-12", "10000.00");
    add(&mut debts, id, "2026-06", "8000.00");
    assert_eq!(year(&debts, 2026).1, "2000.00");
    assert!(debts.yearly_totals().iter().all(|t| t.year != 2025));
}

#[test]
fn test_csd011_ac_19_10_a_new_statement_updates_the_totals() {
    let mut debts = Debts::default();
    let id = debt(&mut debts, "Example Mortgage", "Monthly");
    add(&mut debts, id, "2025-12", "10000.00");
    add(&mut debts, id, "2026-09", "9000.00");
    add(&mut debts, id, "2026-10", "8500.00");
    assert_eq!(year(&debts, 2026), row("8500.00", "1500.00", 2));
}

#[test]
fn test_csd011_ac_19_11_newest_year_first() {
    let mut debts = Debts::default();
    let id = debt(&mut debts, "Example Student Loan", "Yearly");
    for text in ["2024", "2025", "2026"] {
        add(&mut debts, id, text, "1.00");
    }
    let years: Vec<i32> = debts.yearly_totals().iter().map(|t| t.year).collect();
    assert_eq!(years, [2026, 2025, 2024]);
}

// The data of a file

#[test]
fn test_csd011_a_damaged_section_is_not_consistent() {
    let mut debts = Debts::default();
    let id = debt(&mut debts, "Example Mortgage", "Monthly");
    add(&mut debts, id, "2026-09", "150000.00");
    assert!(debts.validate().is_ok());

    let mut bad_slot = debts.clone();
    bad_slot.debts[0].colour_slot = COLOUR_SLOTS;
    assert_eq!(code(bad_slot.validate()), "DEBT_DATA_INVALID");

    let mut orphan = debts.clone();
    orphan.debts.clear();
    assert_eq!(code(orphan.validate()), "DEBT_DATA_INVALID");

    let mut duplicate = debts.clone();
    let statement = duplicate.statements[0].clone();
    duplicate.statements.push(statement);
    assert_eq!(code(duplicate.validate()), "DEBT_DATA_INVALID");
}

// CSD-012 REQ-2 to REQ-5

#[test]
fn test_csd012_ac_2_the_series_runs_from_the_first_statement_to_the_current_year() {
    let mut debts = Debts::default();
    let id = debt(&mut debts, "Example Mortgage", "Monthly");
    add(&mut debts, id, "2024-03", "100.00");
    add(&mut debts, id, "2025-05", "100.00");
    add(&mut debts, id, "2026-05", "100.00");
    let years: Vec<i32> = debts
        .insights(today())
        .years
        .iter()
        .map(|y| y.year)
        .collect();
    assert_eq!(years, [2024, 2025, 2026]);

    // AC-2.2 and AC-2.3
    let mut single = Debts::default();
    let one = debt(&mut single, "Example Mortgage", "Monthly");
    assert_eq!(single.insights(today()).years.len(), 0);
    add(&mut single, one, "2026-09", "100.00");
    assert_eq!(single.insights(today()).years.len(), 1);
}

#[test]
fn test_csd012_ac_3_1_a_gap_year_is_zero_without_segments() {
    let mut debts = Debts::default();
    let id = debt(&mut debts, "Example Student Loan", "Yearly");
    add(&mut debts, id, "2022", "100.00");
    add(&mut debts, id, "2024", "200.00");
    let years = debts.insights(today()).years;
    let gap = years.iter().find(|y| y.year == 2023).unwrap();
    assert_eq!(gap.closing_balance.to_string(), "0.00");
    assert_eq!(gap.repaid, None);
    assert_eq!(gap.statement_count, 0);
    assert_eq!(gap.segments.len(), 0);
    let numbers: Vec<i32> = years.iter().map(|y| y.year).collect();
    assert_eq!(numbers, [2022, 2023, 2024, 2025, 2026]);
}

#[test]
fn test_csd012_ac_4_the_current_year_is_partial_even_without_statements() {
    let mut debts = Debts::default();
    let id = debt(&mut debts, "Example Student Loan", "Yearly");
    add(&mut debts, id, "2025", "100.00");
    let years = debts.insights(today()).years;
    assert_eq!(years.len(), 2);
    assert!(!years[0].is_partial);
    assert!(years[1].is_partial);
    assert_eq!(years[1].statement_count, 0);
}

#[test]
fn test_csd012_ac_5_1_the_segments_are_the_latest_balances_per_debt() {
    let mut debts = Debts::default();
    let mortgage = debt(&mut debts, "Example Mortgage", "Monthly");
    let loan = debt(&mut debts, "Example Student Loan", "Monthly");
    add(&mut debts, mortgage, "2026-03", "151000.00");
    add(&mut debts, mortgage, "2026-09", "150000.00");
    add(&mut debts, loan, "2026-06", "8000.00");
    let year = debts.insights(today()).years.remove(0);
    let segments: Vec<String> = year
        .segments
        .iter()
        .map(|s| s.balance.to_string())
        .collect();
    assert_eq!(segments, ["150000.00", "8000.00"]);
    assert_eq!(year.closing_balance.to_string(), "158000.00");
    assert_eq!(year.statement_count, 3);
}

#[test]
fn test_csd012_ac_5_2_a_debt_without_a_statement_in_the_year_has_no_segment() {
    let mut debts = Debts::default();
    let mortgage = debt(&mut debts, "Example Mortgage", "Monthly");
    let loan = debt(&mut debts, "Example Student Loan", "Yearly");
    add(&mut debts, mortgage, "2025-12", "10000.00");
    add(&mut debts, mortgage, "2026-03", "9500.00");
    add(&mut debts, loan, "2025", "6000.00");
    let years = debts.insights(today()).years;
    assert_eq!(years[0].segments.len(), 2);
    assert_eq!(years[1].segments.len(), 1);
    assert_eq!(years[1].segments[0].debt_id, mortgage);
}

#[test]
fn test_csd012_ac_5_3_a_repaid_debt_has_a_zero_segment() {
    let mut debts = Debts::default();
    let mortgage = debt(&mut debts, "Example Mortgage", "Monthly");
    let car = debt(&mut debts, "Example Car Loan", "Monthly");
    add(&mut debts, mortgage, "2026-09", "150000.00");
    add(&mut debts, car, "2026-09", "0.00");
    let year = debts.insights(today()).years.remove(0);
    let segments: Vec<String> = year
        .segments
        .iter()
        .map(|s| s.balance.to_string())
        .collect();
    assert_eq!(segments, ["150000.00", "0.00"]);
    assert_eq!(year.closing_balance.to_string(), "150000.00");
    assert_eq!(year.statement_count, 2);
}

#[test]
fn test_csd012_ac_5_4_segments_and_debts_follow_creation_order_not_names() {
    let mut debts = Debts::default();
    let loan = debt(&mut debts, "Example Student Loan", "Monthly");
    let mortgage = debt(&mut debts, "Example Mortgage", "Monthly");
    add(&mut debts, loan, "2026-09", "1.00");
    add(&mut debts, mortgage, "2026-09", "2.00");
    let insights = debts.insights(today());
    assert_eq!(insights.years[0].segments[0].debt_id, loan);
    let names: Vec<&str> = insights.debts.iter().map(|d| d.name.as_str()).collect();
    assert_eq!(names, ["Example Student Loan", "Example Mortgage"]);
}

#[test]
fn test_csd012_ac_5_5_the_insights_equal_the_yearly_totals() {
    let mut debts = Debts::default();
    let mortgage = debt(&mut debts, "Example Mortgage", "Monthly");
    let loan = debt(&mut debts, "Example Student Loan", "Yearly");
    add(&mut debts, mortgage, "2025-11", "10000.00");
    add(&mut debts, mortgage, "2025-12", "9800.00");
    add(&mut debts, mortgage, "2026-03", "9000.00");
    add(&mut debts, loan, "2024", "7000.00");
    add(&mut debts, loan, "2025", "6000.00");
    let totals = debts.yearly_totals();
    for year in debts.insights(today()).years {
        let total = totals.iter().find(|t| t.year == year.year).unwrap();
        assert_eq!(year.closing_balance, total.closing_balance);
        assert_eq!(year.repaid, total.repaid);
        assert_eq!(year.statement_count, total.statement_count);
    }
}

#[test]
fn test_csd012_ac_9_2_a_debt_without_statements_is_not_in_the_legend() {
    let mut debts = Debts::default();
    debt(&mut debts, "Example Empty", "Monthly");
    let mortgage = debt(&mut debts, "Example Mortgage", "Monthly");
    add(&mut debts, mortgage, "2026-09", "1.00");
    let insights = debts.insights(today());
    assert_eq!(insights.debts.len(), 1);
    assert_eq!(insights.debts[0].id, mortgage);
}

#[test]
fn test_csd012_ac_15_3_a_moved_statement_follows_to_its_new_year() {
    let mut debts = Debts::default();
    let id = debt(&mut debts, "Example Mortgage", "Monthly");
    add(&mut debts, id, "2026-01", "100.00");
    update(&mut debts, id, "2026-01", "2025-12", "100.00").unwrap();
    let years = debts.insights(today()).years;
    assert_eq!(
        years.iter().map(|y| y.year).collect::<Vec<_>>(),
        [2025, 2026]
    );
    assert_eq!(years[0].segments.len(), 1);
    assert_eq!(years[1].segments.len(), 0);
}
