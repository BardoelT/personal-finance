use std::collections::BTreeMap;
use std::fmt;
use std::str::FromStr;

use rust_decimal::{Decimal, RoundingStrategy};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use time::macros::format_description;
use time::Date;
use uuid::Uuid;

use super::error::DomainError;
use super::money::{zero, Amounts};

const MAX_TEXT_CHARS: usize = 200;
const FIRST_YEAR: i32 = 1900;
const LAST_YEAR: i32 = 9999;

time::serde::format_description!(iso_date, Date, "[year]-[month]-[day]");

/// Parses a date as the user types it: `dd/mm/yyyy` with a two-digit day and
/// month (CSD-001 REQ-22). Other formats and impossible dates are rejected.
fn parse_date(text: &str) -> Result<Date, DomainError> {
    Date::parse(text.trim(), format_description!("[day]/[month]/[year]"))
        .map_err(|_| DomainError::JobDateInvalid)
}

/// Identifies a job.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct JobId(Uuid);

impl JobId {
    fn generate() -> Self {
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
    fn sort_key(&self) -> (i32, u8) {
        match self {
            Self::Paycheck(paycheck) => (paycheck.month.year(), paycheck.month.month()),
            Self::Statement(statement) => (statement.year.value(), 12),
        }
    }
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

struct ValidJob {
    employer: String,
    title: Option<String>,
    start_date: Date,
    end_date: Option<Date>,
    pay_period: Option<PayPeriod>,
}

impl JobInput {
    fn validate(&self) -> Result<ValidJob, DomainError> {
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

/// The sums of some entries, or `None` if there are none.
fn totals_of<'a>(amounts: impl Iterator<Item = &'a Amounts>) -> Option<PeriodTotals> {
    let mut totals = PeriodTotals {
        gross_salary: zero(),
        holiday_pay: zero(),
        bonus: zero(),
        total_gross: zero(),
        entry_count: 0,
    };
    for amounts in amounts {
        totals.gross_salary += amounts.gross_salary();
        totals.holiday_pay += amounts.holiday_pay();
        totals.bonus += amounts.bonus();
        totals.total_gross += amounts.total_gross();
        totals.entry_count += 1;
    }
    (totals.entry_count > 0).then_some(totals)
}

/// Growth in percent from `previous` to `current`, to one decimal with
/// banker's rounding (CSD-006 REQ-5). `None` when `previous` is zero.
fn growth(previous: Decimal, current: Decimal) -> Option<Decimal> {
    let percent = (current - previous)
        .checked_mul(Decimal::ONE_HUNDRED)?
        .checked_div(previous)?;
    let mut rounded = percent.round_dp_with_strategy(1, RoundingStrategy::MidpointNearestEven);
    if rounded.is_zero() {
        // A decrease that rounds to zero shows no sign.
        rounded = Decimal::ZERO;
    }
    rounded.rescale(1);
    Some(rounded)
}

fn non_blank(text: Option<&str>) -> Option<&str> {
    text.map(str::trim).filter(|text| !text.is_empty())
}

/// Totals over all jobs for one calendar year (CSD-001 REQ-11).
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct YearlyTotal {
    pub year: i32,
    pub gross_salary: Decimal,
    pub holiday_pay: Decimal,
    pub bonus: Decimal,
    pub total_gross: Decimal,
    pub entry_count: usize,
}

/// One year of the insights chart: sums over all jobs, whether the year is
/// still running, and its growth against the year before (CSD-006).
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct InsightsYear {
    pub year: i32,
    pub gross_salary: Decimal,
    pub holiday_pay: Decimal,
    pub bonus: Decimal,
    pub total_gross: Decimal,
    pub entry_count: usize,
    pub is_partial: bool,
    /// Percent with one decimal, `None` when there is nothing to compare.
    pub growth_percent: Option<Decimal>,
}

/// Sums of a set of entries over all jobs (CSD-004).
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct PeriodTotals {
    pub gross_salary: Decimal,
    pub holiday_pay: Decimal,
    pub bonus: Decimal,
    pub total_gross: Decimal,
    pub entry_count: usize,
}

/// The two blocks of the dashboard (CSD-004): this year up to the current
/// month, and last month. A period without entries has no totals.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct IncomeOverview {
    pub year: i32,
    pub year_to_date: Option<PeriodTotals>,
    pub last_month: Option<YearMonth>,
    pub last_month_totals: Option<PeriodTotals>,
}

/// Form values for "copy previous month" (CSD-001 REQ-9).
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct PaycheckPrefill {
    pub gross_salary: Option<Decimal>,
    pub holiday_pay: Decimal,
    pub bonus: Decimal,
}

/// All employment income: jobs with their paychecks or annual statements.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Income {
    jobs: Vec<Job>,
    paychecks: Vec<Paycheck>,
    /// A file without annual statements (an older one) has none.
    #[serde(default)]
    statements: Vec<Statement>,
}

impl Income {
    /// Creates a job, monthly unless the input says Yearly (REQ-1).
    ///
    /// # Errors
    /// Returns the validation error of the first invalid field.
    pub fn create_job(&mut self, input: &JobInput) -> Result<Job, DomainError> {
        let valid = input.validate()?;
        let job = Job {
            id: JobId::generate(),
            employer: valid.employer,
            title: valid.title,
            start_date: valid.start_date,
            end_date: valid.end_date,
            pay_period: valid.pay_period.unwrap_or_default(),
        };
        self.jobs.push(job.clone());
        Ok(job)
    }

    /// Replaces the fields of a job. Dates never conflict with entries, but the
    /// pay period is locked once the job has any entry (REQ-24).
    ///
    /// # Errors
    /// Returns `JobNotFound`, a validation error, or `JobPeriodLocked`.
    pub fn update_job(&mut self, id: JobId, input: &JobInput) -> Result<Job, DomainError> {
        let valid = input.validate()?;
        let has_entries = self.entry_count(id) > 0;
        let job = self.job_mut(id)?;
        let pay_period = valid.pay_period.unwrap_or(job.pay_period);
        if pay_period != job.pay_period && has_entries {
            return Err(DomainError::JobPeriodLocked);
        }
        job.pay_period = pay_period;
        job.employer = valid.employer;
        job.title = valid.title;
        job.start_date = valid.start_date;
        job.end_date = valid.end_date;
        Ok(job.clone())
    }

    /// Removes a job and all its entries (REQ-4).
    ///
    /// # Errors
    /// Returns `JobNotFound`.
    pub fn delete_job(&mut self, id: JobId) -> Result<(), DomainError> {
        self.job(id)?;
        self.jobs.retain(|job| job.id != id);
        self.paychecks.retain(|paycheck| paycheck.job_id != id);
        self.statements.retain(|statement| statement.job_id != id);
        Ok(())
    }

    /// Jobs by start date, newest first (REQ-12).
    #[must_use]
    pub fn jobs(&self) -> Vec<&Job> {
        let mut jobs: Vec<&Job> = self.jobs.iter().collect();
        jobs.sort_by(|a, b| {
            b.start_date
                .cmp(&a.start_date)
                .then_with(|| a.employer.cmp(&b.employer))
                .then_with(|| a.id.to_string().cmp(&b.id.to_string()))
        });
        jobs
    }

    /// How many entries (paychecks and annual statements) a job has.
    #[must_use]
    pub fn entry_count(&self, id: JobId) -> usize {
        self.paychecks.iter().filter(|p| p.job_id == id).count()
            + self.statements.iter().filter(|s| s.job_id == id).count()
    }

    /// Adds the paycheck of a month. `today` is the current month; later months
    /// are rejected. The job's own start and end dates do not matter (REQ-5).
    ///
    /// # Errors
    /// Returns `JobNotFound`, `JobPeriodMismatch` for a yearly job,
    /// `PaycheckMonthInFuture` or `PaycheckDuplicateMonth`.
    pub fn add_paycheck(
        &mut self,
        job_id: JobId,
        month: YearMonth,
        amounts: Amounts,
        today: YearMonth,
    ) -> Result<Paycheck, DomainError> {
        if self.job(job_id)?.pay_period != PayPeriod::Monthly {
            return Err(DomainError::JobPeriodMismatch);
        }
        if month > today {
            return Err(DomainError::PaycheckMonthInFuture);
        }
        if self.paycheck(job_id, month).is_some() {
            return Err(DomainError::PaycheckDuplicateMonth);
        }
        let paycheck = Paycheck {
            job_id,
            month,
            amounts,
        };
        self.paychecks.push(paycheck.clone());
        Ok(paycheck)
    }

    /// Changes the amounts of an existing paycheck. The month is fixed (REQ-8).
    ///
    /// # Errors
    /// Returns `JobNotFound` or `PaycheckNotFound`.
    pub fn update_paycheck(
        &mut self,
        job_id: JobId,
        month: YearMonth,
        amounts: Amounts,
    ) -> Result<Paycheck, DomainError> {
        self.job(job_id)?;
        let paycheck = self
            .paychecks
            .iter_mut()
            .find(|p| p.job_id == job_id && p.month == month)
            .ok_or(DomainError::PaycheckNotFound)?;
        paycheck.amounts = amounts;
        Ok(paycheck.clone())
    }

    /// Removes a paycheck without further confirmation (REQ-8).
    ///
    /// # Errors
    /// Returns `JobNotFound` or `PaycheckNotFound`.
    pub fn delete_paycheck(&mut self, job_id: JobId, month: YearMonth) -> Result<(), DomainError> {
        self.job(job_id)?;
        let before = self.paychecks.len();
        self.paychecks
            .retain(|p| !(p.job_id == job_id && p.month == month));
        if self.paychecks.len() == before {
            return Err(DomainError::PaycheckNotFound);
        }
        Ok(())
    }

