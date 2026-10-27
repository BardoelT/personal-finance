#![allow(
    clippy::needless_pass_by_value,
    reason = "Tauri commands receive their arguments by value"
)]

use rust_decimal::Decimal;
use serde::Serialize;
use tauri::State;

use super::AppState;
use crate::domain::{
    AccountInput, Balance, CalculationInput, InvestmentAccount, InvestmentAccountId,
    InvestmentInsightsYear, InvestmentStatement, InvestmentYearlyTotal, Investments,
    InvestmentsOverview, NetDeposits, Note, Period, StatementCalculation, StatementPrefill,
};
use crate::error::AppError;

/// A statement with the name of its account, as the statements table shows it
/// (CSD-005 REQ-17).
#[derive(Serialize)]
pub struct StatementView {
    account_id: InvestmentAccountId,
    account_name: String,
    /// `YYYY-MM` for a monthly account, `YYYY` for a yearly one.
    period: String,
    balance: Decimal,
    net_deposits: Decimal,
    /// Derived from the previous statement, or from 0.00 for the first
    /// statement of an account (REQ-23).
    capital_growth: Decimal,
    note: String,
}

impl StatementView {
    fn new(
        investments: &Investments,
        account: &InvestmentAccount,
        statement: &InvestmentStatement,
    ) -> Self {
        Self {
            account_id: account.id,
            account_name: account.name.clone(),
            period: statement.period.to_string(),
            balance: statement.balance.value(),
            net_deposits: statement.net_deposits.value(),
            capital_growth: investments.capital_growth(statement),
            note: statement.note.as_str().to_owned(),
        }
    }
}

/// The accounts with their latest balances and the total of those (REQ-15).
#[tauri::command]
pub fn list_investment_accounts(
    state: State<'_, AppState>,
) -> Result<InvestmentsOverview, AppError> {
    let project = state.project()?;
    Ok(project.investments().overview())
}

#[tauri::command]
pub fn create_investment_account(
    state: State<'_, AppState>,
    input: AccountInput,
) -> Result<InvestmentAccount, AppError> {
    let mut project = state.project()?;
    Ok(project.investments_mut().create_account(&input)?)
}

#[tauri::command]
pub fn update_investment_account(
    state: State<'_, AppState>,
    account_id: String,
    input: AccountInput,
) -> Result<InvestmentAccount, AppError> {
    let mut project = state.project()?;
    Ok(project
        .investments_mut()
        .update_account(InvestmentAccountId::parse(&account_id)?, &input)?)
}

#[tauri::command]
pub fn delete_investment_account(
    state: State<'_, AppState>,
    account_id: String,
) -> Result<(), AppError> {
    let mut project = state.project()?;
    project
        .investments_mut()
        .delete_account(InvestmentAccountId::parse(&account_id)?)?;
    Ok(())
}

/// The statements of all accounts, newest period first (REQ-16).
#[tauri::command]
pub fn list_investment_statements(
    state: State<'_, AppState>,
) -> Result<Vec<StatementView>, AppError> {
    let project = state.project()?;
    let investments = project.investments();
    Ok(investments
        .all_statements()
        .into_iter()
        .map(|(account, statement)| StatementView::new(investments, account, statement))
        .collect())
}

#[tauri::command]
pub fn add_investment_statement(
    state: State<'_, AppState>,
    account_id: String,
    period: String,
    balance: String,
    net_deposits: Option<String>,
    note: Option<String>,
) -> Result<StatementView, AppError> {
    let today = state.today();
    let account_id = InvestmentAccountId::parse(&account_id)?;
    let period = Period::parse(&period)?;
    let balance = Balance::parse(&balance)?;
    let net_deposits = NetDeposits::parse(net_deposits.as_deref().unwrap_or_default())?;
    let note = Note::new(note.as_deref().unwrap_or_default())?;
    let mut project = state.project()?;
    let statement = project.investments_mut().add_statement(
        account_id,
        period,
        balance,
        net_deposits,
        note,
        today,
    )?;
    statement_view(&project, &statement)
}

#[tauri::command]
pub fn update_investment_statement(
    state: State<'_, AppState>,
    account_id: String,
    period: String,
    balance: String,
    net_deposits: Option<String>,
    note: Option<String>,
) -> Result<StatementView, AppError> {
    let account_id = InvestmentAccountId::parse(&account_id)?;
    let period = Period::parse(&period)?;
    let balance = Balance::parse(&balance)?;
    let net_deposits = NetDeposits::parse(net_deposits.as_deref().unwrap_or_default())?;
    let note = Note::new(note.as_deref().unwrap_or_default())?;
    let mut project = state.project()?;
    let statement = project.investments_mut().update_statement(
        account_id,
        period,
        balance,
        net_deposits,
        note,
    )?;
    statement_view(&project, &statement)
}

#[tauri::command]
pub fn delete_investment_statement(
    state: State<'_, AppState>,
    account_id: String,
    period: String,
) -> Result<(), AppError> {
    let account_id = InvestmentAccountId::parse(&account_id)?;
    let period = Period::parse(&period)?;
    let mut project = state.project()?;
    project
        .investments_mut()
        .delete_statement(account_id, period)?;
    Ok(())
}

/// Form values for "copy previous month" or "copy previous year" (REQ-13).
#[tauri::command]
pub fn copy_previous_investment_statement(
    state: State<'_, AppState>,
    account_id: String,
) -> Result<StatementPrefill, AppError> {
    let project = state.project()?;
    Ok(project
        .investments()
        .prefill_from_latest(InvestmentAccountId::parse(&account_id)?)?)
}

/// Yearly totals over all accounts, newest year first (REQ-26).
#[tauri::command]
pub fn yearly_investment_totals(
    state: State<'_, AppState>,
) -> Result<Vec<InvestmentYearlyTotal>, AppError> {
    let project = state.project()?;
    Ok(project.investments().yearly_totals())
}

/// The yearly series of the Insights chart, computed from the project as it is
/// now, saved or not (CSD-007 REQ-13).
#[tauri::command]
pub fn get_investment_insights(state: State<'_, AppState>) -> Result<InvestmentInsights, AppError> {
    let today = state.today();
    let project = state.project()?;
    Ok(InvestmentInsights {
        years: project.investments().insights(today),
    })
}

/// Payload of `get_investment_insights` (CSD-007 section 5).
#[derive(Serialize)]
pub struct InvestmentInsights {
    years: Vec<InvestmentInsightsYear>,
}

/// Calculates the balance or the capital growth for the statement form
/// (REQ-24, REQ-25). Stores nothing.
#[tauri::command]
pub fn calculate_investment_statement(
    state: State<'_, AppState>,
    account_id: String,
    period: String,
    net_deposits: Option<String>,
    input: CalculationInput,
    value: String,
) -> Result<StatementCalculation, AppError> {
    let account_id = InvestmentAccountId::parse(&account_id)?;
    let period = Period::parse(&period)?;
    let project = state.project()?;
    Ok(project.investments().calculate(
        account_id,
        period,
        net_deposits.as_deref().unwrap_or_default(),
        input,
        &value,
    )?)
}

fn statement_view(
    project: &crate::storage::Project,
    statement: &InvestmentStatement,
) -> Result<StatementView, AppError> {
    let account = project
        .investments()
        .accounts()
        .into_iter()
        .find(|account| account.id == statement.account_id)
        .ok_or(AppError::Internal)?;
    Ok(StatementView::new(
        project.investments(),
        account,
        statement,
    ))
}
