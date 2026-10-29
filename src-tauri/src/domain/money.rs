use std::str::FromStr;

use rust_decimal::{Decimal, RoundingStrategy};
use serde::{Deserialize, Serialize};

use super::error::DomainError;

/// Largest amount accepted per field (1,000,000.00), a guard against typos.
pub const MAX_AMOUNT: Decimal = Decimal::from_parts(100_000_000, 0, 0, false, 2);

/// Longest accepted amount text, a bound on untrusted input.
const MAX_AMOUNT_TEXT_LEN: usize = 32;

/// Currencies a project can use (CSD-003).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum Currency {
    #[default]
    Eur,
    Usd,
    Gbp,
}

impl Currency {
    /// Parses a currency code such as `"EUR"` (case-insensitive).
    #[must_use]
    pub fn parse(text: &str) -> Option<Self> {
        match text.trim().to_ascii_uppercase().as_str() {
            "EUR" => Some(Self::Eur),
            "USD" => Some(Self::Usd),
            "GBP" => Some(Self::Gbp),
            _ => None,
        }
    }
}

/// The three validated amounts of one paycheck, all with exactly two decimals.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Amounts {
    gross_salary: Decimal,
    holiday_pay: Decimal,
    bonus: Decimal,
}

impl Amounts {
    /// Builds amounts from decimals, applying the rules of CSD-001 REQ-7.
    ///
    /// # Errors
    /// Returns the first violated rule: negative, more than two decimals,
    /// above 1,000,000.00, or gross salary not greater than zero.
    pub fn new(
        gross_salary: Decimal,
        holiday_pay: Decimal,
        bonus: Decimal,
    ) -> Result<Self, DomainError> {
        let all = [gross_salary, holiday_pay, bonus];
        if all.iter().any(|amount| *amount < Decimal::ZERO) {
            return Err(DomainError::PaycheckAmountNegative);
        }
        if all.iter().any(|amount| amount.normalize().scale() > 2) {
            return Err(DomainError::PaycheckAmountPrecision);
        }
        if all.iter().any(|amount| *amount > MAX_AMOUNT) {
            return Err(DomainError::PaycheckAmountTooLarge);
        }
        if gross_salary <= Decimal::ZERO {
            return Err(DomainError::PaycheckGrossInvalid);
        }
        Ok(Self {
            gross_salary: with_two_decimals(gross_salary),
            holiday_pay: with_two_decimals(holiday_pay),
            bonus: with_two_decimals(bonus),
        })
    }

    /// Parses amounts typed by the user. Empty holiday pay or bonus mean `0.00`.
    ///
    /// # Errors
    /// An empty gross salary gives `PaycheckGrossInvalid`, text that is not a
    /// plain decimal number gives `PaycheckAmountInvalid`, and otherwise the
    /// rules of [`Amounts::new`] apply.
    pub fn parse(gross_salary: &str, holiday_pay: &str, bonus: &str) -> Result<Self, DomainError> {
        if gross_salary.trim().is_empty() {
            return Err(DomainError::PaycheckGrossInvalid);
        }
        Self::new(
            parse_decimal(gross_salary)?,
            parse_optional_decimal(holiday_pay)?,
            parse_optional_decimal(bonus)?,
        )
    }

    /// Gross salary of the month.
    #[must_use]
    pub fn gross_salary(&self) -> Decimal {
        self.gross_salary
    }

    /// Holiday pay paid out in the month.
    #[must_use]
    pub fn holiday_pay(&self) -> Decimal {
        self.holiday_pay
    }

    /// Bonus paid out in the month.
    #[must_use]
    pub fn bonus(&self) -> Decimal {
        self.bonus
    }

    /// Gross salary + holiday pay + bonus (CSD-001 REQ-10).
    #[must_use]
    pub fn total_gross(&self) -> Decimal {
        self.gross_salary + self.holiday_pay + self.bonus
    }
}

/// A zero amount with two decimals.
#[must_use]
pub fn zero() -> Decimal {
    with_two_decimals(Decimal::ZERO)
}

