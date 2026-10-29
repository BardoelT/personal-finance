//! Bank accounts and their monthly or yearly entries (CSD-009), and the yearly
//! totals and insights built on them (CSD-009 REQ-19, CSD-010).
//!
//! `BankAccounts` holds the data and the rules for changing it; the other files
//! add the value types (`amounts`, `account`, `entry`) and the figures derived
//! from it (`overview`, `totals`, `insights`). The period and note types are
//! those of the investments (CSD-005).

mod account;
mod amounts;
mod entry;
mod insights;
mod overview;
mod totals;

#[cfg(test)]
mod tests;

pub use account::{AccountInput, BankAccount, BankAccountId};
pub use amounts::{bank_note, BankBalance};
pub use entry::{parse_period, BankEntry, EntryPrefill};
pub use insights::BankInsights;
pub use overview::BankOverview;
pub use totals::BankYearlyTotal;

use std::collections::HashSet;

use serde::{Deserialize, Serialize};

use crate::domain::error::DomainError;
use crate::domain::income::YearMonth;
use crate::domain::investments::{Note, Period};

use account::{COLOUR_SLOTS, MAX_NAME_CHARS};

/// All bank accounts with their entries. Accounts are kept in the order they
/// were created (CSD-009 section 5).
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct BankAccounts {
    #[serde(default)]
    accounts: Vec<BankAccount>,
    #[serde(default)]
    entries: Vec<BankEntry>,
}

impl BankAccounts {
    /// Creates an account, monthly unless the input says Yearly, with the
    /// colour slot of REQ-3 (REQ-1, REQ-2).
    ///
    /// # Errors
    /// Returns the name errors of REQ-2 or `BankAccountPeriodUnsupported`.
    pub fn create_account(&mut self, input: &AccountInput) -> Result<BankAccount, DomainError> {
        let valid = input.validate()?;
        if self.name_taken(&valid.name, None) {
            return Err(DomainError::BankAccountNameDuplicate);
        }
        let account = BankAccount {
            id: BankAccountId::generate(),
            name: valid.name,
            entry_period: valid.entry_period.unwrap_or_default(),
            colour_slot: self.next_colour_slot(),
        };
        self.accounts.push(account.clone());
        Ok(account)
    }

    /// Changes the name and entry period of an account. The entries and the
    /// colour slot stay; the period is locked once the account has any entry
    /// (REQ-4, REQ-5).
    ///
    /// # Errors
    /// Returns a validation error, `BankAccountNotFound`,
    /// `BankAccountNameDuplicate` or `BankAccountPeriodLocked`.
    pub fn update_account(
        &mut self,
        id: BankAccountId,
        input: &AccountInput,
    ) -> Result<BankAccount, DomainError> {
        let valid = input.validate()?;
        let current = self.account(id)?.entry_period;
        if self.name_taken(&valid.name, Some(id)) {
            return Err(DomainError::BankAccountNameDuplicate);
        }
        let entry_period = valid.entry_period.unwrap_or(current);
        if entry_period != current && self.entry_count(id) > 0 {
            return Err(DomainError::BankAccountPeriodLocked);
        }
        let account = self.account_mut(id)?;
        account.name = valid.name;
        account.entry_period = entry_period;
        Ok(account.clone())
    }

    /// Removes an account and all its entries (REQ-6).
    ///
    /// # Errors
    /// Returns `BankAccountNotFound`.
    pub fn delete_account(&mut self, id: BankAccountId) -> Result<(), DomainError> {
        self.account(id)?;
        self.accounts.retain(|account| account.id != id);
        self.entries.retain(|entry| entry.account_id != id);
        Ok(())
    }

    /// Accounts alphabetically by name, ignoring case (REQ-18).
    #[must_use]
    pub fn accounts(&self) -> Vec<&BankAccount> {
        let mut accounts: Vec<&BankAccount> = self.accounts.iter().collect();
        accounts.sort_by_cached_key(|account| account_order(account));
        accounts
    }

    /// How many entries an account has.
    #[must_use]
    pub fn entry_count(&self, id: BankAccountId) -> usize {
        self.entries
            .iter()
            .filter(|entry| entry.account_id == id)
            .count()
    }

    /// How many entries all accounts have together.
    #[must_use]
    pub fn total_entry_count(&self) -> usize {
        self.entries.len()
    }

    /// Adds the entry of a period. `today` is the current month; later periods
    /// are rejected. Any earlier period is accepted (REQ-7 to REQ-9).
    ///
    /// # Errors
    /// Returns `BankAccountNotFound`, `BankEntryPeriodMismatch`,
    /// `BankEntryInFuture` or `BankEntryDuplicate`.
    pub fn add_entry(
        &mut self,
        account_id: BankAccountId,
        period: Period,
        balance: BankBalance,
        note: Note,
        today: YearMonth,
    ) -> Result<BankEntry, DomainError> {
        self.check_new_period(account_id, period, today)?;
        let entry = BankEntry {
            account_id,
            period,
            balance,
            note,
        };
        self.entries.push(entry.clone());
        Ok(entry)
    }

