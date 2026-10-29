//! Assets and their monthly or yearly valuations (CSD-015), and the yearly
//! totals and insights built on them (CSD-015 REQ-20, CSD-016).
//!
//! `Assets` holds the data and the rules for changing it; the other files add
//! the value types (`amounts`, `asset`, `valuation`) and the figures derived
//! from it (`overview`, `totals`, `insights`). The period and note types are
//! those of the investments (CSD-005).

mod amounts;
mod asset;
mod insights;
mod overview;
mod totals;
mod valuation;

#[cfg(test)]
mod tests;

pub use amounts::{asset_note, AssetValue};
pub use asset::{Asset, AssetId, AssetInput};
pub use insights::AssetInsights;
pub use overview::AssetsOverview;
pub use totals::AssetYearlyValue;
pub use valuation::{parse_period, AssetValuation, ValuationPrefill};

use std::collections::HashSet;

use serde::{Deserialize, Serialize};

use crate::domain::error::DomainError;
use crate::domain::income::YearMonth;
use crate::domain::investments::{Note, Period};

use asset::{COLOUR_SLOTS, MAX_NAME_CHARS};

/// All assets with their valuations. Assets are kept in the order they were
/// created (CSD-015 section 5).
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Assets {
    #[serde(default)]
    assets: Vec<Asset>,
    #[serde(default)]
    valuations: Vec<AssetValuation>,
}

impl Assets {
    /// Creates an asset, monthly unless the input says Yearly, with the colour
    /// slot of REQ-3 (REQ-1, REQ-2).
    ///
    /// # Errors
    /// Returns the name errors of REQ-2 or `AssetPeriodUnsupported`.
    pub fn create_asset(&mut self, input: &AssetInput) -> Result<Asset, DomainError> {
        let valid = input.validate()?;
        if self.name_taken(&valid.name, None) {
            return Err(DomainError::AssetNameDuplicate);
        }
        let asset = Asset {
            id: AssetId::generate(),
            name: valid.name,
            valuation_period: valid.valuation_period.unwrap_or_default(),
            colour_slot: self.next_colour_slot(),
        };
        self.assets.push(asset.clone());
        Ok(asset)
    }

    /// Changes the name and valuation period of an asset. The valuations and the
    /// colour slot stay; the period is locked once the asset has any valuation
    /// (REQ-4, REQ-5).
    ///
    /// # Errors
    /// Returns a validation error, `AssetNotFound`, `AssetNameDuplicate` or
    /// `AssetPeriodLocked`.
    pub fn update_asset(&mut self, id: AssetId, input: &AssetInput) -> Result<Asset, DomainError> {
        let valid = input.validate()?;
        let current = self.asset(id)?.valuation_period;
        if self.name_taken(&valid.name, Some(id)) {
            return Err(DomainError::AssetNameDuplicate);
        }
        let valuation_period = valid.valuation_period.unwrap_or(current);
        if valuation_period != current && self.valuation_count(id) > 0 {
            return Err(DomainError::AssetPeriodLocked);
        }
        let asset = self.asset_mut(id)?;
        asset.name = valid.name;
        asset.valuation_period = valuation_period;
        Ok(asset.clone())
    }

    /// Removes an asset and all its valuations (REQ-6).
    ///
    /// # Errors
    /// Returns `AssetNotFound`.
    pub fn delete_asset(&mut self, id: AssetId) -> Result<(), DomainError> {
        self.asset(id)?;
        self.assets.retain(|asset| asset.id != id);
        self.valuations.retain(|valuation| valuation.asset_id != id);
        Ok(())
    }

    /// Assets alphabetically by name, ignoring case (REQ-18).
    #[must_use]
    pub fn assets(&self) -> Vec<&Asset> {
        let mut assets: Vec<&Asset> = self.assets.iter().collect();
        assets.sort_by_cached_key(|asset| asset_order(asset));
        assets
    }

    /// How many valuations an asset has.
    #[must_use]
    pub fn valuation_count(&self, id: AssetId) -> usize {
        self.valuations
            .iter()
            .filter(|valuation| valuation.asset_id == id)
            .count()
    }

    /// How many valuations all assets have together.
    #[must_use]
    pub fn total_valuation_count(&self) -> usize {
        self.valuations.len()
    }

    /// Adds the valuation of a period. `today` is the current month; later
    /// periods are rejected. Any earlier period is accepted (REQ-7 to REQ-9).
    ///
    /// # Errors
    /// Returns `AssetNotFound`, `AssetPeriodMismatch`, `AssetValuationInFuture`
    /// or `AssetValuationDuplicate`.
    pub fn add_valuation(
        &mut self,
        asset_id: AssetId,
        period: Period,
        value: AssetValue,
        note: Note,
        today: YearMonth,
    ) -> Result<AssetValuation, DomainError> {
        self.check_new_period(asset_id, period, today)?;
        let valuation = AssetValuation {
            asset_id,
            period,
            value,
            note,
        };
        self.valuations.push(valuation.clone());
        Ok(valuation)
    }

    /// Changes the value and note of a valuation and, when `new_period` is a
    /// different period, moves it there. The new period follows the rules of
    /// adding a valuation: the kind of the asset, not after `today` and not
    /// taken by another valuation of the asset. The edit is applied as a whole
    /// or not at all (REQ-12, REQ-13).
    ///
    /// # Errors
    /// Returns `AssetNotFound`, `AssetValuationNotFound`, `AssetPeriodMismatch`,
    /// `AssetValuationInFuture` or `AssetValuationDuplicate`.
    pub fn update_valuation(
        &mut self,
        asset_id: AssetId,
        period: Period,
        new_period: Period,
        value: AssetValue,
        note: Note,
        today: YearMonth,
    ) -> Result<AssetValuation, DomainError> {
        self.asset(asset_id)?;
        if self.valuation(asset_id, period).is_none() {
            return Err(DomainError::AssetValuationNotFound);
        }
        if new_period != period {
            self.check_new_period(asset_id, new_period, today)?;
        }
        let valuation = self
            .valuations
            .iter_mut()
            .find(|s| s.asset_id == asset_id && s.period == period)
            .ok_or(DomainError::AssetValuationNotFound)?;
        valuation.period = new_period;
        valuation.value = value;
        valuation.note = note;
        Ok(valuation.clone())
    }

