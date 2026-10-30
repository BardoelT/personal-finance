//! The debts with their latest balances and the total debt (CSD-011 REQ-17).

use rust_decimal::Decimal;
use serde::Serialize;

use crate::domain::investments::Period;

use super::{Debt, Debts};

/// One debt row of the Debts view: its latest balance and period (CSD-011
/// REQ-17).
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct DebtSummary {
    #[serde(flatten)]
    pub debt: Debt,
    pub statement_count: usize,
    pub latest_period: Option<Period>,
    pub latest_balance: Option<Decimal>,
    /// Whether the latest statement has the balance 0.00 (CSD-011 REQ-26).
    pub repaid: bool,
}

/// The debts with the total of their latest balances (CSD-011 REQ-17). The
/// total is `None` while no debt has a statement.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct DebtsOverview {
    pub debts: Vec<DebtSummary>,
    pub total: Option<Decimal>,
}

impl Debts {
    /// The debts with their latest balances and the total debt (REQ-17).
    /// Periods are not aligned: each debt counts with its own latest statement.
    #[must_use]
    pub fn overview(&self) -> DebtsOverview {
        let debts: Vec<DebtSummary> = self
            .debts()
            .into_iter()
            .map(|debt| {
                let latest = self.latest(debt.id);
                DebtSummary {
                    debt: debt.clone(),
                    statement_count: self.statement_count(debt.id),
                    latest_period: latest.map(|statement| statement.period),
                    latest_balance: latest.map(|statement| statement.balance.value()),
                    repaid: latest.is_some_and(|statement| statement.balance.value().is_zero()),
                }
            })
            .collect();
        let mut balances = debts.iter().filter_map(|summary| summary.latest_balance);
        let total = balances
            .next()
            .map(|first| balances.fold(first, |sum, balance| sum + balance));
        DebtsOverview { debts, total }
    }
}
