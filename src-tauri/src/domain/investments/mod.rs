//! Investment accounts and their monthly or yearly statements (CSD-005), and
//! the yearly totals and insights built on them (CSD-005 REQ-26, CSD-007).
//!
//! `Investments` holds the data and the rules for changing it; the other files
//! add the value types (`period`, `amounts`, `account`, `statement`) and the
//! figures derived from it (`overview`, `growth`, `totals`, `insights`).

mod account;
mod amounts;
mod growth;
mod insights;
mod overview;
mod period;
mod statement;
mod totals;

#[cfg(test)]
mod tests;

pub use account::{AccountInput, InvestmentAccount, InvestmentAccountId};
pub(crate) use amounts::MAX_BALANCE;
pub use amounts::{Balance, NetDeposits, Note};
pub use growth::{CalculationInput, StatementCalculation};
pub use insights::InvestmentInsightsYear;
pub use overview::InvestmentsOverview;
pub use period::Period;
pub use statement::{InvestmentStatement, StatementPrefill};
pub use totals::InvestmentYearlyTotal;

use std::collections::HashSet;

use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

use crate::domain::error::DomainError;
use crate::domain::income::YearMonth;

use crate::domain::series::{self, MAX_NAME_CHARS};

/// All investment accounts with their statements.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Investments {
    #[serde(default)]
    accounts: Vec<InvestmentAccount>,
    #[serde(default)]
    statements: Vec<InvestmentStatement>,
}

impl Investments {
    /// Creates an account, monthly unless the input says Yearly (REQ-1).
    ///
    /// # Errors
    /// Returns the name errors of REQ-2 or `InvestmentPeriodUnsupported`.
    pub fn create_account(
        &mut self,
        input: &AccountInput,
    ) -> Result<InvestmentAccount, DomainError> {
        let valid = input.validate()?;
        if self.name_taken(&valid.name, None) {
            return Err(DomainError::InvestmentAccountNameDuplicate);
        }
        let account = InvestmentAccount {
            id: InvestmentAccountId::generate(),
            name: valid.name,
            statement_period: valid.statement_period.unwrap_or_default(),
        };
        self.accounts.push(account.clone());
        Ok(account)
    }

    /// Changes the name and statement period of an account. The statements
    /// stay; the period is locked once the account has any (REQ-3, REQ-4).
    ///
    /// # Errors
    /// Returns a validation error, `InvestmentAccountNotFound`,
    /// `InvestmentAccountNameDuplicate` or `InvestmentPeriodLocked`.
    pub fn update_account(
        &mut self,
        id: InvestmentAccountId,
        input: &AccountInput,
    ) -> Result<InvestmentAccount, DomainError> {
        let valid = input.validate()?;
        let current = self.account(id)?.statement_period;
        if self.name_taken(&valid.name, Some(id)) {
            return Err(DomainError::InvestmentAccountNameDuplicate);
        }
        let statement_period = valid.statement_period.unwrap_or(current);
        if statement_period != current && self.statement_count(id) > 0 {
            return Err(DomainError::InvestmentPeriodLocked);
        }
        let account = self.account_mut(id)?;
        account.name = valid.name;
        account.statement_period = statement_period;
        Ok(account.clone())
    }

    /// Removes an account and all its statements (REQ-5).
    ///
    /// # Errors
    /// Returns `InvestmentAccountNotFound`.
    pub fn delete_account(&mut self, id: InvestmentAccountId) -> Result<(), DomainError> {
        self.account(id)?;
        self.accounts.retain(|account| account.id != id);
        self.statements
            .retain(|statement| statement.account_id != id);
        Ok(())
    }

    /// Accounts alphabetically by name, ignoring case (REQ-16).
    #[must_use]
    pub fn accounts(&self) -> Vec<&InvestmentAccount> {
        let mut accounts: Vec<&InvestmentAccount> = self.accounts.iter().collect();
        accounts.sort_by_cached_key(|account| account_order(account));
        accounts
    }

