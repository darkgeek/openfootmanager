use chrono::{TimeZone, Utc};
use domain::manager::Manager;
use domain::player::{Player, PlayerAttributes, Position};
use domain::team::Team;
use ofm_core::clock::GameClock;
use ofm_core::game::Game;
use ofm_core::training_report::{self, PlayerAttributeSnapshot};

fn default_attrs() -> PlayerAttributes {
    PlayerAttributes {
        pace: 70,
        stamina: 70,
        strength: 70,
        agility: 70,
        passing: 70,
        shooting: 70,
        tackling: 70,
        dribbling: 70,
        defending: 70,
        positioning: 70,
        vision: 70,
        decisions: 70,
        composure: 70,
        aggression: 50,
        teamwork: 70,
        leadership: 50,
        handling: 50,
        reflexes: 50,
        aerial: 50,
    }
}

fn make_player(id: &str, name: &str, team_id: &str) -> Player {
    let mut p = Player::new(
        id.to_string(),
        name.to_string(),
        format!("Full {}", name),
        "2000-01-01".to_string(),
        "England".to_string(),
        Position::Midfielder,
        default_attrs(),
    );
    p.team_id = Some(team_id.to_string());
    p.morale = 70;
    p.condition = 80;
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

fn make_game(month: u32, day: u32) -> Game {
    let date = Utc.with_ymd_and_hms(2026, month, day, 12, 0, 0).unwrap();
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

    let p1 = make_player("p1", "Player One", "team1");
    let p2 = make_player("p2", "Player Two", "team1");

    Game::new(clock, manager, vec![team1], vec![p1, p2], vec![], vec![])
}

#[test]
fn test_first_month_creates_snapshot() {
    // January 1st - first month, should create snapshot
    let mut game = make_game(1, 1);

    // No snapshots initially
    assert!(game.training_snapshots.is_empty());

    // Generate report
    training_report::generate_monthly_training_report(&mut game);

    // Should have created a snapshot
    assert_eq!(game.training_snapshots.len(), 1);
    assert_eq!(game.training_snapshots[0].players.len(), 2);

    // No messages should be created on first month
    assert!(game.messages.is_empty());
}

#[test]
fn test_second_month_generates_report() {
    // January 1st - create initial snapshot
    let mut game = make_game(1, 1);
    training_report::generate_monthly_training_report(&mut game);
    assert!(game.messages.is_empty());

    // Advance to February 1st
    game.clock.current_date = Utc.with_ymd_and_hms(2026, 2, 1, 12, 0, 0).unwrap();

    // Modify a player's attribute to simulate training progress
    game.players.iter_mut().find(|p| p.id == "p1").unwrap().attributes.pace = 72;

    // Generate report
    training_report::generate_monthly_training_report(&mut game);

    // Should have generated a report message
    assert_eq!(game.messages.len(), 1);
    let msg = &game.messages[0];
    assert!(msg.subject.contains("Training Report"));
    assert!(msg.subject.contains("February"));
    assert!(msg.body.contains("Player One")); // Player who improved
}

#[test]
fn test_no_change_generates_summary_report() {
    // January 1st - create initial snapshot
    let mut game = make_game(1, 1);
    training_report::generate_monthly_training_report(&mut game);

    // Advance to February 1st - no changes
    game.clock.current_date = Utc.with_ymd_and_hms(2026, 2, 1, 12, 0, 0).unwrap();

    // Generate report
    training_report::generate_monthly_training_report(&mut game);

    // Report should be generated even with no changes (shows summary message)
    assert_eq!(game.messages.len(), 1);
    let body = &game.messages[0].body;
    assert!(body.contains("No significant attribute changes"));
}

#[test]
fn test_attribute_change_calculation() {
    let mut game = make_game(1, 1);
    training_report::generate_monthly_training_report(&mut game);

    // Modify attributes
    let p1 = game.players.iter_mut().find(|p| p.id == "p1").unwrap();
    p1.attributes.pace = 75;      // +5
    p1.attributes.shooting = 68;   // -2
    p1.attributes.stamina = 70;    // no change

    // Advance to February
    game.clock.current_date = Utc.with_ymd_and_hms(2026, 2, 1, 12, 0, 0).unwrap();
    training_report::generate_monthly_training_report(&mut game);

    assert_eq!(game.messages.len(), 1);
    let body = &game.messages[0].body;
    assert!(body.contains("+5"));  // pace increase
    assert!(body.contains("-2")); // shooting decrease
}

#[test]
fn test_snapshot_preserves_all_attributes() {
    let mut game = make_game(1, 1);
    training_report::generate_monthly_training_report(&mut game);

    let snapshot = &game.training_snapshots[0];
    let player_snap = snapshot.players.iter().find(|p| p.player_id == "p1").unwrap();

    // Check all attributes are captured
    assert_eq!(player_snap.pace, 70);
    assert_eq!(player_snap.stamina, 70);
    assert_eq!(player_snap.strength, 70);
    assert_eq!(player_snap.passing, 70);
    assert_eq!(player_snap.shooting, 70);
    assert_eq!(player_snap.tackling, 70);
    assert_eq!(player_snap.dribbling, 70);
    assert_eq!(player_snap.defending, 70);
    assert_eq!(player_snap.positioning, 70);
    assert_eq!(player_snap.vision, 70);
    assert_eq!(player_snap.decisions, 70);
}

#[test]
fn test_no_team_no_snapshot() {
    let date = Utc.with_ymd_and_hms(2026, 1, 1, 12, 0, 0).unwrap();
    let clock = GameClock::new(date);
    let manager = Manager::new(
        "mgr1".to_string(),
        "Test".to_string(),
        "Manager".to_string(),
        "1980-01-01".to_string(),
        "England".to_string(),
    );
    // No team hired
    let game = Game::new(clock, manager, vec![], vec![], vec![], vec![]);

    let mut game = game;
    training_report::generate_monthly_training_report(&mut game);

    // No snapshot created without a team
    assert!(game.training_snapshots.is_empty());
}

#[test]
fn test_players_sorted_by_improvement() {
    let mut game = make_game(1, 1);
    training_report::generate_monthly_training_report(&mut game);

    // Player 1 improves a lot
    game.players.iter_mut().find(|p| p.id == "p1").unwrap().attributes.pace = 80;
    // Player 2 improves a little
    game.players.iter_mut().find(|p| p.id == "p2").unwrap().attributes.pace = 72;

    // Advance to February
    game.clock.current_date = Utc.with_ymd_and_hms(2026, 2, 1, 12, 0, 0).unwrap();
    training_report::generate_monthly_training_report(&mut game);

    let body = &game.messages[0].body;
    let p1_pos = body.find("Player One").unwrap();
    let p2_pos = body.find("Player Two").unwrap();
    
    // Player One should appear first (bigger improvement)
    assert!(p1_pos < p2_pos, "Player with bigger improvement should be listed first");
}

#[test]
fn test_overall_change_displayed() {
    let mut game = make_game(1, 1);
    training_report::generate_monthly_training_report(&mut game);

    // Player 1 improves significantly
    game.players.iter_mut().find(|p| p.id == "p1").unwrap().attributes.pace = 90;
    game.players.iter_mut().find(|p| p.id == "p1").unwrap().attributes.shooting = 90;

    // Advance to February
    game.clock.current_date = Utc.with_ymd_and_hms(2026, 2, 1, 12, 0, 0).unwrap();
    training_report::generate_monthly_training_report(&mut game);

    let body = &game.messages[0].body;
    // Should show overall change
    assert!(body.contains("Overall"));
}
