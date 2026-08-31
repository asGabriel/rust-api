use std::{
    collections::HashMap,
    sync::Arc,
};

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use http_error::{HttpError, HttpResult};
use uuid::Uuid;

use crate::modules::matchmaking::{
    domain::{
        matches::Match,
        partner_history::PartnerHistory,
        player::Player,
        queue::{QueueEntry, SessionQueue},
        session::Session,
        team::{Team, TeamValidator},
    },
    handler::team::use_cases::{
        CourtSuggestion, CreateTeamRequest, ResolvedRotation, UpdateTeamRequest,
    },
    repository::{
        matches::DynMatchRepository, player::DynPlayerRepository, queue::DynSessionQueueRepository,
        session::DynSessionRepository, team::DynTeamRepository,
    },
};

#[async_trait]
pub trait TeamHandler {
    /// Manually assembles a `Draft` team from queued players — the
    /// contingency path when the automatic suggestion doesn't fit. Ignores
    /// the session's `GameMode` gender rules (that only constrains the
    /// automatic path); still requires every player be confirmed in the
    /// session and not already in an active (`Draft`/`Holding`/`Playing`)
    /// team of it. The chosen players leave the session queue.
    async fn create_team(&self, request: CreateTeamRequest) -> HttpResult<Team>;

    async fn list_teams_by_session(&self, session_id: Uuid) -> HttpResult<Vec<Team>>;

    /// Replaces a `Draft` team's roster before it's started. Players entering
    /// the roster leave the queue (or, if on another `Draft`, break that
    /// draft up); players leaving the roster return to the queue.
    async fn update_team(&self, team_id: Uuid, request: UpdateTeamRequest) -> HttpResult<Team>;

    /// Discards a `Draft` that was never started: the row is deleted and its
    /// players return to the queue.
    async fn discard_draft(&self, team_id: Uuid) -> HttpResult<()>;

    /// Applies a match's result to the queue: the loser's players return to
    /// the queue; the winner keeps `Holding` the court, or — at the
    /// consecutive-win cap — is disbanded and its players return too. Then
    /// every idle court (oldest-idle first, so a result on one court can
    /// unstick another) gets a challenger `Draft` suggested from the queue
    /// via `SessionQueue::next_challenger`, the chosen players leaving the
    /// queue in the same step. Starts no match — returns the suggestions for
    /// the operator to confirm.
    async fn resolve_match_result(
        &self,
        session_id: Uuid,
        winner_team_id: Uuid,
        loser_team_id: Uuid,
    ) -> HttpResult<ResolvedRotation>;

    /// Re-runs the idle-court challenger fill on demand, with no match
    /// result to react to — for when courts are stuck `missing_challenger`
    /// and no match is running to trigger it (e.g. players arrived late and
    /// the operator just added them to the session).
    async fn refresh_idle_courts(&self, session_id: Uuid) -> HttpResult<ResolvedRotation>;
}

pub type DynTeamHandler = dyn TeamHandler + Send + Sync;

#[derive(Clone)]
pub struct TeamHandlerImpl {
    pub team_repository: Arc<DynTeamRepository>,
    pub session_repository: Arc<DynSessionRepository>,
    pub player_repository: Arc<DynPlayerRepository>,
    pub match_repository: Arc<DynMatchRepository>,
    pub session_queue_repository: Arc<DynSessionQueueRepository>,
}

impl TeamHandlerImpl {
    async fn load_session(&self, session_id: Uuid) -> HttpResult<Session> {
        self.session_repository
            .get(&session_id)
            .await?
            .ok_or_else(|| Box::new(HttpError::not_found("Session", session_id)))
    }

    async fn session_players(&self, session: &Session) -> HttpResult<Vec<Player>> {
        Ok(self
            .player_repository
            .list()
            .await?
            .into_iter()
            .filter(|player| session.player_ids().contains(player.id()))
            .collect())
    }

