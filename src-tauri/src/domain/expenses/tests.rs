use time::macros::date;
use time::Date;

use super::*;

fn today() -> Date {
    date!(2026 - 10 - 29)
}

fn input(name: &str, amount: &str, date: &str, category: &str) -> ExpenseInput {
    ExpenseInput {
        name: name.to_owned(),
        amount: amount.to_owned(),
        date: date.to_owned(),
        category: category.to_owned(),
        note: None,
    }
}

fn add(expenses: &mut Expenses, name: &str, amount: &str, date: &str, category: &str) -> Expense {
    expenses
        .add_expense(&input(name, amount, date, category), today())
        .unwrap()
}

fn code(result: Result<Expense, DomainError>) -> &'static str {
    result.err().map_or("OK", |error| error.code())
}

fn dec(text: &str) -> rust_decimal::Decimal {
    text.parse().unwrap()
}

fn names(list: &ExpenseList) -> Vec<&str> {
    list.expenses.iter().map(|e| e.name.as_str()).collect()
}

fn with_note(mut input: ExpenseInput, note: &str) -> ExpenseInput {
    input.note = Some(note.to_owned());
    input
}

// REQ-1: add an expense

#[test]
fn test_csd013_ac_1_1_expense_with_a_note() {
    let mut expenses = Expenses::default();
    let added = expenses
        .add_expense(
            &with_note(
                input("Example Roof Repair", "4500.00", "15/09/2026", "Home"),
                "Replaced tiles",
            ),
            today(),
        )
        .unwrap();
    assert_eq!(added.name, "Example Roof Repair");
    assert_eq!(added.amount.value(), dec("4500.00"));
    assert_eq!(added.date.year(), 2026);
    assert_eq!(added.date.month(), 9);
    assert_eq!(added.category, Category::Home);
    assert_eq!(added.note.as_str(), "Replaced tiles");
}

#[test]
fn test_csd013_ac_1_2_expense_without_a_note_has_an_empty_note() {
    let mut expenses = Expenses::default();
    let added = add(
        &mut expenses,
        "Example Washing Machine",
        "650.00",
        "02/10/2026",
        "Appliances",
    );
    assert_eq!(added.note.as_str(), "");
}

#[test]
fn test_csd013_ac_1_4_two_expenses_on_one_day_with_one_name() {
    let mut expenses = Expenses::default();
    add(
        &mut expenses,
        "Example Bike",
        "900.00",
        "10/10/2026",
        "Hobby",
    );
    add(
        &mut expenses,
        "Example Bike",
        "900.00",
        "10/10/2026",
        "Hobby",
    );
    assert_eq!(expenses.total_count(), 2);
}

// REQ-2: name rules

#[test]
fn test_csd013_ac_2_1_invalid_names_are_rejected() {
    for name in ["", "   "] {
        let mut expenses = Expenses::default();
        let result = expenses.add_expense(&input(name, "100.00", "15/09/2026", "Other"), today());
        assert_eq!(code(result), "EXPENSE_NAME_REQUIRED", "{name:?}");
        assert_eq!(expenses.total_count(), 0);
    }
}

#[test]
fn test_csd013_ac_2_2_name_length_boundary() {
    let mut expenses = Expenses::default();
    let ok = expenses.add_expense(
        &input(&"a".repeat(100), "1.00", "15/09/2026", "Other"),
        today(),
    );
    assert_eq!(code(ok), "OK");
    let long = expenses.add_expense(
        &input(&"a".repeat(101), "1.00", "15/09/2026", "Other"),
        today(),
    );
    assert_eq!(code(long), "EXPENSE_NAME_TOO_LONG");
    let multibyte = expenses.add_expense(
        &input(&"é".repeat(100), "1.00", "15/09/2026", "Other"),
        today(),
    );
    assert_eq!(code(multibyte), "OK");
}

#[test]
fn test_csd013_ac_2_3_the_name_is_stored_trimmed() {
    let mut expenses = Expenses::default();
    let added = add(
        &mut expenses,
        "  Example Roof Repair  ",
        "1.00",
        "15/09/2026",
        "Home",
    );
    assert_eq!(added.name, "Example Roof Repair");
}

// REQ-3: category

