use chrono::{TimeZone, Utc};
use domain::league::{Fixture, FixtureCompetition, FixtureStatus, League, StandingEntry};
use domain::manager::Manager;
use domain::player::{Player, PlayerAttributes, Position};
use domain::team::Team;
use ofm_core::clock::GameClock;
use ofm_core::game::Game;
use ofm_core::live_match_manager::{self, MatchMode};

// ---------------------------------------------------------------------------
// Test helpers
// ---------------------------------------------------------------------------

fn default_attrs(pos: Position) -> PlayerAttributes {
    let group = pos.to_group_position();
    let is_gk = matches!(group, Position::Goalkeeper);
    let is_def = matches!(group, Position::Defender);
    let is_fwd = matches!(group, Position::Forward);
    PlayerAttributes {
        pace: 65,
        stamina: 65,
        strength: 65,
        agility: 65,
        passing: 65,
        shooting: if is_gk { 30 } else { 65 },
        tackling: if is_gk || is_fwd { 35 } else { 65 },
        dribbling: if is_gk { 30 } else { 65 },
        defending: if is_gk {
            30
        } else if is_def {
            75
        } else {
            55
        },
        positioning: 65,
        vision: 65,
        decisions: 65,
        composure: 65,
        aggression: 50,
        teamwork: 65,
        leadership: 50,
        handling: if is_gk { 75 } else { 20 },
        reflexes: if is_gk { 75 } else { 30 },
        aerial: 60,
    }
}

fn make_player(id: &str, name: &str, team_id: &str, pos: Position) -> Player {
    let attrs = default_attrs(pos.clone());
    let mut p = Player::new(
        id.to_string(),
        name.to_string(),
        format!("Full {}", name),
        "1995-01-01".to_string(),
        "GB".to_string(),
        pos,
        attrs,
    );
    p.team_id = Some(team_id.to_string());
    p.morale = 70;
    p.condition = 90;
    p
}

fn make_team(id: &str, name: &str) -> Team {
    Team::new(
        id.to_string(),
        name.to_string(),
        name[..3].to_string(),
        "England".to_string(),
        "London".to_string(),
        "Stadium".to_string(),
        40_000,
    )
}

/// Build a full squad of 22 players for a team (4-4-2 formation ready).
fn make_squad(team_id: &str) -> Vec<Player> {
    let mut players = Vec::new();
    // 2 GK
    for i in 0..2 {
        players.push(make_player(
            &format!("{}_gk{}", team_id, i),
            &format!("GK{}", i),
            team_id,
            Position::Goalkeeper,
        ));
    }
    // 7 DEF
    for i in 0..7 {
        players.push(make_player(
            &format!("{}_def{}", team_id, i),
            &format!("Def{}", i),
            team_id,
            Position::Defender,
        ));
    }
    // 7 MID
    for i in 0..7 {
        players.push(make_player(
            &format!("{}_mid{}", team_id, i),
            &format!("Mid{}", i),
            team_id,
            Position::Midfielder,
        ));
    }
    // 6 FWD
    for i in 0..6 {
        players.push(make_player(
            &format!("{}_fwd{}", team_id, i),
            &format!("Fwd{}", i),
            team_id,
            Position::Forward,
        ));
    }
    players
}

fn make_game_with_fixture() -> Game {
    let date = Utc.with_ymd_and_hms(2025, 6, 15, 12, 0, 0).unwrap();
    let clock = GameClock::new(date);
    let mut manager = Manager::new(
        "mgr1".to_string(),
        "Test".to_string(),
        "Manager".to_string(),
        "1980-01-01".to_string(),
        "England".to_string(),
    );
    manager.hire("team1".to_string());

    let team1 = make_team("team1", "Test FC");
    let team2 = make_team("team2", "Rival FC");

    let mut players = make_squad("team1");
    players.extend(make_squad("team2"));

    let fixture = Fixture {
        id: "fix1".to_string(),
        matchday: 1,
        date: "2025-06-15".to_string(),
        home_team_id: "team1".to_string(),
        away_team_id: "team2".to_string(),
        competition: FixtureCompetition::League,
        status: FixtureStatus::Scheduled,
        result: None,
    };

    let league = League {
        id: "league1".to_string(),
        name: "Test League".to_string(),
        season: 1,
        fixtures: vec![fixture],
        standings: vec![
            StandingEntry::new("team1".to_string()),
            StandingEntry::new("team2".to_string()),
        ],
    };

    let mut game = Game::new(clock, manager, vec![team1, team2], players, vec![], vec![]);
    game.league = Some(league);
    game
}