    /// How many finished matches each player has taken part in this session.
    /// Counted per finished match, not per team: a team that wins and holds
    /// the court is one row in `teams` but plays several matches.
    fn games_played_by_player(teams: &[Team], matches: &[Match]) -> HashMap<Uuid, u16> {
        let roster_by_team: HashMap<Uuid, &[Uuid]> = teams
            .iter()
            .map(|team| (*team.id(), team.player_ids().as_slice()))
            .collect();

        let mut games: HashMap<Uuid, u16> = HashMap::new();
        for match_ in matches.iter().filter(|match_| match_.is_finished()) {
            for team_id in [match_.team_a_id(), match_.team_b_id()] {
                for player_id in roster_by_team.get(team_id).copied().unwrap_or(&[]) {
                    *games.entry(*player_id).or_insert(0) += 1;
                }
            }
        }
        games
    }

    /// Returns `player_ids` to the session queue, each with its current
    /// games-played count and a fresh `enqueued_at` (so they sink behind
    /// everyone who has played less / waited longer).
    async fn return_players_to_queue(
        &self,
        session_id: Uuid,
        player_ids: &[Uuid],
        games_played: &HashMap<Uuid, u16>,
    ) -> HttpResult<Vec<QueueEntry>> {
        let entries: Vec<QueueEntry> = player_ids
            .iter()
            .map(|player_id| {
                QueueEntry::new(
                    session_id,
                    *player_id,
                    games_played.get(player_id).copied().unwrap_or(0),
                )
            })
            .collect();
        self.session_queue_repository.insert_many(&entries).await?;
        Ok(entries)
    }
}

#[async_trait]
impl TeamHandler for TeamHandlerImpl {
    async fn create_team(&self, request: CreateTeamRequest) -> HttpResult<Team> {
        let session = self.load_session(request.session_id).await?;

        let existing_teams = self
            .team_repository
            .list_by_session(&request.session_id)
            .await?;

        TeamValidator::new(request.session_id, session.player_ids().clone())
            .validate_new_team(&existing_teams, &request.player_ids)?;

        self.session_queue_repository
            .remove_players(&request.session_id, &request.player_ids)
            .await?;

        let team = Team::new(request.session_id, request.player_ids);
        self.team_repository.insert(team).await
    }

    async fn list_teams_by_session(&self, session_id: Uuid) -> HttpResult<Vec<Team>> {
        self.team_repository.list_by_session(&session_id).await
    }

    async fn update_team(&self, team_id: Uuid, request: UpdateTeamRequest) -> HttpResult<Team> {
        let mut team = self
            .team_repository
            .get(&team_id)
            .await?
            .ok_or_else(|| Box::new(HttpError::not_found("Team", team_id)))?;

        if !team.is_draft() {
            return Err(Box::new(HttpError::conflict(format!(
                "Team {team_id} can't be edited once it isn't a draft"
            ))));
        }

        let session = self.load_session(*team.session_id()).await?;

        let session_matches = self
            .match_repository
            .list_by_session(team.session_id())
            .await?;
        let busy_team_ids = Match::busy_team_ids(&session_matches);

        let other_teams: Vec<Team> = self
            .team_repository
            .list_by_session(team.session_id())
            .await?
            .into_iter()
            .filter(|other| other.id() != team.id())
            .collect();

        let entering_player_ids: Vec<Uuid> = request
            .player_ids
            .iter()
            .copied()
            .filter(|id| !team.player_ids().contains(id))
            .collect();
        let mut returning_player_ids: Vec<Uuid> = team
            .player_ids()
            .iter()
            .copied()
            .filter(|id| !request.player_ids.contains(id))
            .collect();

        let broken_drafts = TeamValidator::new(*team.session_id(), session.player_ids().clone())
            .validate_team_update(
                &other_teams,
                &busy_team_ids,
                &request.player_ids,
                &entering_player_ids,
            )?;

        // Entering players leave the queue; a draft they were pulled from is
        // deleted and its other players fall through to `returning`.
        self.session_queue_repository
            .remove_players(team.session_id(), &entering_player_ids)
            .await?;
        for (broken, leftover) in broken_drafts {
            self.team_repository.delete(broken.id()).await?;
            returning_player_ids.extend(leftover);
        }

        team.set_player_ids(request.player_ids);
        let updated_team = self.team_repository.insert(team).await?;

        if !returning_player_ids.is_empty() {
            let session_teams = self.team_repository.list_by_session(session.id()).await?;
            let games = Self::games_played_by_player(&session_teams, &session_matches);
            self.return_players_to_queue(*session.id(), &returning_player_ids, &games)
                .await?;
        }

        Ok(updated_team)
    }

