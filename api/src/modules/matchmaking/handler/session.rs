use std::sync::Arc;

use async_trait::async_trait;
use http_error::{HttpError, HttpResult};
use uuid::Uuid;

use crate::modules::matchmaking::{
    domain::{
        queue::{QueueEntry, SessionQueue},
        session::Session,
        team_drawer::GamesPlayed,
    },
    handler::session::use_cases::{CreateSessionRequest, UpdateSessionRequest},
    repository::{
        matches::DynMatchRepository, player::DynPlayerRepository, queue::DynSessionQueueRepository,
        session::DynSessionRepository, team::DynTeamRepository,
    },
};

#[async_trait]
pub trait SessionHandler {
    async fn create_session(&self, request: CreateSessionRequest) -> HttpResult<Session>;

    async fn list_sessions(&self) -> HttpResult<Vec<Session>>;

    async fn get_session(&self, id: Uuid) -> HttpResult<Session>;

    async fn update_session(&self, id: Uuid, request: UpdateSessionRequest) -> HttpResult<Session>;

    /// Confirms a rostered player for the session ("check-in"): adds them to
    /// the checked-in list (`player_ids`) and puts them on the waiting queue
    /// at their games-played standing (0 for a first check-in, their prior
    /// count on a re-check-in, derived from the match record), so they become
    /// eligible for the next court draw without jumping the fair ordering.
    /// Requires the player to be on the session roster — returns `409` if
    /// not, `404` if the player does not exist. Idempotent — checking in an
    /// already-confirmed player just returns the session.
    async fn check_in(&self, session_id: Uuid, player_id: Uuid) -> HttpResult<Session>;

    /// Reverses a check-in: removes the player from the checked-in list and
    /// from the waiting queue. They stay on the roster. Idempotent —
    /// checking out a player who is not checked in just returns the session.
    async fn check_out(&self, session_id: Uuid, player_id: Uuid) -> HttpResult<Session>;

    /// The session's waiting list, in "who enters next" order.
    async fn list_queue(&self, session_id: Uuid) -> HttpResult<Vec<QueueEntry>>;

    /// Pins / unpins a queued player — pinned players jump to the front of
    /// the list (see "Prioridades" in the matchmaking skill).
    async fn set_pin(
        &self,
        session_id: Uuid,
        player_id: Uuid,
        pinned: bool,
    ) -> HttpResult<QueueEntry>;
}

pub type DynSessionHandler = dyn SessionHandler + Send + Sync;

#[derive(Clone)]
pub struct SessionHandlerImpl {
    pub session_repository: Arc<DynSessionRepository>,
    pub session_queue_repository: Arc<DynSessionQueueRepository>,
    pub player_repository: Arc<DynPlayerRepository>,
    pub team_repository: Arc<DynTeamRepository>,
    pub match_repository: Arc<DynMatchRepository>,
}

impl SessionHandlerImpl {
    /// Brings the session's waiting queue in step with a change to the
    /// checked-in list (`player_ids`): players in `next` but not `previous`
    /// join the queue, players in `previous` but not `next` leave it.
    /// Joining players re-enter at their games-played standing derived from
    /// the session's match record (so a re-check-in doesn't reset them to 0
    /// and jump the fair ordering); a player who has not played yet enters at
    /// 0. A dropped player who is mid-game stays on their court — only their
    /// queue row goes.
    async fn sync_queue_to_checked_in(
        &self,
        session_id: Uuid,
        previous: &[Uuid],
        next: &[Uuid],
    ) -> HttpResult<()> {
        let joining: Vec<Uuid> = next
            .iter()
            .copied()
            .filter(|id| !previous.contains(id))
            .collect();

        if !joining.is_empty() {
            let games_played = self.session_games_played(session_id).await?;
            let added: Vec<QueueEntry> = joining
                .iter()
                .map(|id| QueueEntry::new(session_id, *id, games_played.for_player(*id)))
                .collect();
            self.session_queue_repository.insert_many(&added).await?;
        }

        let removed: Vec<Uuid> = previous
            .iter()
            .copied()
            .filter(|id| !next.contains(id))
            .collect();
        self.session_queue_repository
            .remove_players(&session_id, &removed)
            .await?;

        Ok(())
    }

