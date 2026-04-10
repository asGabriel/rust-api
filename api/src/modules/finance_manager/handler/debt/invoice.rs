use std::sync::Arc;

use async_trait::async_trait;
use http_error::{ext::OptionHttpExt, HttpResult};
use uuid::Uuid;

use crate::modules::finance_manager::{
    domain::debt::{
        invoice::{
            filters::InvoiceFilters,
            use_cases::{CreateInvoiceRequest, ManageInvoiceDebts},
            Invoice,
        },
        DebtFilters,
    },
    repository::debt::{invoice::DynInvoiceRepository, DynDebtRepository},
};

#[async_trait]
pub trait InvoiceHandler {
    async fn create_invoice(
        &self,
        client_id: Uuid,
        request: CreateInvoiceRequest,
    ) -> HttpResult<Invoice>;

    async fn list_invoices(
        &self,
        client_id: Uuid,
        filters: InvoiceFilters,
    ) -> HttpResult<Vec<Invoice>>;

    async fn manage_invoice(
        &self,
        client_id: Uuid,
        invoice_id: Uuid,
        request: ManageInvoiceDebts,
    ) -> HttpResult<()>;
}

pub type DynInvoiceHandler = dyn InvoiceHandler + Send + Sync;

#[derive(Clone)]
pub struct InvoiceHandlerImpl {
    pub debt_repository: Arc<DynDebtRepository>,
    pub invoice_repository: Arc<DynInvoiceRepository>,
}

#[async_trait]
impl InvoiceHandler for InvoiceHandlerImpl {
    async fn create_invoice(
        &self,
        client_id: Uuid,
        request: CreateInvoiceRequest,
    ) -> HttpResult<Invoice> {
        let invoice = Invoice::from_request(request, client_id);
        let invoice = self.invoice_repository.insert(invoice).await?;

        Ok(invoice)
    }

    async fn list_invoices(
        &self,
        client_id: Uuid,
        filters: InvoiceFilters,
    ) -> HttpResult<Vec<Invoice>> {
        let filters =
            InvoiceFilters::new(client_id).with_related_debt_ids(filters.related_debt_ids);

        self.invoice_repository.list(&filters).await
    }

    async fn manage_invoice(
        &self,
        client_id: Uuid,
        invoice_id: Uuid,
        request: ManageInvoiceDebts,
    ) -> HttpResult<()> {
        if request.is_empty() {
            return Ok(());
        }

        let mut invoice = self
            .invoice_repository
            .get(&invoice_id)
            .await?
            .or_not_found("invoice", invoice_id.to_string())?;

        invoice.belongs_to_client(client_id)?;
        invoice.validate_changes(&request)?;

        let unique_debt_ids = request.unique_debt_ids_referenced();

        let filters = DebtFilters::new(client_id).with_ids(unique_debt_ids.clone());
        let debts = self.debt_repository.list(&filters).await?;

        invoice.apply_changes(&request, &debts);

        self.invoice_repository.update(invoice).await?;

        Ok(())
    }
}
