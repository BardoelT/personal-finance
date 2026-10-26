#![allow(
    clippy::needless_pass_by_value,
    reason = "Tauri commands receive their arguments by value"
)]

use serde::Serialize;
use tauri::State;

use super::AppState;
use crate::domain::IncomeOverview;
use crate::error::AppError;

/// What the dashboard shows (CSD-004): whether there are jobs at all, and the
/// two blocks of figures.
#[derive(Serialize)]
pub struct DashboardView {
    has_jobs: bool,
    #[serde(flatten)]
    overview: IncomeOverview,
}

/// Income this year to date and last month, over all jobs, from the project as
/// it is now, saved or not (CSD-004 REQ-7).
#[tauri::command]
pub fn get_dashboard(state: State<'_, AppState>) -> Result<DashboardView, AppError> {
    let today = state.today();
    let project = state.project()?;
    Ok(DashboardView {
        has_jobs: project.income().has_jobs(),
        overview: project.income().overview(today),
    })
}
