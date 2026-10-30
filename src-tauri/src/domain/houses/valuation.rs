//! One valuation of a house, its change, and the form values for "copy previous" (CSD-015 REQ-7, REQ-15, REQ-19).

use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

use crate::domain::error::DomainError;
use crate::domain::investments::{Note, Period};

use super::{HouseId, HouseValue, Houses};

/// The record of one house for one period (CSD-015 REQ-7).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct HouseValuation {
    #[serde(alias = "asset_id")]
    pub house_id: HouseId,
    pub period: Period,
    pub value: HouseValue,
    #[serde(default)]
    pub note: Note,
}

/// Parses `YYYY-MM` or `YYYY` typed by the user (CSD-015 REQ-7).
///
/// # Errors
/// Returns `HouseValuationPeriodInvalid` for any other text.
pub fn parse_period(text: &str) -> Result<Period, DomainError> {
    Period::parse(text).map_err(|_| DomainError::HouseValuationPeriodInvalid)
}

/// Form values for "copy previous month" or "copy previous year" (CSD-015
/// REQ-15): the note is never copied.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct ValuationPrefill {
    pub value: Option<Decimal>,
}

impl Houses {
    /// Form values for "copy previous month" or "copy previous year": the
    /// value of the latest valuation and no note (REQ-15).
    ///
    /// # Errors
    /// Returns `HouseNotFound`.
    pub fn prefill_from_latest(&self, id: HouseId) -> Result<ValuationPrefill, DomainError> {
        self.house(id)?;
        Ok(ValuationPrefill {
            value: self.latest(id).map(|valuation| valuation.value.value()),
        })
    }

    /// The change of a valuation: its value minus the value of the previous
    /// valuation of the house, or `None` for a first valuation (REQ-19). Derived
    /// each time, never stored.
    #[must_use]
    pub fn change(&self, valuation: &HouseValuation) -> Option<Decimal> {
        self.valuations
            .iter()
            .filter(|s| {
                s.house_id == valuation.house_id
                    && s.period.sort_key() < valuation.period.sort_key()
            })
            .max_by_key(|s| s.period.sort_key())
            .map(|previous| valuation.value.value() - previous.value.value())
    }
}
