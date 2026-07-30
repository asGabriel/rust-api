use std::collections::HashSet;

use http_error::{HttpError, HttpResult};
use rand::{seq::SliceRandom, Rng};
use uuid::Uuid;

use crate::modules::matchmaking::volleyball::domain::game::Pair;

/// A player's standing within a session: how many matches they've already
/// played and won. Used to balance both fairness of participation (rule:
/// prioritize whoever played less) and win-balance within a pair.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlayerStanding {
    pub player_id: Uuid,
    pub games_played: i32,
    pub wins: i32,
}

impl PlayerStanding {
    /// Rule: whoever just left a court sits out the very next draw. If excluding
    /// them would leave fewer than `n` eligible candidates (small roster), they're
    /// added back — least-played first — until there are enough to complete the draw.
    pub fn apply_cooldown(
        candidates: &[PlayerStanding],
        cooldown: &HashSet<Uuid>,
        n: usize,
    ) -> Vec<PlayerStanding> {
        let eligible: Vec<PlayerStanding> = candidates
            .iter()
            .copied()
            .filter(|c| !cooldown.contains(&c.player_id))
            .collect();

        if eligible.len() >= n {
            return eligible;
        }

        let mut cooling_down: Vec<PlayerStanding> = candidates
            .iter()
            .copied()
            .filter(|c| cooldown.contains(&c.player_id))
            .collect();
        cooling_down.sort_by_key(|c| c.games_played);

        let mut result = eligible;
        for candidate in cooling_down {
            if result.len() >= n {
                break;
            }
            result.push(candidate);
        }

        result
    }

    /// Rule: candidates within [min, min+1] games played get priority (a single
    /// extra match doesn't matter). The window expands until there are enough
    /// candidates to fill `n` slots, or the whole pool has been included.
    pub fn build_tolerance_group(candidates: &[PlayerStanding], n: usize) -> Vec<PlayerStanding> {
        if candidates.is_empty() {
            return Vec::new();
        }

        let min_played = candidates
            .iter()
            .map(|c| c.games_played)
            .min()
            .expect("candidates is non-empty");

        let mut window = 1;
        loop {
            let group: Vec<PlayerStanding> = candidates
                .iter()
                .copied()
                .filter(|c| c.games_played <= min_played + window)
                .collect();

            if group.len() >= n || group.len() == candidates.len() {
                return group;
            }

            window += 1;
        }
    }

    /// Randomly picks `n` players out of `group`, keeping the draw dynamic rather
    /// than deterministic.
    pub fn random_select<R: Rng>(
        rng: &mut R,
        group: &[PlayerStanding],
        n: usize,
    ) -> HttpResult<Vec<PlayerStanding>> {
        if group.len() < n {
            return Err(Box::new(HttpError::internal(format!(
                "Not enough eligible players to draw: needed {}, had {}",
                n,
                group.len()
            ))));
        }

        let mut shuffled = group.to_vec();
        shuffled.shuffle(rng);
        shuffled.truncate(n);

        Ok(shuffled)
    }

    /// Balances wins within each pair: sorts the selected players by wins
    /// descending and pairs the extremes together. For `n == 2` that's just the
    /// pair as-is; for `n == 4` it pairs rank1-with-rank4 and rank2-with-rank3,
    /// which also keeps the two resulting pairs roughly matched against each other.
    pub fn pair_by_wins(selected: &[PlayerStanding], n: usize) -> HttpResult<Vec<Pair>> {
        if selected.len() != n {
            return Err(Box::new(HttpError::internal(format!(
                "pair_by_wins expected {} players, got {}",
                n,
                selected.len()
            ))));
        }

        let mut sorted = selected.to_vec();
        sorted.sort_by(|a, b| b.wins.cmp(&a.wins));

        match n {
            2 => Ok(vec![Pair::new(sorted[0].player_id, sorted[1].player_id)]),
            4 => Ok(vec![
                Pair::new(sorted[0].player_id, sorted[3].player_id),
                Pair::new(sorted[1].player_id, sorted[2].player_id),
            ]),
            _ => Err(Box::new(HttpError::bad_request(format!(
                "Unsupported slot count for pairing: {}",
                n
            )))),
        }
    }

