use std::collections::HashSet;

use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::modules::matchmaking::domain::{
    player::{Gender, Player},
    session::GameMode,
    team::Team,
    team_drawer::{PartnerHistory, TeamDrawer},
};

/// Keeps the session's waiting queue moving: when players are freed (a team
/// lost, or a team hit the consecutive-win cap), regroups the currently
/// unteamed players and answers "who's next" so a freed-up court can be
/// refilled automatically. Bound to a `session_id`, `game_mode` and
/// `players_per_team`, the same values a `TeamDrawer` for this session
/// would use, since the queue reuses that same drawer for its own grouping.
pub struct TeamQueueManager {
    session_id: Uuid,
    game_mode: GameMode,
    players_per_team: u8,
}

impl TeamQueueManager {
    pub fn new(session_id: Uuid, game_mode: GameMode, players_per_team: u8) -> Self {
        Self {
            session_id,
            game_mode,
            players_per_team,
        }
    }

    /// Regroups every currently unteamed player — the members of
    /// still-incomplete `Waiting` teams in `waiting_teams`, plus whoever
    /// `freed_player_ids` just added — into as many complete teams as the
    /// pool allows. Returns every team that was created, disbanded, or
    /// otherwise changed, for the caller to persist; a pre-existing
    /// incomplete team with no member selected this round is left
    /// completely untouched (not included).
    ///
    /// Two rules, in order:
    /// 1. **Nobody returns immediately.** Whoever's been unteamed longest
    ///    gets to fill a team slot first — a player freed by this very call
    ///    only gets pulled in when there aren't enough longer-waiting
    ///    players to fill the quota without them.
    /// 2. **Mix as much as `game_mode` allows.** Whichever players do get
    ///    selected are grouped by handing them straight to `TeamDrawer` —
    ///    the exact same best-effort, gender-aware, forced-repeat-only-when-
    ///    truly-necessary pairing already used (and tested) for the
    ///    session's initial draw, so this never invents a second, subtly
    ///    different notion of "avoid repeating a partner."
    ///
    /// This never leaves the queue permanently stuck: as long as the pool
    /// has enough compatible players for one team, this call forms one —
    /// unlike a design that keeps *some* teams waiting indefinitely for a
    /// fresher partner, which can leave a session with no complete team
    /// anywhere and no future match (and so no future call to this
    /// function) to ever revisit them.
    pub fn release_players(
        &self,
        waiting_teams: &[Team],
        freed_player_ids: &[Uuid],
        players: &[Player],
        history: &PartnerHistory,
    ) -> Vec<Team> {
        let now = Utc::now();

        let old_incomplete: Vec<&Team> = waiting_teams
            .iter()
            .filter(|team| team.is_waiting() && !team.is_complete(self.players_per_team))
            .collect();

        let mut pool: Vec<(Uuid, DateTime<Utc>)> = old_incomplete
            .iter()
            .flat_map(|team| {
                team.player_ids()
                    .iter()
                    .map(|player_id| (*player_id, *team.created_at()))
            })
            .collect();
        pool.extend(freed_player_ids.iter().map(|player_id| (*player_id, now)));

        let selected: HashSet<Uuid> = self.select_playable(&pool, players).into_iter().collect();

        // A priority team that was incomplete keeps that flag when it gets
        // completed here — otherwise it would silently lose its guaranteed
        // "next up" spot the instant a partner freed up for it.
        let priority_player_ids: HashSet<Uuid> = old_incomplete
            .iter()
            .filter(|team| team.is_priority())
            .flat_map(|team| team.player_ids().iter().copied())
            .collect();

        let selected_players: Vec<Player> = players
            .iter()
            .filter(|player| selected.contains(player.id()))
            .cloned()
            .collect();

        let groups = TeamDrawer::new(self.game_mode, self.players_per_team)
            .draw(&selected_players, history)
            .unwrap_or_default();

        let mut result: Vec<Team> = groups
            .into_iter()
            .map(|group| {
                let team = Team::new(self.session_id, group.clone());
                if group
                    .iter()
                    .any(|player_id| priority_player_ids.contains(player_id))
                {
                    team.with_priority()
                } else {
                    team
                }
            })
            .collect();

        // A pre-existing incomplete team loses one of its members to a new
        // team above — disband it so the DB stops showing it as Waiting.
        // Untouched ones (nobody of theirs got selected) are left alone.
        let mut already_represented = HashSet::new();
        for team in &old_incomplete {
            if team
                .player_ids()
                .iter()
                .any(|player_id| selected.contains(player_id))
            {
                let mut disbanded = (*team).clone();
                disbanded.disband();
                result.push(disbanded);
            } else {
                already_represented.extend(team.player_ids().iter().copied());
            }
        }

        // Everyone left over — not selected this round, and not already
        // represented by an untouched pre-existing incomplete team — waits
        // as their own new single-player team.
        for (player_id, _) in pool {
            if !selected.contains(&player_id) && !already_represented.contains(&player_id) {
                result.push(Team::new(self.session_id, vec![player_id]));
            }
        }

        result
    }