#[test]
fn test_csd013_ac_3_1_every_category_is_accepted() {
    for name in [
        "Home",
        "Vehicle",
        "Holiday",
        "Sport",
        "Study",
        "Appliances",
        "Hobby",
        "Other",
    ] {
        let mut expenses = Expenses::default();
        let added = add(&mut expenses, "Example", "1.00", "15/09/2026", name);
        assert_eq!(added.category.name(), name);
    }
}

#[test]
fn test_csd013_ac_3_2_and_3_3_unknown_categories_are_rejected() {
    for category in ["Health", "home", "HOME", "", "Travel"] {
        let mut expenses = Expenses::default();
        let result =
            expenses.add_expense(&input("Example", "1.00", "15/09/2026", category), today());
        assert_eq!(code(result), "EXPENSE_CATEGORY_UNSUPPORTED", "{category:?}");
        assert_eq!(expenses.total_count(), 0);
    }
}

// REQ-4: date rules

#[test]
fn test_csd013_ac_4_1_boundary_dates() {
    for (text, expected) in [
        ("29/10/2026", "OK"),
        ("30/10/2026", "EXPENSE_DATE_IN_FUTURE"),
        ("01/01/1900", "OK"),
        ("31/12/1899", "EXPENSE_DATE_INVALID"),
    ] {
        let mut expenses = Expenses::default();
        let result = expenses.add_expense(&input("Example", "1.00", text, "Other"), today());
        assert_eq!(code(result), expected, "{text}");
    }
}

#[test]
fn test_csd013_ac_4_2_invalid_dates_are_rejected() {
    for text in [
        "",
        "31/04/2026",
        "29/02/2026",
        "2026-09-15",
        "15-09-2026",
        "abc",
        "5/9/2026",
        "15/09/26",
    ] {
        let mut expenses = Expenses::default();
        let result = expenses.add_expense(&input("Example", "1.00", text, "Other"), today());
        assert_eq!(code(result), "EXPENSE_DATE_INVALID", "{text:?}");
        assert_eq!(expenses.total_count(), 0);
    }
}

#[test]
fn test_csd013_ac_4_3_a_leap_day_is_a_real_date() {
    let mut expenses = Expenses::default();
    let added = add(&mut expenses, "Example", "1.00", "29/02/2024", "Other");
    assert_eq!((added.date.year(), added.date.month()), (2024, 2));
}

// REQ-5: amount validation

#[test]
fn test_csd013_ac_5_1_invalid_amounts_are_rejected() {
    for (amount, expected) in [
        ("", "EXPENSE_AMOUNT_INVALID"),
        ("abc", "EXPENSE_AMOUNT_INVALID"),
        ("1e3", "EXPENSE_AMOUNT_INVALID"),
        ("2500,50", "EXPENSE_AMOUNT_INVALID"),
        ("0.00", "EXPENSE_AMOUNT_NOT_POSITIVE"),
        ("-0.01", "EXPENSE_AMOUNT_NOT_POSITIVE"),
        ("100.999", "EXPENSE_AMOUNT_PRECISION"),
        ("1000000000.01", "EXPENSE_AMOUNT_TOO_LARGE"),
    ] {
        let mut expenses = Expenses::default();
        let result =
            expenses.add_expense(&input("Example", amount, "15/09/2026", "Other"), today());
        assert_eq!(code(result), expected, "{amount:?}");
        assert_eq!(expenses.total_count(), 0);
    }
}

#[test]
fn test_csd013_ac_5_2_boundary_amounts_are_accepted() {
    for amount in ["0.01", "1000000000.00"] {
        let mut expenses = Expenses::default();
        let added = add(&mut expenses, "Example", amount, "15/09/2026", "Other");
        assert_eq!(added.amount.value(), dec(amount));
    }
}

#[test]
fn test_csd013_ac_5_2_an_amount_is_kept_with_two_decimals() {
    let mut expenses = Expenses::default();
    let added = add(&mut expenses, "Example", "12.5", "15/09/2026", "Other");
    assert_eq!(added.amount.value().to_string(), "12.50");
}

#[test]
fn test_csd013_ac_5_3_an_invalid_amount_in_an_edit_changes_nothing() {
    let mut expenses = Expenses::default();
    let id = add(&mut expenses, "Example", "4500.00", "15/09/2026", "Home").id;
    let result =
        expenses.update_expense(id, &input("Example", "0.00", "15/09/2026", "Home"), today());
    assert_eq!(code(result), "EXPENSE_AMOUNT_NOT_POSITIVE");
    assert_eq!(
        expenses.list(None, None).unwrap().expenses[0]
            .amount
            .value(),
        dec("4500.00")
    );
}

