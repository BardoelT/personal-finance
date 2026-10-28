//! One statement of an account, and the form values for "copy previous" (CSD-005 REQ-6, REQ-13).

use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

use crate::domain::error::DomainError;
use crate::domain::money::zero;

use super::{Balance, InvestmentAccountId, Investments, NetDeposits, Note, Period};

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

/// Form values for "copy previous month" or "copy previous year" (CSD-005
/// REQ-13): the note is never copied and the net deposits start at `0.00`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct StatementPrefill {
    pub balance: Option<Decimal>,
    pub net_deposits: Decimal,
}

impl Investments {
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
}
