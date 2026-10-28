use super::bank_accounts::BankAccounts;
use super::income::Income;
use super::investments::Investments;
use super::settings::Settings;

/// Everything a project file holds: the income data, the investments, the bank
/// accounts and the project settings.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ProjectData {
    pub income: Income,
    pub investments: Investments,
    pub bank_accounts: BankAccounts,
    pub settings: Settings,
}
