//! Jobs: their id, pay period and the rules for the fields the user types (CSD-001 REQ-1 to REQ-4).

use std::fmt;

use serde::{Deserialize, Serialize};
use time::Date;
use uuid::Uuid;

use crate::domain::error::DomainError;

use super::dates::parse_date;

const MAX_TEXT_CHARS: usize = 200;

time::serde::format_description!(iso_date, Date, "[year]-[month]-[day]");

/// Identifies a job.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct JobId(Uuid);

impl JobId {
    pub(super) fn generate() -> Self {
        Self(Uuid::new_v4())
    }

    /// Parses the text form returned to the frontend.
    ///
    /// # Errors
    /// Returns `JobNotFound` for text that is not a job id.
    pub fn parse(text: &str) -> Result<Self, DomainError> {
        Uuid::parse_str(text.trim())
            .map(Self)
            .map_err(|_| DomainError::JobNotFound)
    }
}

impl fmt::Display for JobId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

/// How a job is recorded (CSD-001 REQ-1): one paycheck per month, or one
/// annual statement per calendar year.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum PayPeriod {
    #[default]
    Monthly,
    Yearly,
}

impl PayPeriod {
    /// Parses `"Monthly"` or `"Yearly"` (case-insensitive).
    #[must_use]
    pub fn parse(text: &str) -> Option<Self> {
        match text.trim().to_ascii_lowercase().as_str() {
            "monthly" => Some(Self::Monthly),
            "yearly" => Some(Self::Yearly),
            _ => None,
        }
    }
}

/// One employment relationship (CSD-001 REQ-1).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Job {
    pub id: JobId,
    pub employer: String,
    pub title: Option<String>,
    #[serde(with = "iso_date")]
    pub start_date: Date,
    #[serde(default, with = "iso_date::option")]
    pub end_date: Option<Date>,
    /// A job without a pay period (an older file) is monthly.
    #[serde(default)]
    pub pay_period: PayPeriod,
}

/// Job fields as typed by the user, before validation.
#[derive(Clone, Debug, Default, Deserialize)]
pub struct JobInput {
    #[serde(default)]
    pub employer: String,
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub start_date: Option<String>,
    #[serde(default)]
    pub end_date: Option<String>,
    /// `None` means Monthly when creating and "unchanged" when editing.
    #[serde(default)]
    pub pay_period: Option<String>,
}

pub(super) struct ValidJob {
    pub(super) employer: String,
    pub(super) title: Option<String>,
    pub(super) start_date: Date,
    pub(super) end_date: Option<Date>,
    pub(super) pay_period: Option<PayPeriod>,
}

impl JobInput {
    pub(super) fn validate(&self) -> Result<ValidJob, DomainError> {
        let employer = self.employer.trim();
        if employer.is_empty() {
            return Err(DomainError::JobEmployerRequired);
        }
        let title = non_blank(self.title.as_deref());
        if employer.chars().count() > MAX_TEXT_CHARS
            || title.is_some_and(|title| title.chars().count() > MAX_TEXT_CHARS)
        {
            return Err(DomainError::JobFieldTooLong);
        }
        let start_date = non_blank(self.start_date.as_deref())
            .ok_or(DomainError::JobStartRequired)
            .and_then(parse_date)?;
        let end_date = non_blank(self.end_date.as_deref())
            .map(parse_date)
            .transpose()?;
        let pay_period = non_blank(self.pay_period.as_deref())
            .map(|text| PayPeriod::parse(text).ok_or(DomainError::JobPeriodUnsupported))
            .transpose()?;
        if end_date.is_some_and(|end| end < start_date) {
            return Err(DomainError::JobEndBeforeStart);
        }
        Ok(ValidJob {
            employer: employer.to_owned(),
            title: title.map(str::to_owned),
            start_date,
            end_date,
            pay_period,
        })
    }
}

fn non_blank(text: Option<&str>) -> Option<&str> {
    text.map(str::trim).filter(|text| !text.is_empty())
}
