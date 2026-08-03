use std::{collections::HashMap, sync::Mutex};

use async_trait::async_trait;
use http_error::HttpResult;
use uuid::Uuid;

use crate::modules::matchmaking::domain::team::Team;

#[async_trait]
pub trait TeamRepository {
    async fn insert(&self, team: Team) -> HttpResult<Team>;

    async fn list_by_session(&self, session_id: &Uuid) -> HttpResult<Vec<Team>>;
}

pub type DynTeamRepository = dyn TeamRepository + Send + Sync;

/// Repositório em cache (memória do processo), sem persistência em banco.
/// Serve para viabilizar os testes de sorteio antes de existir a migration.
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
}
