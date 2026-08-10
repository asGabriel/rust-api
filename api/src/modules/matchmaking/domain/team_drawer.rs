use std::collections::HashSet;

use http_error::{HttpError, HttpResult};
use rand::{seq::SliceRandom, thread_rng};
use uuid::Uuid;

use crate::modules::matchmaking::domain::{
    matches::Match,
    player::{Gender, Player},
    session::GameMode,
    team::Team,
};

/// Tracks which players have already been teammates in a session, derived
/// only from teams that actually took a court — a team that was drawn into
/// the queue but got reshuffled away before playing doesn't count against
/// future pairing.
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

/// Forms teams out of a pool of players, honoring the session's `GameMode`
/// filter and, best-effort, avoiding pairing up players who are already
/// teammate history according to a `PartnerHistory` (an empty history, e.g.
/// a session's first draw, simply never blocks a pairing). Players left over
/// (not enough to fill one more team) are never dropped — the last group
/// formed may be an incomplete team, so the caller can still see who's
/// waiting on a partner.
pub struct TeamDrawer {
    game_mode: GameMode,
    players_per_team: u8,
}

impl TeamDrawer {
    pub fn new(game_mode: GameMode, players_per_team: u8) -> Self {
        Self {
            game_mode,
            players_per_team,
        }
    }

    pub fn draw(&self, players: &[Player], history: &PartnerHistory) -> HttpResult<Vec<Vec<Uuid>>> {
        if self.players_per_team == 0 {
            return Err(Box::new(HttpError::bad_request(
                "Session settings must have at least one player per team",
            )));
        }
        self.game_mode
            .validate_players_per_team(self.players_per_team)?;

        match self.game_mode {
            GameMode::Male => Ok(Self::draw_pool_greedy(
                Self::player_ids_of_gender(players, Gender::Male),
                self.players_per_team.into(),
                history,
            )),
            GameMode::Female => Ok(Self::draw_pool_greedy(
                Self::player_ids_of_gender(players, Gender::Female),
                self.players_per_team.into(),
                history,
            )),
            GameMode::Mixed => Ok(self.draw_mixed(players, history)),
        }
    }

    fn draw_mixed(&self, players: &[Player], history: &PartnerHistory) -> Vec<Vec<Uuid>> {
        let players_per_gender: usize = (self.players_per_team / 2).into();

        let male_groups = Self::draw_pool_greedy(
            Self::player_ids_of_gender(players, Gender::Male),
            players_per_gender,
            history,
        );
        let mut female_groups = Self::draw_pool_greedy(
            Self::player_ids_of_gender(players, Gender::Female),
            players_per_gender,
            history,
        );

        let mut teams = Vec::with_capacity(male_groups.len());

        for male_group in male_groups {
            let Some(best_match_index) = (0..female_groups.len())
                .min_by_key(|&i| Self::cross_conflicts(&male_group, &female_groups[i], history))
            else {
                teams.push(male_group);
                continue;
            };

            let mut team = male_group;
            team.extend(female_groups.remove(best_match_index));
            teams.push(team);
        }

        teams.extend(female_groups);

        teams
    }

    fn cross_conflicts(a: &[Uuid], b: &[Uuid], history: &PartnerHistory) -> usize {
        a.iter()
            .flat_map(|x| b.iter().map(move |y| (*x, *y)))
            .filter(|(x, y)| history.have_played_together(*x, *y))
            .count()
    }

    /// Most-constrained-first greedy pairing: shuffles the pool for baseline
    /// randomness, then repeatedly anchors on whichever remaining player has
    /// the most history conflicts and fills their team with the
    /// lowest-conflict candidates available. Falls back to repeating a
    /// pairing when no conflict-free candidate is left — best-effort, not a
    /// global optimum.
    fn draw_pool_greedy(
        mut pool: Vec<Uuid>,
        team_size: usize,
        history: &PartnerHistory,
    ) -> Vec<Vec<Uuid>> {
        if team_size == 0 {
            return Vec::new();
        }

        pool.shuffle(&mut thread_rng());
        let mut teams = Vec::new();

        while !pool.is_empty() {
            if pool.len() <= team_size {
                teams.push(std::mem::take(&mut pool));
                break;
            }

            let anchor_index = (0..pool.len())
                .max_by_key(|&i| {
                    (0..pool.len())
                        .filter(|&j| j != i && history.have_played_together(pool[i], pool[j]))
                        .count()
                })
                .expect("pool is non-empty");
            let mut team = vec![pool.remove(anchor_index)];

            while team.len() < team_size {
                let candidate_index = (0..pool.len())
                    .min_by_key(|&i| {
                        team.iter()
                            .filter(|member| history.have_played_together(**member, pool[i]))
                            .count()
                    })
                    .expect("pool is non-empty");
                team.push(pool.remove(candidate_index));
            }

            teams.push(team);
        }

        teams
    }

