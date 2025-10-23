use chrono::Utc;
use serde::{Deserialize, Serialize};

use crate::modules::finance_manager::{
    domain::{debt::Debt, payment::Payment},
    handler::debt::CreateDebtRequest,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DebtGenerator {
    pub request: CreateDebtRequest,
}

impl DebtGenerator {
    /// Generates a debt from a create debt request
    pub fn generate_debt_from_request(&self) -> Debt {
        Debt::new(
            self.request.account_id,
            self.request.description.clone(),
            self.request.total_amount,
            self.request.paid_amount,
            self.request.discount_amount,
            self.request.due_date,
        )
    }

    pub fn is_paid(&self) -> bool {
        self.request.configuration.is_paid.unwrap_or(false)
    }

    pub fn paid(&self, debt: &Debt) -> Payment {
        Payment::new(
            *debt.id(),
            debt.account_id,
            debt.total_amount,
            debt.total_amount,
            self.request.discount_amount,
            Utc::now().date_naive(),
        )
    }
}
