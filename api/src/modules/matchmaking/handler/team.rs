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
        team::{Team, TeamValidator},
        team_drawer::{PartnerHistory, TeamDrawer},
        team_queue::TeamQueueManager,
    },
    handler::team::use_cases::{CourtAssignment, CreateTeamRequest, ResolvedRotation},
    repository::{
        matches::DynMatchRepository, player::DynPlayerRepository, session::DynSessionRepository,
        team::DynTeamRepository,
    },
};

#[async_trait]
pub trait TeamHandler {
    /// Manually forms a team from players confirmed in the session, entering
    /// it into the queue as `Waiting` — the contingency path for an operator
    /// to force a team in (e.g. the automated draw/rotation got stuck or
    /// needs a manual correction), regardless of the session's `GameMode` or
    /// `ShuffleType`: those only constrain the automated draw and queue
    /// rotation, never a manually assembled team. Still enforces that every
    /// player is confirmed in the session and not currently in another
    /// active (non-disbanded) team of it.
    async fn create_team(&self, request: CreateTeamRequest) -> HttpResult<Team>;

    /// Manual "who plays next" override: forms a team from the given
    /// players and jumps it to the front of the waiting queue
    /// (`Team::with_priority`), so it's the next one an idle court pulls
    /// in — ahead of everyone who's been waiting longer. Unlike
    /// `create_team`, a player already in another `Waiting` team (not
    /// mid-match) can be pulled into this one: that team is disbanded and
    /// its now partner-less remaining player(s) are re-slotted back into
    /// the queue, exactly like a freed player after a match result.
    async fn create_priority_team(&self, request: CreateTeamRequest) -> HttpResult<Team>;

    async fn list_teams_by_session(&self, session_id: Uuid) -> HttpResult<Vec<Team>>;

    /// First draw of teams for a session: random pairing of the session's
    /// confirmed players, honoring the session's `GameMode` filter. Fails if
    /// the session already has teams, since it's meant to initialize them.
    async fn draw_teams(&self, session_id: Uuid) -> HttpResult<Vec<Team>>;

    /// Applies a match's result to the team rotation: the loser is always
    /// disbanded, the winner either keeps holding the court (up to the
    /// consecutive-win cap) or is disbanded too, and every player freed by
    /// either outcome is slotted back into the waiting queue. Returns a
    /// match assignment for every court the now-larger queue can fill —
    /// not just the court that just freed up: any other court left idle by
    /// an earlier result (a `Holding` team with no in-progress match,
    /// because the queue didn't have enough complete teams back then) is
    /// reconsidered too, oldest-idle-first, so a result on one court can
    /// unstick another. A court is left out of the result entirely if the
    /// queue still can't fill it.
    async fn resolve_match_result(
        &self,
        session_id: Uuid,
        winner_team_id: Uuid,
        loser_team_id: Uuid,
    ) -> HttpResult<ResolvedRotation>;
}

pub type DynTeamHandler = dyn TeamHandler + Send + Sync;

#[derive(Clone)]
pub struct TeamHandlerImpl {
    pub team_repository: Arc<DynTeamRepository>,
    pub session_repository: Arc<DynSessionRepository>,
    pub player_repository: Arc<DynPlayerRepository>,
    pub match_repository: Arc<DynMatchRepository>,
}

#[async_trait]
impl TeamHandler for TeamHandlerImpl {
    async fn create_team(&self, request: CreateTeamRequest) -> HttpResult<Team> {
        let session = self
            .session_repository
            .get(&request.session_id)
            .await?
            .ok_or_else(|| Box::new(HttpError::not_found("Session", request.session_id)))?;

        let existing_teams = self
            .team_repository
            .list_by_session(&request.session_id)
            .await?;

        TeamValidator::new(request.session_id, session.player_ids().clone())
            .validate_new_team(&existing_teams, &request.player_ids)?;

        let team = Team::new(request.session_id, request.player_ids);

        self.team_repository.insert(team).await
    }

