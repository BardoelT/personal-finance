//! The yearly net worth: houses, investments, cash and debts per year (CSD-017).

use std::collections::BTreeMap;

use rust_decimal::Decimal;
use serde::Serialize;

use crate::domain::bank_accounts::BankAccounts;
use crate::domain::debts::Debts;
use crate::domain::houses::Houses;
use crate::domain::income::YearMonth;
use crate::domain::investments::Investments;
use crate::domain::money::zero;

#[cfg(test)]
mod tests;

/// One year of the net worth chart (CSD-017 REQ-2, REQ-3).
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct NetWorthYear {
    pub year: i32,
    /// The total value of the houses (CSD-016).
    pub houses: Decimal,
    /// The closing balance of the investments (CSD-007).
    pub investments: Decimal,
    /// The closing balance of the bank accounts (CSD-010).
    pub cash: Decimal,
    /// The total debt, never negative (CSD-012).
    pub debts: Decimal,
    /// Houses plus investments plus cash minus debts. It can be negative.
    pub net_worth: Decimal,
    /// Whether any of the four areas has a record in the year.
    pub has_records: bool,
    pub is_partial: bool,
}

/// The payload of `get_net_worth_insights` (CSD-017 section 5).
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct NetWorthInsights {
    pub years: Vec<NetWorthYear>,
}

/// The four figures of one year and whether they come from any record.
#[derive(Clone, Copy)]
struct Figures {
    houses: Decimal,
    investments: Decimal,
    cash: Decimal,
    debts: Decimal,
    has_records: bool,
}

impl Default for Figures {
    /// All four figures at 0.00 and no records.
    fn default() -> Self {
        Self {
            houses: zero(),
            investments: zero(),
            cash: zero(),
            debts: zero(),
            has_records: false,
        }
    }
}

/// One figure set per calendar year from the earliest year with a record in
/// any area up to and including the current year, oldest first. Each figure is
/// the one the chart of its own area shows (CSD-017 REQ-3, CON-4); an area
/// without a record in a year gives 0.00 and nothing is carried forward
/// (REQ-4). The current year is partial.
#[must_use]
pub fn net_worth_insights(
    houses: &Houses,
    investments: &Investments,
    banks: &BankAccounts,
    debts: &Debts,
    today: YearMonth,
) -> NetWorthInsights {
    let mut by_year: BTreeMap<i32, Figures> = BTreeMap::new();
    for row in houses.insights(today).years {
        let figures = by_year.entry(row.year).or_default();
        figures.houses = row.closing_value;
        figures.has_records |= row.valuation_count > 0;
    }
    for row in investments.insights(today) {
        let figures = by_year.entry(row.year).or_default();
        figures.investments = row.closing_balance;
        figures.has_records |= row.statement_count > 0;
    }
    for row in banks.insights(today).years {
        let figures = by_year.entry(row.year).or_default();
        figures.cash = row.closing_balance;
        figures.has_records |= row.entry_count > 0;
    }
    for row in debts.insights(today).years {
        let figures = by_year.entry(row.year).or_default();
        figures.debts = row.closing_balance;
        figures.has_records |= row.statement_count > 0;
    }
    let Some(&first) = by_year.keys().next() else {
        return NetWorthInsights { years: Vec::new() };
    };
    let years = (first..=today.year())
        .map(|year| {
            let figures = by_year.get(&year).copied().unwrap_or_default();
            NetWorthYear {
                year,
                houses: figures.houses,
                investments: figures.investments,
                cash: figures.cash,
                debts: figures.debts,
                net_worth: figures.houses + figures.investments + figures.cash - figures.debts,
                has_records: figures.has_records,
                is_partial: year == today.year(),
            }
        })
        .collect();
    NetWorthInsights { years }
}
