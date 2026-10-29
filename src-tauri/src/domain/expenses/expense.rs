//! One expense and the fields the user types for it (CSD-013 REQ-1 to REQ-7).

use std::fmt;

use serde::{Deserialize, Serialize};
use time::Date;
use uuid::Uuid;

use crate::domain::error::DomainError;
use crate::domain::investments::Note;

use super::{expense_note, Category, ExpenseAmount, ExpenseDate};

pub(super) const MAX_NAME_CHARS: usize = 100;

/// Identifies an expense.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ExpenseId(Uuid);

impl ExpenseId {
    pub(super) fn generate() -> Self {
        Self(Uuid::new_v4())
    }

    /// Parses the text form returned to the frontend.
    ///
    /// # Errors
    /// Returns `ExpenseNotFound` for text that is not an expense id.
    pub fn parse(text: &str) -> Result<Self, DomainError> {
        Uuid::parse_str(text.trim())
            .map(Self)
            .map_err(|_| DomainError::ExpenseNotFound)
    }
}

impl fmt::Display for ExpenseId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.0, f)
    }
}

/// One purchase or cost (CSD-013 section 3).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Expense {
    pub id: ExpenseId,
    pub name: String,
    pub amount: ExpenseAmount,
    pub date: ExpenseDate,
    pub category: Category,
    #[serde(default)]
    pub note: Note,
}

/// Expense fields as typed by the user, before validation.
#[derive(Clone, Debug, Default, Deserialize)]
pub struct ExpenseInput {
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub amount: String,
    /// `dd/mm/yyyy`.
    #[serde(default)]
    pub date: String,
    #[serde(default)]
    pub category: String,
    #[serde(default)]
    pub note: Option<String>,
}

/// The values of an input that follow every rule, ready to be stored.
pub(super) struct ValidExpense {
    pub(super) name: String,
    pub(super) amount: ExpenseAmount,
    pub(super) date: ExpenseDate,
    pub(super) category: Category,
    pub(super) note: Note,
}

impl ExpenseInput {
    /// Checks every field, in the order name, amount, date, category and note,
    /// and returns the first violated rule. `today` is the local date.
    pub(super) fn validate(&self, today: Date) -> Result<ValidExpense, DomainError> {
        let name = self.name.trim();
        if name.is_empty() {
            return Err(DomainError::ExpenseNameRequired);
        }
        if name.chars().count() > MAX_NAME_CHARS {
            return Err(DomainError::ExpenseNameTooLong);
        }
        Ok(ValidExpense {
            name: name.to_owned(),
            amount: ExpenseAmount::parse(&self.amount)?,
            date: ExpenseDate::parse(&self.date, today)?,
            category: Category::parse(&self.category)?,
            note: expense_note(self.note.as_deref().unwrap_or_default())?,
        })
    }
}
