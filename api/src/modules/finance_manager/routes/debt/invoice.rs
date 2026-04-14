use axum::{
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
    routing::{patch, post},
    Json, Router,
};
use http_error::HttpResult;
use uuid::Uuid;

use crate::modules::{
    finance_manager::domain::debt::invoice::use_cases::{
        CreateInvoiceRequest, ListInvoicesFilters, ManageInvoiceDebts,
    },
    routes::AppState,
};

pub fn configure_routes() -> Router<AppState> {
    Router::new().nest(
        "/invoice",
        Router::new()
            .route("/", post(create_invoice))
            .route("/list", post(list_invoices))
            .route("/{invoice_id}", patch(manage_invoice)),
    )
}

async fn create_invoice(
    state: State<AppState>,
    headers: HeaderMap,
    Json(request): Json<CreateInvoiceRequest>,
) -> HttpResult<impl IntoResponse> {
    let user = state.auth_state.auth_handler.authenticate(&headers).await?;

    let invoice = state
        .finance_manager_state
        .invoice_handler
        .create_invoice(*user.client_id(), request)
        .await?;

    Ok(Json(invoice))
}

async fn list_invoices(
    state: State<AppState>,
    headers: HeaderMap,
    Json(request): Json<ListInvoicesFilters>,
) -> HttpResult<impl IntoResponse> {
    let user = state.auth_state.auth_handler.authenticate(&headers).await?;

    let invoices = state
        .finance_manager_state
        .invoice_handler
        .list_invoices(*user.client_id(), request)
        .await?;

    Ok(Json(invoices))
}

async fn manage_invoice(
    state: State<AppState>,
    headers: HeaderMap,
    Path(invoice_id): Path<Uuid>,
    Json(request): Json<ManageInvoiceDebts>,
) -> HttpResult<impl IntoResponse> {
    let user = state.auth_state.auth_handler.authenticate(&headers).await?;

    state
        .finance_manager_state
        .invoice_handler
        .manage_invoice(*user.client_id(), invoice_id, request)
        .await?;

    Ok(StatusCode::OK)
}
