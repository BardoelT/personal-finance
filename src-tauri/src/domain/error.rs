use thiserror::Error;

/// Validation and lookup failures of the income (CSD-001) and investments
/// (CSD-005) domains.
///
/// Messages are safe to show to the user: they never contain user data.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum DomainError {
    #[error("Enter an employer name.")]
    JobEmployerRequired,
    #[error("Enter a start date.")]
    JobStartRequired,
    #[error("The end date cannot be before the start date.")]
    JobEndBeforeStart,
    #[error("Enter dates as dd/mm/yyyy.")]
    JobDateInvalid,
    #[error("Employer and title can be at most 200 characters.")]
    JobFieldTooLong,
    #[error("Choose EUR, USD or GBP.")]
    ProjectCurrencyUnsupported,
    #[error("This job does not exist.")]
    JobNotFound,
    #[error("Choose Monthly or Yearly.")]
    JobPeriodUnsupported,
    #[error("The pay period cannot change once the job has paychecks or annual statements.")]
    JobPeriodLocked,
    #[error("A monthly job has paychecks and a yearly job has annual statements.")]
    JobPeriodMismatch,
    #[error("Enter the year as YYYY.")]
    StatementYearInvalid,
    #[error("An annual statement cannot be for a year that has not started yet.")]
    StatementYearInFuture,
    #[error("This job already has an annual statement for that year.")]
    StatementDuplicateYear,
    #[error("There is no annual statement for that year.")]
    StatementNotFound,
    #[error("Enter the month as YYYY-MM.")]
    PaycheckMonthInvalid,
    #[error("A paycheck cannot be for a month that has not started yet.")]
    PaycheckMonthInFuture,
    #[error("This job already has a paycheck for that month.")]
    PaycheckDuplicateMonth,
    #[error("There is no paycheck for that month.")]
    PaycheckNotFound,
    #[error("Gross salary must be greater than 0.00.")]
    PaycheckGrossInvalid,
    #[error("Enter amounts as numbers such as 3000.00.")]
    PaycheckAmountInvalid,
    #[error("Amounts cannot be negative.")]
    PaycheckAmountNegative,
    #[error("Amounts can have at most 2 decimal places.")]
    PaycheckAmountPrecision,
    #[error("Amounts can be at most 1000000.00.")]
    PaycheckAmountTooLarge,
    #[error("The income data is not consistent.")]
    DataInvalid,
    #[error("Enter an account name.")]
    InvestmentAccountNameRequired,
    #[error("The account name can be at most 100 characters.")]
    InvestmentAccountNameTooLong,
    #[error("An account with this name already exists.")]
    InvestmentAccountNameDuplicate,
    #[error("This investment account does not exist.")]
    InvestmentAccountNotFound,
    #[error("Choose Monthly or Yearly.")]
    InvestmentPeriodUnsupported,
    #[error("The statement period cannot change once the account has statements.")]
    InvestmentPeriodLocked,
    #[error(
        "A monthly account has monthly statements and a yearly account has yearly statements."
    )]
    InvestmentPeriodMismatch,
    #[error("A statement cannot be for a period that has not started yet.")]
    InvestmentStatementInFuture,
    #[error("Enter the month as YYYY-MM or the year as YYYY.")]
    InvestmentStatementPeriodInvalid,
    #[error("This account already has a statement for that period.")]
    InvestmentStatementDuplicate,
    #[error("There is no statement for that period.")]
    InvestmentStatementNotFound,
    #[error("Enter the balance as a number such as 12500.00.")]
    InvestmentBalanceInvalid,
    #[error("The balance cannot be negative.")]
    InvestmentBalanceNegative,
    #[error("The balance can have at most 2 decimal places.")]
    InvestmentBalancePrecision,
    #[error("The balance can be at most 1000000000.00.")]
    InvestmentBalanceTooLarge,
    #[error("A note can be at most 2000 characters.")]
    InvestmentNoteTooLong,
    #[error("The investment data is not consistent.")]
    InvestmentDataInvalid,
}

