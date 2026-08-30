use std::sync::Arc;

use async_trait::async_trait;
use http_error::{HttpError, HttpResult};
use uuid::Uuid;

use crate::modules::matchmaking::{
    domain::{
        queue::{QueueEntry, SessionQueue},
        session::Session,
    },
    handler::session::use_cases::{CreateSessionRequest, UpdateSessionRequest},
    repository::{queue::DynSessionQueueRepository, session::DynSessionRepository},
};

#[async_trait]
pub trait SessionHandler {
    async fn create_session(&self, request: CreateSessionRequest) -> HttpResult<Session>;

    async fn list_sessions(&self) -> HttpResult<Vec<Session>>;

    async fn get_session(&self, id: Uuid) -> HttpResult<Session>;

    async fn update_session(&self, id: Uuid, request: UpdateSessionRequest) -> HttpResult<Session>;

    /// The session's waiting list, in "who enters next" order.
    async fn list_queue(&self, session_id: Uuid) -> HttpResult<Vec<QueueEntry>>;

    /// Pins / unpins a queued player — pinned players jump to the front of
    /// the list (see "Prioridades" in the matchmaking skill).
    async fn set_pin(
        &self,
        session_id: Uuid,
        player_id: Uuid,
        pinned: bool,
    ) -> HttpResult<QueueEntry>;
}

pub type DynSessionHandler = dyn SessionHandler + Send + Sync;

#[derive(Clone)]
pub struct SessionHandlerImpl {
    pub session_repository: Arc<DynSessionRepository>,
    pub session_queue_repository: Arc<DynSessionQueueRepository>,
}

#[async_trait]
impl SessionHandler for SessionHandlerImpl {
    async fn create_session(&self, request: CreateSessionRequest) -> HttpResult<Session> {
        let session = Session::new(
            request.date,
            request.description,
            request.settings.unwrap_or_default(),
            request.available_courts,
            request.game_mode,
        )?;

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

        if let Some(description) = request.description {
            session.set_description(Some(description));
        }

        if let Some(settings) = request.settings {
            session.set_settings(settings)?;
        }

        if let Some(available_courts) = request.available_courts {
            session.set_available_courts(available_courts);
        }

        if let Some(game_mode) = request.game_mode {
            session.set_game_mode(game_mode)?;
        }

        if let Some(new_player_ids) = request.player_ids {
            let previous: Vec<Uuid> = session.player_ids().clone();
            session.set_player_ids(new_player_ids.clone());
            let session_id = *session.id();

            // Keep the waiting list in step with the roster: newly confirmed
            // players join the queue (0 games), players dropped from the
            // roster leave it. A dropped player who is mid-game stays on
            // their court — only their queue row goes.
            let added: Vec<QueueEntry> = new_player_ids
                .iter()
                .filter(|id| !previous.contains(id))
                .map(|id| QueueEntry::new(session_id, *id, 0))
                .collect();
            self.session_queue_repository.insert_many(&added).await?;

            let removed: Vec<Uuid> = previous
                .into_iter()
                .filter(|id| !new_player_ids.contains(id))
                .collect();
            self.session_queue_repository
                .remove_players(&session_id, &removed)
                .await?;
        }

        self.session_repository.update(session).await
    }

    async fn list_queue(&self, session_id: Uuid) -> HttpResult<Vec<QueueEntry>> {
        let session = self.get_session(session_id).await?;
        let entries = self
            .session_queue_repository
            .list_by_session(&session_id)
            .await?;

        let queue = SessionQueue::new(
            entries,
            *session.game_mode(),
            *session.settings().players_per_team(),
        );
        Ok(queue.ordered().into_iter().cloned().collect())
    }

    async fn set_pin(
        &self,
        session_id: Uuid,
        player_id: Uuid,
        pinned: bool,
    ) -> HttpResult<QueueEntry> {
        self.session_queue_repository
            .set_pinned(&session_id, &player_id, pinned)
            .await?
            .ok_or_else(|| Box::new(HttpError::not_found("Queued player", player_id)))
    }
}

pub mod use_cases {
    use chrono::NaiveDate;
    use serde::{Deserialize, Serialize};
    use uuid::Uuid;

    use crate::modules::matchmaking::domain::session::{GameMode, SessionSettings};

    #[derive(Debug, Clone, Deserialize, Serialize)]
    #[serde(rename_all = "camelCase")]
    pub struct CreateSessionRequest {
        pub date: NaiveDate,
        pub description: Option<String>,
        pub available_courts: u8,
        pub game_mode: GameMode,
        pub settings: Option<SessionSettings>,
    }

    #[derive(Debug, Clone, Default, Deserialize, Serialize)]
    #[serde(rename_all = "camelCase")]
    pub struct UpdateSessionRequest {
        pub date: Option<NaiveDate>,
        pub description: Option<String>,
        pub available_courts: Option<u8>,
        pub game_mode: Option<GameMode>,
        pub settings: Option<SessionSettings>,
        pub player_ids: Option<Vec<Uuid>>,
    }
}