// REQ-6: note validation

#[test]
fn test_csd013_ac_6_1_note_length_boundary() {
    let mut expenses = Expenses::default();
    let ok = expenses.add_expense(
        &with_note(input("A", "1.00", "15/09/2026", "Other"), &"x".repeat(2000)),
        today(),
    );
    assert_eq!(code(ok), "OK");
    let long = expenses.add_expense(
        &with_note(input("B", "1.00", "16/09/2026", "Other"), &"x".repeat(2001)),
        today(),
    );
    assert_eq!(code(long), "EXPENSE_NOTE_TOO_LONG");
    assert_eq!(expenses.total_count(), 1);
}

#[test]
fn test_csd013_ac_6_2_line_breaks_are_kept() {
    let mut expenses = Expenses::default();
    let added = expenses
        .add_expense(
            &with_note(
                input("A", "1.00", "15/09/2026", "Other"),
                "line one\nline two",
            ),
            today(),
        )
        .unwrap();
    assert_eq!(added.note.as_str().lines().count(), 2);
}

// REQ-7: edit an expense

#[test]
fn test_csd013_ac_7_1_edit_every_field() {
    let mut expenses = Expenses::default();
    let id = add(
        &mut expenses,
        "Example Bike",
        "900.00",
        "10/09/2026",
        "Hobby",
    )
    .id;
    let edited = expenses
        .update_expense(
            id,
            &with_note(
                input("Example E-Bike", "1200.00", "12/09/2026", "Vehicle"),
                "Upgraded",
            ),
            today(),
        )
        .unwrap();
    assert_eq!(edited.id, id);
    assert_eq!(edited.name, "Example E-Bike");
    assert_eq!(edited.amount.value(), dec("1200.00"));
    assert_eq!(edited.date.month(), 9);
    assert_eq!(edited.category, Category::Vehicle);
    assert_eq!(edited.note.as_str(), "Upgraded");
    assert_eq!(expenses.total_count(), 1);
}

#[test]
fn test_csd013_ac_7_2_a_note_can_be_cleared() {
    let mut expenses = Expenses::default();
    let id = expenses
        .add_expense(
            &with_note(input("A", "1.00", "15/09/2026", "Other"), "First"),
            today(),
        )
        .unwrap()
        .id;
    let edited = expenses
        .update_expense(id, &input("A", "1.00", "15/09/2026", "Other"), today())
        .unwrap();
    assert_eq!(edited.note.as_str(), "");
}

#[test]
fn test_csd013_ac_7_3_moving_an_expense_to_another_year() {
    let mut expenses = Expenses::default();
    let id = add(
        &mut expenses,
        "Example Bike",
        "900.00",
        "10/01/2026",
        "Hobby",
    )
    .id;
    expenses
        .update_expense(
            id,
            &input("Example Bike", "900.00", "20/12/2025", "Hobby"),
            today(),
        )
        .unwrap();
    let totals = expenses.yearly_totals();
    assert_eq!(totals.len(), 1);
    assert_eq!(totals[0].year, 2025);
    assert_eq!(totals[0].total, dec("900.00"));
    assert_eq!(totals[0].expense_count, 1);
}

#[test]
fn test_csd013_ac_7_4_an_invalid_value_rejects_the_whole_edit() {
    let mut expenses = Expenses::default();
    let id = add(
        &mut expenses,
        "Example Bike",
        "900.00",
        "10/09/2026",
        "Hobby",
    )
    .id;
    let before = expenses.clone();
    let result = expenses.update_expense(
        id,
        &input("Example Bike", "-1.00", "11/09/2026", "Hobby"),
        today(),
    );
    assert_eq!(code(result), "EXPENSE_AMOUNT_NOT_POSITIVE");
    assert_eq!(expenses, before);
}

#[test]
fn test_csd013_ac_7_5_a_date_in_the_future_is_rejected_in_an_edit() {
    let mut expenses = Expenses::default();
    let id = add(
        &mut expenses,
        "Example Bike",
        "900.00",
        "10/09/2026",
        "Hobby",
    )
    .id;
    let before = expenses.clone();
    let result = expenses.update_expense(
        id,
        &input("Example Bike", "900.00", "30/10/2026", "Hobby"),
        today(),
    );
    assert_eq!(code(result), "EXPENSE_DATE_IN_FUTURE");
    assert_eq!(expenses, before);
}

