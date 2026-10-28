//! Capital growth of a statement and the balance-or-growth calculation of the form (CSD-005 REQ-23 to REQ-25).

use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

use crate::domain::error::DomainError;
use crate::domain::money::{with_two_decimals, zero};

use super::amounts::parse_growth;
use super::{Balance, InvestmentAccountId, Investments, NetDeposits, Period, Statement};

/// Which of balance and capital growth the user typed in the statement form
/// (CSD-005 REQ-24).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CalculationInput {
    Balance,
    Growth,
}

/// The balance and capital growth the statement form shows after a
/// calculation (CSD-005 REQ-24).
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct StatementCalculation {
    pub balance: Decimal,
    pub capital_growth: Decimal,
}

impl Investments {
    /// The capital growth of a statement: its balance minus the balance of the
    /// previous statement of the account minus its net deposits. An account is
    /// taken to start at 0.00, so its first statement has no previous balance to
    /// subtract (CSD-005 REQ-23).
    #[must_use]
    pub fn capital_growth(&self, statement: &Statement) -> Decimal {
        let previous = self
            .previous_balance(statement.account_id, statement.period)
            .unwrap_or_else(zero);
        with_two_decimals(statement.balance.value() - previous - statement.net_deposits.value())
    }

    /// Calculates the other one of balance and capital growth for the
    /// statement form, from the value the user typed in and the net deposits.
    /// Nothing is stored (CSD-005 REQ-24, REQ-25).
    ///
    /// # Errors
    /// Returns `InvestmentAccountNotFound`, `InvestmentPeriodMismatch`, or the
    /// net deposits and balance or growth errors of REQ-22, REQ-9 and REQ-25.
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
        let previous = self
            .previous_balance(account_id, period)
            .unwrap_or_else(zero);
        match input {
            CalculationInput::Balance => {
                let balance = Balance::parse(value)?.value();
                Ok(StatementCalculation {
                    balance,
                    capital_growth: with_two_decimals(balance - previous - net_deposits),
                })
            }
            CalculationInput::Growth => {
                let growth = parse_growth(value)?;
                let balance = Balance::new(previous + net_deposits + growth)?;
                Ok(StatementCalculation {
                    balance: balance.value(),
                    capital_growth: growth,
                })
            }
        }
    }
}
