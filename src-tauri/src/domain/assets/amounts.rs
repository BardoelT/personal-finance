//! The value and note of a valuation (CSD-015 REQ-10, REQ-11).

use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

use crate::domain::error::DomainError;
use crate::domain::investments::{Note, MAX_BALANCE};
use crate::domain::money::{parse_plain_decimal, with_two_decimals};

/// What the asset is worth at the end of a period, in the project currency, from
/// 0.00 up, with exactly two decimals (CSD-015 REQ-10).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct AssetValue(Decimal);

impl AssetValue {
    /// Builds a value from a decimal.
    ///
    /// # Errors
    /// Returns the first violated rule: negative, more than two decimals, or
    /// above 1,000,000,000.00. Zero is valid.
    pub fn new(value: Decimal) -> Result<Self, DomainError> {
        if value < Decimal::ZERO {
            return Err(DomainError::AssetValuationValueNegative);
        }
        if value.normalize().scale() > 2 {
            return Err(DomainError::AssetValuationValuePrecision);
        }
        if value > MAX_BALANCE {
            return Err(DomainError::AssetValuationValueTooLarge);
        }
        Ok(Self(with_two_decimals(value)))
    }

    /// Parses a value typed by the user.
    ///
    /// # Errors
    /// Returns `AssetValuationValueInvalid` for text that is not a plain
    /// decimal number, and otherwise the rules of [`AssetValue::new`].
    pub fn parse(text: &str) -> Result<Self, DomainError> {
        parse_plain_decimal(text)
            .ok_or(DomainError::AssetValuationValueInvalid)
            .and_then(Self::new)
    }

    /// The value as a decimal.
    #[must_use]
    pub fn value(self) -> Decimal {
        self.0
    }
}

/// Builds the note of a valuation, at most 2,000 characters (CSD-015 REQ-11).
///
/// # Errors
/// Returns `AssetValuationNoteTooLong`.
pub fn asset_note(text: &str) -> Result<Note, DomainError> {
    Note::new(text).map_err(|_| DomainError::AssetValuationNoteTooLong)
}
