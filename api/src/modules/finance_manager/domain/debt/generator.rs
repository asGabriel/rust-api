use serde::{Deserialize, Serialize};

use crate::modules::finance_manager::{domain::debt::Debt, handler::debt::CreateDebtRequest};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DebtGenerator {
    pub request: CreateDebtRequest,
}

impl DebtGenerator {
    /// Generates a debt from a create debt request
    pub fn generate_debt_from_request(&self) -> Debt {
        Debt::new(
            // self.request.account_id,
            uuid::Uuid::new_v4(),
            self.request.description.clone(),
            self.request.total_amount,
            self.request.paid_amount,
            self.request.discount_amount,
            self.request.due_date,
        )
    }
}