pub(super) fn with_two_decimals(mut amount: Decimal) -> Decimal {
    if amount.is_zero() {
        // Drops the sign of a negative zero such as "-0.00".
        return Decimal::new(0, 2);
    }
    amount.rescale(2);
    amount
}

fn parse_optional_decimal(text: &str) -> Result<Decimal, DomainError> {
    if text.trim().is_empty() {
        Ok(zero())
    } else {
        parse_decimal(text)
    }
}

/// Accepts only `[-]digits[.digits]`, so no separators, exponents or signs
/// that `Decimal::from_str` would otherwise tolerate.
fn parse_decimal(text: &str) -> Result<Decimal, DomainError> {
    parse_plain_decimal(text).ok_or(DomainError::PaycheckAmountInvalid)
}

/// The plain-number rule of [`parse_decimal`] without an error type, for
/// capabilities that report their own error code (CSD-005 REQ-9).
pub(super) fn parse_plain_decimal(text: &str) -> Option<Decimal> {
    let text = text.trim();
    if text.len() > MAX_AMOUNT_TEXT_LEN {
        return None;
    }
    let unsigned = text.strip_prefix('-').unwrap_or(text);
    let (whole, fraction) = unsigned.split_once('.').unwrap_or((unsigned, ""));
    let digits_only = |part: &str| part.bytes().all(|byte| byte.is_ascii_digit());
    let well_formed = !whole.is_empty()
        && digits_only(whole)
        && digits_only(fraction)
        && (fraction.is_empty() != unsigned.contains('.'));
    if !well_formed {
        return None;
    }
    Decimal::from_str(text).ok()
}

/// `part` in percent of `whole`, to one decimal with banker's rounding (CSD-007
/// REQ-6, CSD-018 CON-3). `None` when `whole` is zero.
#[must_use]
pub fn percent_of(part: Decimal, whole: Decimal) -> Option<Decimal> {
    let percent = part.checked_mul(Decimal::ONE_HUNDRED)?.checked_div(whole)?;
    let mut rounded = percent.round_dp_with_strategy(1, RoundingStrategy::MidpointNearestEven);
    if rounded.is_zero() {
        // A decrease that rounds to zero shows no sign.
        rounded = Decimal::ZERO;
    }
    rounded.rescale(1);
    Some(rounded)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn max_amount_is_one_million_with_two_decimals() {
        assert_eq!(MAX_AMOUNT.to_string(), "1000000.00");
    }

    #[test]
    fn parse_rejects_non_plain_numbers() {
        for text in [
            "abc", "1e3", "1_000", "3000,50", "--1", "1.", ".5", "+5", "1.2.3",
        ] {
            assert_eq!(
                Amounts::parse("3000.00", text, "").unwrap_err(),
                DomainError::PaycheckAmountInvalid,
                "holiday text {text:?}"
            );
        }
    }

    #[test]
    fn blank_gross_is_gross_invalid_and_blank_optionals_are_zero() {
        assert_eq!(
            Amounts::parse("  ", "", "").unwrap_err(),
            DomainError::PaycheckGrossInvalid
        );
        let amounts = Amounts::parse("1.00", " ", " ").unwrap();
        assert_eq!(amounts.total_gross().to_string(), "1.00");
    }

    #[test]
    fn parse_accepts_whole_numbers_and_normalises_to_two_decimals() {
        let amounts = Amounts::parse("3000", "", "250.5").unwrap();
        assert_eq!(amounts.gross_salary().to_string(), "3000.00");
        assert_eq!(amounts.holiday_pay().to_string(), "0.00");
        assert_eq!(amounts.bonus().to_string(), "250.50");
    }

    #[test]
    fn trailing_zeros_beyond_two_decimals_are_accepted() {
        let amounts = Amounts::parse("3000.500", "", "").unwrap();
        assert_eq!(amounts.gross_salary().to_string(), "3000.50");
    }

    #[test]
    fn negative_zero_is_not_negative() {
        let amounts = Amounts::parse("3000.00", "-0.00", "").unwrap();
        assert_eq!(amounts.holiday_pay().to_string(), "0.00");
    }
}
