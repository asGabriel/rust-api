use rand::{seq::SliceRandom, thread_rng};
use uuid::Uuid;

use crate::modules::matchmaking::domain::{
    player::{Gender, Player},
    session::GameMode,
    team::Team,
    team_drawer::PartnerHistory,
};

/// Keeps the session's waiting queue moving: when players are freed (a team
/// lost, or a team hit the consecutive-win cap), slots them into the queue —
/// completing an incomplete team waiting for a partner if one is compatible,
/// or starting a new incomplete team otherwise — and answers "who's next" so
/// a freed-up court can be refilled automatically. Bound to a `session_id`,
/// `game_mode` and `players_per_team`, the same values a `TeamDrawer` for
/// this session would use, since the queue has to honor the same shape of
/// team the initial draw formed.
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

    /// Slots freed players into the waiting queue, in random order (no rule
    /// about which of several freed players completes an incomplete team
    /// versus starts a new one). Returns every team that was created or
    /// modified, for the caller to persist — teams left untouched are not
    /// included.
    ///
    /// A freed player prefers completing an incomplete team with a pairing
    /// that's brand new, opening a new single-player team to wait for one
    /// otherwise (phase 1 below). For `GameMode::Open` (`ShuffleType::
    /// RoundRobin`), that patience is unconditional and permanent — its
    /// documented trade-off: the queue may accumulate several incomplete
    /// teams rather than ever repeat a pairing. Every other mode instead
    /// only gets one cycle's worth of patience (phase 2 below): once a team
    /// has already sat incomplete since before this call and still finds no
    /// fresh partner, or the rest of the queue has no complete team left to
    /// keep a court moving, it gets merged with the best (fewest shared
    /// players, then longest-waiting) other leftover team, repeat or not.
    /// Without that second phase, a session with just enough players to
    /// fill its teams would idle its only court forever the moment two
    /// partners lose together, since no future match — and so no future
    /// `release_players` call — would ever come along to give them a second
    /// chance. Across enough matches this plays out as an escalating cycle:
    /// each player gets a genuine shot at a fresh partner every time they're
    /// freed, and only falls back to a repeat once that specific shot comes
    /// up empty — not once, permanently, but every single time it happens.
    pub fn release_players(
        &self,
        waiting_teams: &[Team],
        freed_player_ids: &[Uuid],
        players: &[Player],
        history: &PartnerHistory,
    ) -> Vec<Team> {
        let mut order = freed_player_ids.to_vec();
        order.shuffle(&mut thread_rng());

        let mut pending: Vec<Team> = waiting_teams
            .iter()
            .filter(|team| team.is_waiting() && !team.is_complete(self.players_per_team))
            .cloned()
            .collect();
        let already_waiting_ids: std::collections::HashSet<Uuid> =
            pending.iter().map(|team| *team.id()).collect();
        let queue_has_slack = waiting_teams
            .iter()
            .any(|team| team.is_waiting() && team.is_complete(self.players_per_team));
        let mut touched_ids = std::collections::HashSet::new();

        // Phase 1: give every freed player a shot at a fresh (never
        // partnered) pairing, one at a time. A team opened here stays
        // visible to players processed later in this same pass, so two
        // freed players who haven't partnered before still find each other
        // within this phase, regardless of processing order.
        for &player_id in &order {
            let gender = Self::gender_of(players, player_id);

            let fresh_candidate = (0..pending.len())
                .filter(|&index| {
                    !pending[index].is_complete(self.players_per_team)
                        && self.needs_gender(&pending[index], gender, players)
                })
                .filter(|&index| {
                    !pending[index]
                        .player_ids()
                        .iter()
                        .any(|member| history.have_played_together(*member, player_id))
                })
                .min_by_key(|&index| *pending[index].created_at());

            match fresh_candidate {
                Some(index) => {
                    pending[index].add_player(player_id);
                    touched_ids.insert(*pending[index].id());
                }
                None => {
                    let new_team = Team::new(self.session_id, vec![player_id]);
                    touched_ids.insert(*new_team.id());
                    pending.push(new_team);
                }
            }
        }

        let mut disbanded = Vec::new();

        // Phase 2: force-complete whoever's still incomplete after phase 1,
        // for every mode except Open/RoundRobin (see doc comment above).
        if !self.game_mode.is_open() {
            loop {
                let acceptor_index = (0..pending.len()).find(|&index| {
                    !pending[index].is_complete(self.players_per_team)
                        && (already_waiting_ids.contains(pending[index].id()) || !queue_has_slack)
                });
                let Some(acceptor_index) = acceptor_index else {
                    break;
                };

                let partner_index = (0..pending.len())
                    .filter(|&index| {
                        index != acceptor_index && !pending[index].is_complete(self.players_per_team)
                    })
                    .filter(|&index| {
                        pending[index].player_ids().iter().all(|donor_player_id| {
                            self.needs_gender(
                                &pending[acceptor_index],
                                Self::gender_of(players, *donor_player_id),
                                players,
                            )
                        })
                    })
                    .filter(|&index| {
                        pending[acceptor_index].player_ids().len() + pending[index].player_ids().len()
                            <= self.players_per_team.into()
                    })
                    .min_by_key(|&index| {
                        let conflicts = pending[index]
                            .player_ids()
                            .iter()
                            .filter(|donor_member| {
                                pending[acceptor_index].player_ids().iter().any(|member| {
                                    history.have_played_together(*member, **donor_member)
                                })
                            })
                            .count();
                        (conflicts, *pending[index].created_at())
                    });

                let Some(partner_index) = partner_index else {
                    break;
                };

                let partner_players = pending[partner_index].player_ids().clone();
                for donor_player_id in partner_players {
                    pending[acceptor_index].add_player(donor_player_id);
                }
                touched_ids.insert(*pending[acceptor_index].id());

                let mut partner = pending.remove(partner_index);
                partner.disband();
                disbanded.push(partner);
            }
        }

        pending
            .into_iter()
            .filter(|team| touched_ids.contains(team.id()))
            .chain(disbanded)
            .collect()
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

    /// Whether `team` still needs another player of `gender` to be complete.
    /// Any incomplete team is compatible in a single-gender mode; in `Mixed`
    /// mode, a team only accepts a gender it hasn't already filled its half
    /// of `players_per_team` with.
    fn needs_gender(&self, team: &Team, gender: Gender, players: &[Player]) -> bool {
        if !self.game_mode.is_mixed() {
            return true;
        }

        let target_per_gender: usize = (self.players_per_team / 2).into();
        let current = team
            .player_ids()
            .iter()
            .filter(|player_id| Self::gender_of(players, **player_id) == gender)
            .count();

        current < target_per_gender
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
    fn test_release_players_completes_the_waiting_incomplete_team_and_starts_a_new_one() {
        let session_id = Uuid::new_v4();
        let waiting_alone = Player::new("15".to_string(), Gender::Male);
        let freed_a = Player::new("1".to_string(), Gender::Male);
        let freed_b = Player::new("2".to_string(), Gender::Male);
        let players = vec![waiting_alone.clone(), freed_a.clone(), freed_b.clone()];

        let incomplete_team = Team::new(session_id, vec![*waiting_alone.id()]);
        let manager = TeamQueueManager::new(session_id, GameMode::Male, 2);

        let changed = manager.release_players(
            &[incomplete_team.clone()],
            &[*freed_a.id(), *freed_b.id()],
            &players,
            &PartnerHistory::empty(),
        );

        assert_eq!(changed.len(), 2);
        let completed = changed
            .iter()
            .find(|team| *team.id() == *incomplete_team.id())
            .expect("the original incomplete team should have been completed");
        assert!(completed.is_complete(2));
        assert!(completed.player_ids().contains(waiting_alone.id()));

        let new_incomplete = changed
            .iter()
            .find(|team| *team.id() != *incomplete_team.id())
            .expect("the other freed player should start a new incomplete team");
        assert_eq!(new_incomplete.player_ids().len(), 1);
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
        let manager = TeamQueueManager::new(session_id, GameMode::Mixed, 2);

        let changed = manager.release_players(
            &[incomplete_team.clone()],
            &[*freed_male.id(), *freed_female.id()],
            &players,
            &PartnerHistory::empty(),
        );

        let completed = changed
            .iter()
            .find(|team| *team.id() == *incomplete_team.id())
            .expect("the waiting male should only be completed by the freed female");
        assert!(completed.player_ids().contains(freed_female.id()));
        assert!(!completed.player_ids().contains(freed_male.id()));

        let new_incomplete = changed
            .iter()
            .find(|team| *team.id() != *incomplete_team.id())
            .expect("the freed male starts its own incomplete team");
        assert_eq!(new_incomplete.player_ids(), &vec![*freed_male.id()]);
    }

    /// Regression test: with `players_per_team == 2`, a losing team's two
    /// players are freed together with no other candidate between them —
    /// this used to force them straight back together as the same pairing,
    /// in every `game_mode` except `Open`, because the fresh-partner check
    /// only applied there. It must now hold for every mode, as long as the
    /// rest of the queue has slack (some other complete team) to keep a
    /// court moving while these two wait for a real alternative.
    #[test]
    fn test_release_players_avoids_repeating_a_pairing_when_the_queue_has_slack_elsewhere() {
        let session_id = Uuid::new_v4();
        let player_a = player(Gender::Male);
        let player_b = player(Gender::Male);
        let players = vec![player_a.clone(), player_b.clone()];

        let just_disbanded_team = Team::new(session_id, vec![*player_a.id(), *player_b.id()]);
        let played_match =
            Match::new(session_id, 1, *just_disbanded_team.id(), Uuid::new_v4()).unwrap();
        let history = PartnerHistory::from_matches(&[just_disbanded_team], &[played_match]);

        let other_complete_team =
            Team::new(session_id, vec![Uuid::new_v4(), Uuid::new_v4()]);

        let manager = TeamQueueManager::new(session_id, GameMode::Male, 2);

        let changed = manager.release_players(
            &[other_complete_team],
            &[*player_a.id(), *player_b.id()],
            &players,
            &history,
        );

        assert_eq!(changed.len(), 2);
        for team in &changed {
            assert_eq!(team.player_ids().len(), 1);
        }
    }

    /// Without slack (no other complete team left in the queue), forcing a
    /// repeat is the only way to avoid stalling the session forever: no
    /// future match on the only court means no future `release_players`
    /// call would ever come along to give this pair a second, fresher
    /// chance. This is the minimal-session deadlock the "wait a cycle"
    /// design would otherwise fall into.
    #[test]
    fn test_release_players_forces_a_repeat_when_the_queue_has_no_complete_team_left() {
        let session_id = Uuid::new_v4();
        let player_a = player(Gender::Male);
        let player_b = player(Gender::Male);
        let players = vec![player_a.clone(), player_b.clone()];

        let just_disbanded_team = Team::new(session_id, vec![*player_a.id(), *player_b.id()]);
        let played_match =
            Match::new(session_id, 1, *just_disbanded_team.id(), Uuid::new_v4()).unwrap();
        let history = PartnerHistory::from_matches(&[just_disbanded_team], &[played_match]);

        let manager = TeamQueueManager::new(session_id, GameMode::Male, 2);

        let changed = manager.release_players(
            &[],
            &[*player_a.id(), *player_b.id()],
            &players,
            &history,
        );

        let reformed = changed
            .iter()
            .find(|team| team.is_complete(2))
            .expect("the pair must be reformed to avoid stalling the only court forever");
        assert!(reformed.player_ids().contains(player_a.id()));
        assert!(reformed.player_ids().contains(player_b.id()));
    }

    /// An incomplete team already sitting in the queue before this call
    /// already had its chance at a fresh partner — the next freed player
    /// should complete it (even with a repeat) rather than leaving it
    /// waiting indefinitely while opening yet another incomplete team.
    #[test]
    fn test_release_players_completes_an_already_waiting_team_with_a_repeat_before_opening_a_new_one(
    ) {
        let session_id = Uuid::new_v4();
        let waiting_player = player(Gender::Male);
        let freed_player = player(Gender::Male);
        let players = vec![waiting_player.clone(), freed_player.clone()];

        let already_waiting_incomplete = Team::new(session_id, vec![*waiting_player.id()]);
        let other_complete_team = Team::new(session_id, vec![Uuid::new_v4(), Uuid::new_v4()]);

        let played_team = Team::new(
            session_id,
            vec![*waiting_player.id(), *freed_player.id()],
        );
        let played_match = Match::new(session_id, 1, *played_team.id(), Uuid::new_v4()).unwrap();
        let history = PartnerHistory::from_matches(&[played_team], &[played_match]);

        let manager = TeamQueueManager::new(session_id, GameMode::Male, 2);

        let changed = manager.release_players(
            &[already_waiting_incomplete.clone(), other_complete_team],
            &[*freed_player.id()],
            &players,
            &history,
        );

        let completed = changed
            .iter()
            .find(|team| *team.id() == *already_waiting_incomplete.id())
            .expect("the already-waiting team should be completed, repeat or not");
        assert!(completed.is_complete(2));
        assert!(completed.player_ids().contains(freed_player.id()));
    }

    #[test]
    fn test_release_players_in_open_mode_opens_a_new_team_instead_of_repeating_a_pairing() {
        let session_id = Uuid::new_v4();
        let waiting_player = player(Gender::Male);
        let already_played_with_waiting = player(Gender::Male);
        let fresh_player = player(Gender::Female);
        let players = vec![
            waiting_player.clone(),
            already_played_with_waiting.clone(),
            fresh_player.clone(),
        ];

        let incomplete_team = Team::new(session_id, vec![*waiting_player.id()]);

        let played_team = Team::new(
            session_id,
            vec![*waiting_player.id(), *already_played_with_waiting.id()],
        );
        let played_match = Match::new(session_id, 1, *played_team.id(), Uuid::new_v4()).unwrap();
        let history = PartnerHistory::from_matches(&[played_team], &[played_match]);

        let manager = TeamQueueManager::new(session_id, GameMode::Open, 2);

        let changed = manager.release_players(
            &[incomplete_team.clone()],
            &[*already_played_with_waiting.id(), *fresh_player.id()],
            &players,
            &history,
        );

        let completed = changed
            .iter()
            .find(|team| *team.id() == *incomplete_team.id())
            .expect("the waiting player should only be completed by a fresh partner");
        assert!(completed.player_ids().contains(fresh_player.id()));
        assert!(!completed
            .player_ids()
            .contains(already_played_with_waiting.id()));

        let new_incomplete = changed
            .iter()
            .find(|team| *team.id() != *incomplete_team.id())
            .expect(
                "the player who already partnered with the waiting player should start a new team",
            );
        assert_eq!(
            new_incomplete.player_ids(),
            &vec![*already_played_with_waiting.id()]
        );
    }

    #[test]
    fn test_release_players_in_open_mode_starts_a_new_team_when_the_only_incomplete_team_would_repeat(
    ) {
        let session_id = Uuid::new_v4();
        let waiting_player = player(Gender::Male);
        let already_played_with_waiting = player(Gender::Male);
        let players = vec![waiting_player.clone(), already_played_with_waiting.clone()];

        let incomplete_team = Team::new(session_id, vec![*waiting_player.id()]);

        let played_team = Team::new(
            session_id,
            vec![*waiting_player.id(), *already_played_with_waiting.id()],
        );
        let played_match = Match::new(session_id, 1, *played_team.id(), Uuid::new_v4()).unwrap();
        let history = PartnerHistory::from_matches(&[played_team], &[played_match]);

        let manager = TeamQueueManager::new(session_id, GameMode::Open, 2);

        let changed = manager.release_players(
            &[incomplete_team.clone()],
            &[*already_played_with_waiting.id()],
            &players,
            &history,
        );

        assert_eq!(changed.len(), 1);
        assert_ne!(*changed[0].id(), *incomplete_team.id());
        assert_eq!(
            changed[0].player_ids(),
            &vec![*already_played_with_waiting.id()]
        );
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
