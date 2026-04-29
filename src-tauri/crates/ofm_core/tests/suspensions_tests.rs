use chrono::{TimeZone, Utc};
use domain::league::{Fixture, FixtureCompetition, FixtureStatus, League, StandingEntry};
use domain::manager::Manager;
use domain::player::{Player, PlayerAttributes, Position};
use domain::team::Team;
use engine::report::{MatchReport, PlayerMatchStats, TeamStats};
use ofm_core::clock::GameClock;
use ofm_core::game::Game;
use ofm_core::turn;
use std::collections::HashMap;

// ---------------------------------------------------------------------------
// Test helpers
// ---------------------------------------------------------------------------

fn default_attrs() -> PlayerAttributes {
    PlayerAttributes {
        pace: 60,
        stamina: 60,
        strength: 60,
        agility: 60,
        passing: 60,
        shooting: 60,
        tackling: 60,
        dribbling: 60,
        defending: 60,
        positioning: 60,
        vision: 60,
        decisions: 60,
        composure: 60,
        aggression: 60,
        teamwork: 60,
        leadership: 60,
        handling: 30,
        reflexes: 30,
        aerial: 60,
    }
}

fn gk_attrs() -> PlayerAttributes {
    PlayerAttributes {
        pace: 40,
        stamina: 50,
        strength: 60,
        agility: 70,
        passing: 40,
        shooting: 20,
        tackling: 20,
        dribbling: 20,
        defending: 30,
        positioning: 70,
        vision: 50,
        decisions: 60,
        composure: 70,
        aggression: 30,
        teamwork: 60,
        leadership: 50,
        handling: 80,
        reflexes: 80,
        aerial: 70,
    }
}

