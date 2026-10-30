#![allow(
    clippy::needless_pass_by_value,
    reason = "Tauri commands receive their arguments by value"
)]

use tauri::State;

use super::AppState;
use crate::domain::ratios::{ratios, RatioSources};
use crate::domain::Ratios;
use crate::error::AppError;

/// The yearly ratios between income, wealth, growth and expenses, computed from
/// the project as it is now, saved or not (CSD-018 REQ-12).
#[tauri::command]
pub fn get_ratios(state: State<'_, AppState>) -> Result<Ratios, AppError> {
    let today = state.today();
    let project = state.project()?;
    Ok(ratios(
        &RatioSources {
            income: project.income(),
            expenses: project.expenses(),
            assets: project.assets(),
            investments: project.investments(),
            bank_accounts: project.bank_accounts(),
            debts: project.debts(),
        },
        today,
    ))
}
