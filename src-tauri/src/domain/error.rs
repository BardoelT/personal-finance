use thiserror::Error;

/// Validation and lookup failures of the income domain (CSD-001).
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
    JobCurrencyUnsupported,
    #[error("The currency cannot change once the job has paychecks.")]
    JobCurrencyLocked,
    #[error("This job does not exist.")]
    JobNotFound,
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
            Self::JobCurrencyUnsupported => "JOB_CURRENCY_UNSUPPORTED",
            Self::JobCurrencyLocked => "JOB_CURRENCY_LOCKED",
            Self::JobNotFound => "JOB_NOT_FOUND",
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
        }
    }
}