    async fn discard_draft(&self, team_id: Uuid) -> HttpResult<()> {
        let team = self
            .team_repository
            .get(&team_id)
            .await?
            .ok_or_else(|| Box::new(HttpError::not_found("Team", team_id)))?;

        if !team.is_draft() {
            return Err(Box::new(HttpError::conflict(format!(
                "Team {team_id} isn't a draft and can't be discarded"
            ))));
        }

        self.team_repository.delete(&team_id).await?;

        let session_teams = self
            .team_repository
            .list_by_session(team.session_id())
            .await?;
        let session_matches = self
            .match_repository
            .list_by_session(team.session_id())
            .await?;
        let games = Self::games_played_by_player(&session_teams, &session_matches);
        self.return_players_to_queue(*team.session_id(), team.player_ids(), &games)
            .await?;

        Ok(())
    }

    async fn resolve_match_result(
        &self,
        session_id: Uuid,
        winner_team_id: Uuid,
        loser_team_id: Uuid,
    ) -> HttpResult<ResolvedRotation> {
        let session = self.load_session(session_id).await?;

        let session_teams = self.team_repository.list_by_session(&session_id).await?;
        let session_matches = self.match_repository.list_by_session(&session_id).await?;
        let history = PartnerHistory::from_matches(&session_teams, &session_matches);
        let games_played = Self::games_played_by_player(&session_teams, &session_matches);
        let session_players = self.session_players(&session).await?;

        let mut winner = self
            .team_repository
            .get(&winner_team_id)
            .await?
            .ok_or_else(|| Box::new(HttpError::not_found("Team", winner_team_id)))?;
        let mut loser = self
            .team_repository
            .get(&loser_team_id)
            .await?
            .ok_or_else(|| Box::new(HttpError::not_found("Team", loser_team_id)))?;

        loser.disband();
        winner.register_win();
        let winner_still_holding = winner.is_holding();

        let mut freed_player_ids = loser.player_ids().clone();
        if !winner_still_holding {
            freed_player_ids.extend(winner.player_ids().clone());
        }

        self.team_repository.insert(loser).await?;
        self.team_repository.insert(winner).await?;

        // Loser (and a capped-out winner) go back on the list, one more game
        // to their name — sinking them behind everyone who has played less.
        let mut queue_entries = self
            .session_queue_repository
            .list_by_session(&session_id)
            .await?;
        let returned = self
            .return_players_to_queue(session_id, &freed_player_ids, &games_played)
            .await?;
        queue_entries.extend(returned);

        let courts = self
            .fill_idle_courts(
                &session,
                &session_teams,
                &session_matches,
                &history,
                &session_players,
                queue_entries,
                Some(FinishedMatch {
                    winner_team_id,
                    loser_team_id,
                    winner_still_holding,
                }),
            )
            .await?;

        Ok(ResolvedRotation { courts })
    }

    async fn refresh_idle_courts(&self, session_id: Uuid) -> HttpResult<ResolvedRotation> {
        let session = self.load_session(session_id).await?;
        let session_teams = self.team_repository.list_by_session(&session_id).await?;
        let session_matches = self.match_repository.list_by_session(&session_id).await?;
        let history = PartnerHistory::from_matches(&session_teams, &session_matches);
        let session_players = self.session_players(&session).await?;
        let queue_entries = self
            .session_queue_repository
            .list_by_session(&session_id)
            .await?;

        let courts = self
            .fill_idle_courts(
                &session,
                &session_teams,
                &session_matches,
                &history,
                &session_players,
                queue_entries,
                None,
            )
            .await?;

        Ok(ResolvedRotation { courts })
    }
}

/// The just-finished match, so `fill_idle_courts` can answer "is this
/// court's winner still holding?" from the authoritative mutation instead
/// of the pre-mutation `session_teams` snapshot.
struct FinishedMatch {
    winner_team_id: Uuid,
    loser_team_id: Uuid,
    winner_still_holding: bool,
}

