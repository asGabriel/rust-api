use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use util::getters;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GameDaySettings {
    players_per_team: u8,
    sets_to_win: u8,
    points_per_set: u8,
}

impl Default for GameDaySettings {
    fn default() -> Self {
        Self {
            players_per_team: 6,
            sets_to_win: 2,
            points_per_set: 25,
        }
    }
}

getters! {
    GameDaySettings {
        players_per_team: u8,
        sets_to_win: u8,
        points_per_set: u8,
    }
}

/// Representa o dia de jogos: configurações padrão da rodada, quadras
/// disponíveis e a lista de jogadores confirmados para aquele dia.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GameDay {
    id: Uuid,
    date: NaiveDate,
    settings: GameDaySettings,
    available_courts: u8,
    player_ids: Vec<Uuid>,
    created_at: DateTime<Utc>,
    updated_at: Option<DateTime<Utc>>,
}

impl GameDay {
    pub fn new(date: NaiveDate, settings: GameDaySettings, available_courts: u8) -> Self {
        Self {
            id: Uuid::new_v4(),
            date,
            settings,
            available_courts,
            player_ids: Vec::new(),
            created_at: Utc::now(),
            updated_at: None,
        }
    }

    pub fn set_date(&mut self, date: NaiveDate) {
        self.date = date;
        self.updated_at = Some(Utc::now());
    }

    pub fn set_settings(&mut self, settings: GameDaySettings) {
        self.settings = settings;
        self.updated_at = Some(Utc::now());
    }

    pub fn set_available_courts(&mut self, available_courts: u8) {
        self.available_courts = available_courts;
        self.updated_at = Some(Utc::now());
    }

    pub fn set_player_ids(&mut self, player_ids: Vec<Uuid>) {
        self.player_ids = player_ids;
        self.updated_at = Some(Utc::now());
    }
}

getters! {
    GameDay {
        id: Uuid,
        date: NaiveDate,
        settings: GameDaySettings,
        available_courts: u8,
        player_ids: Vec<Uuid>,
        created_at: DateTime<Utc>,
        updated_at: Option<DateTime<Utc>>,
    }
}
