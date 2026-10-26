//! Investment accounts and their monthly or yearly statements (CSD-005).

use std::collections::HashSet;
use std::fmt;

use rust_decimal::Decimal;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use uuid::Uuid;

use super::error::DomainError;
use super::income::{PayPeriod, Year, YearMonth};
use super::money::{parse_plain_decimal, with_two_decimals, zero};

const MAX_NAME_CHARS: usize = 100;
const MAX_NOTE_CHARS: usize = 2000;

/// Largest balance accepted (1,000,000,000.00), a guard against typos.
pub const MAX_BALANCE: Decimal = Decimal::from_parts(1_215_752_192, 23, 0, false, 2);

/// Identifies an investment account.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct InvestmentAccountId(Uuid);

impl InvestmentAccountId {
    fn generate() -> Self {
        Self(Uuid::new_v4())
    }

    /// Parses the text form returned to the frontend.
    ///
    /// # Errors
    /// Returns `InvestmentAccountNotFound` for text that is not an account id.
    pub fn parse(text: &str) -> Result<Self, DomainError> {
        Uuid::parse_str(text.trim())
            .map(Self)
            .map_err(|_| DomainError::InvestmentAccountNotFound)
    }
}

impl fmt::Display for InvestmentAccountId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.0, f)
    }
}

/// The period of one statement: a calendar month (`YYYY-MM`) or a calendar
/// year (`YYYY`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Period {
    Month(YearMonth),
    Year(Year),
}

impl Period {
    /// Parses `YYYY-MM` or `YYYY`; the shape decides which of the two it is.
    ///
    /// # Errors
    /// Returns `InvestmentStatementPeriodInvalid` for any other text.
    pub fn parse(text: &str) -> Result<Self, DomainError> {
        let invalid = |_| DomainError::InvestmentStatementPeriodInvalid;
        let text = text.trim();
        if text.contains('-') {
            YearMonth::parse(text).map(Self::Month).map_err(invalid)
        } else {
            Year::parse(text).map(Self::Year).map_err(invalid)
        }
    }

    /// The statement period an account needs to hold this period.
    #[must_use]
    pub fn kind(self) -> PayPeriod {
        match self {
            Self::Month(_) => PayPeriod::Monthly,
            Self::Year(_) => PayPeriod::Yearly,
        }
    }

    /// Newest-first ordering key: a year counts as December (CSD-005 REQ-16).
    fn sort_key(self) -> (i32, u8) {
        match self {
            Self::Month(month) => (month.year(), month.month()),
            Self::Year(year) => (year.value(), 12),
        }
    }

    /// Whether the period starts after the current month `today` (REQ-6).
    fn is_after(self, today: YearMonth) -> bool {
        match self {
            Self::Month(month) => month > today,
            Self::Year(year) => year.value() > today.year(),
        }
    }
}

impl fmt::Display for Period {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Month(month) => fmt::Display::fmt(month, f),
            Self::Year(year) => fmt::Display::fmt(year, f),
        }
    }
}

impl Serialize for Period {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.collect_str(self)
    }
}

impl<'de> Deserialize<'de> for Period {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let text = String::deserialize(deserializer)?;
        Self::parse(&text).map_err(serde::de::Error::custom)
    }
}

/// The closing balance of a statement, in the project currency, with exactly
/// two decimals (CSD-005 REQ-9).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Balance(Decimal);

impl Balance {
    /// Builds a balance from a decimal.
    ///
    /// # Errors
    /// Returns the first violated rule: negative, more than two decimals, or
    /// above 1,000,000,000.00. Zero is valid.
    pub fn new(value: Decimal) -> Result<Self, DomainError> {
        if value < Decimal::ZERO {
            return Err(DomainError::InvestmentBalanceNegative);
        }
        if value.normalize().scale() > 2 {
            return Err(DomainError::InvestmentBalancePrecision);
        }
        if value > MAX_BALANCE {
            return Err(DomainError::InvestmentBalanceTooLarge);
        }
        Ok(Self(with_two_decimals(value)))
    }

    /// Parses a balance typed by the user.
    ///
    /// # Errors
    /// Returns `InvestmentBalanceInvalid` for text that is not a plain decimal
    /// number, and otherwise the rules of [`Balance::new`].
    pub fn parse(text: &str) -> Result<Self, DomainError> {
        parse_plain_decimal(text)
            .ok_or(DomainError::InvestmentBalanceInvalid)
            .and_then(Self::new)
    }

    /// The balance as a decimal.
    #[must_use]
    pub fn value(self) -> Decimal {
        self.0
    }
}

/// Checks a signed amount: at most two decimals and an absolute value up to
/// 1,000,000,000.00 (CSD-005 REQ-22, REQ-25).
fn signed_amount(
    value: Decimal,
    precision: DomainError,
    too_large: DomainError,
) -> Result<Decimal, DomainError> {
    if value.normalize().scale() > 2 {
        return Err(precision);
    }
    if value.abs() > MAX_BALANCE {
        return Err(too_large);
    }
    Ok(with_two_decimals(value))
}

/// The net deposits of a statement: what the user put in minus what was taken
/// out during the period, signed, with exactly two decimals (CSD-005 REQ-22).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct NetDeposits(Decimal);

impl Default for NetDeposits {
    fn default() -> Self {
        Self(zero())
    }
}

impl NetDeposits {
    /// Builds net deposits from a decimal.
    ///
    /// # Errors
    /// Returns `InvestmentNetDepositsPrecision` for more than two decimals and
    /// `InvestmentNetDepositsTooLarge` above 1,000,000,000.00 in absolute value.
    pub fn new(value: Decimal) -> Result<Self, DomainError> {
        signed_amount(
            value,
            DomainError::InvestmentNetDepositsPrecision,
            DomainError::InvestmentNetDepositsTooLarge,
        )
        .map(Self)
    }

    /// Parses net deposits typed by the user. Empty text means `0.00`.
    ///
    /// # Errors
    /// Returns `InvestmentNetDepositsInvalid` for text that is not a plain
    /// decimal number, and otherwise the rules of [`NetDeposits::new`].
    pub fn parse(text: &str) -> Result<Self, DomainError> {
        if text.trim().is_empty() {
            return Ok(Self::default());
        }
        parse_plain_decimal(text)
            .ok_or(DomainError::InvestmentNetDepositsInvalid)
            .and_then(Self::new)
    }

    /// The net deposits as a decimal.
    #[must_use]
    pub fn value(self) -> Decimal {
        self.0
    }
}

/// Parses a capital growth typed by the user (CSD-005 REQ-25).
fn parse_growth(text: &str) -> Result<Decimal, DomainError> {
    let value = parse_plain_decimal(text).ok_or(DomainError::InvestmentGrowthInvalid)?;
    signed_amount(
        value,
        DomainError::InvestmentGrowthPrecision,
        DomainError::InvestmentGrowthTooLarge,
    )
}

/// The optional plain-text note of a statement, at most 2,000 characters
/// (CSD-005 REQ-10). It is kept exactly as typed.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Note(String);

impl Note {
    /// Builds a note.
    ///
    /// # Errors
    /// Returns `InvestmentNoteTooLong` above 2,000 characters (CON-8).
    pub fn new(text: &str) -> Result<Self, DomainError> {
        if text.chars().count() > MAX_NOTE_CHARS {
            return Err(DomainError::InvestmentNoteTooLong);
        }
        Ok(Self(text.to_owned()))
    }

    /// The note text.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// One place where the user holds investments (CSD-005 REQ-1).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct InvestmentAccount {
    pub id: InvestmentAccountId,
    pub name: String,
    pub statement_period: PayPeriod,
}

/// The record of one account for one period (CSD-005 REQ-6).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Statement {
    pub account_id: InvestmentAccountId,
    pub period: Period,
    pub balance: Balance,
    #[serde(default)]
    pub net_deposits: NetDeposits,
    #[serde(default)]
    pub note: Note,
}

/// Account fields as typed by the user, before validation.
#[derive(Clone, Debug, Default, Deserialize)]
pub struct AccountInput {
    #[serde(default)]
    pub name: String,
    /// `None` means Monthly when creating and "unchanged" when editing.
    #[serde(default)]
    pub statement_period: Option<String>,
}

struct ValidAccount {
    name: String,
    statement_period: Option<PayPeriod>,
}

