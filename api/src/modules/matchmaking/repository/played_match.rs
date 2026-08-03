use std::{collections::HashMap, sync::Mutex};

use async_trait::async_trait;
use http_error::HttpResult;
use uuid::Uuid;

use crate::modules::matchmaking::domain::played_match::PlayedMatch;

#[async_trait]
pub trait PlayedMatchRepository {
    async fn insert(&self, played_match: PlayedMatch) -> HttpResult<PlayedMatch>;

    async fn list_by_session(&self, session_id: &Uuid) -> HttpResult<Vec<PlayedMatch>>;
}

pub type DynPlayedMatchRepository = dyn PlayedMatchRepository + Send + Sync;

/// Repositório em cache (memória do processo), sem persistência em banco.
/// Serve para viabilizar os testes de sorteio antes de existir a migration.
#[derive(Default)]
pub struct InMemoryPlayedMatchRepository {
    played_matches: Mutex<HashMap<Uuid, PlayedMatch>>,
}

impl InMemoryPlayedMatchRepository {
    pub fn new() -> Self {
        Self::default()
    }
}

#[async_trait]
impl PlayedMatchRepository for InMemoryPlayedMatchRepository {
    async fn insert(&self, played_match: PlayedMatch) -> HttpResult<PlayedMatch> {
        let mut played_matches = self
            .played_matches
            .lock()
            .expect("played match repository lock poisoned");
        played_matches.insert(*played_match.id(), played_match.clone());

        Ok(played_match)
    }

    async fn list_by_session(&self, session_id: &Uuid) -> HttpResult<Vec<PlayedMatch>> {
        let played_matches = self
            .played_matches
            .lock()
            .expect("played match repository lock poisoned");

        Ok(played_matches
            .values()
            .filter(|played_match| played_match.session_id() == session_id)
            .cloned()
            .collect())
    }
}
