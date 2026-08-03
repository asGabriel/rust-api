use std::{collections::HashMap, sync::Mutex};

use async_trait::async_trait;
use http_error::HttpResult;
use uuid::Uuid;

use crate::modules::matchmaking::domain::game_day::GameDay;

#[async_trait]
pub trait GameDayRepository {
    async fn insert(&self, game_day: GameDay) -> HttpResult<GameDay>;

    async fn list(&self) -> HttpResult<Vec<GameDay>>;

    async fn get(&self, id: &Uuid) -> HttpResult<Option<GameDay>>;

    async fn update(&self, game_day: GameDay) -> HttpResult<GameDay>;
}

pub type DynGameDayRepository = dyn GameDayRepository + Send + Sync;

/// Repositório em cache (memória do processo), sem persistência em banco.
/// Serve para viabilizar os testes de sorteio antes de existir a migration.
#[derive(Default)]
pub struct InMemoryGameDayRepository {
    game_days: Mutex<HashMap<Uuid, GameDay>>,
}

impl InMemoryGameDayRepository {
    pub fn new() -> Self {
        Self::default()
    }
}

#[async_trait]
impl GameDayRepository for InMemoryGameDayRepository {
    async fn insert(&self, game_day: GameDay) -> HttpResult<GameDay> {
        let mut game_days = self
            .game_days
            .lock()
            .expect("game day repository lock poisoned");
        game_days.insert(*game_day.id(), game_day.clone());

        Ok(game_day)
    }

    async fn list(&self) -> HttpResult<Vec<GameDay>> {
        let game_days = self
            .game_days
            .lock()
            .expect("game day repository lock poisoned");

        Ok(game_days.values().cloned().collect())
    }

    async fn get(&self, id: &Uuid) -> HttpResult<Option<GameDay>> {
        let game_days = self
            .game_days
            .lock()
            .expect("game day repository lock poisoned");

        Ok(game_days.get(id).cloned())
    }

    async fn update(&self, game_day: GameDay) -> HttpResult<GameDay> {
        let mut game_days = self
            .game_days
            .lock()
            .expect("game day repository lock poisoned");
        game_days.insert(*game_day.id(), game_day.clone());

        Ok(game_day)
    }
}
