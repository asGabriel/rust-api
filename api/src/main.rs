use std::sync::Arc;

use api::modules::{
    auth::{handler::AuthHandlerImpl, repository::user::UserRepositoryImpl, AuthState},
    finance_manager::{
        handler::{
            debt::{invoice::InvoiceHandlerImpl, DebtHandlerImpl},
            financial_instrument::FinancialInstrumentHandlerImpl,
            income::IncomeHandlerImpl,
            payment::PaymentHandlerImpl,
            pubsub::PubSubHandlerImpl,
        },
        repository::{
            debt::{
                installment::InstallmentRepositoryImpl, invoice::InvoiceRepositoryImpl,
                DebtRepositoryImpl,
            },
            financial_instrument::FinancialInstrumentRepositoryImpl,
            income::IncomeRepositoryImpl,
            payment::PaymentRepositoryImpl,
            recurrence::RecurrenceRepositoryImpl,
        },
        FinanceManagerState,
    },
    matchmaking::{
        handler::{
            matches::MatchHandlerImpl, player::PlayerHandlerImpl, session::SessionHandlerImpl,
            team::TeamHandlerImpl,
        },
        repository::{
            matches::MatchRepositoryImpl, player::PlayerRepositoryImpl,
            session::SessionRepositoryImpl, team::TeamRepositoryImpl,
        },
        MatchmakingState,
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

    let pubsub = build_pubsub(pool);

    // Build handlers
    let payment_handler = build_payment_handler(pool, &pubsub);
    let debt_handler = build_debt_handler(pool);
    let invoice_handler = build_invoice_handler(pool);
    let financial_instrument_handler = build_financial_instrument_handler(pool);
    let income_handler = build_income_handler(pool);

    // Build states
    let finance_manager_state = FinanceManagerState {
        payment_handler: Arc::new(payment_handler.clone()),
        debt_handler: Arc::new(debt_handler.clone()),
        invoice_handler: Arc::new(invoice_handler),
        financial_instrument_handler: Arc::new(financial_instrument_handler.clone()),
        income_handler: Arc::new(income_handler.clone()),
    };

    let auth_handler = build_auth_handler(pool);
    let auth_state = AuthState {
        auth_handler: Arc::new(auth_handler),
    };

    let matchmaking_state = build_matchmaking_state(pool);

    let app_state = AppState {
        finance_manager_state: Arc::new(finance_manager_state),
        auth_state: Arc::new(auth_state),
        matchmaking_state: Arc::new(matchmaking_state),
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
        financial_instrument_repository: Arc::new(FinancialInstrumentRepositoryImpl::new(pool)),
        pubsub: Arc::new(pubsub.clone()),
    }
}

fn build_debt_handler(pool: &Pool<Postgres>) -> DebtHandlerImpl {
    DebtHandlerImpl {
        debt_repository: Arc::new(DebtRepositoryImpl::new(pool)),
        installment_repository: Arc::new(InstallmentRepositoryImpl::new(pool)),
        recurrence_repository: Arc::new(RecurrenceRepositoryImpl::new(pool)),
    }
}

fn build_invoice_handler(pool: &Pool<Postgres>) -> InvoiceHandlerImpl {
    InvoiceHandlerImpl {
        invoice_repository: Arc::new(InvoiceRepositoryImpl::new(pool)),
    }
}

fn build_financial_instrument_handler(pool: &Pool<Postgres>) -> FinancialInstrumentHandlerImpl {
    FinancialInstrumentHandlerImpl {
        financial_instrument_repository: Arc::new(FinancialInstrumentRepositoryImpl::new(pool)),
    }
}

fn build_income_handler(pool: &Pool<Postgres>) -> IncomeHandlerImpl {
    IncomeHandlerImpl {
        income_repository: Arc::new(IncomeRepositoryImpl::new(pool)),
    }
}

fn build_matchmaking_state(pool: &Pool<Postgres>) -> MatchmakingState {
    let player_repository = Arc::new(PlayerRepositoryImpl::new(pool));
    let session_repository = Arc::new(SessionRepositoryImpl::new(pool));
    let team_repository = Arc::new(TeamRepositoryImpl::new(pool));
    let match_repository = Arc::new(MatchRepositoryImpl::new(pool));

    let team_handler = Arc::new(TeamHandlerImpl {
        team_repository,
        session_repository: session_repository.clone(),
        player_repository: player_repository.clone(),
        match_repository: match_repository.clone(),
    });

    MatchmakingState {
        player_handler: Arc::new(PlayerHandlerImpl { player_repository }),
        session_handler: Arc::new(SessionHandlerImpl {
            session_repository: session_repository.clone(),
        }),
        team_handler: team_handler.clone(),
        match_handler: Arc::new(MatchHandlerImpl {
            match_repository,
            session_repository,
            team_handler,
        }),
    }
}

fn build_auth_handler(pool: &Pool<Postgres>) -> AuthHandlerImpl {
    let jwt_secret = std::env::var("JWT_SECRET").expect("JWT_SECRET must be set");

    AuthHandlerImpl {
        user_repository: Arc::new(UserRepositoryImpl::new(pool)),
        jwt_secret,
    }
}
