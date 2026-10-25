use std::collections::BTreeMap;
use std::fmt;
use std::str::FromStr;

use rust_decimal::Decimal;
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
}

/// The monthly pay record of one job (CSD-001 REQ-5).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Paycheck {
    pub job_id: JobId,
    pub month: YearMonth,
    #[serde(flatten)]
    pub amounts: Amounts,
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
}

struct ValidJob {
    employer: String,
    title: Option<String>,
    start_date: Date,
    end_date: Option<Date>,
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
        if end_date.is_some_and(|end| end < start_date) {
            return Err(DomainError::JobEndBeforeStart);
        }
        Ok(ValidJob {
            employer: employer.to_owned(),
            title: title.map(str::to_owned),
            start_date,
            end_date,
        })
    }
}

/// The sums of some paychecks, or `None` if there are none.
fn totals_of<'a>(paychecks: impl Iterator<Item = &'a Paycheck>) -> Option<PeriodTotals> {
    let mut totals = PeriodTotals {
        gross_salary: zero(),
        holiday_pay: zero(),
        bonus: zero(),
        total_gross: zero(),
        paycheck_count: 0,
    };
    for paycheck in paychecks {
        totals.gross_salary += paycheck.amounts.gross_salary();
        totals.holiday_pay += paycheck.amounts.holiday_pay();
        totals.bonus += paycheck.amounts.bonus();
        totals.total_gross += paycheck.amounts.total_gross();
        totals.paycheck_count += 1;
    }
    (totals.paycheck_count > 0).then_some(totals)
}

fn non_blank(text: Option<&str>) -> Option<&str> {
    text.map(str::trim).filter(|text| !text.is_empty())
}

/// Totals of one job for one calendar year (CSD-001 REQ-11).
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct YearlyTotal {
    pub year: i32,
    pub gross_salary: Decimal,
    pub holiday_pay: Decimal,
    pub bonus: Decimal,
    pub total_gross: Decimal,
    pub paycheck_count: usize,
}

/// Sums of a set of paychecks over all jobs (CSD-004).
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct PeriodTotals {
    pub gross_salary: Decimal,
    pub holiday_pay: Decimal,
    pub bonus: Decimal,
    pub total_gross: Decimal,
    pub paycheck_count: usize,
}

/// The two blocks of the dashboard (CSD-004): this year up to the current
/// month, and last month. A period without paychecks has no totals.
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

/// All employment income: jobs and their paychecks.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Income {
    jobs: Vec<Job>,
    paychecks: Vec<Paycheck>,
}

