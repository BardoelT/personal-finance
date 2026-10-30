//! The yearly series behind the Houses Insights chart (CSD-016).

use rust_decimal::Decimal;
use serde::Serialize;

use crate::domain::income::YearMonth;
use crate::domain::money::zero;

use super::{HouseId, Houses};

/// A house in the legend of the chart (CSD-016 REQ-9).
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct InsightsHouse {
    pub id: HouseId,
    pub name: String,
    pub colour_slot: u8,
}

/// The part of a year's bar that belongs to one house (CSD-016 REQ-5).
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct InsightsSegment {
    pub house_id: HouseId,
    pub value: Decimal,
}

/// One year of the insights chart (CSD-016 REQ-2).
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct HouseInsightsYear {
    pub year: i32,
    pub closing_value: Decimal,
    pub change: Option<Decimal>,
    pub valuation_count: usize,
    pub is_partial: bool,
    pub segments: Vec<InsightsSegment>,
}

/// The payload of `get_house_insights` (CSD-016 section 5).
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct HouseInsights {
    pub houses: Vec<InsightsHouse>,
    pub years: Vec<HouseInsightsYear>,
}

impl Houses {
    /// One figure set per calendar year from the first valuation up to and
    /// including the current year, oldest first, and the houses that have a
    /// segment in them, in creation order (CSD-016). Years without valuations
    /// are zero and have no segments. The current year is partial. Valuations
    /// after the current year, which only a project file can contain, are not
    /// shown.
    #[must_use]
    pub fn insights(&self, today: YearMonth) -> HouseInsights {
        let counts = self.counted_valuations_per_year();
        let Some(&first) = counts.keys().next() else {
            return HouseInsights {
                houses: Vec::new(),
                years: Vec::new(),
            };
        };
        let years: Vec<HouseInsightsYear> = (first..=today.year())
            .map(|year| {
                let segments: Vec<InsightsSegment> = self
                    .year_segments(year)
                    .into_iter()
                    .map(|(house, value)| InsightsSegment {
                        house_id: house.id,
                        value,
                    })
                    .collect();
                HouseInsightsYear {
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
        let houses = self
            .houses
            .iter()
            .filter(|house| {
                years.iter().any(|year| {
                    year.segments
                        .iter()
                        .any(|segment| segment.house_id == house.id)
                })
            })
            .map(|house| InsightsHouse {
                id: house.id,
                name: house.name.clone(),
                colour_slot: house.colour_slot,
            })
            .collect();
        HouseInsights { houses, years }
    }
}