    /// Picks exactly enough players from `pool` — oldest-waiting first — to
    /// form as many complete teams as `game_mode`'s gender rules currently
    /// allow, never more. In `Mixed` mode the two genders are queued and
    /// capped independently, since a team needs an even split of both.
    fn select_playable(&self, pool: &[(Uuid, DateTime<Utc>)], players: &[Player]) -> Vec<Uuid> {
        let players_per_team: usize = self.players_per_team.into();
        if players_per_team == 0 {
            return Vec::new();
        }

        if self.game_mode.is_mixed() {
            let target_per_gender = players_per_team / 2;

            let mut males: Vec<&(Uuid, DateTime<Utc>)> = pool
                .iter()
                .filter(|(player_id, _)| Self::gender_of(players, *player_id) == Gender::Male)
                .collect();
            let mut females: Vec<&(Uuid, DateTime<Utc>)> = pool
                .iter()
                .filter(|(player_id, _)| Self::gender_of(players, *player_id) == Gender::Female)
                .collect();
            males.sort_by_key(|(_, queued_since)| *queued_since);
            females.sort_by_key(|(_, queued_since)| *queued_since);

            let team_count = (males.len() / target_per_gender).min(females.len() / target_per_gender);

            males
                .into_iter()
                .take(team_count * target_per_gender)
                .chain(females.into_iter().take(team_count * target_per_gender))
                .map(|(player_id, _)| *player_id)
                .collect()
        } else {
            let mut ordered: Vec<&(Uuid, DateTime<Utc>)> = pool.iter().collect();
            ordered.sort_by_key(|(_, queued_since)| *queued_since);

            let team_count = ordered.len() / players_per_team;

            ordered
                .into_iter()
                .take(team_count * players_per_team)
                .map(|(player_id, _)| *player_id)
                .collect()
        }
    }

