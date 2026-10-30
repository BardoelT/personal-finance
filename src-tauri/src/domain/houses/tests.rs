use super::house::COLOUR_SLOTS;
use super::*;
use crate::domain::income::PayPeriod;

fn period(text: &str) -> Period {
    parse_period(text).unwrap()
}

fn today() -> YearMonth {
    YearMonth::parse("2026-10").unwrap()
}

fn value(text: &str) -> HouseValue {
    HouseValue::parse(text).unwrap()
}

fn input(name: &str, period: &str) -> HouseInput {
    HouseInput {
        name: name.to_owned(),
        valuation_period: Some(period.to_owned()),
        purchase_date: Some("01/01/1900".to_owned()),
        purchase_amount: Some("0.00".to_owned()),
        ..HouseInput::default()
    }
}

fn today_date() -> Date {
    time::macros::date!(2026 - 10 - 29)
}

fn new_house(houses: &mut Houses, name: &str, period: &str) -> HouseId {
    houses
        .create_house(&input(name, period), today_date())
        .unwrap()
        .id
}

fn add(houses: &mut Houses, id: HouseId, at: &str, amount: &str) {
    houses
        .add_valuation(id, period(at), value(amount), Note::default(), today())
        .unwrap();
}

fn add_error(houses: &mut Houses, id: HouseId, at: &str, amount: &str) -> DomainError {
    match parse_period(at)
        .and_then(|p| HouseValue::parse(amount).map(|b| (p, b)))
        .and_then(|(p, b)| houses.add_valuation(id, p, b, Note::default(), today()))
    {
        Ok(_) => panic!("expected an error"),
        Err(error) => error,
    }
}

fn code<T>(result: Result<T, DomainError>) -> &'static str {
    result.err().map_or("OK", |error| error.code())
}

fn update(
    houses: &mut Houses,
    id: HouseId,
    from: &str,
    to: &str,
    amount: &str,
) -> Result<HouseValuation, DomainError> {
    houses.update_valuation(
        id,
        period(from),
        period(to),
        value(amount),
        Note::default(),
        today(),
    )
}

/// The change of the valuation of `at`, as text, or "none".
fn change(houses: &Houses, id: HouseId, at: &str) -> String {
    let (_, valuation) = houses
        .all_valuations()
        .into_iter()
        .find(|(d, s)| d.id == id && s.period == period(at))
        .unwrap();
    houses
        .change(valuation)
        .map_or_else(|| "none".to_owned(), |value| value.to_string())
}

// REQ-1, REQ-2

#[test]
fn test_csd015_ac_1_1_a_house_is_monthly_by_default() {
    let mut houses = Houses::default();
    let created = houses
        .create_house(
            &HouseInput {
                name: "Example House".to_owned(),
                valuation_period: None,
                purchase_date: Some("01/01/1900".to_owned()),
                purchase_amount: Some("0.00".to_owned()),
                ..HouseInput::default()
            },
            today_date(),
        )
        .unwrap();
    assert_eq!(created.valuation_period, PayPeriod::Monthly);
    assert_eq!(houses.valuation_count(created.id), 0);
}

#[test]
fn test_csd015_ac_1_2_a_yearly_house() {
    let mut houses = Houses::default();
    new_house(&mut houses, "Example Lodge", "Yearly");
    assert_eq!(houses.houses()[0].valuation_period, PayPeriod::Yearly);
}

#[test]
fn test_csd015_ac_1_3_an_unknown_period_is_rejected() {
    let mut houses = Houses::default();
    let result = houses.create_house(&input("Example House", "Weekly"), today_date());
    assert_eq!(code(result), "HOUSE_PERIOD_UNSUPPORTED");
    assert_eq!(houses.houses().len(), 0);
}

#[test]
fn test_csd015_ac_2_1_invalid_names_are_rejected() {
    let mut houses = Houses::default();
    new_house(&mut houses, "Example House", "Monthly");
    for (name, expected) in [
        ("", "HOUSE_NAME_REQUIRED"),
        ("   ", "HOUSE_NAME_REQUIRED"),
        ("EXAMPLE HOUSE", "HOUSE_NAME_DUPLICATE"),
        ("example house", "HOUSE_NAME_DUPLICATE"),
    ] {
        assert_eq!(
            code(houses.create_house(&input(name, "Monthly"), today_date())),
            expected,
            "{name:?}"
        );
    }
    assert_eq!(houses.houses().len(), 1);
}

#[test]
fn test_csd015_ac_2_2_name_length_boundary() {
    let mut houses = Houses::default();
    assert!(houses
        .create_house(&input(&"a".repeat(100), "Monthly"), today_date())
        .is_ok());
    let result = houses.create_house(&input(&"b".repeat(101), "Monthly"), today_date());
    assert_eq!(code(result), "HOUSE_NAME_TOO_LONG");
}

#[test]
fn test_csd015_ac_2_3_the_name_is_stored_trimmed() {
    let mut houses = Houses::default();
    let created = houses
        .create_house(&input("  Example House  ", "Monthly"), today_date())
        .unwrap();
    assert_eq!(created.name, "Example House");
}

#[test]
fn test_csd015_ac_2_4_renaming_to_an_existing_name_is_rejected() {
    let mut houses = Houses::default();
    new_house(&mut houses, "Example House", "Monthly");
    let land = new_house(&mut houses, "Example Lodge", "Monthly");
    let result = houses.update_house(land, &input("example house", "Monthly"), today_date());
    assert_eq!(code(result), "HOUSE_NAME_DUPLICATE");
}

#[test]
fn test_csd015_ac_2_5_changing_only_the_case_of_its_own_name_is_allowed() {
    let mut houses = Houses::default();
    let land = new_house(&mut houses, "Example Lodge", "Monthly");
    let renamed = houses
        .update_house(land, &input("EXAMPLE LAND", "Monthly"), today_date())
        .unwrap();
    assert_eq!(renamed.name, "EXAMPLE LAND");
}

// REQ-3

