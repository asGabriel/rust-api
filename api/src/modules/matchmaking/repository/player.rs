use std::{collections::HashMap, sync::Mutex};

use async_trait::async_trait;
use http_error::HttpResult;
use uuid::Uuid;

use crate::modules::matchmaking::domain::player::{Gender, Player};

#[async_trait]
pub trait PlayerRepository {
    async fn insert(&self, player: Player) -> HttpResult<Player>;

    async fn list(&self) -> HttpResult<Vec<Player>>;

    async fn get(&self, id: &Uuid) -> HttpResult<Option<Player>>;

    async fn update(&self, player: Player) -> HttpResult<Player>;
}

pub type DynPlayerRepository = dyn PlayerRepository + Send + Sync;

/// Process-memory cache repository, no database persistence.
/// Lets the pairing logic be tested before a migration exists.
#[derive(Default)]
pub struct InMemoryPlayerRepository {
    players: Mutex<HashMap<Uuid, Player>>,
}

const DEFAULT_PLAYERS: [(&str, Gender); 16] = [
    ("Gabriel", Gender::Male),
    ("Bru Varella", Gender::Female),
    ("Luan", Gender::Male),
    ("Jully", Gender::Female),
    ("Brenda", Gender::Female),
    ("Flavia", Gender::Female),
    ("Marcos", Gender::Male),
    ("Roberta", Gender::Female),
    ("Serginho", Gender::Male),
    ("Valdeta", Gender::Female),
    ("Alessandra", Gender::Female),
    ("Andre", Gender::Male),
    ("Angela", Gender::Female),
    ("Douglas", Gender::Male),
    ("Gabe", Gender::Male),
    ("Valdenia", Gender::Female),
];

impl InMemoryPlayerRepository {
    pub fn new() -> Self {
        let players = DEFAULT_PLAYERS
            .into_iter()
            .map(|(name, gender)| {
                let player = Player::new(name.to_string(), gender);
                (*player.id(), player)
            })
            .collect();

        Self {
            players: Mutex::new(players),
        }
    }
}

#[async_trait]
impl PlayerRepository for InMemoryPlayerRepository {
    async fn insert(&self, player: Player) -> HttpResult<Player> {
        let mut players = self
            .players
            .lock()
            .expect("player repository lock poisoned");
        players.insert(*player.id(), player.clone());

        Ok(player)
    }

    async fn list(&self) -> HttpResult<Vec<Player>> {
        let players = self
            .players
            .lock()
            .expect("player repository lock poisoned");

        Ok(players.values().cloned().collect())
    }

    async fn get(&self, id: &Uuid) -> HttpResult<Option<Player>> {
        let players = self
            .players
            .lock()
            .expect("player repository lock poisoned");

        Ok(players.get(id).cloned())
    }

    async fn update(&self, player: Player) -> HttpResult<Player> {
        let mut players = self
            .players
            .lock()
            .expect("player repository lock poisoned");
        players.insert(*player.id(), player.clone());

        Ok(player)
    }
}