impl TeamHandlerImpl {
    /// Drafts a challenger for every court whose most recent match is
    /// finished and that doesn't already have a pending `Draft` waiting for
    /// the operator to confirm — oldest-idle first, so a result on one court
    /// can unstick another. Each `next_challenger` pop removes its players
    /// from the queue first (`DELETE ... RETURNING`); if a concurrent result
    /// already took one, the claimed rows are put back and the slot is
    /// skipped. Persists the drafts; starts no match.
    #[allow(clippy::too_many_arguments)]
    async fn fill_idle_courts(
        &self,
        session: &Session,
        session_teams: &[Team],
        session_matches: &[Match],
        history: &PartnerHistory,
        session_players: &[Player],
        mut queue_entries: Vec<QueueEntry>,
        finished: Option<FinishedMatch>,
    ) -> HttpResult<Vec<CourtSuggestion>> {
        let players_per_team = *session.settings().players_per_team();
        let busy_team_ids = Match::busy_team_ids(session_matches);

        let mut latest_match_by_court: HashMap<u8, &Match> = HashMap::new();
        for match_ in session_matches {
            latest_match_by_court
                .entry(*match_.court())
                .and_modify(|latest| {
                    if match_.started_at() > latest.started_at() {
                        *latest = match_;
                    }
                })
                .or_insert(match_);
        }

        let mut idle: Vec<(u8, DateTime<Utc>, Option<Uuid>)> = Vec::new();
        for (&court, latest) in &latest_match_by_court {
            if !latest.is_finished() {
                continue;
            }
            // The court already has a challenger draft the operator hasn't
            // started or discarded yet — don't stack a second one on the
            // next result (that would drain the queue into dead drafts).
            if session_teams
                .iter()
                .any(|team| team.is_draft_for_court(court))
            {
                continue;
            }

            let holding_team_id =
                [*latest.team_a_id(), *latest.team_b_id()]
                    .into_iter()
                    .find(|&team_id| match &finished {
                        Some(f) if team_id == f.winner_team_id => f.winner_still_holding,
                        Some(f) if team_id == f.loser_team_id => false,
                        _ => session_teams
                            .iter()
                            .find(|team| *team.id() == team_id)
                            .is_some_and(|team| {
                                team.is_holding() && !busy_team_ids.contains(team.id())
                            }),
                    });

            idle.push((
                court,
                latest.played_at().unwrap_or(*latest.started_at()),
                holding_team_id,
            ));
        }
        idle.sort_by_key(|(_, idle_since, _)| *idle_since);

        let mut courts = Vec::new();
        for (court, _, holding_team_id) in idle {
            let needed = if holding_team_id.is_some() { 1 } else { 2 };
            let mut draft_team_ids = Vec::new();
            let mut missing_challenger = false;

            for _ in 0..needed {
                let queue = SessionQueue::new(
                    queue_entries.clone(),
                    *session.game_mode(),
                    players_per_team,
                );
                let Some(suggestion) = queue.next_challenger(session_players, history) else {
                    missing_challenger = true;
                    break;
                };

                queue_entries.retain(|entry| !suggestion.player_ids.contains(entry.player_id()));
                let claimed = self
                    .session_queue_repository
                    .remove_players(session.id(), &suggestion.player_ids)
                    .await?;
                if claimed.len() != suggestion.player_ids.len() {
                    // A concurrent result on another court grabbed one of
                    // these players first. Put the ones we did claim back on
                    // the list and skip this slot.
                    self.session_queue_repository.insert_many(&claimed).await?;
                    queue_entries.extend(claimed);
                    missing_challenger = true;
                    break;
                }
                let draft = self
                    .team_repository
                    .insert(Team::new(*session.id(), suggestion.player_ids).assign_court(court))
                    .await?;
                draft_team_ids.push(*draft.id());
            }

            courts.push(CourtSuggestion {
                court,
                holding_team_id,
                draft_team_ids,
                missing_challenger,
            });
        }

        Ok(courts)
    }
}

pub mod use_cases {
    use serde::{Deserialize, Serialize};
    use uuid::Uuid;

    #[derive(Debug, Clone, Deserialize, Serialize)]
    #[serde(rename_all = "camelCase")]
    pub struct CreateTeamRequest {
        pub session_id: Uuid,
        pub player_ids: Vec<Uuid>,
    }

    #[derive(Debug, Clone, Deserialize, Serialize)]
    #[serde(rename_all = "camelCase")]
    pub struct UpdateTeamRequest {
        pub player_ids: Vec<Uuid>,
    }

