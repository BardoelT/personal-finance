//! The list of expenses and the form values for "copy previous" (CSD-013 REQ-9 to REQ-11).

use std::cmp::Reverse;

use rust_decimal::Decimal;
use serde::Serialize;

use crate::domain::money::zero;

use super::{Category, Expense, Expenses};

/// All expenses with their total (CSD-013 REQ-11).
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct ExpenseList {
    pub expenses: Vec<Expense>,
    /// The sum of the amounts of `expenses`.
    pub total: Decimal,
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
    /// All expenses, newest date first, then by name ignoring case, then in the
    /// order they were added (REQ-10), with their total (REQ-11).
    #[must_use]
    pub fn list(&self) -> ExpenseList {
        let mut expenses = self.expenses.clone();
        expenses.sort_by_cached_key(|expense| (Reverse(expense.date), expense.name.to_lowercase()));
        let total = expenses
            .iter()
            .fold(zero(), |sum, expense| sum + expense.amount.value());
        ExpenseList { expenses, total }
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
