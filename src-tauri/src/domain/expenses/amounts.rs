//! The amount and note of an expense (CSD-013 REQ-5, REQ-6).

use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

use crate::domain::error::DomainError;
use crate::domain::investments::{Note, MAX_BALANCE};
use crate::domain::money::{parse_plain_decimal, with_two_decimals};

/// What an expense cost, in the project currency: above 0.00, with exactly two
/// decimals (CSD-013 REQ-5).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ExpenseAmount(Decimal);

impl ExpenseAmount {
    /// Builds an amount from a decimal.
    ///
    /// # Errors
    /// Returns the first violated rule: not above zero, more than two
    /// decimals, or above 1,000,000,000.00.
    pub fn new(value: Decimal) -> Result<Self, DomainError> {
        if value <= Decimal::ZERO {
            return Err(DomainError::ExpenseAmountNotPositive);
        }
        if value.normalize().scale() > 2 {
            return Err(DomainError::ExpenseAmountPrecision);
        }
        if value > MAX_BALANCE {
            return Err(DomainError::ExpenseAmountTooLarge);
        }
        Ok(Self(with_two_decimals(value)))
    }

    /// Parses an amount typed by the user.
    ///
    /// # Errors
    /// Returns `ExpenseAmountInvalid` for text that is not a plain decimal
    /// number, and otherwise the rules of [`ExpenseAmount::new`].
    pub fn parse(text: &str) -> Result<Self, DomainError> {
        parse_plain_decimal(text)
            .ok_or(DomainError::ExpenseAmountInvalid)
            .and_then(Self::new)
    }

    /// The amount as a decimal.
    #[must_use]
    pub fn value(self) -> Decimal {
        self.0
    }
}

/// Builds the note of an expense, at most 2,000 characters (CSD-013 REQ-6).
///
/// # Errors
/// Returns `ExpenseNoteTooLong`.
pub fn expense_note(text: &str) -> Result<Note, DomainError> {
    Note::new(text).map_err(|_| DomainError::ExpenseNoteTooLong)
}
