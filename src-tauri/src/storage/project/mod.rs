use std::path::{Path, PathBuf};

use serde::Serialize;

use super::{file, LastProject, StorageError};
use crate::domain::{
    BankAccounts, Debts, Expenses, Houses, Income, Investments, ProjectData, Settings,
};

/// The extension of project files.
const EXTENSION: &str = "wealth";

/// Whether the project differs from disk, and whether it has a file yet.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub struct ProjectStatus {
    pub unsaved_changes: bool,
    pub has_path: bool,
}

/// Result of a save that may ask for a path first.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SaveOutcome {
    Saved,
    /// The user cancelled the path dialog; nothing was written.
    Cancelled,
}

/// Result of an open that asks for a file first.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OpenOutcome {
    Opened,
    /// The user cancelled the file dialog; nothing changed.
    Cancelled,
}

/// The project in memory: its data, the file it belongs to and the state last
/// written to or read from that file (CSD-002).
#[derive(Debug, Default)]
pub struct Project {
    data: ProjectData,
    saved: ProjectData,
    path: Option<PathBuf>,
    /// Where the last used path is remembered, if anywhere (REQ-10).
    memory: Option<LastProject>,
    /// Why the remembered file could not be reopened at start (REQ-11).
    startup_notice: Option<StorageError>,
}

impl Project {
    /// An empty project with no path (REQ-6, REQ-8).
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// The income data.
    #[must_use]
    pub fn income(&self) -> &Income {
        &self.data.income
    }

    /// Mutable income data. Edits show up in [`Project::status`] by comparison
    /// with the last saved state, so no separate flag can go stale (REQ-3).
    pub fn income_mut(&mut self) -> &mut Income {
        &mut self.data.income
    }

    /// The investment accounts and statements (CSD-005).
    #[must_use]
    pub fn investments(&self) -> &Investments {
        &self.data.investments
    }

    /// Mutable investments. Like the income data, changes show up in
    /// [`Project::status`] by comparison with the last saved state.
    pub fn investments_mut(&mut self) -> &mut Investments {
        &mut self.data.investments
    }

    /// The bank accounts and their entries (CSD-009).
    #[must_use]
    pub fn bank_accounts(&self) -> &BankAccounts {
        &self.data.bank_accounts
    }

    /// Mutable bank accounts. Like the income data, changes show up in
    /// [`Project::status`] by comparison with the last saved state.
    pub fn bank_accounts_mut(&mut self) -> &mut BankAccounts {
        &mut self.data.bank_accounts
    }

    /// The debts and their statements (CSD-011).
    #[must_use]
    pub fn debts(&self) -> &Debts {
        &self.data.debts
    }

    /// Mutable debts. Like the income data, changes show up in
    /// [`Project::status`] by comparison with the last saved state.
    pub fn debts_mut(&mut self) -> &mut Debts {
        &mut self.data.debts
    }

    /// The houses and their valuations (CSD-015).
    #[must_use]
    pub fn houses(&self) -> &Houses {
        &self.data.houses
    }

    /// Mutable houses. Like the income data, changes show up in
    /// [`Project::status`] by comparison with the last saved state.
    pub fn houses_mut(&mut self) -> &mut Houses {
        &mut self.data.houses
    }

    /// The houses and the debts together, for the changes that touch both
    /// (CSD-020).
    pub fn houses_and_debts_mut(&mut self) -> (&mut Houses, &mut Debts) {
        (&mut self.data.houses, &mut self.data.debts)
    }

    /// The expenses (CSD-013).
    #[must_use]
    pub fn expenses(&self) -> &Expenses {
        &self.data.expenses
    }

    /// Mutable expenses. Like the income data, changes show up in
    /// [`Project::status`] by comparison with the last saved state.
    pub fn expenses_mut(&mut self) -> &mut Expenses {
        &mut self.data.expenses
    }

    /// The project settings (CSD-003).
    #[must_use]
    pub fn settings(&self) -> &Settings {
        &self.data.settings
    }

    /// Mutable project settings. Like the income data, changes show up in
    /// [`Project::status`] by comparison (CSD-003 AC-4.2).
    pub fn settings_mut(&mut self) -> &mut Settings {
        &mut self.data.settings
    }

    /// Unsaved-changes indicator and whether a path is set.
    #[must_use]
    pub fn status(&self) -> ProjectStatus {
        ProjectStatus {
            unsaved_changes: self.data != self.saved,
            has_path: self.path.is_some(),
        }
    }

