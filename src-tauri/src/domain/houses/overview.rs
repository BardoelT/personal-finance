//! The houses with their latest values and the total house (CSD-015 REQ-17).

use rust_decimal::Decimal;
use serde::Serialize;

use crate::domain::investments::Period;

use super::{House, Houses};

/// One house row of the Houses view: its latest value and period (CSD-015
/// REQ-17).
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct HouseSummary {
    #[serde(flatten)]
    pub house: House,
    pub valuation_count: usize,
    pub latest_period: Option<Period>,
    pub latest_value: Option<Decimal>,
    /// Whether the house has no sale (CSD-019 REQ-9).
    pub held: bool,
    /// The sell amount minus the purchase amount of a sold house.
    pub result: Option<Decimal>,
}

/// The houses with the total of the latest values of the held houses (CSD-015
/// REQ-17, CSD-019 REQ-10). The total is `None` while no held house has a
/// valuation.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct HousesOverview {
    pub houses: Vec<HouseSummary>,
    pub total: Option<Decimal>,
}

impl Houses {
    /// The houses with their latest values and the total house (REQ-17).
    /// Periods are not aligned: each house counts with its own latest valuation.
    #[must_use]
    pub fn overview(&self) -> HousesOverview {
        let houses: Vec<HouseSummary> = self
            .houses()
            .into_iter()
            .map(|house| {
                let latest = self.latest(house.id);
                HouseSummary {
                    house: house.clone(),
                    valuation_count: self.valuation_count(house.id),
                    latest_period: latest.map(|valuation| valuation.period),
                    latest_value: latest.map(|valuation| valuation.value.value()),
                    held: house.is_held(),
                    result: house.result(),
                }
            })
            .collect();
        let mut values = houses
            .iter()
            .filter(|summary| summary.held)
            .filter_map(|summary| summary.latest_value);
        let total = values
            .next()
            .map(|first| values.fold(first, |sum, value| sum + value));
        HousesOverview { houses, total }
    }
}
