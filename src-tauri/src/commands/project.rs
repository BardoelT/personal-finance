#![allow(
    clippy::needless_pass_by_value,
    reason = "Tauri commands receive their arguments by value"
)]

use std::path::PathBuf;

use serde::Serialize;
use tauri::{AppHandle, Runtime, State};
use tauri_plugin_dialog::DialogExt;

use super::AppState;
use crate::error::AppError;
use crate::storage::{OpenOutcome, ProjectStatus, SaveOutcome};

/// Result of a command that may have been cancelled in a native dialog.
#[derive(Serialize)]
pub struct DialogResult {
    /// `false` when the user cancelled the dialog and nothing happened.
    done: bool,
    status: ProjectStatus,
}

const FILTER_NAME: &str = "Wealth project";
const FILTER_EXTENSIONS: &[&str] = &["wealth"];

/// Shows the native "save" dialog. Returns `None` when cancelled.
fn pick_save_path<R: Runtime>(app: &AppHandle<R>) -> Option<PathBuf> {
    app.dialog()
        .file()
        .add_filter(FILTER_NAME, FILTER_EXTENSIONS)
        .set_file_name("project.wealth")
        .blocking_save_file()
        .and_then(|path| path.into_path().ok())
}

/// Shows the native "open" dialog. Returns `None` when cancelled.
fn pick_open_path<R: Runtime>(app: &AppHandle<R>) -> Option<PathBuf> {
    app.dialog()
        .file()
        .add_filter(FILTER_NAME, FILTER_EXTENSIONS)
        .blocking_pick_file()
        .and_then(|path| path.into_path().ok())
}

fn status_of(state: &AppState) -> Result<ProjectStatus, AppError> {
    Ok(state.project()?.status())
}

/// Saves to the current path, or after a native dialog when there is none yet.
///
/// The project lock is released while the dialog is open, so closing the
/// window never waits on it.
fn save_flow(
    state: &AppState,
    ask_path: impl FnOnce() -> Option<PathBuf>,
) -> Result<DialogResult, AppError> {
    let has_path = state.project()?.status().has_path;
    let path = if has_path { None } else { ask_path() };
    let mut project = state.project()?;
    let outcome = if has_path {
        project.save()?;
        SaveOutcome::Saved
    } else {
        project.save_asking(|| path)?
    };
    Ok(DialogResult {
        done: outcome == SaveOutcome::Saved,
        status: project.status(),
    })
}

/// Save As: always asks for a path.
fn save_as_flow(
    state: &AppState,
    ask_path: impl FnOnce() -> Option<PathBuf>,
) -> Result<DialogResult, AppError> {
    let path = ask_path();
    let mut project = state.project()?;
    let outcome = project.save_as_asking(|| path)?;
    Ok(DialogResult {
        done: outcome == SaveOutcome::Saved,
        status: project.status(),
    })
}

fn open_flow(
    state: &AppState,
    ask_path: impl FnOnce() -> Option<PathBuf>,
) -> Result<DialogResult, AppError> {
    let path = ask_path();
    let mut project = state.project()?;
    let outcome = project.open_asking(|| path)?;
    Ok(DialogResult {
        done: outcome == OpenOutcome::Opened,
        status: project.status(),
    })
}

#[tauri::command]
pub fn project_status(state: State<'_, AppState>) -> Result<ProjectStatus, AppError> {
    status_of(&state)
}

/// The reason the last project was not reopened at start, once (CSD-002 REQ-11).
#[tauri::command]
pub fn take_startup_notice(state: State<'_, AppState>) -> Result<Option<AppError>, AppError> {
    Ok(state.project()?.take_startup_notice().map(AppError::from))
}

#[tauri::command]
pub fn new_project(state: State<'_, AppState>) -> Result<ProjectStatus, AppError> {
    let mut project = state.project()?;
    project.reset();
    Ok(project.status())
}

#[tauri::command(async)]
pub fn open_project<R: Runtime>(
    app: AppHandle<R>,
    state: State<'_, AppState>,
) -> Result<DialogResult, AppError> {
    open_flow(&state, || pick_open_path(&app))
}

#[tauri::command(async)]
pub fn save_project<R: Runtime>(
    app: AppHandle<R>,
    state: State<'_, AppState>,
) -> Result<DialogResult, AppError> {
    save_flow(&state, || pick_save_path(&app))
}

#[tauri::command(async)]
pub fn save_project_as<R: Runtime>(
    app: AppHandle<R>,
    state: State<'_, AppState>,
) -> Result<DialogResult, AppError> {
    save_as_flow(&state, || pick_save_path(&app))
}

/// Quits without asking: the frontend has already confirmed (CSD-002 REQ-4).
#[tauri::command]
pub fn quit_app<R: Runtime>(app: AppHandle<R>) {
    app.exit(0);
}

#[cfg(test)]
mod tests {
    use tempfile::TempDir;
    use time::UtcOffset;

    use super::*;
    use crate::domain::JobInput;

    fn state_with_job() -> AppState {
        let state = AppState::new(UtcOffset::UTC);
        state
            .project()
            .unwrap()
            .income_mut()
            .create_job(&JobInput {
                employer: "Example Employer".into(),
                start_date: Some("2024-03-01".into()),
                ..JobInput::default()
            })
            .unwrap();
        state
    }

    #[test]
    fn test_csd002_ac_1_3_flow_asks_once_and_reuses_the_path() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("example-a.wealth");
        let state = state_with_job();
        let mut asked = 0;

        let first = save_flow(&state, || {
            asked += 1;
            Some(path.clone())
        })
        .unwrap();
        assert!(first.done);
        assert!(!first.status.unsaved_changes);
        assert!(path.exists());

        let second = save_flow(&state, || {
            asked += 1;
            None
        })
        .unwrap();
        assert!(second.done);
        assert_eq!(asked, 1);
    }

    #[test]
    fn test_csd002_ac_1_5_flow_reports_a_cancelled_dialog() {
        let dir = TempDir::new().unwrap();
        let state = state_with_job();

        let saved = save_flow(&state, || None).unwrap();
        assert!(!saved.done);
        assert!(saved.status.unsaved_changes);
        let saved_as = save_as_flow(&state, || None).unwrap();
        assert!(!saved_as.done);
        let opened = open_flow(&state, || None).unwrap();
        assert!(!opened.done);
        assert!(opened.status.unsaved_changes);
        assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), 0);
    }

    #[test]
    fn test_csd002_ac_1_4_flow_save_as_moves_the_project() {
        let dir = TempDir::new().unwrap();
        let state = state_with_job();
        let path_a = dir.path().join("example-a.wealth");
        let path_b = dir.path().join("example-b.wealth");
        save_flow(&state, || Some(path_a.clone())).unwrap();
        save_as_flow(&state, || Some(path_b.clone())).unwrap();
        assert!(path_b.exists());
        let status = status_of(&state).unwrap();
        assert!(status.has_path && !status.unsaved_changes);
    }

    #[test]
    fn open_flow_replaces_the_project() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("example-a.wealth");
        let source = state_with_job();
        save_flow(&source, || Some(path.clone())).unwrap();

        let target = AppState::new(UtcOffset::UTC);
        let result = open_flow(&target, || Some(path.clone())).unwrap();
        assert!(result.done);
        assert_eq!(target.project().unwrap().income().jobs().len(), 1);
    }
}
