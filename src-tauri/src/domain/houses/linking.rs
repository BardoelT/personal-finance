//! Linking debts to a house, in the debt form, and settling them when the
//! house is sold (CSD-020).
//!
//! A house and its debts live in two places of the project, so the operations
//! work on both and change them together or not at all (CSD-020 CON-2).

use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use time::Date;

use crate::domain::debts::{
    Category, Debt, DebtId, DebtInput, DebtSummary, Debts, OpenDebt, Outcome, Settlement,
};
use crate::domain::error::DomainError;

use super::{House, HouseDate, HouseId, HouseInput, HouseSummary, Houses};

/// The debt side of the Add and Edit house forms (CSD-020 REQ-3, REQ-6).
#[derive(Clone, Debug, Default, Deserialize)]
pub struct DebtLinksInput {
    /// The Repaid or Cancelled choice per debt with an open balance.
    #[serde(default)]
    pub settlements: Vec<SettlementChoice>,
    /// Confirms that undoing a sale also undoes its settlements.
    #[serde(default)]
    pub confirm_undo: bool,
}

/// One choice of the settlement prompt.
#[derive(Clone, Debug, Deserialize)]
pub struct SettlementChoice {
    pub debt_id: String,
    /// `Repaid` or `Cancelled`.
    pub outcome: String,
}

struct Parsed {
    settlements: Vec<(DebtId, Outcome)>,
    confirm_undo: bool,
}

impl DebtLinksInput {
    fn parse(&self) -> Result<Parsed, DomainError> {
        let mut settlements = Vec::new();
        for choice in &self.settlements {
            settlements.push((
                DebtId::parse(&choice.debt_id)?,
                Outcome::parse(&choice.outcome)?,
            ));
        }
        Ok(Parsed {
            settlements,
            confirm_undo: self.confirm_undo,
        })
    }
}

/// Creates a house (CSD-015 REQ-1). It has no linked debts yet, so a sale
/// given at creation settles nothing (CSD-020 REQ-3).
///
/// # Errors
/// Returns the errors of `Houses::create_house` and of the link and
/// settlement rules; nothing changes on an error.
pub fn create_house(
    houses: &mut Houses,
    debts: &mut Debts,
    input: &HouseInput,
    links: &DebtLinksInput,
    today: Date,
) -> Result<House, DomainError> {
    let parsed = links.parse()?;
    let mut new_houses = houses.clone();
    let mut new_debts = debts.clone();
    let house = new_houses.create_house(input, today)?;
    apply(&mut new_debts, None, &house, &parsed)?;
    *houses = new_houses;
    *debts = new_debts;
    Ok(house)
}

/// Changes a house and its linked debts together: the settlement of a new
/// sale, and the undo of a sale (CSD-020 REQ-3 to REQ-7).
///
/// # Errors
/// Returns the errors of `Houses::update_house` and of the link and settlement
/// rules; nothing changes on an error.
pub fn update_house(
    houses: &mut Houses,
    debts: &mut Debts,
    id: HouseId,
    input: &HouseInput,
    links: &DebtLinksInput,
    today: Date,
) -> Result<House, DomainError> {
    let parsed = links.parse()?;
    let mut new_houses = houses.clone();
    let mut new_debts = debts.clone();
    let old = new_houses.house(id)?.clone();
    let house = new_houses.update_house(id, input, today)?;
    apply(&mut new_debts, Some(&old), &house, &parsed)?;
    *houses = new_houses;
    *debts = new_debts;
    Ok(house)
}

/// Deletes a house unless debts are linked to it (CSD-020 REQ-9).
///
/// # Errors
/// Returns `HouseNotFound` or `HouseHasLinkedDebts`.
pub fn delete_house(houses: &mut Houses, debts: &Debts, id: HouseId) -> Result<(), DomainError> {
    houses.house(id)?;
    if !debts.linked_to(id).is_empty() {
        return Err(DomainError::HouseHasLinkedDebts);
    }
    houses.delete_house(id)
}

/// The debts linked to `house_id` with an open balance at a sale on
/// `sell_date`, for the settlement prompt (CSD-020 REQ-15).
///
/// # Errors
/// Returns a date error of CSD-019.
pub fn open_debts(
    debts: &Debts,
    house_id: HouseId,
    sell_date: &str,
    today: Date,
) -> Result<Vec<OpenDebt>, DomainError> {
    let date = HouseDate::parse(sell_date.trim(), today)?;
    debts.open_debts(&debts.linked_to(house_id), date)
}

