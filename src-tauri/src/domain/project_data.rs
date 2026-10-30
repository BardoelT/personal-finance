use super::bank_accounts::BankAccounts;
use super::debts::Debts;
use super::expenses::Expenses;
use super::houses::Houses;
use super::income::Income;
use super::investments::Investments;
use super::settings::Settings;

/// Everything a project file holds: the income data, the investments, the bank
/// accounts, the debts, the expenses, the houses and the project settings.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ProjectData {
    pub income: Income,
    pub investments: Investments,
    pub bank_accounts: BankAccounts,
    pub debts: Debts,
    pub houses: Houses,
    pub expenses: Expenses,
    pub settings: Settings,
}