    /// What a reported result did to the queue, per court: the `Holding`
    /// team still defending (if any) and the challenger `Draft`(s) the queue
    /// suggested, for the operator to confirm/edit/start. `missing_challenger`
    /// means the queue couldn't supply a full team for that court (e.g. a
    /// `Mixed` gender imbalance) — it stays idle until the operator builds
    /// one by hand or the queue fills. A court absent from the list simply
    /// had no finished match to react to.
    #[derive(Debug, Clone, Serialize)]
    #[serde(rename_all = "camelCase")]
    pub struct ResolvedRotation {
        pub courts: Vec<CourtSuggestion>,
    }

    #[derive(Debug, Clone, Serialize)]
    #[serde(rename_all = "camelCase")]
    pub struct CourtSuggestion {
        pub court: u8,
        pub holding_team_id: Option<Uuid>,
        pub draft_team_ids: Vec<Uuid>,
        pub missing_challenger: bool,
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Mutex;

    use chrono::NaiveDate;

    use super::*;
    use crate::modules::matchmaking::{
        domain::{
            player::{Gender, Player},
            session::{GameMode, Session, SessionSettings},
        },
        repository::{
            matches::MatchRepository, player::PlayerRepository, queue::SessionQueueRepository,
            session::SessionRepository, team::TeamRepository,
        },
    };

    #[derive(Default)]
    struct FakeTeams(Mutex<Vec<Team>>);

    #[async_trait]
    impl TeamRepository for FakeTeams {
        async fn insert(&self, team: Team) -> HttpResult<Team> {
            let mut rows = self.0.lock().unwrap();
            match rows.iter_mut().find(|row| row.id() == team.id()) {
                Some(row) => *row = team.clone(),
                None => rows.push(team.clone()),
            }
            Ok(team)
        }
        async fn list_by_session(&self, session_id: &Uuid) -> HttpResult<Vec<Team>> {
            Ok(self
                .0
                .lock()
                .unwrap()
                .iter()
                .filter(|team| team.session_id() == session_id)
                .cloned()
                .collect())
        }
        async fn get(&self, id: &Uuid) -> HttpResult<Option<Team>> {
            Ok(self
                .0
                .lock()
                .unwrap()
                .iter()
                .find(|team| team.id() == id)
                .cloned())
        }
        async fn delete(&self, id: &Uuid) -> HttpResult<()> {
            self.0.lock().unwrap().retain(|team| team.id() != id);
            Ok(())
        }
    }

    #[derive(Default)]
    struct FakeMatches(Mutex<Vec<Match>>);

    #[async_trait]
    impl MatchRepository for FakeMatches {
        async fn insert(&self, match_: Match) -> HttpResult<Match> {
            self.0.lock().unwrap().push(match_.clone());
            Ok(match_)
        }
        async fn list_by_session(&self, session_id: &Uuid) -> HttpResult<Vec<Match>> {
            Ok(self
                .0
                .lock()
                .unwrap()
                .iter()
                .filter(|match_| match_.session_id() == session_id)
                .cloned()
                .collect())
        }
        async fn get(&self, id: &Uuid) -> HttpResult<Option<Match>> {
            Ok(self
                .0
                .lock()
                .unwrap()
                .iter()
                .find(|match_| match_.id() == id)
                .cloned())
        }
        async fn update(&self, match_: Match) -> HttpResult<Match> {
            let mut rows = self.0.lock().unwrap();
            if let Some(row) = rows.iter_mut().find(|row| row.id() == match_.id()) {
                *row = match_.clone();
            }
            Ok(match_)
        }
    }

    struct FakeSessions(Mutex<Vec<Session>>);

    #[async_trait]
    impl SessionRepository for FakeSessions {
        async fn insert(&self, session: Session) -> HttpResult<Session> {
            self.0.lock().unwrap().push(session.clone());
            Ok(session)
        }
        async fn list(&self) -> HttpResult<Vec<Session>> {
            Ok(self.0.lock().unwrap().clone())
        }
        async fn get(&self, id: &Uuid) -> HttpResult<Option<Session>> {
            Ok(self
                .0
                .lock()
                .unwrap()
                .iter()
                .find(|session| session.id() == id)
                .cloned())
        }
        async fn update(&self, session: Session) -> HttpResult<Session> {
            let mut rows = self.0.lock().unwrap();
            if let Some(row) = rows.iter_mut().find(|row| row.id() == session.id()) {
                *row = session.clone();
            }
            Ok(session)
        }
    }