    /// Removes one valuation and nothing else (REQ-14). The caller asks for
    /// the confirmation.
    ///
    /// # Errors
    /// Returns `AssetNotFound` or `AssetValuationNotFound`.
    pub fn delete_valuation(
        &mut self,
        asset_id: AssetId,
        period: Period,
    ) -> Result<(), DomainError> {
        self.asset(asset_id)?;
        let before = self.valuations.len();
        self.valuations
            .retain(|s| !(s.asset_id == asset_id && s.period == period));
        if self.valuations.len() == before {
            return Err(DomainError::AssetValuationNotFound);
        }
        Ok(())
    }

    /// The valuations of all assets, newest period first. A yearly valuation
    /// counts as December of its year; within a period the order is that of
    /// the assets (REQ-18).
    #[must_use]
    pub fn all_valuations(&self) -> Vec<(&Asset, &AssetValuation)> {
        let mut rows: Vec<(&Asset, &AssetValuation)> = self
            .valuations
            .iter()
            .filter_map(|valuation| {
                self.assets
                    .iter()
                    .find(|asset| asset.id == valuation.asset_id)
                    .map(|asset| (asset, valuation))
            })
            .collect();
        rows.sort_by_cached_key(|(asset, valuation)| {
            (
                std::cmp::Reverse(valuation.period.sort_key()),
                asset_order(asset),
            )
        });
        rows
    }

    /// Checks that data read from a file follows the rules of CSD-015, so a
    /// hand-edited or damaged file cannot break them.
    ///
    /// # Errors
    /// Returns `AssetDataInvalid` for the first inconsistency found.
    pub fn validate(&self) -> Result<(), DomainError> {
        let invalid = DomainError::AssetDataInvalid;
        let mut ids = HashSet::new();
        let mut names = HashSet::new();
        for asset in &self.assets {
            let name = asset.name.trim();
            if name.is_empty()
                || name.chars().count() > MAX_NAME_CHARS
                || asset.colour_slot >= COLOUR_SLOTS
                || !ids.insert(asset.id)
                || !names.insert(name.to_lowercase())
            {
                return Err(invalid);
            }
        }
        let mut seen = HashSet::new();
        for valuation in &self.valuations {
            let matches_asset = self
                .asset(valuation.asset_id)
                .is_ok_and(|asset| asset.valuation_period == valuation.period.kind());
            if !matches_asset
                || !seen.insert((valuation.asset_id, valuation.period))
                || AssetValue::new(valuation.value.value()).is_err()
                || asset_note(valuation.note.as_str()).is_err()
            {
                return Err(invalid);
            }
        }
        Ok(())
    }

    fn asset(&self, id: AssetId) -> Result<&Asset, DomainError> {
        self.assets
            .iter()
            .find(|asset| asset.id == id)
            .ok_or(DomainError::AssetNotFound)
    }

    fn asset_mut(&mut self, id: AssetId) -> Result<&mut Asset, DomainError> {
        self.assets
            .iter_mut()
            .find(|asset| asset.id == id)
            .ok_or(DomainError::AssetNotFound)
    }

    fn valuation(&self, asset_id: AssetId, period: Period) -> Option<&AssetValuation> {
        self.valuations
            .iter()
            .find(|s| s.asset_id == asset_id && s.period == period)
    }

    /// The rules a period must follow for an asset to hold a valuation for it:
    /// the kind of the asset, not after `today`, not taken (REQ-7 to REQ-9).
    fn check_new_period(
        &self,
        asset_id: AssetId,
        period: Period,
        today: YearMonth,
    ) -> Result<(), DomainError> {
        if self.asset(asset_id)?.valuation_period != period.kind() {
            return Err(DomainError::AssetPeriodMismatch);
        }
        if period.is_after(today) {
            return Err(DomainError::AssetValuationInFuture);
        }
        if self.valuation(asset_id, period).is_some() {
            return Err(DomainError::AssetValuationDuplicate);
        }
        Ok(())
    }

    /// The valuation with the most recent period of an asset.
    fn latest(&self, id: AssetId) -> Option<&AssetValuation> {
        self.valuations
            .iter()
            .filter(|valuation| valuation.asset_id == id)
            .max_by_key(|valuation| valuation.period.sort_key())
    }

    /// The lowest colour slot used by the fewest assets (REQ-3).
    fn next_colour_slot(&self) -> u8 {
        (0..COLOUR_SLOTS)
            .min_by_key(|slot| {
                (
                    self.assets
                        .iter()
                        .filter(|asset| asset.colour_slot == *slot)
                        .count(),
                    *slot,
                )
            })
            .unwrap_or_default()
    }

    /// Whether another asset already has this name, ignoring case (REQ-2).
    fn name_taken(&self, name: &str, except: Option<AssetId>) -> bool {
        let wanted = name.to_lowercase();
        self.assets
            .iter()
            .any(|asset| Some(asset.id) != except && asset.name.to_lowercase() == wanted)
    }
}

/// Alphabetical order ignoring case, with the id as a stable tie-break.
fn asset_order(asset: &Asset) -> (String, String) {
    (asset.name.to_lowercase(), asset.id.to_string())
}
