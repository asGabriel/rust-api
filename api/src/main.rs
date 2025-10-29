use std::sync::Arc;

use api::modules::{
    chat_bot::{gateway::TelegramGateway, handler::ChatBotHandlerImpl, ChatBotState},
    finance_manager::{
        handler::{
            account::AccountHandlerImpl, debt::DebtHandlerImpl, payment::PaymentHandlerImpl,
            pubsub::PubSubHandlerImpl, recurrence::RecurrenceHandlerImpl,
        },
        repository::{
            account::AccountRepositoryImpl, debt::DebtRepositoryImpl,
            payment::PaymentRepositoryImpl, recurrence::RecurrenceRepositoryImpl,
        },
        FinanceManagerState,
    },
    routes::{self, AppState},
};
use axum::Router;
use database::DbPool;
use sqlx::{Pool, Postgres};

#[tokio::main]
async fn main() {
    let db_conection = DbPool::new().await;
    let pool = db_conection.get_connection();

    // Build handlers
    let payment_handler = build_payment_handler(pool);
    let debt_handler = build_debt_handler(pool);
    let account_handler = build_account_handler(pool);
    let recurrence_handler = build_recurrence_handler(pool);

    // Build states
    let finance_manager_state = FinanceManagerState {
        payment_handler: Arc::new(payment_handler.clone()),
        debt_handler: Arc::new(debt_handler.clone()),
        account_handler: Arc::new(account_handler.clone()),
        recurrence_handler: Arc::new(recurrence_handler.clone()),
    };

    let chat_bot_state = ChatBotState {
        chat_bot_handler: Arc::new(ChatBotHandlerImpl {
            payment_handler: Arc::new(payment_handler.clone()),
            debt_handler: Arc::new(debt_handler.clone()),
            account_handler: Arc::new(account_handler.clone()),
            telegram_gateway: TelegramGateway::new(),
        }),
        payment_handler: Arc::new(payment_handler.clone()),
        telegram_gateway: TelegramGateway::new(),
    };

    let app_state = AppState {
        finance_manager_state: Arc::new(finance_manager_state),
        chat_bot_state: Arc::new(chat_bot_state),
    };

    let app: Router = routes::configure_services().with_state(app_state);

    let port = std::env::var("PORT").unwrap_or_else(|_| "8080".to_string());
    let url = format!("0.0.0.0:{}", port);

    let listener = tokio::net::TcpListener::bind(url).await.unwrap();
    axum::serve(listener, app).await.unwrap();

    db_conection.close().await;
}

fn build_payment_handler(pool: &Pool<Postgres>) -> PaymentHandlerImpl {
    PaymentHandlerImpl {
        payment_repository: Arc::new(PaymentRepositoryImpl::new(pool)),
        debt_repository: Arc::new(DebtRepositoryImpl::new(pool)),
        pubsub: Arc::new(PubSubHandlerImpl {
            debt_repository: Arc::new(DebtRepositoryImpl::new(pool)),
        }),
    }
}

fn build_debt_handler(pool: &Pool<Postgres>) -> DebtHandlerImpl {
    DebtHandlerImpl {
        debt_repository: Arc::new(DebtRepositoryImpl::new(pool)),
        account_repository: Arc::new(AccountRepositoryImpl::new(pool)),
        payment_repository: Arc::new(PaymentRepositoryImpl::new(pool)),
        pubsub: Arc::new(PubSubHandlerImpl {
            debt_repository: Arc::new(DebtRepositoryImpl::new(pool)),
        }),
    }
}

fn build_account_handler(pool: &Pool<Postgres>) -> AccountHandlerImpl {
    AccountHandlerImpl {
        account_repository: Arc::new(AccountRepositoryImpl::new(pool)),
    }
}

fn build_recurrence_handler(pool: &Pool<Postgres>) -> RecurrenceHandlerImpl {
    RecurrenceHandlerImpl {
        recurrence_repository: Arc::new(RecurrenceRepositoryImpl::new(pool)),
        account_repository: Arc::new(AccountRepositoryImpl::new(pool)),
    }
}
