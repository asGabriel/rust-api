use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{delete, post},
    Json, Router,
};
use http_error::HttpResult;
use uuid::Uuid;

use crate::modules::{
    finance::{domain::payment::PaymentFilters, handler::payment::use_cases::CreatePaymentRequest},
    routes::AppState,
};

// Routes are unauthenticated for now, so every request acts as this single client/user.
const ANONYMOUS_ID: Uuid = Uuid::nil();

pub fn configure_routes() -> Router<AppState> {
    Router::new().nest(
        "/payment",
        Router::new()
            .route("/", post(create_payment))
            .route("/list", post(list_payments))
            .route("/{payment_id}", delete(refund_payment)),
    )
}

async fn create_payment(
    state: State<AppState>,
    Json(request): Json<CreatePaymentRequest>,
) -> HttpResult<impl IntoResponse> {
    let payment = state
        .finance_state
        .payment_handler
        .create_payment(ANONYMOUS_ID, request)
        .await?;

    Ok(Json(payment))
}

async fn list_payments(
    state: State<AppState>,
    Json(filters): Json<PaymentFilters>,
) -> HttpResult<impl IntoResponse> {
    let payments = state
        .finance_state
        .payment_handler
        .list_payments(ANONYMOUS_ID, &filters)
        .await?;

    Ok(Json(payments))
}

async fn refund_payment(
    state: State<AppState>,
    Path(payment_id): Path<Uuid>,
) -> HttpResult<impl IntoResponse> {
    state
        .finance_state
        .payment_handler
        .refund_payment(ANONYMOUS_ID, ANONYMOUS_ID, payment_id)
        .await?;

    Ok(StatusCode::OK)
}
