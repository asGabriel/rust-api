use std::collections::HashSet;

use uuid::Uuid;

use crate::modules::matchmaking::domain::{matches::Match, team::Team};

/// Tracks which players have already been teammates in a session, derived
/// only from teams that actually took a court — a `Draft` that was formed
/// but discarded before playing doesn't count against future pairing.
///
/// Used by `SessionQueue::next_challenger` to flag (never block) a repeated
/// pairing for the operator.
pub struct PartnerHistory {
    played_pairs: HashSet<(Uuid, Uuid)>,
}

impl PartnerHistory {
    pub fn empty() -> Self {
        Self {
            played_pairs: HashSet::new(),
        }
    }

    pub fn from_matches(teams: &[Team], matches: &[Match]) -> Self {
        let played_team_ids: HashSet<Uuid> = matches
            .iter()
            .flat_map(|match_| [*match_.team_a_id(), *match_.team_b_id()])
            .collect();

        let mut played_pairs = HashSet::new();
        for team in teams
            .iter()
            .filter(|team| played_team_ids.contains(team.id()))
        {
            let player_ids = team.player_ids();
            for i in 0..player_ids.len() {
                for j in (i + 1)..player_ids.len() {
                    played_pairs.insert(Self::normalize(player_ids[i], player_ids[j]));
                }
            }
        }

        Self { played_pairs }
    }

    pub fn have_played_together(&self, a: Uuid, b: Uuid) -> bool {
        self.played_pairs.contains(&Self::normalize(a, b))
    }

    fn normalize(a: Uuid, b: Uuid) -> (Uuid, Uuid) {
        if a < b {
            (a, b)
        } else {
            (b, a)
        }
    }
}