#[test]
fn test_csd015_ac_3_1_slots_follow_creation_order() {
    let mut houses = Houses::default();
    let slots: Vec<u8> = ["A", "B", "C"]
        .iter()
        .map(|name| {
            houses
                .create_house(&input(name, "Monthly"), today_date())
                .unwrap()
                .colour_slot
        })
        .collect();
    assert_eq!(slots, [0, 1, 2]);
}

#[test]
fn test_csd015_ac_3_2_slots_cycle_after_eight_houses() {
    let mut houses = Houses::default();
    for index in 0..COLOUR_SLOTS {
        new_house(&mut houses, &format!("House {index}"), "Monthly");
    }
    assert_eq!(
        houses
            .create_house(&input("House 9", "Monthly"), today_date())
            .unwrap()
            .colour_slot,
        0
    );
}

#[test]
fn test_csd015_ac_3_3_deleting_a_house_does_not_recolour_the_others() {
    let mut houses = Houses::default();
    let first = new_house(&mut houses, "A", "Monthly");
    new_house(&mut houses, "B", "Monthly");
    new_house(&mut houses, "C", "Monthly");
    houses.delete_house(first).unwrap();
    let slots: Vec<u8> = houses.houses().iter().map(|d| d.colour_slot).collect();
    assert_eq!(slots, [1, 2]);
}

#[test]
fn test_csd015_ac_3_4_a_freed_slot_is_used_first() {
    let mut houses = Houses::default();
    new_house(&mut houses, "A", "Monthly");
    let second = new_house(&mut houses, "B", "Monthly");
    new_house(&mut houses, "C", "Monthly");
    houses.delete_house(second).unwrap();
    assert_eq!(
        houses
            .create_house(&input("D", "Monthly"), today_date())
            .unwrap()
            .colour_slot,
        1
    );
}

#[test]
fn test_csd015_ac_3_5_renaming_keeps_the_slot() {
    let mut houses = Houses::default();
    new_house(&mut houses, "A", "Monthly");
    new_house(&mut houses, "B", "Monthly");
    let third = new_house(&mut houses, "C", "Monthly");
    let renamed = houses
        .update_house(third, &input("Renamed", "Monthly"), today_date())
        .unwrap();
    assert_eq!(renamed.colour_slot, 2);
}

// REQ-4 to REQ-6

#[test]
fn test_csd015_ac_4_1_rename_keeps_valuations() {
    let mut houses = Houses::default();
    let id = new_house(&mut houses, "Example House", "Monthly");
    add(&mut houses, id, "2026-08", "100.00");
    add(&mut houses, id, "2026-09", "90.00");
    houses
        .update_house(id, &input("Example Home", "Monthly"), today_date())
        .unwrap();
    assert_eq!(houses.valuation_count(id), 2);
}

#[test]
fn test_csd015_ac_4_2_the_period_of_a_house_without_valuations_can_change() {
    let mut houses = Houses::default();
    let id = new_house(&mut houses, "Example House", "Monthly");
    let changed = houses
        .update_house(id, &input("Example House", "Yearly"), today_date())
        .unwrap();
    assert_eq!(changed.valuation_period, PayPeriod::Yearly);
}

#[test]
fn test_csd015_ac_4_3_an_invalid_edit_changes_nothing() {
    let mut houses = Houses::default();
    new_house(&mut houses, "Example House", "Monthly");
    let land = new_house(&mut houses, "Example Lodge", "Monthly");
    assert_eq!(
        code(houses.update_house(land, &input("  ", "Monthly"), today_date())),
        "HOUSE_NAME_REQUIRED"
    );
    assert!(houses.houses().iter().any(|d| d.name == "Example Lodge"));
}

#[test]
fn test_csd015_ac_5_1_the_period_is_locked_once_valuations_exist() {
    let mut houses = Houses::default();
    let id = new_house(&mut houses, "Example House", "Monthly");
    add(&mut houses, id, "2026-09", "100.00");
    let result = houses.update_house(id, &input("Example House", "Yearly"), today_date());
    assert_eq!(code(result), "HOUSE_PERIOD_LOCKED");
    assert_eq!(houses.houses()[0].valuation_period, PayPeriod::Monthly);
}

#[test]
fn test_csd015_ac_5_2_the_period_can_change_after_deleting_the_last_valuation() {
    let mut houses = Houses::default();
    let id = new_house(&mut houses, "Example House", "Monthly");
    add(&mut houses, id, "2026-09", "100.00");
    houses.delete_valuation(id, period("2026-09")).unwrap();
    assert!(houses
        .update_house(id, &input("Example House", "Yearly"), today_date())
        .is_ok());
}

#[test]
fn test_csd015_ac_6_1_deleting_a_house_removes_its_valuations() {
    let mut houses = Houses::default();
    let id = new_house(&mut houses, "Example House", "Monthly");
    let other = new_house(&mut houses, "Example Lodge", "Monthly");
    for month in ["2026-07", "2026-08", "2026-09"] {
        add(&mut houses, id, month, "100.00");
    }
    add(&mut houses, other, "2026-09", "50.00");
    houses.delete_house(id).unwrap();
    assert_eq!(houses.houses().len(), 1);
    assert_eq!(houses.total_valuation_count(), 1);
    assert_eq!(code(houses.delete_house(id)), "HOUSE_NOT_FOUND");
}

#[test]
fn test_csd015_ac_6_3_a_house_at_zero_is_deleted_like_any_other() {
    let mut houses = Houses::default();
    let id = new_house(&mut houses, "Example Lodge", "Monthly");
    add(&mut houses, id, "2026-09", "0.00");
    houses.delete_house(id).unwrap();
    assert_eq!(houses.total_valuation_count(), 0);
}

// REQ-7 to REQ-9

#[test]
fn test_csd015_ac_7_1_a_monthly_valuation_with_a_note() {
    let mut houses = Houses::default();
    let id = new_house(&mut houses, "Example House", "Monthly");
    let valuation = houses
        .add_valuation(
            id,
            period("2026-09"),
            value("150000.00"),
            house_note("Valuation report").unwrap(),
            today(),
        )
        .unwrap();
    assert_eq!(valuation.value.value().to_string(), "150000.00");
    assert_eq!(valuation.note.as_str(), "Valuation report");
}

