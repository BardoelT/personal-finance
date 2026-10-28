#![allow(
    clippy::needless_pass_by_value,
    reason = "Tauri commands receive their arguments by value"
)]

use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use tauri::State;

use super::AppState;
use crate::domain::{
    Amounts, Entry, Income, IncomeInsightsYear, IncomeYearlyTotal, Job, JobId, JobInput,
    PaycheckPrefill, Year, YearMonth,
};
use crate::error::AppError;

#[derive(Serialize)]
pub struct JobView {
    #[serde(flatten)]
    job: Job,
    /// Paychecks of a monthly job, or annual statements of a yearly job.
    entry_count: usize,
}

impl JobView {
    fn new(income: &Income, job: &Job) -> Self {
        Self {
            job: job.clone(),
            entry_count: income.entry_count(job.id),
        }
    }
}

/// A paycheck or an annual statement with the job it belongs to, as the entries
/// table shows it (CSD-001 REQ-23).
#[derive(Serialize)]
pub struct EntryView {
    job_id: JobId,
    employer: String,
    /// `"paycheck"` or `"statement"`.
    kind: &'static str,
    /// `YYYY-MM` for a paycheck, `YYYY` for an annual statement.
    period: String,
    gross_salary: Decimal,
    holiday_pay: Decimal,
    bonus: Decimal,
    total_gross: Decimal,
}

impl EntryView {
    fn new(job: &Job, entry: Entry<'_>) -> Self {
        let amounts = entry.amounts();
        Self {
            job_id: job.id,
            employer: job.employer.clone(),
            kind: if entry.is_statement() {
                "statement"
            } else {
                "paycheck"
            },
            period: entry.period(),
            gross_salary: amounts.gross_salary(),
            holiday_pay: amounts.holiday_pay(),
            bonus: amounts.bonus(),
            total_gross: amounts.total_gross(),
        }
    }

    fn of(income: &Income, entry: Entry<'_>) -> Result<Self, AppError> {
        Ok(Self::new(income.job(entry.job_id())?, entry))
    }
}

/// Amounts as typed in the form; empty holiday pay and bonus mean `0.00`.
#[derive(Deserialize)]
pub struct AmountsInput {
    #[serde(default)]
    gross_salary: String,
    #[serde(default)]
    holiday_pay: String,
    #[serde(default)]
    bonus: String,
}

impl AmountsInput {
    fn parse(&self) -> Result<Amounts, AppError> {
        Ok(Amounts::parse(
            &self.gross_salary,
            &self.holiday_pay,
            &self.bonus,
        )?)
    }
}

#[tauri::command]
pub fn list_jobs(state: State<'_, AppState>) -> Result<Vec<JobView>, AppError> {
    let project = state.project()?;
    let income = project.income();
    Ok(income
        .jobs()
        .into_iter()
        .map(|job| JobView::new(income, job))
        .collect())
}

#[tauri::command]
pub fn create_job(state: State<'_, AppState>, input: JobInput) -> Result<JobView, AppError> {
    let mut project = state.project()?;
    let job = project.income_mut().create_job(&input)?;
    Ok(JobView::new(project.income(), &job))
}

#[tauri::command]
pub fn update_job(
    state: State<'_, AppState>,
    job_id: String,
    input: JobInput,
) -> Result<JobView, AppError> {
    let mut project = state.project()?;
    let job = project
        .income_mut()
        .update_job(JobId::parse(&job_id)?, &input)?;
    Ok(JobView::new(project.income(), &job))
}

#[tauri::command]
pub fn delete_job(state: State<'_, AppState>, job_id: String) -> Result<(), AppError> {
    let mut project = state.project()?;
    project.income_mut().delete_job(JobId::parse(&job_id)?)?;
    Ok(())
}

/// The paychecks and annual statements of all jobs, newest period first
/// (CSD-001 REQ-12).
#[tauri::command]
pub fn list_entries(state: State<'_, AppState>) -> Result<Vec<EntryView>, AppError> {
    let project = state.project()?;
    Ok(project
        .income()
        .all_entries()
        .into_iter()
        .map(|(job, entry)| EntryView::new(job, entry))
        .collect())
}

