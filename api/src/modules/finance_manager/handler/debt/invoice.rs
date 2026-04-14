use std::sync::Arc;

use async_trait::async_trait;
use http_error::{ext::OptionHttpExt, HttpResult};
use uuid::Uuid;

use crate::modules::finance_manager::{
    domain::debt::invoice::{
        filters::InvoiceFilters,
        reference_month_as_date,
        use_cases::{CreateInvoiceRequest, ListInvoicesFilters, ManageInvoiceDebts},
        Invoice,
    },
    repository::debt::invoice::DynInvoiceRepository,
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
        request: ListInvoicesFilters,
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
        request: ListInvoicesFilters,
    ) -> HttpResult<Vec<Invoice>> {
        let filters = InvoiceFilters::new(client_id)
            .with_related_debt_ids(request.related_debt_ids)
            .with_reference_date(request.reference_date.map(reference_month_as_date));

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

        invoice.apply_changes(&request);

        self.invoice_repository.update(invoice).await?;

        Ok(())
    }
}
