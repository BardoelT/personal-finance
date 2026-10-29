#![allow(
    clippy::needless_pass_by_value,
    reason = "Tauri commands receive their arguments by value"
)]

use tauri::State;

use super::AppState;
use crate::domain::expenses::ExpenseInput;
use crate::domain::{
    Expense, ExpenseId, ExpenseInsights, ExpenseList, ExpensePrefill, ExpenseYearlyTotal,
};
use crate::error::AppError;

/// All expenses, newest first, with their total (CSD-013 REQ-10, REQ-11).
#[tauri::command]
pub fn list_expenses(state: State<'_, AppState>) -> Result<ExpenseList, AppError> {
    let project = state.project()?;
    Ok(project.expenses().list())
}

#[tauri::command]
pub fn add_expense(state: State<'_, AppState>, input: ExpenseInput) -> Result<Expense, AppError> {
    let today = state.today_date();
    let mut project = state.project()?;
    Ok(project.expenses_mut().add_expense(&input, today)?)
}

/// Applies the edit as a whole or not at all (CSD-013 REQ-7).
#[tauri::command]
pub fn update_expense(
    state: State<'_, AppState>,
    expense_id: String,
    input: ExpenseInput,
) -> Result<Expense, AppError> {
    let today = state.today_date();
    let id = ExpenseId::parse(&expense_id)?;
    let mut project = state.project()?;
    Ok(project.expenses_mut().update_expense(id, &input, today)?)
}

#[tauri::command]
pub fn delete_expense(state: State<'_, AppState>, expense_id: String) -> Result<(), AppError> {
    let id = ExpenseId::parse(&expense_id)?;
    let mut project = state.project()?;
    project.expenses_mut().delete_expense(id)?;
    Ok(())
}

/// Form values for "Copy previous expense", or nothing (CSD-013 REQ-9).
#[tauri::command]
pub fn copy_previous_expense(
    state: State<'_, AppState>,
) -> Result<Option<ExpensePrefill>, AppError> {
    let project = state.project()?;
    Ok(project.expenses().prefill_from_previous())
}

/// Yearly totals over all expenses, newest year first (CSD-013 REQ-12).
#[tauri::command]
pub fn yearly_expense_totals(
    state: State<'_, AppState>,
) -> Result<Vec<ExpenseYearlyTotal>, AppError> {
    let project = state.project()?;
    Ok(project.expenses().yearly_totals())
}

/// The categories and the yearly series of the Insights chart, computed from
/// the project as it is now, saved or not (CSD-014 REQ-13).
#[tauri::command]
pub fn get_expense_insights(state: State<'_, AppState>) -> Result<ExpenseInsights, AppError> {
    let today = state.today();
    let project = state.project()?;
    Ok(project.expenses().insights(today.year()))
}
