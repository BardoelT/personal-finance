use super::account::COLOUR_SLOTS;
use super::*;
use crate::domain::income::PayPeriod;

fn period(text: &str) -> Period {
    parse_period(text).unwrap()
}

fn today() -> YearMonth {
    YearMonth::parse("2026-10").unwrap()
}

fn balance(text: &str) -> BankBalance {
    BankBalance::parse(text).unwrap()
}

fn input(name: &str, period: &str) -> AccountInput {
    AccountInput {
        name: name.to_owned(),
        entry_period: Some(period.to_owned()),
    }
}

fn account(banks: &mut BankAccounts, name: &str, period: &str) -> BankAccountId {
    banks.create_account(&input(name, period)).unwrap().id
}

fn add(banks: &mut BankAccounts, id: BankAccountId, at: &str, amount: &str) {
    banks
        .add_entry(id, period(at), balance(amount), Note::default(), today())
        .unwrap();
}

fn add_error(banks: &mut BankAccounts, id: BankAccountId, at: &str, amount: &str) -> DomainError {
    match parse_period(at)
        .and_then(|p| BankBalance::parse(amount).map(|b| (p, b)))
        .and_then(|(p, b)| banks.add_entry(id, p, b, Note::default(), today()))
    {
        Ok(_) => panic!("expected an error"),
        Err(error) => error,
    }
}

fn codes<T>(result: Result<T, DomainError>) -> &'static str {
    result.err().map_or("OK", |error| error.code())
}

// REQ-1, REQ-2

#[test]
fn test_csd009_ac_1_1_an_account_is_monthly_by_default() {
    let mut banks = BankAccounts::default();
    let created = banks
        .create_account(&AccountInput {
            name: "Example Current Account".to_owned(),
            entry_period: None,
        })
        .unwrap();
    assert_eq!(created.entry_period, PayPeriod::Monthly);
    assert_eq!(banks.entry_count(created.id), 0);
}

#[test]
fn test_csd009_ac_1_2_a_yearly_account() {
    let mut banks = BankAccounts::default();
    let id = account(&mut banks, "Example Savings", "Yearly");
    assert_eq!(banks.accounts()[0].id, id);
    assert_eq!(banks.accounts()[0].entry_period, PayPeriod::Yearly);
}

#[test]
fn test_csd009_ac_1_3_an_unknown_period_is_rejected() {
    let mut banks = BankAccounts::default();
    let result = banks.create_account(&input("Example Current Account", "Weekly"));
    assert_eq!(codes(result), "BANK_ACCOUNT_PERIOD_UNSUPPORTED");
    assert_eq!(banks.accounts().len(), 0);
}

#[test]
fn test_csd009_ac_2_1_invalid_names_are_rejected() {
    let mut banks = BankAccounts::default();
    account(&mut banks, "Example Current Account", "Monthly");
    for (name, code) in [
        ("", "BANK_ACCOUNT_NAME_REQUIRED"),
        ("   ", "BANK_ACCOUNT_NAME_REQUIRED"),
        ("EXAMPLE CURRENT ACCOUNT", "BANK_ACCOUNT_NAME_DUPLICATE"),
        ("example current account", "BANK_ACCOUNT_NAME_DUPLICATE"),
    ] {
        let result = banks.create_account(&input(name, "Monthly"));
        assert_eq!(codes(result), code, "{name:?}");
    }
    assert_eq!(banks.accounts().len(), 1);
}

#[test]
fn test_csd009_ac_2_2_name_length_boundary() {
    let mut banks = BankAccounts::default();
    assert!(banks
        .create_account(&input(&"a".repeat(100), "Monthly"))
        .is_ok());
    let result = banks.create_account(&input(&"b".repeat(101), "Monthly"));
    assert_eq!(codes(result), "BANK_ACCOUNT_NAME_TOO_LONG");
}

#[test]
fn test_csd009_ac_2_3_the_name_is_stored_trimmed() {
    let mut banks = BankAccounts::default();
    let created = banks
        .create_account(&input("  Example Current Account  ", "Monthly"))
        .unwrap();
    assert_eq!(created.name, "Example Current Account");
}

#[test]
fn test_csd009_ac_2_4_renaming_to_an_existing_name_is_rejected() {
    let mut banks = BankAccounts::default();
    account(&mut banks, "Example Current Account", "Monthly");
    let savings = account(&mut banks, "Example Savings", "Monthly");
    let result = banks.update_account(savings, &input("example current account", "Monthly"));
    assert_eq!(codes(result), "BANK_ACCOUNT_NAME_DUPLICATE");
}

#[test]
fn test_csd009_ac_2_5_changing_only_the_case_of_its_own_name_is_allowed() {
    let mut banks = BankAccounts::default();
    let id = account(&mut banks, "Example Savings", "Monthly");
    let renamed = banks
        .update_account(id, &input("EXAMPLE SAVINGS", "Monthly"))
        .unwrap();
    assert_eq!(renamed.name, "EXAMPLE SAVINGS");
}

// REQ-3

#[test]
fn test_csd009_ac_3_1_slots_follow_creation_order() {
    let mut banks = BankAccounts::default();
    let slots: Vec<u8> = ["A", "B", "C"]
        .iter()
        .map(|name| {
            banks
                .create_account(&input(name, "Monthly"))
                .unwrap()
                .colour_slot
        })
        .collect();
    assert_eq!(slots, [0, 1, 2]);
}