impl Income {
    /// Creates a job.
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
        };
        self.jobs.push(job.clone());
        Ok(job)
    }

    /// Replaces the fields of a job. Dates never conflict with paychecks.
    ///
    /// # Errors
    /// Returns `JobNotFound` or a validation error.
    pub fn update_job(&mut self, id: JobId, input: &JobInput) -> Result<Job, DomainError> {
        let valid = input.validate()?;
        let job = self.job_mut(id)?;
        job.employer = valid.employer;
        job.title = valid.title;
        job.start_date = valid.start_date;
        job.end_date = valid.end_date;
        Ok(job.clone())
    }

    /// Removes a job and all its paychecks (REQ-4).
    ///
    /// # Errors
    /// Returns `JobNotFound`.
    pub fn delete_job(&mut self, id: JobId) -> Result<(), DomainError> {
        self.job(id)?;
        self.jobs.retain(|job| job.id != id);
        self.paychecks.retain(|paycheck| paycheck.job_id != id);
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

    /// How many paychecks a job has.
    #[must_use]
    pub fn paycheck_count(&self, id: JobId) -> usize {
        self.paychecks.iter().filter(|p| p.job_id == id).count()
    }

    /// Adds the paycheck of a month. `today` is the current month; later months
    /// are rejected. The job's own start and end dates do not matter (REQ-5).
    ///
    /// # Errors
    /// Returns `JobNotFound`, `PaycheckMonthInFuture` or `PaycheckDuplicateMonth`.
    pub fn add_paycheck(
        &mut self,
        job_id: JobId,
        month: YearMonth,
        amounts: Amounts,
        today: YearMonth,
    ) -> Result<Paycheck, DomainError> {
        self.job(job_id)?;
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

    /// The paychecks of a job, newest month first (REQ-12).
    ///
    /// # Errors
    /// Returns `JobNotFound`.
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

    /// Values to pre-fill the add form: the gross salary of the latest paycheck
    /// and zero holiday pay and bonus (REQ-9).
    ///
    /// # Errors
    /// Returns `JobNotFound`.
    pub fn prefill_from_latest(&self, job_id: JobId) -> Result<PaycheckPrefill, DomainError> {
        let latest = self.paychecks(job_id)?.into_iter().next();
        Ok(PaycheckPrefill {
            gross_salary: latest.map(|p| p.amounts.gross_salary()),
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
    /// summed over all jobs (CSD-004 REQ-2 to REQ-4). Months after `today`,
    /// which only a project file can contain, are not counted.
    #[must_use]
    pub fn overview(&self, today: YearMonth) -> IncomeOverview {
        let last_month = today.previous();
        let year_to_date = totals_of(
            self.paychecks
                .iter()
                .filter(|p| p.month.year() == today.year() && p.month <= today),
        );
        let last_month_totals = last_month
            .and_then(|last| totals_of(self.paychecks.iter().filter(|p| p.month == last)));
        IncomeOverview {
            year: today.year(),
            year_to_date,
            last_month,
            last_month_totals,
        }
    }

    /// The paychecks of all jobs with their job, newest month first and, within
    /// a month, in job order (REQ-12).
    #[must_use]
    pub fn all_paychecks(&self) -> Vec<(&Job, &Paycheck)> {
        let mut rows: Vec<(usize, &Job, &Paycheck)> = Vec::new();
        for (position, job) in self.jobs().into_iter().enumerate() {
            rows.extend(
                self.paychecks
                    .iter()
                    .filter(|paycheck| paycheck.job_id == job.id)
                    .map(|paycheck| (position, job, paycheck)),
            );
        }
        rows.sort_by(|a, b| b.2.month.cmp(&a.2.month).then(a.0.cmp(&b.0)));
        rows.into_iter()
            .map(|(_, job, paycheck)| (job, paycheck))
            .collect()
    }

    /// Sums per calendar year over all jobs, newest year first, without
    /// extrapolating partial years (REQ-11).
    #[must_use]
    pub fn yearly_totals(&self) -> Vec<YearlyTotal> {
        let mut by_year: BTreeMap<i32, YearlyTotal> = BTreeMap::new();
        for paycheck in &self.paychecks {
            let year = paycheck.month.year();
            let total = by_year.entry(year).or_insert_with(|| YearlyTotal {
                year,
                gross_salary: zero(),
                holiday_pay: zero(),
                bonus: zero(),
                total_gross: zero(),
                paycheck_count: 0,
            });
            total.gross_salary += paycheck.amounts.gross_salary();
            total.holiday_pay += paycheck.amounts.holiday_pay();
            total.bonus += paycheck.amounts.bonus();
            total.total_gross += paycheck.amounts.total_gross();
            total.paycheck_count += 1;
        }
        by_year.into_values().rev().collect()
    }

    /// Checks the invariants of data that came from outside, such as a file.
    ///
    /// # Errors
    /// Returns `DataInvalid` for duplicate jobs or months, paychecks of unknown
    /// jobs, blank employers, an end date before a start date, or amounts that
    /// break the paycheck rules.
    pub fn validate(&self) -> Result<(), DomainError> {
        let mut job_ids = std::collections::HashSet::new();
        for job in &self.jobs {
            let employer_ok = !job.employer.trim().is_empty();
            let dates_ok = job.end_date.is_none_or(|end| end >= job.start_date);
            if !employer_ok || !dates_ok || !job_ids.insert(job.id) {
                return Err(DomainError::DataInvalid);
            }
        }
        let mut seen = std::collections::HashSet::new();
        for paycheck in &self.paychecks {
            let amounts = &paycheck.amounts;
            let amounts_ok = Amounts::new(
                amounts.gross_salary(),
                amounts.holiday_pay(),
                amounts.bonus(),
            )
            .is_ok();
            if !job_ids.contains(&paycheck.job_id)
                || !seen.insert((paycheck.job_id, paycheck.month))
                || !amounts_ok
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
        assert_eq!(income.paycheck_count(id), 1);
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
        assert_eq!(income.paycheck_count(id), 0);
        assert_eq!(income.paycheck_count(other), 1);
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
        assert_eq!(income.paycheck_count(id), 1);
    }

    #[test]
    fn test_csd001_ac_5_3_month_after_job_end_is_accepted() {
        let mut income = Income::default();
        let id = job_with(&mut income, "01/03/2024", "31/01/2025");
        add(&mut income, id, "2025-03", "1500.00");
        assert_eq!(income.paycheck_count(id), 1);
    }

    #[test]
    fn test_csd001_ac_5_4_month_later_than_current_is_rejected() {
        let mut income = Income::default();
        let id = job_with(&mut income, "01/03/2024", "");
        let error = income
            .add_paycheck(id, month("2026-11"), amounts("3000.00", "", ""), today())
            .unwrap_err();
        assert_eq!(error.code(), "PAYCHECK_MONTH_IN_FUTURE");
        assert_eq!(income.paycheck_count(id), 0);
    }

    #[test]
    fn test_csd001_ac_5_5_current_month_is_accepted() {
        let mut income = Income::default();
        let id = job_with(&mut income, "01/03/2024", "");
        add(&mut income, id, "2026-10", "3000.00");
        assert_eq!(income.paycheck_count(id), 1);
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
        assert_eq!(income.paycheck_count(first), 1);
        assert_eq!(income.paycheck_count(second), 1);
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
        assert_eq!(income.paycheck_count(id), 1);
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
        assert_eq!(income.paycheck_count(id), 2);
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
        assert_eq!(y2026.paycheck_count, 1);
        assert_eq!(y2025.year, 2025);
        assert_eq!(y2025.gross_salary.to_string(), "6000.00");
        assert_eq!(y2025.holiday_pay.to_string(), "0.00");
        assert_eq!(y2025.bonus.to_string(), "1000.00");
        assert_eq!(y2025.total_gross.to_string(), "7000.00");
        assert_eq!(y2025.paycheck_count, 2);
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
        assert_eq!(year.paycheck_count, 3);
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
            .all_paychecks()
            .into_iter()
            .map(|(job, paycheck)| (paycheck.month.to_string(), job.id))
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
        assert_eq!(income.all_paychecks().len(), 0);
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
            .all_paychecks()
            .into_iter()
            .map(|(job, paycheck)| (job.employer.as_str(), paycheck.month.to_string()))
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
        assert_eq!(totals.paycheck_count, 4);
    }

    #[test]
    fn test_csd004_ac_2_2_other_years_are_not_counted() {
        let mut income = Income::default();
        let id = job_with(&mut income, "01/03/2024", "");
        add(&mut income, id, "2025-12", "3000.00");
        add(&mut income, id, "2026-09", "1000.00");
        let totals = income.overview(today()).year_to_date.unwrap();
        assert_eq!(totals.gross_salary.to_string(), "1000.00");
        assert_eq!(totals.paycheck_count, 1);
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
        assert_eq!(totals.paycheck_count, 1);
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
        assert_eq!(totals.paycheck_count, 2);
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
        assert_eq!(totals.paycheck_count, 1);
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
}
