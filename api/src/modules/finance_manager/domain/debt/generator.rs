use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::modules::finance_manager::{domain::debt::Debt, handler::debt::CreateDebtRequest};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DebtGenerator;

impl DebtGenerator {
    /// Generates a debt from a create debt request
    pub fn generate_debt_from_request(request: CreateDebtRequest, account_id: Uuid) -> Debt {
        Debt::new(
            account_id,
            request.description.clone(),
            request.total_amount,
            request.paid_amount,
            request.discount_amount,
            request.due_date,
        )
    }
}
