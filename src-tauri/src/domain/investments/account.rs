//! Investment accounts: their id, name and statement period (CSD-005 REQ-1 to REQ-5).

use std::fmt;

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::domain::error::DomainError;
use crate::domain::income::PayPeriod;
use crate::domain::series::MAX_NAME_CHARS;

/// Identifies an investment account.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct InvestmentAccountId(Uuid);

impl InvestmentAccountId {
    pub(super) fn generate() -> Self {
        Self(Uuid::new_v4())
    }

    /// Parses the text form returned to the frontend.
    ///
    /// # Errors
    /// Returns `InvestmentAccountNotFound` for text that is not an account id.
    pub fn parse(text: &str) -> Result<Self, DomainError> {
        Uuid::parse_str(text.trim())
            .map(Self)
            .map_err(|_| DomainError::InvestmentAccountNotFound)
    }
}

impl fmt::Display for InvestmentAccountId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.0, f)
    }
}

/// One place where the user holds investments (CSD-005 REQ-1).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct InvestmentAccount {
    pub id: InvestmentAccountId,
    pub name: String,
    pub statement_period: PayPeriod,
}

/// Account fields as typed by the user, before validation.
#[derive(Clone, Debug, Default, Deserialize)]
pub struct AccountInput {
    #[serde(default)]
    pub name: String,
    /// `None` means Monthly when creating and "unchanged" when editing.
    #[serde(default)]
    pub statement_period: Option<String>,
}

pub(super) struct ValidAccount {
    pub(super) name: String,
    pub(super) statement_period: Option<PayPeriod>,
}

impl AccountInput {
    pub(super) fn validate(&self) -> Result<ValidAccount, DomainError> {
        let name = self.name.trim();
        if name.is_empty() {
            return Err(DomainError::InvestmentAccountNameRequired);
        }
        if name.chars().count() > MAX_NAME_CHARS {
            return Err(DomainError::InvestmentAccountNameTooLong);
        }
        let statement_period = self
            .statement_period
            .as_deref()
            .map(str::trim)
            .filter(|text| !text.is_empty())
            .map(|text| PayPeriod::parse(text).ok_or(DomainError::InvestmentPeriodUnsupported))
            .transpose()?;
        Ok(ValidAccount {
            name: name.to_owned(),
            statement_period,
        })
    }
}
