use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
};

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use http_error::{HttpError, HttpResult};
use uuid::Uuid;

use crate::modules::matchmaking::{
    domain::{
        matches::Match,
        player::Player,
        queue::{QueueEntry, SessionQueue},
        session::Session,
        team::{Team, TeamValidator},
        team_drawer::{PartnerHistory, TeamDrawer},
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

    /// Forms the opening `Draft`s for a session that has no `Match` yet:
    /// `TeamDrawer` splits the queued players (gender-aware, history empty)
    /// into up to `2 * available_courts` teams; those players leave the
    /// queue. Returns the drafts for the operator to confirm/start.
    async fn seed_queue(&self, session_id: Uuid) -> HttpResult<Vec<Team>>;

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

    /// How many finished matches each player has taken part in this session,
    /// derived straight from the teams that entered a `Match` — the same
    /// source `PartnerHistory` uses, so "games played" can never drift from
    /// the record.
    fn games_played_by_player(teams: &[Team], matches: &[Match]) -> HashMap<Uuid, u16> {
        let played_team_ids: HashSet<Uuid> = matches
            .iter()
            .filter(|match_| match_.is_finished())
            .flat_map(|match_| [*match_.team_a_id(), *match_.team_b_id()])
            .collect();

        let mut games: HashMap<Uuid, u16> = HashMap::new();
        for team in teams
            .iter()
            .filter(|team| played_team_ids.contains(team.id()))
        {
            for player_id in team.player_ids() {
                *games.entry(*player_id).or_insert(0) += 1;
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

    async fn seed_queue(&self, session_id: Uuid) -> HttpResult<Vec<Team>> {
        let session = self.load_session(session_id).await?;

        if !self
            .match_repository
            .list_by_session(&session_id)
            .await?
            .is_empty()
        {
            return Err(Box::new(HttpError::conflict(
                "Session already has matches — the queue rotates by result from here on",
            )));
        }

        let session_players = self.session_players(&session).await?;
        let queued_ids: HashSet<Uuid> = self
            .session_queue_repository
            .list_by_session(&session_id)
            .await?
            .into_iter()
            .map(|entry| *entry.player_id())
            .collect();
        let queued_players: Vec<Player> = session_players
            .into_iter()
            .filter(|player| queued_ids.contains(player.id()))
            .collect();

        let players_per_team = *session.settings().players_per_team();
        let groups = TeamDrawer::new(*session.game_mode(), players_per_team)
            .draw(&queued_players, &PartnerHistory::empty())?;

        let court_cap = usize::from(*session.available_courts()) * 2;
        let mut created = Vec::new();
        for (i, group) in groups
            .into_iter()
            .filter(|group| group.len() == usize::from(players_per_team))
            .take(court_cap)
            .enumerate()
        {
            // Pairs of opening drafts go to court 1, 2, … in order, so the
            // operator sees which two face off where.
            let court = (i / 2 + 1) as u8;
            self.session_queue_repository
                .remove_players(&session_id, &group)
                .await?;
            created.push(
                self.team_repository
                    .insert(Team::new(session_id, group).assign_court(court))
                    .await?,
            );
        }

        Ok(created)
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