    fn player_ids_of_gender(players: &[Player], gender: Gender) -> Vec<Uuid> {
        players
            .iter()
            .filter(|player| *player.gender() == gender)
            .map(|player| *player.id())
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use http_error::HttpErrorKind;

    use super::*;

    fn player(gender: Gender) -> Player {
        Player::new("Player".to_string(), gender)
    }

    #[test]
    fn test_draw_male_mode_only_pairs_male_players() {
        let players = vec![
            player(Gender::Male),
            player(Gender::Male),
            player(Gender::Male),
            player(Gender::Male),
            player(Gender::Female),
            player(Gender::Female),
        ];
        let male_ids: HashSet<Uuid> = players
            .iter()
            .filter(|player| *player.gender() == Gender::Male)
            .map(|player| *player.id())
            .collect();

        let teams = TeamDrawer::new(GameMode::Male, 2)
            .draw(&players, &PartnerHistory::empty())
            .unwrap();

        assert_eq!(teams.len(), 2);
        for team in teams {
            assert_eq!(team.len(), 2);
            assert!(team.iter().all(|id| male_ids.contains(id)));
        }
    }

    #[test]
    fn test_draw_female_mode_forms_an_incomplete_team_with_the_leftover_player() {
        let players = vec![
            player(Gender::Female),
            player(Gender::Female),
            player(Gender::Female),
            player(Gender::Male),
        ];

        let teams = TeamDrawer::new(GameMode::Female, 2)
            .draw(&players, &PartnerHistory::empty())
            .unwrap();

        assert_eq!(teams.len(), 2);
        let sizes: Vec<usize> = teams.iter().map(Vec::len).collect();
        assert!(sizes.contains(&2));
        assert!(sizes.contains(&1));
    }

    #[test]
    fn test_draw_mixed_mode_pairs_one_man_and_one_woman_per_team() {
        let players = vec![
            player(Gender::Male),
            player(Gender::Male),
            player(Gender::Female),
            player(Gender::Female),
        ];
        let male_ids: HashSet<Uuid> = players
            .iter()
            .filter(|player| *player.gender() == Gender::Male)
            .map(|player| *player.id())
            .collect();
        let female_ids: HashSet<Uuid> = players
            .iter()
            .filter(|player| *player.gender() == Gender::Female)
            .map(|player| *player.id())
            .collect();

        let teams = TeamDrawer::new(GameMode::Mixed, 2)
            .draw(&players, &PartnerHistory::empty())
            .unwrap();

        assert_eq!(teams.len(), 2);
        for team in teams {
            assert_eq!(team.len(), 2);
            assert_eq!(team.iter().filter(|id| male_ids.contains(id)).count(), 1);
            assert_eq!(team.iter().filter(|id| female_ids.contains(id)).count(), 1);
        }
    }

    #[test]
    fn test_draw_mixed_mode_limited_by_smaller_gender_pool_leaves_incomplete_teams() {
        let players = vec![
            player(Gender::Male),
            player(Gender::Male),
            player(Gender::Male),
            player(Gender::Female),
        ];

        let teams = TeamDrawer::new(GameMode::Mixed, 2)
            .draw(&players, &PartnerHistory::empty())
            .unwrap();

        let total_players: usize = teams.iter().map(Vec::len).sum();
        assert_eq!(total_players, 4);
        assert!(teams.iter().any(|team| team.len() == 2));
        assert!(teams.iter().any(|team| team.len() == 1));
    }

    #[test]
    fn test_draw_rejects_zero_players_per_team() {
        let err = TeamDrawer::new(GameMode::Male, 0)
            .draw(&[], &PartnerHistory::empty())
            .unwrap_err();

        assert_eq!(err.kind, HttpErrorKind::BadRequest);
    }

    #[test]
    fn test_draw_mixed_rejects_odd_players_per_team() {
        let players = vec![player(Gender::Male), player(Gender::Female)];

        let err = TeamDrawer::new(GameMode::Mixed, 3)
            .draw(&players, &PartnerHistory::empty())
            .unwrap_err();

        assert_eq!(err.kind, HttpErrorKind::BadRequest);
    }

    #[test]
    fn test_draw_avoids_repeating_a_pair_when_an_alternative_exists() {
        let players = vec![
            player(Gender::Male),
            player(Gender::Male),
            player(Gender::Male),
            player(Gender::Male),
        ];
        let ids: Vec<Uuid> = players.iter().map(|p| *p.id()).collect();

        let mut played_pairs = HashSet::new();
        played_pairs.insert(PartnerHistory::normalize(ids[0], ids[1]));
        let history = PartnerHistory { played_pairs };

        let teams = TeamDrawer::new(GameMode::Male, 2)
            .draw(&players, &history)
            .unwrap();

        assert_eq!(teams.len(), 2);
        for team in teams {
            assert!(!history.have_played_together(team[0], team[1]));
        }
    }

    #[test]
    fn test_draw_accepts_repeating_a_pair_when_forced() {
        let players = vec![player(Gender::Male), player(Gender::Male)];
        let ids: Vec<Uuid> = players.iter().map(|p| *p.id()).collect();

        let mut played_pairs = HashSet::new();
        played_pairs.insert(PartnerHistory::normalize(ids[0], ids[1]));
        let history = PartnerHistory { played_pairs };

        let teams = TeamDrawer::new(GameMode::Male, 2)
            .draw(&players, &history)
            .unwrap();

        assert_eq!(teams.len(), 1);
        assert_eq!(
            teams[0].iter().collect::<HashSet<_>>(),
            ids.iter().collect::<HashSet<_>>()
        );
    }

    #[test]
    fn test_draw_generalizes_to_teams_larger_than_two() {
        let players: Vec<Player> = (0..8).map(|_| player(Gender::Male)).collect();

        let teams = TeamDrawer::new(GameMode::Male, 4)
            .draw(&players, &PartnerHistory::empty())
            .unwrap();

        assert_eq!(teams.len(), 2);
        for team in teams {
            assert_eq!(team.len(), 4);
        }
    }
}
