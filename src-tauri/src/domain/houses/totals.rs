//! Yearly values over all houses (CSD-015 REQ-20, CSD-019 REQ-11).

use std::collections::BTreeMap;

use rust_decimal::Decimal;
use serde::Serialize;

use crate::domain::money::zero;

use super::{House, Houses};

/// The totals of one calendar year over all houses (CSD-015 REQ-20).
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct HouseYearlyValue {
    pub year: i32,
    /// The sum, over the houses with a valuation this year, of the value of
    /// their latest valuation this year.
    pub closing_value: Decimal,
    /// Closing value minus opening value, over the houses that have both;
    /// `None` when no house has an opening value this year.
    pub change: Option<Decimal>,
    pub valuation_count: usize,
}

impl Houses {
    /// Totals per calendar year over all houses, newest year first, without
    /// extrapolating partial years or carrying values forward (REQ-20). A
    /// valuation counts in the year of its own period only.
    #[must_use]
    pub fn yearly_values(&self) -> Vec<HouseYearlyValue> {
        self.counted_valuations_per_year()
            .into_iter()
            .rev()
            .map(|(year, valuation_count)| HouseYearlyValue {
                year,
                closing_value: self
                    .year_segments(year)
                    .into_iter()
                    .fold(zero(), |sum, (_, value)| sum + value),
                change: self.year_change(year),
                valuation_count,
            })
            .collect()
    }

    /// How many valuations count in each calendar year: those of houses held at
    /// the end of that year (CSD-019 REQ-11).
    pub(super) fn counted_valuations_per_year(&self) -> BTreeMap<i32, usize> {
        let mut counts: BTreeMap<i32, usize> = BTreeMap::new();
        for valuation in &self.valuations {
            let year = valuation.period.year();
            let held = self
                .houses
                .iter()
                .any(|house| house.id == valuation.house_id && house.held_at_end_of(year));
            if held {
                *counts.entry(year).or_default() += 1;
            }
        }
        counts
    }

    /// For each house held at the end of `year` with a valuation in it, in
    /// creation order, the value of its latest valuation in that year.
    pub(super) fn year_segments(&self, year: i32) -> Vec<(&House, Decimal)> {
        self.houses
            .iter()
            .filter(|house| house.held_at_end_of(year))
            .filter_map(|house| {
                self.valuations
                    .iter()
                    .filter(|s| s.house_id == house.id && s.period.year() == year)
                    .max_by_key(|s| s.period.sort_key())
                    .map(|s| (house, s.value.value()))
            })
            .collect()
    }

    /// The change in `year`: for each house with a valuation in the year and an
    /// earlier one, its closing value minus its opening value.
    /// `None` when no house has an opening value (CSD-015 section 3).
    pub(super) fn year_change(&self, year: i32) -> Option<Decimal> {
        let mut change: Option<Decimal> = None;
        for (house, closing) in self.year_segments(year) {
            let opening = self
                .valuations
                .iter()
                .filter(|s| s.house_id == house.id && s.period.year() < year)
                .max_by_key(|s| s.period.sort_key())
                .map(|s| s.value.value());
            if let Some(opening) = opening {
                change = Some(change.unwrap_or_else(zero) + closing - opening);
            }
        }
        change
    }
}
