//! Debts: their id, name, statement period and colour slot (CSD-011 REQ-1 to REQ-6).

use std::fmt;

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::domain::error::DomainError;
use crate::domain::income::PayPeriod;

pub(super) const MAX_NAME_CHARS: usize = 100;

/// How many colours the Insights chart has (CSD-011 REQ-3, CSD-012 REQ-8).
pub(super) const COLOUR_SLOTS: u8 = 8;

/// Identifies a debt.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct DebtId(Uuid);

impl DebtId {
    pub(super) fn generate() -> Self {
        Self(Uuid::new_v4())
    }

    /// Parses the text form returned to the frontend.
    ///
    /// # Errors
    /// Returns `DebtNotFound` for text that is not a debt id.
    pub fn parse(text: &str) -> Result<Self, DomainError> {
        Uuid::parse_str(text.trim())
            .map(Self)
            .map_err(|_| DomainError::DebtNotFound)
    }
}

impl fmt::Display for DebtId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.0, f)
    }
}

/// One amount the user owes to one party (CSD-011 REQ-1).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Debt {
    pub id: DebtId,
    pub name: String,
    pub statement_period: PayPeriod,
    /// 0 to 7, fixed when the debt is created (CSD-011 REQ-3).
    pub colour_slot: u8,
}

/// Debt fields as typed by the user, before validation.
#[derive(Clone, Debug, Default, Deserialize)]
pub struct DebtInput {
    #[serde(default)]
    pub name: String,
    /// `None` means Monthly when creating and "unchanged" when editing.
    #[serde(default)]
    pub statement_period: Option<String>,
}

pub(super) struct ValidDebt {
    pub(super) name: String,
    pub(super) statement_period: Option<PayPeriod>,
}

impl DebtInput {
    pub(super) fn validate(&self) -> Result<ValidDebt, DomainError> {
        let name = self.name.trim();
        if name.is_empty() {
            return Err(DomainError::DebtNameRequired);
        }
        if name.chars().count() > MAX_NAME_CHARS {
            return Err(DomainError::DebtNameTooLong);
        }
        let statement_period = self
            .statement_period
            .as_deref()
            .map(str::trim)
            .filter(|text| !text.is_empty())
            .map(|text| PayPeriod::parse(text).ok_or(DomainError::DebtPeriodUnsupported))
            .transpose()?;
        Ok(ValidDebt {
            name: name.to_owned(),
            statement_period,
        })
    }
}