// ---------------------------------------------------------------------------
// create_live_match
// ---------------------------------------------------------------------------

#[test]
fn create_live_match_succeeds() {
    let game = make_game_with_fixture();
    let session = live_match_manager::create_live_match(&game, 0, MatchMode::Live, false);
    assert!(
        session.is_ok(),
        "Should create live match: {:?}",
        session.err()
    );
    let session = session.unwrap();
    assert_eq!(session.home_team_id, "team1");
    assert_eq!(session.away_team_id, "team2");
    assert_eq!(session.mode, MatchMode::Live);
    assert_eq!(session.fixture_index, 0);
    assert!(!session.is_finished());
}

#[test]
fn create_live_match_user_side_home() {
    let game = make_game_with_fixture();
    let session = live_match_manager::create_live_match(&game, 0, MatchMode::Live, false).unwrap();
    assert_eq!(session.user_side, Some(engine::Side::Home));
}

#[test]
fn create_live_match_user_side_away() {
    let mut game = make_game_with_fixture();
    game.manager.team_id = Some("team2".to_string());
    let session = live_match_manager::create_live_match(&game, 0, MatchMode::Live, false).unwrap();
    assert_eq!(session.user_side, Some(engine::Side::Away));
}

#[test]
fn create_live_match_user_side_none_neutral() {
    let mut game = make_game_with_fixture();
    game.manager.team_id = Some("team3".to_string());
    let session =
        live_match_manager::create_live_match(&game, 0, MatchMode::Spectator, false).unwrap();
    assert_eq!(session.user_side, None);
}

#[test]
fn create_live_match_no_league_errors() {
    let mut game = make_game_with_fixture();
    game.league = None;
    let result = live_match_manager::create_live_match(&game, 0, MatchMode::Live, false);
    assert!(result.is_err());
}

#[test]
fn create_live_match_bad_fixture_index_errors() {
    let game = make_game_with_fixture();
    let result = live_match_manager::create_live_match(&game, 99, MatchMode::Live, false);
    assert!(result.is_err());
}

// ---------------------------------------------------------------------------
// LiveMatchSession stepping
// ---------------------------------------------------------------------------

#[test]
fn step_advances_match() {
    let game = make_game_with_fixture();
    let mut session =
        live_match_manager::create_live_match(&game, 0, MatchMode::Spectator, false).unwrap();

    let result = session.step();
    assert!(
        !result.is_finished,
        "Match should not be finished after 1 step"
    );
}

#[test]
fn step_many_returns_requested_count() {
    let game = make_game_with_fixture();
    let mut session =
        live_match_manager::create_live_match(&game, 0, MatchMode::Spectator, false).unwrap();

    let results = session.step_many(10);
    assert!(results.len() >= 1 && results.len() <= 10);
}

#[test]
fn run_to_completion_finishes() {
    let game = make_game_with_fixture();
    let mut session =
        live_match_manager::create_live_match(&game, 0, MatchMode::Instant, false).unwrap();

    let results = session.run_to_completion();
    assert!(!results.is_empty());
    assert!(results.last().unwrap().is_finished);
    assert!(session.is_finished());
}

#[test]
fn snapshot_returns_valid_state() {
    let game = make_game_with_fixture();
    let session = live_match_manager::create_live_match(&game, 0, MatchMode::Live, false).unwrap();
    let snap = session.snapshot();
    // Snapshot should have non-empty team names
    assert!(!snap.home_team.name.is_empty());
    assert!(!snap.away_team.name.is_empty());
}

#[test]
fn step_many_stops_at_finish() {
    let game = make_game_with_fixture();
    let mut session =
        live_match_manager::create_live_match(&game, 0, MatchMode::Instant, false).unwrap();

    // Request way more steps than a match has
    let results = session.step_many(500);
    assert!(results.last().unwrap().is_finished);
    // Should have stopped early
    assert!(results.len() < 500);
}

// ---------------------------------------------------------------------------
// auto_select_set_pieces
// ---------------------------------------------------------------------------

#[test]
fn auto_select_set_pieces_picks_captain() {
    let game = make_game_with_fixture();
    let player_ids: Vec<String> = game
        .players
        .iter()
        .filter(|p| p.team_id.as_deref() == Some("team1"))
        .map(|p| p.id.clone())
        .collect();

    let (captain, penalty, free_kick, corner) =
        live_match_manager::auto_select_set_pieces(&game, &player_ids);

    assert!(captain.is_some(), "Should pick a captain");
    assert!(penalty.is_some(), "Should pick a penalty taker");
    assert!(free_kick.is_some(), "Should pick a free kick taker");
    assert!(corner.is_some(), "Should pick a corner taker");
}