    #[derive(Default)]
    struct FakePlayers(Mutex<Vec<Player>>);

    #[async_trait]
    impl PlayerRepository for FakePlayers {
        async fn insert(&self, player: Player) -> HttpResult<Player> {
            self.0.lock().unwrap().push(player.clone());
            Ok(player)
        }
        async fn list(&self) -> HttpResult<Vec<Player>> {
            Ok(self.0.lock().unwrap().clone())
        }
        async fn get(&self, id: &Uuid) -> HttpResult<Option<Player>> {
            Ok(self
                .0
                .lock()
                .unwrap()
                .iter()
                .find(|player| player.id() == id)
                .cloned())
        }
        async fn update(&self, player: Player) -> HttpResult<Player> {
            Ok(player)
        }
    }

    #[derive(Default)]
    struct FakeQueue(Mutex<Vec<QueueEntry>>);

    #[async_trait]
    impl SessionQueueRepository for FakeQueue {
        async fn list_by_session(&self, session_id: &Uuid) -> HttpResult<Vec<QueueEntry>> {
            Ok(self
                .0
                .lock()
                .unwrap()
                .iter()
                .filter(|entry| entry.session_id() == session_id)
                .cloned()
                .collect())
        }
        async fn insert_many(&self, entries: &[QueueEntry]) -> HttpResult<()> {
            self.0.lock().unwrap().extend(entries.iter().cloned());
            Ok(())
        }
        async fn remove_players(
            &self,
            session_id: &Uuid,
            player_ids: &[Uuid],
        ) -> HttpResult<Vec<QueueEntry>> {
            let mut rows = self.0.lock().unwrap();
            let mut removed = Vec::new();
            rows.retain(|entry| {
                if entry.session_id() == session_id && player_ids.contains(entry.player_id()) {
                    removed.push(entry.clone());
                    false
                } else {
                    true
                }
            });
            Ok(removed)
        }
        async fn set_pinned(
            &self,
            session_id: &Uuid,
            player_id: &Uuid,
            pinned: bool,
        ) -> HttpResult<Option<QueueEntry>> {
            let mut rows = self.0.lock().unwrap();
            let found = rows
                .iter_mut()
                .find(|entry| entry.session_id() == session_id && entry.player_id() == player_id);
            match found {
                Some(entry) => {
                    entry.set_pinned(pinned);
                    Ok(Some(entry.clone()))
                }
                None => Ok(None),
            }
        }
    }

    struct World {
        handler: TeamHandlerImpl,
        session_id: Uuid,
        teams: Arc<FakeTeams>,
        queue: Arc<FakeQueue>,
    }

    impl World {
        fn add_teams(&self, teams: impl IntoIterator<Item = Team>) {
            self.teams.0.lock().unwrap().extend(teams);
        }

        async fn add_finished_match(&self, court: u8, team_a: &Team, team_b: &Team, winner: &Team) {
            let mut m = Match::new(self.session_id, court, *team_a.id(), *team_b.id()).unwrap();
            m.finish(*winner.id()).unwrap();
            self.handler.match_repository.insert(m).await.unwrap();
        }

        async fn enqueue(&self, players: &[(&Player, u16)]) {
            let entries: Vec<QueueEntry> = players
                .iter()
                .map(|(player, games)| QueueEntry::new(self.session_id, *player.id(), *games))
                .collect();
            self.handler
                .session_queue_repository
                .insert_many(&entries)
                .await
                .unwrap();
        }

        fn drafts_by_court(&self) -> Vec<u8> {
            self.teams
                .0
                .lock()
                .unwrap()
                .iter()
                .filter_map(|team| if team.is_draft() { *team.court() } else { None })
                .collect()
        }
    }

