//! Yearly totals over all accounts (CSD-009 REQ-19).

use std::collections::BTreeMap;

use rust_decimal::Decimal;
use serde::Serialize;

use crate::domain::money::zero;

use super::BankAccounts;

/// The totals of one calendar year over all accounts (CSD-009 REQ-19).
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct BankYearlyTotal {
    pub year: i32,
    /// The sum, over the accounts with an entry this year, of the balance of
    /// their latest entry this year.
    pub closing_balance: Decimal,
    pub entry_count: usize,
}

impl BankAccounts {
    /// Totals per calendar year over all accounts, newest year first, without
    /// extrapolating partial years or carrying balances forward (REQ-19). An
    /// entry counts in the year of its own period only.
    #[must_use]
    pub fn yearly_totals(&self) -> Vec<BankYearlyTotal> {
        let mut counts: BTreeMap<i32, usize> = BTreeMap::new();
        for entry in &self.entries {
            *counts.entry(entry.period.year()).or_default() += 1;
        }
        counts
            .into_iter()
            .rev()
            .map(|(year, entry_count)| BankYearlyTotal {
                year,
                closing_balance: self
                    .year_segments(year)
                    .into_iter()
                    .fold(zero(), |sum, (_, balance)| sum + balance),
                entry_count,
            })
            .collect()
    }

    /// For each account with an entry in `year`, in creation order, the balance
    /// of its latest entry in that year.
    pub(super) fn year_segments(&self, year: i32) -> Vec<(&super::BankAccount, Decimal)> {
        self.accounts
            .iter()
            .filter_map(|account| {
                self.entries
                    .iter()
                    .filter(|e| e.account_id == account.id && e.period.year() == year)
                    .max_by_key(|e| e.period.sort_key())
                    .map(|e| (account, e.balance.value()))
            })
            .collect()
    }

    /// The increase of the bank accounts in `year`: for each account with an
    /// entry in the year and an earlier one, the balance of its latest entry in
    /// the year minus that of its latest earlier entry. `None` when no account
    /// has both (CSD-018 section 3).
    #[must_use]
    pub fn increase(&self, year: i32) -> Option<Decimal> {
        let mut increase: Option<Decimal> = None;
        for (account, closing) in self.year_segments(year) {
            let opening = self
                .entries
                .iter()
                .filter(|e| e.account_id == account.id && e.period.year() < year)
                .max_by_key(|e| e.period.sort_key())
                .map(|e| e.balance.value());
            if let Some(opening) = opening {
                increase = Some(increase.unwrap_or_else(zero) + closing - opening);
            }
        }
        increase
    }
}
