//! Tests for CSD-019: the purchase and sale of an asset.

use super::*;
const LEGACY_FILE: &str = r#"{
  "assets": [
    {"id": "11111111-1111-4111-8111-111111111111", "name": "Example House",
     "valuation_period": "Monthly", "colour_slot": 0}
  ],
  "valuations": []
}"#;

fn today_date() -> Date {
    time::macros::date!(2026 - 10 - 29)
}

fn today() -> YearMonth {
    YearMonth::parse("2026-10").unwrap()
}

fn period(text: &str) -> Period {
    parse_period(text).unwrap()
}

/// Input with a purchase and, when `sell_date` is not empty, a sale; an empty
/// text means "not typed".
fn input(name: &str, kind: &str, bought: (&str, &str), sold: (&str, &str)) -> AssetInput {
    let text = |value: &str| Some(value.to_owned());
    AssetInput {
        name: name.to_owned(),
        valuation_period: text(kind),
        purchase_date: text(bought.0),
        purchase_amount: text(bought.1),
        sell_date: text(sold.0),
        sell_amount: text(sold.1),
    }
}

fn create(assets: &mut Assets, bought: (&str, &str), sold: (&str, &str)) -> AssetId {
    assets
        .create_asset(
            &input("Example House", "Monthly", bought, sold),
            today_date(),
        )
        .unwrap()
        .id
}

fn held(assets: &mut Assets, kind: &str, purchase: &str) -> AssetId {
    assets
        .create_asset(
            &input("Example House", kind, (purchase, "250000.00"), ("", "")),
            today_date(),
        )
        .unwrap()
        .id
}

fn add(assets: &mut Assets, id: AssetId, at: &str, amount: &str) -> Result<(), DomainError> {
    assets
        .add_valuation(
            id,
            period(at),
            AssetValue::parse(amount).unwrap(),
            Note::default(),
            today(),
        )
        .map(|_| ())
}

fn code<T>(result: Result<T, DomainError>) -> &'static str {
    result.err().map_or("OK", |error| error.code())
}

fn asset(assets: &Assets, id: AssetId) -> Asset {
    assets
        .assets()
        .into_iter()
        .find(|asset| asset.id == id)
        .unwrap()
        .clone()
}

/// The closing value and change of `year` in the yearly values, as text.
fn yearly(assets: &Assets, year: i32) -> Option<(String, String)> {
    assets
        .yearly_values()
        .into_iter()
        .find(|row| row.year == year)
        .map(|row| {
            (
                row.closing_value.to_string(),
                row.change
                    .map_or_else(|| "none".to_owned(), |c| c.to_string()),
            )
        })
}

fn pair(closing: &str, change: &str) -> (String, String) {
    (closing.to_owned(), change.to_owned())
}

// REQ-1

#[test]
fn test_csd019_ac_1_1_an_asset_with_a_purchase() {
    let mut assets = Assets::default();
    let id = create(&mut assets, ("15/03/2024", "250000.00"), ("", ""));
    let created = asset(&assets, id);
    assert_eq!(
        serde_json::to_value(created.purchase_date).unwrap(),
        "2024-03-15"
    );
    assert_eq!(
        created.purchase_amount.unwrap().value().to_string(),
        "250000.00"
    );
    assert!(created.is_held());
    assert_eq!(created.sell_date, None);
}

#[test]
fn test_csd019_ac_1_2_a_missing_purchase_field_is_rejected() {
    for (date, amount, expected) in [
        ("", "250000.00", "ASSET_PURCHASE_DATE_REQUIRED"),
        ("15/03/2024", "", "ASSET_PURCHASE_AMOUNT_REQUIRED"),
    ] {
        let mut assets = Assets::default();
        let result = assets.create_asset(
            &input("Example House", "Monthly", (date, amount), ("", "")),
            today_date(),
        );
        assert_eq!(code(result), expected);
        assert_eq!(assets.assets().len(), 0);
    }
}

// REQ-2

#[test]
fn test_csd019_ac_2_1_sell_an_asset() {
    let mut assets = Assets::default();
    let id = create(&mut assets, ("15/03/2024", "250000.00"), ("", ""));
    assets
        .update_asset(
            id,
            &input(
                "Example House",
                "Monthly",
                ("15/03/2024", "250000.00"),
                ("10/06/2026", "300000.00"),
            ),
            today_date(),
        )
        .unwrap();
    let sold = asset(&assets, id);
    assert!(!sold.is_held());
    assert_eq!(serde_json::to_value(sold.sell_date).unwrap(), "2026-06-10");
    assert_eq!(sold.sell_amount.unwrap().value().to_string(), "300000.00");
}

