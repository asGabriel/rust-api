use std::{collections::HashMap, sync::Mutex};

use async_trait::async_trait;
use http_error::HttpResult;
use uuid::Uuid;

use crate::modules::matchmaking::domain::session::Session;

#[async_trait]
pub trait SessionRepository {
    async fn insert(&self, session: Session) -> HttpResult<Session>;

    async fn list(&self) -> HttpResult<Vec<Session>>;

    async fn get(&self, id: &Uuid) -> HttpResult<Option<Session>>;

    async fn update(&self, session: Session) -> HttpResult<Session>;
}

pub type DynSessionRepository = dyn SessionRepository + Send + Sync;

/// Process-memory cache repository, no database persistence.
/// Lets the pairing logic be tested before a migration exists.
#[derive(Default)]
pub struct InMemorySessionRepository {
    sessions: Mutex<HashMap<Uuid, Session>>,
}

impl InMemorySessionRepository {
    pub fn new() -> Self {
        Self::default()
    }
}

#[async_trait]
impl SessionRepository for InMemorySessionRepository {
    async fn insert(&self, session: Session) -> HttpResult<Session> {
        let mut sessions = self
            .sessions
            .lock()
            .expect("session repository lock poisoned");
        sessions.insert(*session.id(), session.clone());

        Ok(session)
    }

    async fn list(&self) -> HttpResult<Vec<Session>> {
        let sessions = self
            .sessions
            .lock()
            .expect("session repository lock poisoned");

        Ok(sessions.values().cloned().collect())
    }

    async fn get(&self, id: &Uuid) -> HttpResult<Option<Session>> {
        let sessions = self
            .sessions
            .lock()
            .expect("session repository lock poisoned");

        Ok(sessions.get(id).cloned())
    }

    async fn update(&self, session: Session) -> HttpResult<Session> {
        let mut sessions = self
            .sessions
            .lock()
            .expect("session repository lock poisoned");
        sessions.insert(*session.id(), session.clone());

        Ok(session)
    }
}
