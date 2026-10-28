//! The accounts with their latest balances and the total (CSD-009 REQ-17).

use rust_decimal::Decimal;
use serde::Serialize;

use crate::domain::investments::Period;

use super::{BankAccount, BankAccounts};

/// One account row of the Bank accounts view: its latest balance and period
/// (CSD-009 REQ-17).
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct BankAccountSummary {
    #[serde(flatten)]
    pub account: BankAccount,
    pub entry_count: usize,
    pub latest_period: Option<Period>,
    pub latest_balance: Option<Decimal>,
}

/// The accounts with the total of their latest balances (CSD-009 REQ-17). The
/// total is `None` while no account has an entry.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct BankOverview {
    pub accounts: Vec<BankAccountSummary>,
    pub total: Option<Decimal>,
}

impl BankAccounts {
    /// The accounts with their latest balances and the total (REQ-17). Periods
    /// are not aligned: each account counts with its own latest entry.
    #[must_use]
    pub fn overview(&self) -> BankOverview {
        let accounts: Vec<BankAccountSummary> = self
            .accounts()
            .into_iter()
            .map(|account| {
                let latest = self.latest(account.id);
                BankAccountSummary {
                    account: account.clone(),
                    entry_count: self.entry_count(account.id),
                    latest_period: latest.map(|entry| entry.period),
                    latest_balance: latest.map(|entry| entry.balance.value()),
                }
            })
            .collect();
        let mut balances = accounts.iter().filter_map(|summary| summary.latest_balance);
        let total = balances
            .next()
            .map(|first| balances.fold(first, |sum, balance| sum + balance));
        BankOverview { accounts, total }
    }
}
