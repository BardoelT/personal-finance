//! The assets with their latest values and the total asset (CSD-015 REQ-17).

use rust_decimal::Decimal;
use serde::Serialize;

use crate::domain::investments::Period;

use super::{Asset, Assets};

/// One asset row of the Assets view: its latest value and period (CSD-015
/// REQ-17).
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct AssetSummary {
    #[serde(flatten)]
    pub asset: Asset,
    pub valuation_count: usize,
    pub latest_period: Option<Period>,
    pub latest_value: Option<Decimal>,
}

/// The assets with the total of their latest values (CSD-015 REQ-17). The
/// total is `None` while no asset has a valuation.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct AssetsOverview {
    pub assets: Vec<AssetSummary>,
    pub total: Option<Decimal>,
}

impl Assets {
    /// The assets with their latest values and the total asset (REQ-17).
    /// Periods are not aligned: each asset counts with its own latest valuation.
    #[must_use]
    pub fn overview(&self) -> AssetsOverview {
        let assets: Vec<AssetSummary> = self
            .assets()
            .into_iter()
            .map(|asset| {
                let latest = self.latest(asset.id);
                AssetSummary {
                    asset: asset.clone(),
                    valuation_count: self.valuation_count(asset.id),
                    latest_period: latest.map(|valuation| valuation.period),
                    latest_value: latest.map(|valuation| valuation.value.value()),
                }
            })
            .collect();
        let mut values = assets.iter().filter_map(|summary| summary.latest_value);
        let total = values
            .next()
            .map(|first| values.fold(first, |sum, value| sum + value));
        AssetsOverview { assets, total }
    }
}