    fn world(game_mode: GameMode, available_courts: u8, players: &[Player]) -> World {
        let mut session = Session::new(
            NaiveDate::from_ymd_opt(2026, 1, 1).unwrap(),
            None,
            SessionSettings::default(),
            available_courts,
            game_mode,
        )
        .unwrap();
        session.set_player_ids(players.iter().map(|player| *player.id()).collect());
        let session_id = *session.id();

        let teams = Arc::new(FakeTeams::default());
        let queue = Arc::new(FakeQueue::default());
        let handler = TeamHandlerImpl {
            team_repository: teams.clone(),
            session_repository: Arc::new(FakeSessions(Mutex::new(vec![session]))),
            player_repository: Arc::new(FakePlayers(Mutex::new(players.to_vec()))),
            match_repository: Arc::new(FakeMatches::default()),
            session_queue_repository: queue.clone(),
        };

        World {
            handler,
            session_id,
            teams,
            queue,
        }
    }

    fn male() -> Player {
        Player::new("P".to_string(), Gender::Male)
    }

    fn female() -> Player {
        Player::new("P".to_string(), Gender::Female)
    }

    /// The loser comes back on the list with one more game to its name and
    /// is deprioritised behind fresher players; the winner keeps the court
    /// and gets exactly one challenger draft, tagged with its court.
    #[tokio::test]
    async fn test_resolve_holds_the_winner_and_drafts_one_challenger_from_the_queue() {
        let (a, b, c, d, e, f) = (male(), male(), male(), male(), male(), male());
        let w = world(
            GameMode::Male,
            1,
            &[
                a.clone(),
                b.clone(),
                c.clone(),
                d.clone(),
                e.clone(),
                f.clone(),
            ],
        );

        let ab = Team::new(w.session_id, vec![*a.id(), *b.id()]);
        let cd = Team::new(w.session_id, vec![*c.id(), *d.id()]);
        w.add_teams([ab.clone(), cd.clone()]);
        w.add_finished_match(1, &ab, &cd, &ab).await;
        w.enqueue(&[(&e, 0), (&f, 0)]).await;

        let rotation = w
            .handler
            .resolve_match_result(w.session_id, *ab.id(), *cd.id())
            .await
            .unwrap();

        assert_eq!(rotation.courts.len(), 1);
        let court = &rotation.courts[0];
        assert_eq!(court.court, 1);
        assert_eq!(court.holding_team_id, Some(*ab.id()));
        assert_eq!(court.draft_team_ids.len(), 1);
        assert!(!court.missing_challenger);

        let teams = w.teams.0.lock().unwrap();
        assert!(teams
            .iter()
            .find(|t| t.id() == ab.id())
            .unwrap()
            .is_holding());
        assert!(teams
            .iter()
            .find(|t| t.id() == cd.id())
            .unwrap()
            .is_disbanded());
        let draft = teams
            .iter()
            .find(|t| t.is_draft_for_court(1))
            .expect("a draft tagged for court 1");
        // the fresher players (0 games) get drafted, not the returning losers
        let mut drafted = draft.player_ids().clone();
        drafted.sort();
        let mut expected = vec![*e.id(), *f.id()];
        expected.sort();
        assert_eq!(drafted, expected);
        drop(teams);

        // C and D are back on the list, one game each, still waiting
        let queue = w.queue.0.lock().unwrap();
        for loser in [c.id(), d.id()] {
            assert_eq!(
                *queue
                    .iter()
                    .find(|q| q.player_id() == loser)
                    .unwrap()
                    .games_played(),
                1
            );
        }
    }

    /// A winner past the consecutive-win cap is disbanded too, so the court
    /// needs two fresh challenger drafts, not one.
    #[tokio::test]
    async fn test_resolve_drafts_two_challengers_when_the_winner_hits_the_cap() {
        let (a, b, c, d, e, f) = (male(), male(), male(), male(), male(), male());
        let w = world(
            GameMode::Male,
            1,
            &[
                a.clone(),
                b.clone(),
                c.clone(),
                d.clone(),
                e.clone(),
                f.clone(),
            ],
        );

        let mut ab = Team::new(w.session_id, vec![*a.id(), *b.id()]);
        ab.register_win(); // already won once — this result is the 2nd
        let cd = Team::new(w.session_id, vec![*c.id(), *d.id()]);
        w.add_teams([ab.clone(), cd.clone()]);
        w.add_finished_match(1, &ab, &cd, &ab).await;
        w.enqueue(&[(&e, 0), (&f, 0)]).await;

        let rotation = w
            .handler
            .resolve_match_result(w.session_id, *ab.id(), *cd.id())
            .await
            .unwrap();

        assert_eq!(rotation.courts.len(), 1);
        assert_eq!(rotation.courts[0].holding_team_id, None);
        assert_eq!(rotation.courts[0].draft_team_ids.len(), 2);

        let teams = w.teams.0.lock().unwrap();
        assert!(teams
            .iter()
            .find(|t| t.id() == ab.id())
            .unwrap()
            .is_disbanded());
        assert_eq!(teams.iter().filter(|t| t.is_draft_for_court(1)).count(), 2);
    }