#[test]
fn test_csd019_ac_2_2_undo_a_sale() {
    let mut assets = Assets::default();
    let id = create(
        &mut assets,
        ("15/03/2024", "250000.00"),
        ("10/06/2026", "300000.00"),
    );
    assets
        .update_asset(
            id,
            &input(
                "Example House",
                "Monthly",
                ("15/03/2024", "250000.00"),
                ("", ""),
            ),
            today_date(),
        )
        .unwrap();
    let held = asset(&assets, id);
    assert!(held.is_held());
    assert_eq!(held.sell_amount, None);
}

#[test]
fn test_csd019_ac_2_3_a_sale_on_the_day_of_purchase_is_allowed() {
    let mut assets = Assets::default();
    let result = assets.create_asset(
        &input(
            "Example Car",
            "Monthly",
            ("01/09/2026", "10000.00"),
            ("01/09/2026", "9500.00"),
        ),
        today_date(),
    );
    assert_eq!(code(result), "OK");
}

// REQ-3

#[test]
fn test_csd019_ac_3_1_invalid_dates_are_rejected() {
    for (purchase, sell, sell_amount, expected) in [
        ("31/02/2024", "", "", "ASSET_DATE_INVALID"),
        ("2024-03-15", "", "", "ASSET_DATE_INVALID"),
        ("30/10/2026", "", "", "ASSET_DATE_IN_FUTURE"),
        (
            "15/03/2024",
            "30/10/2026",
            "1000.00",
            "ASSET_DATE_IN_FUTURE",
        ),
        (
            "15/03/2024",
            "14/03/2024",
            "1000.00",
            "ASSET_SELL_BEFORE_PURCHASE",
        ),
    ] {
        let mut assets = Assets::default();
        let result = assets.create_asset(
            &input(
                "Example House",
                "Monthly",
                (purchase, "250000.00"),
                (sell, sell_amount),
            ),
            today_date(),
        );
        assert_eq!(code(result), expected, "{purchase} {sell}");
        assert_eq!(assets.assets().len(), 0);
    }
}

#[test]
fn test_csd019_ac_3_2_today_is_accepted() {
    let mut assets = Assets::default();
    held(&mut assets, "Monthly", "29/10/2026");
    assert_eq!(assets.assets().len(), 1);
}

// REQ-4

#[test]
fn test_csd019_ac_4_1_invalid_amounts_are_rejected() {
    for (amount, expected) in [
        ("abc", "ASSET_AMOUNT_INVALID"),
        ("1e3", "ASSET_AMOUNT_INVALID"),
        ("-0.01", "ASSET_AMOUNT_NEGATIVE"),
        ("1000.999", "ASSET_AMOUNT_PRECISION"),
        ("1000000000.01", "ASSET_AMOUNT_TOO_LARGE"),
    ] {
        let mut assets = Assets::default();
        let id = create(&mut assets, ("15/03/2024", "250000.00"), ("", ""));
        let result = assets.update_asset(
            id,
            &input(
                "Example House",
                "Monthly",
                ("15/03/2024", "250000.00"),
                ("10/06/2026", amount),
            ),
            today_date(),
        );
        assert_eq!(code(result), expected, "{amount}");
        assert!(asset(&assets, id).is_held());
    }
}

#[test]
fn test_csd019_ac_4_2_a_worthless_sale_and_a_free_purchase_are_valid() {
    let mut assets = Assets::default();
    let id = create(&mut assets, ("15/03/2024", "0.00"), ("10/06/2026", "0.00"));
    assert_eq!(asset(&assets, id).result().unwrap().to_string(), "0.00");
}

// REQ-5

#[test]
fn test_csd019_ac_5_1_half_a_sale_is_rejected() {
    for (date, amount) in [("10/06/2026", ""), ("", "300000.00")] {
        let mut assets = Assets::default();
        let id = create(&mut assets, ("15/03/2024", "250000.00"), ("", ""));
        let result = assets.update_asset(
            id,
            &input(
                "Example House",
                "Monthly",
                ("15/03/2024", "250000.00"),
                (date, amount),
            ),
            today_date(),
        );
        assert_eq!(code(result), "ASSET_SALE_INCOMPLETE");
        assert!(asset(&assets, id).is_held());
    }
}

// REQ-6

fn legacy() -> (Assets, AssetId) {
    let assets: Assets = serde_json::from_str(LEGACY_FILE).unwrap();
    assets.validate().unwrap();
    let id = assets.assets()[0].id;
    (assets, id)
}

