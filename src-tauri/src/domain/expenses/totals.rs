//! Yearly totals over all expenses (CSD-013 REQ-12).

use std::collections::BTreeMap;

use rust_decimal::Decimal;
use serde::Serialize;

use super::Expenses;

/// The totals of one calendar year (CSD-013 REQ-12).
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct ExpenseYearlyTotal {
    pub year: i32,
    pub total: Decimal,
    pub expense_count: usize,
    /// The highest amount of a single expense this year.
    pub largest: Decimal,
}

impl Expenses {
    /// Totals per calendar year with at least one expense, newest year first,
    /// without extrapolating partial years (REQ-12).
    #[must_use]
    pub fn yearly_totals(&self) -> Vec<ExpenseYearlyTotal> {
        let mut years: BTreeMap<i32, ExpenseYearlyTotal> = BTreeMap::new();
        for expense in &self.expenses {
            let amount = expense.amount.value();
            let row = years
                .entry(expense.date.year())
                .or_insert_with(|| ExpenseYearlyTotal {
                    year: expense.date.year(),
                    total: Decimal::new(0, 2),
                    expense_count: 0,
                    largest: amount,
                });
            row.total += amount;
            row.expense_count += 1;
            row.largest = row.largest.max(amount);
        }
        years.into_values().rev().collect()
    }
}
