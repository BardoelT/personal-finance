//! Expenses (CSD-013), and the yearly totals and insights built on them
//! (CSD-013 REQ-12, CSD-014).
//!
//! `Expenses` holds the data and the rules for changing it; the other files add
//! the value types (`amounts`, `category`, `date`, `expense`) and the figures
//! derived from it (`overview`, `totals`, `insights`). The note type is that
//! of the investments (CSD-005).

mod amounts;
mod category;
mod date;
mod expense;
mod insights;
mod overview;
mod totals;

#[cfg(test)]
mod tests;

pub use amounts::{expense_note, ExpenseAmount};
pub use category::Category;
pub use date::ExpenseDate;
pub use expense::{Expense, ExpenseId, ExpenseInput};
pub use insights::ExpenseInsights;
pub use overview::{ExpenseList, ExpensePrefill};
pub use totals::ExpenseYearlyTotal;

use std::collections::HashSet;

use serde::{Deserialize, Serialize};
use time::Date;

use crate::domain::error::DomainError;

use expense::MAX_NAME_CHARS;

/// All expenses, in the order they were added (CSD-013 section 5).
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Expenses {
    #[serde(default)]
    expenses: Vec<Expense>,
}

impl Expenses {
    /// Adds an expense. `today` is the local date; later dates are rejected
    /// (REQ-1 to REQ-6).
    ///
    /// # Errors
    /// Returns the first violated rule of the name, amount, date, category or
    /// note.
    pub fn add_expense(
        &mut self,
        input: &ExpenseInput,
        today: Date,
    ) -> Result<Expense, DomainError> {
        let valid = input.validate(today)?;
        let expense = Expense {
            id: ExpenseId::generate(),
            name: valid.name,
            amount: valid.amount,
            date: valid.date,
            category: valid.category,
            note: valid.note,
        };
        self.expenses.push(expense.clone());
        Ok(expense)
    }

    /// Replaces every field of an expense as one change, or none when a value
    /// is invalid (REQ-7).
    ///
    /// # Errors
    /// Returns `ExpenseNotFound` or the first violated rule.
    pub fn update_expense(
        &mut self,
        id: ExpenseId,
        input: &ExpenseInput,
        today: Date,
    ) -> Result<Expense, DomainError> {
        let valid = input.validate(today)?;
        let expense = self
            .expenses
            .iter_mut()
            .find(|expense| expense.id == id)
            .ok_or(DomainError::ExpenseNotFound)?;
        expense.name = valid.name;
        expense.amount = valid.amount;
        expense.date = valid.date;
        expense.category = valid.category;
        expense.note = valid.note;
        Ok(expense.clone())
    }

    /// Removes one expense and nothing else (REQ-8). The caller asks for the
    /// confirmation.
    ///
    /// # Errors
    /// Returns `ExpenseNotFound`.
    pub fn delete_expense(&mut self, id: ExpenseId) -> Result<(), DomainError> {
        let before = self.expenses.len();
        self.expenses.retain(|expense| expense.id != id);
        if self.expenses.len() == before {
            return Err(DomainError::ExpenseNotFound);
        }
        Ok(())
    }

    /// How many expenses there are.
    #[must_use]
    pub fn total_count(&self) -> usize {
        self.expenses.len()
    }

    /// Checks that data read from a file follows the rules of CSD-013, so a
    /// hand-edited or damaged file cannot break them. The date is not checked
    /// against today (REQ-16).
    ///
    /// # Errors
    /// Returns `ExpenseDataInvalid` for the first inconsistency found.
    pub fn validate(&self) -> Result<(), DomainError> {
        let invalid = DomainError::ExpenseDataInvalid;
        let mut ids = HashSet::new();
        for expense in &self.expenses {
            let name = expense.name.trim();
            if name.is_empty()
                || name.chars().count() > MAX_NAME_CHARS
                || !ids.insert(expense.id)
                || ExpenseAmount::new(expense.amount.value()).is_err()
                || expense_note(expense.note.as_str()).is_err()
            {
                return Err(invalid);
            }
        }
        Ok(())
    }
}