#[test]
fn test_csd015_ac_7_2_a_yearly_valuation_without_a_note() {
    let mut houses = Houses::default();
    let id = new_house(&mut houses, "Example Lodge", "Yearly");
    add(&mut houses, id, "2025", "8000.00");
    assert_eq!(houses.all_valuations()[0].1.note.as_str(), "");
}

#[test]
fn test_csd015_ac_7_3_old_periods_are_accepted() {
    let mut houses = Houses::default();
    let id = new_house(&mut houses, "Example House", "Monthly");
    add(&mut houses, id, "2019-01", "200000.00");
    assert_eq!(houses.valuation_count(id), 1);
}

#[test]
fn test_csd015_ac_7_4_the_current_period_is_accepted_and_a_later_one_is_not() {
    let mut houses = Houses::default();
    let monthly = new_house(&mut houses, "Monthly", "Monthly");
    let yearly = new_house(&mut houses, "Yearly", "Yearly");
    add(&mut houses, monthly, "2026-10", "1000.00");
    add(&mut houses, yearly, "2026", "1000.00");
    assert_eq!(
        add_error(&mut houses, monthly, "2026-11", "1000.00").code(),
        "HOUSE_VALUATION_IN_FUTURE"
    );
    assert_eq!(
        add_error(&mut houses, yearly, "2027", "1000.00").code(),
        "HOUSE_VALUATION_IN_FUTURE"
    );
}

#[test]
fn test_csd015_ac_7_5_an_invalid_period_is_rejected() {
    let mut houses = Houses::default();
    let monthly = new_house(&mut houses, "Monthly", "Monthly");
    let yearly = new_house(&mut houses, "Yearly", "Yearly");
    for (id, text) in [
        (monthly, "2026-13"),
        (monthly, "2026-9"),
        (monthly, "abcd"),
        (yearly, "26"),
        (yearly, "abcd"),
        (yearly, "1899"),
    ] {
        assert_eq!(
            add_error(&mut houses, id, text, "1000.00").code(),
            "HOUSE_VALUATION_PERIOD_INVALID",
            "{text}"
        );
    }
}

#[test]
fn test_csd015_ac_8_1_a_duplicate_period_is_rejected() {
    let mut houses = Houses::default();
    let id = new_house(&mut houses, "Example House", "Monthly");
    houses
        .add_valuation(
            id,
            period("2026-09"),
            value("150000.00"),
            house_note("First").unwrap(),
            today(),
        )
        .unwrap();
    assert_eq!(
        add_error(&mut houses, id, "2026-09", "149000.00").code(),
        "HOUSE_VALUATION_DUPLICATE"
    );
    let rows = houses.all_valuations();
    assert_eq!(rows[0].1.value.value().to_string(), "150000.00");
    assert_eq!(rows[0].1.note.as_str(), "First");
}

#[test]
fn test_csd015_ac_8_2_the_same_period_on_another_house_is_allowed() {
    let mut houses = Houses::default();
    let first = new_house(&mut houses, "A", "Monthly");
    let second = new_house(&mut houses, "B", "Monthly");
    add(&mut houses, first, "2026-09", "100.00");
    add(&mut houses, second, "2026-09", "200.00");
    assert_eq!(houses.total_valuation_count(), 2);
}

#[test]
fn test_csd015_ac_9_a_valuation_must_match_the_valuation_period() {
    let mut houses = Houses::default();
    let yearly = new_house(&mut houses, "Yearly", "Yearly");
    let monthly = new_house(&mut houses, "Monthly", "Monthly");
    assert_eq!(
        add_error(&mut houses, yearly, "2026-09", "1000.00").code(),
        "HOUSE_PERIOD_MISMATCH"
    );
    assert_eq!(
        add_error(&mut houses, monthly, "2025", "1000.00").code(),
        "HOUSE_PERIOD_MISMATCH"
    );
}

// REQ-10, REQ-11

#[test]
fn test_csd015_ac_10_1_invalid_values_are_rejected() {
    for (text, expected) in [
        ("", "HOUSE_VALUATION_VALUE_INVALID"),
        ("abc", "HOUSE_VALUATION_VALUE_INVALID"),
        ("1e3", "HOUSE_VALUATION_VALUE_INVALID"),
        ("2500,50", "HOUSE_VALUATION_VALUE_INVALID"),
        ("-0.01", "HOUSE_VALUATION_VALUE_NEGATIVE"),
        ("1000.999", "HOUSE_VALUATION_VALUE_PRECISION"),
        ("1000000000.01", "HOUSE_VALUATION_VALUE_TOO_LARGE"),
    ] {
        assert_eq!(code(HouseValue::parse(text)), expected, "{text:?}");
    }
}

#[test]
fn test_csd015_ac_10_2_boundary_values_are_accepted() {
    for text in ["0.00", "1000000000.00"] {
        assert_eq!(value(text).value().to_string(), text);
    }
}

#[test]
fn test_csd015_ac_10_3_an_invalid_value_in_an_edit_changes_nothing() {
    let mut houses = Houses::default();
    let id = new_house(&mut houses, "Example House", "Monthly");
    add(&mut houses, id, "2026-09", "150000.00");
    assert_eq!(
        code(HouseValue::parse("-1.00")),
        "HOUSE_VALUATION_VALUE_NEGATIVE"
    );
    assert_eq!(
        houses.all_valuations()[0].1.value.value().to_string(),
        "150000.00"
    );
}

#[test]
fn test_csd015_ac_11_note_length_and_line_breaks() {
    assert!(house_note(&"a".repeat(2000)).is_ok());
    assert_eq!(
        code(house_note(&"a".repeat(2001))),
        "HOUSE_VALUATION_NOTE_TOO_LONG"
    );
    assert_eq!(
        house_note("line one\nline two").unwrap().as_str(),
        "line one\nline two"
    );
}

// REQ-12, REQ-13