#[test]
fn test_csd009_ac_3_2_slots_cycle_after_eight_accounts() {
    let mut banks = BankAccounts::default();
    for index in 0..COLOUR_SLOTS {
        account(&mut banks, &format!("Bank {index}"), "Monthly");
    }
    let ninth = banks.create_account(&input("Bank 9", "Monthly")).unwrap();
    assert_eq!(ninth.colour_slot, 0);
}

#[test]
fn test_csd009_ac_3_3_deleting_an_account_does_not_recolour_the_others() {
    let mut banks = BankAccounts::default();
    let first = account(&mut banks, "A", "Monthly");
    account(&mut banks, "B", "Monthly");
    account(&mut banks, "C", "Monthly");
    banks.delete_account(first).unwrap();
    let slots: Vec<u8> = banks.accounts().iter().map(|a| a.colour_slot).collect();
    assert_eq!(slots, [1, 2]);
}

#[test]
fn test_csd009_ac_3_4_a_freed_slot_is_used_first() {
    let mut banks = BankAccounts::default();
    account(&mut banks, "A", "Monthly");
    let second = account(&mut banks, "B", "Monthly");
    account(&mut banks, "C", "Monthly");
    banks.delete_account(second).unwrap();
    let new = banks.create_account(&input("D", "Monthly")).unwrap();
    assert_eq!(new.colour_slot, 1);
}

#[test]
fn test_csd009_ac_3_5_renaming_keeps_the_slot() {
    let mut banks = BankAccounts::default();
    account(&mut banks, "A", "Monthly");
    account(&mut banks, "B", "Monthly");
    let third = account(&mut banks, "C", "Monthly");
    let renamed = banks
        .update_account(third, &input("Renamed", "Monthly"))
        .unwrap();
    assert_eq!(renamed.colour_slot, 2);
}

// REQ-4 to REQ-6

#[test]
fn test_csd009_ac_4_1_rename_keeps_entries() {
    let mut banks = BankAccounts::default();
    let id = account(&mut banks, "Example Current Account", "Monthly");
    add(&mut banks, id, "2026-08", "100.00");
    add(&mut banks, id, "2026-09", "200.00");
    banks
        .update_account(id, &input("Example Account", "Monthly"))
        .unwrap();
    assert_eq!(banks.entry_count(id), 2);
}

#[test]
fn test_csd009_ac_4_2_the_period_of_an_account_without_entries_can_change() {
    let mut banks = BankAccounts::default();
    let id = account(&mut banks, "Example Current Account", "Monthly");
    let changed = banks
        .update_account(id, &input("Example Current Account", "Yearly"))
        .unwrap();
    assert_eq!(changed.entry_period, PayPeriod::Yearly);
}

#[test]
fn test_csd009_ac_5_1_the_period_is_locked_once_entries_exist() {
    let mut banks = BankAccounts::default();
    let id = account(&mut banks, "Example Current Account", "Monthly");
    add(&mut banks, id, "2026-09", "100.00");
    let result = banks.update_account(id, &input("Example Current Account", "Yearly"));
    assert_eq!(codes(result), "BANK_ACCOUNT_PERIOD_LOCKED");
    assert_eq!(banks.accounts()[0].entry_period, PayPeriod::Monthly);
}

#[test]
fn test_csd009_ac_5_2_the_period_can_change_after_deleting_the_last_entry() {
    let mut banks = BankAccounts::default();
    let id = account(&mut banks, "Example Current Account", "Monthly");
    add(&mut banks, id, "2026-09", "100.00");
    banks.delete_entry(id, period("2026-09")).unwrap();
    assert!(banks
        .update_account(id, &input("Example Current Account", "Yearly"))
        .is_ok());
}

#[test]
fn test_csd009_ac_6_1_deleting_an_account_removes_its_entries() {
    let mut banks = BankAccounts::default();
    let id = account(&mut banks, "Example Current Account", "Monthly");
    let other = account(&mut banks, "Example Savings", "Monthly");
    for month in ["2026-07", "2026-08", "2026-09"] {
        add(&mut banks, id, month, "100.00");
    }
    add(&mut banks, other, "2026-09", "50.00");
    banks.delete_account(id).unwrap();
    assert_eq!(banks.accounts().len(), 1);
    assert_eq!(banks.total_entry_count(), 1);
    assert_eq!(codes(banks.delete_account(id)), "BANK_ACCOUNT_NOT_FOUND");
}

// REQ-7 to REQ-9

#[test]
fn test_csd009_ac_7_1_a_monthly_entry_with_a_note() {
    let mut banks = BankAccounts::default();
    let id = account(&mut banks, "Example Current Account", "Monthly");
    let entry = banks
        .add_entry(
            id,
            period("2026-09"),
            balance("2500.00"),
            bank_note("Salary received").unwrap(),
            today(),
        )
        .unwrap();
    assert_eq!(entry.balance.value().to_string(), "2500.00");
    assert_eq!(entry.note.as_str(), "Salary received");
}

#[test]
fn test_csd009_ac_7_2_a_yearly_entry_without_a_note() {
    let mut banks = BankAccounts::default();
    let id = account(&mut banks, "Example Savings", "Yearly");
    add(&mut banks, id, "2025", "8000.00");
    assert_eq!(banks.all_entries()[0].1.note.as_str(), "");
}

#[test]
fn test_csd009_ac_7_3_old_periods_are_accepted() {
    let mut banks = BankAccounts::default();
    let id = account(&mut banks, "Example Current Account", "Monthly");
    add(&mut banks, id, "2019-01", "1000.00");
    assert_eq!(banks.entry_count(id), 1);
}