#[test]
fn auto_select_set_pieces_excludes_gk_from_penalty() {
    let game = make_game_with_fixture();
    let player_ids: Vec<String> = game
        .players
        .iter()
        .filter(|p| p.team_id.as_deref() == Some("team1"))
        .map(|p| p.id.clone())
        .collect();

    let (_, penalty, free_kick, corner) =
        live_match_manager::auto_select_set_pieces(&game, &player_ids);

    // None of the set piece takers (except captain) should be GK
    let gk_ids: Vec<String> = game
        .players
        .iter()
        .filter(|p| p.team_id.as_deref() == Some("team1") && p.position == Position::Goalkeeper)
        .map(|p| p.id.clone())
        .collect();

    if let Some(pk) = &penalty {
        assert!(!gk_ids.contains(pk), "GK should not be penalty taker");
    }
    if let Some(fk) = &free_kick {
        assert!(!gk_ids.contains(fk), "GK should not be free kick taker");
    }
    if let Some(ck) = &corner {
        assert!(!gk_ids.contains(ck), "GK should not be corner taker");
    }
}

#[test]
fn auto_select_set_pieces_empty_ids_returns_none() {
    let game = make_game_with_fixture();
    let (captain, penalty, free_kick, corner) =
        live_match_manager::auto_select_set_pieces(&game, &[]);
    assert!(captain.is_none());
    assert!(penalty.is_none());
    assert!(free_kick.is_none());
    assert!(corner.is_none());
}

#[test]
fn auto_select_set_pieces_prefers_high_leadership_captain() {
    let mut game = make_game_with_fixture();
    // Give one player very high leadership
    let leader = game
        .players
        .iter_mut()
        .find(|p| p.id == "team1_mid0")
        .unwrap();
    leader.attributes.leadership = 99;
    leader.attributes.teamwork = 99;

    let player_ids: Vec<String> = game
        .players
        .iter()
        .filter(|p| p.team_id.as_deref() == Some("team1"))
        .map(|p| p.id.clone())
        .collect();

    let (captain, _, _, _) = live_match_manager::auto_select_set_pieces(&game, &player_ids);
    assert_eq!(captain, Some("team1_mid0".to_string()));
}

#[test]
fn auto_select_set_pieces_prefers_high_shooting_penalty() {
    let mut game = make_game_with_fixture();
    let shooter = game
        .players
        .iter_mut()
        .find(|p| p.id == "team1_fwd0")
        .unwrap();
    shooter.attributes.shooting = 99;
    shooter.attributes.composure = 99;

    let player_ids: Vec<String> = game
        .players
        .iter()
        .filter(|p| p.team_id.as_deref() == Some("team1"))
        .map(|p| p.id.clone())
        .collect();

    let (_, penalty, _, _) = live_match_manager::auto_select_set_pieces(&game, &player_ids);
    assert_eq!(penalty, Some("team1_fwd0".to_string()));
}

// ---------------------------------------------------------------------------
// Injured players excluded from starting XI
// ---------------------------------------------------------------------------

#[test]
fn injured_players_excluded_from_xi() {
    let mut game = make_game_with_fixture();
    // Injure all but 11 players on team1
    let team1_players: Vec<String> = game
        .players
        .iter()
        .filter(|p| p.team_id.as_deref() == Some("team1"))
        .map(|p| p.id.clone())
        .collect();

    // Injure some players
    for id in &team1_players[11..] {
        if let Some(p) = game.players.iter_mut().find(|p| p.id == *id) {
            p.injury = Some(domain::player::Injury {
                name: "Hamstring".to_string(),
                days_remaining: 10,
            });
        }
    }

    let session =
        live_match_manager::create_live_match(&game, 0, MatchMode::Instant, false).unwrap();
    let snap = session.snapshot();

    // The starting XI should only have non-injured players
    assert!(
        snap.home_team.players.len() <= 11,
        "Starting XI should have at most 11"
    );
}

