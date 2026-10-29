//! `#[tauri::command]` handlers: validate input, call the domain, map errors.

pub mod bank_accounts;
pub mod dashboard;
pub mod debts;
pub mod expenses;
pub mod income;
pub mod investments;
pub mod preferences;
pub mod project;
pub mod settings;

use std::sync::{Mutex, MutexGuard};

use time::{Date, OffsetDateTime, UtcOffset};

use crate::domain::YearMonth;
use crate::error::AppError;
use crate::storage::Project;

/// Shared app state: the open project and the local UTC offset, read once at
/// startup while the process is still single-threaded.
pub struct AppState {
    project: Mutex<Project>,
    utc_offset: UtcOffset,
}

impl AppState {
    /// A fresh empty project (CSD-002 REQ-8).
    #[must_use]
    pub fn new(utc_offset: UtcOffset) -> Self {
        Self {
            project: Mutex::new(Project::new()),
            utc_offset,
        }
    }

    /// Locks the project. Keep the guard short-lived: never hold it across a
    /// native dialog, or closing the window could deadlock.
    ///
    /// # Errors
    /// Returns `Internal` if a previous holder panicked.
    pub fn project(&self) -> Result<MutexGuard<'_, Project>, AppError> {
        self.project.lock().map_err(|_| AppError::Internal)
    }

    /// The current calendar month in local time (CSD-001 REQ-5).
    #[must_use]
    pub fn today(&self) -> YearMonth {
        YearMonth::from_date(self.today_date())
    }

    /// The current calendar date in local time (CSD-013 CON-11).
    #[must_use]
    pub fn today_date(&self) -> Date {
        OffsetDateTime::now_utc().to_offset(self.utc_offset).date()
    }
}
