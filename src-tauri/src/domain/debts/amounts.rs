//! The balance and note of a statement (CSD-011 REQ-10, REQ-11).

use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

use crate::domain::error::DomainError;
use crate::domain::investments::{Note, MAX_BALANCE};
use crate::domain::money::{parse_plain_decimal, with_two_decimals};

/// What is still owed at the end of a period, in the project currency, from
/// 0.00 up, with exactly two decimals (CSD-011 REQ-10).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct DebtBalance(Decimal);

impl DebtBalance {
    /// Builds a balance from a decimal.
    ///
    /// # Errors
    /// Returns the first violated rule: negative, more than two decimals, or
    /// above 1,000,000,000.00. Zero is valid.
    pub fn new(value: Decimal) -> Result<Self, DomainError> {
        if value < Decimal::ZERO {
            return Err(DomainError::DebtStatementBalanceNegative);
        }
        if value.normalize().scale() > 2 {
            return Err(DomainError::DebtStatementBalancePrecision);
        }
        if value > MAX_BALANCE {
            return Err(DomainError::DebtStatementBalanceTooLarge);
        }
        Ok(Self(with_two_decimals(value)))
    }

    /// Parses a balance typed by the user.
    ///
    /// # Errors
    /// Returns `DebtStatementBalanceInvalid` for text that is not a plain
    /// decimal number, and otherwise the rules of [`DebtBalance::new`].
    pub fn parse(text: &str) -> Result<Self, DomainError> {
        parse_plain_decimal(text)
            .ok_or(DomainError::DebtStatementBalanceInvalid)
            .and_then(Self::new)
    }

    /// The balance as a decimal.
    #[must_use]
    pub fn value(self) -> Decimal {
        self.0
    }
}

/// Builds the note of a statement, at most 2,000 characters (CSD-011 REQ-11).
///
/// # Errors
/// Returns `DebtStatementNoteTooLong`.
pub fn debt_note(text: &str) -> Result<Note, DomainError> {
    Note::new(text).map_err(|_| DomainError::DebtStatementNoteTooLong)
}
