use std::sync::Arc;

use api::modules::{
    chat_bot::ChatBotState, finance_manager::{
        handler::{
            account::AccountHandlerImpl, debt::DebtHandlerImpl, payment::PaymentHandlerImpl,
        },
        repository::{
            account::AccountRepositoryImpl, debt::DebtRepositoryImpl,
            payment::PaymentRepositoryImpl,
        },
        FinanceManagerState,
    }, worker::WorkerState, routes::{self}, AppState
};
use axum::Router;
use database::DbPool;

#[tokio::main]
async fn main() {
    let db_conection = DbPool::new().await;
    let pool = db_conection.get_connection();
    
    let worker_state = WorkerState::new(pool.clone()).start();

    let finance_manager_state = FinanceManagerState {
        payment_handler: Arc::new(PaymentHandlerImpl {
            payment_repository: Arc::new(PaymentRepositoryImpl::new(pool)),
        }),
        debt_handler: Arc::new(DebtHandlerImpl {
            debt_repository: Arc::new(DebtRepositoryImpl::new(pool)),
            account_repository: Arc::new(AccountRepositoryImpl::new(pool)),
        }),
        account_handler: Arc::new(AccountHandlerImpl {
            worker_state: worker_state.clone(),
            account_repository: Arc::new(AccountRepositoryImpl::new(pool)),
        }),
    };

    let telegram_bot_state = ChatBotState {
        payment_handler: Arc::new(PaymentHandlerImpl {
            payment_repository: Arc::new(PaymentRepositoryImpl::new(pool)),
        }),
    };


    let app_state = AppState {
        worker_state,
        finance_manager_state: Arc::new(finance_manager_state),
        telegram_bot_state: Arc::new(telegram_bot_state),
    };

    let app: Router = routes::configure_services().with_state(app_state);

    let port = std::env::var("PORT").expect("Could not fetch port data.");
    let url = format!("0.0.0.0:{}", port);

    let listener = tokio::net::TcpListener::bind(url).await.unwrap();
    axum::serve(listener, app).await.unwrap();

    db_conection.close().await;
}
