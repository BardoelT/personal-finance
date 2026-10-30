#![allow(
    clippy::needless_pass_by_value,
    reason = "Tauri commands receive their arguments by value"
)]

use tauri::State;

use super::AppState;
use crate::domain::net_worth::net_worth_insights;
use crate::domain::NetWorthInsights;
use crate::error::AppError;

/// The yearly net worth: houses, investments, cash and debts, computed from
/// the project as it is now, saved or not (CSD-017 REQ-13).
#[tauri::command]
pub fn get_net_worth_insights(state: State<'_, AppState>) -> Result<NetWorthInsights, AppError> {
    let today = state.today();
    let project = state.project()?;
    Ok(net_worth_insights(
        project.houses(),
        project.investments(),
        project.bank_accounts(),
        project.debts(),
        today,
    ))
}