#[test]
fn test_csd009_ac_7_4_the_current_period_is_accepted_and_a_later_one_is_not() {
    let mut banks = BankAccounts::default();
    let monthly = account(&mut banks, "Monthly", "Monthly");
    let yearly = account(&mut banks, "Yearly", "Yearly");
    add(&mut banks, monthly, "2026-10", "1000.00");
    add(&mut banks, yearly, "2026", "1000.00");
    assert_eq!(
        add_error(&mut banks, monthly, "2026-11", "1000.00").code(),
        "BANK_ENTRY_IN_FUTURE"
    );
    assert_eq!(
        add_error(&mut banks, yearly, "2027", "1000.00").code(),
        "BANK_ENTRY_IN_FUTURE"
    );
}

#[test]
fn test_csd009_ac_7_5_an_invalid_period_is_rejected() {
    let mut banks = BankAccounts::default();
    let monthly = account(&mut banks, "Monthly", "Monthly");
    let yearly = account(&mut banks, "Yearly", "Yearly");
    for (id, text) in [
        (monthly, "2026-13"),
        (monthly, "2026-9"),
        (monthly, "abcd"),
        (yearly, "26"),
        (yearly, "abcd"),
        (yearly, "1899"),
    ] {
        assert_eq!(
            add_error(&mut banks, id, text, "1000.00").code(),
            "BANK_ENTRY_PERIOD_INVALID",
            "{text}"
        );
    }
}

#[test]
fn test_csd009_ac_8_1_a_duplicate_period_is_rejected() {
    let mut banks = BankAccounts::default();
    let id = account(&mut banks, "Example Current Account", "Monthly");
    banks
        .add_entry(
            id,
            period("2026-09"),
            balance("2500.00"),
            bank_note("First").unwrap(),
            today(),
        )
        .unwrap();
    assert_eq!(
        add_error(&mut banks, id, "2026-09", "3000.00").code(),
        "BANK_ENTRY_DUPLICATE"
    );
    let rows = banks.all_entries();
    assert_eq!(rows[0].1.balance.value().to_string(), "2500.00");
    assert_eq!(rows[0].1.note.as_str(), "First");
}

#[test]
fn test_csd009_ac_8_2_the_same_period_on_another_account_is_allowed() {
    let mut banks = BankAccounts::default();
    let first = account(&mut banks, "A", "Monthly");
    let second = account(&mut banks, "B", "Monthly");
    add(&mut banks, first, "2026-09", "100.00");
    add(&mut banks, second, "2026-09", "200.00");
    assert_eq!(banks.total_entry_count(), 2);
}

#[test]
fn test_csd009_ac_9_1_a_monthly_entry_for_a_yearly_account_is_rejected() {
    let mut banks = BankAccounts::default();
    let id = account(&mut banks, "Yearly", "Yearly");
    assert_eq!(
        add_error(&mut banks, id, "2026-09", "1000.00").code(),
        "BANK_ENTRY_PERIOD_MISMATCH"
    );
}

#[test]
fn test_csd009_ac_9_2_a_yearly_entry_for_a_monthly_account_is_rejected() {
    let mut banks = BankAccounts::default();
    let id = account(&mut banks, "Monthly", "Monthly");
    assert_eq!(
        add_error(&mut banks, id, "2025", "1000.00").code(),
        "BANK_ENTRY_PERIOD_MISMATCH"
    );
}

// REQ-10, REQ-11

#[test]
fn test_csd009_ac_10_1_invalid_balances_are_rejected() {
    for (text, code) in [
        ("", "BANK_ENTRY_BALANCE_INVALID"),
        ("abc", "BANK_ENTRY_BALANCE_INVALID"),
        ("1e3", "BANK_ENTRY_BALANCE_INVALID"),
        ("2500,50", "BANK_ENTRY_BALANCE_INVALID"),
        ("1000.999", "BANK_ENTRY_BALANCE_PRECISION"),
        ("-1000.999", "BANK_ENTRY_BALANCE_PRECISION"),
        ("1000000000.01", "BANK_ENTRY_BALANCE_TOO_LARGE"),
        ("-1000000000.01", "BANK_ENTRY_BALANCE_TOO_LARGE"),
    ] {
        assert_eq!(codes(BankBalance::parse(text)), code, "{text:?}");
    }
}

#[test]
fn test_csd009_ac_10_2_boundary_and_negative_balances_are_accepted() {
    for text in ["0.00", "-150.25", "1000000000.00", "-1000000000.00"] {
        assert_eq!(balance(text).value().to_string(), text);
    }
}

#[test]
fn test_csd009_ac_10_3_an_invalid_balance_in_an_edit_changes_nothing() {
    let mut banks = BankAccounts::default();
    let id = account(&mut banks, "Example Current Account", "Monthly");
    add(&mut banks, id, "2026-09", "2500.00");
    assert_eq!(
        codes(BankBalance::parse("1000.999")),
        "BANK_ENTRY_BALANCE_PRECISION"
    );
    assert_eq!(
        banks.all_entries()[0].1.balance.value().to_string(),
        "2500.00"
    );
}

#[test]
fn test_csd009_ac_11_1_note_length_boundary() {
    assert!(bank_note(&"a".repeat(2000)).is_ok());
    assert_eq!(
        codes(bank_note(&"a".repeat(2001))),
        "BANK_ENTRY_NOTE_TOO_LONG"
    );
}

#[test]
fn test_csd009_ac_11_2_line_breaks_are_kept() {
    assert_eq!(
        bank_note("line one\nline two").unwrap().as_str(),
        "line one\nline two"
    );
}