#[test]
fn test_csd019_ac_6_1_a_legacy_file_opens() {
    let (assets, id) = legacy();
    let legacy = asset(&assets, id);
    assert!(legacy.is_held());
    assert_eq!(legacy.purchase_date, None);
    assert_eq!(legacy.purchase_amount, None);
    let overview = assets.overview();
    assert!(overview.assets[0].held);
}

#[test]
fn test_csd019_ac_6_2_editing_a_legacy_asset_needs_the_purchase() {
    let (mut assets, id) = legacy();
    let result = assets.update_asset(
        id,
        &input("Example Home", "Monthly", ("", ""), ("", "")),
        today_date(),
    );
    assert_eq!(code(result), "ASSET_PURCHASE_DATE_REQUIRED");
    assert_eq!(asset(&assets, id).name, "Example House");
}

#[test]
fn test_csd019_ac_6_3_a_legacy_asset_is_complete_once_the_purchase_is_filled_in() {
    let (mut assets, id) = legacy();
    assets
        .update_asset(
            id,
            &input(
                "Example House",
                "Monthly",
                ("15/03/2024", "250000.00"),
                ("", ""),
            ),
            today_date(),
        )
        .unwrap();
    assert_eq!(
        asset(&assets, id)
            .purchase_amount
            .unwrap()
            .value()
            .to_string(),
        "250000.00"
    );
}

#[test]
fn test_csd019_ac_6_4_a_legacy_asset_has_no_lower_bound_on_valuations() {
    let (mut assets, id) = legacy();
    assert_eq!(code(add(&mut assets, id, "2019-01", "200000.00")), "OK");
}

// REQ-7

#[test]
fn test_csd019_ac_7_1_a_period_outside_the_window_is_rejected() {
    for (at, expected) in [
        ("2024-02", "ASSET_VALUATION_OUTSIDE_HOLDING"),
        ("2024-03", "OK"),
        ("2026-06", "OK"),
        ("2026-07", "ASSET_VALUATION_OUTSIDE_HOLDING"),
    ] {
        let mut assets = Assets::default();
        let id = create(
            &mut assets,
            ("15/03/2024", "250000.00"),
            ("10/06/2026", "300000.00"),
        );
        assert_eq!(code(add(&mut assets, id, at, "1000.00")), expected, "{at}");
    }
}

#[test]
fn test_csd019_ac_7_2_a_yearly_asset_is_compared_by_year() {
    let mut assets = Assets::default();
    let id = held(&mut assets, "Yearly", "15/03/2024");
    assert_eq!(
        code(add(&mut assets, id, "2023", "1000.00")),
        "ASSET_VALUATION_OUTSIDE_HOLDING"
    );
    assert_eq!(code(add(&mut assets, id, "2024", "1000.00")), "OK");
}

#[test]
fn test_csd019_ac_7_2_a_yearly_asset_may_have_its_sale_year() {
    let mut assets = Assets::default();
    let id = assets
        .create_asset(
            &input(
                "Example Land",
                "Yearly",
                ("15/03/2024", "1.00"),
                ("10/06/2026", "1.00"),
            ),
            today_date(),
        )
        .unwrap()
        .id;
    assert_eq!(code(add(&mut assets, id, "2026", "1000.00")), "OK");
}

#[test]
fn test_csd019_ac_7_3_moving_a_valuation_out_of_the_window_is_rejected() {
    let mut assets = Assets::default();
    let id = held(&mut assets, "Monthly", "15/03/2024");
    add(&mut assets, id, "2024-05", "1000.00").unwrap();
    let result = assets.update_valuation(
        id,
        period("2024-05"),
        period("2024-01"),
        AssetValue::parse("1000.00").unwrap(),
        Note::default(),
        today(),
    );
    assert_eq!(code(result), "ASSET_VALUATION_OUTSIDE_HOLDING");
    assert_eq!(assets.valuation_count(id), 1);
    assert!(assets
        .all_valuations()
        .iter()
        .any(|(_, valuation)| valuation.period == period("2024-05")));
}

// REQ-8

#[test]
fn test_csd019_ac_8_1_narrowing_the_window_is_rejected() {
    for (bought, sold) in [
        (("01/06/2024", "250000.00"), ("", "")),
        (("15/03/2024", "250000.00"), ("10/06/2026", "1.00")),
    ] {
        let mut assets = Assets::default();
        let id = held(&mut assets, "Monthly", "15/03/2024");
        add(&mut assets, id, "2024-05", "1000.00").unwrap();
        add(&mut assets, id, "2026-09", "1000.00").unwrap();
        let before = asset(&assets, id);
        let result = assets.update_asset(
            id,
            &input("Example House", "Monthly", bought, sold),
            today_date(),
        );
        assert_eq!(code(result), "ASSET_DATES_EXCLUDE_VALUATIONS");
        assert_eq!(asset(&assets, id), before);
    }
}