    /// Changes the balance and note of an entry and, when `new_period` is a
    /// different period, moves it there. The new period follows the rules of
    /// adding an entry: the kind of the account, not after `today` and not
    /// taken by another entry of the account. The edit is applied as a whole or
    /// not at all (REQ-12, REQ-13).
    ///
    /// # Errors
    /// Returns `BankAccountNotFound`, `BankEntryNotFound`,
    /// `BankEntryPeriodMismatch`, `BankEntryInFuture` or `BankEntryDuplicate`.
    pub fn update_entry(
        &mut self,
        account_id: BankAccountId,
        period: Period,
        new_period: Period,
        balance: BankBalance,
        note: Note,
        today: YearMonth,
    ) -> Result<BankEntry, DomainError> {
        self.account(account_id)?;
        if self.entry(account_id, period).is_none() {
            return Err(DomainError::BankEntryNotFound);
        }
        if new_period != period {
            self.check_new_period(account_id, new_period, today)?;
        }
        let entry = self
            .entries
            .iter_mut()
            .find(|e| e.account_id == account_id && e.period == period)
            .ok_or(DomainError::BankEntryNotFound)?;
        entry.period = new_period;
        entry.balance = balance;
        entry.note = note;
        Ok(entry.clone())
    }

    /// Removes one entry and nothing else (REQ-14). The caller asks for the
    /// confirmation.
    ///
    /// # Errors
    /// Returns `BankAccountNotFound` or `BankEntryNotFound`.
    pub fn delete_entry(
        &mut self,
        account_id: BankAccountId,
        period: Period,
    ) -> Result<(), DomainError> {
        self.account(account_id)?;
        let before = self.entries.len();
        self.entries
            .retain(|e| !(e.account_id == account_id && e.period == period));
        if self.entries.len() == before {
            return Err(DomainError::BankEntryNotFound);
        }
        Ok(())
    }

    /// The entries of all accounts, newest period first. A yearly entry counts
    /// as December of its year; within a period the order is that of the
    /// accounts (REQ-18).
    #[must_use]
    pub fn all_entries(&self) -> Vec<(&BankAccount, &BankEntry)> {
        let mut rows: Vec<(&BankAccount, &BankEntry)> = self
            .entries
            .iter()
            .filter_map(|entry| {
                self.accounts
                    .iter()
                    .find(|account| account.id == entry.account_id)
                    .map(|account| (account, entry))
            })
            .collect();
        rows.sort_by_cached_key(|(account, entry)| {
            (
                std::cmp::Reverse(entry.period.sort_key()),
                account_order(account),
            )
        });
        rows
    }

    /// Checks that data read from a file follows the rules of CSD-009, so a
    /// hand-edited or damaged file cannot break them.
    ///
    /// # Errors
    /// Returns `BankDataInvalid` for the first inconsistency found.
    pub fn validate(&self) -> Result<(), DomainError> {
        let invalid = DomainError::BankDataInvalid;
        let mut ids = HashSet::new();
        let mut names = HashSet::new();
        for account in &self.accounts {
            let name = account.name.trim();
            if name.is_empty()
                || name.chars().count() > MAX_NAME_CHARS
                || account.colour_slot >= COLOUR_SLOTS
                || !ids.insert(account.id)
                || !names.insert(name.to_lowercase())
            {
                return Err(invalid);
            }
        }
        let mut seen = HashSet::new();
        for entry in &self.entries {
            let matches_account = self
                .account(entry.account_id)
                .is_ok_and(|account| account.entry_period == entry.period.kind());
            if !matches_account
                || !seen.insert((entry.account_id, entry.period))
                || BankBalance::new(entry.balance.value()).is_err()
                || bank_note(entry.note.as_str()).is_err()
            {
                return Err(invalid);
            }
        }
        Ok(())
    }

    fn account(&self, id: BankAccountId) -> Result<&BankAccount, DomainError> {
        self.accounts
            .iter()
            .find(|account| account.id == id)
            .ok_or(DomainError::BankAccountNotFound)
    }

    fn account_mut(&mut self, id: BankAccountId) -> Result<&mut BankAccount, DomainError> {
        self.accounts
            .iter_mut()
            .find(|account| account.id == id)
            .ok_or(DomainError::BankAccountNotFound)
    }

    fn entry(&self, account_id: BankAccountId, period: Period) -> Option<&BankEntry> {
        self.entries
            .iter()
            .find(|e| e.account_id == account_id && e.period == period)
    }

    /// The rules a period must follow for an account to hold an entry for it:
    /// the kind of the account, not after `today`, not taken (REQ-7 to REQ-9).
    fn check_new_period(
        &self,
        account_id: BankAccountId,
        period: Period,
        today: YearMonth,
    ) -> Result<(), DomainError> {
        if self.account(account_id)?.entry_period != period.kind() {
            return Err(DomainError::BankEntryPeriodMismatch);
        }
        if period.is_after(today) {
            return Err(DomainError::BankEntryInFuture);
        }
        if self.entry(account_id, period).is_some() {
            return Err(DomainError::BankEntryDuplicate);
        }
        Ok(())
    }

    /// The entry with the most recent period of an account.
    fn latest(&self, id: BankAccountId) -> Option<&BankEntry> {
        self.entries
            .iter()
            .filter(|entry| entry.account_id == id)
            .max_by_key(|entry| entry.period.sort_key())
    }

    /// The lowest colour slot used by the fewest accounts (REQ-3).
    fn next_colour_slot(&self) -> u8 {
        (0..COLOUR_SLOTS)
            .min_by_key(|slot| {
                (
                    self.accounts
                        .iter()
                        .filter(|account| account.colour_slot == *slot)
                        .count(),
                    *slot,
                )
            })
            .unwrap_or_default()
    }

    /// Whether another account already has this name, ignoring case (REQ-2).
    fn name_taken(&self, name: &str, except: Option<BankAccountId>) -> bool {
        let wanted = name.to_lowercase();
        self.accounts
            .iter()
            .any(|account| Some(account.id) != except && account.name.to_lowercase() == wanted)
    }
}

/// Alphabetical order ignoring case, with the id as a stable tie-break.
fn account_order(account: &BankAccount) -> (String, String) {
    (account.name.to_lowercase(), account.id.to_string())
}
