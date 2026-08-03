use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use util::getters;
use uuid::Uuid;

/// Record of a match that already happened: court, teams involved and
/// the winning team.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Match {
    id: Uuid,
    session_id: Uuid,
    court: u8,
    team_a_id: Uuid,
    team_b_id: Uuid,
    winner_team_id: Uuid,
    played_at: DateTime<Utc>,
}

impl Match {
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
    Match {
        id: Uuid,
        session_id: Uuid,
        court: u8,
        team_a_id: Uuid,
        team_b_id: Uuid,
        winner_team_id: Uuid,
        played_at: DateTime<Utc>,
    }
}
