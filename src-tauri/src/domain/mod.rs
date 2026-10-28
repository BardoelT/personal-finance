//! Pure business logic: no Tauri, no IO.

pub mod error;
pub mod income;
pub mod investments;
pub mod money;
pub mod project_data;
pub mod settings;

pub use error::DomainError;
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
pub use project_data::ProjectData;
pub use settings::Settings;