impl AccountInput {
    fn validate(&self) -> Result<ValidAccount, DomainError> {
        let name = self.name.trim();
        if name.is_empty() {
            return Err(DomainError::InvestmentAccountNameRequired);
        }
        if name.chars().count() > MAX_NAME_CHARS {
            return Err(DomainError::InvestmentAccountNameTooLong);
        }
        let statement_period = self
            .statement_period
            .as_deref()
            .map(str::trim)
            .filter(|text| !text.is_empty())
            .map(|text| PayPeriod::parse(text).ok_or(DomainError::InvestmentPeriodUnsupported))
            .transpose()?;
        Ok(ValidAccount {
            name: name.to_owned(),
            statement_period,
        })
    }
}

/// One account row of the Investments view: its latest balance and period
/// (CSD-005 REQ-15).
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct AccountSummary {
    #[serde(flatten)]
    pub account: InvestmentAccount,
    pub statement_count: usize,
    pub latest_period: Option<Period>,
    pub latest_balance: Option<Decimal>,
}

/// The accounts with the total of their latest balances (CSD-005 REQ-15). The
/// total is `None` while no account has a statement.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct InvestmentsOverview {
    pub accounts: Vec<AccountSummary>,
    pub total: Option<Decimal>,
}

/// Form values for "copy previous month" or "copy previous year" (CSD-005
/// REQ-13): the note is never copied and the net deposits start at `0.00`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct StatementPrefill {
    pub balance: Option<Decimal>,
    pub net_deposits: Decimal,
}

/// Which of balance and capital growth the user typed in the statement form
/// (CSD-005 REQ-24).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CalculationInput {
    Balance,
    Growth,
}

/// The balance and capital growth the statement form shows after a
/// calculation. The capital growth is `None` when there is no earlier
/// statement (CSD-005 REQ-24).
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct StatementCalculation {
    pub balance: Decimal,
    pub capital_growth: Option<Decimal>,
}