#[test]
fn test_csd015_ac_12_1_edit_value_and_note_and_clear_the_note() {
    let mut houses = Houses::default();
    let id = new_house(&mut houses, "Example House", "Monthly");
    add(&mut houses, id, "2026-09", "150000.00");
    let edited = houses
        .update_valuation(
            id,
            period("2026-09"),
            period("2026-09"),
            value("149000.00"),
            house_note("Corrected").unwrap(),
            today(),
        )
        .unwrap();
    assert_eq!(edited.value.value().to_string(), "149000.00");
    assert_eq!(edited.note.as_str(), "Corrected");
    update(&mut houses, id, "2026-09", "2026-09", "149000.00").unwrap();
    assert_eq!(houses.all_valuations()[0].1.note.as_str(), "");
}

#[test]
fn test_csd015_ac_12_3_a_valuation_moves_to_another_month() {
    let mut houses = Houses::default();
    let id = new_house(&mut houses, "Example House", "Monthly");
    houses
        .add_valuation(
            id,
            period("2026-09"),
            value("150000.00"),
            house_note("First").unwrap(),
            today(),
        )
        .unwrap();
    houses
        .update_valuation(
            id,
            period("2026-09"),
            period("2026-08"),
            value("150000.00"),
            house_note("First").unwrap(),
            today(),
        )
        .unwrap();
    let rows = houses.all_valuations();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].1.period.to_string(), "2026-08");
    assert_eq!(rows[0].1.note.as_str(), "First");
}

#[test]
fn test_csd015_ac_12_4_the_period_and_the_value_change_together() {
    let mut houses = Houses::default();
    let id = new_house(&mut houses, "Example House", "Monthly");
    add(&mut houses, id, "2026-09", "150000.00");
    update(&mut houses, id, "2026-09", "2026-08", "149000.00").unwrap();
    let rows = houses.all_valuations();
    assert_eq!(rows[0].1.period.to_string(), "2026-08");
    assert_eq!(rows[0].1.value.value().to_string(), "149000.00");
}

#[test]
fn test_csd015_ac_12_5_a_yearly_valuation_moves() {
    let mut houses = Houses::default();
    let id = new_house(&mut houses, "Example Lodge", "Yearly");
    add(&mut houses, id, "2025", "8000.00");
    update(&mut houses, id, "2025", "2024", "8000.00").unwrap();
    assert_eq!(houses.all_valuations()[0].1.period.to_string(), "2024");
}

#[test]
fn test_csd015_ac_12_6_the_yearly_values_follow_the_move() {
    let mut houses = Houses::default();
    let id = new_house(&mut houses, "Example House", "Monthly");
    add(&mut houses, id, "2025-11", "10000.00");
    add(&mut houses, id, "2025-12", "9800.00");
    add(&mut houses, id, "2026-05", "9000.00");
    update(&mut houses, id, "2025-12", "2026-01", "9800.00").unwrap();
    let totals = houses.yearly_values();
    assert_eq!(totals[0].closing_value.to_string(), "9000.00");
    assert_eq!(totals[0].valuation_count, 2);
    assert_eq!(totals[1].closing_value.to_string(), "10000.00");
    assert_eq!(totals[1].valuation_count, 1);
}

#[test]
fn test_csd015_ac_12_8_keeping_the_period_is_not_a_duplicate() {
    let mut houses = Houses::default();
    let id = new_house(&mut houses, "Example House", "Monthly");
    add(&mut houses, id, "2026-09", "150000.00");
    assert!(update(&mut houses, id, "2026-09", "2026-09", "149000.00").is_ok());
}

#[test]
fn test_csd015_ac_13_1_a_new_period_in_the_future_is_rejected() {
    let mut houses = Houses::default();
    let monthly = new_house(&mut houses, "Monthly", "Monthly");
    let yearly = new_house(&mut houses, "Yearly", "Yearly");
    add(&mut houses, monthly, "2026-09", "1000.00");
    add(&mut houses, yearly, "2025", "1000.00");
    assert_eq!(
        code(update(&mut houses, monthly, "2026-09", "2026-11", "1.00")),
        "HOUSE_VALUATION_IN_FUTURE"
    );
    assert_eq!(
        code(update(&mut houses, yearly, "2025", "2027", "1.00")),
        "HOUSE_VALUATION_IN_FUTURE"
    );
}

#[test]
fn test_csd015_ac_13_2_a_taken_period_is_rejected_and_nothing_changes() {
    let mut houses = Houses::default();
    let id = new_house(&mut houses, "Example House", "Monthly");
    add(&mut houses, id, "2026-08", "151000.00");
    add(&mut houses, id, "2026-09", "150000.00");
    assert_eq!(
        code(update(&mut houses, id, "2026-09", "2026-08", "149000.00")),
        "HOUSE_VALUATION_DUPLICATE"
    );
    let rows: Vec<String> = houses
        .all_valuations()
        .iter()
        .map(|(_, s)| format!("{} {}", s.period, s.value.value()))
        .collect();
    assert_eq!(rows, ["2026-09 150000.00", "2026-08 151000.00"]);
}

#[test]
fn test_csd015_ac_13_3_a_period_of_the_wrong_kind_is_rejected() {
    let mut houses = Houses::default();
    let monthly = new_house(&mut houses, "Monthly", "Monthly");
    let yearly = new_house(&mut houses, "Yearly", "Yearly");
    add(&mut houses, monthly, "2026-09", "1.00");
    add(&mut houses, yearly, "2025", "1.00");
    assert_eq!(
        code(update(&mut houses, monthly, "2026-09", "2026", "1.00")),
        "HOUSE_PERIOD_MISMATCH"
    );
    assert_eq!(
        code(update(&mut houses, yearly, "2025", "2024-12", "1.00")),
        "HOUSE_PERIOD_MISMATCH"
    );
}

