//! The date of an expense (CSD-013 REQ-4).

use serde::de::Error as _;
use serde::ser::Error as _;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use time::macros::format_description;
use time::Date;

use crate::domain::error::DomainError;

const FIRST_YEAR: i32 = 1900;

/// A calendar day from 01/01/1900. The user types and sees `dd/mm/yyyy`; the
/// project file and IPC hold `YYYY-MM-DD`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ExpenseDate(Date);

impl ExpenseDate {
    /// Builds a date, which must not be before 01/01/1900.
    ///
    /// # Errors
    /// Returns `ExpenseDateInvalid`.
    pub fn new(date: Date) -> Result<Self, DomainError> {
        if date.year() < FIRST_YEAR {
            return Err(DomainError::ExpenseDateInvalid);
        }
        Ok(Self(date))
    }

    /// Parses `dd/mm/yyyy` as typed by the user. `today` is valid, a later
    /// date is not.
    ///
    /// # Errors
    /// Returns `ExpenseDateInvalid` for text that is not a real date from
    /// 01/01/1900, and `ExpenseDateInFuture` for a date after `today`.
    pub fn parse(text: &str, today: Date) -> Result<Self, DomainError> {
        let date = Date::parse(text.trim(), format_description!("[day]/[month]/[year]"))
            .map_err(|_| DomainError::ExpenseDateInvalid)
            .and_then(Self::new)?;
        if date.0 > today {
            return Err(DomainError::ExpenseDateInFuture);
        }
        Ok(date)
    }

    /// The calendar year.
    #[must_use]
    pub fn year(self) -> i32 {
        self.0.year()
    }

    /// The calendar month, 1 to 12.
    #[cfg(test)]
    #[must_use]
    pub fn month(self) -> u8 {
        u8::from(self.0.month())
    }
}

impl Serialize for ExpenseDate {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.0
            .format(format_description!("[year]-[month]-[day]"))
            .map_err(S::Error::custom)
            .and_then(|text| serializer.serialize_str(&text))
    }
}

impl<'de> Deserialize<'de> for ExpenseDate {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let text = String::deserialize(deserializer)?;
        Date::parse(&text, format_description!("[year]-[month]-[day]"))
            .map_err(D::Error::custom)
            .and_then(|date| Self::new(date).map_err(D::Error::custom))
    }
}
