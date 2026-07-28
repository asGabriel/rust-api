use std::collections::HashSet;
use std::sync::Arc;

use async_trait::async_trait;
use http_error::{ext::OptionHttpExt, HttpError, HttpResult};
use rand::{rngs::StdRng, SeedableRng};
use uuid::Uuid;

use crate::modules::volleyball::{
    domain::{
        draw::{draw, PlayerStanding},
        game::{resolve_departures, Game, GameFilters},
        session::{Session, SessionFilters},
    },
    handler::session::use_cases::{
        AddRosterPlayersRequest, CreateSessionRequest, SessionStateResponse,
    },
    repository::{game::DynGameRepository, session::DynSessionRepository},
};

/// Rule: always 2 active courts, 4 players each -> at least 8 players needed
/// to start a session.
const MIN_ROSTER_SIZE: usize = 8;

#[async_trait]
pub trait SessionHandler {
    async fn create_session(&self, request: CreateSessionRequest) -> HttpResult<Session>;

    async fn list_sessions(&self, filters: SessionFilters) -> HttpResult<Vec<Session>>;

    async fn add_players_to_roster(
        &self,
        session_id: Uuid,
        request: AddRosterPlayersRequest,
    ) -> HttpResult<Vec<crate::modules::volleyball::domain::player::Player>>;

    async fn start_session(&self, session_id: Uuid) -> HttpResult<Vec<Game>>;

    async fn get_session_state(&self, session_id: Uuid) -> HttpResult<SessionStateResponse>;

    async fn close_session(&self, session_id: Uuid) -> HttpResult<Session>;
}

pub type DynSessionHandler = dyn SessionHandler + Send + Sync;

#[derive(Clone)]
pub struct SessionHandlerImpl {
    pub session_repository: Arc<DynSessionRepository>,
    pub game_repository: Arc<DynGameRepository>,
}

#[async_trait]
impl SessionHandler for SessionHandlerImpl {
    async fn create_session(&self, request: CreateSessionRequest) -> HttpResult<Session> {
        let session = Session::new(request.session_date);
        self.session_repository.insert(session).await
    }

    async fn list_sessions(&self, filters: SessionFilters) -> HttpResult<Vec<Session>> {
        self.session_repository.list(&filters).await
    }

    async fn add_players_to_roster(
        &self,
        session_id: Uuid,
        request: AddRosterPlayersRequest,
    ) -> HttpResult<Vec<crate::modules::volleyball::domain::player::Player>> {
        self.session_repository
            .get_by_id(session_id)
            .await?
            .or_not_found("session", session_id.to_string())?;

        self.session_repository
            .add_roster_players(session_id, &request.player_ids)
            .await?;

        self.session_repository
            .list_roster_players(session_id)
            .await
    }

    async fn start_session(&self, session_id: Uuid) -> HttpResult<Vec<Game>> {
        self.session_repository
            .get_by_id(session_id)
            .await?
            .or_not_found("session", session_id.to_string())?;

        let existing_games = self
            .game_repository
            .list(&GameFilters::default().with_session_id(session_id))
            .await?;

        if !existing_games.is_empty() {
            return Err(Box::new(HttpError::bad_request("Session already started")));
        }

        let roster_ids = self
            .session_repository
            .list_roster_player_ids(session_id)
            .await?;

        if roster_ids.len() < MIN_ROSTER_SIZE {
            return Err(Box::new(HttpError::bad_request(format!(
                "Need at least {} players to start a session with 2 courts, got {}",
                MIN_ROSTER_SIZE,
                roster_ids.len()
            ))));
        }

        let mut standings: Vec<PlayerStanding> = roster_ids
            .into_iter()
            .map(|player_id| PlayerStanding {
                player_id,
                games_played: 0,
                wins: 0,
            })
            .collect();

        let mut rng = StdRng::from_entropy();
        let no_cooldown = HashSet::new();

        let court_1_pairs = draw(&mut rng, &standings, &no_cooldown, 4)?;
        let drawn_ids: HashSet<Uuid> = court_1_pairs.iter().flat_map(|p| p.players()).collect();
        standings.retain(|s| !drawn_ids.contains(&s.player_id));

        let court_2_pairs = draw(&mut rng, &standings, &no_cooldown, 4)?;

        let game_1 = Game::new_pending(session_id, 1, court_1_pairs[0], court_1_pairs[1])?;
        let game_2 = Game::new_pending(session_id, 2, court_2_pairs[0], court_2_pairs[1])?;

        let game_1 = self.game_repository.insert(game_1).await?;
        let game_2 = self.game_repository.insert(game_2).await?;

        Ok(vec![game_1, game_2])
    }

