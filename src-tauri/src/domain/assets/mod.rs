//! Assets and their monthly or yearly valuations (CSD-015), and the yearly
//! totals and insights built on them (CSD-015 REQ-20, CSD-016).
//!
//! `Assets` holds the data and the rules for changing it; the other files add
//! the value types (`amounts`, `asset`, `valuation`) and the figures derived
//! from it (`overview`, `totals`, `insights`). The period and note types are
//! those of the investments (CSD-005).

mod amounts;
mod asset;
mod dates;
mod insights;
mod overview;
mod totals;
mod valuation;

#[cfg(test)]
mod purchase_tests;
#[cfg(test)]
mod tests;

pub use amounts::{asset_note, AssetValue};
pub use asset::{Asset, AssetId, AssetInput};
pub use dates::AssetDate;
pub use insights::AssetInsights;
pub use overview::AssetsOverview;
pub use totals::AssetYearlyValue;
pub use valuation::{parse_period, AssetValuation, ValuationPrefill};

use std::collections::HashSet;

use serde::{Deserialize, Serialize};
use time::Date;

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
    /// slot of REQ-3 (REQ-1, REQ-2). `today` is the current date, which bounds
    /// the purchase and sell dates (CSD-019).
    ///
    /// # Errors
    /// Returns the name errors of REQ-2, `AssetPeriodUnsupported` or the
    /// purchase and sale errors of CSD-019 REQ-1 to REQ-5.
    pub fn create_asset(&mut self, input: &AssetInput, today: Date) -> Result<Asset, DomainError> {
        let valid = input.validate(today)?;
        if self.name_taken(&valid.name, None) {
            return Err(DomainError::AssetNameDuplicate);
        }
        let asset = Asset {
            id: AssetId::generate(),
            name: valid.name,
            valuation_period: valid.valuation_period.unwrap_or_default(),
            colour_slot: self.next_colour_slot(),
            purchase_date: Some(valid.purchase_date),
            purchase_amount: Some(valid.purchase_amount),
            sell_date: valid.sale.map(|(date, _)| date),
            sell_amount: valid.sale.map(|(_, amount)| amount),
        };
        self.assets.push(asset.clone());
        Ok(asset)
    }

    /// Changes the name, valuation period, purchase and sale of an asset. The
    /// valuations and the colour slot stay; the period is locked once the asset
    /// has any valuation, and the dates may not leave a valuation outside the
    /// holding window (REQ-4, REQ-5, CSD-019 REQ-6, REQ-8).
    ///
    /// # Errors
    /// Returns a validation error, `AssetNotFound`, `AssetNameDuplicate`,
    /// `AssetPeriodLocked` or `AssetDatesExcludeValuations`.
    pub fn update_asset(
        &mut self,
        id: AssetId,
        input: &AssetInput,
        today: Date,
    ) -> Result<Asset, DomainError> {
        let valid = input.validate(today)?;
        let current = self.asset(id)?.valuation_period;
        if self.name_taken(&valid.name, Some(id)) {
            return Err(DomainError::AssetNameDuplicate);
        }
        let valuation_period = valid.valuation_period.unwrap_or(current);
        if valuation_period != current && self.valuation_count(id) > 0 {
            return Err(DomainError::AssetPeriodLocked);
        }
        let mut updated = self.asset(id)?.clone();
        updated.name = valid.name;
        updated.valuation_period = valuation_period;
        updated.purchase_date = Some(valid.purchase_date);
        updated.purchase_amount = Some(valid.purchase_amount);
        updated.sell_date = valid.sale.map(|(date, _)| date);
        updated.sell_amount = valid.sale.map(|(_, amount)| amount);
        if self
            .valuations
            .iter()
            .any(|valuation| valuation.asset_id == id && !updated.window_contains(valuation.period))
        {
            return Err(DomainError::AssetDatesExcludeValuations);
        }
        *self.asset_mut(id)? = updated.clone();
        Ok(updated)
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
                || !purchase_and_sale_consistent(asset)
            {
                return Err(invalid);
            }
        }
        let mut seen = HashSet::new();
        for valuation in &self.valuations {
            let matches_asset = self
                .asset(valuation.asset_id)
                .is_ok_and(|asset| asset.valuation_period == valuation.period.kind());
            let in_window = self
                .asset(valuation.asset_id)
                .is_ok_and(|asset| asset.window_contains(valuation.period));
            if !matches_asset
                || !in_window
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
    /// the kind of the asset, not after `today`, inside the holding window
    /// (CSD-019 REQ-7), not taken (REQ-7 to REQ-9).
    fn check_new_period(
        &self,
        asset_id: AssetId,
        period: Period,
        today: YearMonth,
    ) -> Result<(), DomainError> {
        let asset = self.asset(asset_id)?;
        if asset.valuation_period != period.kind() {
            return Err(DomainError::AssetPeriodMismatch);
        }
        if period.is_after(today) {
            return Err(DomainError::AssetValuationInFuture);
        }
        if !asset.window_contains(period) {
            return Err(DomainError::AssetValuationOutsideHolding);
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

/// Whether the purchase and sale of an asset read from a file follow CSD-019:
/// a purchase is both fields or neither, a sale needs a purchase and is both
/// fields, the sell date is not before the purchase date and the amounts are
/// valid.
fn purchase_and_sale_consistent(asset: &Asset) -> bool {
    let amounts_valid = [asset.purchase_amount, asset.sell_amount]
        .into_iter()
        .flatten()
        .all(|amount| AssetValue::new(amount.value()).is_ok());
    let purchase = match (asset.purchase_date, asset.purchase_amount) {
        (Some(date), Some(_)) => Some(date),
        (None, None) => None,
        _ => return false,
    };
    let sale_valid = match (asset.sell_date, asset.sell_amount) {
        (None, None) => true,
        (Some(sold), Some(_)) => purchase.is_some_and(|bought| sold >= bought),
        _ => false,
    };
    amounts_valid && sale_valid
}

/// Alphabetical order ignoring case, with the id as a stable tie-break.
fn asset_order(asset: &Asset) -> (String, String) {
    (asset.name.to_lowercase(), asset.id.to_string())
}