#[test]
fn test_csd013_ac_7_7_a_stale_screen_is_not_found() {
    let mut expenses = Expenses::default();
    let id = add(
        &mut expenses,
        "Example Bike",
        "900.00",
        "10/09/2026",
        "Hobby",
    )
    .id;
    expenses.delete_expense(id).unwrap();
    let result = expenses.update_expense(
        id,
        &input("Example Bike", "900.00", "10/09/2026", "Hobby"),
        today(),
    );
    assert_eq!(code(result), "EXPENSE_NOT_FOUND");
    assert_eq!(
        expenses.delete_expense(id).unwrap_err().code(),
        "EXPENSE_NOT_FOUND"
    );
    assert_eq!(
        ExpenseId::parse("nope").unwrap_err().code(),
        "EXPENSE_NOT_FOUND"
    );
}

// REQ-8: delete an expense

#[test]
fn test_csd013_ac_8_1_and_8_3_deleting_removes_only_that_expense() {
    let mut expenses = Expenses::default();
    let bike = add(
        &mut expenses,
        "Example Bike",
        "900.00",
        "10/09/2025",
        "Hobby",
    )
    .id;
    add(
        &mut expenses,
        "Example Roof Repair",
        "4500.00",
        "15/09/2026",
        "Home",
    );
    expenses.delete_expense(bike).unwrap();
    let list = expenses.list(None, None).unwrap();
    assert_eq!(names(&list), ["Example Roof Repair"]);
    assert_eq!(
        expenses
            .yearly_totals()
            .iter()
            .map(|t| t.year)
            .collect::<Vec<_>>(),
        [2026]
    );
}

// REQ-9: copy the previous expense

#[test]
fn test_csd013_ac_9_1_the_form_is_filled_from_the_previous_expense() {
    let mut expenses = Expenses::default();
    expenses
        .add_expense(
            &with_note(
                input("Example Bike", "900.00", "10/09/2026", "Hobby"),
                "First",
            ),
            today(),
        )
        .unwrap();
    add(
        &mut expenses,
        "Example Roof Repair",
        "4500.00",
        "15/09/2026",
        "Home",
    );
    let prefill = expenses.prefill_from_previous().unwrap();
    assert_eq!(prefill.name, "Example Roof Repair");
    assert_eq!(prefill.amount, dec("4500.00"));
    assert_eq!(prefill.category, Category::Home);
    assert_eq!(expenses.total_count(), 2);
}

#[test]
fn test_csd013_ac_9_2_on_the_same_date_the_one_added_last_is_copied() {
    let mut expenses = Expenses::default();
    add(
        &mut expenses,
        "Example Bike",
        "900.00",
        "10/09/2026",
        "Hobby",
    );
    add(
        &mut expenses,
        "Example Helmet",
        "80.00",
        "10/09/2026",
        "Sport",
    );
    assert_eq!(
        expenses.prefill_from_previous().unwrap().name,
        "Example Helmet"
    );
}

#[test]
fn test_csd013_ac_9_3_no_expenses_gives_no_prefill() {
    assert_eq!(Expenses::default().prefill_from_previous(), None);
}

// REQ-10: ordering

#[test]
fn test_csd013_ac_10_1_newest_first() {
    let mut expenses = Expenses::default();
    add(&mut expenses, "A", "1.00", "10/09/2026", "Other");
    add(&mut expenses, "B", "1.00", "15/10/2026", "Other");
    add(&mut expenses, "C", "1.00", "01/01/2026", "Other");
    assert_eq!(names(&expenses.list(None, None).unwrap()), ["B", "A", "C"]);
}

#[test]
fn test_csd013_ac_10_2_same_date_is_ordered_by_name_ignoring_case() {
    let mut expenses = Expenses::default();
    add(&mut expenses, "example sofa", "1.00", "10/09/2026", "Other");
    add(&mut expenses, "Example Bike", "1.00", "10/09/2026", "Other");
    assert_eq!(
        names(&expenses.list(None, None).unwrap()),
        ["Example Bike", "example sofa"]
    );
}