// REQ-12, REQ-13

fn update(
    banks: &mut BankAccounts,
    id: BankAccountId,
    from: &str,
    to: &str,
    amount: &str,
) -> Result<BankEntry, DomainError> {
    banks.update_entry(
        id,
        period(from),
        period(to),
        balance(amount),
        Note::default(),
        today(),
    )
}

#[test]
fn test_csd009_ac_12_1_edit_balance_and_note() {
    let mut banks = BankAccounts::default();
    let id = account(&mut banks, "Example Current Account", "Monthly");
    add(&mut banks, id, "2026-09", "2500.00");
    let entry = banks
        .update_entry(
            id,
            period("2026-09"),
            period("2026-09"),
            balance("2750.00"),
            bank_note("Corrected").unwrap(),
            today(),
        )
        .unwrap();
    assert_eq!(entry.balance.value().to_string(), "2750.00");
    assert_eq!(entry.note.as_str(), "Corrected");
    // AC-12.2: an empty note clears it.
    update(&mut banks, id, "2026-09", "2026-09", "2750.00").unwrap();
    assert_eq!(banks.all_entries()[0].1.note.as_str(), "");
}

#[test]
fn test_csd009_ac_12_3_an_entry_moves_to_another_month() {
    let mut banks = BankAccounts::default();
    let id = account(&mut banks, "Example Current Account", "Monthly");
    banks
        .add_entry(
            id,
            period("2026-09"),
            balance("2500.00"),
            bank_note("First").unwrap(),
            today(),
        )
        .unwrap();
    banks
        .update_entry(
            id,
            period("2026-09"),
            period("2026-08"),
            balance("2500.00"),
            bank_note("First").unwrap(),
            today(),
        )
        .unwrap();
    let rows = banks.all_entries();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].1.period.to_string(), "2026-08");
    assert_eq!(rows[0].1.note.as_str(), "First");
}

#[test]
fn test_csd009_ac_12_4_the_period_and_the_balance_change_together() {
    let mut banks = BankAccounts::default();
    let id = account(&mut banks, "Example Current Account", "Monthly");
    add(&mut banks, id, "2026-09", "2500.00");
    update(&mut banks, id, "2026-09", "2026-08", "2750.00").unwrap();
    let rows = banks.all_entries();
    assert_eq!(rows[0].1.period.to_string(), "2026-08");
    assert_eq!(rows[0].1.balance.value().to_string(), "2750.00");
}

#[test]
fn test_csd009_ac_12_5_a_yearly_entry_moves() {
    let mut banks = BankAccounts::default();
    let id = account(&mut banks, "Example Savings", "Yearly");
    add(&mut banks, id, "2025", "8000.00");
    update(&mut banks, id, "2025", "2024", "8000.00").unwrap();
    assert_eq!(banks.all_entries()[0].1.period.to_string(), "2024");
}

#[test]
fn test_csd009_ac_12_6_the_yearly_totals_follow_the_move() {
    let mut banks = BankAccounts::default();
    let id = account(&mut banks, "Example Current Account", "Monthly");
    add(&mut banks, id, "2025-11", "2000.00");
    add(&mut banks, id, "2025-12", "2500.00");
    add(&mut banks, id, "2026-05", "3000.00");
    update(&mut banks, id, "2025-12", "2026-01", "2500.00").unwrap();
    let totals = banks.yearly_totals();
    assert_eq!(totals[0].year, 2026);
    assert_eq!(totals[0].closing_balance.to_string(), "3000.00");
    assert_eq!(totals[0].entry_count, 2);
    assert_eq!(totals[1].closing_balance.to_string(), "2000.00");
    assert_eq!(totals[1].entry_count, 1);
}

#[test]
fn test_csd009_ac_12_8_keeping_the_period_is_not_a_duplicate() {
    let mut banks = BankAccounts::default();
    let id = account(&mut banks, "Example Current Account", "Monthly");
    add(&mut banks, id, "2026-09", "2500.00");
    let entry = update(&mut banks, id, "2026-09", "2026-09", "2750.00").unwrap();
    assert_eq!(entry.balance.value().to_string(), "2750.00");
}

#[test]
fn test_csd009_ac_13_1_a_new_period_in_the_future_is_rejected() {
    let mut banks = BankAccounts::default();
    let monthly = account(&mut banks, "Monthly", "Monthly");
    let yearly = account(&mut banks, "Yearly", "Yearly");
    add(&mut banks, monthly, "2026-09", "2500.00");
    add(&mut banks, yearly, "2025", "2500.00");
    assert_eq!(
        codes(update(&mut banks, monthly, "2026-09", "2026-11", "1.00")),
        "BANK_ENTRY_IN_FUTURE"
    );
    assert_eq!(
        codes(update(&mut banks, yearly, "2025", "2027", "1.00")),
        "BANK_ENTRY_IN_FUTURE"
    );
}

#[test]
fn test_csd009_ac_13_2_a_taken_period_is_rejected_and_nothing_changes() {
    let mut banks = BankAccounts::default();
    let id = account(&mut banks, "Example Current Account", "Monthly");
    add(&mut banks, id, "2026-08", "2000.00");
    add(&mut banks, id, "2026-09", "2500.00");
    assert_eq!(
        codes(update(&mut banks, id, "2026-09", "2026-08", "3000.00")),
        "BANK_ENTRY_DUPLICATE"
    );
    let balances: Vec<String> = banks
        .all_entries()
        .iter()
        .map(|(_, e)| format!("{} {}", e.period, e.balance.value()))
        .collect();
    assert_eq!(balances, ["2026-09 2500.00", "2026-08 2000.00"]);
}

