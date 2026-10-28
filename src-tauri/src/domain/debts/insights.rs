//! The yearly series behind the Debts Insights chart (CSD-012).

use std::collections::BTreeMap;

use rust_decimal::Decimal;
use serde::Serialize;

use crate::domain::income::YearMonth;
use crate::domain::money::zero;

use super::{DebtId, Debts};

/// A debt in the legend of the chart (CSD-012 REQ-9).
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct InsightsDebt {
    pub id: DebtId,
    pub name: String,
    pub colour_slot: u8,
}

/// The part of a year's bar that belongs to one debt (CSD-012 REQ-5).
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct InsightsSegment {
    pub debt_id: DebtId,
    pub balance: Decimal,
}

/// One year of the insights chart (CSD-012 REQ-2).
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct DebtInsightsYear {
    pub year: i32,
    pub closing_balance: Decimal,
    pub repaid: Option<Decimal>,
    pub statement_count: usize,
    pub is_partial: bool,
    pub segments: Vec<InsightsSegment>,
}

/// The payload of `get_debt_insights` (CSD-012 section 5).
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct DebtInsights {
    pub debts: Vec<InsightsDebt>,
    pub years: Vec<DebtInsightsYear>,
}

impl Debts {
    /// One figure set per calendar year from the first statement up to and
    /// including the current year, oldest first, and the debts that have a
    /// segment in them, in creation order (CSD-012). Years without statements
    /// are zero and have no segments. The current year is partial. Statements
    /// after the current year, which only a project file can contain, are not
    /// shown.
    #[must_use]
    pub fn insights(&self, today: YearMonth) -> DebtInsights {
        let mut counts: BTreeMap<i32, usize> = BTreeMap::new();
        for statement in &self.statements {
            *counts.entry(statement.period.year()).or_default() += 1;
        }
        let Some(&first) = counts.keys().next() else {
            return DebtInsights {
                debts: Vec::new(),
                years: Vec::new(),
            };
        };
        let years: Vec<DebtInsightsYear> = (first..=today.year())
            .map(|year| {
                let segments: Vec<InsightsSegment> = self
                    .year_segments(year)
                    .into_iter()
                    .map(|(debt, balance)| InsightsSegment {
                        debt_id: debt.id,
                        balance,
                    })
                    .collect();
                DebtInsightsYear {
                    year,
                    closing_balance: segments
                        .iter()
                        .fold(zero(), |sum, segment| sum + segment.balance),
                    repaid: self.repaid(year),
                    statement_count: counts.get(&year).copied().unwrap_or(0),
                    is_partial: year == today.year(),
                    segments,
                }
            })
            .collect();
        let debts = self
            .debts
            .iter()
            .filter(|debt| {
                years.iter().any(|year| {
                    year.segments
                        .iter()
                        .any(|segment| segment.debt_id == debt.id)
                })
            })
            .map(|debt| InsightsDebt {
                id: debt.id,
                name: debt.name.clone(),
                colour_slot: debt.colour_slot,
            })
            .collect();
        DebtInsights { debts, years }
    }
}
