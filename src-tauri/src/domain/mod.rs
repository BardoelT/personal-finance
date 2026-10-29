//! Pure business logic: no Tauri, no IO.

pub mod assets;
pub mod bank_accounts;
pub mod debts;
pub mod error;
pub mod expenses;
pub mod income;
pub mod investments;
pub mod money;
pub mod preferences;
pub mod project_data;
pub mod settings;

pub use assets::{
    Asset, AssetId, AssetInsights, AssetValuation, AssetYearlyValue, Assets, AssetsOverview,
    ValuationPrefill as AssetValuationPrefill,
};
pub use bank_accounts::{
    BankAccount, BankAccountId, BankAccounts, BankBalance, BankEntry, BankInsights, BankOverview,
    BankYearlyTotal, EntryPrefill,
};
pub use debts::{
    Debt, DebtId, DebtInsights, DebtStatement, DebtYearlyTotal, Debts, DebtsOverview,
    StatementPrefill as DebtStatementPrefill,
};
pub use error::DomainError;
pub use expenses::{
    Expense, ExpenseId, ExpenseInsights, ExpenseList, ExpensePrefill, ExpenseYearlyTotal, Expenses,
};
pub use income::{
    Entry, Income, IncomeInsightsYear, IncomeOverview, IncomeYearlyTotal, Job, JobId, JobInput,
    PaycheckPrefill, Year, YearMonth,
};
pub use investments::{
    AccountInput, Balance, CalculationInput, InvestmentAccount, InvestmentAccountId,
    InvestmentInsightsYear, InvestmentStatement, InvestmentYearlyTotal, Investments,
    InvestmentsOverview, NetDeposits, Note, Period, StatementCalculation, StatementPrefill,
};
pub use money::{Amounts, Currency};
pub use preferences::Theme;
pub use project_data::ProjectData;
pub use settings::Settings;