    /// Adds the annual statement of a year. `today` is the current month, whose
    /// year is the latest accepted one. The job's own start and end dates do not
    /// matter (REQ-25).
    ///
    /// # Errors
    /// Returns `JobNotFound`, `JobPeriodMismatch` for a monthly job,
    /// `StatementYearInFuture` or `StatementDuplicateYear`.
    pub fn add_statement(
        &mut self,
        job_id: JobId,
        year: Year,
        amounts: Amounts,
        today: YearMonth,
    ) -> Result<Statement, DomainError> {
        if self.job(job_id)?.pay_period != PayPeriod::Yearly {
            return Err(DomainError::JobPeriodMismatch);
        }
        if year.value() > today.year() {
            return Err(DomainError::StatementYearInFuture);
        }
        if self.statement(job_id, year).is_some() {
            return Err(DomainError::StatementDuplicateYear);
        }
        let statement = Statement {
            job_id,
            year,
            amounts,
        };
        self.statements.push(statement.clone());
        Ok(statement)
    }

    /// Changes the amounts of an existing annual statement. The year is fixed
    /// (REQ-8).
    ///
    /// # Errors
    /// Returns `JobNotFound` or `StatementNotFound`.
    pub fn update_statement(
        &mut self,
        job_id: JobId,
        year: Year,
        amounts: Amounts,
    ) -> Result<Statement, DomainError> {
        self.job(job_id)?;
        let statement = self
            .statements
            .iter_mut()
            .find(|s| s.job_id == job_id && s.year == year)
            .ok_or(DomainError::StatementNotFound)?;
        statement.amounts = amounts;
        Ok(statement.clone())
    }

    /// Removes an annual statement without further confirmation (REQ-8).
    ///
    /// # Errors
    /// Returns `JobNotFound` or `StatementNotFound`.
    pub fn delete_statement(&mut self, job_id: JobId, year: Year) -> Result<(), DomainError> {
        self.job(job_id)?;
        let before = self.statements.len();
        self.statements
            .retain(|s| !(s.job_id == job_id && s.year == year));
        if self.statements.len() == before {
            return Err(DomainError::StatementNotFound);
        }
        Ok(())
    }

    /// The annual statements of a job, newest year first. Used by the tests; the
    /// views list entries of all jobs (`all_entries`).
    ///
    /// # Errors
    /// Returns `JobNotFound`.
    #[cfg(test)]
    pub fn statements(&self, job_id: JobId) -> Result<Vec<&Statement>, DomainError> {
        self.job(job_id)?;
        let mut statements: Vec<&Statement> = self
            .statements
            .iter()
            .filter(|s| s.job_id == job_id)
            .collect();
        statements.sort_by_key(|statement| std::cmp::Reverse(statement.year));
        Ok(statements)
    }

    /// The paychecks of a job, newest month first. Used by the tests; the views
    /// list entries of all jobs (`all_entries`).
    ///
    /// # Errors
    /// Returns `JobNotFound`.
    #[cfg(test)]
    pub fn paychecks(&self, job_id: JobId) -> Result<Vec<&Paycheck>, DomainError> {
        self.job(job_id)?;
        let mut paychecks: Vec<&Paycheck> = self
            .paychecks
            .iter()
            .filter(|p| p.job_id == job_id)
            .collect();
        paychecks.sort_by_key(|paycheck| std::cmp::Reverse(paycheck.month));
        Ok(paychecks)
    }

    /// Values to pre-fill the add form: the gross salary of the latest entry of
    /// the job (paycheck or annual statement) and zero holiday pay and bonus
    /// (REQ-9).
    ///
    /// # Errors
    /// Returns `JobNotFound`.
    pub fn prefill_from_latest(&self, job_id: JobId) -> Result<PaycheckPrefill, DomainError> {
        self.job(job_id)?;
        let latest = self
            .entries()
            .filter(|entry| entry.job_id() == job_id)
            .max_by_key(Entry::sort_key);
        Ok(PaycheckPrefill {
            gross_salary: latest.map(|entry| entry.amounts().gross_salary()),
            holiday_pay: zero(),
            bonus: zero(),
        })
    }

    /// Whether any job exists.
    #[must_use]
    pub fn has_jobs(&self) -> bool {
        !self.jobs.is_empty()
    }

    /// Income this year up to and including `today`'s month, and last month,
    /// summed over all jobs (CSD-004 REQ-2 to REQ-4). An annual statement of this
    /// year counts in full in the first block and never in the second. Months
    /// after `today`, which only a project file can contain, are not counted.
    #[must_use]
    pub fn overview(&self, today: YearMonth) -> IncomeOverview {
        let last_month = today.previous();
        let year_to_date = totals_of(
            self.paychecks
                .iter()
                .filter(|p| p.month.year() == today.year() && p.month <= today)
                .map(|p| &p.amounts)
                .chain(
                    self.statements
                        .iter()
                        .filter(|s| s.year.value() == today.year())
                        .map(|s| &s.amounts),
                ),
        );
        let last_month_totals = last_month.and_then(|last| {
            totals_of(
                self.paychecks
                    .iter()
                    .filter(|p| p.month == last)
                    .map(|p| &p.amounts),
            )
        });
        IncomeOverview {
            year: today.year(),
            year_to_date,
            last_month,
            last_month_totals,
        }
    }

    /// All entries, paychecks and annual statements.
    fn entries(&self) -> impl Iterator<Item = Entry<'_>> {
        self.paychecks
            .iter()
            .map(Entry::Paycheck)
            .chain(self.statements.iter().map(Entry::Statement))
    }