    /// `Mixed` with no player of a needed gender left in the queue → the
    /// court is reported `missing_challenger` and no draft is created.
    #[tokio::test]
    async fn test_resolve_reports_missing_challenger_when_a_gender_runs_out() {
        let (m1, f1, m2, m3) = (male(), female(), male(), male());
        let w = world(
            GameMode::Mixed,
            1,
            &[m1.clone(), f1.clone(), m2.clone(), m3.clone()],
        );

        // t2 is deliberately mis-composed (two men) — resolve doesn't police
        // team gender; the point is the queue ends up all-male.
        let t1 = Team::new(w.session_id, vec![*m1.id(), *f1.id()]);
        let t2 = Team::new(w.session_id, vec![*m2.id(), *m3.id()]);
        w.add_teams([t1.clone(), t2.clone()]);
        w.add_finished_match(1, &t1, &t2, &t1).await;

        let rotation = w
            .handler
            .resolve_match_result(w.session_id, *t1.id(), *t2.id())
            .await
            .unwrap();

        assert_eq!(rotation.courts.len(), 1);
        assert!(rotation.courts[0].missing_challenger);
        assert!(rotation.courts[0].draft_team_ids.is_empty());
        assert_eq!(
            w.teams
                .0
                .lock()
                .unwrap()
                .iter()
                .filter(|t| t.is_draft())
                .count(),
            0
        );
    }

    /// Regression for the guardian's C1: a court that already has a pending
    /// challenger draft is NOT filled again when a *different* court's result
    /// comes in. Two successive results leave two drafts total — one per
    /// court — never three.
    #[tokio::test]
    async fn test_resolve_does_not_stack_drafts_on_a_court_that_already_has_a_pending_draft() {
        let players: Vec<Player> = (0..8).map(|_| male()).collect();
        let w = world(GameMode::Male, 2, &players);

        let ab = Team::new(w.session_id, vec![*players[0].id(), *players[1].id()]);
        let cd = Team::new(w.session_id, vec![*players[2].id(), *players[3].id()]);
        let ef = Team::new(w.session_id, vec![*players[4].id(), *players[5].id()]);
        let gh = Team::new(w.session_id, vec![*players[6].id(), *players[7].id()]);
        w.add_teams([ab.clone(), cd.clone(), ef.clone(), gh.clone()]);

        // court 1 done (AB beat CD); court 2 still running (EF vs GH)
        w.add_finished_match(1, &ab, &cd, &ab).await;
        let court2_match = Match::new(w.session_id, 2, *ef.id(), *gh.id()).unwrap();
        let court2_match_id = *court2_match.id();
        w.handler
            .match_repository
            .insert(court2_match)
            .await
            .unwrap();

        let first = w
            .handler
            .resolve_match_result(w.session_id, *ab.id(), *cd.id())
            .await
            .unwrap();
        assert_eq!(first.courts.len(), 1);
        assert_eq!(first.courts[0].court, 1);
        assert_eq!(w.drafts_by_court(), vec![1]);

        // court 2's match now finishes
        let mut m2 = w
            .handler
            .match_repository
            .get(&court2_match_id)
            .await
            .unwrap()
            .unwrap();
        m2.finish(*ef.id()).unwrap();
        w.handler.match_repository.update(m2).await.unwrap();

        let second = w
            .handler
            .resolve_match_result(w.session_id, *ef.id(), *gh.id())
            .await
            .unwrap();

        // only court 2 is filled — court 1 already has its pending draft
        assert_eq!(second.courts.len(), 1);
        assert_eq!(second.courts[0].court, 2);
        let mut drafts = w.drafts_by_court();
        drafts.sort();
        assert_eq!(drafts, vec![1, 2], "one draft per court, not three");
    }
}