/// The debt form fields CSD-020 adds to those of CSD-011 (REQ-1).
#[derive(Clone, Debug, Default, Deserialize)]
pub struct DebtFormInput {
    #[serde(flatten)]
    pub debt: DebtInput,
    /// `Mortgage`, `Loan` or `Other`; `Other` on create and unchanged on an
    /// edit when left out.
    #[serde(default)]
    pub category: Option<String>,
    /// The house of a Mortgage or Loan; none or empty for no house.
    #[serde(default)]
    pub house_id: Option<String>,
    /// `Repaid` or `Cancelled` when the debt is linked to a house that is
    /// already sold and has an open balance (CSD-020 REQ-17).
    #[serde(default)]
    pub settlement: Option<String>,
}

/// Creates a debt with its category and optional house (CSD-020 REQ-1,
/// REQ-2).
///
/// # Errors
/// Returns the errors of `Debts::create_debt` and the category and link rules;
/// nothing changes on an error.
pub fn create_debt(
    houses: &Houses,
    debts: &mut Debts,
    input: &DebtFormInput,
) -> Result<Debt, DomainError> {
    let mut new_debts = debts.clone();
    let id = new_debts.create_debt(&input.debt)?.id;
    apply_category_and_link(houses, &mut new_debts, id, input)?;
    *debts = new_debts;
    debts.get(id).cloned().ok_or(DomainError::DebtNotFound)
}

/// Changes a debt with its category and optional house (CSD-020 REQ-1,
/// REQ-2, REQ-8).
///
/// # Errors
/// Returns the errors of `Debts::update_debt` and the category and link rules;
/// nothing changes on an error.
pub fn update_debt(
    houses: &Houses,
    debts: &mut Debts,
    id: DebtId,
    input: &DebtFormInput,
) -> Result<Debt, DomainError> {
    let mut new_debts = debts.clone();
    new_debts.update_debt(id, &input.debt)?;
    apply_category_and_link(houses, &mut new_debts, id, input)?;
    *debts = new_debts;
    debts.get(id).cloned().ok_or(DomainError::DebtNotFound)
}

fn apply_category_and_link(
    houses: &Houses,
    debts: &mut Debts,
    id: DebtId,
    input: &DebtFormInput,
) -> Result<(), DomainError> {
    let category = match input.category.as_deref().map(str::trim) {
        Some(text) if !text.is_empty() => Category::parse(text)?,
        _ => debts.get(id).map(|debt| debt.category).unwrap_or_default(),
    };
    let wanted = match input.house_id.as_deref().map(str::trim) {
        Some(text) if !text.is_empty() => Some(HouseId::parse(text)?),
        _ => None,
    };
    if wanted.is_some() && !category.can_link() {
        return Err(DomainError::DebtLinkCategoryInvalid);
    }
    let outcome = match input.settlement.as_deref().map(str::trim) {
        Some(text) if !text.is_empty() => Some(Outcome::parse(text)?),
        _ => None,
    };
    let current = debts.house_of(id);
    debts.set_category(id, category)?;
    if wanted == current {
        return if outcome.is_some() {
            Err(DomainError::HouseSettlementInvalid)
        } else {
            Ok(())
        };
    }
    if debts.is_settled(id) {
        return Err(DomainError::DebtSettledLocked);
    }
    if current.is_some_and(|house_id| houses.get(house_id).is_some_and(|house| !house.is_held())) {
        return Err(DomainError::HouseLinksLockedSold);
    }
    let Some(house_id) = wanted else {
        debts.set_house(id, None)?;
        return if outcome.is_some() {
            Err(DomainError::HouseSettlementInvalid)
        } else {
            Ok(())
        };
    };
    let house = houses.get(house_id).ok_or(DomainError::HouseNotFound)?;
    debts.set_house(id, Some(house_id))?;
    match house.sell_date {
        Some(date) => settle_on_link(debts, id, house, date, outcome),
        None if outcome.is_some() => Err(DomainError::HouseSettlementInvalid),
        None => Ok(()),
    }
}

/// Settles a debt that is linked to a house sold on `date`, when it has an
/// open balance (CSD-020 REQ-17).
fn settle_on_link(
    debts: &mut Debts,
    id: DebtId,
    house: &House,
    date: HouseDate,
    outcome: Option<Outcome>,
) -> Result<(), DomainError> {
    match (debts.open_balance(id, date)?, outcome) {
        (Some(_), Some(outcome)) => debts.settle(id, outcome, &house.name, date),
        (Some(_), None) => Err(DomainError::HouseSettlementRequired),
        (None, Some(_)) => Err(DomainError::HouseSettlementInvalid),
        (None, None) => Ok(()),
    }
}

/// The open balance of a debt at the sell date of the sold house it would be
/// linked to, `None` for a held house or without an open balance, for the
/// settlement prompt (CSD-020 REQ-15, REQ-17).
///
/// # Errors
/// Returns `DebtNotFound` or `HouseNotFound`.
pub fn debt_open_balance(
    houses: &Houses,
    debts: &Debts,
    debt_id: DebtId,
    house_id: HouseId,
) -> Result<Option<OpenDebt>, DomainError> {
    let house = houses.get(house_id).ok_or(DomainError::HouseNotFound)?;
    let debt = debts.get(debt_id).ok_or(DomainError::DebtNotFound)?;
    let Some(date) = house.sell_date else {
        return Ok(None);
    };
    Ok(debts.open_balance(debt_id, date)?.map(|amount| OpenDebt {
        debt_id,
        debt_name: debt.name.clone(),
        amount,
    }))
}

