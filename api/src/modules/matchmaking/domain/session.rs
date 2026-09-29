use chrono::{DateTime, NaiveDate, Utc};
use http_error::{HttpError, HttpResult};
use serde::{Deserialize, Serialize};
use util::getters;
use uuid::Uuid;

/// Filter applied when drawing teams for a `Session`: pairs restricted to
/// men, restricted to women, mixed (1 man + 1 woman per team), or open
/// (gender not considered at all).
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
    /// Players confirmed present ("checked in") for this session. Drives the
    /// waiting queue, team draws and match start. Always a subset of
    /// `roster_player_ids`; moved only through check-in / check-out.
    player_ids: Vec<Uuid>,
    /// Players selected/expected for this session (the roster). A planning
    /// list, edited in bulk via `PATCH /matchmaking/sessions/{id}`; being on
    /// it is the precondition for checking in.
    roster_player_ids: Vec<Uuid>,
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
    ) -> HttpResult<Self> {
        game_mode.validate_players_per_team(*settings.players_per_team())?;

        Ok(Self {
            id: Uuid::new_v4(),
            date,
            description,
            settings,
            available_courts,
            game_mode,
            player_ids: Vec::new(),
            roster_player_ids: Vec::new(),
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
        self.game_mode = game_mode;
        self.updated_at = Some(Utc::now());

        Ok(())
    }

    pub fn set_player_ids(&mut self, player_ids: Vec<Uuid>) {
        self.player_ids = player_ids;
        self.updated_at = Some(Utc::now());
    }

    /// Replaces the session roster. Keeps the invariant that the checked-in
    /// list is a subset of the roster: any currently checked-in player who
    /// is not in `roster_player_ids` is also checked out. Returns those
    /// cascaded-out player ids so the caller can sync the waiting queue.
    pub fn set_roster_player_ids(&mut self, roster_player_ids: Vec<Uuid>) -> Vec<Uuid> {
        let checked_out: Vec<Uuid> = self
            .player_ids
            .iter()
            .copied()
            .filter(|id| !roster_player_ids.contains(id))
            .collect();
        self.player_ids.retain(|id| roster_player_ids.contains(id));
        self.roster_player_ids = roster_player_ids;
        self.updated_at = Some(Utc::now());
        checked_out
    }

    /// Whether `player_id` is confirmed (checked in) for this session.
    pub fn has_player(&self, player_id: &Uuid) -> bool {
        self.player_ids.contains(player_id)
    }

    /// Whether `player_id` is on the session roster — the precondition for a
    /// check-in.
    pub fn is_on_roster(&self, player_id: &Uuid) -> bool {
        self.roster_player_ids.contains(player_id)
    }

    /// Checks a player in: adds them to the checked-in list (`player_ids`)
    /// unless they are already checked in. The caller is responsible for
    /// having verified the player is on the roster. Returns `true` when the
    /// list changed, `false` on a no-op (already checked in), so the caller
    /// only touches the queue when something actually changed.
    pub fn check_in_player(&mut self, player_id: Uuid) -> bool {
        if self.has_player(&player_id) {
            return false;
        }

        self.player_ids.push(player_id);
        self.updated_at = Some(Utc::now());
        true
    }

    /// Checks a player out: removes them from the checked-in list
    /// (`player_ids`) if present. They stay on the roster. Returns `true`
    /// when the list changed, `false` on a no-op (was not checked in).
    pub fn check_out_player(&mut self, player_id: &Uuid) -> bool {
        let before = self.player_ids.len();
        self.player_ids.retain(|id| id != player_id);

        let changed = self.player_ids.len() != before;
        if changed {
            self.updated_at = Some(Utc::now());
        }
        changed
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
        player_ids: Vec<Uuid>,
        roster_player_ids: Vec<Uuid>,
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
            player_ids: row.get("player_ids"),
            roster_player_ids: row.get("roster_player_ids"),
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

        let err = Session::new(date(), None, settings, 2, GameMode::Mixed).unwrap_err();

        assert_eq!(err.kind, HttpErrorKind::BadRequest);
    }

    #[test]
    fn test_new_session_allows_mixed_mode_with_even_players_per_team() {
        let session = Session::new(date(), None, SessionSettings::default(), 2, GameMode::Mixed);

        assert!(session.is_ok());
    }

    #[test]
    fn test_set_game_mode_rejects_mixed_when_current_settings_are_odd() {
        let mut settings = SessionSettings::default();
        settings.players_per_team = 3;
        let mut session = Session::new(date(), None, settings, 2, GameMode::Male).unwrap();

        let err = session.set_game_mode(GameMode::Mixed).unwrap_err();

        assert_eq!(err.kind, HttpErrorKind::BadRequest);
    }

    #[test]
    fn test_set_settings_rejects_odd_players_per_team_when_mode_is_mixed() {
        let mut session =
            Session::new(date(), None, SessionSettings::default(), 2, GameMode::Mixed).unwrap();
        let mut odd_settings = SessionSettings::default();
        odd_settings.players_per_team = 3;

        let err = session.set_settings(odd_settings).unwrap_err();

        assert_eq!(err.kind, HttpErrorKind::BadRequest);
    }

    #[test]
    fn test_check_in_player_adds_to_roster_once_and_is_idempotent() {
        let mut session =
            Session::new(date(), None, SessionSettings::default(), 2, GameMode::Open).unwrap();
        let player_id = Uuid::new_v4();

        assert!(session.check_in_player(player_id));
        assert!(session.has_player(&player_id));
        assert_eq!(session.player_ids(), &vec![player_id]);
        assert!(session.updated_at().is_some());

        // second check-in is a no-op
        assert!(!session.check_in_player(player_id));
        assert_eq!(session.player_ids(), &vec![player_id]);
    }

    #[test]
    fn test_check_out_player_removes_from_roster_and_is_idempotent() {
        let mut session =
            Session::new(date(), None, SessionSettings::default(), 2, GameMode::Open).unwrap();
        let player_id = Uuid::new_v4();
        session.check_in_player(player_id);

        assert!(session.check_out_player(&player_id));
        assert!(!session.has_player(&player_id));
        assert!(session.player_ids().is_empty());

        // checking a stranger out is a no-op
        assert!(!session.check_out_player(&Uuid::new_v4()));
    }

    #[test]
    fn test_set_roster_player_ids_replaces_roster_and_bumps_updated_at() {
        let mut session =
            Session::new(date(), None, SessionSettings::default(), 2, GameMode::Open).unwrap();
        let a = Uuid::new_v4();
        let b = Uuid::new_v4();

        let cascaded = session.set_roster_player_ids(vec![a, b]);

        assert!(cascaded.is_empty());
        assert!(session.is_on_roster(&a));
        assert!(session.is_on_roster(&b));
        assert!(!session.is_on_roster(&Uuid::new_v4()));
        assert!(session.updated_at().is_some());
        // roster is independent of the checked-in list
        assert!(session.player_ids().is_empty());
    }

    #[test]
    fn test_set_roster_player_ids_checks_out_players_dropped_from_the_roster() {
        let mut session =
            Session::new(date(), None, SessionSettings::default(), 2, GameMode::Open).unwrap();
        let staying = Uuid::new_v4();
        let leaving = Uuid::new_v4();
        session.set_roster_player_ids(vec![staying, leaving]);
        session.check_in_player(staying);
        session.check_in_player(leaving);

        let cascaded = session.set_roster_player_ids(vec![staying]);

        assert_eq!(cascaded, vec![leaving]);
        assert!(!session.is_on_roster(&leaving));
        assert!(!session.has_player(&leaving));
        assert!(session.has_player(&staying));
    }
}