#[test]
fn test_csd019_ac_8_2_widening_the_window_is_accepted() {
    let mut assets = Assets::default();
    let id = held(&mut assets, "Monthly", "15/03/2024");
    add(&mut assets, id, "2024-05", "1000.00").unwrap();
    assets
        .update_asset(
            id,
            &input(
                "Example House",
                "Monthly",
                ("01/01/2024", "250000.00"),
                ("", ""),
            ),
            today_date(),
        )
        .unwrap();
    assert_eq!(
        serde_json::to_value(asset(&assets, id).purchase_date).unwrap(),
        "2024-01-01"
    );
}

// REQ-9

#[test]
fn test_csd019_ac_9_1_a_held_asset() {
    let mut assets = Assets::default();
    let id = held(&mut assets, "Monthly", "15/03/2024");
    let overview = assets.overview();
    let row = &overview.assets[0];
    assert_eq!(row.asset.id, id);
    assert!(row.held);
    assert_eq!(row.result, None);
}

#[test]
fn test_csd019_ac_9_2_a_sold_asset_with_its_result() {
    for (sell, result) in [
        ("300000.00", "50000.00"),
        ("240000.00", "-10000.00"),
        ("250000.00", "0.00"),
    ] {
        let mut assets = Assets::default();
        create(
            &mut assets,
            ("15/03/2024", "250000.00"),
            ("10/06/2026", sell),
        );
        let overview = assets.overview();
        assert!(!overview.assets[0].held);
        assert_eq!(overview.assets[0].result.unwrap().to_string(), result);
    }
}

#[test]
fn test_csd019_ac_9_3_a_sold_asset_stays_in_the_list() {
    let mut assets = Assets::default();
    create(
        &mut assets,
        ("15/03/2024", "250000.00"),
        ("10/06/2026", "300000.00"),
    );
    assets
        .create_asset(
            &input("Example Land", "Monthly", ("15/03/2024", "1.00"), ("", "")),
            today_date(),
        )
        .unwrap();
    let names: Vec<String> = assets
        .overview()
        .assets
        .into_iter()
        .map(|row| row.asset.name)
        .collect();
    assert_eq!(names, ["Example House", "Example Land"]);
}

// REQ-10

#[test]
fn test_csd019_ac_10_1_a_sold_asset_is_not_in_the_total() {
    let mut assets = Assets::default();
    let sold = create(
        &mut assets,
        ("15/03/2024", "250000.00"),
        ("10/06/2026", "300000.00"),
    );
    add(&mut assets, sold, "2026-05", "300000.00").unwrap();
    let land = assets
        .create_asset(
            &input("Example Land", "Monthly", ("15/03/2024", "1.00"), ("", "")),
            today_date(),
        )
        .unwrap()
        .id;
    add(&mut assets, land, "2026-09", "50000.00").unwrap();
    let overview = assets.overview();
    assert_eq!(overview.total.unwrap().to_string(), "50000.00");
    let house = overview
        .assets
        .iter()
        .find(|row| row.asset.id == sold)
        .unwrap();
    assert_eq!(house.latest_value.unwrap().to_string(), "300000.00");
    assert_eq!(house.latest_period, Some(period("2026-05")));
}

#[test]
fn test_csd019_ac_10_2_selling_an_asset_lowers_the_total() {
    let mut assets = Assets::default();
    let house = held(&mut assets, "Monthly", "15/03/2024");
    add(&mut assets, house, "2026-09", "300000.00").unwrap();
    let land = assets
        .create_asset(
            &input("Example Land", "Monthly", ("15/03/2024", "1.00"), ("", "")),
            today_date(),
        )
        .unwrap()
        .id;
    add(&mut assets, land, "2026-09", "50000.00").unwrap();
    assert_eq!(assets.overview().total.unwrap().to_string(), "350000.00");
    assets
        .update_asset(
            house,
            &input(
                "Example House",
                "Monthly",
                ("15/03/2024", "250000.00"),
                ("10/10/2026", "310000.00"),
            ),
            today_date(),
        )
        .unwrap();
    assert_eq!(assets.overview().total.unwrap().to_string(), "50000.00");
}

// REQ-11