#[test]
fn test_csd009_ac_13_3_a_period_of_the_wrong_kind_is_rejected() {
    let mut banks = BankAccounts::default();
    let monthly = account(&mut banks, "Monthly", "Monthly");
    let yearly = account(&mut banks, "Yearly", "Yearly");
    add(&mut banks, monthly, "2026-09", "1.00");
    add(&mut banks, yearly, "2025", "1.00");
    assert_eq!(
        codes(update(&mut banks, monthly, "2026-09", "2026", "1.00")),
        "BANK_ENTRY_PERIOD_MISMATCH"
    );
    assert_eq!(
        codes(update(&mut banks, yearly, "2025", "2024-12", "1.00")),
        "BANK_ENTRY_PERIOD_MISMATCH"
    );
}

#[test]
fn test_csd009_ac_13_5_another_account_may_have_the_same_period() {
    let mut banks = BankAccounts::default();
    let first = account(&mut banks, "A", "Monthly");
    let second = account(&mut banks, "B", "Monthly");
    add(&mut banks, first, "2026-08", "1.00");
    add(&mut banks, second, "2026-09", "2.00");
    update(&mut banks, second, "2026-09", "2026-08", "2.00").unwrap();
    assert!(banks
        .all_entries()
        .iter()
        .all(|(_, entry)| entry.period.to_string() == "2026-08"));
}

#[test]
fn test_csd009_ac_13_6_an_invalid_period_is_rejected() {
    assert_eq!(codes(parse_period("2026-13")), "BANK_ENTRY_PERIOD_INVALID");
}

// REQ-14, REQ-15

#[test]
fn test_csd009_ac_14_1_deleting_an_entry_removes_only_that_entry() {
    let mut banks = BankAccounts::default();
    let id = account(&mut banks, "Example Current Account", "Monthly");
    add(&mut banks, id, "2026-08", "1.00");
    add(&mut banks, id, "2026-09", "2.00");
    banks.delete_entry(id, period("2026-09")).unwrap();
    let rows = banks.all_entries();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].1.period.to_string(), "2026-08");
    assert_eq!(
        codes(banks.delete_entry(id, period("2026-09"))),
        "BANK_ENTRY_NOT_FOUND"
    );
}

#[test]
fn test_csd009_ac_15_1_the_form_is_prefilled_from_the_latest_entry_without_a_note() {
    let mut banks = BankAccounts::default();
    let id = account(&mut banks, "Example Current Account", "Monthly");
    add(&mut banks, id, "2026-07", "2000.00");
    banks
        .add_entry(
            id,
            period("2026-08"),
            balance("2500.00"),
            bank_note("Salary received").unwrap(),
            today(),
        )
        .unwrap();
    let prefill = banks.prefill_from_latest(id).unwrap();
    assert_eq!(prefill.balance.unwrap().to_string(), "2500.00");
    assert_eq!(banks.entry_count(id), 2);
}

#[test]
fn test_csd009_ac_15_2_an_account_without_entries_has_an_empty_prefill() {
    let mut banks = BankAccounts::default();
    let id = account(&mut banks, "Example Current Account", "Monthly");
    assert_eq!(banks.prefill_from_latest(id).unwrap().balance, None);
}

#[test]
fn test_csd009_ac_15_3_a_negative_balance_is_copied_as_it_is() {
    let mut banks = BankAccounts::default();
    let id = account(&mut banks, "Example Current Account", "Monthly");
    add(&mut banks, id, "2026-08", "-150.25");
    assert_eq!(
        banks
            .prefill_from_latest(id)
            .unwrap()
            .balance
            .unwrap()
            .to_string(),
        "-150.25"
    );
}

// REQ-17, REQ-18

#[test]
fn test_csd009_ac_17_1_the_latest_balance_per_account() {
    let mut banks = BankAccounts::default();
    let id = account(&mut banks, "Example Current Account", "Monthly");
    add(&mut banks, id, "2026-07", "2000.00");
    add(&mut banks, id, "2026-09", "2500.00");
    let overview = banks.overview();
    assert_eq!(
        overview.accounts[0].latest_balance.unwrap().to_string(),
        "2500.00"
    );
    assert_eq!(
        overview.accounts[0].latest_period.unwrap().to_string(),
        "2026-09"
    );
}

#[test]
fn test_csd009_ac_17_2_the_total_spans_accounts_with_different_periods() {
    let mut banks = BankAccounts::default();
    let monthly = account(&mut banks, "Example Current Account", "Monthly");
    let yearly = account(&mut banks, "Example Savings", "Yearly");
    add(&mut banks, monthly, "2026-09", "2500.50");
    add(&mut banks, yearly, "2025", "8000.00");
    let overview = banks.overview();
    assert_eq!(overview.total.unwrap().to_string(), "10500.50");
    let periods: Vec<String> = overview
        .accounts
        .iter()
        .map(|a| a.latest_period.unwrap().to_string())
        .collect();
    assert_eq!(periods, ["2026-09", "2025"]);
}

#[test]
fn test_csd009_ac_17_3_accounts_without_entries_do_not_count() {
    let mut banks = BankAccounts::default();
    let id = account(&mut banks, "Example Current Account", "Monthly");
    account(&mut banks, "Example Empty", "Monthly");
    add(&mut banks, id, "2026-09", "2500.00");
    assert_eq!(banks.overview().total.unwrap().to_string(), "2500.00");
    let empty = BankAccounts::default();
    assert_eq!(empty.overview().total, None);
}

