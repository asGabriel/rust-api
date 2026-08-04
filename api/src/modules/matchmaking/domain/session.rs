use chrono::{DateTime, NaiveDate, Utc};
use http_error::{HttpError, HttpResult};
use serde::{Deserialize, Serialize};
use util::getters;
use uuid::Uuid;

/// Filter applied when drawing teams for a `Session`: pairs restricted to
/// men, restricted to women, or mixed (1 man + 1 woman per team).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum GameMode {
    Male,
    Female,
    Mixed,
}

impl GameMode {
    pub fn is_mixed(&self) -> bool {
        *self == GameMode::Mixed
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
    settings: SessionSettings,
    available_courts: u8,
    game_mode: GameMode,
    player_ids: Vec<Uuid>,
    created_at: DateTime<Utc>,
    updated_at: Option<DateTime<Utc>>,
}

impl Session {
    pub fn new(
        date: NaiveDate,
        settings: SessionSettings,
        available_courts: u8,
        game_mode: GameMode,
    ) -> HttpResult<Self> {
        game_mode.validate_players_per_team(*settings.players_per_team())?;

        Ok(Self {
            id: Uuid::new_v4(),
            date,
            settings,
            available_courts,
            game_mode,
            player_ids: Vec::new(),
            created_at: Utc::now(),
            updated_at: None,
        })
    }

    pub fn set_date(&mut self, date: NaiveDate) {
        self.date = date;
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
        self.game_mode = game_mode;
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
        settings: SessionSettings,
        available_courts: u8,
        game_mode: GameMode,
        player_ids: Vec<Uuid>,
        created_at: DateTime<Utc>,
        updated_at: Option<DateTime<Utc>>,
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

        let err = Session::new(date(), settings, 2, GameMode::Mixed).unwrap_err();

        assert_eq!(err.kind, HttpErrorKind::BadRequest);
    }

    #[test]
    fn test_new_session_allows_mixed_mode_with_even_players_per_team() {
        let session = Session::new(date(), SessionSettings::default(), 2, GameMode::Mixed);

        assert!(session.is_ok());
    }

    #[test]
    fn test_set_game_mode_rejects_mixed_when_current_settings_are_odd() {
        let mut settings = SessionSettings::default();
        settings.players_per_team = 3;
        let mut session = Session::new(date(), settings, 2, GameMode::Male).unwrap();

        let err = session.set_game_mode(GameMode::Mixed).unwrap_err();

        assert_eq!(err.kind, HttpErrorKind::BadRequest);
    }

    #[test]
    fn test_set_settings_rejects_odd_players_per_team_when_mode_is_mixed() {
        let mut session =
            Session::new(date(), SessionSettings::default(), 2, GameMode::Mixed).unwrap();
        let mut odd_settings = SessionSettings::default();
        odd_settings.players_per_team = 3;

        let err = session.set_settings(odd_settings).unwrap_err();

        assert_eq!(err.kind, HttpErrorKind::BadRequest);
    }
}