/// Applies the debt side of a saved house. `old` is the house before the save,
/// `None` for a new house.
fn apply(
    debts: &mut Debts,
    old: Option<&House>,
    house: &House,
    parsed: &Parsed,
) -> Result<(), DomainError> {
    let was_sold = old.is_some_and(|old| !old.is_held());
    let current = debts.linked_to(house.id);
    let settled: Vec<DebtId> = current
        .iter()
        .copied()
        .filter(|id| debts.is_settled(*id))
        .collect();
    if was_sold && !house.is_held() {
        if old.is_some_and(|old| old.sell_date != house.sell_date) && !settled.is_empty() {
            return Err(DomainError::HouseSellDateLocked);
        }
        if !parsed.settlements.is_empty() {
            return Err(DomainError::HouseSettlementInvalid);
        }
        return Ok(());
    }
    if was_sold {
        if !settled.is_empty() && !parsed.confirm_undo {
            return Err(DomainError::HouseSaleUndoUnconfirmed);
        }
        for id in settled {
            debts.unsettle(id)?;
        }
    }
    match house.sell_date {
        Some(date) => settle_all(debts, house, date, &parsed.settlements),
        None if parsed.settlements.is_empty() => Ok(()),
        None => Err(DomainError::HouseSettlementInvalid),
    }
}

fn settle_all(
    debts: &mut Debts,
    house: &House,
    date: HouseDate,
    choices: &[(DebtId, Outcome)],
) -> Result<(), DomainError> {
    let open = debts.open_debts(&debts.linked_to(house.id), date)?;
    if choices
        .iter()
        .any(|(id, _)| !open.iter().any(|debt| debt.debt_id == *id))
    {
        return Err(DomainError::HouseSettlementInvalid);
    }
    for debt in &open {
        let (_, outcome) = choices
            .iter()
            .find(|(id, _)| *id == debt.debt_id)
            .ok_or(DomainError::HouseSettlementRequired)?;
        debts.settle(debt.debt_id, *outcome, &house.name, date)?;
    }
    Ok(())
}

/// A debt linked to a house, as the Houses view shows it (CSD-020 REQ-13).
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct LinkedDebt {
    pub debt_id: DebtId,
    pub debt_name: String,
    pub category: Category,
    pub latest_balance: Option<Decimal>,
    pub settlement: Option<Settlement>,
}

/// A house row with its linked debts.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct HouseRow {
    #[serde(flatten)]
    pub summary: HouseSummary,
    pub linked_debts: Vec<LinkedDebt>,
}

/// The houses overview with the linked debts of each house.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct HousesView {
    pub houses: Vec<HouseRow>,
    pub total: Option<Decimal>,
}

/// A debt row with the name of its house, as the Debts view shows it
/// (CSD-020 REQ-14).
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct DebtRow {
    #[serde(flatten)]
    pub summary: DebtSummary,
    pub house_name: Option<String>,
}

/// The debts overview with the house of each debt.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct DebtsView {
    pub debts: Vec<DebtRow>,
    pub total: Option<Decimal>,
}

/// The houses overview (CSD-015 REQ-17) with the debts linked to each house.
#[must_use]
pub fn houses_view(houses: &Houses, debts: &Debts) -> HousesView {
    let overview = houses.overview();
    let summaries = debts.overview().debts;
    let rows = overview
        .houses
        .into_iter()
        .map(|summary| {
            let linked_debts = summaries
                .iter()
                .filter(|row| row.debt.house_id == Some(summary.house.id))
                .map(|row| LinkedDebt {
                    debt_id: row.debt.id,
                    debt_name: row.debt.name.clone(),
                    category: row.debt.category,
                    latest_balance: row.latest_balance,
                    settlement: row.debt.settlement.clone(),
                })
                .collect();
            HouseRow {
                summary,
                linked_debts,
            }
        })
        .collect();
    HousesView {
        houses: rows,
        total: overview.total,
    }
}

/// The debts overview (CSD-011 REQ-17) with the name of each linked house.
#[must_use]
pub fn debts_view(debts: &Debts, houses: &Houses) -> DebtsView {
    let overview = debts.overview();
    let rows = overview
        .debts
        .into_iter()
        .map(|summary| {
            let house_name = summary
                .debt
                .house_id
                .and_then(|id| houses.get(id))
                .map(|house| house.name.clone());
            DebtRow {
                summary,
                house_name,
            }
        })
        .collect();
    DebtsView {
        debts: rows,
        total: overview.total,
    }
}
