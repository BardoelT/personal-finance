//! Houses and their monthly or yearly valuations (CSD-015), and the yearly
//! totals and insights built on them (CSD-015 REQ-20, CSD-016).
//!
//! `Houses` holds the data and the rules for changing it; the other files add
//! the value types (`amounts`, `house`, `valuation`) and the figures derived
//! from it (`overview`, `totals`, `insights`). The period and note types are
//! those of the investments (CSD-005).

mod amounts;
mod dates;
mod house;
mod insights;
mod overview;
mod totals;
mod valuation;

#[cfg(test)]
mod purchase_tests;
#[cfg(test)]
mod tests;

pub use amounts::{house_note, HouseValue};
pub use dates::HouseDate;
pub use house::{House, HouseId, HouseInput};
pub use insights::HouseInsights;
pub use overview::HousesOverview;
pub use totals::HouseYearlyValue;
pub use valuation::{parse_period, HouseValuation, ValuationPrefill};

use std::collections::HashSet;

use serde::{Deserialize, Serialize};
use time::Date;

use crate::domain::error::DomainError;
use crate::domain::income::YearMonth;
use crate::domain::investments::{Note, Period};

use house::{COLOUR_SLOTS, MAX_NAME_CHARS};

/// All houses with their valuations. Houses are kept in the order they were
/// created (CSD-015 section 5).
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Houses {
    #[serde(default, alias = "assets")]
    houses: Vec<House>,
    #[serde(default)]
    valuations: Vec<HouseValuation>,
}

impl Houses {
    /// Creates a house, monthly unless the input says Yearly, with the colour
    /// slot of REQ-3 (REQ-1, REQ-2). `today` is the current date, which bounds
    /// the purchase and sell dates (CSD-019).
    ///
    /// # Errors
    /// Returns the name errors of REQ-2, `HousePeriodUnsupported` or the
    /// purchase and sale errors of CSD-019 REQ-1 to REQ-5.
    pub fn create_house(&mut self, input: &HouseInput, today: Date) -> Result<House, DomainError> {
        let valid = input.validate(today)?;
        if self.name_taken(&valid.name, None) {
            return Err(DomainError::HouseNameDuplicate);
        }
        let house = House {
            id: HouseId::generate(),
            name: valid.name,
            valuation_period: valid.valuation_period.unwrap_or_default(),
            colour_slot: self.next_colour_slot(),
            purchase_date: Some(valid.purchase_date),
            purchase_amount: Some(valid.purchase_amount),
            sell_date: valid.sale.map(|(date, _)| date),
            sell_amount: valid.sale.map(|(_, amount)| amount),
        };
        self.houses.push(house.clone());
        Ok(house)
    }

    /// Changes the name, valuation period, purchase and sale of a house. The
    /// valuations and the colour slot stay; the period is locked once the house
    /// has any valuation, and the dates may not leave a valuation outside the
    /// holding window (REQ-4, REQ-5, CSD-019 REQ-6, REQ-8).
    ///
    /// # Errors
    /// Returns a validation error, `HouseNotFound`, `HouseNameDuplicate`,
    /// `HousePeriodLocked` or `HouseDatesExcludeValuations`.
    pub fn update_house(
        &mut self,
        id: HouseId,
        input: &HouseInput,
        today: Date,
    ) -> Result<House, DomainError> {
        let valid = input.validate(today)?;
        let current = self.house(id)?.valuation_period;
        if self.name_taken(&valid.name, Some(id)) {
            return Err(DomainError::HouseNameDuplicate);
        }
        let valuation_period = valid.valuation_period.unwrap_or(current);
        if valuation_period != current && self.valuation_count(id) > 0 {
            return Err(DomainError::HousePeriodLocked);
        }
        let mut updated = self.house(id)?.clone();
        updated.name = valid.name;
        updated.valuation_period = valuation_period;
        updated.purchase_date = Some(valid.purchase_date);
        updated.purchase_amount = Some(valid.purchase_amount);
        updated.sell_date = valid.sale.map(|(date, _)| date);
        updated.sell_amount = valid.sale.map(|(_, amount)| amount);
        if self
            .valuations
            .iter()
            .any(|valuation| valuation.house_id == id && !updated.window_contains(valuation.period))
        {
            return Err(DomainError::HouseDatesExcludeValuations);
        }
        *self.house_mut(id)? = updated.clone();
        Ok(updated)
    }

