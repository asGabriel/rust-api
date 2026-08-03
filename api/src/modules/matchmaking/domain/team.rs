use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use util::getters;
use uuid::Uuid;

/// Uma dupla/equipe formada a partir dos jogadores confirmados em um
/// `GameDay`, usada para compor as partidas daquele dia.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Team {
    id: Uuid,
    game_day_id: Uuid,
    player_ids: Vec<Uuid>,
    created_at: DateTime<Utc>,
}

impl Team {
    pub fn new(game_day_id: Uuid, player_ids: Vec<Uuid>) -> Self {
        Self {
            id: Uuid::new_v4(),
            game_day_id,
            player_ids,
            created_at: Utc::now(),
        }
    }
}

getters! {
    Team {
        id: Uuid,
        game_day_id: Uuid,
        player_ids: Vec<Uuid>,
        created_at: DateTime<Utc>,
    }
}