#[tauri::command]
pub fn add_paycheck(
    state: State<'_, AppState>,
    job_id: String,
    month: String,
    amounts: AmountsInput,
) -> Result<EntryView, AppError> {
    let today = state.today();
    let job_id = JobId::parse(&job_id)?;
    let month = YearMonth::parse(&month)?;
    let amounts = amounts.parse()?;
    let mut project = state.project()?;
    let paycheck = project
        .income_mut()
        .add_paycheck(job_id, month, amounts, today)?;
    EntryView::of(project.income(), Entry::Paycheck(&paycheck))
}

#[tauri::command]
pub fn add_statement(
    state: State<'_, AppState>,
    job_id: String,
    year: String,
    amounts: AmountsInput,
) -> Result<EntryView, AppError> {
    let today = state.today();
    let job_id = JobId::parse(&job_id)?;
    let year = Year::parse(&year)?;
    let amounts = amounts.parse()?;
    let mut project = state.project()?;
    let statement = project
        .income_mut()
        .add_statement(job_id, year, amounts, today)?;
    EntryView::of(project.income(), Entry::AnnualStatement(&statement))
}

#[tauri::command]
pub fn update_paycheck(
    state: State<'_, AppState>,
    job_id: String,
    month: String,
    amounts: AmountsInput,
) -> Result<EntryView, AppError> {
    let job_id = JobId::parse(&job_id)?;
    let month = YearMonth::parse(&month)?;
    let amounts = amounts.parse()?;
    let mut project = state.project()?;
    let paycheck = project
        .income_mut()
        .update_paycheck(job_id, month, amounts)?;
    EntryView::of(project.income(), Entry::Paycheck(&paycheck))
}

#[tauri::command]
pub fn update_statement(
    state: State<'_, AppState>,
    job_id: String,
    year: String,
    amounts: AmountsInput,
) -> Result<EntryView, AppError> {
    let job_id = JobId::parse(&job_id)?;
    let year = Year::parse(&year)?;
    let amounts = amounts.parse()?;
    let mut project = state.project()?;
    let statement = project
        .income_mut()
        .update_statement(job_id, year, amounts)?;
    EntryView::of(project.income(), Entry::AnnualStatement(&statement))
}

#[tauri::command]
pub fn delete_paycheck(
    state: State<'_, AppState>,
    job_id: String,
    month: String,
) -> Result<(), AppError> {
    let job_id = JobId::parse(&job_id)?;
    let month = YearMonth::parse(&month)?;
    let mut project = state.project()?;
    project.income_mut().delete_paycheck(job_id, month)?;
    Ok(())
}

#[tauri::command]
pub fn delete_statement(
    state: State<'_, AppState>,
    job_id: String,
    year: String,
) -> Result<(), AppError> {
    let job_id = JobId::parse(&job_id)?;
    let year = Year::parse(&year)?;
    let mut project = state.project()?;
    project.income_mut().delete_statement(job_id, year)?;
    Ok(())
}

/// Form values for "copy previous month" or "copy previous year" (CSD-001 REQ-9).
#[tauri::command]
pub fn copy_previous_entry(
    state: State<'_, AppState>,
    job_id: String,
) -> Result<PaycheckPrefill, AppError> {
    let project = state.project()?;
    Ok(project
        .income()
        .prefill_from_latest(JobId::parse(&job_id)?)?)
}

/// Yearly totals summed over all jobs (CSD-001 REQ-11).
#[tauri::command]
pub fn yearly_totals(state: State<'_, AppState>) -> Result<Vec<IncomeYearlyTotal>, AppError> {
    let project = state.project()?;
    Ok(project.income().yearly_totals())
}

/// The yearly chart of the Income page, over all jobs, from the project as it
/// is now, saved or not (CSD-006 REQ-11).
#[tauri::command]
pub fn get_income_insights(state: State<'_, AppState>) -> Result<IncomeInsights, AppError> {
    let today = state.today();
    let project = state.project()?;
    Ok(IncomeInsights {
        years: project.income().insights(today),
    })
}

/// Payload of `get_income_insights` (CSD-006 section 5).
#[derive(Serialize)]
pub struct IncomeInsights {
    years: Vec<IncomeInsightsYear>,
}