#[test]
fn test_csd009_ac_17_4_an_overdrawn_account_lowers_the_total() {
    let mut banks = BankAccounts::default();
    let current = account(&mut banks, "Example Current Account", "Monthly");
    let overdraft = account(&mut banks, "Example Overdraft", "Monthly");
    add(&mut banks, current, "2026-09", "2500.00");
    add(&mut banks, overdraft, "2026-09", "-400.00");
    let overview = banks.overview();
    assert_eq!(overview.total.unwrap().to_string(), "2100.00");
    let latest: Vec<String> = overview
        .accounts
        .iter()
        .map(|a| a.latest_balance.unwrap().to_string())
        .collect();
    assert_eq!(latest, ["2500.00", "-400.00"]);
}

#[test]
fn test_csd009_ac_17_5_a_new_latest_entry_updates_the_total_and_an_older_one_does_not() {
    let mut banks = BankAccounts::default();
    let id = account(&mut banks, "Example Current Account", "Monthly");
    add(&mut banks, id, "2026-09", "2500.00");
    add(&mut banks, id, "2026-01", "900.00");
    assert_eq!(banks.overview().total.unwrap().to_string(), "2500.00");
    add(&mut banks, id, "2026-10", "3000.00");
    let overview = banks.overview();
    assert_eq!(overview.total.unwrap().to_string(), "3000.00");
    assert_eq!(
        overview.accounts[0].latest_period.unwrap().to_string(),
        "2026-10"
    );
}

#[test]
fn test_csd009_ac_18_1_accounts_are_alphabetical_ignoring_case() {
    let mut banks = BankAccounts::default();
    for name in [
        "example savings",
        "Example Current Account",
        "Example Joint",
    ] {
        account(&mut banks, name, "Monthly");
    }
    let names: Vec<&str> = banks.accounts().iter().map(|a| a.name.as_str()).collect();
    assert_eq!(
        names,
        [
            "Example Current Account",
            "Example Joint",
            "example savings"
        ]
    );
}