    /// Removes a house and all its valuations (REQ-6).
    ///
    /// # Errors
    /// Returns `HouseNotFound`.
    pub fn delete_house(&mut self, id: HouseId) -> Result<(), DomainError> {
        self.house(id)?;
        self.houses.retain(|house| house.id != id);
        self.valuations.retain(|valuation| valuation.house_id != id);
        Ok(())
    }

    /// Houses alphabetically by name, ignoring case (REQ-18).
    #[must_use]
    pub fn houses(&self) -> Vec<&House> {
        let mut houses: Vec<&House> = self.houses.iter().collect();
        houses.sort_by_cached_key(|house| house_order(house));
        houses
    }

    /// How many valuations a house has.
    #[must_use]
    pub fn valuation_count(&self, id: HouseId) -> usize {
        self.valuations
            .iter()
            .filter(|valuation| valuation.house_id == id)
            .count()
    }

    /// How many valuations all houses have together.
    #[must_use]
    pub fn total_valuation_count(&self) -> usize {
        self.valuations.len()
    }

    /// Adds the valuation of a period. `today` is the current month; later
    /// periods are rejected. Any earlier period is accepted (REQ-7 to REQ-9).
    ///
    /// # Errors
    /// Returns `HouseNotFound`, `HousePeriodMismatch`, `HouseValuationInFuture`
    /// or `HouseValuationDuplicate`.
    pub fn add_valuation(
        &mut self,
        house_id: HouseId,
        period: Period,
        value: HouseValue,
        note: Note,
        today: YearMonth,
    ) -> Result<HouseValuation, DomainError> {
        self.check_new_period(house_id, period, today)?;
        let valuation = HouseValuation {
            house_id,
            period,
            value,
            note,
        };
        self.valuations.push(valuation.clone());
        Ok(valuation)
    }

    /// Changes the value and note of a valuation and, when `new_period` is a
    /// different period, moves it there. The new period follows the rules of
    /// adding a valuation: the kind of the house, not after `today` and not
    /// taken by another valuation of the house. The edit is applied as a whole
    /// or not at all (REQ-12, REQ-13).
    ///
    /// # Errors
    /// Returns `HouseNotFound`, `HouseValuationNotFound`, `HousePeriodMismatch`,
    /// `HouseValuationInFuture` or `HouseValuationDuplicate`.
    pub fn update_valuation(
        &mut self,
        house_id: HouseId,
        period: Period,
        new_period: Period,
        value: HouseValue,
        note: Note,
        today: YearMonth,
    ) -> Result<HouseValuation, DomainError> {
        self.house(house_id)?;
        if self.valuation(house_id, period).is_none() {
            return Err(DomainError::HouseValuationNotFound);
        }
        if new_period != period {
            self.check_new_period(house_id, new_period, today)?;
        }
        let valuation = self
            .valuations
            .iter_mut()
            .find(|s| s.house_id == house_id && s.period == period)
            .ok_or(DomainError::HouseValuationNotFound)?;
        valuation.period = new_period;
        valuation.value = value;
        valuation.note = note;
        Ok(valuation.clone())
    }

    /// Removes one valuation and nothing else (REQ-14). The caller asks for
    /// the confirmation.
    ///
    /// # Errors
    /// Returns `HouseNotFound` or `HouseValuationNotFound`.
    pub fn delete_valuation(
        &mut self,
        house_id: HouseId,
        period: Period,
    ) -> Result<(), DomainError> {
        self.house(house_id)?;
        let before = self.valuations.len();
        self.valuations
            .retain(|s| !(s.house_id == house_id && s.period == period));
        if self.valuations.len() == before {
            return Err(DomainError::HouseValuationNotFound);
        }
        Ok(())
    }

    /// The valuations of all houses, newest period first. A yearly valuation
    /// counts as December of its year; within a period the order is that of
    /// the houses (REQ-18).
    #[must_use]
    pub fn all_valuations(&self) -> Vec<(&House, &HouseValuation)> {
        let mut rows: Vec<(&House, &HouseValuation)> = self
            .valuations
            .iter()
            .filter_map(|valuation| {
                self.houses
                    .iter()
                    .find(|house| house.id == valuation.house_id)
                    .map(|house| (house, valuation))
            })
            .collect();
        rows.sort_by_cached_key(|(house, valuation)| {
            (
                std::cmp::Reverse(valuation.period.sort_key()),
                house_order(house),
            )
        });
        rows
    }

