use std::collections::HashSet;

use chrono::{DateTime, Utc};
use http_error::{HttpError, HttpResult};
use serde::{Deserialize, Serialize};
use util::getters;
use uuid::Uuid;

/// A team's place in the session's rotation: `Draft` while it's a suggested
/// (or manually assembled) lineup the operator hasn't started yet, `Holding`
/// a court after a win (until it either loses or hits the consecutive-win
/// cap), `Playing` while it has an in-progress match, or `Disbanded` once
/// it's done — lost, hit the win cap, or a draft that was discarded —
/// freeing its players back into the session queue.
///
/// There is no standing queue of teams: waiting players live in
/// `matchmaking.session_queue` as individuals, and a `Team` row exists only
/// from the moment it's drafted for a court onward. See the matchmaking
/// skill, "Fila e rotação de quadra".
///
/// `Playing` is never set by this domain layer — no method here constructs
/// it. It's written exclusively by the `trg_match_marks_teams_playing`
/// trigger (`migrations/matchmaking/20260817130000_team-playing-status.sql`)
/// the instant a `Match` row with no `winner_team_id` yet is inserted, so it
/// can never drift from whether the team actually has an open match: there's
/// no second write for application code to forget. If you're tempted to set
/// `TeamStatus::Playing` from Rust, that's a sign the write belongs in
/// `Match::new`/`create_match` at the SQL level instead — see the migration.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum TeamStatus {
    Draft,
    Holding,
    Playing,
    Disbanded,
}

impl From<String> for TeamStatus {
    fn from(s: String) -> Self {
        match s.as_str() {
            "HOLDING" => TeamStatus::Holding,
            "PLAYING" => TeamStatus::Playing,
            "DISBANDED" => TeamStatus::Disbanded,
            _ => TeamStatus::Draft,
        }
    }
}

impl From<TeamStatus> for String {
    fn from(status: TeamStatus) -> Self {
        match status {
            TeamStatus::Draft => "DRAFT".to_string(),
            TeamStatus::Holding => "HOLDING".to_string(),
            TeamStatus::Playing => "PLAYING".to_string(),
            TeamStatus::Disbanded => "DISBANDED".to_string(),
        }
    }
}

/// A pair/team formed from the players confirmed in a `Session`,
/// used to compose that day's matches.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Team {
    id: Uuid,
    session_id: Uuid,
    player_ids: Vec<Uuid>,
    status: TeamStatus,
    consecutive_wins: u8,
    /// The court this team is drafted as the next challenger for, if any.
    /// `Some` only for a `Draft` the queue formed for a specific idle court
    /// (or a seeded opening draft) — it's how `resolve_match_result` knows
    /// that court already has a pending challenger and must not be filled
    /// again on the next result. A manually assembled draft has `None` (the
    /// operator picks the court when starting the match); once a team is
    /// `Playing`/`Holding` the court lives on the `Match`, not here.
    court: Option<u8>,
    created_at: DateTime<Utc>,
}

impl Team {
    /// A team is rotated out after this many consecutive wins on the same
    /// court, so other waiting teams get a turn.
    const MAX_CONSECUTIVE_WINS: u8 = 2;

    /// A freshly drafted lineup — the operator confirms (or edits, or
    /// discards) it before it starts a match. Not yet tied to a court; use
    /// `assign_court` for a draft the queue formed for a specific idle one.
    pub fn new(session_id: Uuid, player_ids: Vec<Uuid>) -> Self {
        Self {
            id: Uuid::new_v4(),
            session_id,
            player_ids,
            status: TeamStatus::Draft,
            consecutive_wins: 0,
            court: None,
            created_at: Utc::now(),
        }
    }

    /// Marks this draft as the pending challenger for `court`, so the
    /// court-fill in `resolve_match_result` won't draft a second challenger
    /// for it before the operator confirms this one.
    pub fn assign_court(mut self, court: u8) -> Self {
        self.court = Some(court);
        self
    }

    pub fn is_draft(&self) -> bool {
        self.status == TeamStatus::Draft
    }

