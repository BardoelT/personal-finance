//! Yearly totals over all debts (CSD-011 REQ-19).

use std::collections::BTreeMap;

use rust_decimal::Decimal;
use serde::Serialize;

use crate::domain::money::zero;

use super::{Debt, Debts};

/// The totals of one calendar year over all debts (CSD-011 REQ-19).
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct DebtYearlyTotal {
    pub year: i32,
    /// The sum, over the debts with a statement this year, of the balance of
    /// their latest statement this year.
    pub closing_balance: Decimal,
    /// Opening balance minus closing balance, over the debts that have both;
    /// `None` when no debt has an opening balance this year.
    pub repaid: Option<Decimal>,
    pub statement_count: usize,
}

impl Debts {
    /// Totals per calendar year over all debts, newest year first, without
    /// extrapolating partial years or carrying balances forward (REQ-19). A
    /// statement counts in the year of its own period only.
    #[must_use]
    pub fn yearly_totals(&self) -> Vec<DebtYearlyTotal> {
        let mut counts: BTreeMap<i32, usize> = BTreeMap::new();
        for statement in &self.statements {
            *counts.entry(statement.period.year()).or_default() += 1;
        }
        counts
            .into_iter()
            .rev()
            .map(|(year, statement_count)| DebtYearlyTotal {
                year,
                closing_balance: self
                    .year_segments(year)
                    .into_iter()
                    .fold(zero(), |sum, (_, balance)| sum + balance),
                repaid: self.repaid(year),
                statement_count,
            })
            .collect()
    }

    /// For each debt with a statement in `year`, in creation order, the balance
    /// of its latest statement in that year.
    pub(super) fn year_segments(&self, year: i32) -> Vec<(&Debt, Decimal)> {
        self.debts
            .iter()
            .filter_map(|debt| {
                self.statements
                    .iter()
                    .filter(|s| s.debt_id == debt.id && s.period.year() == year)
                    .max_by_key(|s| s.period.sort_key())
                    .map(|s| (debt, s.balance.value()))
            })
            .collect()
    }

    /// The amount repaid in `year`: for each debt with a statement in the year
    /// and an earlier one, its opening balance minus its closing balance.
    /// `None` when no debt has an opening balance (CSD-011 section 3).
    pub(super) fn repaid(&self, year: i32) -> Option<Decimal> {
        let mut repaid: Option<Decimal> = None;
        for (debt, closing) in self.year_segments(year) {
            let opening = self
                .statements
                .iter()
                .filter(|s| s.debt_id == debt.id && s.period.year() < year)
                .max_by_key(|s| s.period.sort_key())
                .map(|s| s.balance.value());
            if let Some(opening) = opening {
                repaid = Some(repaid.unwrap_or_else(zero) + opening - closing);
            }
        }
        repaid
    }
}
