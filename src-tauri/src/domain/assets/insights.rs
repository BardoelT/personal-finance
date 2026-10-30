//! The yearly series behind the Assets Insights chart (CSD-016).

use rust_decimal::Decimal;
use serde::Serialize;

use crate::domain::income::YearMonth;
use crate::domain::money::zero;

use super::{AssetId, Assets};

/// An asset in the legend of the chart (CSD-016 REQ-9).
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct InsightsAsset {
    pub id: AssetId,
    pub name: String,
    pub colour_slot: u8,
}

/// The part of a year's bar that belongs to one asset (CSD-016 REQ-5).
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct InsightsSegment {
    pub asset_id: AssetId,
    pub value: Decimal,
}

/// One year of the insights chart (CSD-016 REQ-2).
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct AssetInsightsYear {
    pub year: i32,
    pub closing_value: Decimal,
    pub change: Option<Decimal>,
    pub valuation_count: usize,
    pub is_partial: bool,
    pub segments: Vec<InsightsSegment>,
}

/// The payload of `get_asset_insights` (CSD-016 section 5).
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct AssetInsights {
    pub assets: Vec<InsightsAsset>,
    pub years: Vec<AssetInsightsYear>,
}

impl Assets {
    /// One figure set per calendar year from the first valuation up to and
    /// including the current year, oldest first, and the assets that have a
    /// segment in them, in creation order (CSD-016). Years without valuations
    /// are zero and have no segments. The current year is partial. Valuations
    /// after the current year, which only a project file can contain, are not
    /// shown.
    #[must_use]
    pub fn insights(&self, today: YearMonth) -> AssetInsights {
        let counts = self.counted_valuations_per_year();
        let Some(&first) = counts.keys().next() else {
            return AssetInsights {
                assets: Vec::new(),
                years: Vec::new(),
            };
        };
        let years: Vec<AssetInsightsYear> = (first..=today.year())
            .map(|year| {
                let segments: Vec<InsightsSegment> = self
                    .year_segments(year)
                    .into_iter()
                    .map(|(asset, value)| InsightsSegment {
                        asset_id: asset.id,
                        value,
                    })
                    .collect();
                AssetInsightsYear {
                    year,
                    closing_value: segments
                        .iter()
                        .fold(zero(), |sum, segment| sum + segment.value),
                    change: self.year_change(year),
                    valuation_count: counts.get(&year).copied().unwrap_or(0),
                    is_partial: year == today.year(),
                    segments,
                }
            })
            .collect();
        let assets = self
            .assets
            .iter()
            .filter(|asset| {
                years.iter().any(|year| {
                    year.segments
                        .iter()
                        .any(|segment| segment.asset_id == asset.id)
                })
            })
            .map(|asset| InsightsAsset {
                id: asset.id,
                name: asset.name.clone(),
                colour_slot: asset.colour_slot,
            })
            .collect();
        AssetInsights { assets, years }
    }
}
