use std::collections::HashMap;

use uuid::Uuid;

use crate::modules::matchmaking::domain::{matches::Match, team::Team};

/// How many finished matches each player has taken part in this session,
/// derived from the teams that actually entered a `Match` — the same source
/// `PartnerHistory` uses, so a player's game count can never drift from the
/// record. Counted per finished match, not per team: a team that wins and
/// holds the court is one row in `teams` but plays several matches. Used to
/// re-seat a player in the queue at their true standing (coming off a court,
/// out of a broken draft, or on a re-check-in) rather than at 0, which would
/// jump them ahead of everyone in the fair ordering.
pub struct GamesPlayed {
    by_player: HashMap<Uuid, u16>,
}

impl GamesPlayed {
    pub fn from_matches(teams: &[Team], matches: &[Match]) -> Self {
        let roster_by_team: HashMap<Uuid, &[Uuid]> = teams
            .iter()
            .map(|team| (*team.id(), team.player_ids().as_slice()))
            .collect();

        let mut by_player: HashMap<Uuid, u16> = HashMap::new();
        for match_ in matches.iter().filter(|match_| match_.is_finished()) {
            for team_id in [match_.team_a_id(), match_.team_b_id()] {
                for player_id in roster_by_team.get(team_id).copied().unwrap_or(&[]) {
                    *by_player.entry(*player_id).or_insert(0) += 1;
                }
            }
        }

        Self { by_player }
    }

    /// The player's finished-match count this session, `0` if they have not
    /// played yet.
    pub fn for_player(&self, player_id: Uuid) -> u16 {
        self.by_player.get(&player_id).copied().unwrap_or(0)
    }
}
