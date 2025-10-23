use async_trait::async_trait;
use http_error::HttpResult;

use crate::{
    TelegramApiClient,
    domain::send_message::{SendMessageRequest, SendMessageResponse},
};

#[async_trait]
pub trait TelegramApiGateway {
    async fn send_message(&self, request: SendMessageRequest) -> HttpResult<SendMessageResponse>;
}

#[async_trait]
impl TelegramApiGateway for TelegramApiClient {
    async fn send_message(&self, request: SendMessageRequest) -> HttpResult<SendMessageResponse> {
        let response = self
            .client
            .post(format!("{}/sendMessage", self.host))
            .json(&request)
            .send()
            .await?;

        let result = response.json::<SendMessageResponse>().await?;

        Ok(result)
    }
}
