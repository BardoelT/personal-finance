//! Calendar months (`YYYY-MM`), years (`YYYY`) and dates, as the income and investments domains use them.

use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Deserializer, Serialize, Serializer};
use time::macros::format_description;
use time::Date;

use crate::domain::error::DomainError;

const FIRST_YEAR: i32 = 1900;

const LAST_YEAR: i32 = 9999;

/// Parses a date as the user types it: `dd/mm/yyyy` with a two-digit day and
/// month (CSD-001 REQ-22). Other formats and impossible dates are rejected.
pub(super) fn parse_date(text: &str) -> Result<Date, DomainError> {
    Date::parse(text.trim(), format_description!("[day]/[month]/[year]"))
        .map_err(|_| DomainError::JobDateInvalid)
}

/// A calendar month, written `YYYY-MM`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct YearMonth {
    year: i32,
    month: u8,
}

impl YearMonth {
    /// Builds a month if `month` is 1 to 12 and `year` is 1900 to 9999.
    #[must_use]
    pub fn new(year: i32, month: u8) -> Option<Self> {
        ((FIRST_YEAR..=LAST_YEAR).contains(&year) && (1..=12).contains(&month))
            .then_some(Self { year, month })
    }

    /// The month a date falls in.
    #[must_use]
    pub fn from_date(date: Date) -> Self {
        Self {
            year: date.year(),
            month: u8::from(date.month()),
        }
    }

    /// Parses `YYYY-MM`.
    ///
    /// # Errors
    /// Returns `PaycheckMonthInvalid` for any other text.
    pub fn parse(text: &str) -> Result<Self, DomainError> {
        let invalid = || DomainError::PaycheckMonthInvalid;
        let (year, month) = text.trim().split_once('-').ok_or_else(invalid)?;
        let plain =
            |part: &str, len: usize| part.len() == len && part.bytes().all(|b| b.is_ascii_digit());
        if !plain(year, 4) || !plain(month, 2) {
            return Err(invalid());
        }
        let year = year.parse().map_err(|_| invalid())?;
        let month = month.parse().map_err(|_| invalid())?;
        Self::new(year, month).ok_or_else(invalid)
    }

    /// The calendar year.
    #[must_use]
    pub fn year(self) -> i32 {
        self.year
    }

    /// The month number, 1 to 12.
    #[must_use]
    pub fn month(self) -> u8 {
        self.month
    }

    /// The month before this one; January looks back to December. `None` only
    /// at the start of the supported years.
    #[must_use]
    pub fn previous(self) -> Option<Self> {
        if self.month == 1 {
            Self::new(self.year - 1, 12)
        } else {
            Self::new(self.year, self.month - 1)
        }
    }
}

impl fmt::Display for YearMonth {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:04}-{:02}", self.year, self.month)
    }
}

impl FromStr for YearMonth {
    type Err = DomainError;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        Self::parse(text)
    }
}

impl Serialize for YearMonth {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.collect_str(self)
    }
}

impl<'de> Deserialize<'de> for YearMonth {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let text = String::deserialize(deserializer)?;
        Self::parse(&text).map_err(serde::de::Error::custom)
    }
}

/// A calendar year, written `YYYY` (CSD-001 REQ-25).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Year(i32);

impl Year {
    /// Builds a year from 1900 to 9999.
    #[must_use]
    pub fn new(year: i32) -> Option<Self> {
        (FIRST_YEAR..=LAST_YEAR)
            .contains(&year)
            .then_some(Self(year))
    }

    /// Parses `YYYY`.
    ///
    /// # Errors
    /// Returns `StatementYearInvalid` for any other text.
    pub fn parse(text: &str) -> Result<Self, DomainError> {
        let invalid = || DomainError::StatementYearInvalid;
        let text = text.trim();
        if text.len() != 4 || !text.bytes().all(|b| b.is_ascii_digit()) {
            return Err(invalid());
        }
        text.parse().ok().and_then(Self::new).ok_or_else(invalid)
    }

    /// The year as a number.
    #[must_use]
    pub fn value(self) -> i32 {
        self.0
    }
}

impl fmt::Display for Year {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:04}", self.0)
    }
}

impl Serialize for Year {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.collect_str(self)
    }
}

impl<'de> Deserialize<'de> for Year {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let text = String::deserialize(deserializer)?;
        Self::parse(&text).map_err(serde::de::Error::custom)
    }
}