    async fn get_session_state(&self, session_id: Uuid) -> HttpResult<SessionStateResponse> {
        let session = self
            .session_repository
            .get_by_id(session_id)
            .await?
            .or_not_found("session", session_id.to_string())?;

        let roster = self
            .session_repository
            .list_roster_players(session_id)
            .await?;

        let pending_games = self
            .game_repository
            .list(
                &GameFilters::default()
                    .with_session_id(session_id)
                    .with_pending_only(true),
            )
            .await?;

        let busy_ids: HashSet<Uuid> = pending_games
            .iter()
            .flat_map(|g| {
                let mut ids = g.team_a().players().to_vec();
                ids.extend(g.team_b().players());
                ids
            })
            .collect();

        let waiting_player_ids: Vec<Uuid> = roster
            .iter()
            .map(|p| *p.id())
            .filter(|id| !busy_ids.contains(id))
            .collect();

        let departing_from_last_result = match self
            .game_repository
            .get_most_recently_finished_game(session_id)
            .await?
        {
            Some(most_recent) => {
                let previous = self
                    .game_repository
                    .get_previous_finished_on_court(
                        session_id,
                        *most_recent.court(),
                        *most_recent.id(),
                    )
                    .await?;

                resolve_departures(&most_recent, previous.as_ref())?.departing_player_ids
            }
            None => Vec::new(),
        };

        // Only surface departures that are still actually waiting: if the
        // roster was too small and the cooldown fallback already pulled them
        // back into a pending game, they're no longer "on cooldown".
        let cooldown_player_ids: Vec<Uuid> = departing_from_last_result
            .into_iter()
            .filter(|id| waiting_player_ids.contains(id))
            .collect();

        Ok(SessionStateResponse {
            session,
            roster,
            pending_games,
            waiting_player_ids,
            cooldown_player_ids,
        })
    }

    async fn close_session(&self, session_id: Uuid) -> HttpResult<Session> {
        let mut session = self
            .session_repository
            .get_by_id(session_id)
            .await?
            .or_not_found("session", session_id.to_string())?;

        let pending_games = self
            .game_repository
            .list(
                &GameFilters::default()
                    .with_session_id(session_id)
                    .with_pending_only(true),
            )
            .await?;

        if !pending_games.is_empty() {
            return Err(Box::new(HttpError::bad_request(
                "Cannot close a session with pending games",
            )));
        }

        session.close()?;
        self.session_repository.update(session).await
    }
}

pub mod use_cases {
    use chrono::NaiveDate;
    use serde::{Deserialize, Serialize};
    use uuid::Uuid;

    use crate::modules::volleyball::domain::{game::Game, player::Player, session::Session};

    #[derive(Debug, Clone, Deserialize, Serialize)]
    #[serde(rename_all = "camelCase")]
    pub struct CreateSessionRequest {
        pub session_date: NaiveDate,
    }

    #[derive(Debug, Clone, Deserialize, Serialize)]
    #[serde(rename_all = "camelCase")]
    pub struct AddRosterPlayersRequest {
        pub player_ids: Vec<Uuid>,
    }

    #[derive(Debug, Clone, Serialize)]
    #[serde(rename_all = "camelCase")]
    pub struct SessionStateResponse {
        pub session: Session,
        pub roster: Vec<Player>,
        pub pending_games: Vec<Game>,
        pub waiting_player_ids: Vec<Uuid>,
        pub cooldown_player_ids: Vec<Uuid>,
    }
}
