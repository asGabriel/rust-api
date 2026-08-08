use std::{collections::HashMap, sync::Mutex};

use async_trait::async_trait;
use http_error::HttpResult;
use uuid::Uuid;

use crate::modules::matchmaking::domain::team::Team;

#[async_trait]
pub trait TeamRepository {
    async fn insert(&self, team: Team) -> HttpResult<Team>;

    async fn list_by_session(&self, session_id: &Uuid) -> HttpResult<Vec<Team>>;

    async fn get(&self, id: &Uuid) -> HttpResult<Option<Team>>;
}

pub type DynTeamRepository = dyn TeamRepository + Send + Sync;

/// Process-memory cache repository, no database persistence.
/// Lets the pairing logic be tested before a migration exists.
#[derive(Default)]
pub struct InMemoryTeamRepository {
    teams: Mutex<HashMap<Uuid, Team>>,
}

impl InMemoryTeamRepository {
    pub fn new() -> Self {
        Self::default()
    }
}

#[async_trait]
impl TeamRepository for InMemoryTeamRepository {
    async fn insert(&self, team: Team) -> HttpResult<Team> {
        let mut teams = self.teams.lock().expect("team repository lock poisoned");
        teams.insert(*team.id(), team.clone());

        Ok(team)
    }

    async fn list_by_session(&self, session_id: &Uuid) -> HttpResult<Vec<Team>> {
        let teams = self.teams.lock().expect("team repository lock poisoned");

        Ok(teams
            .values()
            .filter(|team| team.session_id() == session_id)
            .cloned()
            .collect())
    }

    async fn get(&self, id: &Uuid) -> HttpResult<Option<Team>> {
        let teams = self.teams.lock().expect("team repository lock poisoned");

        Ok(teams.get(id).cloned())
    }
}