    /// Checks that data read from a file follows the rules of CSD-015, so a
    /// hand-edited or damaged file cannot break them.
    ///
    /// # Errors
    /// Returns `HouseDataInvalid` for the first inconsistency found.
    pub fn validate(&self) -> Result<(), DomainError> {
        let invalid = DomainError::HouseDataInvalid;
        let mut ids = HashSet::new();
        let mut names = HashSet::new();
        for house in &self.houses {
            let name = house.name.trim();
            if name.is_empty()
                || name.chars().count() > MAX_NAME_CHARS
                || house.colour_slot >= COLOUR_SLOTS
                || !ids.insert(house.id)
                || !names.insert(name.to_lowercase())
                || !purchase_and_sale_consistent(house)
            {
                return Err(invalid);
            }
        }
        let mut seen = HashSet::new();
        for valuation in &self.valuations {
            let matches_house = self
                .house(valuation.house_id)
                .is_ok_and(|house| house.valuation_period == valuation.period.kind());
            let in_window = self
                .house(valuation.house_id)
                .is_ok_and(|house| house.window_contains(valuation.period));
            if !matches_house
                || !in_window
                || !seen.insert((valuation.house_id, valuation.period))
                || HouseValue::new(valuation.value.value()).is_err()
                || house_note(valuation.note.as_str()).is_err()
            {
                return Err(invalid);
            }
        }
        Ok(())
    }

    fn house(&self, id: HouseId) -> Result<&House, DomainError> {
        self.houses
            .iter()
            .find(|house| house.id == id)
            .ok_or(DomainError::HouseNotFound)
    }

    fn house_mut(&mut self, id: HouseId) -> Result<&mut House, DomainError> {
        self.houses
            .iter_mut()
            .find(|house| house.id == id)
            .ok_or(DomainError::HouseNotFound)
    }

    fn valuation(&self, house_id: HouseId, period: Period) -> Option<&HouseValuation> {
        self.valuations
            .iter()
            .find(|s| s.house_id == house_id && s.period == period)
    }

    /// The rules a period must follow for a house to hold a valuation for it:
    /// the kind of the house, not after `today`, inside the holding window
    /// (CSD-019 REQ-7), not taken (REQ-7 to REQ-9).
    fn check_new_period(
        &self,
        house_id: HouseId,
        period: Period,
        today: YearMonth,
    ) -> Result<(), DomainError> {
        let house = self.house(house_id)?;
        if house.valuation_period != period.kind() {
            return Err(DomainError::HousePeriodMismatch);
        }
        if period.is_after(today) {
            return Err(DomainError::HouseValuationInFuture);
        }
        if !house.window_contains(period) {
            return Err(DomainError::HouseValuationOutsideHolding);
        }
        if self.valuation(house_id, period).is_some() {
            return Err(DomainError::HouseValuationDuplicate);
        }
        Ok(())
    }

    /// The valuation with the most recent period of a house.
    fn latest(&self, id: HouseId) -> Option<&HouseValuation> {
        self.valuations
            .iter()
            .filter(|valuation| valuation.house_id == id)
            .max_by_key(|valuation| valuation.period.sort_key())
    }

    /// The lowest colour slot used by the fewest houses (REQ-3).
    fn next_colour_slot(&self) -> u8 {
        (0..COLOUR_SLOTS)
            .min_by_key(|slot| {
                (
                    self.houses
                        .iter()
                        .filter(|house| house.colour_slot == *slot)
                        .count(),
                    *slot,
                )
            })
            .unwrap_or_default()
    }

    /// Whether another house already has this name, ignoring case (REQ-2).
    fn name_taken(&self, name: &str, except: Option<HouseId>) -> bool {
        let wanted = name.to_lowercase();
        self.houses
            .iter()
            .any(|house| Some(house.id) != except && house.name.to_lowercase() == wanted)
    }
}

/// Whether the purchase and sale of a house read from a file follow CSD-019:
/// a purchase is both fields or neither, a sale needs a purchase and is both
/// fields, the sell date is not before the purchase date and the amounts are
/// valid.
fn purchase_and_sale_consistent(house: &House) -> bool {
    let amounts_valid = [house.purchase_amount, house.sell_amount]
        .into_iter()
        .flatten()
        .all(|amount| HouseValue::new(amount.value()).is_ok());
    let purchase = match (house.purchase_date, house.purchase_amount) {
        (Some(date), Some(_)) => Some(date),
        (None, None) => None,
        _ => return false,
    };
    let sale_valid = match (house.sell_date, house.sell_amount) {
        (None, None) => true,
        (Some(sold), Some(_)) => purchase.is_some_and(|bought| sold >= bought),
        _ => false,
    };
    amounts_valid && sale_valid
}

/// Alphabetical order ignoring case, with the id as a stable tie-break.
fn house_order(house: &House) -> (String, String) {
    (house.name.to_lowercase(), house.id.to_string())
}
