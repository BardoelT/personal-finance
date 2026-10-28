//! Paychecks, annual statements and the entries that cover both (CSD-001 REQ-5 to REQ-9).

use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

use crate::domain::money::Amounts;

use super::{JobId, Year, YearMonth};

/// The monthly pay record of one job (CSD-001 REQ-5).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Paycheck {
    pub job_id: JobId,
    pub month: YearMonth,
    #[serde(flatten)]
    pub amounts: Amounts,
}

/// The yearly pay record of one job (CSD-001 REQ-25).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Statement {
    pub job_id: JobId,
    pub year: Year,
    #[serde(flatten)]
    pub amounts: Amounts,
}

/// A paycheck or an annual statement (CSD-001 "entry").
#[derive(Clone, Copy, Debug)]
pub enum Entry<'a> {
    Paycheck(&'a Paycheck),
    Statement(&'a Statement),
}

impl<'a> Entry<'a> {
    /// The job the entry belongs to.
    #[must_use]
    pub fn job_id(&self) -> JobId {
        match self {
            Self::Paycheck(paycheck) => paycheck.job_id,
            Self::Statement(statement) => statement.job_id,
        }
    }

    /// The amounts of the entry.
    #[must_use]
    pub fn amounts(&self) -> &'a Amounts {
        match self {
            Self::Paycheck(paycheck) => &paycheck.amounts,
            Self::Statement(statement) => &statement.amounts,
        }
    }

    /// The period as shown: `YYYY-MM` for a paycheck, `YYYY` for a statement.
    #[must_use]
    pub fn period(&self) -> String {
        match self {
            Self::Paycheck(paycheck) => paycheck.month.to_string(),
            Self::Statement(statement) => statement.year.to_string(),
        }
    }

    /// The calendar year the entry counts in.
    #[must_use]
    pub fn year(&self) -> i32 {
        match self {
            Self::Paycheck(paycheck) => paycheck.month.year(),
            Self::Statement(statement) => statement.year.value(),
        }
    }

    /// Whether this is an annual statement.
    #[must_use]
    pub fn is_statement(&self) -> bool {
        matches!(self, Self::Statement(_))
    }

    /// Newest first ordering: an annual statement counts as December of its year
    /// (CSD-001 REQ-12).
    pub(super) fn sort_key(&self) -> (i32, u8) {
        match self {
            Self::Paycheck(paycheck) => (paycheck.month.year(), paycheck.month.month()),
            Self::Statement(statement) => (statement.year.value(), 12),
        }
    }
}

/// Form values for "copy previous month" (CSD-001 REQ-9).
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct PaycheckPrefill {
    pub gross_salary: Option<Decimal>,
    pub holiday_pay: Decimal,
    pub bonus: Decimal,
}
