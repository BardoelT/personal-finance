#![allow(
    clippy::needless_pass_by_value,
    reason = "Tauri commands receive their arguments by value"
)]

use rust_decimal::Decimal;
use serde::Serialize;
use tauri::State;

use super::AppState;
use crate::domain::debts::OpenDebt;
use crate::domain::debts::{debt_note, parse_period, DebtBalance};
use crate::domain::houses::linking::{self, DebtFormInput, DebtsView};
use crate::domain::HouseId;
use crate::domain::{
    Debt, DebtId, DebtInsights, DebtStatement, DebtStatementPrefill, DebtYearlyTotal, Debts,
};
use crate::error::AppError;
use crate::storage::Project;

/// A statement with the name of its debt and its change, as the statements
/// table shows it (CSD-011 REQ-20).
#[derive(Serialize)]
pub struct StatementView {
    debt_id: DebtId,
    debt_name: String,
    /// `YYYY-MM` for a monthly debt, `YYYY` for a yearly one.
    period: String,
    balance: Decimal,
    /// The previous statement's balance minus this one; `None` for a first
    /// statement (REQ-25).
    change: Option<Decimal>,
    note: String,
}

impl StatementView {
    fn new(debts: &Debts, debt: &Debt, statement: &DebtStatement) -> Self {
        Self {
            debt_id: debt.id,
            debt_name: debt.name.clone(),
            period: statement.period.to_string(),
            balance: statement.balance.value(),
            change: debts.change(statement),
            note: statement.note.as_str().to_owned(),
        }
    }
}

/// The debts with their latest balances and the total debt (REQ-17).
#[tauri::command]
pub fn list_debts(state: State<'_, AppState>) -> Result<DebtsView, AppError> {
    let project = state.project()?;
    Ok(linking::debts_view(project.debts(), project.houses()))
}

#[tauri::command]
pub fn create_debt(state: State<'_, AppState>, input: DebtFormInput) -> Result<Debt, AppError> {
    let mut project = state.project()?;
    let (houses, debts) = project.houses_and_debts_mut();
    Ok(linking::create_debt(houses, debts, &input)?)
}

#[tauri::command]
pub fn update_debt(
    state: State<'_, AppState>,
    debt_id: String,
    input: DebtFormInput,
) -> Result<Debt, AppError> {
    let mut project = state.project()?;
    let (houses, debts) = project.houses_and_debts_mut();
    Ok(linking::update_debt(
        houses,
        debts,
        DebtId::parse(&debt_id)?,
        &input,
    )?)
}

#[tauri::command]
pub fn delete_debt(state: State<'_, AppState>, debt_id: String) -> Result<(), AppError> {
    let mut project = state.project()?;
    project.debts_mut().delete_debt(DebtId::parse(&debt_id)?)?;
    Ok(())
}

/// The open balance of a debt at the sale of the house it would be linked to,
/// for the settlement prompt (CSD-020 REQ-15, REQ-17).
#[tauri::command]
pub fn debt_open_balance(
    state: State<'_, AppState>,
    debt_id: String,
    house_id: String,
) -> Result<Option<OpenDebt>, AppError> {
    let project = state.project()?;
    Ok(linking::debt_open_balance(
        project.houses(),
        project.debts(),
        DebtId::parse(&debt_id)?,
        HouseId::parse(&house_id)?,
    )?)
}

/// The statements of all debts, newest period first (REQ-18).
#[tauri::command]
pub fn list_debt_statements(state: State<'_, AppState>) -> Result<Vec<StatementView>, AppError> {
    let project = state.project()?;
    let debts = project.debts();
    Ok(debts
        .all_statements()
        .into_iter()
        .map(|(debt, statement)| StatementView::new(debts, debt, statement))
        .collect())
}

#[tauri::command]
pub fn add_debt_statement(
    state: State<'_, AppState>,
    debt_id: String,
    period: String,
    balance: String,
    note: Option<String>,
) -> Result<StatementView, AppError> {
    let today = state.today();
    let debt_id = DebtId::parse(&debt_id)?;
    let period = parse_period(&period)?;
    let balance = DebtBalance::parse(&balance)?;
    let note = debt_note(note.as_deref().unwrap_or_default())?;
    let mut project = state.project()?;
    let statement = project
        .debts_mut()
        .add_statement(debt_id, period, balance, note, today)?;
    statement_view(&project, &statement)
}

#[tauri::command]
pub fn update_debt_statement(
    state: State<'_, AppState>,
    debt_id: String,
    period: String,
    new_period: String,
    balance: String,
    note: Option<String>,
) -> Result<StatementView, AppError> {
    let today = state.today();
    let debt_id = DebtId::parse(&debt_id)?;
    let period = parse_period(&period)?;
    let new_period = parse_period(&new_period)?;
    let balance = DebtBalance::parse(&balance)?;
    let note = debt_note(note.as_deref().unwrap_or_default())?;
    let mut project = state.project()?;
    let statement = project
        .debts_mut()
        .update_statement(debt_id, period, new_period, balance, note, today)?;
    statement_view(&project, &statement)
}

#[tauri::command]
pub fn delete_debt_statement(
    state: State<'_, AppState>,
    debt_id: String,
    period: String,
) -> Result<(), AppError> {
    let debt_id = DebtId::parse(&debt_id)?;
    let period = parse_period(&period)?;
    let mut project = state.project()?;
    project.debts_mut().delete_statement(debt_id, period)?;
    Ok(())
}

/// Form values for "copy previous month" or "copy previous year" (REQ-15).
#[tauri::command]
pub fn copy_previous_debt_statement(
    state: State<'_, AppState>,
    debt_id: String,
) -> Result<DebtStatementPrefill, AppError> {
    let project = state.project()?;
    Ok(project
        .debts()
        .prefill_from_latest(DebtId::parse(&debt_id)?)?)
}

/// Yearly totals over all debts, newest year first (REQ-19).
#[tauri::command]
pub fn yearly_debt_totals(state: State<'_, AppState>) -> Result<Vec<DebtYearlyTotal>, AppError> {
    let project = state.project()?;
    Ok(project.debts().yearly_totals())
}

/// The debts and the yearly series of the Insights chart, computed from the
/// project as it is now, saved or not (CSD-012 REQ-15).
#[tauri::command]
pub fn get_debt_insights(state: State<'_, AppState>) -> Result<DebtInsights, AppError> {
    let today = state.today();
    let project = state.project()?;
    Ok(project.debts().insights(today))
}

fn statement_view(project: &Project, statement: &DebtStatement) -> Result<StatementView, AppError> {
    let debts = project.debts();
    let debt = debts
        .debts()
        .into_iter()
        .find(|debt| debt.id == statement.debt_id)
        .ok_or(AppError::Internal)?;
    Ok(StatementView::new(debts, debt, statement))
}
