//! The yearly ratios between income, wealth, growth and expenses (CSD-018).

use std::collections::BTreeMap;

use rust_decimal::Decimal;
use serde::Serialize;

use crate::domain::bank_accounts::BankAccounts;
use crate::domain::debts::Debts;
use crate::domain::expenses::Expenses;
use crate::domain::houses::Houses;
use crate::domain::income::{Income, YearMonth};
use crate::domain::investments::Investments;
use crate::domain::money::{percent_of, zero};
use crate::domain::net_worth::net_worth_insights;

#[cfg(test)]
mod tests;

/// One column of the ratios table (CSD-018 REQ-3). A figure that cannot be
/// computed is `None`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct RatiosYear {
    pub year: i32,
    pub is_partial: bool,
    /// The total gross of the year, 0.00 without income (REQ-4).
    pub total_income: Decimal,
    /// Bank increase plus net deposits plus repaid (REQ-5).
    pub wealth_accumulation: Option<Decimal>,
    /// The wealth accumulation in percent of the total income (REQ-6).
    pub wealth_percent: Option<Decimal>,
    /// The growth percentage of the Investments chart (REQ-7).
    pub investment_growth_percent: Option<Decimal>,
    /// The net worth against that of the year before, in percent (REQ-8).
    pub wealth_growth_percent: Option<Decimal>,
    /// Wealth accumulation plus expenses (REQ-9).
    pub disposable_income: Option<Decimal>,
    /// The disposable income in percent of the total income (REQ-9).
    pub disposable_percent: Option<Decimal>,
}

/// The payload of `get_ratios` (CSD-018 section 5).
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Ratios {
    pub years: Vec<RatiosYear>,
}

/// What the ratios are computed from: the data of every area.
pub struct RatioSources<'a> {
    pub income: &'a Income,
    pub expenses: &'a Expenses,
    pub houses: &'a Houses,
    pub investments: &'a Investments,
    pub bank_accounts: &'a BankAccounts,
    pub debts: &'a Debts,
}

/// One column per calendar year from the earliest year in which any area has a
/// record up to and including the current year, oldest first (REQ-2). Every
/// figure comes from the code that gives it to its own page (CON-2).
#[must_use]
pub fn ratios(sources: &RatioSources<'_>, today: YearMonth) -> Ratios {
    let income: BTreeMap<i32, Decimal> = sources
        .income
        .yearly_totals()
        .into_iter()
        .map(|total| (total.year, total.total_gross))
        .collect();
    let expenses: BTreeMap<i32, Decimal> = sources
        .expenses
        .yearly_totals()
        .into_iter()
        .map(|total| (total.year, total.total))
        .collect();
    let deposits: BTreeMap<i32, Decimal> = sources
        .investments
        .yearly_totals()
        .into_iter()
        .map(|total| (total.year, total.net_deposits))
        .collect();
    let repaid: BTreeMap<i32, Option<Decimal>> = sources
        .debts
        .yearly_totals()
        .into_iter()
        .map(|total| {
            // A Cancelled debt was not paid with cash (CSD-020 REQ-12).
            let cancelled = sources.debts.cancelled_in(total.year);
            (total.year, total.repaid.map(|repaid| repaid - cancelled))
        })
        .collect();
    let investment_growth: BTreeMap<i32, Option<Decimal>> = sources
        .investments
        .insights(today)
        .into_iter()
        .map(|row| (row.year, row.growth_percent))
        .collect();
    let net_worth: BTreeMap<i32, (bool, Decimal)> = net_worth_insights(
        sources.houses,
        sources.investments,
        sources.bank_accounts,
        sources.debts,
        today,
    )
    .years
    .into_iter()
    .map(|row| (row.year, (row.has_records, row.net_worth)))
    .collect();

    let earliest = [&income, &expenses]
        .iter()
        .filter_map(|series| series.keys().next().copied())
        .chain(net_worth.keys().next().copied())
        .min();
    let Some(first) = earliest else {
        return Ratios { years: Vec::new() };
    };
    let years = (first..=today.year())
        .map(|year| {
            let total_income = income.get(&year).copied().unwrap_or_else(zero);
            let wealth_accumulation = wealth_accumulation(
                sources.bank_accounts.increase(year),
                deposits.get(&year).copied(),
                repaid.get(&year).copied().flatten(),
            );
            let disposable_income = wealth_accumulation
                .map(|wealth| wealth + expenses.get(&year).copied().unwrap_or_else(zero));
            RatiosYear {
                year,
                is_partial: year == today.year(),
                total_income,
                wealth_accumulation,
                wealth_percent: wealth_accumulation
                    .and_then(|wealth| percent_of(wealth, total_income)),
                investment_growth_percent: investment_growth.get(&year).copied().flatten(),
                wealth_growth_percent: wealth_growth(&net_worth, year),
                disposable_income,
                disposable_percent: disposable_income
                    .and_then(|disposable| percent_of(disposable, total_income)),
            }
        })
        .collect();
    Ratios { years }
}

/// The three parts added up, a part that is none counting as 0.00, or none
/// when every part is none (REQ-5).
fn wealth_accumulation(
    bank_increase: Option<Decimal>,
    net_deposits: Option<Decimal>,
    repaid: Option<Decimal>,
) -> Option<Decimal> {
    if bank_increase.is_none() && net_deposits.is_none() && repaid.is_none() {
        return None;
    }
    Some(
        bank_increase.unwrap_or_else(zero)
            + net_deposits.unwrap_or_else(zero)
            + repaid.unwrap_or_else(zero),
    )
}

/// The net worth of `year` against that of the year before, in percent. None
/// unless both years have records and the earlier net worth is positive
/// (REQ-8).
fn wealth_growth(net_worth: &BTreeMap<i32, (bool, Decimal)>, year: i32) -> Option<Decimal> {
    let (has_records, now) = *net_worth.get(&year)?;
    let (earlier_has_records, before) = *net_worth.get(&(year - 1))?;
    if !has_records || !earlier_has_records || before <= Decimal::ZERO {
        return None;
    }
    percent_of(now - before, before)
}