/// All investment accounts with their statements.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Investments {
    #[serde(default)]
    accounts: Vec<InvestmentAccount>,
    #[serde(default)]
    statements: Vec<Statement>,
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
    ) -> Result<Statement, DomainError> {
        if self.account(account_id)?.statement_period != period.kind() {
            return Err(DomainError::InvestmentPeriodMismatch);
        }
        if period.is_after(today) {
            return Err(DomainError::InvestmentStatementInFuture);
        }
        if self.statement(account_id, period).is_some() {
            return Err(DomainError::InvestmentStatementDuplicate);
        }
        let statement = Statement {
            account_id,
            period,
            balance,
            net_deposits,
            note,
        };
        self.statements.push(statement.clone());
        Ok(statement)
    }

    /// Changes the balance, net deposits and note of a statement. The period is
    /// fixed (REQ-11).
    ///
    /// # Errors
    /// Returns `InvestmentAccountNotFound` or `InvestmentStatementNotFound`.
    pub fn update_statement(
        &mut self,
        account_id: InvestmentAccountId,
        period: Period,
        balance: Balance,
        net_deposits: NetDeposits,
        note: Note,
    ) -> Result<Statement, DomainError> {
        self.account(account_id)?;
        let statement = self
            .statements
            .iter_mut()
            .find(|s| s.account_id == account_id && s.period == period)
            .ok_or(DomainError::InvestmentStatementNotFound)?;
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
    pub fn all_statements(&self) -> Vec<(&InvestmentAccount, &Statement)> {
        let mut rows: Vec<(&InvestmentAccount, &Statement)> = self
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

    /// The accounts with their latest balances and the total (REQ-15). Periods
    /// are not aligned: each account counts with its own latest statement.
    #[must_use]
    pub fn overview(&self) -> InvestmentsOverview {
        let accounts: Vec<AccountSummary> = self
            .accounts()
            .into_iter()
            .map(|account| {
                let latest = self.latest(account.id);
                AccountSummary {
                    account: account.clone(),
                    statement_count: self.statement_count(account.id),
                    latest_period: latest.map(|statement| statement.period),
                    latest_balance: latest.map(|statement| statement.balance.value()),
                }
            })
            .collect();
        let mut balances = accounts.iter().filter_map(|summary| summary.latest_balance);
        let total = balances
            .next()
            .map(|first| balances.fold(first, |sum, balance| sum + balance));
        InvestmentsOverview { accounts, total }
    }

    /// Form values for "copy previous month" or "copy previous year": the
    /// balance of the latest statement and no note (REQ-13).
    ///
    /// # Errors
    /// Returns `InvestmentAccountNotFound`.
    pub fn prefill_from_latest(
        &self,
        id: InvestmentAccountId,
    ) -> Result<StatementPrefill, DomainError> {
        self.account(id)?;
        Ok(StatementPrefill {
            balance: self.latest(id).map(|statement| statement.balance.value()),
            net_deposits: zero(),
        })
    }

    /// The capital growth of a statement: its balance minus the balance of the
    /// previous statement of the account minus its net deposits. `None` for
    /// the first statement of an account (CSD-005 REQ-23).
    #[must_use]
    pub fn capital_growth(&self, statement: &Statement) -> Option<Decimal> {
        let previous = self.previous_balance(statement.account_id, statement.period)?;
        Some(with_two_decimals(
            statement.balance.value() - previous - statement.net_deposits.value(),
        ))
    }

    /// Calculates the other one of balance and capital growth for the
    /// statement form, from the value the user typed in and the net deposits.
    /// Nothing is stored (CSD-005 REQ-24, REQ-25).
    ///
    /// # Errors
    /// Returns `InvestmentAccountNotFound`, `InvestmentPeriodMismatch`, the
    /// net deposits and balance or growth errors of REQ-22, REQ-9 and REQ-25,
    /// or `InvestmentGrowthUnavailable` when a growth is typed in and there is
    /// no earlier statement.
    pub fn calculate(
        &self,
        account_id: InvestmentAccountId,
        period: Period,
        net_deposits: &str,
        input: CalculationInput,
        value: &str,
    ) -> Result<StatementCalculation, DomainError> {
        if self.account(account_id)?.statement_period != period.kind() {
            return Err(DomainError::InvestmentPeriodMismatch);
        }
        let net_deposits = NetDeposits::parse(net_deposits)?.value();
        let previous = self.previous_balance(account_id, period);
        match input {
            CalculationInput::Balance => {
                let balance = Balance::parse(value)?.value();
                Ok(StatementCalculation {
                    balance,
                    capital_growth: previous
                        .map(|previous| with_two_decimals(balance - previous - net_deposits)),
                })
            }
            CalculationInput::Growth => {
                let previous = previous.ok_or(DomainError::InvestmentGrowthUnavailable)?;
                let growth = parse_growth(value)?;
                let balance = Balance::new(previous + net_deposits + growth)?;
                Ok(StatementCalculation {
                    balance: balance.value(),
                    capital_growth: Some(growth),
                })
            }
        }
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

    fn statement(&self, account_id: InvestmentAccountId, period: Period) -> Option<&Statement> {
        self.statements
            .iter()
            .find(|s| s.account_id == account_id && s.period == period)
    }

    /// The balance of the statement of the account with the latest period
    /// before `period`, also when periods in between have none.
    fn previous_balance(&self, id: InvestmentAccountId, period: Period) -> Option<Decimal> {
        self.statements
            .iter()
            .filter(|s| s.account_id == id && s.period.sort_key() < period.sort_key())
            .max_by_key(|s| s.period.sort_key())
            .map(|s| s.balance.value())
    }

    /// The statement with the most recent period of an account.
    fn latest(&self, id: InvestmentAccountId) -> Option<&Statement> {
        self.statements
            .iter()
            .filter(|statement| statement.account_id == id)
            .max_by_key(|statement| statement.period.sort_key())
    }

    /// Whether another account already has this name, ignoring case (REQ-2).
    fn name_taken(&self, name: &str, except: Option<InvestmentAccountId>) -> bool {
        let wanted = name.to_lowercase();
        self.accounts
            .iter()
            .any(|account| Some(account.id) != except && account.name.to_lowercase() == wanted)
    }
}

/// Alphabetical order ignoring case, with the id as a stable tie-break.
fn account_order(account: &InvestmentAccount) -> (String, String) {
    (account.name.to_lowercase(), account.id.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn month(text: &str) -> Period {
        Period::parse(text).unwrap()
    }

    fn today() -> YearMonth {
        YearMonth::parse("2026-10").unwrap()
    }

    fn balance(text: &str) -> Balance {
        Balance::parse(text).unwrap()
    }

    fn no_note() -> Note {
        Note::default()
    }

    fn deposits(text: &str) -> NetDeposits {
        NetDeposits::parse(text).unwrap()
    }

    fn no_deposits() -> NetDeposits {
        NetDeposits::default()
    }

    fn add_with_deposits(
        investments: &mut Investments,
        id: InvestmentAccountId,
        period: &str,
        amount: &str,
        net_deposits: &str,
    ) {
        investments
            .add_statement(
                id,
                month(period),
                balance(amount),
                deposits(net_deposits),
                no_note(),
                today(),
            )
            .unwrap();
    }

    /// The capital growth of the statement of `period`, as text.
    fn growth(investments: &Investments, id: InvestmentAccountId, period: &str) -> Option<String> {
        let (_, statement) = investments
            .all_statements()
            .into_iter()
            .find(|(account, statement)| account.id == id && statement.period == month(period))
            .unwrap();
        investments
            .capital_growth(statement)
            .map(|value| value.to_string())
    }

    fn calculate(
        investments: &Investments,
        id: InvestmentAccountId,
        period: &str,
        net_deposits: &str,
        input: CalculationInput,
        value: &str,
    ) -> Result<StatementCalculation, DomainError> {
        investments.calculate(id, month(period), net_deposits, input, value)
    }

    /// A monthly account with a statement for 2026-08 of 12000.00.
    fn account_with_august() -> (Investments, InvestmentAccountId) {
        let mut investments = Investments::default();
        let id = add_account(&mut investments, "Example Brokerage", "Monthly");
        add(&mut investments, id, "2026-08", "12000.00");
        (investments, id)
    }

    fn input(name: &str, period: Option<&str>) -> AccountInput {
        AccountInput {
            name: name.to_owned(),
            statement_period: period.map(str::to_owned),
        }
    }

    fn add_account(investments: &mut Investments, name: &str, period: &str) -> InvestmentAccountId {
        investments
            .create_account(&input(name, Some(period)))
            .unwrap()
            .id
    }

    fn add(investments: &mut Investments, id: InvestmentAccountId, period: &str, amount: &str) {
        investments
            .add_statement(
                id,
                month(period),
                balance(amount),
                no_deposits(),
                no_note(),
                today(),
            )
            .unwrap();
    }

    fn periods(investments: &Investments) -> Vec<String> {
        investments
            .all_statements()
            .into_iter()
            .map(|(account, statement)| format!("{} {}", statement.period, account.name))
            .collect()
    }

    // ---- REQ-1 to REQ-5: accounts ----

    #[test]
    fn test_csd005_ac_1_1_an_account_is_monthly_by_default() {
        let mut investments = Investments::default();
        let account = investments
            .create_account(&input("Example Brokerage", None))
            .unwrap();
        assert_eq!(account.name, "Example Brokerage");
        assert_eq!(account.statement_period, PayPeriod::Monthly);
        assert_eq!(investments.statement_count(account.id), 0);
    }

    #[test]
    fn test_csd005_ac_1_2_a_yearly_account() {
        let mut investments = Investments::default();
        let account = investments
            .create_account(&input("Example Pension", Some("Yearly")))
            .unwrap();
        assert_eq!(account.statement_period, PayPeriod::Yearly);
    }

    #[test]
    fn test_csd005_ac_1_3_an_unknown_period_is_rejected() {
        let mut investments = Investments::default();
        let error = investments
            .create_account(&input("Example Brokerage", Some("Weekly")))
            .unwrap_err();
        assert_eq!(error.code(), "INVESTMENT_PERIOD_UNSUPPORTED");
        assert_eq!(investments.accounts().len(), 0);
    }

    #[test]
    fn test_csd005_ac_2_1_invalid_names_are_rejected() {
        for (name, code) in [
            ("", "INVESTMENT_ACCOUNT_NAME_REQUIRED"),
            ("   ", "INVESTMENT_ACCOUNT_NAME_REQUIRED"),
            ("EXAMPLE BROKERAGE", "INVESTMENT_ACCOUNT_NAME_DUPLICATE"),
            ("example brokerage", "INVESTMENT_ACCOUNT_NAME_DUPLICATE"),
        ] {
            let mut investments = Investments::default();
            add_account(&mut investments, "Example Brokerage", "Monthly");
            let error = investments.create_account(&input(name, None)).unwrap_err();
            assert_eq!(error.code(), code, "{name:?}");
            assert_eq!(investments.accounts().len(), 1, "{name:?}");
        }
    }

    #[test]
    fn test_csd005_ac_2_2_the_name_length_limit_counts_characters() {
        let mut investments = Investments::default();
        investments
            .create_account(&input(&"a".repeat(100), None))
            .unwrap();
        let error = investments
            .create_account(&input(&"b".repeat(101), None))
            .unwrap_err();
        assert_eq!(error.code(), "INVESTMENT_ACCOUNT_NAME_TOO_LONG");
        // 100 multi-byte characters are within the limit (CON-8).
        investments
            .create_account(&input(&"é".repeat(100), None))
            .unwrap();
    }

    #[test]
    fn test_csd005_ac_2_3_the_name_is_stored_trimmed() {
        let mut investments = Investments::default();
        let account = investments
            .create_account(&input("  Example Brokerage  ", None))
            .unwrap();
        assert_eq!(account.name, "Example Brokerage");
    }

    #[test]
    fn test_csd005_ac_2_4_renaming_to_an_existing_name_is_rejected() {
        let mut investments = Investments::default();
        add_account(&mut investments, "Example Brokerage", "Monthly");
        let pension = add_account(&mut investments, "Example Pension", "Yearly");
        let error = investments
            .update_account(pension, &input("example brokerage", None))
            .unwrap_err();
        assert_eq!(error.code(), "INVESTMENT_ACCOUNT_NAME_DUPLICATE");
        assert_eq!(investments.accounts().len(), 2);
    }

    #[test]
    fn test_csd005_ac_2_5_changing_only_the_case_of_its_own_name_is_allowed() {
        let mut investments = Investments::default();
        let id = add_account(&mut investments, "Example Brokerage", "Monthly");
        let account = investments
            .update_account(id, &input("EXAMPLE BROKERAGE", None))
            .unwrap();
        assert_eq!(account.name, "EXAMPLE BROKERAGE");
    }

    #[test]
    fn test_csd005_ac_3_1_renaming_keeps_the_statements() {
        let mut investments = Investments::default();
        let id = add_account(&mut investments, "Example Brokerage", "Monthly");
        add(&mut investments, id, "2026-08", "1000.00");
        add(&mut investments, id, "2026-09", "1100.00");
        let account = investments
            .update_account(id, &input("Example Broker", None))
            .unwrap();
        assert_eq!(account.name, "Example Broker");
        assert_eq!(account.statement_period, PayPeriod::Monthly);
        assert_eq!(investments.statement_count(id), 2);
    }

    #[test]
    fn test_csd005_ac_3_2_the_period_of_an_account_without_statements_can_change() {
        let mut investments = Investments::default();
        let id = add_account(&mut investments, "Example Brokerage", "Monthly");
        let account = investments
            .update_account(id, &input("Example Brokerage", Some("Yearly")))
            .unwrap();
        assert_eq!(account.statement_period, PayPeriod::Yearly);
    }

    #[test]
    fn test_csd005_ac_4_1_the_period_is_locked_once_statements_exist() {
        let mut investments = Investments::default();
        let id = add_account(&mut investments, "Example Brokerage", "Monthly");
        add(&mut investments, id, "2026-09", "12500.00");
        let error = investments
            .update_account(id, &input("Example Brokerage", Some("Yearly")))
            .unwrap_err();
        assert_eq!(error.code(), "INVESTMENT_PERIOD_LOCKED");
        assert_eq!(
            investments.accounts()[0].statement_period,
            PayPeriod::Monthly
        );
        // Keeping the same period is not a change.
        investments
            .update_account(id, &input("Example Brokerage", Some("Monthly")))
            .unwrap();
    }

    #[test]
    fn test_csd005_ac_4_2_the_period_can_change_after_deleting_the_last_statement() {
        let mut investments = Investments::default();
        let id = add_account(&mut investments, "Example Brokerage", "Monthly");
        add(&mut investments, id, "2026-09", "12500.00");
        investments.delete_statement(id, month("2026-09")).unwrap();
        let account = investments
            .update_account(id, &input("Example Brokerage", Some("Yearly")))
            .unwrap();
        assert_eq!(account.statement_period, PayPeriod::Yearly);
    }

    #[test]
    fn test_csd005_ac_5_1_deleting_an_account_deletes_its_statements() {
        let mut investments = Investments::default();
        let id = add_account(&mut investments, "Example Brokerage", "Monthly");
        let other = add_account(&mut investments, "Example Crypto", "Monthly");
        for period in ["2026-07", "2026-08", "2026-09"] {
            add(&mut investments, id, period, "1000.00");
        }
        add(&mut investments, other, "2026-09", "50.00");
        investments.delete_account(id).unwrap();
        assert_eq!(investments.accounts().len(), 1);
        assert_eq!(investments.total_statement_count(), 1);
        assert_eq!(
            investments.delete_account(id).unwrap_err().code(),
            "INVESTMENT_ACCOUNT_NOT_FOUND"
        );
    }

    // ---- REQ-6 to REQ-14: statements ----

    #[test]
    fn test_csd005_ac_6_1_a_monthly_statement_with_a_note() {
        let mut investments = Investments::default();
        let id = add_account(&mut investments, "Example Brokerage", "Monthly");
        let note = Note::new("Bought example fund").unwrap();
        let statement = investments
            .add_statement(
                id,
                month("2026-09"),
                balance("12500.00"),
                no_deposits(),
                note,
                today(),
            )
            .unwrap();
        assert_eq!(statement.balance.value().to_string(), "12500.00");
        assert_eq!(statement.note.as_str(), "Bought example fund");
    }

    #[test]
    fn test_csd005_ac_6_2_a_yearly_statement_without_a_note() {
        let mut investments = Investments::default();
        let id = add_account(&mut investments, "Example Pension", "Yearly");
        let statement = investments
            .add_statement(
                id,
                month("2025"),
                balance("80000.00"),
                no_deposits(),
                no_note(),
                today(),
            )
            .unwrap();
        assert_eq!(statement.balance.value().to_string(), "80000.00");
        assert_eq!(statement.note.as_str(), "");
    }

    #[test]
    fn test_csd005_ac_6_3_old_periods_are_accepted() {
        let mut investments = Investments::default();
        let id = add_account(&mut investments, "Example Brokerage", "Monthly");
        add(&mut investments, id, "2019-01", "1000.00");
        assert_eq!(investments.statement_count(id), 1);
    }

    #[test]
    fn test_csd005_ac_6_4_the_current_period_is_accepted_and_later_ones_are_not() {
        for (kind, period, ok) in [
            ("Monthly", "2026-10", true),
            ("Monthly", "2026-11", false),
            ("Yearly", "2026", true),
            ("Yearly", "2027", false),
        ] {
            let mut investments = Investments::default();
            let id = add_account(&mut investments, "Example Account", kind);
            let result = investments.add_statement(
                id,
                month(period),
                balance("1000.00"),
                no_deposits(),
                no_note(),
                today(),
            );
            match (ok, result) {
                (true, Ok(_)) => {}
                (false, Err(error)) => {
                    assert_eq!(error.code(), "INVESTMENT_STATEMENT_IN_FUTURE");
                    assert_eq!(investments.statement_count(id), 0);
                }
                (ok, result) => panic!("{kind} {period}: expected ok={ok}, got {result:?}"),
            }
        }
    }

    #[test]
    fn test_csd005_ac_6_5_an_invalid_period_is_rejected() {
        for text in [
            "2026-13", "2026-9", "abcd", "26", "1899", "", "2026-", "-2026",
        ] {
            assert_eq!(
                Period::parse(text).unwrap_err().code(),
                "INVESTMENT_STATEMENT_PERIOD_INVALID",
                "{text:?}"
            );
        }
        assert_eq!(month("2026-09").to_string(), "2026-09");
        assert_eq!(month(" 2025 ").to_string(), "2025");
    }

    #[test]
    fn test_csd005_ac_7_1_a_duplicate_period_is_rejected_and_changes_nothing() {
        let mut investments = Investments::default();
        let id = add_account(&mut investments, "Example Brokerage", "Monthly");
        let first = Note::new("First").unwrap();
        investments
            .add_statement(
                id,
                month("2026-09"),
                balance("12500.00"),
                no_deposits(),
                first,
                today(),
            )
            .unwrap();
        let error = investments
            .add_statement(
                id,
                month("2026-09"),
                balance("13000.00"),
                no_deposits(),
                no_note(),
                today(),
            )
            .unwrap_err();
        assert_eq!(error.code(), "INVESTMENT_STATEMENT_DUPLICATE");
        let rows = investments.all_statements();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].1.balance.value().to_string(), "12500.00");
        assert_eq!(rows[0].1.note.as_str(), "First");
    }

    #[test]
    fn test_csd005_ac_7_2_the_same_period_on_another_account_is_allowed() {
        let mut investments = Investments::default();
        let first = add_account(&mut investments, "Example Brokerage", "Monthly");
        let second = add_account(&mut investments, "Example Crypto", "Monthly");
        add(&mut investments, first, "2026-09", "1000.00");
        add(&mut investments, second, "2026-09", "50.00");
        assert_eq!(investments.total_statement_count(), 2);
    }

    #[test]
    fn test_csd005_ac_8_1_a_monthly_statement_for_a_yearly_account_is_rejected() {
        let mut investments = Investments::default();
        let id = add_account(&mut investments, "Example Pension", "Yearly");
        let error = investments
            .add_statement(
                id,
                month("2026-09"),
                balance("1000.00"),
                no_deposits(),
                no_note(),
                today(),
            )
            .unwrap_err();
        assert_eq!(error.code(), "INVESTMENT_PERIOD_MISMATCH");
    }

    #[test]
    fn test_csd005_ac_8_2_a_yearly_statement_for_a_monthly_account_is_rejected() {
        let mut investments = Investments::default();
        let id = add_account(&mut investments, "Example Brokerage", "Monthly");
        let error = investments
            .add_statement(
                id,
                month("2025"),
                balance("1000.00"),
                no_deposits(),
                no_note(),
                today(),
            )
            .unwrap_err();
        assert_eq!(error.code(), "INVESTMENT_PERIOD_MISMATCH");
    }

    #[test]
    fn test_csd005_ac_9_1_invalid_balances_are_rejected() {
        for (text, code) in [
            ("-0.01", "INVESTMENT_BALANCE_NEGATIVE"),
            ("1000.999", "INVESTMENT_BALANCE_PRECISION"),
            ("1000000000.01", "INVESTMENT_BALANCE_TOO_LARGE"),
        ] {
            assert_eq!(Balance::parse(text).unwrap_err().code(), code, "{text}");
        }
    }

    #[test]
    fn test_csd005_ac_9_2_boundary_balances_are_accepted() {
        assert_eq!(balance("0.00").value().to_string(), "0.00");
        assert_eq!(balance("0").value().to_string(), "0.00");
        assert_eq!(balance("-0.00").value().to_string(), "0.00");
        assert_eq!(
            balance("1000000000.00").value().to_string(),
            "1000000000.00"
        );
        assert_eq!(balance("12500.5").value().to_string(), "12500.50");
        assert_eq!(MAX_BALANCE.to_string(), "1000000000.00");
    }

    #[test]
    fn test_csd005_ac_9_3_an_invalid_balance_in_an_edit_changes_nothing() {
        // The command parses the balance before the domain is called, so an
        // invalid one never reaches `update_statement`.
        let mut investments = Investments::default();
        let id = add_account(&mut investments, "Example Brokerage", "Monthly");
        add(&mut investments, id, "2026-09", "12500.00");
        assert_eq!(
            Balance::parse("-1.00").unwrap_err().code(),
            "INVESTMENT_BALANCE_NEGATIVE"
        );
        assert_eq!(
            investments.all_statements()[0]
                .1
                .balance
                .value()
                .to_string(),
            "12500.00"
        );
    }

    #[test]
    fn test_csd005_ac_9_4_text_that_is_not_a_plain_number_is_rejected() {
        for text in [
            "", "  ", "abc", "1e3", "1_000", "12500,50", "--1", "1.", ".5", "+5",
        ] {
            assert_eq!(
                Balance::parse(text).unwrap_err().code(),
                "INVESTMENT_BALANCE_INVALID",
                "{text:?}"
            );
        }
    }

    #[test]
    fn test_csd005_ac_10_1_the_note_length_limit_counts_characters() {
        assert!(Note::new(&"a".repeat(2000)).is_ok());
        assert!(Note::new(&"é".repeat(2000)).is_ok());
        assert_eq!(
            Note::new(&"a".repeat(2001)).unwrap_err().code(),
            "INVESTMENT_NOTE_TOO_LONG"
        );
    }

    #[test]
    fn test_csd005_ac_10_2_line_breaks_and_spaces_are_kept() {
        let note = Note::new(" line one\nline two ").unwrap();
        assert_eq!(note.as_str(), " line one\nline two ");
        assert_eq!(note.as_str().lines().count(), 2);
    }

    #[test]
    fn test_csd005_ac_11_1_editing_changes_balance_and_note() {
        let mut investments = Investments::default();
        let id = add_account(&mut investments, "Example Brokerage", "Monthly");
        let first = Note::new("First").unwrap();
        investments
            .add_statement(
                id,
                month("2026-09"),
                balance("12500.00"),
                no_deposits(),
                first,
                today(),
            )
            .unwrap();
        let corrected = Note::new("Corrected").unwrap();
        let statement = investments
            .update_statement(
                id,
                month("2026-09"),
                balance("12750.00"),
                no_deposits(),
                corrected,
            )
            .unwrap();
        assert_eq!(statement.balance.value().to_string(), "12750.00");
        assert_eq!(statement.note.as_str(), "Corrected");
        assert_eq!(investments.total_statement_count(), 1);
    }

    #[test]
    fn test_csd005_ac_11_2_a_note_can_be_cleared() {
        let mut investments = Investments::default();
        let id = add_account(&mut investments, "Example Brokerage", "Monthly");
        let first = Note::new("First").unwrap();
        investments
            .add_statement(
                id,
                month("2026-09"),
                balance("12500.00"),
                no_deposits(),
                first,
                today(),
            )
            .unwrap();
        let statement = investments
            .update_statement(
                id,
                month("2026-09"),
                balance("12500.00"),
                no_deposits(),
                no_note(),
            )
            .unwrap();
        assert_eq!(statement.note.as_str(), "");
    }

    #[test]
    fn test_csd005_ac_11_4_editing_a_missing_statement_is_an_error() {
        let mut investments = Investments::default();
        let id = add_account(&mut investments, "Example Brokerage", "Monthly");
        let error = investments
            .update_statement(
                id,
                month("2026-09"),
                balance("1.00"),
                no_deposits(),
                no_note(),
            )
            .unwrap_err();
        assert_eq!(error.code(), "INVESTMENT_STATEMENT_NOT_FOUND");
    }

    #[test]
    fn test_csd005_ac_12_1_deleting_a_statement_keeps_the_others() {
        let mut investments = Investments::default();
        let id = add_account(&mut investments, "Example Brokerage", "Monthly");
        add(&mut investments, id, "2026-08", "1000.00");
        add(&mut investments, id, "2026-09", "1100.00");
        investments.delete_statement(id, month("2026-09")).unwrap();
        assert_eq!(periods(&investments), ["2026-08 Example Brokerage"]);
        assert_eq!(
            investments
                .delete_statement(id, month("2026-09"))
                .unwrap_err()
                .code(),
            "INVESTMENT_STATEMENT_NOT_FOUND"
        );
    }

    #[test]
    fn test_csd005_ac_13_1_copy_previous_takes_the_latest_balance_and_no_note() {
        let mut investments = Investments::default();
        let id = add_account(&mut investments, "Example Brokerage", "Monthly");
        add(&mut investments, id, "2026-07", "12000.00");
        let note = Note::new("Bought example fund").unwrap();
        investments
            .add_statement(
                id,
                month("2026-08"),
                balance("12500.00"),
                no_deposits(),
                note,
                today(),
            )
            .unwrap();
        let prefill = investments.prefill_from_latest(id).unwrap();
        assert_eq!(prefill.balance.unwrap().to_string(), "12500.00");
        assert_eq!(investments.total_statement_count(), 2);
    }

    #[test]
    fn test_csd005_ac_13_2_copy_previous_without_statements_is_empty() {
        let mut investments = Investments::default();
        let id = add_account(&mut investments, "Example Brokerage", "Monthly");
        assert_eq!(investments.prefill_from_latest(id).unwrap().balance, None);
    }

    // ---- REQ-15, REQ-16: latest balance, total, ordering ----

    #[test]
    fn test_csd005_ac_15_1_each_account_shows_its_latest_balance() {
        let mut investments = Investments::default();
        let id = add_account(&mut investments, "Example Brokerage", "Monthly");
        add(&mut investments, id, "2026-07", "12000.00");
        add(&mut investments, id, "2026-09", "12500.00");
        let overview = investments.overview();
        let summary = &overview.accounts[0];
        assert_eq!(summary.latest_period, Some(month("2026-09")));
        assert_eq!(summary.latest_balance.unwrap().to_string(), "12500.00");
        assert_eq!(summary.statement_count, 2);
    }

    #[test]
    fn test_csd005_ac_15_2_the_total_adds_latest_balances_of_different_periods() {
        let mut investments = Investments::default();
        let brokerage = add_account(&mut investments, "Example Brokerage", "Monthly");
        let pension = add_account(&mut investments, "Example Pension", "Yearly");
        add(&mut investments, brokerage, "2026-09", "12500.50");
        add(&mut investments, pension, "2025", "80000.00");
        let overview = investments.overview();
        assert_eq!(overview.total.unwrap().to_string(), "92500.50");
        let latest: Vec<String> = overview
            .accounts
            .iter()
            .filter_map(|summary| summary.latest_period.map(|period| period.to_string()))
            .collect();
        assert_eq!(latest, ["2026-09", "2025"]);
    }

    #[test]
    fn test_csd005_ac_15_3_accounts_without_statements_do_not_count() {
        let mut investments = Investments::default();
        let brokerage = add_account(&mut investments, "Example Brokerage", "Monthly");
        add_account(&mut investments, "Example Crypto", "Monthly");
        add(&mut investments, brokerage, "2026-09", "12500.00");
        let overview = investments.overview();
        assert_eq!(overview.total.unwrap().to_string(), "12500.00");
        assert_eq!(overview.accounts[1].latest_balance, None);
        assert_eq!(overview.accounts[1].latest_period, None);
    }

    #[test]
    fn test_csd005_ac_15_3_without_any_statement_there_is_no_total() {
        let mut investments = Investments::default();
        assert_eq!(investments.overview().total, None);
        add_account(&mut investments, "Example Brokerage", "Monthly");
        assert_eq!(investments.overview().total, None);
    }

    #[test]
    fn test_csd005_ac_15_4_a_new_latest_statement_updates_the_total() {
        let mut investments = Investments::default();
        let id = add_account(&mut investments, "Example Brokerage", "Monthly");
        add(&mut investments, id, "2026-09", "12500.00");
        add(&mut investments, id, "2026-10", "13000.00");
        let overview = investments.overview();
        assert_eq!(overview.accounts[0].latest_period, Some(month("2026-10")));
        assert_eq!(overview.total.unwrap().to_string(), "13000.00");
    }

    #[test]
    fn test_csd005_ac_15_5_an_older_statement_does_not_change_the_latest_balance() {
        let mut investments = Investments::default();
        let id = add_account(&mut investments, "Example Brokerage", "Monthly");
        add(&mut investments, id, "2026-09", "12500.00");
        add(&mut investments, id, "2026-01", "9000.00");
        let overview = investments.overview();
        assert_eq!(overview.accounts[0].latest_period, Some(month("2026-09")));
        assert_eq!(overview.total.unwrap().to_string(), "12500.00");
    }

    #[test]
    fn test_csd005_ac_16_1_accounts_are_alphabetical_ignoring_case() {
        let mut investments = Investments::default();
        add_account(&mut investments, "example pension", "Yearly");
        add_account(&mut investments, "Example Brokerage", "Monthly");
        add_account(&mut investments, "Example Crypto", "Monthly");
        let names: Vec<&str> = investments
            .accounts()
            .into_iter()
            .map(|account| account.name.as_str())
            .collect();
        assert_eq!(
            names,
            ["Example Brokerage", "Example Crypto", "example pension"]
        );
    }

    #[test]
    fn test_csd005_ac_16_2_the_newest_statement_is_first() {
        let mut investments = Investments::default();
        let id = add_account(&mut investments, "Example Brokerage", "Monthly");
        add(&mut investments, id, "2026-07", "1000.00");
        add(&mut investments, id, "2026-09", "1100.00");
        assert_eq!(
            periods(&investments),
            ["2026-09 Example Brokerage", "2026-07 Example Brokerage"]
        );
    }

    #[test]
    fn test_csd005_ac_16_3_a_yearly_statement_counts_as_december() {
        let mut investments = Investments::default();
        let brokerage = add_account(&mut investments, "Example Brokerage", "Monthly");
        let pension = add_account(&mut investments, "Example Pension", "Yearly");
        add(&mut investments, brokerage, "2025-11", "1000.00");
        add(&mut investments, brokerage, "2025-12", "1100.00");
        add(&mut investments, pension, "2025", "80000.00");
        assert_eq!(
            periods(&investments),
            [
                "2025-12 Example Brokerage",
                "2025 Example Pension",
                "2025-11 Example Brokerage"
            ]
        );
    }

    #[test]
    fn test_csd005_ac_16_4_the_same_period_is_ordered_by_account() {
        let mut investments = Investments::default();
        let crypto = add_account(&mut investments, "Example Crypto", "Monthly");
        let brokerage = add_account(&mut investments, "Example Brokerage", "Monthly");
        add(&mut investments, crypto, "2026-09", "50.00");
        add(&mut investments, brokerage, "2026-09", "1000.00");
        assert_eq!(
            periods(&investments),
            ["2026-09 Example Brokerage", "2026-09 Example Crypto"]
        );
    }

    // ---- REQ-19: data read from a file ----

    #[test]
    fn test_csd005_ac_19_1_serialising_and_reading_back_gives_the_same_data() {
        let mut investments = Investments::default();
        let id = add_account(&mut investments, "Example Pension", "Yearly");
        let note = Note::new("Line one\nLine two").unwrap();
        investments
            .add_statement(
                id,
                month("2025"),
                balance("80000.00"),
                no_deposits(),
                note,
                today(),
            )
            .unwrap();
        let json = serde_json::to_string(&investments).unwrap();
        assert!(json.contains("\"balance\":\"80000.00\""), "{json}");
        let back: Investments = serde_json::from_str(&json).unwrap();
        assert_eq!(back, investments);
        back.validate().unwrap();
    }

    #[test]
    fn test_csd005_ac_19_2_a_missing_section_is_empty() {
        let investments: Investments = serde_json::from_str("{}").unwrap();
        assert_eq!(investments, Investments::default());
        investments.validate().unwrap();
    }

    // ---- REQ-13: net deposits are not copied ----

    #[test]
    fn test_csd005_ac_13_3_net_deposits_are_not_copied() {
        let mut investments = Investments::default();
        let id = add_account(&mut investments, "Example Brokerage", "Monthly");
        add_with_deposits(&mut investments, id, "2026-08", "12500.00", "300.00");
        let prefill = investments.prefill_from_latest(id).unwrap();
        assert_eq!(prefill.balance.unwrap().to_string(), "12500.00");
        assert_eq!(prefill.net_deposits.to_string(), "0.00");
    }

    // ---- REQ-19: net deposits in the file ----

    #[test]
    fn test_csd005_ac_19_3_net_deposits_round_trip() {
        let mut investments = Investments::default();
        let id = add_account(&mut investments, "Example Brokerage", "Monthly");
        add_with_deposits(&mut investments, id, "2026-09", "12500.00", "-250.00");
        let json = serde_json::to_string(&investments).unwrap();
        assert!(json.contains("\"net_deposits\":\"-250.00\""), "{json}");
        let back: Investments = serde_json::from_str(&json).unwrap();
        assert_eq!(back, investments);
        back.validate().unwrap();
    }

    #[test]
    fn test_csd005_ac_19_4_a_statement_without_net_deposits_gets_zero() {
        let id = Uuid::new_v4();
        let json = format!(
            r#"{{"accounts":[{{"id":"{id}","name":"Example","statement_period":"Monthly"}}],
                "statements":[{{"account_id":"{id}","period":"2026-09","balance":"12500.00"}}]}}"#
        );
        let investments: Investments = serde_json::from_str(&json).unwrap();
        investments.validate().unwrap();
        let rows = investments.all_statements();
        assert_eq!(rows[0].1.net_deposits.value().to_string(), "0.00");
    }

    #[test]
    fn validate_rejects_net_deposits_out_of_range() {
        let id = Uuid::new_v4();
        let json = format!(
            r#"{{"accounts":[{{"id":"{id}","name":"Example","statement_period":"Monthly"}}],
                "statements":[{{"account_id":"{id}","period":"2026-09","balance":"1.00",
                "net_deposits":"1000000000.01"}}]}}"#
        );
        let investments: Investments = serde_json::from_str(&json).unwrap();
        assert_eq!(
            investments.validate().unwrap_err().code(),
            "INVESTMENT_DATA_INVALID"
        );
    }

    // ---- REQ-21, REQ-22: net deposits ----

    #[test]
    fn test_csd005_ac_21_1_net_deposits_default_to_zero() {
        let mut investments = Investments::default();
        let id = add_account(&mut investments, "Example Brokerage", "Monthly");
        let statement = investments
            .add_statement(
                id,
                month("2026-09"),
                balance("12500.00"),
                NetDeposits::parse("").unwrap(),
                no_note(),
                today(),
            )
            .unwrap();
        assert_eq!(statement.net_deposits.value().to_string(), "0.00");
        assert_eq!(
            NetDeposits::parse("  ").unwrap().value().to_string(),
            "0.00"
        );
    }

    #[test]
    fn test_csd005_ac_21_2_a_deposit_is_stored() {
        let mut investments = Investments::default();
        let id = add_account(&mut investments, "Example Brokerage", "Monthly");
        add_with_deposits(&mut investments, id, "2026-09", "12500.00", "300.00");
        let rows = investments.all_statements();
        assert_eq!(rows[0].1.net_deposits.value().to_string(), "300.00");
    }

    #[test]
    fn test_csd005_ac_21_3_a_net_withdrawal_is_stored() {
        let mut investments = Investments::default();
        let id = add_account(&mut investments, "Example Brokerage", "Monthly");
        add_with_deposits(&mut investments, id, "2026-09", "12500.00", "-250.00");
        let rows = investments.all_statements();
        assert_eq!(rows[0].1.net_deposits.value().to_string(), "-250.00");
    }

    #[test]
    fn test_csd005_ac_21_4_editing_changes_the_net_deposits() {
        let mut investments = Investments::default();
        let id = add_account(&mut investments, "Example Brokerage", "Monthly");
        add_with_deposits(&mut investments, id, "2026-09", "12500.00", "300.00");
        let statement = investments
            .update_statement(
                id,
                month("2026-09"),
                balance("12500.00"),
                deposits("0.00"),
                no_note(),
            )
            .unwrap();
        assert_eq!(statement.balance.value().to_string(), "12500.00");
        assert_eq!(statement.net_deposits.value().to_string(), "0.00");
    }

    #[test]
    fn test_csd005_ac_22_1_invalid_net_deposits_are_rejected() {
        for (text, code) in [
            ("abc", "INVESTMENT_NET_DEPOSITS_INVALID"),
            ("1e3", "INVESTMENT_NET_DEPOSITS_INVALID"),
            ("300,50", "INVESTMENT_NET_DEPOSITS_INVALID"),
            ("100.999", "INVESTMENT_NET_DEPOSITS_PRECISION"),
            ("-100.999", "INVESTMENT_NET_DEPOSITS_PRECISION"),
            ("1000000000.01", "INVESTMENT_NET_DEPOSITS_TOO_LARGE"),
            ("-1000000000.01", "INVESTMENT_NET_DEPOSITS_TOO_LARGE"),
        ] {
            assert_eq!(NetDeposits::parse(text).unwrap_err().code(), code, "{text}");
        }
    }

    #[test]
    fn test_csd005_ac_22_2_boundary_net_deposits_are_accepted() {
        for (text, shown) in [
            ("0.00", "0.00"),
            ("-0.00", "0.00"),
            ("1000000000.00", "1000000000.00"),
            ("-1000000000.00", "-1000000000.00"),
            ("300.5", "300.50"),
        ] {
            assert_eq!(deposits(text).value().to_string(), shown, "{text}");
        }
    }

    #[test]
    fn test_csd005_ac_22_3_invalid_net_deposits_in_an_edit_change_nothing() {
        // The command parses the net deposits before the domain is called.
        let mut investments = Investments::default();
        let id = add_account(&mut investments, "Example Brokerage", "Monthly");
        add_with_deposits(&mut investments, id, "2026-09", "12500.00", "300.00");
        assert_eq!(
            NetDeposits::parse("100.999").unwrap_err().code(),
            "INVESTMENT_NET_DEPOSITS_PRECISION"
        );
        let rows = investments.all_statements();
        assert_eq!(rows[0].1.net_deposits.value().to_string(), "300.00");
    }

    // ---- REQ-23: capital growth ----

    #[test]
    fn test_csd005_ac_23_1_growth_with_a_deposit() {
        let (mut investments, id) = account_with_august();
        add_with_deposits(&mut investments, id, "2026-09", "12500.00", "300.00");
        assert_eq!(growth(&investments, id, "2026-09").unwrap(), "200.00");
    }

    #[test]
    fn test_csd005_ac_23_2_a_loss_is_a_negative_growth() {
        let (mut investments, id) = account_with_august();
        add(&mut investments, id, "2026-09", "11000.00");
        assert_eq!(growth(&investments, id, "2026-09").unwrap(), "-1000.00");
    }

    #[test]
    fn test_csd005_ac_23_3_the_first_statement_has_no_growth() {
        let mut investments = Investments::default();
        let id = add_account(&mut investments, "Example Brokerage", "Monthly");
        add(&mut investments, id, "2026-09", "12500.00");
        assert_eq!(growth(&investments, id, "2026-09"), None);
    }

    #[test]
    fn test_csd005_ac_23_4_skipped_periods_are_bridged() {
        let mut investments = Investments::default();
        let id = add_account(&mut investments, "Example Brokerage", "Monthly");
        add(&mut investments, id, "2026-06", "10000.00");
        add_with_deposits(&mut investments, id, "2026-09", "10900.00", "600.00");
        assert_eq!(growth(&investments, id, "2026-09").unwrap(), "300.00");
    }

    #[test]
    fn test_csd005_ac_23_5_a_yearly_account() {
        let mut investments = Investments::default();
        let id = add_account(&mut investments, "Example Pension", "Yearly");
        add(&mut investments, id, "2024", "70000.00");
        add_with_deposits(&mut investments, id, "2025", "80000.00", "6000.00");
        assert_eq!(growth(&investments, id, "2025").unwrap(), "4000.00");
    }

    #[test]
    fn test_csd005_ac_23_5_the_previous_statement_is_of_the_same_account() {
        let mut investments = Investments::default();
        let brokerage = add_account(&mut investments, "Example Brokerage", "Monthly");
        let crypto = add_account(&mut investments, "Example Crypto", "Monthly");
        add(&mut investments, crypto, "2026-08", "50.00");
        add(&mut investments, brokerage, "2026-09", "12500.00");
        assert_eq!(growth(&investments, brokerage, "2026-09"), None);
    }

    #[test]
    fn test_csd005_ac_23_6_editing_an_earlier_balance_changes_the_later_growth() {
        let (mut investments, id) = account_with_august();
        add_with_deposits(&mut investments, id, "2026-09", "12500.00", "300.00");
        investments
            .update_statement(
                id,
                month("2026-08"),
                balance("12100.00"),
                no_deposits(),
                no_note(),
            )
            .unwrap();
        assert_eq!(growth(&investments, id, "2026-09").unwrap(), "100.00");
    }

    #[test]
    fn test_csd005_ac_23_7_a_statement_added_before_the_first_gives_it_a_growth() {
        let mut investments = Investments::default();
        let id = add_account(&mut investments, "Example Brokerage", "Monthly");
        add(&mut investments, id, "2026-09", "12500.00");
        add(&mut investments, id, "2026-07", "12000.00");
        assert_eq!(growth(&investments, id, "2026-07"), None);
        assert_eq!(growth(&investments, id, "2026-09").unwrap(), "500.00");
    }

    #[test]
    fn test_csd005_ac_23_8_deleting_the_previous_statement_changes_the_growth() {
        let mut investments = Investments::default();
        let id = add_account(&mut investments, "Example Brokerage", "Monthly");
        add(&mut investments, id, "2026-07", "12000.00");
        add(&mut investments, id, "2026-08", "12200.00");
        add(&mut investments, id, "2026-09", "12500.00");
        investments.delete_statement(id, month("2026-08")).unwrap();
        assert_eq!(growth(&investments, id, "2026-09").unwrap(), "500.00");
    }

    #[test]
    fn test_csd005_ac_23_9_growth_is_never_stored() {
        let (mut investments, id) = account_with_august();
        add_with_deposits(&mut investments, id, "2026-09", "12500.00", "300.00");
        let json = serde_json::to_string(&investments).unwrap();
        assert!(!json.contains("growth"), "{json}");
    }

    // ---- REQ-24, REQ-25: the form calculation ----

    #[test]
    fn test_csd005_ac_24_1_a_balance_gives_the_growth() {
        let (investments, id) = account_with_august();
        let result = calculate(
            &investments,
            id,
            "2026-09",
            "",
            CalculationInput::Balance,
            "12500.00",
        )
        .unwrap();
        assert_eq!(result.balance.to_string(), "12500.00");
        assert_eq!(result.capital_growth.unwrap().to_string(), "500.00");
    }

    #[test]
    fn test_csd005_ac_24_2_a_growth_gives_the_balance() {
        let (investments, id) = account_with_august();
        let result = calculate(
            &investments,
            id,
            "2026-09",
            "300.00",
            CalculationInput::Growth,
            "200.00",
        )
        .unwrap();
        assert_eq!(result.balance.to_string(), "12500.00");
        assert_eq!(result.capital_growth.unwrap().to_string(), "200.00");
    }

    #[test]
    fn test_csd005_ac_24_3_changing_net_deposits_recalculates_the_growth() {
        let (investments, id) = account_with_august();
        let result = calculate(
            &investments,
            id,
            "2026-09",
            "300.00",
            CalculationInput::Balance,
            "12500.00",
        )
        .unwrap();
        assert_eq!(result.balance.to_string(), "12500.00");
        assert_eq!(result.capital_growth.unwrap().to_string(), "200.00");
    }

    #[test]
    fn test_csd005_ac_24_4_changing_net_deposits_recalculates_the_balance() {
        let (investments, id) = account_with_august();
        let result = calculate(
            &investments,
            id,
            "2026-09",
            "300.00",
            CalculationInput::Growth,
            "200.00",
        )
        .unwrap();
        assert_eq!(result.balance.to_string(), "12500.00");
        assert_eq!(result.capital_growth.unwrap().to_string(), "200.00");
    }

    #[test]
    fn test_csd005_ac_24_5_a_negative_growth_and_net_withdrawal() {
        let (investments, id) = account_with_august();
        let result = calculate(
            &investments,
            id,
            "2026-09",
            "-500.00",
            CalculationInput::Growth,
            "-250.50",
        )
        .unwrap();
        assert_eq!(result.balance.to_string(), "11249.50");
    }

    #[test]
    fn test_csd005_ac_24_7_the_first_statement_has_no_growth_to_calculate() {
        let mut investments = Investments::default();
        let id = add_account(&mut investments, "Example Brokerage", "Monthly");
        let result = calculate(
            &investments,
            id,
            "2026-09",
            "300.00",
            CalculationInput::Balance,
            "12500.00",
        )
        .unwrap();
        assert_eq!(result.capital_growth, None);
    }

    #[test]
    fn test_csd005_ac_24_12_a_growth_without_an_earlier_statement_is_rejected() {
        let mut investments = Investments::default();
        let id = add_account(&mut investments, "Example Brokerage", "Monthly");
        let error = calculate(
            &investments,
            id,
            "2026-09",
            "",
            CalculationInput::Growth,
            "200.00",
        )
        .unwrap_err();
        assert_eq!(error.code(), "INVESTMENT_GROWTH_UNAVAILABLE");
    }

    #[test]
    fn test_csd005_ac_24_8_a_period_before_all_statements_has_no_growth_to_calculate() {
        let mut investments = Investments::default();
        let id = add_account(&mut investments, "Example Brokerage", "Monthly");
        add(&mut investments, id, "2026-09", "12500.00");
        let result = calculate(
            &investments,
            id,
            "2026-07",
            "",
            CalculationInput::Balance,
            "12000.00",
        )
        .unwrap();
        assert_eq!(result.capital_growth, None);
    }

    #[test]
    fn test_csd005_ac_24_9_calculating_stores_nothing() {
        let (investments, id) = account_with_august();
        let before = investments.clone();
        calculate(
            &investments,
            id,
            "2026-09",
            "300.00",
            CalculationInput::Growth,
            "200.00",
        )
        .unwrap();
        assert_eq!(investments, before);
    }

    #[test]
    fn test_csd005_ac_24_10_the_edit_form_shows_the_calculated_growth() {
        let (mut investments, id) = account_with_august();
        add_with_deposits(&mut investments, id, "2026-09", "12500.00", "300.00");
        // Opening the statement calculates the growth from its stored balance
        // and net deposits, without the statement itself being the previous one.
        let result = calculate(
            &investments,
            id,
            "2026-09",
            "300.00",
            CalculationInput::Balance,
            "12500.00",
        )
        .unwrap();
        assert_eq!(result.capital_growth.unwrap().to_string(), "200.00");
    }

    #[test]
    fn test_csd005_ac_24_11_a_yearly_account_calculates_against_the_previous_year() {
        let mut investments = Investments::default();
        let id = add_account(&mut investments, "Example Pension", "Yearly");
        add(&mut investments, id, "2024", "70000.00");
        let result = investments
            .calculate(
                id,
                month("2025"),
                "6000.00",
                CalculationInput::Balance,
                "80000.00",
            )
            .unwrap();
        assert_eq!(result.capital_growth.unwrap().to_string(), "4000.00");
    }

    #[test]
    fn test_csd005_ac_25_1_a_growth_that_gives_a_negative_balance_is_rejected() {
        let (investments, id) = account_with_august();
        let error = calculate(
            &investments,
            id,
            "2026-09",
            "0.00",
            CalculationInput::Growth,
            "-12500.00",
        )
        .unwrap_err();
        assert_eq!(error.code(), "INVESTMENT_BALANCE_NEGATIVE");
    }

    #[test]
    fn test_csd005_ac_25_2_invalid_growth_is_rejected() {
        let (investments, id) = account_with_august();
        for (text, code) in [
            ("abc", "INVESTMENT_GROWTH_INVALID"),
            ("", "INVESTMENT_GROWTH_INVALID"),
            ("100.999", "INVESTMENT_GROWTH_PRECISION"),
            ("1000000000.01", "INVESTMENT_GROWTH_TOO_LARGE"),
            ("-1000000000.01", "INVESTMENT_GROWTH_TOO_LARGE"),
        ] {
            let error = calculate(
                &investments,
                id,
                "2026-09",
                "",
                CalculationInput::Growth,
                text,
            )
            .unwrap_err();
            assert_eq!(error.code(), code, "{text:?}");
        }
    }

    #[test]
    fn test_csd005_ac_25_3_a_growth_that_gives_a_balance_above_the_limit_is_rejected() {
        let mut investments = Investments::default();
        let id = add_account(&mut investments, "Example Brokerage", "Monthly");
        add(&mut investments, id, "2026-08", "999999999.00");
        let error = calculate(
            &investments,
            id,
            "2026-09",
            "0.00",
            CalculationInput::Growth,
            "2.00",
        )
        .unwrap_err();
        assert_eq!(error.code(), "INVESTMENT_BALANCE_TOO_LARGE");
    }

    #[test]
    fn test_csd005_ac_25_4_invalid_net_deposits_in_the_form_are_rejected() {
        let (investments, id) = account_with_august();
        for (text, code) in [
            ("100.999", "INVESTMENT_NET_DEPOSITS_PRECISION"),
            ("abc", "INVESTMENT_NET_DEPOSITS_INVALID"),
            ("1000000000.01", "INVESTMENT_NET_DEPOSITS_TOO_LARGE"),
        ] {
            let error = calculate(
                &investments,
                id,
                "2026-09",
                text,
                CalculationInput::Balance,
                "12500.00",
            )
            .unwrap_err();
            assert_eq!(error.code(), code, "{text}");
        }
    }

    #[test]
    fn test_csd005_ac_25_5_an_invalid_balance_in_the_form_is_rejected() {
        let (investments, id) = account_with_august();
        let error = calculate(
            &investments,
            id,
            "2026-09",
            "",
            CalculationInput::Balance,
            "-1.00",
        )
        .unwrap_err();
        assert_eq!(error.code(), "INVESTMENT_BALANCE_NEGATIVE");
    }

    #[test]
    fn calculating_needs_an_existing_account_and_a_matching_period() {
        let (investments, id) = account_with_august();
        let missing = InvestmentAccountId::parse(&Uuid::new_v4().to_string()).unwrap();
        assert_eq!(
            calculate(
                &investments,
                missing,
                "2026-09",
                "",
                CalculationInput::Balance,
                "1.00"
            )
            .unwrap_err()
            .code(),
            "INVESTMENT_ACCOUNT_NOT_FOUND"
        );
        assert_eq!(
            investments
                .calculate(id, month("2026"), "", CalculationInput::Balance, "1.00")
                .unwrap_err()
                .code(),
            "INVESTMENT_PERIOD_MISMATCH"
        );
    }

    #[test]
    fn validate_rejects_inconsistent_files() {
        let id = Uuid::new_v4();
        let other = Uuid::new_v4();
        let cases = [
            // duplicate names, ignoring case
            format!(
                r#"{{"accounts":[
                    {{"id":"{id}","name":"Example","statement_period":"Monthly"}},
                    {{"id":"{other}","name":"EXAMPLE","statement_period":"Monthly"}}]}}"#
            ),
            // statement of an unknown account
            format!(
                r#"{{"statements":[{{"account_id":"{id}","period":"2026-09","balance":"1.00"}}]}}"#
            ),
            // statement period does not match the account
            format!(
                r#"{{"accounts":[{{"id":"{id}","name":"Example","statement_period":"Yearly"}}],
                    "statements":[{{"account_id":"{id}","period":"2026-09","balance":"1.00"}}]}}"#
            ),
            // duplicate period
            format!(
                r#"{{"accounts":[{{"id":"{id}","name":"Example","statement_period":"Monthly"}}],
                    "statements":[
                    {{"account_id":"{id}","period":"2026-09","balance":"1.00"}},
                    {{"account_id":"{id}","period":"2026-09","balance":"2.00"}}]}}"#
            ),
            // negative balance
            format!(
                r#"{{"accounts":[{{"id":"{id}","name":"Example","statement_period":"Monthly"}}],
                    "statements":[{{"account_id":"{id}","period":"2026-09","balance":"-1.00"}}]}}"#
            ),
            // blank name
            format!(r#"{{"accounts":[{{"id":"{id}","name":"  ","statement_period":"Monthly"}}]}}"#),
        ];
        for json in cases {
            let investments: Investments = serde_json::from_str(&json).unwrap();
            assert_eq!(
                investments.validate().unwrap_err().code(),
                "INVESTMENT_DATA_INVALID",
                "{json}"
            );
        }
    }

    #[test]
    fn an_unparseable_period_in_a_file_fails_to_read() {
        let json = r#"{"statements":[{"account_id":"00000000-0000-0000-0000-000000000000","period":"2026-13","balance":"1.00"}]}"#;
        assert!(serde_json::from_str::<Investments>(json).is_err());
    }
}