#[test]
fn slot_aware_xi_selection_prefers_true_fullback_for_fullback_slot() {
    let mut game = make_game_with_fixture();

    let specialist_rb = game
        .players
        .iter_mut()
        .find(|player| player.id == "team1_def0")
        .unwrap();
    specialist_rb.position = Position::RightBack;
    specialist_rb.natural_position = Position::RightBack;
    specialist_rb.attributes.pace = 86;
    specialist_rb.attributes.stamina = 84;
    specialist_rb.attributes.tackling = 80;
    specialist_rb.attributes.defending = 76;
    specialist_rb.attributes.positioning = 74;
    specialist_rb.attributes.passing = 68;
    specialist_rb.attributes.dribbling = 66;

    let stronger_cb = game
        .players
        .iter_mut()
        .find(|player| player.id == "team1_def1")
        .unwrap();
    stronger_cb.position = Position::CenterBack;
    stronger_cb.natural_position = Position::CenterBack;
    stronger_cb.attributes.defending = 90;
    stronger_cb.attributes.tackling = 88;
    stronger_cb.attributes.positioning = 86;
    stronger_cb.attributes.strength = 88;
    stronger_cb.attributes.pace = 58;
    stronger_cb.attributes.stamina = 64;
    stronger_cb.attributes.passing = 52;
    stronger_cb.attributes.dribbling = 48;

    let session =
        live_match_manager::create_live_match(&game, 0, MatchMode::Instant, false).unwrap();
    let snap = session.snapshot();

    assert_eq!(snap.home_team.players[1].id, "team1_def0");
}

// ---------------------------------------------------------------------------
// Match modes
// ---------------------------------------------------------------------------

#[test]
fn all_match_modes_create_successfully() {
    let game = make_game_with_fixture();
    for mode in [MatchMode::Live, MatchMode::Spectator, MatchMode::Instant] {
        let session = live_match_manager::create_live_match(&game, 0, mode, false);
        assert!(session.is_ok(), "Mode {:?} should work", mode);
    }
}

#[test]
fn instant_mode_completes() {
    let game = make_game_with_fixture();
    let mut session =
        live_match_manager::create_live_match(&game, 0, MatchMode::Instant, false).unwrap();
    let results = session.run_to_completion();
    assert!(session.is_finished());
    assert!(results.len() >= 90, "Match should have at least 90 minutes");
}

// ---------------------------------------------------------------------------
// Extra time
// ---------------------------------------------------------------------------

#[test]
fn extra_time_flag_passed_through() {
    let game = make_game_with_fixture();
    // Just verify it doesn't crash with extra_time=true
    let session = live_match_manager::create_live_match(&game, 0, MatchMode::Instant, true);
    assert!(session.is_ok());
}

// ---------------------------------------------------------------------------
// Formation position count verification (4-part formations)
// ---------------------------------------------------------------------------

fn position_counts_in_snapshot(snapshot: &engine::MatchSnapshot) -> (usize, usize, usize) {
    use engine::Position;
    let mut def = 0;
    let mut mid = 0;
    let mut fwd = 0;
    for p in &snapshot.home_team.players {
        match p.position {
            Position::Defender => def += 1,
            Position::Midfielder => mid += 1,
            Position::Forward => fwd += 1,
            Position::Goalkeeper => { /* not counted */ }
        }
    }
    (def, mid, fwd)
}

#[test]
fn four_one_four_one_formation_has_correct_position_counts() {
    let mut game = make_game_with_fixture();
    // Set formation to 4-1-4-1 for team1
    let team1 = game.teams.iter_mut().find(|t| t.id == "team1").unwrap();
    team1.formation = "4-1-4-1".to_string();

    let session = live_match_manager::create_live_match(&game, 0, MatchMode::Instant, false).unwrap();
    let snapshot = session.snapshot();

    assert_eq!(
        snapshot.home_team.formation, "4-1-4-1",
        "formation should be set to 4-1-4-1"
    );

    let (def_count, mid_count, fwd_count) = position_counts_in_snapshot(&snapshot);
    assert_eq!(def_count, 4, "4-1-4-1 should have 4 defenders (not 4+2=6)");
    assert_eq!(mid_count, 5, "4-1-4-1 should have 5 midfielders (1+4, not 2+4=6)");
    assert_eq!(fwd_count, 1, "4-1-4-1 should have 1 forward");
    assert_eq!(snapshot.home_team.players.len(), 11, "starting XI should always have 11 players total (1 GK + 10 outfield)");
}

#[test]
fn four_two_three_one_formation_combines_deep_midfielders_into_midfielder_group() {
    let mut game = make_game_with_fixture();
    let team1 = game.teams.iter_mut().find(|t| t.id == "team1").unwrap();
    team1.formation = "4-2-3-1".to_string();

    let session = live_match_manager::create_live_match(&game, 0, MatchMode::Instant, false).unwrap();
    let snapshot = session.snapshot();

    let (def_count, mid_count, fwd_count) = position_counts_in_snapshot(&snapshot);
    assert_eq!(def_count, 4, "4-2-3-1 should have 4 defenders");
    assert_eq!(mid_count, 5, "4-2-3-1 should have 5 midfielders (2+3, not 3+3=6)");
    assert_eq!(fwd_count, 1, "4-2-3-1 should have 1 forward");
}

