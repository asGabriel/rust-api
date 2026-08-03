use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use util::getters;
use uuid::Uuid;

/// Registro de uma partida já disputada: quadra, times envolvidos e o
/// time vencedor.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlayedMatch {
    id: Uuid,
    session_id: Uuid,
    court: u8,
    team_a_id: Uuid,
    team_b_id: Uuid,
    winner_team_id: Uuid,
    played_at: DateTime<Utc>,
}

impl PlayedMatch {
    pub fn new(
        session_id: Uuid,
        court: u8,
        team_a_id: Uuid,
        team_b_id: Uuid,
        winner_team_id: Uuid,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            session_id,
            court,
            team_a_id,
            team_b_id,
            winner_team_id,
            played_at: Utc::now(),
        }
    }
}

getters! {
    PlayedMatch {
        id: Uuid,
        session_id: Uuid,
        court: u8,
        team_a_id: Uuid,
        team_b_id: Uuid,
        winner_team_id: Uuid,
        played_at: DateTime<Utc>,
    }
}
