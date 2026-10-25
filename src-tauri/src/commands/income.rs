#![allow(
    clippy::needless_pass_by_value,
    reason = "Tauri commands receive their arguments by value"
)]

use serde::{Deserialize, Serialize};
use tauri::State;

use super::AppState;
use crate::domain::{
    Amounts, Income, Job, JobId, JobInput, Paycheck, PaycheckPrefill, YearMonth, YearlyTotal,
};
use crate::error::AppError;

#[derive(Serialize)]
pub struct JobView {
    #[serde(flatten)]
    job: Job,
    paycheck_count: usize,
}

impl JobView {
    fn new(income: &Income, job: &Job) -> Self {
        Self {
            job: job.clone(),
            paycheck_count: income.paycheck_count(job.id),
        }
    }
}

/// A paycheck with the job it belongs to, as the paychecks table shows it.
#[derive(Serialize)]
pub struct PaycheckView {
    #[serde(flatten)]
    paycheck: Paycheck,
    total_gross: String,
    employer: String,
}

impl PaycheckView {
    fn new(job: &Job, paycheck: &Paycheck) -> Self {
        Self {
            total_gross: paycheck.amounts.total_gross().to_string(),
            paycheck: paycheck.clone(),
            employer: job.employer.clone(),
        }
    }

    fn of(income: &Income, paycheck: &Paycheck) -> Result<Self, AppError> {
        Ok(Self::new(income.job(paycheck.job_id)?, paycheck))
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

/// The paychecks of all jobs, newest month first (CSD-001 REQ-12).
#[tauri::command]
pub fn list_paychecks(state: State<'_, AppState>) -> Result<Vec<PaycheckView>, AppError> {
    let project = state.project()?;
    Ok(project
        .income()
        .all_paychecks()
        .into_iter()
        .map(|(job, paycheck)| PaycheckView::new(job, paycheck))
        .collect())
}

#[tauri::command]
pub fn add_paycheck(
    state: State<'_, AppState>,
    job_id: String,
    month: String,
    amounts: AmountsInput,
) -> Result<PaycheckView, AppError> {
    let today = state.today();
    let job_id = JobId::parse(&job_id)?;
    let month = YearMonth::parse(&month)?;
    let amounts = amounts.parse()?;
    let mut project = state.project()?;
    let paycheck = project
        .income_mut()
        .add_paycheck(job_id, month, amounts, today)?;
    PaycheckView::of(project.income(), &paycheck)
}

#[tauri::command]
pub fn update_paycheck(
    state: State<'_, AppState>,
    job_id: String,
    month: String,
    amounts: AmountsInput,
) -> Result<PaycheckView, AppError> {
    let job_id = JobId::parse(&job_id)?;
    let month = YearMonth::parse(&month)?;
    let amounts = amounts.parse()?;
    let mut project = state.project()?;
    let paycheck = project
        .income_mut()
        .update_paycheck(job_id, month, amounts)?;
    PaycheckView::of(project.income(), &paycheck)
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
pub fn copy_previous_paycheck(
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
pub fn yearly_totals(state: State<'_, AppState>) -> Result<Vec<YearlyTotal>, AppError> {
    let project = state.project()?;
    Ok(project.income().yearly_totals())
}