    /// Whether this team is a `Draft` still pending confirmation for `court`.
    pub fn is_draft_for_court(&self, court: u8) -> bool {
        self.is_draft() && self.court == Some(court)
    }

    pub fn is_holding(&self) -> bool {
        self.status == TeamStatus::Holding
    }

    pub fn is_playing(&self) -> bool {
        self.status == TeamStatus::Playing
    }

    pub fn is_disbanded(&self) -> bool {
        self.status == TeamStatus::Disbanded
    }

    /// Whether this team's roster is exactly a full team — the queue never
    /// drafts a short or oversized team, but the manual paths can, so a
    /// match can't start until this holds.
    pub fn has_full_roster(&self, players_per_team: u8) -> bool {
        self.player_ids.len() == usize::from(players_per_team)
    }

    /// Frees this team's players back into the queue: it's done, whether it
    /// just lost or hit the consecutive-win cap. Idempotent.
    pub fn disband(&mut self) {
        self.status = TeamStatus::Disbanded;
    }

    /// Adds a freed player to this (presumably incomplete) waiting team,
    /// completing it or extending how many players it still needs.
    pub fn add_player(&mut self, player_id: Uuid) {
        self.player_ids.push(player_id);
    }

    /// Replaces this team's whole roster — the "editar time" contingency
    /// path (see `TeamValidator::validate_team_update`), as opposed to
    /// `add_player` completing an incomplete team one freed player at a
    /// time.
    pub fn set_player_ids(&mut self, player_ids: Vec<Uuid>) {
        self.player_ids = player_ids;
    }

    /// Records a win. A team keeps holding its court after a win, but only
    /// up to `MAX_CONSECUTIVE_WINS` in a row — past that, it's disbanded
    /// like a loss would be, so other waiting teams get a turn.
    pub fn register_win(&mut self) {
        self.consecutive_wins += 1;

        if self.consecutive_wins >= Self::MAX_CONSECUTIVE_WINS {
            self.disband();
        } else {
            self.status = TeamStatus::Holding;
        }
    }
}

getters! {
    Team {
        id: Uuid,
        session_id: Uuid,
        player_ids: Vec<Uuid>,
        status: TeamStatus,
        consecutive_wins: u8,
        court: Option<u8>,
        created_at: DateTime<Utc>,
    }
}

impl From<&sqlx::postgres::PgRow> for Team {
    fn from(row: &sqlx::postgres::PgRow) -> Self {
        use sqlx::Row;

        Self {
            id: row.get("id"),
            session_id: row.get("session_id"),
            player_ids: row.get("player_ids"),
            status: row.get::<String, _>("status").into(),
            consecutive_wins: row.get::<i16, _>("consecutive_wins") as u8,
            court: row.get::<Option<i16>, _>("court").map(|court| court as u8),
            created_at: row.get("created_at"),
        }
    }
}

/// Centralizes the validation rules for forming a `Team` within a `Session`,
/// including the manual (contingency) path: an operator picking specific
/// players to force a team into the queue, regardless of the session's
/// `GameMode` — that only constrains the automated draw and queue
/// rotation, never a manually assembled team.
/// Bound to a `session_id` and that session's confirmed `player_ids` at
/// construction so it always scopes its checks to that session, regardless
/// of what `existing_teams` the caller passes in.
pub struct TeamValidator {
    session_id: Uuid,
    session_player_ids: Vec<Uuid>,
}

impl TeamValidator {
    pub fn new(session_id: Uuid, session_player_ids: Vec<Uuid>) -> Self {
        Self {
            session_id,
            session_player_ids,
        }
    }

    /// A player cannot repeat within the same team, must be a player
    /// confirmed for the session, and cannot be in another active
    /// (non-disbanded) team of the same session at the same time. A
    /// disbanded team's players are free again, so they never block a new
    /// team here.
    pub fn validate_new_team(
        &self,
        existing_teams: &[Team],
        player_ids: &[Uuid],
    ) -> HttpResult<()> {
        Self::reject_duplicate_players_in_team(player_ids)?;
        self.reject_players_not_confirmed_in_session(player_ids)?;
        self.reject_players_already_in_active_team(existing_teams, player_ids)?;

        Ok(())
    }

