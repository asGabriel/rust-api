use std::collections::HashSet;
use std::sync::Arc;

use async_trait::async_trait;
use http_error::{ext::OptionHttpExt, HttpError, HttpResult};
use rand::{rngs::StdRng, SeedableRng};
use uuid::Uuid;

use crate::modules::volleyball::{
    domain::{
        draw::draw,
        game::{resolve_departures, Game, GameFilters},
    },
    handler::game::use_cases::{GameResultOutcome, RecordGameResultRequest},
    repository::game::DynGameRepository,
};

#[async_trait]
pub trait GameHandler {
    async fn record_result(
        &self,
        game_id: Uuid,
        request: RecordGameResultRequest,
    ) -> HttpResult<GameResultOutcome>;

    async fn list_games(&self, filters: GameFilters) -> HttpResult<Vec<Game>>;
}

pub type DynGameHandler = dyn GameHandler + Send + Sync;

#[derive(Clone)]
pub struct GameHandlerImpl {
    pub game_repository: Arc<DynGameRepository>,
}

#[async_trait]
impl GameHandler for GameHandlerImpl {
    async fn record_result(
        &self,
        game_id: Uuid,
        request: RecordGameResultRequest,
    ) -> HttpResult<GameResultOutcome> {
        let mut game = self
            .game_repository
            .get_by_id(game_id)
            .await?
            .or_not_found("game", game_id.to_string())?;

        if !game.is_pending() {
            return Err(Box::new(HttpError::bad_request(
                "Game result already recorded",
            )));
        }

        game.record_result(request.winner)?;
        let finished_game = self.game_repository.update(game).await?;

        let previous = self
            .game_repository
            .get_previous_finished_on_court(
                *finished_game.session_id(),
                *finished_game.court(),
                *finished_game.id(),
            )
            .await?;

        let outcome = resolve_departures(&finished_game, previous.as_ref())?;
        let n = outcome.slots_needed();

        let mut excluded: Vec<Uuid> = self
            .game_repository
            .list_players_in_pending_games(*finished_game.session_id())
            .await?;

        if let Some(retained_pair) = outcome.retained_pair {
            excluded.extend(retained_pair.players());
        }

        let standings = self
            .game_repository
            .compute_eligible_standings(*finished_game.session_id(), &excluded)
            .await?;

        let cooldown: HashSet<Uuid> = outcome.departing_player_ids.iter().copied().collect();
        let mut rng = StdRng::from_entropy();
        let pairs = draw(&mut rng, &standings, &cooldown, n)?;

        let new_game = match (n, outcome.retained_pair) {
            (2, Some(retained)) => Game::new_pending(
                *finished_game.session_id(),
                *finished_game.court(),
                retained,
                pairs[0],
            )?,
            (4, None) => Game::new_pending(
                *finished_game.session_id(),
                *finished_game.court(),
                pairs[0],
                pairs[1],
            )?,
            _ => {
                return Err(Box::new(HttpError::internal(
                    "Unexpected departure outcome shape",
                )))
            }
        };

        let new_game = self.game_repository.insert(new_game).await?;

        Ok(GameResultOutcome {
            finished_game,
            new_game,
        })
    }

    async fn list_games(&self, filters: GameFilters) -> HttpResult<Vec<Game>> {
        self.game_repository.list(&filters).await
    }
}

pub mod use_cases {
    use serde::{Deserialize, Serialize};

    use crate::modules::volleyball::domain::game::{Game, GameWinner};

    #[derive(Debug, Clone, Deserialize, Serialize)]
    #[serde(rename_all = "camelCase")]
    pub struct RecordGameResultRequest {
        pub winner: GameWinner,
    }

    #[derive(Debug, Clone, Serialize)]
    #[serde(rename_all = "camelCase")]
    pub struct GameResultOutcome {
        pub finished_game: Game,
        pub new_game: Game,
    }
}