    /// The entries of all jobs with their job, newest period first (an annual
    /// statement counts as December of its year) and, within a period, in job
    /// order (REQ-12).
    #[must_use]
    pub fn all_entries(&self) -> Vec<(&Job, Entry<'_>)> {
        let mut rows: Vec<(usize, &Job, Entry<'_>)> = Vec::new();
        for (position, job) in self.jobs().into_iter().enumerate() {
            rows.extend(
                self.entries()
                    .filter(|entry| entry.job_id() == job.id)
                    .map(|entry| (position, job, entry)),
            );
        }
        rows.sort_by(|a, b| b.2.sort_key().cmp(&a.2.sort_key()).then(a.0.cmp(&b.0)));
        rows.into_iter()
            .map(|(_, job, entry)| (job, entry))
            .collect()
    }

    /// Sums per calendar year over all jobs, newest year first, without
    /// extrapolating partial years (REQ-11). An annual statement counts in its
    /// own year only.
    #[must_use]
    pub fn yearly_totals(&self) -> Vec<YearlyTotal> {
        let mut by_year: BTreeMap<i32, YearlyTotal> = BTreeMap::new();
        for entry in self.entries() {
            let year = entry.year();
            let amounts = entry.amounts();
            let total = by_year.entry(year).or_insert_with(|| YearlyTotal {
                year,
                gross_salary: zero(),
                holiday_pay: zero(),
                bonus: zero(),
                total_gross: zero(),
                entry_count: 0,
            });
            total.gross_salary += amounts.gross_salary();
            total.holiday_pay += amounts.holiday_pay();
            total.bonus += amounts.bonus();
            total.total_gross += amounts.total_gross();
            total.entry_count += 1;
        }
        by_year.into_values().rev().collect()
    }

    /// One figure set per calendar year from the first entry up to and
    /// including the current year, oldest first (CSD-006). Years without
    /// entries are zero. The current year is partial and, like a year without
    /// entries or after one, has no growth. Entries after the current year,
    /// which only a project file can contain, are not shown.
    #[must_use]
    pub fn insights(&self, today: YearMonth) -> Vec<InsightsYear> {
        let totals: BTreeMap<i32, YearlyTotal> = self
            .yearly_totals()
            .into_iter()
            .map(|total| (total.year, total))
            .collect();
        let Some(&first) = totals.keys().next() else {
            return Vec::new();
        };
        let mut series: Vec<InsightsYear> = Vec::new();
        for year in first..=today.year() {
            let (gross_salary, holiday_pay, bonus, total_gross, entry_count) =
                totals.get(&year).map_or_else(
                    || (zero(), zero(), zero(), zero(), 0),
                    |t| {
                        (
                            t.gross_salary,
                            t.holiday_pay,
                            t.bonus,
                            t.total_gross,
                            t.entry_count,
                        )
                    },
                );
            let is_partial = year == today.year();
            let growth_percent = match series.last() {
                Some(previous) if !is_partial && entry_count > 0 => {
                    growth(previous.total_gross, total_gross)
                }
                _ => None,
            };
            series.push(InsightsYear {
                year,
                gross_salary,
                holiday_pay,
                bonus,
                total_gross,
                entry_count,
                is_partial,
                growth_percent,
            });
        }
        series
    }

    /// Checks the invariants of data that came from outside, such as a file.
    ///
    /// # Errors
    /// Returns `DataInvalid` for duplicate jobs, months or years, entries of
    /// unknown jobs or of a job with the other pay period, blank employers, an
    /// end date before a start date, or amounts that break the amount rules.
    pub fn validate(&self) -> Result<(), DomainError> {
        let mut jobs = std::collections::HashMap::new();
        for job in &self.jobs {
            let employer_ok = !job.employer.trim().is_empty();
            let dates_ok = job.end_date.is_none_or(|end| end >= job.start_date);
            if !employer_ok || !dates_ok || jobs.insert(job.id, job.pay_period).is_some() {
                return Err(DomainError::DataInvalid);
            }
        }
        let amounts_ok = |amounts: &Amounts| {
            Amounts::new(
                amounts.gross_salary(),
                amounts.holiday_pay(),
                amounts.bonus(),
            )
            .is_ok()
        };
        let mut seen_months = std::collections::HashSet::new();
        for paycheck in &self.paychecks {
            // A paycheck belongs to a monthly job (REQ-27).
            if jobs.get(&paycheck.job_id) != Some(&PayPeriod::Monthly)
                || !seen_months.insert((paycheck.job_id, paycheck.month))
                || !amounts_ok(&paycheck.amounts)
            {
                return Err(DomainError::DataInvalid);
            }
        }
        let mut seen_years = std::collections::HashSet::new();
        for statement in &self.statements {
            // An annual statement belongs to a yearly job (REQ-27).
            if jobs.get(&statement.job_id) != Some(&PayPeriod::Yearly)
                || !seen_years.insert((statement.job_id, statement.year))
                || !amounts_ok(&statement.amounts)
            {
                return Err(DomainError::DataInvalid);
            }
        }
        Ok(())
    }

    /// A job by id.
    ///
    /// # Errors
    /// Returns `JobNotFound`.
    pub fn job(&self, id: JobId) -> Result<&Job, DomainError> {
        self.jobs
            .iter()
            .find(|job| job.id == id)
            .ok_or(DomainError::JobNotFound)
    }

    fn job_mut(&mut self, id: JobId) -> Result<&mut Job, DomainError> {
        self.jobs
            .iter_mut()
            .find(|job| job.id == id)
            .ok_or(DomainError::JobNotFound)
    }

    fn paycheck(&self, job_id: JobId, month: YearMonth) -> Option<&Paycheck> {
        self.paychecks
            .iter()
            .find(|p| p.job_id == job_id && p.month == month)
    }

    fn statement(&self, job_id: JobId, year: Year) -> Option<&Statement> {
        self.statements
            .iter()
            .find(|s| s.job_id == job_id && s.year == year)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn month(text: &str) -> YearMonth {
        YearMonth::parse(text).unwrap()
    }

    fn today() -> YearMonth {
        month("2026-10")
    }

    fn input(employer: &str, start: &str, end: &str) -> JobInput {
        JobInput {
            employer: employer.to_owned(),
            title: None,
            start_date: Some(start.to_owned()),
            end_date: Some(end.to_owned()),
            pay_period: None,
        }
    }

    fn job_with(income: &mut Income, start: &str, end: &str) -> JobId {
        income
            .create_job(&input("Example Employer", start, end))
            .unwrap()
            .id
    }

    fn amounts(gross: &str, holiday: &str, bonus: &str) -> Amounts {
        Amounts::parse(gross, holiday, bonus).unwrap()
    }

    fn add(income: &mut Income, job: JobId, m: &str, gross: &str) -> Paycheck {
        income
            .add_paycheck(job, month(m), amounts(gross, "", ""), today())
            .unwrap()
    }

    // ---- REQ-1: add a job ----

    #[test]
    fn test_csd001_ac_1_1_job_created_with_defaults() {
        let mut income = Income::default();
        let job = income
            .create_job(&JobInput {
                employer: "Example Employer".into(),
                start_date: Some("01/03/2024".into()),
                ..JobInput::default()
            })
            .unwrap();
        assert_eq!(income.jobs().len(), 1);
        assert_eq!(job.employer, "Example Employer");
        assert_eq!(job.start_date.to_string(), "2024-03-01");
        assert_eq!(job.title, None);
        assert_eq!(job.end_date, None);
    }

    #[test]
    fn test_csd001_ac_1_2_invalid_job_is_rejected() {
        let cases = [
            ("", "01/03/2024", "", DomainError::JobEmployerRequired),
            ("   ", "01/03/2024", "", DomainError::JobEmployerRequired),
            ("Example Employer", "", "", DomainError::JobStartRequired),
            (
                "Example Employer",
                "01/03/2024",
                "29/02/2024",
                DomainError::JobEndBeforeStart,
            ),
            (
                "Example Employer",
                "01-03-2024",
                "",
                DomainError::JobDateInvalid,
            ),
        ];
        for (employer, start, end, expected) in cases {
            let mut income = Income::default();
            let error = income.create_job(&input(employer, start, end)).unwrap_err();
            assert_eq!(error, expected, "{employer:?} {start:?} {end:?}");
            assert_eq!(income.jobs().len(), 0);
        }
    }

    #[test]
    fn test_csd001_ac_1_2_error_codes_match_the_spec() {
        assert_eq!(
            DomainError::JobEmployerRequired.code(),
            "JOB_EMPLOYER_REQUIRED"
        );
        assert_eq!(DomainError::JobStartRequired.code(), "JOB_START_REQUIRED");
        assert_eq!(
            DomainError::JobEndBeforeStart.code(),
            "JOB_END_BEFORE_START"
        );
    }

    #[test]
    fn overlong_text_is_rejected() {
        let mut income = Income::default();
        let long = "x".repeat(201);
        let error = income
            .create_job(&input(&long, "01/03/2024", ""))
            .unwrap_err();
        assert_eq!(error, DomainError::JobFieldTooLong);
    }

    // ---- REQ-2: edit a job ----

    #[test]
    fn test_csd001_ac_2_1_dates_change_freely_with_paychecks_present() {
        let mut income = Income::default();
        let id = job_with(&mut income, "01/03/2024", "");
        add(&mut income, id, "2024-03", "3000.00");
        let updated = income
            .update_job(id, &input("Example Employer", "01/06/2024", "31/01/2025"))
            .unwrap();
        assert_eq!(updated.start_date.to_string(), "2024-06-01");
        assert_eq!(
            updated.end_date.map(|d| d.to_string()).as_deref(),
            Some("2025-01-31")
        );
        assert_eq!(income.entry_count(id), 1);
    }

    #[test]
    fn test_csd001_ac_2_2_end_before_start_is_rejected_on_edit() {
        let mut income = Income::default();
        let id = job_with(&mut income, "01/03/2024", "");
        let error = income
            .update_job(id, &input("Example Employer", "01/03/2024", "01/02/2024"))
            .unwrap_err();
        assert_eq!(error.code(), "JOB_END_BEFORE_START");
        assert_eq!(income.jobs()[0].end_date, None);
    }

    #[test]
    fn editing_an_unknown_job_is_rejected() {
        let mut income = Income::default();
        let error = income
            .update_job(
                JobId::generate(),
                &input("Example Employer", "01/03/2024", ""),
            )
            .unwrap_err();
        assert_eq!(error, DomainError::JobNotFound);
    }

    // ---- REQ-4: delete a job ----

    #[test]
    fn test_csd001_ac_4_1_deletion_cascades() {
        let mut income = Income::default();
        let id = job_with(&mut income, "01/03/2024", "");
        let other = job_with(&mut income, "01/01/2020", "");
        for m in ["2026-07", "2026-08", "2026-09"] {
            add(&mut income, id, m, "3000.00");
        }
        add(&mut income, other, "2026-09", "1000.00");
        income.delete_job(id).unwrap();
        assert_eq!(income.jobs().len(), 1);
        assert_eq!(income.entry_count(id), 0);
        assert_eq!(income.entry_count(other), 1);
        assert_eq!(income.paychecks(id).unwrap_err(), DomainError::JobNotFound);
    }

    // AC-4.2 (cancelled deletion) is a UI confirmation: the domain only deletes
    // when asked to, covered by the manual checklist.

    // ---- REQ-5: add a paycheck ----

    #[test]
    fn test_csd001_ac_5_1_paycheck_with_defaults() {
        let mut income = Income::default();
        let id = job_with(&mut income, "01/03/2024", "");
        let paycheck = add(&mut income, id, "2026-09", "3000.00");
        assert_eq!(paycheck.amounts.gross_salary().to_string(), "3000.00");
        assert_eq!(paycheck.amounts.holiday_pay().to_string(), "0.00");
        assert_eq!(paycheck.amounts.bonus().to_string(), "0.00");
        assert_eq!(paycheck.amounts.total_gross().to_string(), "3000.00");
    }

    #[test]
    fn test_csd001_ac_5_2_month_before_job_start_is_accepted() {
        let mut income = Income::default();
        let id = job_with(&mut income, "01/03/2024", "");
        add(&mut income, id, "2024-01", "2800.00");
        assert_eq!(income.entry_count(id), 1);
    }

    #[test]
    fn test_csd001_ac_5_3_month_after_job_end_is_accepted() {
        let mut income = Income::default();
        let id = job_with(&mut income, "01/03/2024", "31/01/2025");
        add(&mut income, id, "2025-03", "1500.00");
        assert_eq!(income.entry_count(id), 1);
    }

    #[test]
    fn test_csd001_ac_5_4_month_later_than_current_is_rejected() {
        let mut income = Income::default();
        let id = job_with(&mut income, "01/03/2024", "");
        let error = income
            .add_paycheck(id, month("2026-11"), amounts("3000.00", "", ""), today())
            .unwrap_err();
        assert_eq!(error.code(), "PAYCHECK_MONTH_IN_FUTURE");
        assert_eq!(income.entry_count(id), 0);
    }

    #[test]
    fn test_csd001_ac_5_5_current_month_is_accepted() {
        let mut income = Income::default();
        let id = job_with(&mut income, "01/03/2024", "");
        add(&mut income, id, "2026-10", "3000.00");
        assert_eq!(income.entry_count(id), 1);
    }

    #[test]
    fn adding_a_paycheck_to_an_unknown_job_is_rejected() {
        let mut income = Income::default();
        let error = income
            .add_paycheck(
                JobId::generate(),
                month("2026-09"),
                amounts("1.00", "", ""),
                today(),
            )
            .unwrap_err();
        assert_eq!(error, DomainError::JobNotFound);
    }

    // ---- REQ-6: one paycheck per job per month ----

    #[test]
    fn test_csd001_ac_6_1_duplicate_month_rejected() {
        let mut income = Income::default();
        let id = job_with(&mut income, "01/03/2024", "");
        add(&mut income, id, "2026-09", "3000.00");
        let error = income
            .add_paycheck(id, month("2026-09"), amounts("3100.00", "", ""), today())
            .unwrap_err();
        assert_eq!(error.code(), "PAYCHECK_DUPLICATE_MONTH");
        let paychecks = income.paychecks(id).unwrap();
        assert_eq!(paychecks.len(), 1);
        assert_eq!(paychecks[0].amounts.gross_salary().to_string(), "3000.00");
    }

    #[test]
    fn test_csd001_ac_6_2_same_month_on_another_job_is_allowed() {
        let mut income = Income::default();
        let first = job_with(&mut income, "01/03/2024", "");
        let second = job_with(&mut income, "01/01/2025", "");
        add(&mut income, first, "2026-09", "3000.00");
        add(&mut income, second, "2026-09", "1000.00");
        assert_eq!(income.entry_count(first), 1);
        assert_eq!(income.entry_count(second), 1);
    }

    // ---- REQ-7: amount validation ----

    #[test]
    fn test_csd001_ac_7_1_invalid_amounts_rejected() {
        let cases = [
            ("0.00", "0.00", "0.00", "PAYCHECK_GROSS_INVALID"),
            ("-100.00", "0.00", "0.00", "PAYCHECK_AMOUNT_NEGATIVE"),
            ("3000.00", "-1.00", "0.00", "PAYCHECK_AMOUNT_NEGATIVE"),
            ("3000.00", "0.00", "-5.00", "PAYCHECK_AMOUNT_NEGATIVE"),
            ("3000.999", "0.00", "0.00", "PAYCHECK_AMOUNT_PRECISION"),
            ("1000000.01", "0.00", "0.00", "PAYCHECK_AMOUNT_TOO_LARGE"),
        ];
        for (gross, holiday, bonus, code) in cases {
            let mut income = Income::default();
            let id = job_with(&mut income, "01/03/2024", "");
            let error = Amounts::parse(gross, holiday, bonus).unwrap_err();
            assert_eq!(error.code(), code, "{gross} {holiday} {bonus}");
            assert_eq!(income.paychecks(id).unwrap().len(), 0);
        }
    }

    #[test]
    fn test_csd001_ac_7_2_boundary_amount_accepted() {
        let mut income = Income::default();
        let id = job_with(&mut income, "01/03/2024", "");
        add(&mut income, id, "2026-09", "1000000.00");
        assert_eq!(income.entry_count(id), 1);
    }

    // ---- REQ-8: edit and delete a paycheck ----

    #[test]
    fn test_csd001_ac_8_1_edit_amounts() {
        let mut income = Income::default();
        let id = job_with(&mut income, "01/03/2024", "");
        add(&mut income, id, "2026-05", "3000.00");
        let updated = income
            .update_paycheck(id, month("2026-05"), amounts("3000.00", "2400.00", "0.00"))
            .unwrap();
        assert_eq!(updated.amounts.holiday_pay().to_string(), "2400.00");
        assert_eq!(updated.amounts.total_gross().to_string(), "5400.00");
    }

    #[test]
    fn test_csd001_ac_8_2_invalid_edit_rejected() {
        let mut income = Income::default();
        let id = job_with(&mut income, "01/03/2024", "");
        add(&mut income, id, "2026-05", "3000.00");
        let error = Amounts::parse("-1.00", "", "").unwrap_err();
        assert_eq!(error.code(), "PAYCHECK_AMOUNT_NEGATIVE");
        let kept = income.paychecks(id).unwrap();
        assert_eq!(kept[0].amounts.gross_salary().to_string(), "3000.00");
    }

    #[test]
    fn test_csd001_ac_8_3_delete_paycheck() {
        let mut income = Income::default();
        let id = job_with(&mut income, "01/03/2024", "");
        add(&mut income, id, "2026-04", "3000.00");
        add(&mut income, id, "2026-05", "3000.00");
        income.delete_paycheck(id, month("2026-05")).unwrap();
        let months: Vec<String> = income
            .paychecks(id)
            .unwrap()
            .iter()
            .map(|p| p.month.to_string())
            .collect();
        assert_eq!(months, ["2026-04"]);
    }

    #[test]
    fn test_csd001_ac_8_4_month_stays_fixed_on_edit() {
        let mut income = Income::default();
        let id = job_with(&mut income, "01/03/2024", "");
        add(&mut income, id, "2026-05", "3000.00");
        let updated = income
            .update_paycheck(id, month("2026-05"), amounts("3100.00", "", ""))
            .unwrap();
        assert_eq!(updated.month.to_string(), "2026-05");
        // The API has no way to move a paycheck: only the amounts are editable.
        assert_eq!(income.paychecks(id).unwrap().len(), 1);
    }

    #[test]
    fn editing_or_deleting_a_missing_paycheck_is_rejected() {
        let mut income = Income::default();
        let id = job_with(&mut income, "01/03/2024", "");
        assert_eq!(
            income
                .update_paycheck(id, month("2026-05"), amounts("1.00", "", ""))
                .unwrap_err(),
            DomainError::PaycheckNotFound
        );
        assert_eq!(
            income.delete_paycheck(id, month("2026-05")).unwrap_err(),
            DomainError::PaycheckNotFound
        );
    }

    // ---- REQ-9: copy previous month ----

    #[test]
    fn test_csd001_ac_9_1_form_prefilled_from_latest_paycheck() {
        let mut income = Income::default();
        let id = job_with(&mut income, "01/03/2024", "");
        add(&mut income, id, "2026-07", "3000.00");
        income
            .add_paycheck(
                id,
                month("2026-08"),
                amounts("3100.00", "", "500.00"),
                today(),
            )
            .unwrap();
        let prefill = income.prefill_from_latest(id).unwrap();
        assert_eq!(prefill.gross_salary.unwrap().to_string(), "3100.00");
        assert_eq!(prefill.holiday_pay.to_string(), "0.00");
        assert_eq!(prefill.bonus.to_string(), "0.00");
        assert_eq!(income.entry_count(id), 2);
    }

    #[test]
    fn test_csd001_ac_9_2_no_paycheck_to_copy() {
        let mut income = Income::default();
        let id = job_with(&mut income, "01/03/2024", "");
        let prefill = income.prefill_from_latest(id).unwrap();
        assert_eq!(prefill.gross_salary, None);
        assert_eq!(prefill.holiday_pay.to_string(), "0.00");
        assert_eq!(prefill.bonus.to_string(), "0.00");
    }

    // ---- REQ-10: total gross ----

    #[test]
    fn test_csd001_ac_10_1_total_is_the_sum() {
        let amounts = amounts("3000.00", "240.00", "500.00");
        assert_eq!(amounts.total_gross().to_string(), "3740.00");
    }

    // ---- REQ-11: yearly totals ----

    #[test]
    fn test_csd001_ac_11_1_totals_per_calendar_year() {
        let mut income = Income::default();
        let id = job_with(&mut income, "01/03/2024", "");
        add(&mut income, id, "2025-11", "3000.00");
        income
            .add_paycheck(
                id,
                month("2025-12"),
                amounts("3000.00", "", "1000.00"),
                today(),
            )
            .unwrap();
        income
            .add_paycheck(
                id,
                month("2026-05"),
                amounts("3000.00", "2400.00", ""),
                today(),
            )
            .unwrap();
        let totals = income.yearly_totals();
        assert_eq!(totals.len(), 2);
        let (y2026, y2025) = (&totals[0], &totals[1]);
        assert_eq!(y2026.year, 2026);
        assert_eq!(y2026.gross_salary.to_string(), "3000.00");
        assert_eq!(y2026.holiday_pay.to_string(), "2400.00");
        assert_eq!(y2026.bonus.to_string(), "0.00");
        assert_eq!(y2026.total_gross.to_string(), "5400.00");
        assert_eq!(y2026.entry_count, 1);
        assert_eq!(y2025.year, 2025);
        assert_eq!(y2025.gross_salary.to_string(), "6000.00");
        assert_eq!(y2025.holiday_pay.to_string(), "0.00");
        assert_eq!(y2025.bonus.to_string(), "1000.00");
        assert_eq!(y2025.total_gross.to_string(), "7000.00");
        assert_eq!(y2025.entry_count, 2);
    }

    #[test]
    fn test_csd001_ac_11_2_no_paychecks_gives_no_totals() {
        let mut income = Income::default();
        job_with(&mut income, "01/03/2024", "");
        assert_eq!(income.yearly_totals().len(), 0);
    }

    #[test]
    fn test_csd001_ac_11_3_totals_add_up_all_jobs() {
        let mut income = Income::default();
        let first = job_with(&mut income, "01/03/2024", "");
        let second = job_with(&mut income, "01/01/2020", "");
        add(&mut income, first, "2026-08", "3000.00");
        income
            .add_paycheck(
                first,
                month("2026-09"),
                amounts("3000.00", "", "500.00"),
                today(),
            )
            .unwrap();
        add(&mut income, second, "2026-09", "1000.00");

        let totals = income.yearly_totals();
        assert_eq!(totals.len(), 1);
        let year = &totals[0];
        assert_eq!(year.year, 2026);
        assert_eq!(year.gross_salary.to_string(), "7000.00");
        assert_eq!(year.holiday_pay.to_string(), "0.00");
        assert_eq!(year.bonus.to_string(), "500.00");
        assert_eq!(year.total_gross.to_string(), "7500.00");
        assert_eq!(year.entry_count, 3);
    }

    // ---- REQ-12: ordering ----

    #[test]
    fn test_csd001_ac_12_1_newest_first() {
        let mut income = Income::default();
        let old = job_with(&mut income, "01/01/2020", "");
        let recent = job_with(&mut income, "01/03/2024", "");
        add(&mut income, recent, "2026-07", "3000.00");
        add(&mut income, recent, "2026-09", "3000.00");
        let jobs: Vec<JobId> = income.jobs().iter().map(|job| job.id).collect();
        assert_eq!(jobs, [recent, old]);
        let months: Vec<String> = income
            .paychecks(recent)
            .unwrap()
            .iter()
            .map(|p| p.month.to_string())
            .collect();
        assert_eq!(months, ["2026-09", "2026-07"]);
    }

    #[test]
    fn test_csd001_ac_12_2_same_month_is_ordered_by_job() {
        let mut income = Income::default();
        let old = job_with(&mut income, "01/01/2020", "");
        let recent = job_with(&mut income, "01/03/2024", "");
        // Added in the opposite order of the expected result.
        add(&mut income, old, "2026-09", "1000.00");
        add(&mut income, recent, "2026-09", "3000.00");
        add(&mut income, old, "2026-10", "1000.00");

        let order: Vec<(String, JobId)> = income
            .all_entries()
            .into_iter()
            .map(|(job, entry)| (entry.period(), job.id))
            .collect();
        assert_eq!(
            order,
            [
                ("2026-10".to_owned(), old),
                ("2026-09".to_owned(), recent),
                ("2026-09".to_owned(), old),
            ]
        );
    }

    #[test]
    fn test_csd001_ac_12_3_yearly_totals_newest_year_first() {
        let mut income = Income::default();
        let id = job_with(&mut income, "01/03/2024", "");
        add(&mut income, id, "2024-05", "1000.00");
        add(&mut income, id, "2026-05", "1000.00");
        add(&mut income, id, "2025-05", "1000.00");
        let years: Vec<i32> = income.yearly_totals().iter().map(|t| t.year).collect();
        assert_eq!(years, [2026, 2025, 2024]);
    }

    // ---- REQ-13: empty states ----

    #[test]
    fn test_csd001_ac_13_1_no_jobs() {
        assert_eq!(Income::default().jobs().len(), 0);
    }

    #[test]
    fn test_csd001_ac_13_2_job_without_paychecks_is_not_an_error() {
        let mut income = Income::default();
        let id = job_with(&mut income, "01/03/2024", "");
        assert_eq!(income.paychecks(id).unwrap().len(), 0);
        assert_eq!(income.all_entries().len(), 0);
        assert_eq!(income.yearly_totals().len(), 0);
    }

    // ---- REQ-23: employment view (data behind the paychecks table) ----

    #[test]
    fn test_csd001_ac_23_3_paychecks_of_all_jobs_come_with_their_job() {
        let mut income = Income::default();
        let recent = job_with(&mut income, "01/03/2024", "");
        let older = income
            .create_job(&input("Older Example Employer", "01/01/2020", ""))
            .unwrap()
            .id;
        add(&mut income, recent, "2026-09", "3000.00");
        add(&mut income, older, "2026-08", "1000.00");

        let rows: Vec<(&str, String)> = income
            .all_entries()
            .into_iter()
            .map(|(job, entry)| (job.employer.as_str(), entry.period()))
            .collect();
        assert_eq!(
            rows,
            [
                ("Example Employer", "2026-09".to_owned()),
                ("Older Example Employer", "2026-08".to_owned()),
            ]
        );
    }

    // ---- CSD-004: income dashboard ----

    /// The paychecks of CSD-004 AC-2.1: two jobs, today 25/10/2026.
    fn dashboard_income() -> Income {
        let mut income = Income::default();
        let recent = job_with(&mut income, "01/03/2024", "");
        let older = income
            .create_job(&input("Older Example Employer", "01/01/2020", ""))
            .unwrap()
            .id;
        add(&mut income, recent, "2026-08", "3000.00");
        income
            .add_paycheck(
                recent,
                month("2026-09"),
                amounts("3000.00", "", "500.00"),
                today(),
            )
            .unwrap();
        add(&mut income, recent, "2026-10", "3000.00");
        add(&mut income, older, "2026-09", "1000.00");
        income
    }

    #[test]
    fn test_csd004_ac_2_1_year_to_date_over_all_jobs() {
        let overview = dashboard_income().overview(today());
        assert_eq!(overview.year, 2026);
        let totals = overview.year_to_date.unwrap();
        assert_eq!(totals.gross_salary.to_string(), "10000.00");
        assert_eq!(totals.holiday_pay.to_string(), "0.00");
        assert_eq!(totals.bonus.to_string(), "500.00");
        assert_eq!(totals.total_gross.to_string(), "10500.00");
        assert_eq!(totals.entry_count, 4);
    }

    #[test]
    fn test_csd004_ac_2_2_other_years_are_not_counted() {
        let mut income = Income::default();
        let id = job_with(&mut income, "01/03/2024", "");
        add(&mut income, id, "2025-12", "3000.00");
        add(&mut income, id, "2026-09", "1000.00");
        let totals = income.overview(today()).year_to_date.unwrap();
        assert_eq!(totals.gross_salary.to_string(), "1000.00");
        assert_eq!(totals.entry_count, 1);
    }

    #[test]
    fn test_csd004_ac_2_3_months_after_the_current_month_are_not_counted() {
        let mut income = Income::default();
        let id = job_with(&mut income, "01/03/2024", "");
        add(&mut income, id, "2026-10", "3000.00");
        // Only a project file can hold a later month, so allow it here.
        income
            .add_paycheck(
                id,
                month("2026-12"),
                amounts("3000.00", "", ""),
                month("2026-12"),
            )
            .unwrap();
        let totals = income.overview(today()).year_to_date.unwrap();
        assert_eq!(totals.gross_salary.to_string(), "3000.00");
        assert_eq!(totals.entry_count, 1);
    }

    #[test]
    fn test_csd004_ac_3_1_last_month_is_the_previous_calendar_month() {
        let overview = dashboard_income().overview(today());
        assert_eq!(overview.last_month.unwrap().to_string(), "2026-09");
        let totals = overview.last_month_totals.unwrap();
        assert_eq!(totals.gross_salary.to_string(), "4000.00");
        assert_eq!(totals.holiday_pay.to_string(), "0.00");
        assert_eq!(totals.bonus.to_string(), "500.00");
        assert_eq!(totals.total_gross.to_string(), "4500.00");
        assert_eq!(totals.entry_count, 2);
    }

    #[test]
    fn test_csd004_ac_3_2_january_looks_back_to_december() {
        let mut income = Income::default();
        let id = job_with(&mut income, "01/03/2024", "");
        income
            .add_paycheck(
                id,
                month("2026-12"),
                amounts("3000.00", "", ""),
                month("2027-01"),
            )
            .unwrap();
        let overview = income.overview(month("2027-01"));
        assert_eq!(overview.last_month.unwrap().to_string(), "2026-12");
        let totals = overview.last_month_totals.unwrap();
        assert_eq!(totals.total_gross.to_string(), "3000.00");
        assert_eq!(totals.entry_count, 1);
    }

    #[test]
    fn test_csd004_ac_3_3_the_current_month_is_not_last_month() {
        let mut income = Income::default();
        let id = job_with(&mut income, "01/03/2024", "");
        add(&mut income, id, "2026-10", "3000.00");
        let overview = income.overview(today());
        assert_eq!(overview.last_month.unwrap().to_string(), "2026-09");
        assert_eq!(overview.last_month_totals, None);
    }

    #[test]
    fn test_csd004_ac_4_1_nothing_earned_this_year_yet_has_no_totals() {
        let mut income = Income::default();
        let id = job_with(&mut income, "01/03/2024", "");
        income
            .add_paycheck(
                id,
                month("2026-12"),
                amounts("3000.00", "", ""),
                month("2027-01"),
            )
            .unwrap();
        let overview = income.overview(month("2027-01"));
        assert_eq!(overview.year, 2027);
        assert_eq!(overview.year_to_date, None);
    }

    #[test]
    fn test_csd004_ac_4_2_nothing_earned_last_month_has_no_totals() {
        let mut income = Income::default();
        let id = job_with(&mut income, "01/03/2024", "");
        add(&mut income, id, "2026-10", "3000.00");
        let overview = income.overview(today());
        assert_eq!(overview.last_month_totals, None);
        assert!(overview.year_to_date.is_some());
    }

    #[test]
    fn test_csd004_ac_5_1_has_jobs_tells_an_empty_project_apart() {
        let mut income = Income::default();
        assert!(!income.has_jobs());
        job_with(&mut income, "01/03/2024", "");
        assert!(income.has_jobs());
        // A job without paychecks has no totals, and no zero amounts either.
        let overview = income.overview(today());
        assert_eq!(overview.year_to_date, None);
        assert_eq!(overview.last_month_totals, None);
    }

    #[test]
    fn year_month_previous_wraps_and_stops_at_the_first_year() {
        assert_eq!(month("2026-10").previous().unwrap().to_string(), "2026-09");
        assert_eq!(month("2027-01").previous().unwrap().to_string(), "2026-12");
        assert_eq!(month("1900-01").previous(), None);
    }

    // ---- REQ-22: date format ----

    #[test]
    fn test_csd001_ac_22_1_typed_dates_become_calendar_dates() {
        let mut income = Income::default();
        let job = income
            .create_job(&input("Example Employer", "01/03/2024", "31/01/2025"))
            .unwrap();
        // The domain stores real dates; commands return them as ISO 8601 and the
        // frontend shows them as dd/mm/yyyy (manual checklist for the display).
        assert_eq!(job.start_date.to_string(), "2024-03-01");
        assert_eq!(job.end_date.unwrap().to_string(), "2025-01-31");
        let json = serde_json::to_value(&job).unwrap();
        assert_eq!(json["start_date"], "2024-03-01");
        assert_eq!(json["end_date"], "2025-01-31");
    }

    #[test]
    fn test_csd001_ac_22_2_other_date_formats_are_rejected() {
        let cases = [
            ("2024-03-01", "year first"),
            ("1/3/2024", "no two-digit day and month"),
            ("1/03/2024", "no two-digit day"),
            ("01/3/2024", "no two-digit month"),
            ("03-01-2024", "wrong separator"),
            ("31/02/2024", "not a calendar date"),
            ("29/02/2023", "not a leap year"),
            ("01/13/2024", "month 13"),
            ("00/03/2024", "day 0"),
            ("01/03/24", "two-digit year"),
            ("01/03/20244", "five-digit year"),
            ("01.03.2024", "dots"),
            ("1 March 2024", "words"),
            ("01/03/2024 10:00", "time attached"),
        ];
        for (start, reason) in cases {
            let mut income = Income::default();
            let error = income
                .create_job(&input("Example Employer", start, ""))
                .unwrap_err();
            assert_eq!(error.code(), "JOB_DATE_INVALID", "{start:?}: {reason}");
            assert_eq!(income.jobs().len(), 0, "{start:?}: {reason}");
        }
    }

    #[test]
    fn test_csd001_ac_22_2_end_date_uses_the_same_format() {
        let mut income = Income::default();
        let error = income
            .create_job(&input("Example Employer", "01/03/2024", "2025-01-31"))
            .unwrap_err();
        assert_eq!(error, DomainError::JobDateInvalid);
        let id = job_with(&mut income, "01/03/2024", "");
        let error = income
            .update_job(id, &input("Example Employer", "01/03/2024", "31-01-2025"))
            .unwrap_err();
        assert_eq!(error, DomainError::JobDateInvalid);
    }

    #[test]
    fn dates_are_accepted_with_surrounding_whitespace_and_on_leap_days() {
        let mut income = Income::default();
        let job = income
            .create_job(&input("Example Employer", " 29/02/2024 ", ""))
            .unwrap();
        assert_eq!(job.start_date.to_string(), "2024-02-29");
    }

    #[test]
    fn the_date_error_message_names_the_expected_format() {
        assert_eq!(
            DomainError::JobDateInvalid.to_string(),
            "Enter dates as dd/mm/yyyy."
        );
    }

    #[test]
    fn test_csd001_ac_22_3_months_stay_year_month() {
        let mut income = Income::default();
        let id = job_with(&mut income, "01/03/2024", "");
        let paycheck = add(&mut income, id, "2026-09", "3000.00");
        assert_eq!(paycheck.month.to_string(), "2026-09");
        let json = serde_json::to_value(&paycheck).unwrap();
        assert_eq!(json["month"], "2026-09");
        for bad in ["09/2026", "01/09/2026", "09-2026"] {
            assert_eq!(
                YearMonth::parse(bad).unwrap_err(),
                DomainError::PaycheckMonthInvalid,
                "{bad:?}"
            );
        }
    }

    // ---- annual statements (CSD-001 0.9) ----

    fn year(text: &str) -> Year {
        Year::parse(text).unwrap()
    }

    fn yearly_job(income: &mut Income) -> JobId {
        income
            .create_job(&JobInput {
                pay_period: Some("Yearly".to_owned()),
                ..input("Example Employer", "01/03/2024", "")
            })
            .unwrap()
            .id
    }

    fn statement(income: &mut Income, job: JobId, y: &str, gross: &str) -> Statement {
        income
            .add_statement(job, year(y), amounts(gross, "", ""), today())
            .unwrap()
    }

    #[test]
    fn test_csd001_ac_1_5_a_job_can_be_yearly() {
        let mut income = Income::default();
        let id = yearly_job(&mut income);
        assert_eq!(income.job(id).unwrap().pay_period, PayPeriod::Yearly);
    }

    #[test]
    fn test_csd001_ac_1_1_a_job_is_monthly_by_default() {
        let mut income = Income::default();
        let id = job_with(&mut income, "01/03/2024", "");
        assert_eq!(income.job(id).unwrap().pay_period, PayPeriod::Monthly);
        // Blank text means the default as well.
        let blank = income
            .create_job(&JobInput {
                pay_period: Some("  ".to_owned()),
                ..input("Example Employer", "01/03/2024", "")
            })
            .unwrap();
        assert_eq!(blank.pay_period, PayPeriod::Monthly);
    }

    #[test]
    fn test_csd001_ac_1_6_unsupported_pay_periods_are_rejected() {
        for text in ["Weekly", "monthly!", "Year", "12"] {
            let mut income = Income::default();
            let error = income
                .create_job(&JobInput {
                    pay_period: Some(text.to_owned()),
                    ..input("Example Employer", "01/03/2024", "")
                })
                .unwrap_err();
            assert_eq!(error.code(), "JOB_PERIOD_UNSUPPORTED", "{text:?}");
            assert_eq!(income.jobs().len(), 0);
        }
    }

    #[test]
    fn pay_periods_are_parsed_case_insensitively() {
        assert_eq!(PayPeriod::parse(" yearly "), Some(PayPeriod::Yearly));
        assert_eq!(PayPeriod::parse("MONTHLY"), Some(PayPeriod::Monthly));
        assert_eq!(PayPeriod::parse(""), None);
    }

    #[test]
    fn test_csd001_ac_24_1_the_pay_period_is_locked_once_entries_exist() {
        let mut income = Income::default();
        let monthly = job_with(&mut income, "01/03/2024", "");
        add(&mut income, monthly, "2026-09", "3000.00");
        let error = income
            .update_job(
                monthly,
                &JobInput {
                    pay_period: Some("Yearly".to_owned()),
                    ..input("Example Employer", "01/03/2024", "")
                },
            )
            .unwrap_err();
        assert_eq!(error.code(), "JOB_PERIOD_LOCKED");
        assert_eq!(income.job(monthly).unwrap().pay_period, PayPeriod::Monthly);

        let yearly = yearly_job(&mut income);
        statement(&mut income, yearly, "2025", "40000.00");
        let error = income
            .update_job(
                yearly,
                &JobInput {
                    pay_period: Some("Monthly".to_owned()),
                    ..input("Example Employer", "01/03/2024", "")
                },
            )
            .unwrap_err();
        assert_eq!(error, DomainError::JobPeriodLocked);
    }

    #[test]
    fn test_csd001_ac_24_2_the_pay_period_can_change_without_entries() {
        let mut income = Income::default();
        let id = job_with(&mut income, "01/03/2024", "");
        let updated = income
            .update_job(
                id,
                &JobInput {
                    pay_period: Some("Yearly".to_owned()),
                    ..input("Example Employer", "01/03/2024", "")
                },
            )
            .unwrap();
        assert_eq!(updated.pay_period, PayPeriod::Yearly);
    }

    #[test]
    fn test_csd001_ac_24_3_keeping_or_omitting_the_pay_period_is_allowed_with_entries() {
        let mut income = Income::default();
        let id = yearly_job(&mut income);
        statement(&mut income, id, "2025", "40000.00");
        // Same value, and no value at all ("unchanged"), both keep Yearly.
        for period in [Some("Yearly".to_owned()), None] {
            let updated = income
                .update_job(
                    id,
                    &JobInput {
                        pay_period: period,
                        ..input("Renamed Employer", "01/03/2024", "")
                    },
                )
                .unwrap();
            assert_eq!(updated.employer, "Renamed Employer");
            assert_eq!(updated.pay_period, PayPeriod::Yearly);
        }
    }

    #[test]
    fn test_csd001_ac_25_1_an_annual_statement_has_defaults() {
        let mut income = Income::default();
        let id = yearly_job(&mut income);
        let statement = statement(&mut income, id, "2025", "40000.00");
        assert_eq!(statement.amounts.gross_salary().to_string(), "40000.00");
        assert_eq!(statement.amounts.holiday_pay().to_string(), "0.00");
        assert_eq!(statement.amounts.bonus().to_string(), "0.00");
        assert_eq!(statement.amounts.total_gross().to_string(), "40000.00");
    }

    #[test]
    fn test_csd001_ac_25_2_a_year_before_the_job_start_is_accepted() {
        let mut income = Income::default();
        let id = yearly_job(&mut income);
        statement(&mut income, id, "2022", "38000.00");
        assert_eq!(income.entry_count(id), 1);
    }

    #[test]
    fn test_csd001_ac_25_3_a_year_later_than_the_current_year_is_rejected() {
        let mut income = Income::default();
        let id = yearly_job(&mut income);
        let error = income
            .add_statement(id, year("2027"), amounts("40000.00", "", ""), today())
            .unwrap_err();
        assert_eq!(error.code(), "STATEMENT_YEAR_IN_FUTURE");
        assert_eq!(income.entry_count(id), 0);
    }

    #[test]
    fn test_csd001_ac_25_4_the_current_year_is_accepted() {
        let mut income = Income::default();
        let id = yearly_job(&mut income);
        statement(&mut income, id, "2026", "40000.00");
        assert_eq!(income.entry_count(id), 1);
    }

    #[test]
    fn test_csd001_ac_25_5_an_invalid_year_is_rejected() {
        for text in [
            "26", "2026-05", "abcd", "1899", "", " ", "20266", "-202", "2026 ",
        ] {
            let result = Year::parse(text);
            if text.trim() == "2026" {
                // Surrounding whitespace is trimmed like other typed input.
                assert_eq!(result.unwrap().to_string(), "2026");
            } else {
                assert_eq!(
                    result.unwrap_err().code(),
                    "STATEMENT_YEAR_INVALID",
                    "{text:?}"
                );
            }
        }
        assert_eq!(year("1900").to_string(), "1900");
        assert_eq!(year("9999").to_string(), "9999");
    }

    #[test]
    fn test_csd001_ac_26_1_a_duplicate_year_is_rejected() {
        let mut income = Income::default();
        let id = yearly_job(&mut income);
        statement(&mut income, id, "2025", "40000.00");
        let error = income
            .add_statement(id, year("2025"), amounts("41000.00", "", ""), today())
            .unwrap_err();
        assert_eq!(error.code(), "STATEMENT_DUPLICATE_YEAR");
        let kept = income.statements(id).unwrap();
        assert_eq!(kept.len(), 1);
        assert_eq!(kept[0].amounts.gross_salary().to_string(), "40000.00");
    }

    #[test]
    fn test_csd001_ac_26_2_the_same_year_on_another_job_is_allowed() {
        let mut income = Income::default();
        let first = yearly_job(&mut income);
        let second = yearly_job(&mut income);
        statement(&mut income, first, "2025", "40000.00");
        statement(&mut income, second, "2025", "30000.00");
        assert_eq!(income.entry_count(first), 1);
        assert_eq!(income.entry_count(second), 1);
    }

    #[test]
    fn test_csd001_ac_27_1_a_paycheck_for_a_yearly_job_is_rejected() {
        let mut income = Income::default();
        let id = yearly_job(&mut income);
        let error = income
            .add_paycheck(id, month("2026-09"), amounts("3000.00", "", ""), today())
            .unwrap_err();
        assert_eq!(error.code(), "JOB_PERIOD_MISMATCH");
        assert_eq!(income.entry_count(id), 0);
    }

    #[test]
    fn test_csd001_ac_27_2_a_statement_for_a_monthly_job_is_rejected() {
        let mut income = Income::default();
        let id = job_with(&mut income, "01/03/2024", "");
        let error = income
            .add_statement(id, year("2025"), amounts("40000.00", "", ""), today())
            .unwrap_err();
        assert_eq!(error.code(), "JOB_PERIOD_MISMATCH");
        assert_eq!(income.entry_count(id), 0);
    }

    #[test]
    fn adding_a_statement_to_an_unknown_job_is_rejected() {
        let mut income = Income::default();
        let error = income
            .add_statement(
                JobId::generate(),
                year("2025"),
                amounts("1.00", "", ""),
                today(),
            )
            .unwrap_err();
        assert_eq!(error, DomainError::JobNotFound);
    }

    #[test]
    fn test_csd001_ac_8_5_a_statement_can_be_edited_and_deleted() {
        let mut income = Income::default();
        let id = yearly_job(&mut income);
        statement(&mut income, id, "2025", "40000.00");
        let updated = income
            .update_statement(id, year("2025"), amounts("40000.00", "", "3000.00"))
            .unwrap();
        assert_eq!(updated.amounts.bonus().to_string(), "3000.00");
        assert_eq!(updated.amounts.total_gross().to_string(), "43000.00");
        income.delete_statement(id, year("2025")).unwrap();
        assert_eq!(income.statements(id).unwrap().len(), 0);
        assert_eq!(
            income.delete_statement(id, year("2025")).unwrap_err(),
            DomainError::StatementNotFound
        );
        assert_eq!(
            income
                .update_statement(id, year("2025"), amounts("1.00", "", ""))
                .unwrap_err(),
            DomainError::StatementNotFound
        );
    }

    #[test]
    fn test_csd001_ac_8_6_the_year_of_a_statement_is_fixed() {
        let mut income = Income::default();
        let id = yearly_job(&mut income);
        statement(&mut income, id, "2025", "40000.00");
        let updated = income
            .update_statement(id, year("2025"), amounts("41000.00", "", ""))
            .unwrap();
        assert_eq!(updated.year.to_string(), "2025");
        // The API has no way to move a statement: only the amounts are editable.
        assert_eq!(income.statements(id).unwrap().len(), 1);
    }

    #[test]
    fn test_csd001_ac_8_7_an_invalid_statement_edit_is_rejected() {
        let mut income = Income::default();
        let id = yearly_job(&mut income);
        statement(&mut income, id, "2025", "40000.00");
        let error = Amounts::parse("-1.00", "", "").unwrap_err();
        assert_eq!(error.code(), "PAYCHECK_AMOUNT_NEGATIVE");
        let kept = income.statements(id).unwrap();
        assert_eq!(kept[0].amounts.gross_salary().to_string(), "40000.00");
    }

    #[test]
    fn test_csd001_ac_9_4_copy_previous_year_uses_the_latest_statement() {
        let mut income = Income::default();
        let id = yearly_job(&mut income);
        statement(&mut income, id, "2024", "40000.00");
        income
            .add_statement(
                id,
                year("2025"),
                amounts("42000.00", "", "3000.00"),
                today(),
            )
            .unwrap();
        let prefill = income.prefill_from_latest(id).unwrap();
        assert_eq!(prefill.gross_salary.unwrap().to_string(), "42000.00");
        assert_eq!(prefill.holiday_pay.to_string(), "0.00");
        assert_eq!(prefill.bonus.to_string(), "0.00");
        assert_eq!(income.entry_count(id), 2);
    }

    #[test]
    fn test_csd001_ac_9_5_no_statement_to_copy() {
        let mut income = Income::default();
        let id = yearly_job(&mut income);
        let prefill = income.prefill_from_latest(id).unwrap();
        assert_eq!(prefill.gross_salary, None);
        assert_eq!(prefill.holiday_pay.to_string(), "0.00");
        assert_eq!(prefill.bonus.to_string(), "0.00");
    }

    #[test]
    fn test_csd001_ac_10_2_total_of_an_annual_statement() {
        let amounts = amounts("40000.00", "2000.00", "3000.00");
        assert_eq!(amounts.total_gross().to_string(), "45000.00");
    }

    #[test]
    fn test_csd001_ac_11_5_statements_count_in_their_year_next_to_paychecks() {
        let mut income = Income::default();
        let yearly = yearly_job(&mut income);
        let monthly = job_with(&mut income, "01/01/2020", "");
        income
            .add_statement(
                yearly,
                year("2025"),
                amounts("40000.00", "", "3000.00"),
                today(),
            )
            .unwrap();
        add(&mut income, monthly, "2025-12", "3000.00");

        let totals = income.yearly_totals();
        assert_eq!(totals.len(), 1);
        assert_eq!(totals[0].year, 2025);
        assert_eq!(totals[0].gross_salary.to_string(), "43000.00");
        assert_eq!(totals[0].holiday_pay.to_string(), "0.00");
        assert_eq!(totals[0].bonus.to_string(), "3000.00");
        assert_eq!(totals[0].total_gross.to_string(), "46000.00");
        assert_eq!(totals[0].entry_count, 2);
    }

    #[test]
    fn test_csd001_ac_11_6_a_statement_does_not_count_in_other_years() {
        let mut income = Income::default();
        let id = yearly_job(&mut income);
        statement(&mut income, id, "2025", "40000.00");
        let years: Vec<i32> = income.yearly_totals().iter().map(|t| t.year).collect();
        assert_eq!(years, [2025]);
    }

    #[test]
    fn test_csd001_ac_12_4_a_statement_sorts_as_december() {
        let mut income = Income::default();
        let yearly = yearly_job(&mut income);
        let monthly = job_with(&mut income, "01/01/2020", "");
        add(&mut income, monthly, "2025-12", "3000.00");
        add(&mut income, monthly, "2026-10", "3000.00");
        statement(&mut income, yearly, "2026", "40000.00");
        let periods: Vec<String> = income
            .all_entries()
            .into_iter()
            .map(|(_, entry)| entry.period())
            .collect();
        assert_eq!(periods, ["2026", "2026-10", "2025-12"]);
    }

    #[test]
    fn test_csd001_ac_12_5_the_same_period_is_ordered_by_job() {
        let mut income = Income::default();
        // "Example Employer" starts later, so it comes first in job order.
        let monthly = job_with(&mut income, "01/03/2024", "");
        let yearly = income
            .create_job(&JobInput {
                pay_period: Some("Yearly".to_owned()),
                ..input("Older Example Employer", "01/01/2020", "")
            })
            .unwrap()
            .id;
        // 2026-12 is later than today, so allow it for this ordering test.
        income
            .add_paycheck(
                monthly,
                month("2026-12"),
                amounts("3000.00", "", ""),
                month("2026-12"),
            )
            .unwrap();
        statement(&mut income, yearly, "2026", "40000.00");
        let rows: Vec<(String, bool)> = income
            .all_entries()
            .into_iter()
            .map(|(job, entry)| (job.employer.clone(), entry.is_statement()))
            .collect();
        assert_eq!(
            rows,
            [
                ("Example Employer".to_owned(), false),
                ("Older Example Employer".to_owned(), true),
            ]
        );
    }

    #[test]
    fn test_csd001_ac_23_3_entries_of_both_kinds_come_with_job_and_period() {
        let mut income = Income::default();
        let monthly = job_with(&mut income, "01/03/2024", "");
        let yearly = income
            .create_job(&JobInput {
                pay_period: Some("Yearly".to_owned()),
                ..input("Older Example Employer", "01/01/2020", "")
            })
            .unwrap()
            .id;
        add(&mut income, monthly, "2026-09", "3000.00");
        statement(&mut income, yearly, "2025", "40000.00");
        let rows: Vec<(&str, String, &str)> = income
            .all_entries()
            .into_iter()
            .map(|(job, entry)| {
                (
                    job.employer.as_str(),
                    entry.period(),
                    if entry.is_statement() {
                        "statement"
                    } else {
                        "paycheck"
                    },
                )
            })
            .collect();
        assert_eq!(
            rows,
            [
                ("Example Employer", "2026-09".to_owned(), "paycheck"),
                ("Older Example Employer", "2025".to_owned(), "statement"),
            ]
        );
    }

    #[test]
    fn test_csd001_ac_4_1_deleting_a_job_removes_its_statements() {
        let mut income = Income::default();
        let id = yearly_job(&mut income);
        statement(&mut income, id, "2024", "40000.00");
        statement(&mut income, id, "2025", "42000.00");
        income.delete_job(id).unwrap();
        assert_eq!(income.all_entries().len(), 0);
        assert_eq!(income.yearly_totals().len(), 0);
    }

    // ---- CSD-004 with annual statements ----

    #[test]
    fn test_csd004_ac_2_4_an_annual_statement_counts_in_full_this_year() {
        let mut income = Income::default();
        let yearly = yearly_job(&mut income);
        let monthly = job_with(&mut income, "01/01/2020", "");
        income
            .add_statement(
                yearly,
                year("2026"),
                amounts("40000.00", "", "3000.00"),
                today(),
            )
            .unwrap();
        add(&mut income, monthly, "2026-09", "3000.00");
        let totals = income.overview(today()).year_to_date.unwrap();
        assert_eq!(totals.gross_salary.to_string(), "43000.00");
        assert_eq!(totals.holiday_pay.to_string(), "0.00");
        assert_eq!(totals.bonus.to_string(), "3000.00");
        assert_eq!(totals.total_gross.to_string(), "46000.00");
        assert_eq!(totals.entry_count, 2);
    }

    #[test]
    fn test_csd004_ac_2_5_statements_of_other_years_are_not_counted() {
        let mut income = Income::default();
        let id = yearly_job(&mut income);
        statement(&mut income, id, "2025", "40000.00");
        assert_eq!(income.overview(today()).year_to_date, None);
    }

    #[test]
    fn test_csd004_ac_3_4_annual_statements_are_not_last_month() {
        let mut income = Income::default();
        let id = yearly_job(&mut income);
        statement(&mut income, id, "2026", "40000.00");
        let overview = income.overview(today());
        assert_eq!(overview.last_month.unwrap().to_string(), "2026-09");
        assert_eq!(overview.last_month_totals, None);
        // The statement is still part of this year.
        assert!(overview.year_to_date.is_some());
    }

    // ---- data from outside (a project file) ----

    #[test]
    fn validate_rejects_entries_that_do_not_match_the_pay_period() {
        let mut income = Income::default();
        let monthly = job_with(&mut income, "01/03/2024", "");
        let yearly = yearly_job(&mut income);
        add(&mut income, monthly, "2026-09", "3000.00");
        statement(&mut income, yearly, "2025", "40000.00");
        assert!(income.validate().is_ok());

        // A paycheck of the yearly job, and a statement of the monthly job.
        let mut wrong_paycheck = income.clone();
        wrong_paycheck.paychecks[0].job_id = yearly;
        assert_eq!(
            wrong_paycheck.validate().unwrap_err(),
            DomainError::DataInvalid
        );
        let mut wrong_statement = income.clone();
        wrong_statement.statements[0].job_id = monthly;
        assert_eq!(
            wrong_statement.validate().unwrap_err(),
            DomainError::DataInvalid
        );

        let mut duplicate = income.clone();
        duplicate.statements.push(duplicate.statements[0].clone());
        assert_eq!(duplicate.validate().unwrap_err(), DomainError::DataInvalid);

        let mut orphan = income.clone();
        orphan.statements[0].job_id = JobId::generate();
        assert_eq!(orphan.validate().unwrap_err(), DomainError::DataInvalid);
    }

    #[test]
    fn year_orders_and_prints_with_four_digits() {
        assert!(year("2025") < year("2026"));
        assert_eq!(year("1900").to_string(), "1900");
        assert_eq!(Year::new(1899), None);
        assert_eq!(Year::new(10000), None);
        let json = serde_json::to_string(&year("2025")).unwrap();
        assert_eq!(json, "\"2025\"");
        assert_eq!(serde_json::from_str::<Year>(&json).unwrap(), year("2025"));
        assert!(serde_json::from_str::<Year>("\"25\"").is_err());
    }

    // ---- supporting types ----

    #[test]
    fn year_month_round_trips_and_orders() {
        assert_eq!(month("2026-09").to_string(), "2026-09");
        assert!(month("2025-12") < month("2026-01"));
        for bad in [
            "2026-9", "26-09", "2026-13", "2026-00", "2026/09", "", "1899-12",
        ] {
            assert_eq!(
                YearMonth::parse(bad).unwrap_err(),
                DomainError::PaycheckMonthInvalid,
                "{bad:?}"
            );
        }
    }

    #[test]
    fn validate_rejects_inconsistent_data() {
        let mut income = Income::default();
        let id = job_with(&mut income, "01/03/2024", "");
        add(&mut income, id, "2026-09", "3000.00");
        assert!(income.validate().is_ok());

        let mut orphan = income.clone();
        orphan.paychecks[0].job_id = JobId::generate();
        assert_eq!(orphan.validate().unwrap_err(), DomainError::DataInvalid);

        let mut duplicate = income.clone();
        duplicate.paychecks.push(duplicate.paychecks[0].clone());
        assert_eq!(duplicate.validate().unwrap_err(), DomainError::DataInvalid);

        let mut blank = income.clone();
        blank.jobs[0].employer = "  ".into();
        assert_eq!(blank.validate().unwrap_err(), DomainError::DataInvalid);
    }

    // ---- CSD-006: income insights ----

    fn yearly_job_named(income: &mut Income, employer: &str) -> JobId {
        income
            .create_job(&JobInput {
                pay_period: Some("Yearly".to_owned()),
                ..input(employer, "01/01/2020", "")
            })
            .unwrap()
            .id
    }

    fn add_statement_of(income: &mut Income, job: JobId, y: &str, parts: [&str; 3]) {
        income
            .add_statement(job, year(y), amounts(parts[0], parts[1], parts[2]), today())
            .unwrap();
    }

    fn years_of(series: &[InsightsYear]) -> Vec<i32> {
        series.iter().map(|y| y.year).collect()
    }

    fn growth_of(series: &[InsightsYear], y: i32) -> Option<String> {
        series
            .iter()
            .find(|row| row.year == y)
            .unwrap()
            .growth_percent
            .map(|growth| growth.to_string())
    }

    #[test]
    fn test_csd006_ac_2_1_series_runs_from_first_entry_to_current_year() {
        let mut income = Income::default();
        let id = job_with(&mut income, "01/01/2024", "");
        add(&mut income, id, "2024-03", "3000.00");
        add(&mut income, id, "2025-05", "3000.00");
        add(&mut income, id, "2026-09", "3000.00");
        assert_eq!(years_of(&income.insights(today())), [2024, 2025, 2026]);
    }

    #[test]
    fn test_csd006_ac_2_2_two_jobs_are_summed() {
        let mut income = Income::default();
        let employer = yearly_job_named(&mut income, "Example Employer");
        let agency = yearly_job_named(&mut income, "Example Agency");
        add_statement_of(
            &mut income,
            employer,
            "2025",
            ["30000.00", "2400.00", "0.00"],
        );
        add_statement_of(&mut income, agency, "2025", ["5000.00", "0.00", "1000.00"]);
        let series = income.insights(today());
        let row = series.iter().find(|row| row.year == 2025).unwrap();
        assert_eq!(row.gross_salary.to_string(), "35000.00");
        assert_eq!(row.holiday_pay.to_string(), "2400.00");
        assert_eq!(row.bonus.to_string(), "1000.00");
        assert_eq!(row.total_gross.to_string(), "38400.00");
        assert_eq!(row.entry_count, 2);
    }

    #[test]
    fn test_csd006_ac_2_3_a_single_year() {
        let mut income = Income::default();
        let id = job_with(&mut income, "01/03/2024", "");
        add(&mut income, id, "2026-09", "3000.00");
        assert_eq!(years_of(&income.insights(today())), [2026]);
    }

    #[test]
    fn test_csd006_ac_3_1_gap_year_is_zero() {
        let mut income = Income::default();
        let id = yearly_job_named(&mut income, "Example Employer");
        add_statement_of(&mut income, id, "2022", ["30000.00", "", ""]);
        add_statement_of(&mut income, id, "2024", ["31000.00", "", ""]);
        let series = income.insights(today());
        assert_eq!(years_of(&series), [2022, 2023, 2024, 2025, 2026]);
        let gap = &series[1];
        assert_eq!(gap.total_gross.to_string(), "0.00");
        assert_eq!(gap.gross_salary.to_string(), "0.00");
        assert_eq!(gap.entry_count, 0);
    }

    #[test]
    fn test_csd006_ac_4_1_only_the_current_year_is_partial() {
        let mut income = Income::default();
        let id = job_with(&mut income, "01/01/2024", "");
        add(&mut income, id, "2025-05", "3000.00");
        add(&mut income, id, "2026-09", "3000.00");
        let series = income.insights(today());
        assert!(!series[0].is_partial);
        assert!(series[1].is_partial);
    }

    #[test]
    fn test_csd006_ac_5_1_growth_per_year() {
        for (previous, current, growth) in [
            ("32400.00", "34480.00", "6.4"),
            ("40000.00", "39000.00", "-2.5"),
            // Banker's rounding: 0.45 goes to the even 0.4, 0.55 to 0.6.
            ("20000.00", "20090.00", "0.4"),
            ("20000.00", "20110.00", "0.6"),
            ("20000.00", "20000.00", "0.0"),
            // A decrease that rounds to zero shows no sign.
            ("100000.00", "99960.00", "0.0"),
        ] {
            let mut income = Income::default();
            let id = yearly_job_named(&mut income, "Example Employer");
            add_statement_of(&mut income, id, "2023", [previous, "", ""]);
            add_statement_of(&mut income, id, "2024", [current, "", ""]);
            let series = income.insights(today());
            assert_eq!(
                growth_of(&series, 2024).as_deref(),
                Some(growth),
                "{previous} -> {current}"
            );
        }
    }

    #[test]
    fn test_csd006_ac_6_1_first_year_has_no_growth() {
        let mut income = Income::default();
        let id = yearly_job_named(&mut income, "Example Employer");
        add_statement_of(&mut income, id, "2024", ["31000.00", "", ""]);
        assert_eq!(growth_of(&income.insights(today()), 2024), None);
    }

    #[test]
    fn test_csd006_ac_6_2_current_year_has_no_growth() {
        let mut income = Income::default();
        let id = job_with(&mut income, "01/01/2025", "");
        add(&mut income, id, "2025-05", "34480.00");
        add(&mut income, id, "2026-05", "28000.00");
        assert_eq!(growth_of(&income.insights(today()), 2026), None);
    }

    #[test]
    fn test_csd006_ac_6_3_year_after_a_gap_has_no_growth() {
        let mut income = Income::default();
        let id = yearly_job_named(&mut income, "Example Employer");
        add_statement_of(&mut income, id, "2022", ["30000.00", "", ""]);
        add_statement_of(&mut income, id, "2024", ["31000.00", "", ""]);
        let series = income.insights(today());
        assert_eq!(growth_of(&series, 2023), None);
        assert_eq!(growth_of(&series, 2024), None);
    }

    #[test]
    fn test_csd006_ac_10_1_no_entries_gives_an_empty_series() {
        let mut income = Income::default();
        assert_eq!(income.insights(today()).len(), 0);
        job_with(&mut income, "01/03/2024", "");
        assert_eq!(income.insights(today()).len(), 0);
    }
}
