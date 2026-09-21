use async_trait::async_trait;
use chrono::Datelike;
use http_error::{ext::OptionHttpExt, HttpError, HttpResult};
use uuid::Uuid;

use util::DeletedBy;

use crate::modules::finance::{
    domain::debt::{Debt, DebtFilters},
    handler::debt::use_cases::{CreateDebtRequest, UpdateDebtRequest},
    repository::debt::DynDebtRepository,
};
use std::sync::Arc;

pub type DynDebtHandler = dyn DebtHandler + Send + Sync;

#[async_trait]
pub trait DebtHandler {
    async fn register_new_debt(
        &self,
        client_id: Uuid,
        request: CreateDebtRequest,
    ) -> HttpResult<Debt>;

    async fn list_debts(&self, client_id: Uuid, filters: &DebtFilters) -> HttpResult<Vec<Debt>>;

    async fn update_debt(
        &self,
        client_id: Uuid,
        debt_id: Uuid,
        request: UpdateDebtRequest,
    ) -> HttpResult<Debt>;

    async fn soft_delete_debt(
        &self,
        client_id: Uuid,
        user_id: Uuid,
        debt_id: Uuid,
    ) -> HttpResult<()>;
}

#[derive(Clone)]
pub struct DebtHandlerImpl {
    pub debt_repository: Arc<DynDebtRepository>,
}

#[async_trait]
impl DebtHandler for DebtHandlerImpl {
    async fn register_new_debt(
        &self,
        client_id: Uuid,
        request: CreateDebtRequest,
    ) -> HttpResult<Debt> {
        request.validate()?;

        let mut debt = Debt::new(
            client_id,
            request.description,
            request.total_amount,
            request.paid_amount,
            request.due_date,
            request.category,
            request.expense_type,
            request.tags,
            request.installment_count,
        );

        if debt.has_installment_count() {
            let due_day = request.due_date.day();
            let children = debt.generate_installment_children(due_day)?;

            let mut group = vec![debt];
            group.extend(children);

            let mut inserted = self.debt_repository.insert_many(group).await?;
            Ok(inserted.remove(0))
        } else {
            self.debt_repository.insert(debt).await
        }
    }

    async fn list_debts(&self, client_id: Uuid, filters: &DebtFilters) -> HttpResult<Vec<Debt>> {
        let built = DebtFilters::new(client_id)
            .with_optional_statuses(filters.statuses().clone())
            .with_optional_ids(filters.ids().clone())
            .with_optional_start_date(*filters.start_date())
            .with_optional_end_date(*filters.end_date())
            .with_optional_category_names(filters.category_names().clone())
            .with_optional_parent_id(*filters.parent_id())
            .with_include_children(*filters.include_children());

        self.debt_repository.list(&built).await
    }

    async fn update_debt(
        &self,
        client_id: Uuid,
        debt_id: Uuid,
        request: UpdateDebtRequest,
    ) -> HttpResult<Debt> {
        let mut debt = self
            .debt_repository
            .get_by_id(&debt_id)
            .await?
            .or_not_found("debt", debt_id.to_string())?;

        if debt.client_id() != &client_id {
            return Err(Box::new(HttpError::forbidden(
                "You don't have permission to update this debt",
            )));
        }

        if debt.is_installment_child() {
            return Err(Box::new(HttpError::bad_request(
                "Installment debts are frozen at generation and cannot be edited directly",
            )));
        }

        if let Some(category) = request.category {
            debt.set_category(category);
        }
        if let Some(expense_type) = request.expense_type {
            debt.set_expense_type(expense_type);
        }
        if let Some(tags) = request.tags {
            debt.set_tags(tags);
        }
        if let Some(description) = request.description {
            debt.set_description(description);
        }
        if let Some(due_date) = request.due_date {
            debt.set_due_date(due_date);
        }

        self.debt_repository.update(debt).await
    }

    async fn soft_delete_debt(
        &self,
        client_id: Uuid,
        user_id: Uuid,
        debt_id: Uuid,
    ) -> HttpResult<()> {
        let debt = self
            .debt_repository
            .get_by_id(&debt_id)
            .await?
            .or_not_found("debt", debt_id.to_string())?;

        if debt.client_id() != &client_id {
            return Err(Box::new(HttpError::forbidden(
                "You don't have permission to delete this debt",
            )));
        }

        self.debt_repository
            .soft_delete_cascade(client_id, debt_id, DeletedBy::new(user_id))
            .await
    }
}

pub mod use_cases {
    use chrono::NaiveDate;
    use http_error::{HttpError, HttpResult};
    use rust_decimal::Decimal;
    use serde::{Deserialize, Serialize};

    use crate::modules::finance::domain::debt::{DebtCategory, ExpenseType};

    #[derive(Debug, Clone, Serialize, Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct CreateDebtRequest {
        pub category: Option<DebtCategory>,
        pub expense_type: Option<ExpenseType>,
        pub tags: Option<Vec<String>>,
        pub description: String,
        pub due_date: NaiveDate,
        pub total_amount: Decimal,
        pub paid_amount: Option<Decimal>,
        pub installment_count: Option<i32>,
    }

    impl CreateDebtRequest {
        pub fn validate(&self) -> HttpResult<()> {
            if self.invalid_total_amount() {
                return Err(Box::new(HttpError::bad_request(
                    "Total amount must be greater than zero",
                )));
            }

            if let Some(installment_count) = self.installment_count {
                if installment_count < 2 {
                    return Err(Box::new(HttpError::bad_request(
                        "Installment count must be at least 2",
                    )));
                }

                if self
                    .paid_amount
                    .is_some_and(|amount| amount > Decimal::ZERO)
                {
                    return Err(Box::new(HttpError::bad_request(
                        "An installment debt cannot be created with a paid amount — only the individual installments (children) are payable",
                    )));
                }
            }

            Ok(())
        }

        fn invalid_total_amount(&self) -> bool {
            self.total_amount <= Decimal::ZERO
        }
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct UpdateDebtRequest {
        pub category: Option<DebtCategory>,
        pub expense_type: Option<ExpenseType>,
        pub tags: Option<Vec<String>>,
        pub description: Option<String>,
        pub due_date: Option<NaiveDate>,
    }
}