#[test]
fn test_csd013_ac_10_3_equal_expenses_keep_the_order_they_were_added() {
    let mut expenses = Expenses::default();
    let first = add(&mut expenses, "Example Bike", "1.00", "10/09/2026", "Other").id;
    let second = add(&mut expenses, "Example Bike", "2.00", "10/09/2026", "Other").id;
    let list = expenses.list(None, None).unwrap();
    assert_eq!(list.expenses[0].id, first);
    assert_eq!(list.expenses[1].id, second);
}

// REQ-11: filter by year and month

#[test]
fn test_csd013_ac_11_2_filter_by_month() {
    let mut expenses = Expenses::default();
    add(
        &mut expenses,
        "Example Bike",
        "900.00",
        "10/09/2026",
        "Hobby",
    );
    add(
        &mut expenses,
        "Example Helmet",
        "80.00",
        "12/09/2026",
        "Sport",
    );
    add(
        &mut expenses,
        "Example Sofa",
        "1200.00",
        "03/10/2026",
        "Home",
    );
    let list = expenses.list(Some(2026), Some(9)).unwrap();
    assert_eq!(names(&list), ["Example Helmet", "Example Bike"]);
    assert_eq!(list.total, dec("980.00"));
}

#[test]
fn test_csd013_ac_11_3_all_years_and_a_month_without_a_year_is_ignored() {
    let mut expenses = Expenses::default();
    add(&mut expenses, "A", "900.00", "10/09/2025", "Hobby");
    add(&mut expenses, "B", "1200.00", "03/10/2026", "Home");
    assert_eq!(expenses.list(None, None).unwrap().total, dec("2100.00"));
    assert_eq!(expenses.list(None, Some(9)).unwrap().total, dec("2100.00"));
    assert_eq!(
        expenses.list(Some(2025), None).unwrap().total,
        dec("900.00")
    );
}

#[test]
fn test_csd013_ac_11_4_only_years_with_expenses_are_offered() {
    let mut expenses = Expenses::default();
    add(&mut expenses, "A", "1.00", "15/09/2024", "Other");
    add(&mut expenses, "B", "1.00", "10/03/2026", "Other");
    assert_eq!(expenses.list(Some(2026), None).unwrap().years, [2026, 2024]);
}

#[test]
fn test_csd013_ac_11_5_a_month_without_expenses_is_empty_with_a_zero_total() {
    let mut expenses = Expenses::default();
    add(&mut expenses, "A", "1.00", "10/09/2026", "Other");
    let list = expenses.list(Some(2026), Some(10)).unwrap();
    assert_eq!(list.expenses.len(), 0);
    assert_eq!(list.total.to_string(), "0.00");
}

#[test]
fn test_csd013_ac_11_6_the_month_is_the_month_of_the_date() {
    let mut expenses = Expenses::default();
    add(&mut expenses, "End", "1.00", "31/10/2025", "Other");
    add(&mut expenses, "Start", "1.00", "01/11/2025", "Other");
    assert_eq!(
        names(&expenses.list(Some(2025), Some(10)).unwrap()),
        ["End"]
    );
}

#[test]
fn test_csd013_ac_11_9_no_expenses() {
    let list = Expenses::default().list(None, None).unwrap();
    assert!(list.expenses.is_empty() && list.years.is_empty());
    assert_eq!(list.total.to_string(), "0.00");
}

#[test]
fn test_csd013_ac_11_a_month_outside_1_to_12_is_rejected() {
    let expenses = Expenses::default();
    for month in [0, 13] {
        assert_eq!(
            expenses.list(Some(2026), Some(month)).unwrap_err().code(),
            "EXPENSE_DATE_INVALID"
        );
    }
}

// REQ-12: yearly totals

#[test]
fn test_csd013_ac_12_1_totals_per_calendar_year() {
    let mut expenses = Expenses::default();
    add(&mut expenses, "A", "900.00", "10/09/2025", "Hobby");
    add(&mut expenses, "B", "80.00", "12/09/2025", "Sport");
    add(&mut expenses, "C", "1200.00", "03/10/2026", "Home");
    let totals = expenses.yearly_totals();
    assert_eq!(totals.len(), 2);
    assert_eq!(
        (
            totals[0].year,
            totals[0].total,
            totals[0].expense_count,
            totals[0].largest
        ),
        (2026, dec("1200.00"), 1, dec("1200.00"))
    );
    assert_eq!(
        (
            totals[1].year,
            totals[1].total,
            totals[1].expense_count,
            totals[1].largest
        ),
        (2025, dec("980.00"), 2, dec("900.00"))
    );
}

