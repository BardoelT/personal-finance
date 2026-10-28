//! The period of a statement: a calendar month (`YYYY-MM`) or a calendar year (`YYYY`).

use std::fmt;

use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::domain::error::DomainError;
use crate::domain::income::{PayPeriod, Year, YearMonth};

/// The period of one statement: a calendar month (`YYYY-MM`) or a calendar
/// year (`YYYY`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Period {
    Month(YearMonth),
    Year(Year),
}

impl Period {
    /// Parses `YYYY-MM` or `YYYY`; the shape decides which of the two it is.
    ///
    /// # Errors
    /// Returns `InvestmentStatementPeriodInvalid` for any other text.
    pub fn parse(text: &str) -> Result<Self, DomainError> {
        let invalid = |_| DomainError::InvestmentStatementPeriodInvalid;
        let text = text.trim();
        if text.contains('-') {
            YearMonth::parse(text).map(Self::Month).map_err(invalid)
        } else {
            Year::parse(text).map(Self::Year).map_err(invalid)
        }
    }

    /// The statement period an account needs to hold this period.
    #[must_use]
    pub fn kind(self) -> PayPeriod {
        match self {
            Self::Month(_) => PayPeriod::Monthly,
            Self::Year(_) => PayPeriod::Yearly,
        }
    }

    /// The calendar year of the period; a statement counts in this year only.
    pub(super) fn year(self) -> i32 {
        self.sort_key().0
    }

    /// Newest-first ordering key: a year counts as December (CSD-005 REQ-16).
    pub(super) fn sort_key(self) -> (i32, u8) {
        match self {
            Self::Month(month) => (month.year(), month.month()),
            Self::Year(year) => (year.value(), 12),
        }
    }

    /// Whether the period starts after the current month `today` (REQ-6).
    pub(super) fn is_after(self, today: YearMonth) -> bool {
        match self {
            Self::Month(month) => month > today,
            Self::Year(year) => year.value() > today.year(),
        }
    }
}

impl fmt::Display for Period {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Month(month) => fmt::Display::fmt(month, f),
            Self::Year(year) => fmt::Display::fmt(year, f),
        }
    }
}

impl Serialize for Period {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.collect_str(self)
    }
}

impl<'de> Deserialize<'de> for Period {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let text = String::deserialize(deserializer)?;
        Self::parse(&text).map_err(serde::de::Error::custom)
    }
}