    /// The per-player finished-match count for the session, from the team
    /// and match records — the single source `PartnerHistory` also reads.
    async fn session_games_played(&self, session_id: Uuid) -> HttpResult<GamesPlayed> {
        let teams = self.team_repository.list_by_session(&session_id).await?;
        let matches = self.match_repository.list_by_session(&session_id).await?;
        Ok(GamesPlayed::from_matches(&teams, &matches))
    }
}

#[async_trait]
impl SessionHandler for SessionHandlerImpl {
    async fn create_session(&self, request: CreateSessionRequest) -> HttpResult<Session> {
        let session = Session::new(
            request.date,
            request.description,
            request.settings.unwrap_or_default(),
            request.available_courts,
            request.game_mode,
        )?;

        self.session_repository.insert(session).await
    }

    async fn list_sessions(&self) -> HttpResult<Vec<Session>> {
        self.session_repository.list().await
    }

    async fn get_session(&self, id: Uuid) -> HttpResult<Session> {
        self.session_repository
            .get(&id)
            .await?
            .ok_or_else(|| Box::new(HttpError::not_found("Session", id)))
    }

    async fn update_session(&self, id: Uuid, request: UpdateSessionRequest) -> HttpResult<Session> {
        let mut session = self.get_session(id).await?;

        if let Some(date) = request.date {
            session.set_date(date);
        }

        if let Some(description) = request.description {
            session.set_description(Some(description));
        }

        if let Some(settings) = request.settings {
            session.set_settings(settings)?;
        }

        if let Some(available_courts) = request.available_courts {
            session.set_available_courts(available_courts);
        }

        if let Some(game_mode) = request.game_mode {
            session.set_game_mode(game_mode)?;
        }

        if let Some(new_roster) = request.roster_player_ids {
            // Dropping a still-checked-in player from the roster also checks
            // them out (`Session::set_roster_player_ids` keeps that
            // invariant); their waiting-queue rows go with them.
            let checked_out = session.set_roster_player_ids(new_roster);
            if !checked_out.is_empty() {
                self.sync_queue_to_checked_in(*session.id(), &checked_out, &[])
                    .await?;
            }
        }

        self.session_repository.update(session).await
    }

    async fn check_in(&self, session_id: Uuid, player_id: Uuid) -> HttpResult<Session> {
        let mut session = self.get_session(session_id).await?;

        // `matchmaking.session.player_ids` is a bare UUID[] with no FK, so
        // an unknown id would otherwise sit silently in the checked-in list.
        if self.player_repository.get(&player_id).await?.is_none() {
            return Err(Box::new(HttpError::not_found("Player", player_id)));
        }

        // Check-in is a confirmation step layered on roster selection: the
        // player must already be on the session roster (set via PATCH).
        if !session.is_on_roster(&player_id) {
            return Err(Box::new(HttpError::conflict(
                "Player is not on the session roster",
            )));
        }

        if session.check_in_player(player_id) {
            self.sync_queue_to_checked_in(*session.id(), &[], &[player_id])
                .await?;
            return self.session_repository.update(session).await;
        }

        Ok(session)
    }

    async fn check_out(&self, session_id: Uuid, player_id: Uuid) -> HttpResult<Session> {
        let mut session = self.get_session(session_id).await?;

        if session.check_out_player(&player_id) {
            self.sync_queue_to_checked_in(*session.id(), &[player_id], &[])
                .await?;
            return self.session_repository.update(session).await;
        }

        Ok(session)
    }

    async fn list_queue(&self, session_id: Uuid) -> HttpResult<Vec<QueueEntry>> {
        let session = self.get_session(session_id).await?;
        let entries = self
            .session_queue_repository
            .list_by_session(&session_id)
            .await?;

        let queue = SessionQueue::new(
            entries,
            *session.game_mode(),
            *session.settings().players_per_team(),
        );
        Ok(queue.ordered().into_iter().cloned().collect())
    }

    async fn set_pin(
        &self,
        session_id: Uuid,
        player_id: Uuid,
        pinned: bool,
    ) -> HttpResult<QueueEntry> {
        self.session_queue_repository
            .set_pinned(&session_id, &player_id, pinned)
            .await?
            .ok_or_else(|| Box::new(HttpError::not_found("Queued player", player_id)))
    }
}

pub mod use_cases {
    use chrono::NaiveDate;
    use serde::{Deserialize, Serialize};
    use uuid::Uuid;

    use crate::modules::matchmaking::domain::session::{GameMode, SessionSettings};