#[test]
fn test_csd013_ac_12_2_no_expenses_gives_no_totals() {
    assert_eq!(Expenses::default().yearly_totals().len(), 0);
}

#[test]
fn test_csd013_ac_12_4_a_gap_year_is_not_listed() {
    let mut expenses = Expenses::default();
    add(&mut expenses, "A", "1.00", "15/09/2024", "Other");
    add(&mut expenses, "B", "1.00", "10/03/2026", "Other");
    let years: Vec<i32> = expenses.yearly_totals().iter().map(|t| t.year).collect();
    assert_eq!(years, [2026, 2024]);
}

#[test]
fn test_csd013_ac_12_5_largest_is_over_all_categories() {
    let mut expenses = Expenses::default();
    add(&mut expenses, "A", "900.00", "10/09/2026", "Hobby");
    add(&mut expenses, "B", "4500.00", "11/09/2026", "Home");
    assert_eq!(expenses.yearly_totals()[0].largest, dec("4500.00"));
}

#[test]
fn test_csd013_ac_12_6_totals_follow_changes() {
    let mut expenses = Expenses::default();
    add(&mut expenses, "A", "900.00", "10/09/2026", "Hobby");
    add(&mut expenses, "B", "1200.00", "03/10/2026", "Home");
    let row = &expenses.yearly_totals()[0];
    assert_eq!(
        (row.total, row.expense_count, row.largest),
        (dec("2100.00"), 2, dec("1200.00"))
    );
}

#[test]
fn test_csd013_ac_12_7_the_first_and_the_current_year() {
    let mut expenses = Expenses::default();
    add(&mut expenses, "A", "1.00", "01/01/1900", "Other");
    add(&mut expenses, "B", "1.00", "29/10/2026", "Other");
    let years: Vec<i32> = expenses.yearly_totals().iter().map(|t| t.year).collect();
    assert_eq!(years, [2026, 1900]);
}

// REQ-16: the project file section

#[test]
fn test_csd013_ac_16_1_serialisation_round_trip_keeps_everything() {
    let mut expenses = Expenses::default();
    expenses
        .add_expense(
            &with_note(
                input("Example Roof Repair", "4500.00", "15/09/2026", "Home"),
                "Line one\nLine two",
            ),
            today(),
        )
        .unwrap();
    let json = serde_json::to_value(&expenses).unwrap();
    assert_eq!(json["expenses"][0]["date"], "2026-09-15");
    assert_eq!(json["expenses"][0]["amount"], "4500.00");
    assert_eq!(json["expenses"][0]["category"], "Home");
    let back: Expenses = serde_json::from_value(json).unwrap();
    assert_eq!(back, expenses);
    back.validate().unwrap();
}

#[test]
fn test_csd013_ac_16_2_a_missing_section_is_empty() {
    let back: Expenses = serde_json::from_str("{}").unwrap();
    assert_eq!(back, Expenses::default());
}

#[test]
fn test_csd013_ac_16_4_damaged_expenses_are_rejected() {
    let id = "671166f4-5827-4794-bcdc-54d7824fb650";
    let expense = |amount: &str, date: &str, category: &str| {
        format!(
            r#"{{"expenses":[{{"id":"{id}","name":"Example","amount":"{amount}","date":"{date}","category":"{category}"}}]}}"#
        )
    };
    for (damage, text) in [
        ("category", expense("1.00", "2026-09-15", "Health")),
        ("date", expense("1.00", "2026-02-30", "Home")),
        ("old date", expense("1.00", "1899-12-31", "Home")),
        ("zero amount", expense("0.00", "2026-09-15", "Home")),
        ("three decimals", expense("1.001", "2026-09-15", "Home")),
    ] {
        let parsed = serde_json::from_str::<Expenses>(&text);
        assert!(
            parsed.is_err() || parsed.unwrap().validate().is_err(),
            "{damage}"
        );
    }
    let two = format!(
        r#"{{"expenses":[{{"id":"{id}","name":"A","amount":"1.00","date":"2026-09-15","category":"Home"}},{{"id":"{id}","name":"B","amount":"1.00","date":"2026-09-15","category":"Home"}}]}}"#
    );
    let parsed: Expenses = serde_json::from_str(&two).unwrap();
    assert_eq!(
        parsed.validate().unwrap_err().code(),
        "EXPENSE_DATA_INVALID"
    );
}

