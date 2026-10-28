//! The amounts and text of a statement: balance, net deposits and note (CSD-005 REQ-9, REQ-10, REQ-22).

use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

use crate::domain::error::DomainError;
use crate::domain::money::{parse_plain_decimal, with_two_decimals, zero};

const MAX_NOTE_CHARS: usize = 2000;

/// Largest balance accepted (1,000,000,000.00), a guard against typos.
pub const MAX_BALANCE: Decimal = Decimal::from_parts(1_215_752_192, 23, 0, false, 2);

/// The closing balance of a statement, in the project currency, with exactly
/// two decimals (CSD-005 REQ-9).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Balance(Decimal);

impl Balance {
    /// Builds a balance from a decimal.
    ///
    /// # Errors
    /// Returns the first violated rule: negative, more than two decimals, or
    /// above 1,000,000,000.00. Zero is valid.
    pub fn new(value: Decimal) -> Result<Self, DomainError> {
        if value < Decimal::ZERO {
            return Err(DomainError::InvestmentBalanceNegative);
        }
        if value.normalize().scale() > 2 {
            return Err(DomainError::InvestmentBalancePrecision);
        }
        if value > MAX_BALANCE {
            return Err(DomainError::InvestmentBalanceTooLarge);
        }
        Ok(Self(with_two_decimals(value)))
    }

    /// Parses a balance typed by the user.
    ///
    /// # Errors
    /// Returns `InvestmentBalanceInvalid` for text that is not a plain decimal
    /// number, and otherwise the rules of [`Balance::new`].
    pub fn parse(text: &str) -> Result<Self, DomainError> {
        parse_plain_decimal(text)
            .ok_or(DomainError::InvestmentBalanceInvalid)
            .and_then(Self::new)
    }

    /// The balance as a decimal.
    #[must_use]
    pub fn value(self) -> Decimal {
        self.0
    }
}

/// Checks a signed amount: at most two decimals and an absolute value up to
/// 1,000,000,000.00 (CSD-005 REQ-22, REQ-25).
fn signed_amount(
    value: Decimal,
    precision: DomainError,
    too_large: DomainError,
) -> Result<Decimal, DomainError> {
    if value.normalize().scale() > 2 {
        return Err(precision);
    }
    if value.abs() > MAX_BALANCE {
        return Err(too_large);
    }
    Ok(with_two_decimals(value))
}

/// The net deposits of a statement: what the user put in minus what was taken
/// out during the period, signed, with exactly two decimals (CSD-005 REQ-22).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct NetDeposits(Decimal);

impl Default for NetDeposits {
    fn default() -> Self {
        Self(zero())
    }
}

impl NetDeposits {
    /// Builds net deposits from a decimal.
    ///
    /// # Errors
    /// Returns `InvestmentNetDepositsPrecision` for more than two decimals and
    /// `InvestmentNetDepositsTooLarge` above 1,000,000,000.00 in absolute value.
    pub fn new(value: Decimal) -> Result<Self, DomainError> {
        signed_amount(
            value,
            DomainError::InvestmentNetDepositsPrecision,
            DomainError::InvestmentNetDepositsTooLarge,
        )
        .map(Self)
    }

    /// Parses net deposits typed by the user. Empty text means `0.00`.
    ///
    /// # Errors
    /// Returns `InvestmentNetDepositsInvalid` for text that is not a plain
    /// decimal number, and otherwise the rules of [`NetDeposits::new`].
    pub fn parse(text: &str) -> Result<Self, DomainError> {
        if text.trim().is_empty() {
            return Ok(Self::default());
        }
        parse_plain_decimal(text)
            .ok_or(DomainError::InvestmentNetDepositsInvalid)
            .and_then(Self::new)
    }

    /// The net deposits as a decimal.
    #[must_use]
    pub fn value(self) -> Decimal {
        self.0
    }
}

/// Parses a capital growth typed by the user (CSD-005 REQ-25).
pub(super) fn parse_growth(text: &str) -> Result<Decimal, DomainError> {
    let value = parse_plain_decimal(text).ok_or(DomainError::InvestmentGrowthInvalid)?;
    signed_amount(
        value,
        DomainError::InvestmentGrowthPrecision,
        DomainError::InvestmentGrowthTooLarge,
    )
}

/// The optional plain-text note of a statement, at most 2,000 characters
/// (CSD-005 REQ-10). It is kept exactly as typed.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Note(String);

impl Note {
    /// Builds a note.
    ///
    /// # Errors
    /// Returns `InvestmentNoteTooLong` above 2,000 characters (CON-8).
    pub fn new(text: &str) -> Result<Self, DomainError> {
        if text.chars().count() > MAX_NOTE_CHARS {
            return Err(DomainError::InvestmentNoteTooLong);
        }
        Ok(Self(text.to_owned()))
    }

    /// The note text.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}