    /// The next `count` complete teams in the queue — priority teams
    /// (`Team::with_priority`, the manual "who plays next" override) always
    /// come first, oldest-priority-first; then everyone else, oldest first.
    /// These are the ones that should auto-fill a court that just freed up.
    pub fn next_complete_teams<'a>(
        &self,
        waiting_teams: &'a [Team],
        count: usize,
    ) -> Vec<&'a Team> {
        let mut complete: Vec<&Team> = waiting_teams
            .iter()
            .filter(|team| team.is_waiting() && team.is_complete(self.players_per_team))
            .collect();
        complete.sort_by_key(|team| (!team.is_priority(), *team.created_at()));

        complete.into_iter().take(count).collect()
    }

    fn gender_of(players: &[Player], player_id: Uuid) -> Gender {
        players
            .iter()
            .find(|player| *player.id() == player_id)
            .map(|player| *player.gender())
            .expect("freed player must be part of the session roster")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::modules::matchmaking::domain::matches::Match;

    fn player(gender: Gender) -> Player {
        Player::new("Player".to_string(), gender)
    }

    #[test]
    fn test_release_players_completes_the_waiting_incomplete_team_and_disbands_the_old_row() {
        let session_id = Uuid::new_v4();
        let waiting_alone = player(Gender::Male);
        let freed_a = player(Gender::Male);
        let freed_b = player(Gender::Male);
        let players = vec![waiting_alone.clone(), freed_a.clone(), freed_b.clone()];

        let incomplete_team = Team::new(session_id, vec![*waiting_alone.id()]);
        std::thread::sleep(std::time::Duration::from_millis(2));
        let manager = TeamQueueManager::new(session_id, GameMode::Male, 2);

        let changed = manager.release_players(
            &[incomplete_team.clone()],
            &[*freed_a.id(), *freed_b.id()],
            &players,
            &PartnerHistory::empty(),
        );

        // The old incomplete row is disbanded rather than reused, a new
        // complete team is created for `waiting_alone` plus whichever freed
        // player it paired with, and the other freed player waits alone.
        assert_eq!(changed.len(), 3);

        let disbanded_old_row = changed
            .iter()
            .find(|team| *team.id() == *incomplete_team.id())
            .expect("the old incomplete row should still be reported so its status updates");
        assert!(!disbanded_old_row.is_waiting());

        let completed = changed
            .iter()
            .find(|team| team.is_complete(2) && team.player_ids().contains(waiting_alone.id()))
            .expect("waiting_alone should have been completed into a new team");
        assert!(
            completed.player_ids().contains(freed_a.id())
                || completed.player_ids().contains(freed_b.id())
        );

        let leftover_id = if completed.player_ids().contains(freed_a.id()) {
            freed_b.id()
        } else {
            freed_a.id()
        };
        let new_incomplete = changed
            .iter()
            .find(|team| team.player_ids() == &vec![*leftover_id])
            .expect("the other freed player should start a new incomplete team");
        assert!(!new_incomplete.is_complete(2));
    }

    /// A priority incomplete team (e.g. left partner-less by
    /// `create_priority_team` stealing its other member) must keep its
    /// `priority` flag once completed here — the old design preserved it by
    /// mutating the row in place; this design always disbands and recreates,
    /// so priority has to be carried over explicitly or it's silently lost.
    #[test]
    fn test_release_players_preserves_priority_when_completing_an_incomplete_priority_team() {
        let session_id = Uuid::new_v4();
        let waiting_alone = player(Gender::Male);
        let freed_player = player(Gender::Male);
        let players = vec![waiting_alone.clone(), freed_player.clone()];

        let priority_incomplete_team =
            Team::new(session_id, vec![*waiting_alone.id()]).with_priority();
        let manager = TeamQueueManager::new(session_id, GameMode::Male, 2);

        let changed = manager.release_players(
            &[priority_incomplete_team],
            &[*freed_player.id()],
            &players,
            &PartnerHistory::empty(),
        );

        let completed = changed
            .iter()
            .find(|team| team.is_complete(2))
            .expect("the incomplete team should have been completed");
        assert!(completed.is_priority());
    }

    #[test]
    fn test_release_players_forms_a_new_complete_team_when_no_incomplete_team_is_waiting() {
        let session_id = Uuid::new_v4();
        let freed_a = player(Gender::Female);
        let freed_b = player(Gender::Female);
        let players = vec![freed_a.clone(), freed_b.clone()];

        let manager = TeamQueueManager::new(session_id, GameMode::Female, 2);

        let changed = manager.release_players(
            &[],
            &[*freed_a.id(), *freed_b.id()],
            &players,
            &PartnerHistory::empty(),
        );

        assert_eq!(changed.len(), 1);
        assert!(changed[0].is_complete(2));
        assert!(changed[0].player_ids().contains(freed_a.id()));
        assert!(changed[0].player_ids().contains(freed_b.id()));
    }

    #[test]
    fn test_release_players_in_mixed_mode_only_completes_a_team_needing_that_gender() {
        let session_id = Uuid::new_v4();
        let waiting_male = player(Gender::Male);
        let freed_male = player(Gender::Male);
        let freed_female = player(Gender::Female);
        let players = vec![
            waiting_male.clone(),
            freed_male.clone(),
            freed_female.clone(),
        ];

        let incomplete_team = Team::new(session_id, vec![*waiting_male.id()]);
        std::thread::sleep(std::time::Duration::from_millis(2));
        let manager = TeamQueueManager::new(session_id, GameMode::Mixed, 2);

        let changed = manager.release_players(
            &[incomplete_team.clone()],
            &[*freed_male.id(), *freed_female.id()],
            &players,
            &PartnerHistory::empty(),
        );

        let completed = changed
            .iter()
            .find(|team| team.is_complete(2))
            .expect("the waiting male should only be completed by the freed female");
        assert!(completed.player_ids().contains(waiting_male.id()));
        assert!(completed.player_ids().contains(freed_female.id()));
        assert!(!completed.player_ids().contains(freed_male.id()));

        let new_incomplete = changed
            .iter()
            .find(|team| team.player_ids() == &vec![*freed_male.id()])
            .expect("the freed male starts its own incomplete team");
        assert!(!new_incomplete.is_complete(2));
    }

    /// Rule 2 ("a player who just left the court can't return immediately"):
    /// with more longer-waiting players available than a single freed
    /// player, the longer-waiting ones fill the team(s) that can be formed
    /// right now and the freshly freed player is left waiting instead.
    #[test]
    fn test_release_players_prefers_longer_waiting_players_over_a_freshly_freed_one() {
        let session_id = Uuid::new_v4();
        let a = player(Gender::Male);
        let b = player(Gender::Male);
        let c = player(Gender::Male);
        let d = player(Gender::Male);
        let just_freed = player(Gender::Male);
        let players = vec![
            a.clone(),
            b.clone(),
            c.clone(),
            d.clone(),
            just_freed.clone(),
        ];

        let waiting_teams = vec![
            Team::new(session_id, vec![*a.id()]),
            Team::new(session_id, vec![*b.id()]),
            Team::new(session_id, vec![*c.id()]),
            Team::new(session_id, vec![*d.id()]),
        ];
        std::thread::sleep(std::time::Duration::from_millis(2));
        let manager = TeamQueueManager::new(session_id, GameMode::Male, 2);

        let changed = manager.release_players(
            &waiting_teams,
            &[*just_freed.id()],
            &players,
            &PartnerHistory::empty(),
        );

        let complete_teams: Vec<&Team> = changed.iter().filter(|team| team.is_complete(2)).collect();
        assert_eq!(complete_teams.len(), 2);
        assert!(complete_teams
            .iter()
            .all(|team| !team.player_ids().contains(just_freed.id())));

        let mut seated: Vec<Uuid> = complete_teams
            .iter()
            .flat_map(|team| team.player_ids().clone())
            .collect();
        seated.sort();
        let mut expected = vec![*a.id(), *b.id(), *c.id(), *d.id()];
        expected.sort();
        assert_eq!(seated, expected);

        let still_waiting = changed
            .iter()
            .find(|team| team.player_ids() == &vec![*just_freed.id()]);
        assert!(still_waiting.is_none() || !still_waiting.unwrap().is_complete(2));
    }

    /// With exactly two players in the pool and no one else to mix with,
    /// they have to form a team together even though they already played —
    /// best-effort mixing (rule 3) never blocks the court from opening.
    #[test]
    fn test_release_players_forms_a_team_from_exactly_two_players_even_if_they_already_played_together(
    ) {
        let session_id = Uuid::new_v4();
        let player_a = player(Gender::Male);
        let player_b = player(Gender::Male);
        let players = vec![player_a.clone(), player_b.clone()];

        let just_disbanded_team = Team::new(session_id, vec![*player_a.id(), *player_b.id()]);
        let played_match =
            Match::new(session_id, 1, *just_disbanded_team.id(), Uuid::new_v4()).unwrap();
        let history = PartnerHistory::from_matches(&[just_disbanded_team], &[played_match]);

        let manager = TeamQueueManager::new(session_id, GameMode::Male, 2);

        let changed =
            manager.release_players(&[], &[*player_a.id(), *player_b.id()], &players, &history);

        let reformed = changed
            .iter()
            .find(|team| team.is_complete(2))
            .expect("the pair must be reformed to avoid stalling the only court forever");
        assert!(reformed.player_ids().contains(player_a.id()));
        assert!(reformed.player_ids().contains(player_b.id()));
    }

    /// Same scenario in `GameMode::Open` — it no longer has a special "hold
    /// out forever" behavior, so it forms the team just like every other
    /// mode instead of leaving both players stuck as incomplete teams.
    #[test]
    fn test_release_players_in_open_mode_forms_a_team_even_with_no_fresh_alternative() {
        let session_id = Uuid::new_v4();
        let player_a = player(Gender::Male);
        let player_b = player(Gender::Male);
        let players = vec![player_a.clone(), player_b.clone()];

        let just_played_team = Team::new(session_id, vec![*player_a.id(), *player_b.id()]);
        let played_match =
            Match::new(session_id, 1, *just_played_team.id(), Uuid::new_v4()).unwrap();
        let history = PartnerHistory::from_matches(&[just_played_team], &[played_match]);

        let manager = TeamQueueManager::new(session_id, GameMode::Open, 2);

        let changed =
            manager.release_players(&[], &[*player_a.id(), *player_b.id()], &players, &history);

        let reformed = changed
            .iter()
            .find(|team| team.is_complete(2))
            .expect("Open must still form a team to avoid stalling the only court forever");
        assert!(reformed.player_ids().contains(player_a.id()));
        assert!(reformed.player_ids().contains(player_b.id()));
    }

    /// Regression test for the real production deadlock: a session that had
    /// played enough matches to exhaust every possible pairing ended up
    /// with most players stuck as solo incomplete teams and zero complete
    /// teams anywhere, so its only court could never open a match again.
    /// Here every pair among 4 players already shares history (simulated by
    /// a single 4-player team having "played" together) — even so, the
    /// queue must still group everyone into complete teams instead of
    /// leaving anyone stuck.
    #[test]
    fn test_release_players_never_leaves_players_stuck_even_when_every_pairing_is_already_exhausted(
    ) {
        let session_id = Uuid::new_v4();
        let a = player(Gender::Male);
        let b = player(Gender::Male);
        let c = player(Gender::Male);
        let d = player(Gender::Male);
        let players = vec![a.clone(), b.clone(), c.clone(), d.clone()];

        let saturated_team = Team::new(session_id, vec![*a.id(), *b.id(), *c.id(), *d.id()]);
        let played_match =
            Match::new(session_id, 1, *saturated_team.id(), Uuid::new_v4()).unwrap();
        let history = PartnerHistory::from_matches(&[saturated_team], &[played_match]);

        let manager = TeamQueueManager::new(session_id, GameMode::Open, 2);

        let changed = manager.release_players(
            &[],
            &[*a.id(), *b.id(), *c.id(), *d.id()],
            &players,
            &history,
        );

        let complete_teams: Vec<&Team> = changed.iter().filter(|team| team.is_complete(2)).collect();
        assert_eq!(complete_teams.len(), 2);
        assert!(changed.iter().all(|team| team.is_complete(2)));

        let mut seated: Vec<Uuid> = complete_teams
            .iter()
            .flat_map(|team| team.player_ids().clone())
            .collect();
        seated.sort();
        let mut expected = vec![*a.id(), *b.id(), *c.id(), *d.id()];
        expected.sort();
        assert_eq!(seated, expected);
    }

    #[test]
    fn test_next_complete_teams_skips_incomplete_and_orders_by_creation() {
        let session_id = Uuid::new_v4();
        let manager = TeamQueueManager::new(session_id, GameMode::Male, 2);

        let incomplete = Team::new(session_id, vec![Uuid::new_v4()]);
        let first_complete = Team::new(session_id, vec![Uuid::new_v4(), Uuid::new_v4()]);
        std::thread::sleep(std::time::Duration::from_millis(2));
        let second_complete = Team::new(session_id, vec![Uuid::new_v4(), Uuid::new_v4()]);

        let waiting_teams = vec![incomplete, second_complete.clone(), first_complete.clone()];

        let next = manager.next_complete_teams(&waiting_teams, 2);

        assert_eq!(next.len(), 2);
        assert_eq!(*next[0].id(), *first_complete.id());
        assert_eq!(*next[1].id(), *second_complete.id());
    }

    #[test]
    fn test_next_complete_teams_ignores_non_waiting_teams() {
        let session_id = Uuid::new_v4();
        let manager = TeamQueueManager::new(session_id, GameMode::Male, 2);

        let mut holding = Team::new(session_id, vec![Uuid::new_v4(), Uuid::new_v4()]);
        holding.register_win();

        let waiting_teams = [holding];
        let next = manager.next_complete_teams(&waiting_teams, 1);

        assert!(next.is_empty());
    }

    /// A priority team jumps every non-priority team, even ones that have
    /// been waiting longer.
    #[test]
    fn test_next_complete_teams_puts_priority_teams_first() {
        let session_id = Uuid::new_v4();
        let manager = TeamQueueManager::new(session_id, GameMode::Male, 2);

        let long_waiting = Team::new(session_id, vec![Uuid::new_v4(), Uuid::new_v4()]);
        std::thread::sleep(std::time::Duration::from_millis(2));
        let priority = Team::new(session_id, vec![Uuid::new_v4(), Uuid::new_v4()]).with_priority();

        let waiting_teams = vec![long_waiting.clone(), priority.clone()];

        let next = manager.next_complete_teams(&waiting_teams, 2);

        assert_eq!(*next[0].id(), *priority.id());
        assert_eq!(*next[1].id(), *long_waiting.id());
    }
}