#[test]
fn test_csd015_ac_13_5_another_house_may_have_the_same_period() {
    let mut houses = Houses::default();
    let first = new_house(&mut houses, "A", "Monthly");
    let second = new_house(&mut houses, "B", "Monthly");
    add(&mut houses, first, "2026-08", "1.00");
    add(&mut houses, second, "2026-09", "2.00");
    update(&mut houses, second, "2026-09", "2026-08", "2.00").unwrap();
    assert!(houses
        .all_valuations()
        .iter()
        .all(|(_, s)| s.period.to_string() == "2026-08"));
}

#[test]
fn test_csd015_ac_13_6_an_invalid_period_is_rejected() {
    assert_eq!(
        code(parse_period("2026-13")),
        "HOUSE_VALUATION_PERIOD_INVALID"
    );
}

// REQ-14, REQ-15

#[test]
fn test_csd015_ac_14_1_deleting_a_valuation_removes_only_that_valuation() {
    let mut houses = Houses::default();
    let id = new_house(&mut houses, "Example House", "Monthly");
    add(&mut houses, id, "2026-08", "1.00");
    add(&mut houses, id, "2026-09", "2.00");
    houses.delete_valuation(id, period("2026-09")).unwrap();
    let rows = houses.all_valuations();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].1.period.to_string(), "2026-08");
    assert_eq!(
        code(houses.delete_valuation(id, period("2026-09"))),
        "HOUSE_VALUATION_NOT_FOUND"
    );
}

#[test]
fn test_csd015_ac_15_copy_previous() {
    let mut houses = Houses::default();
    let id = new_house(&mut houses, "Example House", "Monthly");
    // AC-15.2: nothing to copy yet.
    assert_eq!(houses.prefill_from_latest(id).unwrap().value, None);
    add(&mut houses, id, "2026-07", "151000.00");
    houses
        .add_valuation(
            id,
            period("2026-08"),
            value("150000.00"),
            house_note("Valuation report").unwrap(),
            today(),
        )
        .unwrap();
    // AC-15.1: the latest value, and nothing is stored.
    assert_eq!(
        houses
            .prefill_from_latest(id)
            .unwrap()
            .value
            .unwrap()
            .to_string(),
        "150000.00"
    );
    assert_eq!(houses.valuation_count(id), 2);
    // AC-15.3: 0.00 is copied as it is.
    add(&mut houses, id, "2026-09", "0.00");
    assert_eq!(
        houses
            .prefill_from_latest(id)
            .unwrap()
            .value
            .unwrap()
            .to_string(),
        "0.00"
    );
}

// REQ-17, REQ-18

#[test]
fn test_csd015_ac_17_1_the_latest_value_per_house() {
    let mut houses = Houses::default();
    let id = new_house(&mut houses, "Example House", "Monthly");
    add(&mut houses, id, "2026-07", "151000.00");
    add(&mut houses, id, "2026-09", "150000.00");
    let overview = houses.overview();
    assert_eq!(
        overview.houses[0].latest_value.unwrap().to_string(),
        "150000.00"
    );
    assert_eq!(
        overview.houses[0].latest_period.unwrap().to_string(),
        "2026-09"
    );
}

#[test]
fn test_csd015_ac_17_2_the_total_spans_houses_with_different_periods() {
    let mut houses = Houses::default();
    let monthly = new_house(&mut houses, "Example House", "Monthly");
    let yearly = new_house(&mut houses, "Example Lodge", "Yearly");
    add(&mut houses, monthly, "2026-09", "150000.50");
    add(&mut houses, yearly, "2025", "8000.00");
    let overview = houses.overview();
    assert_eq!(overview.total.unwrap().to_string(), "158000.50");
    let periods: Vec<String> = overview
        .houses
        .iter()
        .map(|d| d.latest_period.unwrap().to_string())
        .collect();
    assert_eq!(periods, ["2026-09", "2025"]);
}

#[test]
fn test_csd015_ac_17_3_houses_without_valuations_do_not_count() {
    let mut houses = Houses::default();
    let id = new_house(&mut houses, "Example House", "Monthly");
    new_house(&mut houses, "Example Empty", "Monthly");
    add(&mut houses, id, "2026-09", "150000.00");
    assert_eq!(houses.overview().total.unwrap().to_string(), "150000.00");
    assert_eq!(Houses::default().overview().total, None);
}

#[test]
fn test_csd015_ac_17_4_a_house_at_zero_counts_as_zero() {
    let mut houses = Houses::default();
    let house = new_house(&mut houses, "Example House", "Monthly");
    let car = new_house(&mut houses, "Example Cabin", "Monthly");
    add(&mut houses, house, "2026-09", "150000.00");
    add(&mut houses, car, "2026-09", "0.00");
    let overview = houses.overview();
    assert_eq!(overview.total.unwrap().to_string(), "150000.00");
    assert_eq!(overview.houses[0].latest_value.unwrap().to_string(), "0.00");
}

#[test]
fn test_csd015_ac_17_5_a_new_latest_valuation_updates_the_total_and_an_older_one_does_not() {
    let mut houses = Houses::default();
    let id = new_house(&mut houses, "Example House", "Monthly");
    add(&mut houses, id, "2026-09", "150000.00");
    add(&mut houses, id, "2026-01", "160000.00");
    assert_eq!(houses.overview().total.unwrap().to_string(), "150000.00");
    add(&mut houses, id, "2026-10", "149000.00");
    let overview = houses.overview();
    assert_eq!(overview.total.unwrap().to_string(), "149000.00");
    assert_eq!(
        overview.houses[0].latest_period.unwrap().to_string(),
        "2026-10"
    );
}

#[test]
fn test_csd015_ac_18_1_houses_are_alphabetical_ignoring_case() {
    let mut houses = Houses::default();
    for name in ["example land", "Example House", "Example Cabin"] {
        new_house(&mut houses, name, "Monthly");
    }
    let names: Vec<&str> = houses.houses().iter().map(|d| d.name.as_str()).collect();
    assert_eq!(names, ["Example Cabin", "Example House", "example land"]);
}

