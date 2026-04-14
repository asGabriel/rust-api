use std::collections::HashSet;

use chrono::{DateTime, Utc};
use http_error::{HttpError, HttpResult};
use serde::{Deserialize, Serialize};
use util::{getters, DeletedBy};
use uuid::Uuid;

use crate::modules::finance_manager::domain::debt::invoice::use_cases::{
    CreateInvoiceRequest, ManageInvoiceDebts,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Invoice {
    id: Uuid,
    client_id: Uuid,

    /// The name of the invoice
    name: String,
    /// The debts that are related to the invoice.
    #[serde(default)]
    related_debt_ids: HashSet<Uuid>,

    created_at: DateTime<Utc>,
    updated_at: Option<DateTime<Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    deleted_by: Option<DeletedBy>,
}

getters! {
    Invoice {
        id: Uuid,
        client_id: Uuid,
        name: String,
        related_debt_ids: HashSet<Uuid>,
        created_at: DateTime<Utc>,
        updated_at: Option<DateTime<Utc>>,
        deleted_by: Option<DeletedBy>,
    }
}

impl From<&sqlx::postgres::PgRow> for Invoice {
    fn from(row: &sqlx::postgres::PgRow) -> Self {
        use sqlx::{types::Json, Row};

        Self {
            id: row.get("id"),
            client_id: row.get("client_id"),
            name: row.get("name"),
            related_debt_ids: row
                .get::<Vec<Uuid>, _>("related_debt_ids")
                .into_iter()
                .collect(),
            created_at: row.get("created_at"),
            updated_at: row.get("updated_at"),
            deleted_by: row
                .get::<Option<Json<DeletedBy>>, _>("deleted_by")
                .map(|j| j.0),
        }
    }
}

#[derive(Copy, Clone)]
enum ExpectedDebtLink {
    MustBeLinked,
    MustNotBeLinked,
}

/// Falhas de validação ao vincular dívidas à fatura; mensagens únicas para API.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum InvoiceValidationError {
    DebtNotFoundInInvoice,
    DebtAlreadyInInvoice,
}

impl InvoiceValidationError {
    fn message(self) -> &'static str {
        match self {
            Self::DebtNotFoundInInvoice => "Debt not found in invoice",
            Self::DebtAlreadyInInvoice => "Debt already in invoice",
        }
    }
}

impl From<InvoiceValidationError> for HttpError {
    fn from(value: InvoiceValidationError) -> Self {
        HttpError::bad_request(value.message())
    }
}

impl Invoice {
    pub fn from_request(request: CreateInvoiceRequest, client_id: Uuid) -> Self {
        Self {
            id: Uuid::new_v4(),
            client_id,
            name: request.name,
            related_debt_ids: HashSet::new(),
            created_at: Utc::now(),
            updated_at: None,
            deleted_by: None,
        }
    }

    pub fn belongs_to_client(&self, client_id: Uuid) -> HttpResult<()> {
        if *self.client_id() != client_id {
            return Err(Box::new(HttpError::forbidden(
                "You don't have permission to manage this invoice",
            )));
        }

        Ok(())
    }

    pub fn validate_changes(&self, request: &ManageInvoiceDebts) -> HttpResult<()> {
        if request.is_empty() {
            return Err(Box::new(HttpError::bad_request("No changes to apply")));
        }

        self.validate_debt_ids(&request.add_debt_ids, ExpectedDebtLink::MustNotBeLinked)?;
        self.validate_debt_ids(&request.remove_debt_ids, ExpectedDebtLink::MustBeLinked)?;

        Ok(())
    }

    fn validate_debt_ids(&self, debt_ids: &[Uuid], expected: ExpectedDebtLink) -> HttpResult<()> {
        for &debt_id in debt_ids {
            let is_linked = self.related_debt_ids.contains(&debt_id);
            match (expected, is_linked) {
                (ExpectedDebtLink::MustBeLinked, false) => {
                    return Err(Box::new(
                        InvoiceValidationError::DebtNotFoundInInvoice.into(),
                    ));
                }
                (ExpectedDebtLink::MustNotBeLinked, true) => {
                    return Err(Box::new(
                        InvoiceValidationError::DebtAlreadyInInvoice.into(),
                    ));
                }
                _ => {}
            }
        }

        Ok(())
    }

    pub fn apply_changes(&mut self, request: &ManageInvoiceDebts) {
        self.add_related_debt_ids(&request.add_debt_ids);
        self.remove_related_debt_ids(&request.remove_debt_ids);
        self.updated_at = Some(Utc::now());
    }

    fn add_related_debt_ids(&mut self, ids: &[Uuid]) {
        self.related_debt_ids.extend(ids.iter().copied());
    }

    fn remove_related_debt_ids(&mut self, ids: &[Uuid]) {
        if ids.is_empty() {
            return;
        }
        self.related_debt_ids.retain(|id| !ids.contains(id));
    }
}

pub mod use_cases {
    use serde::{Deserialize, Serialize};
    use uuid::Uuid;

    #[derive(Debug, Clone, Deserialize, Serialize)]
    #[serde(rename_all = "camelCase")]
    pub struct CreateInvoiceRequest {
        pub name: String,
    }

    #[derive(Debug, Clone, Deserialize, Serialize, Default)]
    #[serde(rename_all = "camelCase")]
    pub struct ListInvoicesFilters {
        #[serde(default)]
        pub related_debt_ids: Option<Vec<Uuid>>,
    }

    #[derive(Debug, Clone, Deserialize, Serialize)]
    #[serde(rename_all = "camelCase")]
    pub struct ManageInvoiceDebts {
        #[serde(default)]
        pub add_debt_ids: Vec<Uuid>,
        #[serde(default)]
        pub remove_debt_ids: Vec<Uuid>,
    }

    impl ManageInvoiceDebts {
        pub fn is_empty(&self) -> bool {
            self.add_debt_ids.is_empty() && self.remove_debt_ids.is_empty()
        }
    }
}

pub mod filters {
    use serde::{Deserialize, Serialize};
    use uuid::Uuid;

    #[derive(Debug, Clone, Deserialize, Serialize, Default)]
    #[serde(rename_all = "camelCase")]
    pub struct InvoiceFilters {
        pub client_id: Uuid,
        pub related_debt_ids: Option<Vec<Uuid>>,
    }

    impl InvoiceFilters {
        pub fn new(client_id: Uuid) -> Self {
            Self {
                client_id,
                ..Default::default()
            }
        }

        pub fn with_related_debt_ids(mut self, related_debt_ids: Option<Vec<Uuid>>) -> Self {
            if let Some(ids) = related_debt_ids {
                self.related_debt_ids = Some(ids);
            }
            self
        }
    }
}
