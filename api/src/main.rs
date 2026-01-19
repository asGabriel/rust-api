use std::sync::Arc;

use api::modules::{
    auth::{handler::AuthHandlerImpl, repository::user::UserRepositoryImpl, AuthState},
    chat_bot::{gateway::TelegramGateway, handler::ChatBotHandlerImpl, ChatBotState},
    finance_manager::{
        handler::{
            account::AccountHandlerImpl, debt::DebtHandlerImpl, income::IncomeHandlerImpl,
            payment::PaymentHandlerImpl, pubsub::PubSubHandlerImpl,
            recurrence::RecurrenceHandlerImpl,
        },
        repository::{
            account::AccountRepositoryImpl,
            debt::{installment::InstallmentRepositoryImpl, DebtRepositoryImpl},
            income::IncomeRepositoryImpl,
            payment::PaymentRepositoryImpl,
            recurrence::RecurrenceRepositoryImpl,
        },
        FinanceManagerState,
    },
    routes::{self, AppState},
};
use axum::Router;
use database::DbPool;
use sqlx::{Pool, Postgres};
use uuid::Uuid;

#[tokio::main]
async fn main() {
    let db_conection = DbPool::new().await;
    let pool = db_conection.get_connection();

    let pubsub = build_pubsub(pool);

    // Build handlers
    let payment_handler = build_payment_handler(pool, &pubsub);
    let debt_handler = build_debt_handler(pool, &pubsub);
    let account_handler = build_account_handler(pool);
    let recurrence_handler = build_recurrence_handler(pool);
    let income_handler = build_income_handler(pool);

    // Build states
    let finance_manager_state = FinanceManagerState {
        payment_handler: Arc::new(payment_handler.clone()),
        debt_handler: Arc::new(debt_handler.clone()),
        account_handler: Arc::new(account_handler.clone()),
        recurrence_handler: Arc::new(recurrence_handler.clone()),
        income_handler: Arc::new(income_handler.clone()),
    };

    let telegram_client_id = std::env::var("TELEGRAM_CLIENT_ID")
        .ok()
        .and_then(|s| Uuid::parse_str(&s).ok())
        .unwrap_or(Uuid::nil());

    let chat_bot_state = ChatBotState {
        chat_bot_handler: Arc::new(ChatBotHandlerImpl {
            income_handler: Arc::new(income_handler.clone()),
            payment_handler: Arc::new(payment_handler.clone()),
            debt_handler: Arc::new(debt_handler.clone()),
            account_handler: Arc::new(account_handler.clone()),
            telegram_gateway: TelegramGateway::new(),
            client_id: telegram_client_id,
        }),
        payment_handler: Arc::new(payment_handler.clone()),
        telegram_gateway: TelegramGateway::new(),
    };

    let auth_handler = build_auth_handler(pool);
    let auth_state = AuthState {
        auth_handler: Arc::new(auth_handler),
    };

    let app_state = AppState {
        finance_manager_state: Arc::new(finance_manager_state),
        chat_bot_state: Arc::new(chat_bot_state),
        auth_state: Arc::new(auth_state),
    };

    let app: Router = routes::configure_services().with_state(app_state);

    let port = std::env::var("PORT").unwrap_or_else(|_| "8080".to_string());
    let url = format!("0.0.0.0:{}", port);

    let listener = tokio::net::TcpListener::bind(url).await.unwrap();
    axum::serve(listener, app).await.unwrap();

    db_conection.close().await;
}

fn build_pubsub(pool: &Pool<Postgres>) -> PubSubHandlerImpl {
    PubSubHandlerImpl {
        debt_repository: Arc::new(DebtRepositoryImpl::new(pool)),
        installment_repository: Arc::new(InstallmentRepositoryImpl::new(pool)),
    }
}

fn build_payment_handler(pool: &Pool<Postgres>, pubsub: &PubSubHandlerImpl) -> PaymentHandlerImpl {
    PaymentHandlerImpl {
        payment_repository: Arc::new(PaymentRepositoryImpl::new(pool)),
        debt_repository: Arc::new(DebtRepositoryImpl::new(pool)),
        account_repository: Arc::new(AccountRepositoryImpl::new(pool)),
        pubsub: Arc::new(pubsub.clone()),
    }
}

fn build_debt_handler(pool: &Pool<Postgres>, pubsub: &PubSubHandlerImpl) -> DebtHandlerImpl {
    DebtHandlerImpl {
        debt_repository: Arc::new(DebtRepositoryImpl::new(pool)),
        account_repository: Arc::new(AccountRepositoryImpl::new(pool)),
        payment_repository: Arc::new(PaymentRepositoryImpl::new(pool)),
        installment_repository: Arc::new(InstallmentRepositoryImpl::new(pool)),
        recurrence_repository: Arc::new(RecurrenceRepositoryImpl::new(pool)),
        pubsub: Arc::new(pubsub.clone()),
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

fn build_income_handler(pool: &Pool<Postgres>) -> IncomeHandlerImpl {
    IncomeHandlerImpl {
        income_repository: Arc::new(IncomeRepositoryImpl::new(pool)),
        account_repository: Arc::new(AccountRepositoryImpl::new(pool)),
    }
}

fn build_auth_handler(pool: &Pool<Postgres>) -> AuthHandlerImpl {
    let jwt_secret = std::env::var("JWT_SECRET").expect("JWT_SECRET must be set");

    AuthHandlerImpl {
        user_repository: Arc::new(UserRepositoryImpl::new(pool)),
        jwt_secret,
    }
}