#[test]
fn test_csd013_ac_16_5_the_order_they_were_added_survives() {
    let mut expenses = Expenses::default();
    let first = add(&mut expenses, "Example Bike", "1.00", "10/09/2026", "Other").id;
    add(&mut expenses, "Example Bike", "2.00", "10/09/2026", "Other");
    let back: Expenses = serde_json::from_str(&serde_json::to_string(&expenses).unwrap()).unwrap();
    assert_eq!(back.list(None, None).unwrap().expenses[0].id, first);
}

// CSD-014: insights

fn insights(expenses: &Expenses) -> ExpenseInsights {
    expenses.insights(2026)
}

#[test]
fn test_csd014_ac_1_1_the_series_runs_from_the_first_expense_to_the_current_year() {
    let mut expenses = Expenses::default();
    add(&mut expenses, "A", "1.00", "15/03/2024", "Other");
    add(&mut expenses, "B", "1.00", "15/03/2025", "Other");
    add(&mut expenses, "C", "1.00", "15/03/2026", "Other");
    let years: Vec<i32> = insights(&expenses).years.iter().map(|y| y.year).collect();
    assert_eq!(years, [2024, 2025, 2026]);
}

#[test]
fn test_csd014_ac_1_2_a_single_year() {
    let mut expenses = Expenses::default();
    add(&mut expenses, "A", "1.00", "15/09/2026", "Other");
    assert_eq!(insights(&expenses).years.len(), 1);
}

#[test]
fn test_csd014_ac_1_3_no_expenses_gives_an_empty_series_and_all_categories() {
    let result = insights(&Expenses::default());
    assert_eq!(result.years.len(), 0);
    assert_eq!(result.categories.len(), 8);
}

#[test]
fn test_csd014_ac_1_4_an_old_expense_extends_the_series() {
    let mut expenses = Expenses::default();
    add(&mut expenses, "A", "1.00", "01/01/2022", "Other");
    let years: Vec<i32> = insights(&expenses).years.iter().map(|y| y.year).collect();
    assert_eq!(years, [2022, 2023, 2024, 2025, 2026]);
}

#[test]
fn test_csd014_ac_2_1_a_gap_year_is_zero_without_segments() {
    let mut expenses = Expenses::default();
    add(&mut expenses, "A", "1.00", "15/03/2022", "Other");
    add(&mut expenses, "B", "1.00", "15/03/2024", "Other");
    let result = insights(&expenses);
    let gap = &result.years[1];
    assert_eq!(gap.year, 2023);
    assert_eq!(gap.total.to_string(), "0.00");
    assert_eq!(gap.expense_count, 0);
    assert_eq!(gap.segments.len(), 0);
}

#[test]
fn test_csd014_ac_3_1_and_3_2_the_current_year_is_partial() {
    let mut expenses = Expenses::default();
    add(&mut expenses, "A", "1.00", "15/12/2025", "Other");
    let result = insights(&expenses);
    assert_eq!(result.years.len(), 2);
    assert!(!result.years[0].is_partial);
    let current = &result.years[1];
    assert!(current.is_partial);
    assert_eq!(
        (current.total.to_string(), current.expense_count),
        ("0.00".to_owned(), 0)
    );
}

#[test]
fn test_csd014_ac_4_1_segments_are_the_sums_per_category() {
    let mut expenses = Expenses::default();
    add(&mut expenses, "A", "4500.00", "15/03/2026", "Home");
    add(&mut expenses, "B", "1200.00", "02/06/2026", "Home");
    add(&mut expenses, "C", "900.00", "10/09/2026", "Hobby");
    let year = &insights(&expenses).years[0];
    assert_eq!(year.segments.len(), 2);
    assert_eq!(
        (year.segments[0].category, year.segments[0].amount),
        (Category::Home, dec("5700.00"))
    );
    assert_eq!(
        (year.segments[1].category, year.segments[1].amount),
        (Category::Hobby, dec("900.00"))
    );
    assert_eq!((year.total, year.expense_count), (dec("6600.00"), 3));
}

#[test]
fn test_csd014_ac_4_2_a_category_without_an_expense_has_no_segment() {
    let mut expenses = Expenses::default();
    add(&mut expenses, "A", "900.00", "10/09/2025", "Hobby");
    add(&mut expenses, "B", "650.00", "02/10/2026", "Appliances");
    let result = insights(&expenses);
    assert_eq!(result.years[0].segments.len(), 1);
    assert_eq!(result.years[0].segments[0].category, Category::Hobby);
    assert_eq!(result.years[1].segments.len(), 1);
    assert_eq!(result.years[1].segments[0].category, Category::Appliances);
}