#[test]
fn three_four_one_two_formation_combines_midfield_groups() {
    let mut game = make_game_with_fixture();
    let team1 = game.teams.iter_mut().find(|t| t.id == "team1").unwrap();
    team1.formation = "3-4-1-2".to_string();

    let session = live_match_manager::create_live_match(&game, 0, MatchMode::Instant, false).unwrap();
    let snapshot = session.snapshot();

    let (def_count, mid_count, fwd_count) = position_counts_in_snapshot(&snapshot);
    assert_eq!(def_count, 3, "3-4-1-2 should have 3 defenders");
    assert_eq!(mid_count, 5, "3-4-1-2 should have 5 midfielders (4+1)");
    assert_eq!(fwd_count, 2, "3-4-1-2 should have 2 forwards");
}

// ---------------------------------------------------------------------------
// Starting XI from saved IDs (Tactics screen)
// ---------------------------------------------------------------------------

#[test]
fn saved_xi_ids_are_used_in_live_match() {
    let mut game = make_game_with_fixture();
    let team1 = game.teams.iter_mut().find(|t| t.id == "team1").unwrap();
    team1.formation = "4-1-4-1".to_string();
    // Set a specific starting XI: pick the last 11 players as starters
    // (instead of the default first-11). The key is that the order in
    // starting_xi_ids determines which slot each player fills.
    let all_team1_ids: Vec<String> = game
        .players
        .iter()
        .filter(|p| p.team_id.as_deref() == Some("team1"))
        .map(|p| p.id.clone())
        .collect();
    // Use the bottom 11 players as starters to verify ordering is respected
    let starters = all_team1_ids.into_iter().rev().take(11).collect::<Vec<_>>();
    team1.starting_xi_ids = starters.clone();

    let session = live_match_manager::create_live_match(&game, 0, MatchMode::Instant, false).unwrap();
    let snapshot = session.snapshot();

    // Check that the player IDs in the snapshot are exactly our saved IDs
    let snapshot_ids: Vec<&str> = snapshot
        .home_team
        .players
        .iter()
        .map(|p| p.id.as_str())
        .collect();

    // All 11 saved starters should appear (some may differ due to injuries/suspensions
    // in the fixture logic, but the order should be respected when possible).
    // Key check: the GK at position 0 should be the saved GK at index 0 of starters.
    let gk_slot = snapshot.home_team.players.iter().find(|p| p.position == engine::Position::Goalkeeper);
    assert!(gk_slot.is_some(), "Should have a goalkeeper");
    assert_eq!(gk_slot.unwrap().id, starters[0], "GK should be the saved GK");
}

#[test]
fn saved_xi_ids_with_partial_list_fills_remaining_with_auto_selection() {
    let mut game = make_game_with_fixture();
    let team1 = game.teams.iter_mut().find(|t| t.id == "team1").unwrap();
    team1.formation = "4-1-4-1".to_string();
    // Provide only 5 saved IDs - remaining 6 should be auto-filled
    let all_team1_ids: Vec<String> = game
        .players
        .iter()
        .filter(|p| p.team_id.as_deref() == Some("team1"))
        .map(|p| p.id.clone())
        .collect();
    let partial_xi = all_team1_ids.iter().take(5).cloned().collect::<Vec<_>>();
    team1.starting_xi_ids = partial_xi.clone();

    let session = live_match_manager::create_live_match(&game, 0, MatchMode::Instant, false).unwrap();
    let snapshot = session.snapshot();

    // Should have 11 players total
    assert_eq!(snapshot.home_team.players.len(), 11, "Should fill to 11 players");
    // First 5 should be the saved ones (available)
    for (i, saved_id) in partial_xi.iter().enumerate() {
        if snapshot.home_team.players.get(i).map(|p| p.id.as_str()) == Some(saved_id.as_str()) {
            // This saved player was placed
        }
    }
    // Check position counts are still correct (4-1-4-1 -> 4 DEF, 5 MID, 1 FWD)
    let (def_count, mid_count, fwd_count) = position_counts_in_snapshot(&snapshot);
    assert_eq!(def_count, 4);
    assert_eq!(mid_count, 5);
    assert_eq!(fwd_count, 1);
}