#[test]
fn test_csd009_ac_18_2_entries_are_newest_first_and_a_year_counts_as_december() {
    let mut banks = BankAccounts::default();
    let monthly = account(&mut banks, "Monthly", "Monthly");
    let yearly = account(&mut banks, "Yearly", "Yearly");
    add(&mut banks, monthly, "2025-11", "1.00");
    add(&mut banks, monthly, "2025-12", "1.00");
    add(&mut banks, yearly, "2025", "1.00");
    add(&mut banks, monthly, "2026-07", "1.00");
    let order: Vec<String> = banks
        .all_entries()
        .iter()
        .map(|(a, e)| format!("{} {}", a.name, e.period))
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
fn test_csd009_ac_18_4_the_same_period_is_ordered_by_account() {
    let mut banks = BankAccounts::default();
    let joint = account(&mut banks, "Example Joint", "Monthly");
    let current = account(&mut banks, "Example Current Account", "Monthly");
    add(&mut banks, joint, "2026-09", "1.00");
    add(&mut banks, current, "2026-09", "1.00");
    assert_eq!(banks.all_entries()[0].0.name, "Example Current Account");
}

// REQ-19

fn year(banks: &BankAccounts, year: i32) -> (String, usize) {
    let total = banks
        .yearly_totals()
        .into_iter()
        .find(|total| total.year == year)
        .unwrap();
    (total.closing_balance.to_string(), total.entry_count)
}

#[test]
fn test_csd009_ac_19_1_totals_per_calendar_year() {
    let mut banks = BankAccounts::default();
    let id = account(&mut banks, "Example Current Account", "Monthly");
    add(&mut banks, id, "2025-11", "2000.00");
    add(&mut banks, id, "2025-12", "2500.00");
    add(&mut banks, id, "2026-05", "3000.00");
    assert_eq!(year(&banks, 2026), ("3000.00".to_owned(), 1));
    assert_eq!(year(&banks, 2025), ("2500.00".to_owned(), 2));
}

#[test]
fn test_csd009_ac_19_2_no_entries_no_totals() {
    let mut banks = BankAccounts::default();
    account(&mut banks, "Example Current Account", "Monthly");
    assert_eq!(banks.yearly_totals().len(), 0);
}

#[test]
fn test_csd009_ac_19_3_totals_add_up_all_accounts() {
    let mut banks = BankAccounts::default();
    let current = account(&mut banks, "Example Current Account", "Monthly");
    let savings = account(&mut banks, "Example Savings", "Monthly");
    add(&mut banks, current, "2026-08", "2000.00");
    add(&mut banks, current, "2026-09", "2500.00");
    add(&mut banks, savings, "2026-09", "500.00");
    assert_eq!(banks.yearly_totals().len(), 1);
    assert_eq!(year(&banks, 2026), ("3000.00".to_owned(), 3));
}

#[test]
fn test_csd009_ac_19_4_the_closing_balance_is_the_latest_balance_not_a_sum() {
    let mut banks = BankAccounts::default();
    let id = account(&mut banks, "Example Current Account", "Monthly");
    add(&mut banks, id, "2026-03", "1000.00");
    add(&mut banks, id, "2026-09", "2000.00");
    assert_eq!(year(&banks, 2026), ("2000.00".to_owned(), 2));
}

#[test]
fn test_csd009_ac_19_5_monthly_and_yearly_accounts_add_up_together() {
    let mut banks = BankAccounts::default();
    let current = account(&mut banks, "Example Current Account", "Monthly");
    let savings = account(&mut banks, "Example Savings", "Yearly");
    add(&mut banks, current, "2025-12", "2000.00");
    add(&mut banks, savings, "2024", "7000.00");
    add(&mut banks, savings, "2025", "8000.00");
    assert_eq!(year(&banks, 2025), ("10000.00".to_owned(), 2));
    assert_eq!(year(&banks, 2024), ("7000.00".to_owned(), 1));
}

#[test]
fn test_csd009_ac_19_6_an_account_without_an_entry_in_a_year_does_not_count_in_it() {
    let mut banks = BankAccounts::default();
    let current = account(&mut banks, "Example Current Account", "Monthly");
    let savings = account(&mut banks, "Example Savings", "Yearly");
    add(&mut banks, current, "2025-12", "2000.00");
    add(&mut banks, current, "2026-03", "2500.00");
    add(&mut banks, savings, "2025", "8000.00");
    assert_eq!(year(&banks, 2026), ("2500.00".to_owned(), 1));
    assert_eq!(year(&banks, 2025), ("10000.00".to_owned(), 2));
}

#[test]
fn test_csd009_ac_19_7_an_overdrawn_account_lowers_the_closing_balance() {
    let mut banks = BankAccounts::default();
    let current = account(&mut banks, "Example Current Account", "Monthly");
    let overdraft = account(&mut banks, "Example Overdraft", "Monthly");
    add(&mut banks, current, "2026-09", "2500.00");
    add(&mut banks, overdraft, "2026-09", "-400.00");
    assert_eq!(year(&banks, 2026), ("2100.00".to_owned(), 2));
}

#[test]
fn test_csd009_ac_19_8_a_new_entry_updates_the_totals() {
    let mut banks = BankAccounts::default();
    let id = account(&mut banks, "Example Current Account", "Monthly");
    add(&mut banks, id, "2026-09", "2500.00");
    add(&mut banks, id, "2026-10", "3000.00");
    assert_eq!(year(&banks, 2026), ("3000.00".to_owned(), 2));
}

#[test]
fn test_csd009_ac_19_9_newest_year_first() {
    let mut banks = BankAccounts::default();
    let id = account(&mut banks, "Example Savings", "Yearly");
    for text in ["2024", "2025", "2026"] {
        add(&mut banks, id, text, "1.00");
    }
    let years: Vec<i32> = banks.yearly_totals().iter().map(|t| t.year).collect();
    assert_eq!(years, [2026, 2025, 2024]);
}

// The data of a file

#[test]
fn test_csd009_a_damaged_section_is_not_consistent() {
    let mut banks = BankAccounts::default();
    let id = account(&mut banks, "Example Current Account", "Monthly");
    add(&mut banks, id, "2026-09", "2500.00");
    assert!(banks.validate().is_ok());

    let mut bad_slot = banks.clone();
    bad_slot.accounts[0].colour_slot = COLOUR_SLOTS;
    assert_eq!(codes(bad_slot.validate()), "BANK_DATA_INVALID");

    let mut orphan = banks.clone();
    orphan.accounts.clear();
    assert_eq!(codes(orphan.validate()), "BANK_DATA_INVALID");

    let mut duplicate = banks.clone();
    let entry = duplicate.entries[0].clone();
    duplicate.entries.push(entry);
    assert_eq!(codes(duplicate.validate()), "BANK_DATA_INVALID");
}

// CSD-010 REQ-2 to REQ-5

fn insights_today() -> YearMonth {
    YearMonth::parse("2026-10").unwrap()
}

#[test]
fn test_csd010_ac_2_1_the_series_runs_from_the_first_entry_to_the_current_year() {
    let mut banks = BankAccounts::default();
    let id = account(&mut banks, "Example Current Account", "Monthly");
    add(&mut banks, id, "2024-03", "100.00");
    add(&mut banks, id, "2025-05", "100.00");
    add(&mut banks, id, "2026-05", "100.00");
    let years: Vec<i32> = banks
        .insights(insights_today())
        .years
        .iter()
        .map(|y| y.year)
        .collect();
    assert_eq!(years, [2024, 2025, 2026]);
}

#[test]
fn test_csd010_ac_2_2_a_single_year() {
    let mut banks = BankAccounts::default();
    let id = account(&mut banks, "Example Current Account", "Monthly");
    add(&mut banks, id, "2026-09", "100.00");
    assert_eq!(banks.insights(insights_today()).years.len(), 1);
}

#[test]
fn test_csd010_ac_2_3_no_entries_no_series() {
    let mut banks = BankAccounts::default();
    account(&mut banks, "Example Current Account", "Monthly");
    let insights = banks.insights(insights_today());
    assert_eq!(insights.years.len(), 0);
    assert_eq!(insights.banks.len(), 0);
}

#[test]
fn test_csd010_ac_3_1_a_gap_year_is_zero_without_segments() {
    let mut banks = BankAccounts::default();
    let id = account(&mut banks, "Example Savings", "Yearly");
    add(&mut banks, id, "2022", "100.00");
    add(&mut banks, id, "2024", "200.00");
    let years = banks.insights(insights_today()).years;
    let gap = years.iter().find(|y| y.year == 2023).unwrap();
    assert_eq!(gap.closing_balance.to_string(), "0.00");
    assert_eq!(gap.entry_count, 0);
    assert_eq!(gap.segments.len(), 0);
    let numbers: Vec<i32> = years.iter().map(|y| y.year).collect();
    assert_eq!(numbers, [2022, 2023, 2024, 2025, 2026]);
}

#[test]
fn test_csd010_ac_4_1_the_current_year_is_partial() {
    let mut banks = BankAccounts::default();
    let id = account(&mut banks, "Example Savings", "Yearly");
    add(&mut banks, id, "2025", "100.00");
    // AC-4.2: the current year is in the series even without entries.
    let years = banks.insights(insights_today()).years;
    assert_eq!(years.len(), 2);
    assert!(!years[0].is_partial);
    assert!(years[1].is_partial);
    assert_eq!(years[1].entry_count, 0);
}

#[test]
fn test_csd010_ac_5_1_the_segments_are_the_latest_balances_per_bank() {
    let mut banks = BankAccounts::default();
    let current = account(&mut banks, "Example Current Account", "Monthly");
    let savings = account(&mut banks, "Example Savings", "Monthly");
    add(&mut banks, current, "2026-03", "2000.00");
    add(&mut banks, current, "2026-09", "2500.00");
    add(&mut banks, savings, "2026-06", "8000.00");
    let year = banks.insights(insights_today()).years.remove(0);
    let segments: Vec<String> = year
        .segments
        .iter()
        .map(|s| s.balance.to_string())
        .collect();
    assert_eq!(segments, ["2500.00", "8000.00"]);
    assert_eq!(year.closing_balance.to_string(), "10500.00");
    assert_eq!(year.entry_count, 3);
}

#[test]
fn test_csd010_ac_5_2_a_bank_without_an_entry_in_the_year_has_no_segment() {
    let mut banks = BankAccounts::default();
    let current = account(&mut banks, "Example Current Account", "Monthly");
    let savings = account(&mut banks, "Example Savings", "Yearly");
    add(&mut banks, current, "2025-12", "2000.00");
    add(&mut banks, current, "2026-03", "2500.00");
    add(&mut banks, savings, "2025", "8000.00");
    let years = banks.insights(insights_today()).years;
    assert_eq!(years[0].segments.len(), 2);
    assert_eq!(years[1].segments.len(), 1);
    assert_eq!(years[1].segments[0].account_id, current);
}

#[test]
fn test_csd010_ac_5_3_an_overdrawn_bank_has_a_negative_segment() {
    let mut banks = BankAccounts::default();
    let current = account(&mut banks, "Example Current Account", "Monthly");
    let overdraft = account(&mut banks, "Example Overdraft", "Monthly");
    add(&mut banks, current, "2026-09", "5000.00");
    add(&mut banks, overdraft, "2026-09", "-1000.00");
    let year = banks.insights(insights_today()).years.remove(0);
    let segments: Vec<String> = year
        .segments
        .iter()
        .map(|s| s.balance.to_string())
        .collect();
    assert_eq!(segments, ["5000.00", "-1000.00"]);
    assert_eq!(year.closing_balance.to_string(), "4000.00");
}

#[test]
fn test_csd010_ac_5_4_segments_and_banks_follow_creation_order_not_names() {
    let mut banks = BankAccounts::default();
    let savings = account(&mut banks, "Example Savings", "Monthly");
    let current = account(&mut banks, "Example Current Account", "Monthly");
    add(&mut banks, savings, "2026-09", "1.00");
    add(&mut banks, current, "2026-09", "2.00");
    let insights = banks.insights(insights_today());
    assert_eq!(insights.years[0].segments[0].account_id, savings);
    let names: Vec<&str> = insights.banks.iter().map(|b| b.name.as_str()).collect();
    assert_eq!(names, ["Example Savings", "Example Current Account"]);
}

#[test]
fn test_csd010_ac_5_5_the_insights_equal_the_yearly_totals() {
    let mut banks = BankAccounts::default();
    let current = account(&mut banks, "Example Current Account", "Monthly");
    let savings = account(&mut banks, "Example Savings", "Yearly");
    add(&mut banks, current, "2025-11", "2000.00");
    add(&mut banks, current, "2026-03", "2500.00");
    add(&mut banks, savings, "2025", "8000.00");
    let totals = banks.yearly_totals();
    for year in banks.insights(insights_today()).years {
        let total = totals.iter().find(|t| t.year == year.year).unwrap();
        assert_eq!(year.closing_balance, total.closing_balance);
        assert_eq!(year.entry_count, total.entry_count);
    }
}

#[test]
fn test_csd010_ac_9_2_a_bank_without_entries_is_not_in_the_legend() {
    let mut banks = BankAccounts::default();
    account(&mut banks, "Example Empty", "Monthly");
    let savings = account(&mut banks, "Example Savings", "Monthly");
    add(&mut banks, savings, "2026-09", "1.00");
    let insights = banks.insights(insights_today());
    assert_eq!(insights.banks.len(), 1);
    assert_eq!(insights.banks[0].id, savings);
}

#[test]
fn test_csd010_ac_15_3_a_moved_entry_follows_to_its_new_year() {
    let mut banks = BankAccounts::default();
    let id = account(&mut banks, "Example Current Account", "Monthly");
    add(&mut banks, id, "2026-01", "100.00");
    update(&mut banks, id, "2026-01", "2025-12", "100.00").unwrap();
    let years = banks.insights(insights_today()).years;
    assert_eq!(
        years.iter().map(|y| y.year).collect::<Vec<_>>(),
        [2025, 2026]
    );
    assert_eq!(years[0].segments.len(), 1);
    assert_eq!(years[1].segments.len(), 0);
}
