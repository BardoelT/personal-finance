//! The yearly series behind the Investments Insights chart (CSD-007).

use std::collections::BTreeMap;

use rust_decimal::Decimal;
use serde::Serialize;

use crate::domain::income::YearMonth;
use crate::domain::money::{percent_of, zero};

use super::{InvestmentYearlyTotal, Investments};

/// One year of the insights chart: the parts of the closing balance, whether
/// the year is still running and its growth percentage (CSD-007).
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct InvestmentInsightsYear {
    pub year: i32,
    pub opening_balance: Decimal,
    pub net_deposits: Decimal,
    pub capital_growth: Decimal,
    /// Capital growth of the accounts with an opening balance, in percent of
    /// the opening balance, one decimal; `None` when the opening balance is zero.
    pub growth_percent: Option<Decimal>,
    pub closing_balance: Decimal,
    pub statement_count: usize,
    pub is_partial: bool,
}

/// Capital growth in percent of the opening balance (CSD-007 REQ-6). `None`
/// when the opening balance is zero.
fn growth_percent(growth: Decimal, opening: Decimal) -> Option<Decimal> {
    percent_of(growth, opening)
}

impl Investments {
    /// One figure set per calendar year from the first statement up to and
    /// including the current year, oldest first (CSD-007). Years without
    /// statements are zero. The current year is partial. Statements after the
    /// current year, which only a project file can contain, are not shown.
    #[must_use]
    pub fn insights(&self, today: YearMonth) -> Vec<InvestmentInsightsYear> {
        let totals: BTreeMap<i32, InvestmentYearlyTotal> = self
            .yearly_totals()
            .into_iter()
            .map(|total| (total.year, total))
            .collect();
        let Some(&first) = totals.keys().next() else {
            return Vec::new();
        };
        (first..=today.year())
            .map(|year| {
                let (net_deposits, capital_growth, closing_balance, statement_count) =
                    totals.get(&year).map_or_else(
                        || (zero(), zero(), zero(), 0),
                        |total| {
                            (
                                total.net_deposits,
                                total.capital_growth,
                                total.closing_balance,
                                total.statement_count,
                            )
                        },
                    );
                let opening_balance = self.opening_balance(year);
                InvestmentInsightsYear {
                    year,
                    opening_balance,
                    net_deposits,
                    capital_growth,
                    growth_percent: growth_percent(self.growth_with_opening(year), opening_balance),
                    closing_balance,
                    statement_count,
                    is_partial: year == today.year(),
                }
            })
            .collect()
    }

    /// The balance a year opens with: for each account with a statement in
    /// the year, its latest balance before the year (CSD-007 section 3).
    fn opening_balance(&self, year: i32) -> Decimal {
        self.accounts
            .iter()
            .filter(|account| {
                self.statements
                    .iter()
                    .any(|s| s.account_id == account.id && s.period.year() == year)
            })
            .filter_map(|account| {
                self.statements
                    .iter()
                    .filter(|s| s.account_id == account.id && s.period.year() < year)
                    .max_by_key(|s| s.period.sort_key())
                    .map(|s| s.balance.value())
            })
            .fold(zero(), |sum, balance| sum + balance)
    }

    /// The capital growth in a year of the accounts that have an opening
    /// balance in it: a statement in the year and one before it (CSD-007 REQ-6).
    fn growth_with_opening(&self, year: i32) -> Decimal {
        self.accounts
            .iter()
            .filter(|account| {
                self.statements
                    .iter()
                    .any(|s| s.account_id == account.id && s.period.year() < year)
            })
            .flat_map(|account| {
                self.statements
                    .iter()
                    .filter(move |s| s.account_id == account.id && s.period.year() == year)
            })
            .fold(zero(), |sum, statement| {
                sum + self.capital_growth(statement)
            })
    }
}
