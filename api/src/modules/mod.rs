use std::sync::Arc;

use crate::modules::{chat_bot::ChatBotState, finance_manager::FinanceManagerState, worker::WorkerState};

pub mod chat_bot;
pub mod finance_manager;
pub mod worker;

pub mod routes;

#[derive(Clone)]
pub struct AppState {
    pub worker_state: Arc<WorkerState>,
    pub finance_manager_state: Arc<FinanceManagerState>,
    pub telegram_bot_state: Arc<ChatBotState>,
}
