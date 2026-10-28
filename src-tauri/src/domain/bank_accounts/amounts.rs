//! The balance and note of an entry (CSD-009 REQ-10, REQ-11).

use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

use crate::domain::error::DomainError;
use crate::domain::investments::{Note, MAX_BALANCE};
use crate::domain::money::{parse_plain_decimal, with_two_decimals};

/// The closing balance of an entry, in the project currency, signed, with
/// exactly two decimals (CSD-009 REQ-10). Negative when the account is
/// overdrawn.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct BankBalance(Decimal);

impl BankBalance {
    /// Builds a balance from a decimal.
    ///
    /// # Errors
    /// Returns `BankEntryBalancePrecision` for more than two decimals and
    /// `BankEntryBalanceTooLarge` above 1,000,000,000.00 in absolute value.
    /// Zero and negative values are valid.
    pub fn new(value: Decimal) -> Result<Self, DomainError> {
        if value.normalize().scale() > 2 {
            return Err(DomainError::BankEntryBalancePrecision);
        }
        if value.abs() > MAX_BALANCE {
            return Err(DomainError::BankEntryBalanceTooLarge);
        }
        Ok(Self(with_two_decimals(value)))
    }

    /// Parses a balance typed by the user.
    ///
    /// # Errors
    /// Returns `BankEntryBalanceInvalid` for text that is not a plain decimal
    /// number, and otherwise the rules of [`BankBalance::new`].
    pub fn parse(text: &str) -> Result<Self, DomainError> {
        parse_plain_decimal(text)
            .ok_or(DomainError::BankEntryBalanceInvalid)
            .and_then(Self::new)
    }

    /// The balance as a decimal.
    #[must_use]
    pub fn value(self) -> Decimal {
        self.0
    }
}

/// Builds the note of an entry, at most 2,000 characters (CSD-009 REQ-11).
///
/// # Errors
/// Returns `BankEntryNoteTooLong`.
pub fn bank_note(text: &str) -> Result<Note, DomainError> {
    Note::new(text).map_err(|_| DomainError::BankEntryNoteTooLong)
}
