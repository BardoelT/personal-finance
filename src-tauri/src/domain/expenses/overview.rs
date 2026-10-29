//! The filtered list of expenses and the form values for "copy previous" (CSD-013 REQ-9 to REQ-11).

use std::cmp::Reverse;
use std::collections::BTreeSet;

use rust_decimal::Decimal;
use serde::Serialize;

use crate::domain::error::DomainError;
use crate::domain::money::zero;

use super::{Category, Expense, Expenses};

/// The expenses a filter lets through, with their total and the years that
/// can be chosen (CSD-013 REQ-11).
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct ExpenseList {
    pub expenses: Vec<Expense>,
    /// The sum of the amounts of `expenses`.
    pub total: Decimal,
    /// Every year with at least one expense, newest first, whatever the filter.
    pub years: Vec<i32>,
}

/// Form values for "Copy previous expense": the date and the note are never
/// copied (CSD-013 REQ-9).
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct ExpensePrefill {
    pub name: String,
    pub amount: Decimal,
    pub category: Category,
}

impl Expenses {
    /// The expenses of a year and, with a month, of that month of the year,
    /// newest date first, then by name ignoring case, then in the order they
    /// were added (REQ-10, REQ-11). Without a year nothing is filtered, and a
    /// month without a year is ignored.
    ///
    /// # Errors
    /// Returns `ExpenseDateInvalid` for a month outside 1 to 12.
    pub fn list(&self, year: Option<i32>, month: Option<u8>) -> Result<ExpenseList, DomainError> {
        if month.is_some_and(|month| !(1..=12).contains(&month)) {
            return Err(DomainError::ExpenseDateInvalid);
        }
        let mut expenses: Vec<Expense> = self
            .expenses
            .iter()
            .filter(|expense| {
                year.is_none_or(|year| expense.date.year() == year)
                    && (year.is_none() || month.is_none_or(|month| expense.date.month() == month))
            })
            .cloned()
            .collect();
        expenses.sort_by_cached_key(|expense| (Reverse(expense.date), expense.name.to_lowercase()));
        let total = expenses
            .iter()
            .fold(zero(), |sum, expense| sum + expense.amount.value());
        let years: BTreeSet<i32> = self.expenses.iter().map(|e| e.date.year()).collect();
        Ok(ExpenseList {
            expenses,
            total,
            years: years.into_iter().rev().collect(),
        })
    }

    /// The name, amount and category of the previous expense: the one with the
    /// latest date, and among those the one added last (REQ-9).
    #[must_use]
    pub fn prefill_from_previous(&self) -> Option<ExpensePrefill> {
        self.expenses
            .iter()
            .max_by_key(|expense| expense.date)
            .map(|expense| ExpensePrefill {
                name: expense.name.clone(),
                amount: expense.amount.value(),
                category: expense.category,
            })
    }
}
