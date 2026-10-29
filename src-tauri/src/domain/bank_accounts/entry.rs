//! One entry of an account, and the form values for "copy previous" (CSD-009 REQ-7, REQ-15).

use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

use crate::domain::error::DomainError;
use crate::domain::investments::{Note, Period};

use super::{BankAccountId, BankAccounts, BankBalance};

/// The record of one account for one period (CSD-009 REQ-7).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct BankEntry {
    pub account_id: BankAccountId,
    pub period: Period,
    pub balance: BankBalance,
    #[serde(default)]
    pub note: Note,
}

/// Parses `YYYY-MM` or `YYYY` typed by the user (CSD-009 REQ-7).
///
/// # Errors
/// Returns `BankEntryPeriodInvalid` for any other text.
pub fn parse_period(text: &str) -> Result<Period, DomainError> {
    Period::parse(text).map_err(|_| DomainError::BankEntryPeriodInvalid)
}

/// Form values for "copy previous month" or "copy previous year" (CSD-009
/// REQ-15): the note is never copied.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct EntryPrefill {
    pub balance: Option<Decimal>,
}

impl BankAccounts {
    /// Form values for "copy previous month" or "copy previous year": the
    /// balance of the latest entry and no note (REQ-15).
    ///
    /// # Errors
    /// Returns `BankAccountNotFound`.
    pub fn prefill_from_latest(&self, id: BankAccountId) -> Result<EntryPrefill, DomainError> {
        self.account(id)?;
        Ok(EntryPrefill {
            balance: self.latest(id).map(|entry| entry.balance.value()),
        })
    }
}
