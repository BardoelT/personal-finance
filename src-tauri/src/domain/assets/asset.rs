//! Assets: their id, name, valuation period, colour slot (CSD-015 REQ-1 to
//! REQ-6) and purchase and sale (CSD-019 REQ-1 to REQ-5).

use std::fmt;

use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use time::Date;
use uuid::Uuid;

use crate::domain::error::DomainError;
use crate::domain::income::PayPeriod;
use crate::domain::investments::Period;

use super::{AssetDate, AssetValue};

pub(super) const MAX_NAME_CHARS: usize = 100;

/// How many colours the Insights chart has (CSD-015 REQ-3, CSD-016 REQ-8).
pub(super) const COLOUR_SLOTS: u8 = 8;

/// Identifies an asset.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct AssetId(Uuid);

impl AssetId {
    pub(super) fn generate() -> Self {
        Self(Uuid::new_v4())
    }

    /// Parses the text form returned to the frontend.
    ///
    /// # Errors
    /// Returns `AssetNotFound` for text that is not an asset id.
    pub fn parse(text: &str) -> Result<Self, DomainError> {
        Uuid::parse_str(text.trim())
            .map(Self)
            .map_err(|_| DomainError::AssetNotFound)
    }
}

impl fmt::Display for AssetId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.0, f)
    }
}

/// One thing the user owns that has a value (CSD-015 REQ-1).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Asset {
    pub id: AssetId,
    pub name: String,
    pub valuation_period: PayPeriod,
    /// 0 to 7, fixed when the asset is created (CSD-015 REQ-3).
    pub colour_slot: u8,
    /// `None` only for an asset from a file made before CSD-019 (REQ-6).
    #[serde(default)]
    pub purchase_date: Option<AssetDate>,
    #[serde(default)]
    pub purchase_amount: Option<AssetValue>,
    /// The sale: both fields or neither (CSD-019 REQ-5).
    #[serde(default)]
    pub sell_date: Option<AssetDate>,
    #[serde(default)]
    pub sell_amount: Option<AssetValue>,
}

impl Asset {
    /// Whether the asset has no sale (CSD-019 section 3).
    #[must_use]
    pub fn is_held(&self) -> bool {
        self.sell_date.is_none()
    }

    /// The sell amount minus the purchase amount of a sold asset (CSD-019
    /// REQ-9). It is derived and never stored.
    #[must_use]
    pub fn result(&self) -> Option<Decimal> {
        Some(self.sell_amount?.value() - self.purchase_amount?.value())
    }

    /// Whether the asset was purchased on or before 31/12 of `year` and not
    /// sold on or before it (CSD-019 REQ-11). An asset without a purchase
    /// counts as purchased before any year.
    pub(super) fn held_at_end_of(&self, year: i32) -> bool {
        self.purchase_date.is_none_or(|date| date.year() <= year)
            && self.sell_date.is_none_or(|date| date.year() > year)
    }

    /// Whether `period` lies in the holding window: not before the period of
    /// the purchase date and not after the period of the sell date, compared
    /// in the month or year of the period (CSD-019 REQ-7).
    pub(super) fn window_contains(&self, period: Period) -> bool {
        let key = |date: AssetDate| match period {
            Period::Month(_) => (date.year(), date.month()),
            Period::Year(_) => (date.year(), 0),
        };
        let own = match period {
            Period::Month(_) => period.sort_key(),
            Period::Year(_) => (period.year(), 0),
        };
        self.purchase_date.is_none_or(|date| own >= key(date))
            && self.sell_date.is_none_or(|date| own <= key(date))
    }
}

/// Asset fields as typed by the user, before validation.
#[derive(Clone, Debug, Default, Deserialize)]
pub struct AssetInput {
    #[serde(default)]
    pub name: String,
    /// `None` means Monthly when creating and "unchanged" when editing.
    #[serde(default)]
    pub valuation_period: Option<String>,
    /// `dd/mm/yyyy`; required (CSD-019 REQ-1).
    #[serde(default)]
    pub purchase_date: Option<String>,
    #[serde(default)]
    pub purchase_amount: Option<String>,
    /// Both or neither (CSD-019 REQ-5).
    #[serde(default)]
    pub sell_date: Option<String>,
    #[serde(default)]
    pub sell_amount: Option<String>,
}

pub(super) struct ValidAsset {
    pub(super) name: String,
    pub(super) valuation_period: Option<PayPeriod>,
    pub(super) purchase_date: AssetDate,
    pub(super) purchase_amount: AssetValue,
    pub(super) sale: Option<(AssetDate, AssetValue)>,
}

/// The text of an optional field, trimmed, `None` when empty.
fn filled(text: Option<&str>) -> Option<&str> {
    text.map(str::trim).filter(|text| !text.is_empty())
}

impl AssetInput {
    /// Validates the input; `today` is the current date, the latest a purchase
    /// or sell date may be (CSD-019 REQ-1 to REQ-5).
    pub(super) fn validate(&self, today: Date) -> Result<ValidAsset, DomainError> {
        let name = self.name.trim();
        if name.is_empty() {
            return Err(DomainError::AssetNameRequired);
        }
        if name.chars().count() > MAX_NAME_CHARS {
            return Err(DomainError::AssetNameTooLong);
        }
        let valuation_period = self
            .valuation_period
            .as_deref()
            .map(str::trim)
            .filter(|text| !text.is_empty())
            .map(|text| PayPeriod::parse(text).ok_or(DomainError::AssetPeriodUnsupported))
            .transpose()?;
        let purchase_date = filled(self.purchase_date.as_deref())
            .ok_or(DomainError::AssetPurchaseDateRequired)
            .and_then(|text| AssetDate::parse(text, today))?;
        let purchase_amount = filled(self.purchase_amount.as_deref())
            .ok_or(DomainError::AssetPurchaseAmountRequired)
            .and_then(AssetValue::parse_amount)?;
        let sale = match (
            filled(self.sell_date.as_deref()),
            filled(self.sell_amount.as_deref()),
        ) {
            (None, None) => None,
            (Some(date), Some(amount)) => {
                let date = AssetDate::parse(date, today)?;
                if date < purchase_date {
                    return Err(DomainError::AssetSellBeforePurchase);
                }
                Some((date, AssetValue::parse_amount(amount)?))
            }
            _ => return Err(DomainError::AssetSaleIncomplete),
        };
        Ok(ValidAsset {
            name: name.to_owned(),
            valuation_period,
            purchase_date,
            purchase_amount,
            sale,
        })
    }
}
