//! Linking debts to a house and settling them when it is sold (CSD-020).
//!
//! A house and its debts live in two places of the project, so the operations
//! work on both and change them together or not at all (CSD-020 CON-2).

use std::collections::HashSet;

use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use time::Date;

use crate::domain::debts::{DebtId, DebtSummary, Debts, OpenDebt, Outcome, Settlement};
use crate::domain::error::DomainError;

use super::{House, HouseDate, HouseId, HouseInput, HouseSummary, Houses};

/// The debt side of the Add and Edit house forms (CSD-020 REQ-1, REQ-3,
/// REQ-6).
#[derive(Clone, Debug, Default, Deserialize)]
pub struct DebtLinksInput {
    /// The full selection of linked debts.
    #[serde(default)]
    pub debt_ids: Vec<String>,
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
    debt_ids: Vec<DebtId>,
    settlements: Vec<(DebtId, Outcome)>,
    confirm_undo: bool,
}

impl DebtLinksInput {
    fn parse(&self) -> Result<Parsed, DomainError> {
        let mut debt_ids = Vec::new();
        for text in &self.debt_ids {
            let id = DebtId::parse(text)?;
            if !debt_ids.contains(&id) {
                debt_ids.push(id);
            }
        }
        let mut settlements = Vec::new();
        for choice in &self.settlements {
            settlements.push((
                DebtId::parse(&choice.debt_id)?,
                Outcome::parse(&choice.outcome)?,
            ));
        }
        Ok(Parsed {
            debt_ids,
            settlements,
            confirm_undo: self.confirm_undo,
        })
    }
}

/// Creates a house, links the selected debts and, when the house is created
/// sold, settles them (CSD-020 REQ-1, REQ-3).
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

/// Changes a house and its debts together: links, the settlement of a new sale,
/// and the undo of a sale (CSD-020 REQ-1 to REQ-7).
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

/// The linked debts with an open balance at a sale on `sell_date`, for the
/// settlement prompt (CSD-020 REQ-15).
///
/// # Errors
/// Returns a date error of CSD-019 or `DebtNotFound`.
pub fn open_debts(
    debts: &Debts,
    debt_ids: &[String],
    sell_date: &str,
    today: Date,
) -> Result<Vec<OpenDebt>, DomainError> {
    let date = HouseDate::parse(sell_date.trim(), today)?;
    let ids: Result<Vec<DebtId>, DomainError> = debt_ids.iter().map(|t| DebtId::parse(t)).collect();
    debts.open_debts(&ids?, date)
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
        let wanted: HashSet<DebtId> = parsed.debt_ids.iter().copied().collect();
        if wanted != current.iter().copied().collect() {
            return Err(DomainError::HouseLinksLockedSold);
        }
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
    set_links(debts, house.id, &parsed.debt_ids)?;
    match house.sell_date {
        Some(date) => settle_all(debts, house, date, &parsed.settlements),
        None if parsed.settlements.is_empty() => Ok(()),
        None => Err(DomainError::HouseSettlementInvalid),
    }
}

fn set_links(debts: &mut Debts, id: HouseId, wanted: &[DebtId]) -> Result<(), DomainError> {
    for debt in wanted {
        if !debts.exists(*debt) {
            return Err(DomainError::DebtNotFound);
        }
        if debts.house_of(*debt).is_some_and(|other| other != id) {
            return Err(DomainError::HouseDebtLinkedElsewhere);
        }
    }
    for debt in debts.linked_to(id) {
        if !wanted.contains(&debt) {
            if debts.is_settled(debt) {
                return Err(DomainError::DebtSettledLocked);
            }
            debts.set_house(debt, None)?;
        }
    }
    for debt in wanted {
        debts.set_house(*debt, Some(id))?;
    }
    Ok(())
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