impl DomainError {
    /// Stable machine-readable code for the frontend.
    #[must_use]
    pub fn code(&self) -> &'static str {
        match self {
            Self::JobEmployerRequired => "JOB_EMPLOYER_REQUIRED",
            Self::JobStartRequired => "JOB_START_REQUIRED",
            Self::JobEndBeforeStart => "JOB_END_BEFORE_START",
            Self::JobDateInvalid => "JOB_DATE_INVALID",
            Self::JobFieldTooLong => "JOB_FIELD_TOO_LONG",
            Self::ProjectCurrencyUnsupported => "PROJECT_CURRENCY_UNSUPPORTED",
            Self::JobNotFound => "JOB_NOT_FOUND",
            Self::JobPeriodUnsupported => "JOB_PERIOD_UNSUPPORTED",
            Self::JobPeriodLocked => "JOB_PERIOD_LOCKED",
            Self::JobPeriodMismatch => "JOB_PERIOD_MISMATCH",
            Self::StatementYearInvalid => "STATEMENT_YEAR_INVALID",
            Self::StatementYearInFuture => "STATEMENT_YEAR_IN_FUTURE",
            Self::StatementDuplicateYear => "STATEMENT_DUPLICATE_YEAR",
            Self::StatementNotFound => "STATEMENT_NOT_FOUND",
            Self::PaycheckMonthInvalid => "PAYCHECK_MONTH_INVALID",
            Self::PaycheckMonthInFuture => "PAYCHECK_MONTH_IN_FUTURE",
            Self::PaycheckDuplicateMonth => "PAYCHECK_DUPLICATE_MONTH",
            Self::PaycheckNotFound => "PAYCHECK_NOT_FOUND",
            Self::PaycheckGrossInvalid => "PAYCHECK_GROSS_INVALID",
            Self::PaycheckAmountInvalid => "PAYCHECK_AMOUNT_INVALID",
            Self::PaycheckAmountNegative => "PAYCHECK_AMOUNT_NEGATIVE",
            Self::PaycheckAmountPrecision => "PAYCHECK_AMOUNT_PRECISION",
            Self::PaycheckAmountTooLarge => "PAYCHECK_AMOUNT_TOO_LARGE",
            Self::DataInvalid => "INCOME_DATA_INVALID",
            Self::InvestmentAccountNameRequired => "INVESTMENT_ACCOUNT_NAME_REQUIRED",
            Self::InvestmentAccountNameTooLong => "INVESTMENT_ACCOUNT_NAME_TOO_LONG",
            Self::InvestmentAccountNameDuplicate => "INVESTMENT_ACCOUNT_NAME_DUPLICATE",
            Self::InvestmentAccountNotFound => "INVESTMENT_ACCOUNT_NOT_FOUND",
            Self::InvestmentPeriodUnsupported => "INVESTMENT_PERIOD_UNSUPPORTED",
            Self::InvestmentPeriodLocked => "INVESTMENT_PERIOD_LOCKED",
            Self::InvestmentPeriodMismatch => "INVESTMENT_PERIOD_MISMATCH",
            Self::InvestmentStatementInFuture => "INVESTMENT_STATEMENT_IN_FUTURE",
            Self::InvestmentStatementPeriodInvalid => "INVESTMENT_STATEMENT_PERIOD_INVALID",
            Self::InvestmentStatementDuplicate => "INVESTMENT_STATEMENT_DUPLICATE",
            Self::InvestmentStatementNotFound => "INVESTMENT_STATEMENT_NOT_FOUND",
            Self::InvestmentBalanceInvalid => "INVESTMENT_BALANCE_INVALID",
            Self::InvestmentBalanceNegative => "INVESTMENT_BALANCE_NEGATIVE",
            Self::InvestmentBalancePrecision => "INVESTMENT_BALANCE_PRECISION",
            Self::InvestmentBalanceTooLarge => "INVESTMENT_BALANCE_TOO_LARGE",
            Self::InvestmentNoteTooLong => "INVESTMENT_NOTE_TOO_LONG",
            Self::InvestmentDataInvalid => "INVESTMENT_DATA_INVALID",
        }
    }
}
