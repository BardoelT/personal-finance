//! Employment income: jobs, their paychecks and annual statements (CSD-001), and
//! the totals and insights built on them (CSD-001 REQ-11, CSD-004, CSD-006).
//!
//! `Income` holds the data and the rules for changing it; the other files add
//! the value types (`dates`, `job`, `entry`) and the figures derived from it
//! (`totals`, `insights`).

mod dates;
mod entry;
mod insights;
mod job;
mod totals;

#[cfg(test)]
mod tests;

pub use dates::{Year, YearMonth};
pub use entry::{AnnualStatement, Entry, Paycheck, PaycheckPrefill};
pub use insights::IncomeInsightsYear;
pub use job::{Job, JobId, JobInput, PayPeriod};
pub use totals::{IncomeOverview, IncomeYearlyTotal};

use serde::{Deserialize, Serialize};

use crate::domain::error::DomainError;
use crate::domain::money::{zero, Amounts};

/// All employment income: jobs with their paychecks or annual statements.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Income {
    jobs: Vec<Job>,
    paychecks: Vec<Paycheck>,
    /// A file without annual statements (an older one) has none.
    #[serde(default)]
    statements: Vec<AnnualStatement>,
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
    ) -> Result<AnnualStatement, DomainError> {
        if self.job(job_id)?.pay_period != PayPeriod::Yearly {
            return Err(DomainError::JobPeriodMismatch);
        }
        if year.value() > today.year() {
            return Err(DomainError::StatementYearInFuture);
        }
        if self.statement(job_id, year).is_some() {
            return Err(DomainError::StatementDuplicateYear);
        }
        let statement = AnnualStatement {
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
    ) -> Result<AnnualStatement, DomainError> {
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
    pub fn statements(&self, job_id: JobId) -> Result<Vec<&AnnualStatement>, DomainError> {
        self.job(job_id)?;
        let mut statements: Vec<&AnnualStatement> = self
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

    /// All entries, paychecks and annual statements.
    fn entries(&self) -> impl Iterator<Item = Entry<'_>> {
        self.paychecks
            .iter()
            .map(Entry::Paycheck)
            .chain(self.statements.iter().map(Entry::AnnualStatement))
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

    fn statement(&self, job_id: JobId, year: Year) -> Option<&AnnualStatement> {
        self.statements
            .iter()
            .find(|s| s.job_id == job_id && s.year == year)
    }
}
