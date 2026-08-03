use std::sync::Arc;

use async_trait::async_trait;
use http_error::{HttpError, HttpResult};
use uuid::Uuid;

use crate::modules::matchmaking::{
    domain::session::Session,
    handler::session::use_cases::{CreateSessionRequest, UpdateSessionRequest},
    repository::session::DynSessionRepository,
};

#[async_trait]
pub trait SessionHandler {
    async fn create_session(&self, request: CreateSessionRequest) -> HttpResult<Session>;

    async fn list_sessions(&self) -> HttpResult<Vec<Session>>;

    async fn get_session(&self, id: Uuid) -> HttpResult<Session>;

    async fn update_session(&self, id: Uuid, request: UpdateSessionRequest) -> HttpResult<Session>;
}

pub type DynSessionHandler = dyn SessionHandler + Send + Sync;

#[derive(Clone)]
pub struct SessionHandlerImpl {
    pub session_repository: Arc<DynSessionRepository>,
}

#[async_trait]
impl SessionHandler for SessionHandlerImpl {
    async fn create_session(&self, request: CreateSessionRequest) -> HttpResult<Session> {
        let session = Session::new(
            request.date,
            request.settings.unwrap_or_default(),
            request.available_courts,
        );

        self.session_repository.insert(session).await
    }

    async fn list_sessions(&self) -> HttpResult<Vec<Session>> {
        self.session_repository.list().await
    }

    async fn get_session(&self, id: Uuid) -> HttpResult<Session> {
        self.session_repository
            .get(&id)
            .await?
            .ok_or_else(|| Box::new(HttpError::not_found("Session", id)))
    }

    async fn update_session(&self, id: Uuid, request: UpdateSessionRequest) -> HttpResult<Session> {
        let mut session = self.get_session(id).await?;

        if let Some(date) = request.date {
            session.set_date(date);
        }

        if let Some(settings) = request.settings {
            session.set_settings(settings);
        }

        if let Some(available_courts) = request.available_courts {
            session.set_available_courts(available_courts);
        }

        if let Some(player_ids) = request.player_ids {
            session.set_player_ids(player_ids);
        }

        self.session_repository.update(session).await
    }
}

pub mod use_cases {
    use chrono::NaiveDate;
    use serde::{Deserialize, Serialize};
    use uuid::Uuid;

    use crate::modules::matchmaking::domain::session::SessionSettings;

    #[derive(Debug, Clone, Deserialize, Serialize)]
    #[serde(rename_all = "camelCase")]
    pub struct CreateSessionRequest {
        pub date: NaiveDate,
        pub available_courts: u8,
        pub settings: Option<SessionSettings>,
    }

    #[derive(Debug, Clone, Default, Deserialize, Serialize)]
    #[serde(rename_all = "camelCase")]
    pub struct UpdateSessionRequest {
        pub date: Option<NaiveDate>,
        pub available_courts: Option<u8>,
        pub settings: Option<SessionSettings>,
        pub player_ids: Option<Vec<Uuid>>,
    }
}
