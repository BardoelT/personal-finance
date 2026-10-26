#![allow(
    clippy::needless_pass_by_value,
    reason = "Tauri commands receive their arguments by value"
)]

use serde::Serialize;
use tauri::State;

use super::AppState;
use crate::domain::Currency;
use crate::error::AppError;
use crate::storage::Project;

/// The project settings as the Settings page needs them (CSD-003).
#[derive(Serialize)]
pub struct SettingsView {
    currency: Currency,
    /// How many entries (paychecks, annual statements and investment statements)
    /// exist, so the frontend knows whether changing the
    /// currency needs a confirmation (REQ-5).
    entry_count: usize,
}

impl SettingsView {
    fn of(project: &Project) -> Self {
        Self {
            currency: project.settings().currency,
            entry_count: project.income().all_entries().len()
                + project.investments().total_statement_count(),
        }
    }
}

#[tauri::command]
pub fn get_settings(state: State<'_, AppState>) -> Result<SettingsView, AppError> {
    Ok(SettingsView::of(&*state.project()?))
}

/// Changes the project currency without converting any amount (CSD-003 REQ-4).
#[tauri::command]
pub fn set_currency(
    state: State<'_, AppState>,
    currency: String,
) -> Result<SettingsView, AppError> {
    let mut project = state.project()?;
    project.settings_mut().set_currency(&currency)?;
    Ok(SettingsView::of(&project))
}
