//! Persistence: the project file (CSD-002).

mod file;
mod project;

pub use project::{OpenOutcome, Project, ProjectStatus, SaveOutcome};

use std::io;

use thiserror::Error;

/// Failures while saving or opening a project file.
///
/// Messages never contain file paths or operating-system details; the cause is
/// kept as the error source only.
#[derive(Debug, Error)]
pub enum StorageError {
    #[error("The project could not be saved.")]
    SaveFailed(#[source] io::Error),
    #[error("The file could not be opened.")]
    OpenFailed(#[source] io::Error),
    #[error("This is not a valid project file.")]
    InvalidFile,
    #[error("The project has no file yet.")]
    NoPath,
}

impl StorageError {
    /// Stable machine-readable code for the frontend.
    #[must_use]
    pub fn code(&self) -> &'static str {
        match self {
            Self::SaveFailed(_) => "PROJECT_SAVE_FAILED",
            Self::OpenFailed(_) => "PROJECT_OPEN_FAILED",
            Self::InvalidFile => "PROJECT_INVALID_FILE",
            Self::NoPath => "PROJECT_NO_PATH",
        }
    }
}