    /// Validates replacing a `Draft` team's whole roster with
    /// `new_player_ids` — the operator editing a suggested lineup before
    /// starting it. Only `entering_player_ids` (the players in
    /// `new_player_ids` that aren't already on the team being edited) are
    /// checked against other teams — a player staying on the team, or simply
    /// leaving it, is never treated as being "stolen". An entering player
    /// already on another `Draft` that isn't busy can be pulled (that draft
    /// is broken up); one in a `Holding` team or mid-match can't.
    /// `other_teams` must exclude the team being edited, so it's never
    /// mistaken for its own origin team.
    ///
    /// Returns, for each distinct draft a player was pulled from, that team
    /// paired with the ids of its now partner-less remaining players — the
    /// caller must disband that draft and return those players to the queue.
    pub fn validate_team_update(
        &self,
        other_teams: &[Team],
        busy_team_ids: &HashSet<Uuid>,
        new_player_ids: &[Uuid],
        entering_player_ids: &[Uuid],
    ) -> HttpResult<Vec<(Team, Vec<Uuid>)>> {
        if new_player_ids.is_empty() {
            return Err(Box::new(HttpError::bad_request(
                "A team must keep at least one player — disband it instead of emptying its roster",
            )));
        }

        Self::reject_duplicate_players_in_team(new_player_ids)?;
        self.reject_players_not_confirmed_in_session(new_player_ids)?;
        self.collect_broken_origin_teams(other_teams, busy_team_ids, entering_player_ids)
    }

    fn collect_broken_origin_teams(
        &self,
        existing_teams: &[Team],
        busy_team_ids: &HashSet<Uuid>,
        player_ids: &[Uuid],
    ) -> HttpResult<Vec<(Team, Vec<Uuid>)>> {
        let mut broken: Vec<(Team, Vec<Uuid>)> = Vec::new();

        for player_id in player_ids {
            let Some(owner) = existing_teams.iter().find(|team| {
                team.session_id() == &self.session_id
                    && !team.is_disbanded()
                    && team.player_ids().contains(player_id)
            }) else {
                continue;
            };

            if !owner.is_draft() || busy_team_ids.contains(owner.id()) {
                return Err(Box::new(HttpError::conflict(format!(
                    "Player {player_id} is in a team that can't be broken up right now"
                ))));
            }

            match broken.iter_mut().find(|(team, _)| team.id() == owner.id()) {
                Some((_, remaining)) => remaining.retain(|id| id != player_id),
                None => {
                    let remaining = owner
                        .player_ids()
                        .iter()
                        .copied()
                        .filter(|id| id != player_id)
                        .collect();
                    broken.push((owner.clone(), remaining));
                }
            }
        }

        Ok(broken)
    }

    fn reject_duplicate_players_in_team(player_ids: &[Uuid]) -> HttpResult<()> {
        let mut seen = HashSet::with_capacity(player_ids.len());

        for player_id in player_ids {
            if !seen.insert(player_id) {
                return Err(Box::new(HttpError::bad_request(format!(
                    "Player {player_id} is duplicated in the same team"
                ))));
            }
        }

        Ok(())
    }

    fn reject_players_not_confirmed_in_session(&self, player_ids: &[Uuid]) -> HttpResult<()> {
        if let Some(player_id) = player_ids
            .iter()
            .find(|player_id| !self.session_player_ids.contains(player_id))
        {
            return Err(Box::new(HttpError::bad_request(format!(
                "Player {player_id} is not a confirmed player of this session"
            ))));
        }

        Ok(())
    }

