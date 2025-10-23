use std::sync::Arc;

use async_trait::async_trait;
use http_error::HttpResult;
use telegram_api::{
    domain::send_message::{SendMessageRequest, SendMessageResponse},
    telegram_api::TelegramApiGateway,
    TelegramApiClient,
};

pub type DynTelegramApiGateway = dyn TelegramApiGateway + Send + Sync;

pub struct TelegramGateway {
    client: TelegramApiClient,
}

impl TelegramGateway {
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            client: TelegramApiClient::new(),
        })
    }
}

#[async_trait]
impl TelegramApiGateway for TelegramGateway {
    async fn send_message(&self, request: SendMessageRequest) -> HttpResult<SendMessageResponse> {
        self.client.send_message(request).await
    }
}
