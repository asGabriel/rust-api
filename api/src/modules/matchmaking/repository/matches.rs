use std::{collections::HashMap, sync::Mutex};

use async_trait::async_trait;
use http_error::HttpResult;
use uuid::Uuid;

use crate::modules::matchmaking::domain::matches::Match;

#[async_trait]
pub trait MatchRepository {
    async fn insert(&self, match_: Match) -> HttpResult<Match>;

    async fn list_by_session(&self, session_id: &Uuid) -> HttpResult<Vec<Match>>;
}

pub type DynMatchRepository = dyn MatchRepository + Send + Sync;

/// Process-memory cache repository, no database persistence.
/// Lets the pairing logic be tested before a migration exists.
#[derive(Default)]
pub struct InMemoryMatchRepository {
    matches: Mutex<HashMap<Uuid, Match>>,
}

impl InMemoryMatchRepository {
    pub fn new() -> Self {
        Self::default()
    }
}

#[async_trait]
impl MatchRepository for InMemoryMatchRepository {
    async fn insert(&self, match_: Match) -> HttpResult<Match> {
        let mut matches = self.matches.lock().expect("match repository lock poisoned");
        matches.insert(*match_.id(), match_.clone());

        Ok(match_)
    }

    async fn list_by_session(&self, session_id: &Uuid) -> HttpResult<Vec<Match>> {
        let matches = self.matches.lock().expect("match repository lock poisoned");

        Ok(matches
            .values()
            .filter(|match_| match_.session_id() == session_id)
            .cloned()
            .collect())
    }
}
