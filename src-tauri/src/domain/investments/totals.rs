//! Yearly totals over all accounts (CSD-005 REQ-26).

use std::collections::{BTreeMap, HashMap};

use rust_decimal::Decimal;
use serde::Serialize;

use crate::domain::money::zero;

use super::{InvestmentAccountId, Investments, Statement};

/// The totals of one calendar year over all accounts (CSD-005 REQ-26).
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct InvestmentYearlyTotal {
    pub year: i32,
    /// The sum, over the accounts with a statement this year, of the balance
    /// of their latest statement this year.
    pub closing_balance: Decimal,
    pub net_deposits: Decimal,
    pub capital_growth: Decimal,
    pub statement_count: usize,
}

impl Investments {
    /// Totals per calendar year over all accounts, newest year first, without
    /// extrapolating partial years or carrying balances forward (REQ-26). A
    /// statement counts in the year of its own period only.
    #[must_use]
    pub fn yearly_totals(&self) -> Vec<InvestmentYearlyTotal> {
        let mut by_year: BTreeMap<i32, Vec<&Statement>> = BTreeMap::new();
        for statement in &self.statements {
            by_year
                .entry(statement.period.year())
                .or_default()
                .push(statement);
        }
        by_year
            .into_iter()
            .rev()
            .map(|(year, statements)| self.year_total(year, &statements))
            .collect()
    }

    fn year_total(&self, year: i32, statements: &[&Statement]) -> InvestmentYearlyTotal {
        let mut latest: HashMap<InvestmentAccountId, &Statement> = HashMap::new();
        let mut net_deposits = zero();
        let mut capital_growth = zero();
        for statement in statements {
            net_deposits += statement.net_deposits.value();
            capital_growth += self.capital_growth(statement);
            let newest = latest.entry(statement.account_id).or_insert(statement);
            if statement.period.sort_key() > newest.period.sort_key() {
                *newest = statement;
            }
        }
        let closing_balance = latest
            .values()
            .fold(zero(), |sum, statement| sum + statement.balance.value());
        InvestmentYearlyTotal {
            year,
            closing_balance,
            net_deposits,
            capital_growth,
            statement_count: statements.len(),
        }
    }
}