    /// Replaces the project with an empty one that has no path (REQ-6).
    ///
    /// The remembered path is left alone: starting a new project is not using
    /// another file (REQ-10).
    pub fn reset(&mut self) {
        let memory = self.memory.take();
        *self = Self::new();
        self.memory = memory;
    }

    /// Starts using `memory` and reopens the file it remembers, if any (REQ-8).
    ///
    /// If that file cannot be opened the project stays empty, the path is
    /// forgotten so the error does not repeat, and the reason is kept for
    /// [`Project::take_startup_notice`] (REQ-11).
    pub fn restore(&mut self, memory: LastProject) {
        let remembered = memory.load();
        self.memory = Some(memory);
        let Some(path) = remembered else {
            return;
        };
        if let Err(error) = self.open(&path) {
            if let Some(memory) = &self.memory {
                memory.clear();
            }
            self.startup_notice = Some(error);
        }
    }

    /// The reason the last project was not reopened, once.
    pub fn take_startup_notice(&mut self) -> Option<StorageError> {
        self.startup_notice.take()
    }

    /// Remembers the current path. Best effort: a project that was saved or
    /// opened fine must not fail because the settings could not be written.
    fn remember(&self) {
        if let (Some(memory), Some(path)) = (&self.memory, &self.path) {
            let _ = memory.store(path);
        }
    }

    /// Writes to the current path.
    ///
    /// # Errors
    /// Returns `NoPath` when the project has no file yet, or `SaveFailed`; in
    /// both cases the unsaved indicator stays and any existing file is intact.
    pub fn save(&mut self) -> Result<(), StorageError> {
        let path = self.path.clone().ok_or(StorageError::NoPath)?;
        self.save_as(path)
    }

    /// Writes to `path`, which becomes the project's path (REQ-1).
    ///
    /// # Errors
    /// Returns `SaveFailed`; the project keeps its previous path and stays unsaved.
    pub fn save_as(&mut self, path: PathBuf) -> Result<(), StorageError> {
        let path = with_extension(path);
        file::write(&path, &self.data)?;
        self.saved = self.data.clone();
        self.path = Some(path);
        self.remember();
        Ok(())
    }

    /// Saves to the current path, or asks for one if there is none yet. A
    /// `None` from `ask_path` means the user cancelled (AC-1.3, AC-1.5).
    ///
    /// # Errors
    /// Returns `SaveFailed`.
    pub fn save_asking(
        &mut self,
        ask_path: impl FnOnce() -> Option<PathBuf>,
    ) -> Result<SaveOutcome, StorageError> {
        if self.path.is_some() {
            self.save()?;
        } else {
            let Some(path) = ask_path() else {
                return Ok(SaveOutcome::Cancelled);
            };
            self.save_as(path)?;
        }
        Ok(SaveOutcome::Saved)
    }

    /// Always asks for a path, even if there is one (Save As, AC-1.4).
    ///
    /// # Errors
    /// Returns `SaveFailed`.
    pub fn save_as_asking(
        &mut self,
        ask_path: impl FnOnce() -> Option<PathBuf>,
    ) -> Result<SaveOutcome, StorageError> {
        let Some(path) = ask_path() else {
            return Ok(SaveOutcome::Cancelled);
        };
        self.save_as(path)?;
        Ok(SaveOutcome::Saved)
    }

    /// Replaces the project with the contents of `path` (REQ-2).
    ///
    /// # Errors
    /// Returns `OpenFailed` or `InvalidFile`; the project is left unchanged.
    pub fn open(&mut self, path: &Path) -> Result<(), StorageError> {
        let data = file::read(path)?;
        self.saved = data.clone();
        self.data = data;
        self.path = Some(path.to_path_buf());
        self.remember();
        Ok(())
    }

    /// Opens the file `ask_path` returns; `None` means the user cancelled.
    ///
    /// # Errors
    /// Returns `OpenFailed` or `InvalidFile`; the project is left unchanged.
    pub fn open_asking(
        &mut self,
        ask_path: impl FnOnce() -> Option<PathBuf>,
    ) -> Result<OpenOutcome, StorageError> {
        let Some(path) = ask_path() else {
            return Ok(OpenOutcome::Cancelled);
        };
        self.open(&path)?;
        Ok(OpenOutcome::Opened)
    }
}

fn with_extension(path: PathBuf) -> PathBuf {
    if path.extension().is_some() {
        path
    } else {
        path.with_extension(EXTENSION)
    }
}

#[cfg(test)]
mod bank_accounts_tests;
#[cfg(test)]
mod debts_tests;
#[cfg(test)]
mod expenses_tests;
#[cfg(test)]
mod houses_tests;
#[cfg(test)]
mod investments_tests;
#[cfg(test)]
mod tests;
