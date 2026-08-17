use chrono::{DateTime, NaiveDate, Utc};
use http_error::{HttpError, HttpResult};
use serde::{Deserialize, Serialize};
use util::getters;
use uuid::Uuid;

/// Filter applied when drawing teams for a `Session`: pairs restricted to
/// men, restricted to women, mixed (1 man + 1 woman per team), or open
/// (gender not considered at all — only valid together with
/// `ShuffleType::RoundRobin`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum GameMode {
    Male,
    Female,
    Mixed,
    Open,
}

impl GameMode {
    pub fn is_mixed(&self) -> bool {
        *self == GameMode::Mixed
    }

    pub fn is_open(&self) -> bool {
        *self == GameMode::Open
    }

    /// Whether the queue must never pair up two players who already played
    /// together while a fresh pairing is available. Only `Open` demands
    /// this — the other modes keep the best-effort pairing they've always had.
    pub fn requires_fresh_partner(&self) -> bool {
        self.is_open()
    }

    /// `Mixed` needs to split `players_per_team` evenly between men and
    /// women, so it only makes sense for an even count. Checked here, at the
    /// Session boundary, so a Session can never be saved in a configuration
    /// that a later team draw would be unable to honor.
    pub fn validate_players_per_team(&self, players_per_team: u8) -> HttpResult<()> {
        if self.is_mixed() && !players_per_team.is_multiple_of(2) {
            return Err(Box::new(HttpError::bad_request(
                "Mixed game mode requires an even number of players per team",
            )));
        }

        Ok(())
    }

    /// `Open` (gender not considered) only makes sense paired with
    /// `ShuffleType::RoundRobin` (the strategy that leans on that freedom to
    /// chase novel pairings), and `RoundRobin` only makes sense paired with
    /// `Open` — checked here, at the Session boundary, so the two fields can
    /// never drift into an inconsistent combination.
    pub fn validate_shuffle_type(&self, shuffle_type: ShuffleType) -> HttpResult<()> {
        let compatible = match self {
            GameMode::Open => shuffle_type == ShuffleType::RoundRobin,
            _ => shuffle_type != ShuffleType::RoundRobin,
        };

        if !compatible {
            return Err(Box::new(HttpError::bad_request(
                "GameMode::Open and ShuffleType::RoundRobin can only be used together",
            )));
        }

        Ok(())
    }
}

impl From<String> for GameMode {
    fn from(s: String) -> Self {
        match s.as_str() {
            "FEMALE" => GameMode::Female,
            "MIXED" => GameMode::Mixed,
            "OPEN" => GameMode::Open,
            _ => GameMode::Male,
        }
    }
}

impl From<GameMode> for String {
    fn from(game_mode: GameMode) -> Self {
        match game_mode {
            GameMode::Male => "MALE".to_string(),
            GameMode::Female => "FEMALE".to_string(),
            GameMode::Mixed => "MIXED".to_string(),
            GameMode::Open => "OPEN".to_string(),
        }
    }
}

/// The strategy governing how a session's team queue evolves as matches
/// finish.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ShuffleType {
    KingAndQueen,
    /// Same win/loss court-holding mechanic as `KingAndQueen`, but the queue
    /// only completes an incomplete team with a freed player when that
    /// pairing is brand new — it opens a new team instead of repeating a
    /// pairing while a fresh alternative exists. Only valid with
    /// `GameMode::Open`.
    RoundRobin,
}

impl From<String> for ShuffleType {
    fn from(s: String) -> Self {
        match s.as_str() {
            "ROUND_ROBIN" => ShuffleType::RoundRobin,
            _ => ShuffleType::KingAndQueen,
        }
    }
}

