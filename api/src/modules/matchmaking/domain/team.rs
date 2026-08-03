use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use util::getters;
use uuid::Uuid;

/// A pair/team formed from the players confirmed in a `Session`,
/// used to compose that day's matches.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Team {
    id: Uuid,
    session_id: Uuid,
    player_ids: Vec<Uuid>,
    created_at: DateTime<Utc>,
}

impl Team {
    pub fn new(session_id: Uuid, player_ids: Vec<Uuid>) -> Self {
        Self {
            id: Uuid::new_v4(),
            session_id,
            player_ids,
            created_at: Utc::now(),
        }
    }
}

getters! {
    Team {
        id: Uuid,
        session_id: Uuid,
        player_ids: Vec<Uuid>,
        created_at: DateTime<Utc>,
    }
}