    /// How many statements an account has.
    #[must_use]
    pub fn statement_count(&self, id: InvestmentAccountId) -> usize {
        self.statements
            .iter()
            .filter(|statement| statement.account_id == id)
            .count()
    }

    /// How many statements all accounts have together.
    #[must_use]
    pub fn total_statement_count(&self) -> usize {
        self.statements.len()
    }

    /// Adds the statement of a period. `today` is the current month; later
    /// periods are rejected. Any earlier period is accepted (REQ-6).
    ///
    /// # Errors
    /// Returns `InvestmentAccountNotFound`, `InvestmentPeriodMismatch`,
    /// `InvestmentStatementInFuture` or `InvestmentStatementDuplicate`.
    pub fn add_statement(
        &mut self,
        account_id: InvestmentAccountId,
        period: Period,
        balance: Balance,
        net_deposits: NetDeposits,
        note: Note,
        today: YearMonth,
    ) -> Result<InvestmentStatement, DomainError> {
        if self.account(account_id)?.statement_period != period.kind() {
            return Err(DomainError::InvestmentPeriodMismatch);
        }
        if period.is_after(today) {
            return Err(DomainError::InvestmentStatementInFuture);
        }
        if self.statement(account_id, period).is_some() {
            return Err(DomainError::InvestmentStatementDuplicate);
        }
        let statement = InvestmentStatement {
            account_id,
            period,
            balance,
            net_deposits,
            note,
        };
        self.statements.push(statement.clone());
        Ok(statement)
    }

