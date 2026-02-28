use axum::{
    extract::{Path, State},
    http::HeaderMap,
    response::IntoResponse,
    routing::{delete, post},
    Json, Router,
};
use http_error::HttpResult;
use uuid::Uuid;

use crate::modules::{
    finance_manager::{
        handler::payment::use_cases::CreatePaymentRequest,
        repository::payment::use_cases::PaymentFilters,
    },
    routes::AppState,
};

pub fn configure_routes() -> Router<AppState> {
    Router::new().nest(
        "/payment",
        Router::new()
            .route("/", post(create_payment))
            .route("/list", post(list_payments))
            .route("/{id}/refund", delete(refund_payment)),
    )
}

async fn create_payment(
    state: State<AppState>,
    Json(request): Json<CreatePaymentRequest>,
) -> HttpResult<impl IntoResponse> {
    let payment = state
        .finance_manager_state
        .payment_handler
        .create_payment(request)
        .await?;

    Ok(Json(payment))
}

async fn list_payments(
    state: State<AppState>,
    headers: HeaderMap,
    Json(filters): Json<PaymentFilters>,
) -> HttpResult<impl IntoResponse> {
    let user = state.auth_state.auth_handler.authenticate(&headers).await?;
    let payments = state
        .finance_manager_state
        .payment_handler
        .list_payments(*user.client_id(), filters)
        .await?;

    Ok(Json(payments))
}

async fn refund_payment(
    state: State<AppState>,
    headers: HeaderMap,
    Path(id): Path<Uuid>,
) -> HttpResult<impl IntoResponse> {
    let user = state.auth_state.auth_handler.authenticate(&headers).await?;
    state
        .finance_manager_state
        .payment_handler
        .refund_payment(*user.client_id(), id)
        .await?;

    Ok(Json(
        serde_json::json!({ "message": "Payment refunded successfully" }),
    ))
}