    async fn create_priority_team(&self, request: CreateTeamRequest) -> HttpResult<Team> {
        let session = self
            .session_repository
            .get(&request.session_id)
            .await?
            .ok_or_else(|| Box::new(HttpError::not_found("Session", request.session_id)))?;

        let existing_teams = self
            .team_repository
            .list_by_session(&request.session_id)
            .await?;
        let session_matches = self
            .match_repository
            .list_by_session(&request.session_id)
            .await?;
        let busy_team_ids = Match::busy_team_ids(&session_matches);

        let broken_origin_teams =
            TeamValidator::new(request.session_id, session.player_ids().clone())
                .validate_priority_team(&existing_teams, &busy_team_ids, &request.player_ids)?;

        let history = PartnerHistory::from_matches(&existing_teams, &session_matches);
        let queue_manager = TeamQueueManager::new(
            request.session_id,
            *session.game_mode(),
            *session.settings().players_per_team(),
        );

        let session_players: Vec<_> = self
            .player_repository
            .list()
            .await?
            .into_iter()
            .filter(|player| session.player_ids().contains(player.id()))
            .collect();

        let broken_ids: HashSet<Uuid> = broken_origin_teams
            .iter()
            .map(|(team, _)| *team.id())
            .collect();
        let mut remaining_waiting_teams: Vec<Team> = existing_teams
            .into_iter()
            .filter(|team| team.is_waiting() && !broken_ids.contains(team.id()))
            .collect();

        for (mut origin, leftover_player_ids) in broken_origin_teams {
            origin.disband();
            self.team_repository.insert(origin).await?;

            if leftover_player_ids.is_empty() {
                continue;
            }

            let changed_teams = queue_manager.release_players(
                &remaining_waiting_teams,
                &leftover_player_ids,
                &session_players,
                &history,
            );

            for changed in changed_teams {
                self.team_repository.insert(changed.clone()).await?;

                match remaining_waiting_teams
                    .iter_mut()
                    .find(|team| team.id() == changed.id())
                {
                    Some(existing) => *existing = changed,
                    None => remaining_waiting_teams.push(changed),
                }
            }
        }

        let team = Team::new(request.session_id, request.player_ids).with_priority();

        self.team_repository.insert(team).await
    }

    async fn list_teams_by_session(&self, session_id: Uuid) -> HttpResult<Vec<Team>> {
        self.team_repository.list_by_session(&session_id).await
    }

    async fn draw_teams(&self, session_id: Uuid) -> HttpResult<Vec<Team>> {
        let existing_teams = self.team_repository.list_by_session(&session_id).await?;
        if !existing_teams.is_empty() {
            return Err(Box::new(HttpError::conflict(
                "Session already has teams drawn",
            )));
        }

        let session = self
            .session_repository
            .get(&session_id)
            .await?
            .ok_or_else(|| Box::new(HttpError::not_found("Session", session_id)))?;

        let session_players: Vec<_> = self
            .player_repository
            .list()
            .await?
            .into_iter()
            .filter(|player| session.player_ids().contains(player.id()))
            .collect();

        let drawn_teams =
            TeamDrawer::new(*session.game_mode(), *session.settings().players_per_team())
                .draw(&session_players, &PartnerHistory::empty())?;

        if drawn_teams.is_empty() {
            return Err(Box::new(HttpError::bad_request(
                "Not enough players to form a team with the session's game mode",
            )));
        }

        let validator = TeamValidator::new(session_id, session.player_ids().clone());
        let mut created_teams = Vec::with_capacity(drawn_teams.len());

        for player_ids in drawn_teams {
            validator.validate_new_team(&created_teams, &player_ids)?;

            let team = Team::new(session_id, player_ids);
            created_teams.push(self.team_repository.insert(team).await?);
        }

        Ok(created_teams)
    }