    /// Changes the balance, net deposits and note of a statement and, when
    /// `new_period` is a different period, moves it there. The new period
    /// follows the rules of adding a statement: the kind of the account, not
    /// after `today` and not taken by another statement of the account. The
    /// edit is applied as a whole or not at all (REQ-11, REQ-28, REQ-29).
    ///
    /// # Errors
    /// Returns `InvestmentAccountNotFound`, `InvestmentStatementNotFound`,
    /// `InvestmentPeriodMismatch`, `InvestmentStatementInFuture` or
    /// `InvestmentStatementDuplicate`.
    #[allow(
        clippy::too_many_arguments,
        reason = "add_statement's values, plus the statement's current period and `today`"
    )]
    pub fn update_statement(
        &mut self,
        account_id: InvestmentAccountId,
        period: Period,
        new_period: Period,
        balance: Balance,
        net_deposits: NetDeposits,
        note: Note,
        today: YearMonth,
    ) -> Result<InvestmentStatement, DomainError> {
        let kind = self.account(account_id)?.statement_period;
        if self.statement(account_id, period).is_none() {
            return Err(DomainError::InvestmentStatementNotFound);
        }
        if new_period != period {
            if kind != new_period.kind() {
                return Err(DomainError::InvestmentPeriodMismatch);
            }
            if new_period.is_after(today) {
                return Err(DomainError::InvestmentStatementInFuture);
            }
            if self.statement(account_id, new_period).is_some() {
                return Err(DomainError::InvestmentStatementDuplicate);
            }
        }
        let statement = self
            .statements
            .iter_mut()
            .find(|s| s.account_id == account_id && s.period == period)
            .ok_or(DomainError::InvestmentStatementNotFound)?;
        statement.period = new_period;
        statement.balance = balance;
        statement.net_deposits = net_deposits;
        statement.note = note;
        Ok(statement.clone())
    }

    /// Removes one statement and nothing else (REQ-12). The caller asks for the
    /// confirmation.
    ///
    /// # Errors
    /// Returns `InvestmentAccountNotFound` or `InvestmentStatementNotFound`.
    pub fn delete_statement(
        &mut self,
        account_id: InvestmentAccountId,
        period: Period,
    ) -> Result<(), DomainError> {
        self.account(account_id)?;
        let before = self.statements.len();
        self.statements
            .retain(|s| !(s.account_id == account_id && s.period == period));
        if self.statements.len() == before {
            return Err(DomainError::InvestmentStatementNotFound);
        }
        Ok(())
    }

    /// The statements of all accounts, newest period first. A yearly statement
    /// counts as December of its year; within a period the order is that of the
    /// accounts (REQ-16).
    #[must_use]
    pub fn all_statements(&self) -> Vec<(&InvestmentAccount, &InvestmentStatement)> {
        let mut rows: Vec<(&InvestmentAccount, &InvestmentStatement)> = self
            .statements
            .iter()
            .filter_map(|statement| {
                self.accounts
                    .iter()
                    .find(|account| account.id == statement.account_id)
                    .map(|account| (account, statement))
            })
            .collect();
        rows.sort_by_cached_key(|(account, statement)| {
            (
                std::cmp::Reverse(statement.period.sort_key()),
                account_order(account),
            )
        });
        rows
    }

    /// Checks that data read from a file follows the rules of CSD-005, so a
    /// hand-edited or damaged file cannot break them.
    ///
    /// # Errors
    /// Returns `InvestmentDataInvalid` for the first inconsistency found.
    pub fn validate(&self) -> Result<(), DomainError> {
        let invalid = DomainError::InvestmentDataInvalid;
        let mut ids = HashSet::new();
        let mut names = HashSet::new();
        for account in &self.accounts {
            let name = account.name.trim();
            if name.is_empty()
                || name.chars().count() > MAX_NAME_CHARS
                || !ids.insert(account.id)
                || !names.insert(name.to_lowercase())
            {
                return Err(invalid);
            }
        }
        let mut seen = HashSet::new();
        for statement in &self.statements {
            let matches_account = self
                .account(statement.account_id)
                .is_ok_and(|account| account.statement_period == statement.period.kind());
            if !matches_account
                || !seen.insert((statement.account_id, statement.period))
                || Balance::new(statement.balance.value()).is_err()
                || NetDeposits::new(statement.net_deposits.value()).is_err()
                || Note::new(statement.note.as_str()).is_err()
            {
                return Err(invalid);
            }
        }
        Ok(())
    }

    fn account(&self, id: InvestmentAccountId) -> Result<&InvestmentAccount, DomainError> {
        self.accounts
            .iter()
            .find(|account| account.id == id)
            .ok_or(DomainError::InvestmentAccountNotFound)
    }

    fn account_mut(
        &mut self,
        id: InvestmentAccountId,
    ) -> Result<&mut InvestmentAccount, DomainError> {
        self.accounts
            .iter_mut()
            .find(|account| account.id == id)
            .ok_or(DomainError::InvestmentAccountNotFound)
    }

    fn statement(
        &self,
        account_id: InvestmentAccountId,
        period: Period,
    ) -> Option<&InvestmentStatement> {
        self.statements
            .iter()
            .find(|s| s.account_id == account_id && s.period == period)
    }

    /// The balance of the statement of the account with the latest period
    /// before `period`, also when periods in between have none. The statement
    /// of `ignore`, the one being edited, never counts (REQ-24).
    fn previous_balance(
        &self,
        id: InvestmentAccountId,
        period: Period,
        ignore: Option<Period>,
    ) -> Option<Decimal> {
        self.statements
            .iter()
            .filter(|s| {
                s.account_id == id
                    && s.period.sort_key() < period.sort_key()
                    && Some(s.period) != ignore
            })
            .max_by_key(|s| s.period.sort_key())
            .map(|s| s.balance.value())
    }

    /// The statement with the most recent period of an account.
    fn latest(&self, id: InvestmentAccountId) -> Option<&InvestmentStatement> {
        self.statements
            .iter()
            .filter(|statement| statement.account_id == id)
            .max_by_key(|statement| statement.period.sort_key())
    }

    /// Whether another account already has this name, ignoring case (REQ-2).
    fn name_taken(&self, name: &str, except: Option<InvestmentAccountId>) -> bool {
        series::name_taken(
            self.accounts
                .iter()
                .map(|account| (account.id, account.name.as_str())),
            name,
            except,
        )
    }
}

/// Alphabetical order ignoring case, with the id as a stable tie-break.
fn account_order(account: &InvestmentAccount) -> (String, String) {
    series::name_order(&account.name, &account.id)
}
