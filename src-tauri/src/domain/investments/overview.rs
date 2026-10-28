//! The accounts with their latest balances and the total (CSD-005 REQ-15).

use rust_decimal::Decimal;
use serde::Serialize;

use super::{InvestmentAccount, Investments, Period};

/// One account row of the Investments view: its latest balance and period
/// (CSD-005 REQ-15).
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct AccountSummary {
    #[serde(flatten)]
    pub account: InvestmentAccount,
    pub statement_count: usize,
    pub latest_period: Option<Period>,
    pub latest_balance: Option<Decimal>,
}

/// The accounts with the total of their latest balances (CSD-005 REQ-15). The
/// total is `None` while no account has a statement.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct InvestmentsOverview {
    pub accounts: Vec<AccountSummary>,
    pub total: Option<Decimal>,
}

impl Investments {
    /// The accounts with their latest balances and the total (REQ-15). Periods
    /// are not aligned: each account counts with its own latest statement.
    #[must_use]
    pub fn overview(&self) -> InvestmentsOverview {
        let accounts: Vec<AccountSummary> = self
            .accounts()
            .into_iter()
            .map(|account| {
                let latest = self.latest(account.id);
                AccountSummary {
                    account: account.clone(),
                    statement_count: self.statement_count(account.id),
                    latest_period: latest.map(|statement| statement.period),
                    latest_balance: latest.map(|statement| statement.balance.value()),
                }
            })
            .collect();
        let mut balances = accounts.iter().filter_map(|summary| summary.latest_balance);
        let total = balances
            .next()
            .map(|first| balances.fold(first, |sum, balance| sum + balance));
        InvestmentsOverview { accounts, total }
    }
}