    async fn resolve_match_result(
        &self,
        session_id: Uuid,
        winner_team_id: Uuid,
        loser_team_id: Uuid,
    ) -> HttpResult<ResolvedRotation> {
        let session = self
            .session_repository
            .get(&session_id)
            .await?
            .ok_or_else(|| Box::new(HttpError::not_found("Session", session_id)))?;

        let session_teams = self.team_repository.list_by_session(&session_id).await?;
        let session_matches = self.match_repository.list_by_session(&session_id).await?;
        let history = PartnerHistory::from_matches(&session_teams, &session_matches);
        let busy_team_ids = Match::busy_team_ids(&session_matches);

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

        let session_players: Vec<_> = self
            .player_repository
            .list()
            .await?
            .into_iter()
            .filter(|player| session.player_ids().contains(player.id()))
            .collect();

        let queue_manager = TeamQueueManager::new(
            session_id,
            *session.game_mode(),
            *session.settings().players_per_team(),
        );

        // The winner/loser rows above are stale here (fetched before this
        // resolution mutated them), and a `Waiting` team elsewhere in the
        // session might actually be busy on another court right now — both
        // must be kept out of the queue view this rotation acts on.
        let mut waiting_teams: Vec<Team> = session_teams
            .iter()
            .filter(|team| {
                team.is_waiting()
                    && *team.id() != winner_team_id
                    && *team.id() != loser_team_id
                    && !busy_team_ids.contains(team.id())
            })
            .cloned()
            .collect();

        let changed_teams = queue_manager.release_players(
            &waiting_teams,
            &freed_player_ids,
            &session_players,
            &history,
        );

        for team in &changed_teams {
            self.team_repository.insert(team.clone()).await?;
        }

        for changed in changed_teams {
            match waiting_teams
                .iter_mut()
                .find(|team| team.id() == changed.id())
            {
                Some(existing) => *existing = changed,
                None => waiting_teams.push(changed),
            }
        }

        // Every court that needs a new match: for each court this session
        // has ever opened, look at its most recent match. If that match is
        // still unfinished, the court is genuinely busy — skip it. If it's
        // finished, the court is idle and needs refilling: by whichever of
        // its two teams is still `Holding` (the winner, if it stayed on
        // court) plus one challenger, or by two fresh teams if neither
        // survived (loser always disbands; the winner does too past the
        // consecutive-win cap — see `Team::register_win`). This is what
        // makes an old idle court reachable again from a *different*
        // court's result: nothing here is scoped to `winner_team_id`/
        // `loser_team_id`, so the court that just freed up is just one
        // instance of the same scan, not a special case. Oldest-idle-first,
        // same FIFO fairness as the player queue itself.
        let mut latest_match_by_court: HashMap<u8, &Match> = HashMap::new();
        for match_ in &session_matches {
            latest_match_by_court
                .entry(*match_.court())
                .and_modify(|latest| {
                    if match_.started_at() > latest.started_at() {
                        *latest = match_;
                    }
                })
                .or_insert(match_);
        }

        let mut slots: Vec<CourtSlot> = Vec::new();
        for (&court, latest) in &latest_match_by_court {
            if !latest.is_finished() {
                continue;
            }

            // `session_teams` was fetched before this call mutated `winner`/
            // `loser` — at that point both still read `Playing` (they were
            // mid-match), never `Holding`, so the just-finished court's own
            // pair can't be read off that stale snapshot. `winner_still_holding`
            // is the authoritative answer for `winner_team_id`; `loser_team_id`
            // is never holding (always disbanded). Every other court's pair
            // is untouched by this call, so the stale snapshot is accurate
            // for them.
            let holding_team_id = [*latest.team_a_id(), *latest.team_b_id()]
                .into_iter()
                .find(|&team_id| {
                    if team_id == winner_team_id {
                        winner_still_holding
                    } else if team_id == loser_team_id {
                        false
                    } else {
                        session_teams
                            .iter()
                            .find(|team| *team.id() == team_id)
                            .is_some_and(|team| team.is_holding() && !busy_team_ids.contains(team.id()))
                    }
                });

            slots.push(CourtSlot {
                court,
                idle_since: latest.played_at().unwrap_or(*latest.started_at()),
                fixed_team_id: holding_team_id,
                needed: if holding_team_id.is_some() { 1 } else { 2 },
            });
        }

        slots.sort_by_key(|slot| slot.idle_since);

        let mut pool = waiting_teams;
        let mut court_assignments = Vec::new();

        for slot in slots {
            let picked: Vec<Uuid> = queue_manager
                .next_complete_teams(&pool, slot.needed)
                .into_iter()
                .map(|team| *team.id())
                .collect();

            if picked.len() != slot.needed {
                continue;
            }

            pool.retain(|team| !picked.contains(team.id()));

            let (team_a_id, team_b_id) = match slot.fixed_team_id {
                Some(fixed) => (fixed, picked[0]),
                None => (picked[0], picked[1]),
            };

            court_assignments.push(CourtAssignment {
                court: slot.court,
                team_a_id,
                team_b_id,
            });
        }

        Ok(ResolvedRotation { court_assignments })
    }
}

/// A court whose most recent match is finished and needs a new one — the
/// court that just freed up is just one instance of this, found the same
/// way as any other idle court. `fixed_team_id` is the team already
/// anchored to it (a `Holding` team, still defending) — `None` when neither
/// of that court's last two teams is still around (the loser always
/// disbands, and so does a winner that hit the consecutive-win cap), so
/// both sides come fresh from the queue.
struct CourtSlot {
    court: u8,
    idle_since: DateTime<Utc>,
    fixed_team_id: Option<Uuid>,
    needed: usize,
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

    /// The outcome of applying a match result to the team rotation: a match
    /// assignment for every court the queue could fill — the one that just
    /// freed up, and any other court left idle by an earlier result that
    /// the now-larger queue can finally serve. A court missing from this
    /// list just stays idle; the queue still doesn't have what it needs.
    #[derive(Debug, Clone)]
    pub struct ResolvedRotation {
        pub court_assignments: Vec<CourtAssignment>,
    }

    #[derive(Debug, Clone)]
    pub struct CourtAssignment {
        pub court: u8,
        pub team_a_id: Uuid,
        pub team_b_id: Uuid,
    }
}