#[test]
fn test_csd015_ac_18_2_valuations_are_newest_first_and_a_year_counts_as_december() {
    let mut houses = Houses::default();
    let monthly = new_house(&mut houses, "Monthly", "Monthly");
    let yearly = new_house(&mut houses, "Yearly", "Yearly");
    add(&mut houses, monthly, "2025-11", "1.00");
    add(&mut houses, monthly, "2025-12", "1.00");
    add(&mut houses, yearly, "2025", "1.00");
    add(&mut houses, monthly, "2026-07", "1.00");
    let order: Vec<String> = houses
        .all_valuations()
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
fn test_csd015_ac_18_4_the_same_period_is_ordered_by_house() {
    let mut houses = Houses::default();
    let house = new_house(&mut houses, "Example House", "Monthly");
    let car = new_house(&mut houses, "Example Cabin", "Monthly");
    add(&mut houses, house, "2026-09", "1.00");
    add(&mut houses, car, "2026-09", "1.00");
    assert_eq!(houses.all_valuations()[0].0.name, "Example Cabin");
}

// REQ-19

#[test]
fn test_csd015_ac_19_1_the_change_follows_the_previous_valuation() {
    let mut houses = Houses::default();
    let id = new_house(&mut houses, "Example House", "Monthly");
    add(&mut houses, id, "2026-07", "10000.00");
    add(&mut houses, id, "2026-08", "10500.00");
    add(&mut houses, id, "2026-09", "10400.00");
    assert_eq!(change(&houses, id, "2026-07"), "none");
    assert_eq!(change(&houses, id, "2026-08"), "500.00");
    assert_eq!(change(&houses, id, "2026-09"), "-100.00");
}

#[test]
fn test_csd015_ac_19_2_a_gap_does_not_matter() {
    let mut houses = Houses::default();
    let id = new_house(&mut houses, "Example House", "Monthly");
    add(&mut houses, id, "2026-03", "10000.00");
    add(&mut houses, id, "2026-09", "12000.00");
    assert_eq!(change(&houses, id, "2026-09"), "2000.00");
}

#[test]
fn test_csd015_ac_19_3_yearly_valuations() {
    let mut houses = Houses::default();
    let id = new_house(&mut houses, "Example Lodge", "Yearly");
    add(&mut houses, id, "2024", "80000.00");
    add(&mut houses, id, "2025", "85000.00");
    assert_eq!(change(&houses, id, "2025"), "5000.00");
    assert_eq!(change(&houses, id, "2024"), "none");
}

#[test]
fn test_csd015_ac_19_4_another_house_does_not_count() {
    let mut houses = Houses::default();
    let house = new_house(&mut houses, "Example House", "Monthly");
    let car = new_house(&mut houses, "Example Cabin", "Monthly");
    add(&mut houses, house, "2026-09", "300000.00");
    add(&mut houses, car, "2026-08", "15000.00");
    add(&mut houses, car, "2026-09", "14500.00");
    assert_eq!(change(&houses, house, "2026-09"), "none");
    assert_eq!(change(&houses, car, "2026-09"), "-500.00");
}

#[test]
fn test_csd015_ac_19_5_changes_follow_a_move() {
    let mut houses = Houses::default();
    let id = new_house(&mut houses, "Example House", "Monthly");
    add(&mut houses, id, "2026-07", "10000.00");
    add(&mut houses, id, "2026-08", "10500.00");
    add(&mut houses, id, "2026-09", "11000.00");
    houses
        .update_valuation(
            id,
            period("2026-08"),
            period("2026-10"),
            value("10500.00"),
            Note::default(),
            today(),
        )
        .unwrap();
    assert_eq!(change(&houses, id, "2026-09"), "1000.00");
    assert_eq!(change(&houses, id, "2026-10"), "-500.00");
}

#[test]
fn test_csd015_ac_19_6_editing_an_earlier_valuation_changes_the_later_change() {
    let mut houses = Houses::default();
    let id = new_house(&mut houses, "Example House", "Monthly");
    add(&mut houses, id, "2026-08", "10500.00");
    add(&mut houses, id, "2026-09", "11000.00");
    houses
        .update_valuation(
            id,
            period("2026-08"),
            period("2026-08"),
            value("10200.00"),
            Note::default(),
            today(),
        )
        .unwrap();
    assert_eq!(change(&houses, id, "2026-09"), "800.00");
}

// REQ-20

/// The closing value, the change ("none" for none) and the valuation count of
/// `year`.
fn year(houses: &Houses, year: i32) -> (String, String, usize) {
    let total = houses
        .yearly_values()
        .into_iter()
        .find(|total| total.year == year)
        .unwrap();
    (
        total.closing_value.to_string(),
        total
            .change
            .map_or_else(|| "none".to_owned(), |value| value.to_string()),
        total.valuation_count,
    )
}

fn row(closing: &str, change: &str, count: usize) -> (String, String, usize) {
    (closing.to_owned(), change.to_owned(), count)
}

#[test]
fn test_csd015_ac_20_1_values_per_calendar_year() {
    let mut houses = Houses::default();
    let id = new_house(&mut houses, "Example House", "Monthly");
    add(&mut houses, id, "2025-11", "10000.00");
    add(&mut houses, id, "2025-12", "10200.00");
    add(&mut houses, id, "2026-05", "11000.00");
    assert_eq!(year(&houses, 2026), row("11000.00", "800.00", 1));
    assert_eq!(year(&houses, 2025), row("10200.00", "none", 2));
}

#[test]
fn test_csd015_ac_20_2_no_valuations_no_yearly_values() {
    let mut houses = Houses::default();
    new_house(&mut houses, "Example House", "Monthly");
    assert_eq!(houses.yearly_values().len(), 0);
}

#[test]
fn test_csd015_ac_20_3_values_add_up_all_houses() {
    let mut houses = Houses::default();
    let house = new_house(&mut houses, "Example House", "Monthly");
    let land = new_house(&mut houses, "Example Lodge", "Monthly");
    add(&mut houses, house, "2026-08", "300000.00");
    add(&mut houses, house, "2026-09", "301000.00");
    add(&mut houses, land, "2026-09", "50000.00");
    assert_eq!(houses.yearly_values().len(), 1);
    assert_eq!(year(&houses, 2026), row("351000.00", "none", 3));
}

#[test]
fn test_csd015_ac_20_4_the_closing_value_is_the_latest_value_not_a_sum() {
    let mut houses = Houses::default();
    let id = new_house(&mut houses, "Example House", "Monthly");
    add(&mut houses, id, "2026-03", "10000.00");
    add(&mut houses, id, "2026-09", "12000.00");
    assert_eq!(year(&houses, 2026).0, "12000.00");
}

#[test]
fn test_csd015_ac_20_5_monthly_and_yearly_houses_add_up_together() {
    let mut houses = Houses::default();
    let house = new_house(&mut houses, "Example House", "Monthly");
    let land = new_house(&mut houses, "Example Lodge", "Yearly");
    add(&mut houses, house, "2025-12", "300000.00");
    add(&mut houses, land, "2024", "50000.00");
    add(&mut houses, land, "2025", "55000.00");
    assert_eq!(year(&houses, 2025), row("355000.00", "5000.00", 2));
    assert_eq!(year(&houses, 2024), row("50000.00", "none", 1));
}

#[test]
fn test_csd015_ac_20_6_a_house_without_a_valuation_in_a_year_does_not_count_in_it() {
    let mut houses = Houses::default();
    let house = new_house(&mut houses, "Example House", "Monthly");
    let land = new_house(&mut houses, "Example Lodge", "Yearly");
    add(&mut houses, house, "2025-12", "300000.00");
    add(&mut houses, house, "2026-03", "310000.00");
    add(&mut houses, land, "2025", "55000.00");
    assert_eq!(year(&houses, 2026), row("310000.00", "10000.00", 1));
    assert_eq!(year(&houses, 2025), row("355000.00", "none", 2));
}

#[test]
fn test_csd015_ac_20_7_a_house_that_lost_value_shows_a_negative_change() {
    let mut houses = Houses::default();
    let id = new_house(&mut houses, "Example Cabin", "Monthly");
    add(&mut houses, id, "2025-12", "15000.00");
    add(&mut houses, id, "2026-03", "14000.00");
    assert_eq!(year(&houses, 2026).1, "-1000.00");
}

#[test]
fn test_csd015_ac_20_8_a_house_without_an_opening_value_adds_no_change() {
    let mut houses = Houses::default();
    let house = new_house(&mut houses, "Example House", "Monthly");
    let car = new_house(&mut houses, "Example Cabin", "Monthly");
    add(&mut houses, house, "2025-12", "300000.00");
    add(&mut houses, house, "2026-03", "310000.00");
    add(&mut houses, car, "2026-03", "15000.00");
    assert_eq!(year(&houses, 2026), row("325000.00", "10000.00", 2));
}

#[test]
fn test_csd015_ac_20_9_change_after_a_gap_counts_in_the_later_year() {
    let mut houses = Houses::default();
    let id = new_house(&mut houses, "Example House", "Monthly");
    add(&mut houses, id, "2024-12", "10000.00");
    add(&mut houses, id, "2026-06", "12000.00");
    assert_eq!(year(&houses, 2026).1, "2000.00");
    assert!(houses.yearly_values().iter().all(|t| t.year != 2025));
}

#[test]
fn test_csd015_ac_20_10_a_new_valuation_updates_the_yearly_values() {
    let mut houses = Houses::default();
    let id = new_house(&mut houses, "Example House", "Monthly");
    add(&mut houses, id, "2025-12", "10000.00");
    add(&mut houses, id, "2026-09", "11000.00");
    add(&mut houses, id, "2026-10", "11500.00");
    assert_eq!(year(&houses, 2026), row("11500.00", "1500.00", 2));
}

#[test]
fn test_csd015_ac_20_11_newest_year_first() {
    let mut houses = Houses::default();
    let id = new_house(&mut houses, "Example Lodge", "Yearly");
    for text in ["2024", "2025", "2026"] {
        add(&mut houses, id, text, "1.00");
    }
    let years: Vec<i32> = houses.yearly_values().iter().map(|t| t.year).collect();
    assert_eq!(years, [2026, 2025, 2024]);
}

// The data of a file

#[test]
fn test_csd015_a_damaged_section_is_not_consistent() {
    let mut houses = Houses::default();
    let id = new_house(&mut houses, "Example House", "Monthly");
    add(&mut houses, id, "2026-09", "150000.00");
    assert!(houses.validate().is_ok());

    let mut bad_slot = houses.clone();
    bad_slot.houses[0].colour_slot = COLOUR_SLOTS;
    assert_eq!(code(bad_slot.validate()), "HOUSE_DATA_INVALID");

    let mut orphan = houses.clone();
    orphan.houses.clear();
    assert_eq!(code(orphan.validate()), "HOUSE_DATA_INVALID");

    let mut duplicate = houses.clone();
    let valuation = duplicate.valuations[0].clone();
    duplicate.valuations.push(valuation);
    assert_eq!(code(duplicate.validate()), "HOUSE_DATA_INVALID");
}

// CSD-016 REQ-2 to REQ-5

#[test]
fn test_csd016_ac_2_the_series_runs_from_the_first_valuation_to_the_current_year() {
    let mut houses = Houses::default();
    let id = new_house(&mut houses, "Example House", "Monthly");
    add(&mut houses, id, "2024-03", "100.00");
    add(&mut houses, id, "2025-05", "100.00");
    add(&mut houses, id, "2026-05", "100.00");
    let years: Vec<i32> = houses
        .insights(today())
        .years
        .iter()
        .map(|y| y.year)
        .collect();
    assert_eq!(years, [2024, 2025, 2026]);

    // AC-2.2 and AC-2.3
    let mut single = Houses::default();
    let one = new_house(&mut single, "Example House", "Monthly");
    assert_eq!(single.insights(today()).years.len(), 0);
    add(&mut single, one, "2026-09", "100.00");
    assert_eq!(single.insights(today()).years.len(), 1);
}

#[test]
fn test_csd016_ac_3_1_a_gap_year_is_zero_without_segments() {
    let mut houses = Houses::default();
    let id = new_house(&mut houses, "Example Lodge", "Yearly");
    add(&mut houses, id, "2022", "100.00");
    add(&mut houses, id, "2024", "200.00");
    let years = houses.insights(today()).years;
    let gap = years.iter().find(|y| y.year == 2023).unwrap();
    assert_eq!(gap.closing_value.to_string(), "0.00");
    assert_eq!(gap.change, None);
    assert_eq!(gap.valuation_count, 0);
    assert_eq!(gap.segments.len(), 0);
    let numbers: Vec<i32> = years.iter().map(|y| y.year).collect();
    assert_eq!(numbers, [2022, 2023, 2024, 2025, 2026]);
}

#[test]
fn test_csd016_ac_4_the_current_year_is_partial_even_without_valuations() {
    let mut houses = Houses::default();
    let id = new_house(&mut houses, "Example Lodge", "Yearly");
    add(&mut houses, id, "2025", "100.00");
    let years = houses.insights(today()).years;
    assert_eq!(years.len(), 2);
    assert!(!years[0].is_partial);
    assert!(years[1].is_partial);
    assert_eq!(years[1].valuation_count, 0);
}

#[test]
fn test_csd016_ac_5_1_the_segments_are_the_latest_values_per_house() {
    let mut houses = Houses::default();
    let house = new_house(&mut houses, "Example House", "Monthly");
    let land = new_house(&mut houses, "Example Lodge", "Monthly");
    add(&mut houses, house, "2026-03", "151000.00");
    add(&mut houses, house, "2026-09", "150000.00");
    add(&mut houses, land, "2026-06", "8000.00");
    let year = houses.insights(today()).years.remove(0);
    let segments: Vec<String> = year.segments.iter().map(|s| s.value.to_string()).collect();
    assert_eq!(segments, ["150000.00", "8000.00"]);
    assert_eq!(year.closing_value.to_string(), "158000.00");
    assert_eq!(year.valuation_count, 3);
}

#[test]
fn test_csd016_ac_5_2_a_house_without_a_valuation_in_the_year_has_no_segment() {
    let mut houses = Houses::default();
    let house = new_house(&mut houses, "Example House", "Monthly");
    let land = new_house(&mut houses, "Example Lodge", "Yearly");
    add(&mut houses, house, "2025-12", "10000.00");
    add(&mut houses, house, "2026-03", "9500.00");
    add(&mut houses, land, "2025", "6000.00");
    let years = houses.insights(today()).years;
    assert_eq!(years[0].segments.len(), 2);
    assert_eq!(years[1].segments.len(), 1);
    assert_eq!(years[1].segments[0].house_id, house);
}

#[test]
fn test_csd016_ac_5_3_a_house_at_zero_has_a_zero_segment() {
    let mut houses = Houses::default();
    let house = new_house(&mut houses, "Example House", "Monthly");
    let car = new_house(&mut houses, "Example Cabin", "Monthly");
    add(&mut houses, house, "2026-09", "150000.00");
    add(&mut houses, car, "2026-09", "0.00");
    let year = houses.insights(today()).years.remove(0);
    let segments: Vec<String> = year.segments.iter().map(|s| s.value.to_string()).collect();
    assert_eq!(segments, ["150000.00", "0.00"]);
    assert_eq!(year.closing_value.to_string(), "150000.00");
    assert_eq!(year.valuation_count, 2);
}

#[test]
fn test_csd016_ac_5_4_segments_and_houses_follow_creation_order_not_names() {
    let mut houses = Houses::default();
    let land = new_house(&mut houses, "Example Lodge", "Monthly");
    let house = new_house(&mut houses, "Example House", "Monthly");
    add(&mut houses, land, "2026-09", "1.00");
    add(&mut houses, house, "2026-09", "2.00");
    let insights = houses.insights(today());
    assert_eq!(insights.years[0].segments[0].house_id, land);
    let names: Vec<&str> = insights.houses.iter().map(|d| d.name.as_str()).collect();
    assert_eq!(names, ["Example Lodge", "Example House"]);
}

#[test]
fn test_csd016_ac_5_5_the_insights_equal_the_yearly_values() {
    let mut houses = Houses::default();
    let house = new_house(&mut houses, "Example House", "Monthly");
    let land = new_house(&mut houses, "Example Lodge", "Yearly");
    add(&mut houses, house, "2025-11", "10000.00");
    add(&mut houses, house, "2025-12", "9800.00");
    add(&mut houses, house, "2026-03", "9000.00");
    add(&mut houses, land, "2024", "7000.00");
    add(&mut houses, land, "2025", "6000.00");
    let totals = houses.yearly_values();
    for year in houses.insights(today()).years {
        let total = totals.iter().find(|t| t.year == year.year).unwrap();
        assert_eq!(year.closing_value, total.closing_value);
        assert_eq!(year.change, total.change);
        assert_eq!(year.valuation_count, total.valuation_count);
    }
}

#[test]
fn test_csd016_ac_9_2_a_house_without_valuations_is_not_in_the_legend() {
    let mut houses = Houses::default();
    new_house(&mut houses, "Example Empty", "Monthly");
    let house = new_house(&mut houses, "Example House", "Monthly");
    add(&mut houses, house, "2026-09", "1.00");
    let insights = houses.insights(today());
    assert_eq!(insights.houses.len(), 1);
    assert_eq!(insights.houses[0].id, house);
}

#[test]
fn test_csd016_ac_15_3_a_moved_valuation_follows_to_its_new_year() {
    let mut houses = Houses::default();
    let id = new_house(&mut houses, "Example House", "Monthly");
    add(&mut houses, id, "2026-01", "100.00");
    update(&mut houses, id, "2026-01", "2025-12", "100.00").unwrap();
    let years = houses.insights(today()).years;
    assert_eq!(
        years.iter().map(|y| y.year).collect::<Vec<_>>(),
        [2025, 2026]
    );
    assert_eq!(years[0].segments.len(), 1);
    assert_eq!(years[1].segments.len(), 0);
}
