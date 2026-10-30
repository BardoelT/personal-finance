//! Bank accounts: their id, name, entry period and colour slot (CSD-009 REQ-1 to REQ-6).

use std::fmt;

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::domain::error::DomainError;
use crate::domain::income::PayPeriod;
use crate::domain::series::MAX_NAME_CHARS;

/// Identifies a bank account.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct BankAccountId(Uuid);

impl BankAccountId {
    pub(super) fn generate() -> Self {
        Self(Uuid::new_v4())
    }

    /// Parses the text form returned to the frontend.
    ///
    /// # Errors
    /// Returns `BankAccountNotFound` for text that is not an account id.
    pub fn parse(text: &str) -> Result<Self, DomainError> {
        Uuid::parse_str(text.trim())
            .map(Self)
            .map_err(|_| DomainError::BankAccountNotFound)
    }
}

impl fmt::Display for BankAccountId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.0, f)
    }
}

/// One place where the user keeps money at a bank (CSD-009 REQ-1).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct BankAccount {
    pub id: BankAccountId,
    pub name: String,
    pub entry_period: PayPeriod,
    /// 0 to 7, fixed when the account is created (CSD-009 REQ-3).
    pub colour_slot: u8,
}

/// Account fields as typed by the user, before validation.
#[derive(Clone, Debug, Default, Deserialize)]
pub struct AccountInput {
    #[serde(default)]
    pub name: String,
    /// `None` means Monthly when creating and "unchanged" when editing.
    #[serde(default)]
    pub entry_period: Option<String>,
}

pub(super) struct ValidAccount {
    pub(super) name: String,
    pub(super) entry_period: Option<PayPeriod>,
}

impl AccountInput {
    pub(super) fn validate(&self) -> Result<ValidAccount, DomainError> {
        let name = self.name.trim();
        if name.is_empty() {
            return Err(DomainError::BankAccountNameRequired);
        }
        if name.chars().count() > MAX_NAME_CHARS {
            return Err(DomainError::BankAccountNameTooLong);
        }
        let entry_period = self
            .entry_period
            .as_deref()
            .map(str::trim)
            .filter(|text| !text.is_empty())
            .map(|text| PayPeriod::parse(text).ok_or(DomainError::BankAccountPeriodUnsupported))
            .transpose()?;
        Ok(ValidAccount {
            name: name.to_owned(),
            entry_period,
        })
    }
}