    #[derive(Debug, Clone, Deserialize, Serialize)]
    #[serde(rename_all = "camelCase")]
    pub struct CreateSessionRequest {
        pub date: NaiveDate,
        pub description: Option<String>,
        pub available_courts: u8,
        pub game_mode: GameMode,
        pub settings: Option<SessionSettings>,
    }

    #[derive(Debug, Clone, Default, Deserialize, Serialize)]
    #[serde(rename_all = "camelCase")]
    pub struct UpdateSessionRequest {
        pub date: Option<NaiveDate>,
        pub description: Option<String>,
        pub available_courts: Option<u8>,
        pub game_mode: Option<GameMode>,
        pub settings: Option<SessionSettings>,
        /// The session roster (players selected/expected for the session).
        /// Replaces the list wholesale. Checking in / out is the only way to
        /// move the checked-in list (`player_ids`); this only moves the
        /// roster, except that dropping a still-checked-in player from the
        /// roster also checks them out.
        pub roster_player_ids: Option<Vec<Uuid>>,
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Mutex;

    use chrono::NaiveDate;
    use http_error::HttpErrorKind;

    use super::*;
    use crate::modules::matchmaking::{
        domain::{
            matches::Match,
            player::{Gender, Player},
            session::{GameMode, SessionSettings},
            team::Team,
        },
        repository::{
            matches::MatchRepository, player::PlayerRepository, queue::SessionQueueRepository,
            session::SessionRepository, team::TeamRepository,
        },
    };

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
            let mut rows = self.0.lock().unwrap();
            for entry in entries {
                let exists = rows.iter().any(|row| {
                    row.session_id() == entry.session_id() && row.player_id() == entry.player_id()
                });
                if !exists {
                    rows.push(entry.clone());
                }
            }
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
            _session_id: &Uuid,
            _player_id: &Uuid,
            _pinned: bool,
        ) -> HttpResult<Option<QueueEntry>> {
            Ok(None)
        }
    }

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
                .find(|t| t.id() == id)
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
                .find(|m| m.id() == id)
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

    struct World {
        handler: SessionHandlerImpl,
        session_id: Uuid,
        queue: Arc<FakeQueue>,
        teams: Arc<FakeTeams>,
        matches: Arc<FakeMatches>,
    }

    impl World {
        /// Puts a player on the session roster without checking them in — the
        /// precondition a check-in now needs.
        async fn add_to_roster(&self, player_id: Uuid) {
            let mut session = self.handler.get_session(self.session_id).await.unwrap();
            let mut roster = session.roster_player_ids().clone();
            roster.push(player_id);
            session.set_roster_player_ids(roster);
            self.handler
                .session_repository
                .update(session)
                .await
                .unwrap();
        }

        /// Records a finished match between two ad-hoc teams of the given
        /// players, so `GamesPlayed` credits each of them one game.
        async fn add_finished_game(&self, team_a: &[Uuid], team_b: &[Uuid]) {
            let a = Team::new(self.session_id, team_a.to_vec());
            let b = Team::new(self.session_id, team_b.to_vec());
            let mut m = Match::new(self.session_id, 1, *a.id(), *b.id()).unwrap();
            m.finish(*a.id()).unwrap();
            self.teams.insert(a).await.unwrap();
            self.teams.insert(b).await.unwrap();
            self.matches.insert(m).await.unwrap();
        }
    }

    /// A session with `players` already checked in (and, by the invariant,
    /// on the roster).
    fn world(checked_in: &[Player]) -> World {
        let mut session = Session::new(
            NaiveDate::from_ymd_opt(2026, 1, 1).unwrap(),
            None,
            SessionSettings::default(),
            2,
            GameMode::Open,
        )
        .unwrap();
        let ids: Vec<Uuid> = checked_in.iter().map(|p| *p.id()).collect();
        session.set_roster_player_ids(ids.clone());
        session.set_player_ids(ids);
        let session_id = *session.id();

        let queue = Arc::new(FakeQueue::default());
        let teams = Arc::new(FakeTeams::default());
        let matches = Arc::new(FakeMatches::default());
        let handler = SessionHandlerImpl {
            session_repository: Arc::new(FakeSessions(Mutex::new(vec![session]))),
            session_queue_repository: queue.clone(),
            player_repository: Arc::new(FakePlayers(Mutex::new(checked_in.to_vec()))),
            team_repository: teams.clone(),
            match_repository: matches.clone(),
        };

        World {
            handler,
            session_id,
            queue,
            teams,
            matches,
        }
    }

    fn player() -> Player {
        Player::new("P".to_string(), Gender::Male)
    }

    fn queued_ids(queue: &FakeQueue) -> Vec<Uuid> {
        queue
            .0
            .lock()
            .unwrap()
            .iter()
            .map(|entry| *entry.player_id())
            .collect()
    }

    #[tokio::test]
    async fn test_check_in_adds_to_checked_in_list_and_enqueues_with_zero_games() {
        let newcomer = player();
        let w = world(&[]);
        w.handler
            .player_repository
            .insert(newcomer.clone())
            .await
            .unwrap();
        w.add_to_roster(*newcomer.id()).await;

        let session = w
            .handler
            .check_in(w.session_id, *newcomer.id())
            .await
            .unwrap();

        assert!(session.has_player(newcomer.id()));
        let queue = w.queue.0.lock().unwrap();
        assert_eq!(queue.len(), 1);
        assert_eq!(queue[0].player_id(), newcomer.id());
        assert_eq!(*queue[0].games_played(), 0);
    }

    #[tokio::test]
    async fn test_check_in_unknown_player_is_not_found() {
        let w = world(&[]);

        let err = w
            .handler
            .check_in(w.session_id, Uuid::new_v4())
            .await
            .unwrap_err();

        assert_eq!(err.kind, HttpErrorKind::NotFound);
        assert!(queued_ids(&w.queue).is_empty());
    }

    #[tokio::test]
    async fn test_check_in_is_idempotent_and_does_not_duplicate_queue_row() {
        let already = player();
        let w = world(&[already.clone()]);
        // simulate the queue row that the first check-in would have created
        w.handler
            .session_queue_repository
            .insert_many(&[QueueEntry::new(w.session_id, *already.id(), 3)])
            .await
            .unwrap();

        let session = w
            .handler
            .check_in(w.session_id, *already.id())
            .await
            .unwrap();

        assert_eq!(session.player_ids(), &vec![*already.id()]);
        let queue = w.queue.0.lock().unwrap();
        assert_eq!(queue.len(), 1);
        // untouched: still carries the 3 games it already had
        assert_eq!(*queue[0].games_played(), 3);
    }

    #[tokio::test]
    async fn test_check_in_of_a_player_already_on_a_court_does_not_re_enqueue() {
        // On the roster, but no queue row (they are mid-game). A redundant
        // check-in must stay a no-op and never try to insert a second row.
        let on_court = player();
        let w = world(&[on_court.clone()]);

        let session = w
            .handler
            .check_in(w.session_id, *on_court.id())
            .await
            .unwrap();

        assert_eq!(session.player_ids(), &vec![*on_court.id()]);
        assert!(queued_ids(&w.queue).is_empty());
    }

    #[tokio::test]
    async fn test_check_in_tolerates_a_stale_queue_row_without_erroring() {
        // Roster and queue out of step: player left the roster while a row
        // lingered (e.g. returned from a court after check-out). Checking
        // them back in must not raise a UNIQUE violation.
        let returning = player();
        let w = world(&[]);
        w.handler
            .player_repository
            .insert(returning.clone())
            .await
            .unwrap();
        w.add_to_roster(*returning.id()).await;
        w.handler
            .session_queue_repository
            .insert_many(&[QueueEntry::new(w.session_id, *returning.id(), 2)])
            .await
            .unwrap();

        let session = w
            .handler
            .check_in(w.session_id, *returning.id())
            .await
            .unwrap();

        assert!(session.has_player(returning.id()));
        assert_eq!(queued_ids(&w.queue), vec![*returning.id()]);
    }

    #[tokio::test]
    async fn test_check_out_removes_from_roster_and_queue() {
        let leaving = player();
        let staying = player();
        let w = world(&[leaving.clone(), staying.clone()]);
        w.handler
            .session_queue_repository
            .insert_many(&[
                QueueEntry::new(w.session_id, *leaving.id(), 0),
                QueueEntry::new(w.session_id, *staying.id(), 0),
            ])
            .await
            .unwrap();

        let session = w
            .handler
            .check_out(w.session_id, *leaving.id())
            .await
            .unwrap();

        assert!(!session.has_player(leaving.id()));
        assert!(session.has_player(staying.id()));
        assert_eq!(queued_ids(&w.queue), vec![*staying.id()]);
    }

    #[tokio::test]
    async fn test_re_check_in_restores_games_played_from_match_history() {
        // Player checks in, plays 3 games, checks out, then checks back in.
        // They must re-enter the queue at 3 games — not 0 — so they don't
        // jump ahead of everyone in the fair (games-played) ordering.
        let returning = player();
        let partner = player();
        let w = world(&[returning.clone(), partner.clone()]);
        for _ in 0..3 {
            w.add_finished_game(&[*returning.id()], &[*partner.id()])
                .await;
        }

        w.handler
            .check_out(w.session_id, *returning.id())
            .await
            .unwrap();
        assert!(queued_ids(&w.queue).is_empty());

        let session = w
            .handler
            .check_in(w.session_id, *returning.id())
            .await
            .unwrap();

        assert!(session.has_player(returning.id()));
        let queue = w.queue.0.lock().unwrap();
        assert_eq!(queue.len(), 1);
        assert_eq!(queue[0].player_id(), returning.id());
        assert_eq!(*queue[0].games_played(), 3);
    }

    #[tokio::test]
    async fn test_first_check_in_of_a_player_who_never_played_enters_at_zero() {
        let newcomer = player();
        let veteran = player();
        // history exists for someone else — must not leak onto the newcomer
        let w = world(&[veteran.clone()]);
        w.handler
            .player_repository
            .insert(newcomer.clone())
            .await
            .unwrap();
        w.add_to_roster(*newcomer.id()).await;
        w.add_finished_game(&[*veteran.id()], &[Uuid::new_v4()])
            .await;

        w.handler
            .check_in(w.session_id, *newcomer.id())
            .await
            .unwrap();

        let queue = w.queue.0.lock().unwrap();
        assert_eq!(queue.len(), 1);
        assert_eq!(*queue[0].games_played(), 0);
    }

    #[tokio::test]
    async fn test_check_out_player_not_on_roster_is_a_no_op() {
        let w = world(&[]);

        let session = w
            .handler
            .check_out(w.session_id, Uuid::new_v4())
            .await
            .unwrap();

        assert!(session.player_ids().is_empty());
    }

    #[tokio::test]
    async fn test_check_in_rejects_player_not_on_roster() {
        let outsider = player();
        let w = world(&[]);
        w.handler
            .player_repository
            .insert(outsider.clone())
            .await
            .unwrap();

        let err = w
            .handler
            .check_in(w.session_id, *outsider.id())
            .await
            .unwrap_err();

        assert_eq!(err.kind, HttpErrorKind::Conflict);
        assert!(queued_ids(&w.queue).is_empty());
        let session = w.handler.get_session(w.session_id).await.unwrap();
        assert!(!session.has_player(outsider.id()));
    }

    #[tokio::test]
    async fn test_update_session_sets_roster_without_touching_checked_in_or_queue() {
        let a = player();
        let b = player();
        let w = world(&[]);

        let session = w
            .handler
            .update_session(
                w.session_id,
                UpdateSessionRequest {
                    roster_player_ids: Some(vec![*a.id(), *b.id()]),
                    ..Default::default()
                },
            )
            .await
            .unwrap();

        assert!(session.is_on_roster(a.id()));
        assert!(session.is_on_roster(b.id()));
        assert!(session.player_ids().is_empty());
        assert!(queued_ids(&w.queue).is_empty());
    }

    #[tokio::test]
    async fn test_update_session_dropping_checked_in_player_from_roster_checks_them_out() {
        let leaving = player();
        let staying = player();
        let w = world(&[leaving.clone(), staying.clone()]);
        w.handler
            .session_queue_repository
            .insert_many(&[
                QueueEntry::new(w.session_id, *leaving.id(), 0),
                QueueEntry::new(w.session_id, *staying.id(), 0),
            ])
            .await
            .unwrap();

        let session = w
            .handler
            .update_session(
                w.session_id,
                UpdateSessionRequest {
                    roster_player_ids: Some(vec![*staying.id()]),
                    ..Default::default()
                },
            )
            .await
            .unwrap();

        assert!(!session.is_on_roster(leaving.id()));
        assert!(!session.has_player(leaving.id()));
        assert!(session.has_player(staying.id()));
        assert_eq!(queued_ids(&w.queue), vec![*staying.id()]);
    }
}
