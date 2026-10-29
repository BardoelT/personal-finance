//! Yearly values over all assets (CSD-015 REQ-20).

use std::collections::BTreeMap;

use rust_decimal::Decimal;
use serde::Serialize;

use crate::domain::money::zero;

use super::{Asset, Assets};

/// The totals of one calendar year over all assets (CSD-015 REQ-20).
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct AssetYearlyValue {
    pub year: i32,
    /// The sum, over the assets with a valuation this year, of the value of
    /// their latest valuation this year.
    pub closing_value: Decimal,
    /// Closing value minus opening value, over the assets that have both;
    /// `None` when no asset has an opening value this year.
    pub change: Option<Decimal>,
    pub valuation_count: usize,
}

impl Assets {
    /// Totals per calendar year over all assets, newest year first, without
    /// extrapolating partial years or carrying values forward (REQ-20). A
    /// valuation counts in the year of its own period only.
    #[must_use]
    pub fn yearly_values(&self) -> Vec<AssetYearlyValue> {
        let mut counts: BTreeMap<i32, usize> = BTreeMap::new();
        for valuation in &self.valuations {
            *counts.entry(valuation.period.year()).or_default() += 1;
        }
        counts
            .into_iter()
            .rev()
            .map(|(year, valuation_count)| AssetYearlyValue {
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

    /// For each asset with a valuation in `year`, in creation order, the value
    /// of its latest valuation in that year.
    pub(super) fn year_segments(&self, year: i32) -> Vec<(&Asset, Decimal)> {
        self.assets
            .iter()
            .filter_map(|asset| {
                self.valuations
                    .iter()
                    .filter(|s| s.asset_id == asset.id && s.period.year() == year)
                    .max_by_key(|s| s.period.sort_key())
                    .map(|s| (asset, s.value.value()))
            })
            .collect()
    }

    /// The change in `year`: for each asset with a valuation in the year and an
    /// earlier one, its closing value minus its opening value.
    /// `None` when no asset has an opening value (CSD-015 section 3).
    pub(super) fn year_change(&self, year: i32) -> Option<Decimal> {
        let mut change: Option<Decimal> = None;
        for (asset, closing) in self.year_segments(year) {
            let opening = self
                .valuations
                .iter()
                .filter(|s| s.asset_id == asset.id && s.period.year() < year)
                .max_by_key(|s| s.period.sort_key())
                .map(|s| s.value.value());
            if let Some(opening) = opening {
                change = Some(change.unwrap_or_else(zero) + closing - opening);
            }
        }
        change
    }
}