impl From<ShuffleType> for String {
    fn from(shuffle_type: ShuffleType) -> Self {
        match shuffle_type {
            ShuffleType::KingAndQueen => "KING_AND_QUEEN".to_string(),
            ShuffleType::RoundRobin => "ROUND_ROBIN".to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionSettings {
    players_per_team: u8,
    sets_to_win: u8,
    points_per_set: u8,
}

impl Default for SessionSettings {
    fn default() -> Self {
        Self {
            players_per_team: 2,
            sets_to_win: 2,
            points_per_set: 21,
        }
    }
}

getters! {
    SessionSettings {
        players_per_team: u8,
        sets_to_win: u8,
        points_per_set: u8,
    }
}

/// Represents the day of games: default settings for the round,
/// available courts and the list of players confirmed for that day.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Session {
    id: Uuid,
    date: NaiveDate,
    /// Free-text title so the session can be told apart at a glance beyond
    /// its date (e.g. distinguishing two sessions on the same day).
    description: Option<String>,
    settings: SessionSettings,
    available_courts: u8,
    game_mode: GameMode,
    shuffle_type: ShuffleType,
    player_ids: Vec<Uuid>,
    created_at: DateTime<Utc>,
    updated_at: Option<DateTime<Utc>>,
}

impl Session {
    pub fn new(
        date: NaiveDate,
        description: Option<String>,
        settings: SessionSettings,
        available_courts: u8,
        game_mode: GameMode,
        shuffle_type: ShuffleType,
    ) -> HttpResult<Self> {
        game_mode.validate_players_per_team(*settings.players_per_team())?;
        game_mode.validate_shuffle_type(shuffle_type)?;

        Ok(Self {
            id: Uuid::new_v4(),
            date,
            description,
            settings,
            available_courts,
            game_mode,
            shuffle_type,
            player_ids: Vec::new(),
            created_at: Utc::now(),
            updated_at: None,
        })
    }

    pub fn set_date(&mut self, date: NaiveDate) {
        self.date = date;
        self.updated_at = Some(Utc::now());
    }

    pub fn set_description(&mut self, description: Option<String>) {
        self.description = description;
        self.updated_at = Some(Utc::now());
    }

    pub fn set_settings(&mut self, settings: SessionSettings) -> HttpResult<()> {
        self.game_mode
            .validate_players_per_team(*settings.players_per_team())?;
        self.settings = settings;
        self.updated_at = Some(Utc::now());

        Ok(())
    }

    pub fn set_available_courts(&mut self, available_courts: u8) {
        self.available_courts = available_courts;
        self.updated_at = Some(Utc::now());
    }

    pub fn set_game_mode(&mut self, game_mode: GameMode) -> HttpResult<()> {
        game_mode.validate_players_per_team(*self.settings.players_per_team())?;
        game_mode.validate_shuffle_type(self.shuffle_type)?;
        self.game_mode = game_mode;
        self.updated_at = Some(Utc::now());

        Ok(())
    }

    pub fn set_shuffle_type(&mut self, shuffle_type: ShuffleType) -> HttpResult<()> {
        self.game_mode.validate_shuffle_type(shuffle_type)?;
        self.shuffle_type = shuffle_type;
        self.updated_at = Some(Utc::now());

        Ok(())
    }

    /// Sets `game_mode` and `shuffle_type` together, validating the pair as
    /// the target state rather than validating each field against the
    /// other's current value in sequence — needed because swapping between
    /// two mutually exclusive combinations (e.g. `Male`+`KingAndQueen` to
    /// `Open`+`RoundRobin`) has no valid intermediate state for
    /// `set_game_mode`/`set_shuffle_type` to pass through one at a time.
    pub fn set_game_mode_and_shuffle_type(
        &mut self,
        game_mode: GameMode,
        shuffle_type: ShuffleType,
    ) -> HttpResult<()> {
        game_mode.validate_players_per_team(*self.settings.players_per_team())?;
        game_mode.validate_shuffle_type(shuffle_type)?;
        self.game_mode = game_mode;
        self.shuffle_type = shuffle_type;
        self.updated_at = Some(Utc::now());

        Ok(())
    }

    pub fn set_player_ids(&mut self, player_ids: Vec<Uuid>) {
        self.player_ids = player_ids;
        self.updated_at = Some(Utc::now());
    }
}

getters! {
    Session {
        id: Uuid,
        date: NaiveDate,
        description: Option<String>,
        settings: SessionSettings,
        available_courts: u8,
        game_mode: GameMode,
        shuffle_type: ShuffleType,
        player_ids: Vec<Uuid>,
        created_at: DateTime<Utc>,
        updated_at: Option<DateTime<Utc>>,
    }
}

impl From<&sqlx::postgres::PgRow> for Session {
    fn from(row: &sqlx::postgres::PgRow) -> Self {
        use sqlx::Row;

        let settings = SessionSettings {
            players_per_team: row.get::<i16, _>("players_per_team") as u8,
            sets_to_win: row.get::<i16, _>("sets_to_win") as u8,
            points_per_set: row.get::<i16, _>("points_per_set") as u8,
        };

        Self {
            id: row.get("id"),
            date: row.get("date"),
            description: row.get("description"),
            settings,
            available_courts: row.get::<i16, _>("available_courts") as u8,
            game_mode: row.get::<String, _>("game_mode").into(),
            shuffle_type: row.get::<String, _>("shuffle_type").into(),
            player_ids: row.get("player_ids"),
            created_at: row.get("created_at"),
            updated_at: row.get("updated_at"),
        }
    }
}

#[cfg(test)]
mod tests {
    use http_error::HttpErrorKind;

    use super::*;

    fn date() -> NaiveDate {
        NaiveDate::from_ymd_opt(2026, 1, 1).unwrap()
    }

    #[test]
    fn test_new_session_rejects_mixed_mode_with_odd_players_per_team() {
        let mut settings = SessionSettings::default();
        settings.players_per_team = 3;

        let err = Session::new(
            date(),
            None,
            settings,
            2,
            GameMode::Mixed,
            ShuffleType::KingAndQueen,
        )
        .unwrap_err();

        assert_eq!(err.kind, HttpErrorKind::BadRequest);
    }

    #[test]
    fn test_new_session_allows_mixed_mode_with_even_players_per_team() {
        let session = Session::new(
            date(),
            None,
            SessionSettings::default(),
            2,
            GameMode::Mixed,
            ShuffleType::KingAndQueen,
        );

        assert!(session.is_ok());
    }

    #[test]
    fn test_set_game_mode_rejects_mixed_when_current_settings_are_odd() {
        let mut settings = SessionSettings::default();
        settings.players_per_team = 3;
        let mut session = Session::new(
            date(),
            None,
            settings,
            2,
            GameMode::Male,
            ShuffleType::KingAndQueen,
        )
        .unwrap();

        let err = session.set_game_mode(GameMode::Mixed).unwrap_err();

        assert_eq!(err.kind, HttpErrorKind::BadRequest);
    }

    #[test]
    fn test_set_settings_rejects_odd_players_per_team_when_mode_is_mixed() {
        let mut session = Session::new(
            date(),
            None,
            SessionSettings::default(),
            2,
            GameMode::Mixed,
            ShuffleType::KingAndQueen,
        )
        .unwrap();
        let mut odd_settings = SessionSettings::default();
        odd_settings.players_per_team = 3;

        let err = session.set_settings(odd_settings).unwrap_err();

        assert_eq!(err.kind, HttpErrorKind::BadRequest);
    }

    #[test]
    fn test_new_session_rejects_open_mode_with_king_and_queen_shuffle() {
        let err = Session::new(
            date(),
            None,
            SessionSettings::default(),
            2,
            GameMode::Open,
            ShuffleType::KingAndQueen,
        )
        .unwrap_err();

        assert_eq!(err.kind, HttpErrorKind::BadRequest);
    }

    #[test]
    fn test_new_session_rejects_round_robin_shuffle_with_non_open_mode() {
        let err = Session::new(
            date(),
            None,
            SessionSettings::default(),
            2,
            GameMode::Male,
            ShuffleType::RoundRobin,
        )
        .unwrap_err();

        assert_eq!(err.kind, HttpErrorKind::BadRequest);
    }

    #[test]
    fn test_new_session_allows_open_mode_with_round_robin_shuffle() {
        let session = Session::new(
            date(),
            None,
            SessionSettings::default(),
            2,
            GameMode::Open,
            ShuffleType::RoundRobin,
        );

        assert!(session.is_ok());
    }

    #[test]
    fn test_set_game_mode_rejects_open_when_current_shuffle_is_king_and_queen() {
        let mut session = Session::new(
            date(),
            None,
            SessionSettings::default(),
            2,
            GameMode::Male,
            ShuffleType::KingAndQueen,
        )
        .unwrap();

        let err = session.set_game_mode(GameMode::Open).unwrap_err();

        assert_eq!(err.kind, HttpErrorKind::BadRequest);
    }

    #[test]
    fn test_set_shuffle_type_rejects_round_robin_when_current_mode_is_not_open() {
        let mut session = Session::new(
            date(),
            None,
            SessionSettings::default(),
            2,
            GameMode::Male,
            ShuffleType::KingAndQueen,
        )
        .unwrap();

        let err = session
            .set_shuffle_type(ShuffleType::RoundRobin)
            .unwrap_err();

        assert_eq!(err.kind, HttpErrorKind::BadRequest);
    }

    #[test]
    fn test_set_game_mode_and_shuffle_type_together_allows_swapping_to_open_round_robin() {
        let mut session = Session::new(
            date(),
            None,
            SessionSettings::default(),
            2,
            GameMode::Male,
            ShuffleType::KingAndQueen,
        )
        .unwrap();

        session
            .set_game_mode_and_shuffle_type(GameMode::Open, ShuffleType::RoundRobin)
            .unwrap();

        assert_eq!(*session.game_mode(), GameMode::Open);
        assert_eq!(*session.shuffle_type(), ShuffleType::RoundRobin);
    }

    #[test]
    fn test_set_game_mode_and_shuffle_type_rejects_mismatched_pair() {
        let mut session = Session::new(
            date(),
            None,
            SessionSettings::default(),
            2,
            GameMode::Male,
            ShuffleType::KingAndQueen,
        )
        .unwrap();

        let err = session
            .set_game_mode_and_shuffle_type(GameMode::Open, ShuffleType::KingAndQueen)
            .unwrap_err();

        assert_eq!(err.kind, HttpErrorKind::BadRequest);
    }
}
