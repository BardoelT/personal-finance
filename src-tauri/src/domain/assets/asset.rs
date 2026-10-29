//! Assets: their id, name, valuation period and colour slot (CSD-015 REQ-1 to REQ-6).

use std::fmt;

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::domain::error::DomainError;
use crate::domain::income::PayPeriod;

pub(super) const MAX_NAME_CHARS: usize = 100;

/// How many colours the Insights chart has (CSD-015 REQ-3, CSD-016 REQ-8).
pub(super) const COLOUR_SLOTS: u8 = 8;

/// Identifies an asset.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct AssetId(Uuid);

impl AssetId {
    pub(super) fn generate() -> Self {
        Self(Uuid::new_v4())
    }

    /// Parses the text form returned to the frontend.
    ///
    /// # Errors
    /// Returns `AssetNotFound` for text that is not an asset id.
    pub fn parse(text: &str) -> Result<Self, DomainError> {
        Uuid::parse_str(text.trim())
            .map(Self)
            .map_err(|_| DomainError::AssetNotFound)
    }
}

impl fmt::Display for AssetId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.0, f)
    }
}

/// One thing the user owns that has a value (CSD-015 REQ-1).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Asset {
    pub id: AssetId,
    pub name: String,
    pub valuation_period: PayPeriod,
    /// 0 to 7, fixed when the asset is created (CSD-015 REQ-3).
    pub colour_slot: u8,
}

/// Asset fields as typed by the user, before validation.
#[derive(Clone, Debug, Default, Deserialize)]
pub struct AssetInput {
    #[serde(default)]
    pub name: String,
    /// `None` means Monthly when creating and "unchanged" when editing.
    #[serde(default)]
    pub valuation_period: Option<String>,
}

pub(super) struct ValidAsset {
    pub(super) name: String,
    pub(super) valuation_period: Option<PayPeriod>,
}

impl AssetInput {
    pub(super) fn validate(&self) -> Result<ValidAsset, DomainError> {
        let name = self.name.trim();
        if name.is_empty() {
            return Err(DomainError::AssetNameRequired);
        }
        if name.chars().count() > MAX_NAME_CHARS {
            return Err(DomainError::AssetNameTooLong);
        }
        let valuation_period = self
            .valuation_period
            .as_deref()
            .map(str::trim)
            .filter(|text| !text.is_empty())
            .map(|text| PayPeriod::parse(text).ok_or(DomainError::AssetPeriodUnsupported))
            .transpose()?;
        Ok(ValidAsset {
            name: name.to_owned(),
            valuation_period,
        })
    }
}