fn make_player(id: &str, name: &str, team_id: &str, pos: Position) -> Player {
    let attrs = if pos == Position::Goalkeeper {
        gk_attrs()
    } else {
        default_attrs()
    };
    let mut p = Player::new(
        id.to_string(),
        name.to_string(),
        name.to_string(),
        "1995-01-01".to_string(),
        "England".to_string(),
        pos,
        attrs,
    );
    p.team_id = Some(team_id.to_string());
    p.morale = 70;
    p.condition = 100;
    p.fitness = 100;
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

fn make_squad(team_id: &str, prefix: &str) -> Vec<Player> {
    let mut players = Vec::new();
    // 1 GK
    players.push(make_player(
        &format!("{}_gk", prefix),
        &format!("{} GK", prefix),
        team_id,
        Position::Goalkeeper,
    ));
    // 4 DEF
    for i in 0..4 {
        players.push(make_player(
            &format!("{}_def{}", prefix, i),
            &format!("{} Def{}", prefix, i),
            team_id,
            Position::Defender,
        ));
    }
    // 4 MID
    for i in 0..4 {
        players.push(make_player(
            &format!("{}_mid{}", prefix, i),
            &format!("{} Mid{}", prefix, i),
            team_id,
            Position::Midfielder,
        ));
    }
    // 2 FWD
    for i in 0..2 {
        players.push(make_player(
            &format!("{}_fwd{}", prefix, i),
            &format!("{} Fwd{}", prefix, i),
            team_id,
            Position::Forward,
        ));
    }
    players
}

/// Creates a game with 2 teams and a single league fixture.
fn make_game_with_two_teams() -> Game {
    let date = Utc.with_ymd_and_hms(2025, 3, 1, 12, 0, 0).unwrap();
    let clock = GameClock::new(date);
    let mut manager = Manager::new(
        "mgr1".to_string(),
        "Test".to_string(),
        "Manager".to_string(),
        "1980-01-01".to_string(),
        "England".to_string(),
    );
    manager.hire("team1".to_string());

    let team1 = make_team("team1", "Home FC");
    let team2 = make_team("team2", "Away United");
    let mut players = make_squad("team1", "p1");
    players.extend(make_squad("team2", "p2"));

    let today = date.format("%Y-%m-%d").to_string();
    let league = League {
        id: "league1".to_string(),
        name: "Test League".to_string(),
        season: 2025,
        fixtures: vec![Fixture {
            id: "fixture1".to_string(),
            matchday: 1,
            date: today,
            home_team_id: "team1".to_string(),
            away_team_id: "team2".to_string(),
            competition: FixtureCompetition::League,
            status: FixtureStatus::Scheduled,
            result: None,
        }],
        standings: vec![
            StandingEntry::new("team1".to_string()),
            StandingEntry::new("team2".to_string()),
        ],
    };

    let mut game = Game::new(clock, manager, vec![team1, team2], players, vec![], vec![]);
    game.league = Some(league);
    game
}

/// Creates a match report with specified yellow/red cards.
fn report_with_cards(
    home_cards: Vec<(&str, u8, u8)>, // (player_id, yellow_cards, red_cards)
    away_cards: Vec<(&str, u8, u8)>,
) -> MatchReport {
    let mut player_stats = HashMap::new();

    // Helper to insert player stats
    let insert_player = |stats: &mut HashMap<String, PlayerMatchStats>,
                         id: &str,
                         yellows: u8,
                         reds: u8| {
        stats.insert(
            id.to_string(),
            PlayerMatchStats {
                minutes_played: 90,
                goals: 0,
                assists: 0,
                shots: 0,
                shots_on_target: 0,
                passes_completed: 30,
                passes_attempted: 35,
                tackles_won: 2,
                interceptions: 1,
                fouls_committed: 1,
                yellow_cards: yellows,
                red_cards: reds,
                rating: 6.5,
            },
        );
    };

    // Home team cards
    for (player_id, yellows, reds) in home_cards {
        insert_player(&mut player_stats, player_id, yellows, reds);
    }

    // Away team cards
    for (player_id, yellows, reds) in away_cards {
        insert_player(&mut player_stats, player_id, yellows, reds);
    }

    // Add default entries for all players in squads (no cards)
    let all_players = [
        "p1_gk", "p1_def0", "p1_def1", "p1_def2", "p1_def3",
        "p1_mid0", "p1_mid1", "p1_mid2", "p1_mid3",
        "p1_fwd0", "p1_fwd1",
        "p2_gk", "p2_def0", "p2_def1", "p2_def2", "p2_def3",
        "p2_mid0", "p2_mid1", "p2_mid2", "p2_mid3",
        "p2_fwd0", "p2_fwd1",
    ];
    for id in all_players {
        if !player_stats.contains_key(id) {
            player_stats.insert(
                id.to_string(),
                PlayerMatchStats {
                    minutes_played: 90,
                    ..Default::default()
                },
            );
        }
    }

    MatchReport {
        home_goals: 0,
        away_goals: 0,
        home_stats: TeamStats::default(),
        away_stats: TeamStats::default(),
        events: vec![],
        goals: vec![],
        player_stats,
        home_possession: 50.0,
        total_minutes: 90,
    }
}

// ---------------------------------------------------------------------------
// Suspension Tests
// ---------------------------------------------------------------------------

/// Red card should result in 1 match suspension
#[test]
fn red_card_gives_one_match_suspension() {
    let mut game = make_game_with_two_teams();
    let report = report_with_cards(
        vec![("p1_def0", 0, 1)], // p1_def0 gets a red card
        vec![],
    );
    turn::apply_match_report(&mut game, 0, "team1", "team2", &report);

    let player = game.players.iter().find(|p| p.id == "p1_def0").unwrap();
    assert_eq!(
        player.suspension_games_remaining, 1,
        "Player with red card should have 1 match suspension"
    );
}

/// Red card in friendly match should NOT result in suspension
#[test]
fn red_card_in_friendly_has_no_suspension() {
    let mut game = make_game_with_two_teams();
    // Make it a friendly
    if let Some(league) = &mut game.league {
        league.fixtures[0].competition = FixtureCompetition::Friendly;
    }
    let report = report_with_cards(
        vec![("p1_def0", 0, 1)],
        vec![],
    );
    turn::apply_match_report(&mut game, 0, "team1", "team2", &report);

    let player = game.players.iter().find(|p| p.id == "p1_def0").unwrap();
    assert_eq!(
        player.suspension_games_remaining, 0,
        "Red card in friendly should NOT result in suspension"
    );
}

/// 3 yellow cards in one match should result in 1 match suspension
#[test]
fn three_yellow_cards_gives_one_match_suspension() {
    let mut game = make_game_with_two_teams();
    let report = report_with_cards(
        vec![("p1_def1", 3, 0)], // p1_def1 gets 3 yellow cards
        vec![],
    );
    turn::apply_match_report(&mut game, 0, "team1", "team2", &report);

    let player = game.players.iter().find(|p| p.id == "p1_def1").unwrap();
    assert_eq!(
        player.suspension_games_remaining, 1,
        "Player with 3 yellow cards should have 1 match suspension"
    );
    assert_eq!(
        player.accumulated_yellow_cards, 0,
        "Accumulated yellows should reset after triggering suspension"
    );
}

/// 2 yellow cards in one match + 1 in another match = 3 total = suspension
#[test]
fn yellow_cards_accumulate_across_matches() {
    let mut game = make_game_with_two_teams();

    // Match 1: Player gets 2 yellow cards
    let report1 = report_with_cards(vec![("p1_def2", 2, 0)], vec![]);
    turn::apply_match_report(&mut game, 0, "team1", "team2", &report1);

    let player = game.players.iter().find(|p| p.id == "p1_def2").unwrap();
    assert_eq!(
        player.accumulated_yellow_cards, 2,
        "After 2 yellows, accumulated should be 2"
    );
    assert_eq!(player.suspension_games_remaining, 0, "No suspension yet (need 3)");

    // Add second fixture
    if let Some(league) = &mut game.league {
        let fixture2 = Fixture {
            id: "fixture2".to_string(),
            matchday: 2,
            date: "2025-03-08".to_string(),
            home_team_id: "team1".to_string(),
            away_team_id: "team2".to_string(),
            competition: FixtureCompetition::League,
            status: FixtureStatus::Scheduled,
            result: None,
        };
        league.fixtures.push(fixture2);
    }

    // Match 2: Player gets 1 yellow card → total 3 → suspension
    let report2 = report_with_cards(vec![("p1_def2", 1, 0)], vec![]);
    turn::apply_match_report(&mut game, 1, "team1", "team2", &report2);

    let player_after = game.players.iter().find(|p| p.id == "p1_def2").unwrap();
    assert_eq!(
        player_after.suspension_games_remaining, 1,
        "After 3 total yellows, should have 1 match suspension"
    );
    assert_eq!(
        player_after.accumulated_yellow_cards, 0,
        "Accumulated yellows should reset after suspension"
    );
}

/// Existing suspension should decrement when player participates in match
#[test]
fn existing_suspension_decrements_after_match() {
    let mut game = make_game_with_two_teams();

    // Player starts with 1 suspension
    if let Some(p) = game.players.iter_mut().find(|p| p.id == "p1_def3") {
        p.suspension_games_remaining = 1;
    }

    let report = report_with_cards(vec![], vec![]);
    turn::apply_match_report(&mut game, 0, "team1", "team2", &report);

    let player = game.players.iter().find(|p| p.id == "p1_def3").unwrap();
    assert_eq!(
        player.suspension_games_remaining, 0,
        "Suspension should decrement to 0 after match"
    );
}

/// Suspension should not decrement for players not in the match
#[test]
fn suspension_unchanged_for_uninvolved_players() {
    let mut game = make_game_with_two_teams();

    // Set suspension for player in match (p1_mid0) and player on third team (p3_gk)
    if let Some(p) = game.players.iter_mut().find(|p| p.id == "p1_mid0") {
        p.suspension_games_remaining = 1;
    }

    // Add third team with player that has suspension
    let team3 = make_team("team3", "Third FC");
    game.teams.push(team3);
    let mut player = make_player("p3_gk", "Third GK", "team3", Position::Goalkeeper);
    player.suspension_games_remaining = 1;
    game.players.push(player);

    let report = report_with_cards(vec![], vec![]);
    turn::apply_match_report(&mut game, 0, "team1", "team2", &report);

    let p1_mid = game.players.iter().find(|p| p.id == "p1_mid0").unwrap();
    assert_eq!(
        p1_mid.suspension_games_remaining, 0,
        "Suspension should decrement for player in match"
    );

    let p3_gk = game.players.iter().find(|p| p.id == "p3_gk").unwrap();
    assert_eq!(
        p3_gk.suspension_games_remaining, 1,
        "Suspension should NOT change for player not in match"
    );
}

/// Multiple red cards in one match = multiple suspensions
#[test]
fn multiple_red_cards_gives_multiple_suspensions() {
    let mut game = make_game_with_two_teams();
    let report = report_with_cards(
        vec![("p1_def0", 0, 2)], // p1_def0 gets 2 red cards
        vec![],
    );
    turn::apply_match_report(&mut game, 0, "team1", "team2", &report);

    let player = game.players.iter().find(|p| p.id == "p1_def0").unwrap();
    assert_eq!(
        player.suspension_games_remaining, 2,
        "Player with 2 red cards should have 2 match suspensions"
    );
}

/// Yellow cards in friendly do NOT accumulate
#[test]
fn yellow_cards_in_friendly_do_not_count() {
    let mut game = make_game_with_two_teams();
    // Make it a friendly
    if let Some(league) = &mut game.league {
        league.fixtures[0].competition = FixtureCompetition::Friendly;
    }
    let report = report_with_cards(
        vec![("p1_mid1", 2, 0)], // p1_mid1 gets 2 yellows in friendly
        vec![],
    );
    turn::apply_match_report(&mut game, 0, "team1", "team2", &report);

    let player = game.players.iter().find(|p| p.id == "p1_mid1").unwrap();
    assert_eq!(
        player.accumulated_yellow_cards, 0,
        "Yellow cards in friendly should NOT accumulate"
    );
}

/// Player gets red card AND already has suspension → both apply
#[test]
fn red_card_with_existing_suspension() {
    let mut game = make_game_with_two_teams();

    // Player has existing suspension
    if let Some(p) = game.players.iter_mut().find(|p| p.id == "p1_mid2") {
        p.suspension_games_remaining = 1;
    }

    // Gets red card in this match
    let report = report_with_cards(vec![("p1_mid2", 0, 1)], vec![]);
    turn::apply_match_report(&mut game, 0, "team1", "team2", &report);

    let player = game.players.iter().find(|p| p.id == "p1_mid2").unwrap();
    // 1 (existing) - 1 (match participation) + 1 (new red) = 1
    assert_eq!(
        player.suspension_games_remaining, 1,
        "Existing suspension decrements, then new red adds: net 1 suspension"
    );
}

/// 6 yellow cards = 2 suspensions
#[test]
fn six_yellow_cards_gives_two_suspensions() {
    let mut game = make_game_with_two_teams();
    let report = report_with_cards(
        vec![("p1_fwd0", 6, 0)], // p1_fwd0 gets 6 yellow cards
        vec![],
    );
    turn::apply_match_report(&mut game, 0, "team1", "team2", &report);

    let player = game.players.iter().find(|p| p.id == "p1_fwd0").unwrap();
    assert_eq!(
        player.suspension_games_remaining, 2,
        "Player with 6 yellow cards should have 2 match suspensions"
    );
    assert_eq!(
        player.accumulated_yellow_cards, 0,
        "Accumulated yellows should reset"
    );
}

/// Away team player gets red card - should also get suspension
#[test]
fn away_team_red_card_gives_suspension() {
    let mut game = make_game_with_two_teams();
    let report = report_with_cards(
        vec![],
        vec![("p2_def0", 0, 1)], // p2_def0 (away team) gets red card
    );
    turn::apply_match_report(&mut game, 0, "team1", "team2", &report);

    let player = game.players.iter().find(|p| p.id == "p2_def0").unwrap();
    assert_eq!(
        player.suspension_games_remaining, 1,
        "Away team player with red card should have 1 match suspension"
    );
}

/// Three accumulated yellow cards should reset to 0, not 1
#[test]
fn accumulated_yellows_reset_properly() {
    let mut game = make_game_with_two_teams();

    // Simulate a player who already has 2 accumulated yellows
    if let Some(p) = game.players.iter_mut().find(|p| p.id == "p1_fwd1") {
        p.accumulated_yellow_cards = 2;
    }

    // Now gets 1 yellow card → total 3 → suspension, reset to 0
    let report = report_with_cards(vec![("p1_fwd1", 1, 0)], vec![]);
    turn::apply_match_report(&mut game, 0, "team1", "team2", &report);

    let player = game.players.iter().find(|p| p.id == "p1_fwd1").unwrap();
    assert_eq!(
        player.suspension_games_remaining, 1,
        "Should have 1 suspension"
    );
    assert_eq!(
        player.accumulated_yellow_cards, 0,
        "Accumulated yellows should reset to 0, not 1"
    );
}

/// Injured players still serve their suspension when missing matches
#[test]
fn injured_players_serve_suspension_too() {
    let mut game = make_game_with_two_teams();

    // Player has suspension AND is injured
    if let Some(p) = game.players.iter_mut().find(|p| p.id == "p1_def0") {
        p.suspension_games_remaining = 2;
        p.injury = Some(domain::player::Injury {
            name: "Hamstring".to_string(),
            days_remaining: 10, // Will miss multiple matches
        });
    }

    // Simulate match (player won't play due to injury, but suspension still decrements)
    let report = report_with_cards(vec![], vec![]);
    turn::apply_match_report(&mut game, 0, "team1", "team2", &report);

    let player = game.players.iter().find(|p| p.id == "p1_def0").unwrap();
    assert_eq!(
        player.suspension_games_remaining, 1,
        "Suspension should decrement even though player is injured"
    );
    assert!(
        player.injury.is_some(),
        "Injury should remain"
    );
}

/// Second yellow card (two yellows = one red) should trigger suspension.
/// SecondYellow is recorded as 1 yellow + 1 red in player_stats.
#[test]
fn second_yellow_gives_one_match_suspension() {
    let mut game = make_game_with_two_teams();
    // Second yellow = 1 yellow card (the second one) + 1 red card
    // This simulates the match report where SecondYellow was recorded
    let report = report_with_cards(
        vec![("p1_def0", 1, 1)], // 1 yellow + 1 red (SecondYellow)
        vec![],
    );
    turn::apply_match_report(&mut game, 0, "team1", "team2", &report);

    let player = game.players.iter().find(|p| p.id == "p1_def0").unwrap();
    assert_eq!(
        player.suspension_games_remaining, 1,
        "Player with SecondYellow (1 yellow + 1 red) should have 1 match suspension"
    );
}