    /// Orchestrates the full draw: cooldown exclusion (with fallback) -> fairness
    /// tolerance group -> random pick -> win-balanced pairing.
    pub fn draw<R: Rng>(
        rng: &mut R,
        eligible: &[PlayerStanding],
        cooldown: &HashSet<Uuid>,
        n: usize,
    ) -> HttpResult<Vec<Pair>> {
        let after_cooldown = Self::apply_cooldown(eligible, cooldown, n);
        let tolerance_group = Self::build_tolerance_group(&after_cooldown, n);
        let selected = Self::random_select(rng, &tolerance_group, n)?;

        Self::pair_by_wins(&selected, n)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::{rngs::StdRng, SeedableRng};

    fn standing(byte: u8, games_played: i32, wins: i32) -> PlayerStanding {
        PlayerStanding {
            player_id: Uuid::from_bytes([byte; 16]),
            games_played,
            wins,
        }
    }

    #[test]
    fn apply_cooldown_excludes_normally() {
        let candidates = vec![standing(1, 0, 0), standing(2, 0, 0), standing(3, 0, 0)];
        let cooldown: HashSet<Uuid> = [candidates[0].player_id].into_iter().collect();

        let result = PlayerStanding::apply_cooldown(&candidates, &cooldown, 2);

        assert_eq!(result.len(), 2);
        assert!(!result
            .iter()
            .any(|c| c.player_id == candidates[0].player_id));
    }

    #[test]
    fn apply_cooldown_falls_back_when_pool_too_small() {
        let candidates = vec![standing(1, 5, 0), standing(2, 1, 0)];
        // both players are on cooldown, but we need 2 -> must pull them back
        let cooldown: HashSet<Uuid> = candidates.iter().map(|c| c.player_id).collect();

        let result = PlayerStanding::apply_cooldown(&candidates, &cooldown, 2);

        assert_eq!(result.len(), 2);
        // least-played-first: player with games_played=1 should come before games_played=5
        assert_eq!(result[0].player_id, candidates[1].player_id);
    }

    #[test]
    fn tolerance_group_keeps_min_and_min_plus_one() {
        let candidates = vec![
            standing(1, 0, 0),
            standing(2, 1, 0),
            standing(3, 5, 0), // far behind in matches played, should be excluded
        ];

        let group = PlayerStanding::build_tolerance_group(&candidates, 2);

        assert_eq!(group.len(), 2);
        assert!(!group.iter().any(|c| c.games_played == 5));
    }

    #[test]
    fn tolerance_group_expands_window_when_not_enough_candidates() {
        let candidates = vec![standing(1, 0, 0), standing(2, 3, 0), standing(3, 5, 0)];

        // min=0, min+1=1 group only has 1 member; needs 3 -> must expand until whole pool included
        let group = PlayerStanding::build_tolerance_group(&candidates, 3);

        assert_eq!(group.len(), 3);
    }

    #[test]
    fn random_select_errors_when_group_too_small() {
        let group = vec![standing(1, 0, 0)];
        let mut rng = StdRng::seed_from_u64(42);

        let result = PlayerStanding::random_select(&mut rng, &group, 2);

        assert!(result.is_err());
    }

    #[test]
    fn random_select_returns_exactly_n_distinct_players() {
        let group = vec![
            standing(1, 0, 0),
            standing(2, 0, 0),
            standing(3, 0, 0),
            standing(4, 0, 0),
        ];
        let mut rng = StdRng::seed_from_u64(7);

        let selected = PlayerStanding::random_select(&mut rng, &group, 2).unwrap();

        assert_eq!(selected.len(), 2);
        assert_ne!(selected[0].player_id, selected[1].player_id);
    }

    #[test]
    fn pair_by_wins_n2_is_trivial_pair() {
        let selected = vec![standing(1, 0, 3), standing(2, 0, 1)];

        let pairs = PlayerStanding::pair_by_wins(&selected, 2).unwrap();

        assert_eq!(pairs.len(), 1);
        assert!(pairs[0].contains(selected[0].player_id));
        assert!(pairs[0].contains(selected[1].player_id));
    }

    #[test]
    fn pair_by_wins_n4_pairs_extremes_together() {
        // wins: 10, 5, 3, 0 (rank1..rank4) -> expect (rank1,rank4) and (rank2,rank3)
        let p1 = standing(1, 0, 10);
        let p2 = standing(2, 0, 5);
        let p3 = standing(3, 0, 3);
        let p4 = standing(4, 0, 0);
        let selected = vec![p4, p1, p3, p2]; // shuffled input order

        let pairs = PlayerStanding::pair_by_wins(&selected, 4).unwrap();

        assert_eq!(pairs.len(), 2);
        assert!(pairs[0].same_players_as(&Pair::new(p1.player_id, p4.player_id)));
        assert!(pairs[1].same_players_as(&Pair::new(p2.player_id, p3.player_id)));
    }

    #[test]
    fn pair_by_wins_rejects_unsupported_n() {
        let selected = vec![standing(1, 0, 0), standing(2, 0, 0), standing(3, 0, 0)];

        let result = PlayerStanding::pair_by_wins(&selected, 3);

        assert!(result.is_err());
    }

    #[test]
    fn draw_end_to_end_produces_disjoint_balanced_pairs() {
        let eligible = vec![
            standing(1, 0, 5),
            standing(2, 0, 0),
            standing(3, 1, 2),
            standing(4, 1, 1),
            standing(5, 4, 9),
        ];
        let cooldown = HashSet::new();
        let mut rng = StdRng::seed_from_u64(123);

        let pairs = PlayerStanding::draw(&mut rng, &eligible, &cooldown, 4).unwrap();

        assert_eq!(pairs.len(), 2);
        let drawn_ids: Vec<Uuid> = pairs.iter().flat_map(|p| p.players()).collect();
        assert_eq!(drawn_ids.len(), 4);
        assert_eq!(drawn_ids.iter().collect::<HashSet<_>>().len(), 4);
        // player 5 (games_played=4) is far behind the min(0) + tolerance(1) window and must not be drawn
        assert!(!drawn_ids.contains(&eligible[4].player_id));
    }
}