    fn reject_players_already_in_active_team(
        &self,
        existing_teams: &[Team],
        player_ids: &[Uuid],
    ) -> HttpResult<()> {
        let players_in_session: HashSet<&Uuid> = existing_teams
            .iter()
            .filter(|team| team.session_id() == &self.session_id && !team.is_disbanded())
            .flat_map(|team| team.player_ids())
            .collect();

        if let Some(player_id) = player_ids
            .iter()
            .find(|player_id| players_in_session.contains(player_id))
        {
            return Err(Box::new(HttpError::conflict(format!(
                "Player {player_id} is already in another team for this session"
            ))));
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use http_error::HttpErrorKind;

    use super::*;

    /// `TeamStatus::Playing` is only ever written by the database trigger
    /// (see the doc comment on `TeamStatus`), so the round trip through the
    /// DB row string is exactly the surface where a typo would silently
    /// break the invariant the trigger exists to guarantee.
    #[test]
    fn test_team_status_playing_round_trips_through_its_db_string() {
        let status: TeamStatus = "PLAYING".to_string().into();

        assert_eq!(status, TeamStatus::Playing);
        assert_eq!(String::from(TeamStatus::Playing), "PLAYING");
    }

    #[test]
    fn test_new_team_starts_draft_with_no_wins() {
        let team = Team::new(Uuid::new_v4(), vec![Uuid::new_v4(), Uuid::new_v4()]);

        assert!(team.is_draft());
        assert_eq!(*team.consecutive_wins(), 0);
    }

    #[test]
    fn test_register_win_once_holds_the_court() {
        let mut team = Team::new(Uuid::new_v4(), vec![Uuid::new_v4(), Uuid::new_v4()]);

        team.register_win();

        assert!(team.is_holding());
        assert_eq!(*team.consecutive_wins(), 1);
    }

    #[test]
    fn test_register_win_twice_in_a_row_disbands_the_team() {
        let mut team = Team::new(Uuid::new_v4(), vec![Uuid::new_v4(), Uuid::new_v4()]);

        team.register_win();
        team.register_win();

        assert!(team.is_disbanded());
        assert_eq!(*team.consecutive_wins(), 2);
    }

    #[test]
    fn test_disband_is_idempotent() {
        let mut team = Team::new(Uuid::new_v4(), vec![Uuid::new_v4(), Uuid::new_v4()]);

        team.disband();
        team.disband();

        assert!(team.is_disbanded());
    }

    #[test]
    fn test_has_full_roster_is_an_exact_count_not_a_minimum() {
        assert!(!Team::new(Uuid::new_v4(), vec![Uuid::new_v4()]).has_full_roster(2));
        assert!(Team::new(Uuid::new_v4(), vec![Uuid::new_v4(), Uuid::new_v4()]).has_full_roster(2));
        // one player over the session's team size is not a valid full team
        let oversized = Team::new(
            Uuid::new_v4(),
            vec![Uuid::new_v4(), Uuid::new_v4(), Uuid::new_v4()],
        );
        assert!(!oversized.has_full_roster(2));
    }

    #[test]
    fn test_add_player_extends_the_team() {
        let player_id = Uuid::new_v4();
        let mut team = Team::new(Uuid::new_v4(), vec![Uuid::new_v4()]);

        team.add_player(player_id);

        assert!(team.player_ids().contains(&player_id));
        assert!(team.has_full_roster(2));
    }

    #[test]
    fn test_validate_new_team_with_no_existing_teams() {
        let player_ids = vec![Uuid::new_v4(), Uuid::new_v4()];
        let validator = TeamValidator::new(Uuid::new_v4(), player_ids.clone());

        assert!(validator.validate_new_team(&[], &player_ids).is_ok());
    }

    #[test]
    fn test_validate_new_team_with_single_player() {
        let player_ids = vec![Uuid::new_v4()];
        let validator = TeamValidator::new(Uuid::new_v4(), player_ids.clone());

        assert!(validator.validate_new_team(&[], &player_ids).is_ok());
    }

    #[test]
    fn test_validate_new_team_rejects_duplicated_player_in_same_team() {
        let player_id = Uuid::new_v4();
        let player_ids = vec![player_id, player_id];
        let validator = TeamValidator::new(Uuid::new_v4(), player_ids.clone());

        let err = validator.validate_new_team(&[], &player_ids).unwrap_err();

        assert_eq!(err.kind, HttpErrorKind::BadRequest);
    }

    #[test]
    fn test_validate_new_team_rejects_player_not_confirmed_in_session() {
        let session_id = Uuid::new_v4();
        let confirmed_player = Uuid::new_v4();
        let outsider_player = Uuid::new_v4();

        let validator = TeamValidator::new(session_id, vec![confirmed_player]);
        let player_ids = vec![confirmed_player, outsider_player];

        let err = validator.validate_new_team(&[], &player_ids).unwrap_err();

        assert_eq!(err.kind, HttpErrorKind::BadRequest);
    }

    #[test]
    fn test_validate_new_team_rejects_player_already_in_another_team_of_same_session() {
        let session_id = Uuid::new_v4();
        let repeated_player = Uuid::new_v4();
        let new_player = Uuid::new_v4();

        let validator = TeamValidator::new(
            session_id,
            vec![repeated_player, Uuid::new_v4(), new_player],
        );
        let existing_team = Team::new(session_id, vec![repeated_player, Uuid::new_v4()]);
        let player_ids = vec![repeated_player, new_player];

        let err = validator
            .validate_new_team(std::slice::from_ref(&existing_team), &player_ids)
            .unwrap_err();

        assert_eq!(err.kind, HttpErrorKind::Conflict);
    }

    /// A disbanded team's players are free again — they must not block a
    /// manually created team, which is exactly the contingency scenario a
    /// manual team is meant to unblock (e.g. re-forming a team right after
    /// its previous one lost and was disbanded).
    #[test]
    fn test_validate_new_team_ignores_players_from_a_disbanded_team() {
        let session_id = Uuid::new_v4();
        let freed_player = Uuid::new_v4();
        let partner = Uuid::new_v4();

        let validator = TeamValidator::new(session_id, vec![freed_player, partner]);
        let mut disbanded_team = Team::new(session_id, vec![freed_player, Uuid::new_v4()]);
        disbanded_team.disband();
        let player_ids = vec![freed_player, partner];

        let result =
            validator.validate_new_team(std::slice::from_ref(&disbanded_team), &player_ids);

        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_new_team_allows_when_no_players_overlap_with_existing_team() {
        let session_id = Uuid::new_v4();
        let player_ids = vec![Uuid::new_v4(), Uuid::new_v4()];
        let existing_team = Team::new(session_id, vec![Uuid::new_v4(), Uuid::new_v4()]);
        let validator = TeamValidator::new(session_id, player_ids.clone());

        let result = validator.validate_new_team(std::slice::from_ref(&existing_team), &player_ids);

        assert!(result.is_ok());
    }

    /// The validator scopes the check to its own `session_id`, so a team
    /// belonging to a different session must not block a shared player,
    /// even if the caller passes it in `existing_teams` by mistake.
    #[test]
    fn test_validate_new_team_ignores_teams_from_other_sessions() {
        let session_id = Uuid::new_v4();
        let other_session_id = Uuid::new_v4();
        let shared_player = Uuid::new_v4();
        let player_ids = vec![shared_player, Uuid::new_v4()];

        let validator = TeamValidator::new(session_id, player_ids.clone());
        let team_from_other_session =
            Team::new(other_session_id, vec![shared_player, Uuid::new_v4()]);

        let result = validator
            .validate_new_team(std::slice::from_ref(&team_from_other_session), &player_ids);

        assert!(result.is_ok());
    }

    /// With a mixed list, the session filter must reject the overlap coming
    /// from the same-session team and ignore the other-session one.
    #[test]
    fn test_validate_new_team_filters_by_session_in_a_mixed_team_list() {
        let session_id = Uuid::new_v4();
        let other_session_id = Uuid::new_v4();
        let repeated_player = Uuid::new_v4();
        let new_player = Uuid::new_v4();

        let validator = TeamValidator::new(session_id, vec![repeated_player, new_player]);
        let team_in_same_session = Team::new(session_id, vec![repeated_player, Uuid::new_v4()]);
        let team_in_other_session =
            Team::new(other_session_id, vec![repeated_player, Uuid::new_v4()]);
        let player_ids = vec![repeated_player, new_player];

        let err = validator
            .validate_new_team(&[team_in_same_session, team_in_other_session], &player_ids)
            .unwrap_err();

        assert_eq!(err.kind, HttpErrorKind::Conflict);
    }

    #[test]
    fn test_set_player_ids_replaces_the_whole_roster() {
        let mut team = Team::new(Uuid::new_v4(), vec![Uuid::new_v4(), Uuid::new_v4()]);
        let new_roster = vec![Uuid::new_v4(), Uuid::new_v4()];

        team.set_player_ids(new_roster.clone());

        assert_eq!(team.player_ids(), &new_roster);
    }

    #[test]
    fn test_validate_team_update_allows_a_player_free_in_the_session() {
        let session_id = Uuid::new_v4();
        let new_player_ids = vec![Uuid::new_v4(), Uuid::new_v4()];
        let validator = TeamValidator::new(session_id, new_player_ids.clone());

        let broken = validator
            .validate_team_update(&[], &HashSet::new(), &new_player_ids, &new_player_ids)
            .unwrap();

        assert!(broken.is_empty());
    }

    /// The whole point of editing a team: pulling a player out of a
    /// `Waiting` team that isn't mid-match must report that team so the
    /// caller can disband it and re-slot its now partner-less teammate.
    #[test]
    fn test_validate_team_update_reports_the_origin_team_of_a_pulled_player() {
        let session_id = Uuid::new_v4();
        let pulled_player = Uuid::new_v4();
        let stranded_partner = Uuid::new_v4();
        let staying_player = Uuid::new_v4();

        let origin_team = Team::new(session_id, vec![pulled_player, stranded_partner]);
        let origin_team_id = *origin_team.id();
        let validator = TeamValidator::new(
            session_id,
            vec![pulled_player, stranded_partner, staying_player],
        );
        let new_player_ids = vec![staying_player, pulled_player];
        let entering_player_ids = vec![pulled_player];

        let broken = validator
            .validate_team_update(
                std::slice::from_ref(&origin_team),
                &HashSet::new(),
                &new_player_ids,
                &entering_player_ids,
            )
            .unwrap();

        assert_eq!(broken.len(), 1);
        let (reported_team, remaining) = &broken[0];
        assert_eq!(reported_team.id(), &origin_team_id);
        assert_eq!(remaining, &vec![stranded_partner]);
    }

    /// A player already on the team being edited must never be treated as
    /// "stolen" from it — only genuinely new entrants are checked against
    /// other teams.
    #[test]
    fn test_validate_team_update_ignores_players_already_on_the_edited_team() {
        let session_id = Uuid::new_v4();
        let staying_player = Uuid::new_v4();
        let other_player = Uuid::new_v4();
        let validator = TeamValidator::new(session_id, vec![staying_player, other_player]);

        let broken = validator
            .validate_team_update(
                &[],
                &HashSet::new(),
                &[staying_player, other_player],
                &[other_player],
            )
            .unwrap();

        assert!(broken.is_empty());
    }

    #[test]
    fn test_validate_team_update_rejects_emptying_the_roster() {
        let session_id = Uuid::new_v4();
        let validator = TeamValidator::new(session_id, vec![]);

        let err = validator
            .validate_team_update(&[], &HashSet::new(), &[], &[])
            .unwrap_err();

        assert_eq!(err.kind, HttpErrorKind::BadRequest);
    }

    #[test]
    fn test_validate_team_update_rejects_pulling_a_player_from_a_holding_team() {
        let session_id = Uuid::new_v4();
        let player_id = Uuid::new_v4();
        let new_partner = Uuid::new_v4();
        let mut holding_team = Team::new(session_id, vec![player_id, Uuid::new_v4()]);
        holding_team.register_win();
        let validator = TeamValidator::new(session_id, vec![player_id, new_partner]);

        let err = validator
            .validate_team_update(
                std::slice::from_ref(&holding_team),
                &HashSet::new(),
                &[player_id, new_partner],
                &[player_id],
            )
            .unwrap_err();

        assert_eq!(err.kind, HttpErrorKind::Conflict);
    }
}