#[test]
fn test_csd014_ac_4_3_segments_follow_the_fixed_order_not_the_order_added() {
    let mut expenses = Expenses::default();
    add(&mut expenses, "A", "900.00", "10/09/2026", "Hobby");
    add(&mut expenses, "B", "4500.00", "11/09/2026", "Home");
    let order: Vec<Category> = insights(&expenses).years[0]
        .segments
        .iter()
        .map(|s| s.category)
        .collect();
    assert_eq!(order, [Category::Home, Category::Hobby]);
}

#[test]
fn test_csd014_ac_4_4_the_insights_equal_the_yearly_totals() {
    let mut expenses = Expenses::default();
    add(&mut expenses, "A", "900.00", "10/09/2025", "Hobby");
    add(&mut expenses, "B", "80.00", "11/09/2025", "Sport");
    add(&mut expenses, "C", "1200.00", "03/10/2026", "Appliances");
    let result = insights(&expenses);
    for total in expenses.yearly_totals() {
        let year = result.years.iter().find(|y| y.year == total.year).unwrap();
        assert_eq!(
            (year.total, year.expense_count),
            (total.total, total.expense_count)
        );
    }
}

#[test]
fn test_csd014_ac_4_5_all_eight_categories_in_the_fixed_order() {
    let mut expenses = Expenses::default();
    for category in [
        "Other",
        "Hobby",
        "Appliances",
        "Study",
        "Sport",
        "Holiday",
        "Vehicle",
        "Home",
    ] {
        add(&mut expenses, "Example", "100.00", "15/03/2026", category);
    }
    let year = &insights(&expenses).years[0];
    let order: Vec<&str> = year.segments.iter().map(|s| s.category.name()).collect();
    assert_eq!(
        order,
        [
            "Home",
            "Vehicle",
            "Holiday",
            "Sport",
            "Study",
            "Appliances",
            "Hobby",
            "Other"
        ]
    );
    assert!(year.segments.iter().all(|s| s.amount == dec("100.00")));
    assert_eq!((year.total, year.expense_count), (dec("800.00"), 8));
}

#[test]
fn test_csd014_ac_4_6_cents_add_up_exactly() {
    let mut expenses = Expenses::default();
    add(&mut expenses, "A", "0.10", "15/03/2026", "Other");
    add(&mut expenses, "B", "0.20", "15/03/2026", "Other");
    assert_eq!(
        insights(&expenses).years[0].segments[0].amount.to_string(),
        "0.30"
    );
}

#[test]
fn test_csd014_ac_4_7_an_edit_moves_the_segment() {
    let mut expenses = Expenses::default();
    let id = add(&mut expenses, "A", "900.00", "10/09/2026", "Hobby").id;
    expenses
        .update_expense(id, &input("A", "900.00", "10/09/2026", "Sport"), today())
        .unwrap();
    let year = &insights(&expenses).years[0];
    assert_eq!(year.segments.len(), 1);
    assert_eq!(year.segments[0].category, Category::Sport);
}

#[test]
fn test_csd014_ac_6_2_colour_slots_are_fixed_by_the_category_list() {
    let slots: Vec<(&str, u8)> = insights(&Expenses::default())
        .categories
        .iter()
        .map(|c| (c.name.name(), c.colour_slot))
        .collect();
    assert_eq!(
        slots,
        [
            ("Home", 0),
            ("Vehicle", 1),
            ("Holiday", 2),
            ("Sport", 3),
            ("Study", 4),
            ("Appliances", 5),
            ("Hobby", 6),
            ("Other", 7)
        ]
    );
}

#[test]
fn test_csd014_expenses_after_the_current_year_are_not_shown() {
    let mut expenses = Expenses::default();
    add(&mut expenses, "A", "1.00", "15/03/2026", "Other");
    let mut value = serde_json::to_value(&expenses).unwrap();
    value["expenses"][0]["date"] = "2027-01-01".into();
    let future: Expenses = serde_json::from_value(value).unwrap();
    // The file is valid; the chart simply stops at the current year.
    assert!(future.validate().is_ok());
    assert!(insights(&future).years.iter().all(|y| y.expense_count == 0));
}
