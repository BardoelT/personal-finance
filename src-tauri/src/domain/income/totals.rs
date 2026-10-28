//! Yearly totals and the dashboard overview over all jobs (CSD-001 REQ-11, CSD-004).

use std::collections::BTreeMap;

use rust_decimal::Decimal;
use serde::Serialize;

use crate::domain::money::{zero, Amounts};

use super::{Entry, Income, Job, YearMonth};

/// The sums of some entries, or `None` if there are none.
fn totals_of<'a>(amounts: impl Iterator<Item = &'a Amounts>) -> Option<IncomePeriodTotals> {
    let mut totals = IncomePeriodTotals {
        gross_salary: zero(),
        holiday_pay: zero(),
        bonus: zero(),
        total_gross: zero(),
        entry_count: 0,
    };
    for amounts in amounts {
        totals.gross_salary += amounts.gross_salary();
        totals.holiday_pay += amounts.holiday_pay();
        totals.bonus += amounts.bonus();
        totals.total_gross += amounts.total_gross();
        totals.entry_count += 1;
    }
    (totals.entry_count > 0).then_some(totals)
}

/// Totals over all jobs for one calendar year (CSD-001 REQ-11).
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct IncomeYearlyTotal {
    pub year: i32,
    pub gross_salary: Decimal,
    pub holiday_pay: Decimal,
    pub bonus: Decimal,
    pub total_gross: Decimal,
    pub entry_count: usize,
}

/// Sums of a set of entries over all jobs (CSD-004).
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct IncomePeriodTotals {
    pub gross_salary: Decimal,
    pub holiday_pay: Decimal,
    pub bonus: Decimal,
    pub total_gross: Decimal,
    pub entry_count: usize,
}

/// The two blocks of the dashboard (CSD-004): this year up to the current
/// month, and last month. A period without entries has no totals.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct IncomeOverview {
    pub year: i32,
    pub year_to_date: Option<IncomePeriodTotals>,
    pub last_month: Option<YearMonth>,
    pub last_month_totals: Option<IncomePeriodTotals>,
}

impl Income {
    /// Income this year up to and including `today`'s month, and last month,
    /// summed over all jobs (CSD-004 REQ-2 to REQ-4). An annual statement of this
    /// year counts in full in the first block and never in the second. Months
    /// after `today`, which only a project file can contain, are not counted.
    #[must_use]
    pub fn overview(&self, today: YearMonth) -> IncomeOverview {
        let last_month = today.previous();
        let year_to_date = totals_of(
            self.paychecks
                .iter()
                .filter(|p| p.month.year() == today.year() && p.month <= today)
                .map(|p| &p.amounts)
                .chain(
                    self.statements
                        .iter()
                        .filter(|s| s.year.value() == today.year())
                        .map(|s| &s.amounts),
                ),
        );
        let last_month_totals = last_month.and_then(|last| {
            totals_of(
                self.paychecks
                    .iter()
                    .filter(|p| p.month == last)
                    .map(|p| &p.amounts),
            )
        });
        IncomeOverview {
            year: today.year(),
            year_to_date,
            last_month,
            last_month_totals,
        }
    }

    /// The entries of all jobs with their job, newest period first (an annual
    /// statement counts as December of its year) and, within a period, in job
    /// order (REQ-12).
    #[must_use]
    pub fn all_entries(&self) -> Vec<(&Job, Entry<'_>)> {
        let mut rows: Vec<(usize, &Job, Entry<'_>)> = Vec::new();
        for (position, job) in self.jobs().into_iter().enumerate() {
            rows.extend(
                self.entries()
                    .filter(|entry| entry.job_id() == job.id)
                    .map(|entry| (position, job, entry)),
            );
        }
        rows.sort_by(|a, b| b.2.sort_key().cmp(&a.2.sort_key()).then(a.0.cmp(&b.0)));
        rows.into_iter()
            .map(|(_, job, entry)| (job, entry))
            .collect()
    }

    /// Sums per calendar year over all jobs, newest year first, without
    /// extrapolating partial years (REQ-11). An annual statement counts in its
    /// own year only.
    #[must_use]
    pub fn yearly_totals(&self) -> Vec<IncomeYearlyTotal> {
        let mut by_year: BTreeMap<i32, IncomeYearlyTotal> = BTreeMap::new();
        for entry in self.entries() {
            let year = entry.year();
            let amounts = entry.amounts();
            let total = by_year.entry(year).or_insert_with(|| IncomeYearlyTotal {
                year,
                gross_salary: zero(),
                holiday_pay: zero(),
                bonus: zero(),
                total_gross: zero(),
                entry_count: 0,
            });
            total.gross_salary += amounts.gross_salary();
            total.holiday_pay += amounts.holiday_pay();
            total.bonus += amounts.bonus();
            total.total_gross += amounts.total_gross();
            total.entry_count += 1;
        }
        by_year.into_values().rev().collect()
    }
}
