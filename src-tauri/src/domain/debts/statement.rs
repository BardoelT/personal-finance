//! One statement of a debt, its change, and the form values for "copy previous" (CSD-011 REQ-7, REQ-15, REQ-25).

use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

use crate::domain::error::DomainError;
use crate::domain::investments::{Note, Period};

use super::{DebtBalance, DebtId, Debts};

/// The record of one debt for one period (CSD-011 REQ-7).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DebtStatement {
    pub debt_id: DebtId,
    pub period: Period,
    pub balance: DebtBalance,
    #[serde(default)]
    pub note: Note,
}

/// Parses `YYYY-MM` or `YYYY` typed by the user (CSD-011 REQ-7).
///
/// # Errors
/// Returns `DebtStatementPeriodInvalid` for any other text.
pub fn parse_period(text: &str) -> Result<Period, DomainError> {
    Period::parse(text).map_err(|_| DomainError::DebtStatementPeriodInvalid)
}

/// Form values for "copy previous month" or "copy previous year" (CSD-011
/// REQ-15): the note is never copied.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct StatementPrefill {
    pub balance: Option<Decimal>,
}

impl Debts {
    /// Form values for "copy previous month" or "copy previous year": the
    /// balance of the latest statement and no note (REQ-15).
    ///
    /// # Errors
    /// Returns `DebtNotFound`.
    pub fn prefill_from_latest(&self, id: DebtId) -> Result<StatementPrefill, DomainError> {
        self.debt(id)?;
        Ok(StatementPrefill {
            balance: self.latest(id).map(|statement| statement.balance.value()),
        })
    }

    /// The change of a statement: the balance of the previous statement of the
    /// debt minus its own, or `None` for a first statement (REQ-25). Derived
    /// each time, never stored.
    #[must_use]
    pub fn change(&self, statement: &DebtStatement) -> Option<Decimal> {
        self.statements
            .iter()
            .filter(|s| {
                s.debt_id == statement.debt_id && s.period.sort_key() < statement.period.sort_key()
            })
            .max_by_key(|s| s.period.sort_key())
            .map(|previous| previous.balance.value() - statement.balance.value())
    }
}