fn sold_house_with_valuations() -> (Assets, AssetId) {
    let mut assets = Assets::default();
    let id = create(
        &mut assets,
        ("15/03/2024", "250000.00"),
        ("10/06/2026", "300000.00"),
    );
    add(&mut assets, id, "2024-12", "200000.00").unwrap();
    add(&mut assets, id, "2025-12", "210000.00").unwrap();
    add(&mut assets, id, "2026-03", "215000.00").unwrap();
    (assets, id)
}

#[test]
fn test_csd019_ac_11_1_a_sold_asset_stops_counting_in_its_sale_year() {
    let (assets, _) = sold_house_with_valuations();
    assert_eq!(yearly(&assets, 2025), Some(pair("210000.00", "10000.00")));
    assert_eq!(yearly(&assets, 2024), Some(pair("200000.00", "none")));
    assert_eq!(yearly(&assets, 2026), None);
}

#[test]
fn test_csd019_ac_11_2_the_insights_show_zero_from_the_sale_year() {
    let (assets, _) = sold_house_with_valuations();
    let insights = assets.insights(today());
    let year = |y: i32| insights.years.iter().find(|row| row.year == y).unwrap();
    assert_eq!(year(2025).closing_value.to_string(), "210000.00");
    assert_eq!(year(2026).closing_value.to_string(), "0.00");
    assert_eq!(year(2026).segments.len(), 0);
}

#[test]
fn test_csd019_ac_11_3_year_end_boundary() {
    for (sell, expected) in [
        (("31/12/2025", "1.00"), None),
        (("", ""), Some(pair("1000.00", "none"))),
    ] {
        let mut assets = Assets::default();
        let id = create(&mut assets, ("15/03/2024", "250000.00"), sell);
        add(&mut assets, id, "2025-12", "1000.00").unwrap();
        assert_eq!(yearly(&assets, 2025), expected, "{sell:?}");
    }
}

#[test]
fn test_csd019_ac_11_4_other_assets_are_unaffected() {
    let (mut assets, _) = sold_house_with_valuations();
    let land = assets
        .create_asset(
            &input("Example Land", "Monthly", ("15/03/2024", "1.00"), ("", "")),
            today_date(),
        )
        .unwrap()
        .id;
    add(&mut assets, land, "2026-03", "50000.00").unwrap();
    assert_eq!(yearly(&assets, 2026), Some(pair("50000.00", "none")));
}

#[test]
fn test_csd019_ac_11_5_a_purchase_after_year_end_is_not_counted_in_that_year() {
    let mut assets = Assets::default();
    let id = held(&mut assets, "Monthly", "15/01/2026");
    add(&mut assets, id, "2026-03", "1000.00").unwrap();
    assert_eq!(yearly(&assets, 2025), None);
    assert_eq!(yearly(&assets, 2026), Some(pair("1000.00", "none")));
}

// REQ-13

#[test]
fn test_csd019_ac_13_1_the_purchase_and_sale_survive_a_round_trip() {
    let mut assets = Assets::default();
    let id = create(
        &mut assets,
        ("15/03/2024", "250000.00"),
        ("10/06/2026", "300000.00"),
    );
    let json = serde_json::to_string(&assets).unwrap();
    let restored: Assets = serde_json::from_str(&json).unwrap();
    restored.validate().unwrap();
    assert_eq!(restored, assets);
    assert_eq!(
        asset(&restored, id).result().unwrap().to_string(),
        "50000.00"
    );
}

#[test]
fn test_csd019_ac_13_2_a_damaged_sale_is_invalid() {
    let mut assets = Assets::default();
    held(&mut assets, "Monthly", "15/03/2024");
    let mut json = serde_json::to_value(&assets).unwrap();
    json["assets"][0]["sell_date"] = serde_json::json!("2026-06-10");
    let damaged: Assets = serde_json::from_value(json).unwrap();
    assert_eq!(damaged.validate().unwrap_err().code(), "ASSET_DATA_INVALID");
}

#[test]
fn test_csd019_ac_13_3_a_valuation_outside_the_window_is_invalid() {
    let mut assets = Assets::default();
    let id = held(&mut assets, "Monthly", "15/03/2024");
    add(&mut assets, id, "2024-05", "1000.00").unwrap();
    let mut json = serde_json::to_value(&assets).unwrap();
    json["valuations"][0]["period"] = serde_json::json!("2023-12");
    let damaged: Assets = serde_json::from_value(json).unwrap();
    assert_eq!(damaged.validate().unwrap_err().code(), "ASSET_DATA_INVALID");
}
