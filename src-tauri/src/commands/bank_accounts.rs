#![allow(
    clippy::needless_pass_by_value,
    reason = "Tauri commands receive their arguments by value"
)]

use rust_decimal::Decimal;
use serde::Serialize;
use tauri::State;

use super::AppState;
use crate::domain::bank_accounts::{bank_note, parse_period, AccountInput};
use crate::domain::{
    BankAccount, BankAccountId, BankAccounts, BankBalance, BankEntry, BankInsights, BankOverview,
    BankYearlyTotal, EntryPrefill,
};
use crate::error::AppError;
use crate::storage::Project;

/// An entry with the name of its account, as the entries table shows it
/// (CSD-009 REQ-20).
#[derive(Serialize)]
pub struct EntryView {
    account_id: BankAccountId,
    account_name: String,
    /// `YYYY-MM` for a monthly account, `YYYY` for a yearly one.
    period: String,
    balance: Decimal,
    note: String,
}

impl EntryView {
    fn new(account: &BankAccount, entry: &BankEntry) -> Self {
        Self {
            account_id: account.id,
            account_name: account.name.clone(),
            period: entry.period.to_string(),
            balance: entry.balance.value(),
            note: entry.note.as_str().to_owned(),
        }
    }
}

/// The accounts with their latest balances and the total of those (REQ-17).
#[tauri::command]
pub fn list_bank_accounts(state: State<'_, AppState>) -> Result<BankOverview, AppError> {
    let project = state.project()?;
    Ok(project.bank_accounts().overview())
}

#[tauri::command]
pub fn create_bank_account(
    state: State<'_, AppState>,
    input: AccountInput,
) -> Result<BankAccount, AppError> {
    let mut project = state.project()?;
    Ok(project.bank_accounts_mut().create_account(&input)?)
}

#[tauri::command]
pub fn update_bank_account(
    state: State<'_, AppState>,
    account_id: String,
    input: AccountInput,
) -> Result<BankAccount, AppError> {
    let mut project = state.project()?;
    Ok(project
        .bank_accounts_mut()
        .update_account(BankAccountId::parse(&account_id)?, &input)?)
}

#[tauri::command]
pub fn delete_bank_account(state: State<'_, AppState>, account_id: String) -> Result<(), AppError> {
    let mut project = state.project()?;
    project
        .bank_accounts_mut()
        .delete_account(BankAccountId::parse(&account_id)?)?;
    Ok(())
}

/// The entries of all accounts, newest period first (REQ-18).
#[tauri::command]
pub fn list_bank_entries(state: State<'_, AppState>) -> Result<Vec<EntryView>, AppError> {
    let project = state.project()?;
    Ok(project
        .bank_accounts()
        .all_entries()
        .into_iter()
        .map(|(account, entry)| EntryView::new(account, entry))
        .collect())
}

#[tauri::command]
pub fn add_bank_entry(
    state: State<'_, AppState>,
    account_id: String,
    period: String,
    balance: String,
    note: Option<String>,
) -> Result<EntryView, AppError> {
    let today = state.today();
    let account_id = BankAccountId::parse(&account_id)?;
    let period = parse_period(&period)?;
    let balance = BankBalance::parse(&balance)?;
    let note = bank_note(note.as_deref().unwrap_or_default())?;
    let mut project = state.project()?;
    let entry = project
        .bank_accounts_mut()
        .add_entry(account_id, period, balance, note, today)?;
    entry_view(&project, &entry)
}

#[tauri::command]
pub fn update_bank_entry(
    state: State<'_, AppState>,
    account_id: String,
    period: String,
    new_period: String,
    balance: String,
    note: Option<String>,
) -> Result<EntryView, AppError> {
    let today = state.today();
    let account_id = BankAccountId::parse(&account_id)?;
    let period = parse_period(&period)?;
    let new_period = parse_period(&new_period)?;
    let balance = BankBalance::parse(&balance)?;
    let note = bank_note(note.as_deref().unwrap_or_default())?;
    let mut project = state.project()?;
    let entry = project
        .bank_accounts_mut()
        .update_entry(account_id, period, new_period, balance, note, today)?;
    entry_view(&project, &entry)
}

#[tauri::command]
pub fn delete_bank_entry(
    state: State<'_, AppState>,
    account_id: String,
    period: String,
) -> Result<(), AppError> {
    let account_id = BankAccountId::parse(&account_id)?;
    let period = parse_period(&period)?;
    let mut project = state.project()?;
    project
        .bank_accounts_mut()
        .delete_entry(account_id, period)?;
    Ok(())
}

/// Form values for "copy previous month" or "copy previous year" (REQ-15).
#[tauri::command]
pub fn copy_previous_bank_entry(
    state: State<'_, AppState>,
    account_id: String,
) -> Result<EntryPrefill, AppError> {
    let project = state.project()?;
    Ok(project
        .bank_accounts()
        .prefill_from_latest(BankAccountId::parse(&account_id)?)?)
}

/// Yearly totals over all accounts, newest year first (REQ-19).
#[tauri::command]
pub fn yearly_bank_totals(state: State<'_, AppState>) -> Result<Vec<BankYearlyTotal>, AppError> {
    let project = state.project()?;
    Ok(project.bank_accounts().yearly_totals())
}

/// The banks and the yearly series of the Insights chart, computed from the
/// project as it is now, saved or not (CSD-010 REQ-15).
#[tauri::command]
pub fn get_bank_insights(state: State<'_, AppState>) -> Result<BankInsights, AppError> {
    let today = state.today();
    let project = state.project()?;
    Ok(project.bank_accounts().insights(today))
}

fn entry_view(project: &Project, entry: &BankEntry) -> Result<EntryView, AppError> {
    let bank_accounts: &BankAccounts = project.bank_accounts();
    let account = bank_accounts
        .accounts()
        .into_iter()
        .find(|account| account.id == entry.account_id)
        .ok_or(AppError::Internal)?;
    Ok(EntryView::new(account, entry))
}
