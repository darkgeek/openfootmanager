use chrono::{TimeZone, Utc};
use domain::manager::Manager;
use domain::player::{Player, PlayerAttributes, Position};
use domain::team::Team;
use ofm_core::clock::GameClock;
use ofm_core::game::Game;
use ofm_core::training_report;

fn make_player(id: &str, name: &str, team_id: &str) -> Player {
    let mut p = Player::new(
        id.to_string(),
        name.to_string(),
        format!("Full {}", name),
        "2000-01-01".to_string(),
        "England".to_string(),
        Position::Midfielder,
        PlayerAttributes {
            pace: 70, stamina: 70, strength: 70, agility: 70,
            passing: 70, shooting: 70, tackling: 70, dribbling: 70,
            defending: 70, positioning: 70, vision: 70, decisions: 70,
            composure: 70, aggression: 50, teamwork: 70, leadership: 50,
            handling: 50, reflexes: 50, aerial: 50,
        },
    );
    p.team_id = Some(team_id.to_string());
    p.morale = 70;
    p.condition = 80;
    p
}

fn make_game(month: u32, day: u32) -> Game {
    let date = Utc.with_ymd_and_hms(2027, month, day, 12, 0, 0).unwrap();
    let clock = GameClock::new(date);
    let mut manager = Manager::new(
        "mgr1".to_string(),
        "Test".to_string(),
        "Manager".to_string(),
        "1980-01-01".to_string(),
        "England".to_string(),
    );
    manager.hire("team1".to_string());
    let team1 = Team::new(
        "team1".to_string(), "Test FC".to_string(), "TFC".to_string(),
        "England".to_string(), "London".to_string(), "Stadium".to_string(), 40000,
    );
    let p1 = make_player("p1", "Player One", "team1");
    let p2 = make_player("p2", "Player Two", "team1");
    Game::new(clock, manager, vec![team1], vec![p1, p2], vec![], vec![])
}

#[test]
fn test_ensure_initial_snapshot_creates_snapshot() {
    let mut game = make_game(2, 5); // Feb 5 - any date
    assert!(game.training_snapshots.is_empty());
    
    training_report::ensure_initial_snapshot(&mut game);
    
    assert_eq!(game.training_snapshots.len(), 1);
    assert_eq!(game.training_snapshots[0].players.len(), 2);
    assert!(game.messages.is_empty()); // No report, just snapshot
}

#[test]
fn test_ensure_initial_snapshot_idempotent() {
    let mut game = make_game(2, 5);
    training_report::ensure_initial_snapshot(&mut game);
    assert_eq!(game.training_snapshots.len(), 1);
    
    // Second call should not add another snapshot
    training_report::ensure_initial_snapshot(&mut game);
    assert_eq!(game.training_snapshots.len(), 1);
}

#[test]
fn test_report_generated_after_ensure_and_next_month() {
    let mut game = make_game(1, 15); // Jan 15
    
    // Create initial snapshot
    training_report::ensure_initial_snapshot(&mut game);
    assert!(game.messages.is_empty());
    
    // Advance to Feb 1
    game.clock.current_date = Utc.with_ymd_and_hms(2027, 2, 1, 12, 0, 0).unwrap();
    
    // Modify a player
    game.players.iter_mut().find(|p| p.id == "p1").unwrap().attributes.pace = 75;
    
    training_report::generate_monthly_training_report(&mut game);
    
    assert!(!game.messages.is_empty(), "Report should have been generated");
    let msg = &game.messages[0];
    assert!(msg.subject.contains("Training Report"), "Subject should mention Training Report, got: {}", msg.subject);
    assert!(msg.subject.contains("February"), "Subject should mention February, got: {}", msg.subject);
    eprintln!("SUCCESS! Message subject: {}", msg.subject);
    eprintln!("Message body:\n{}", msg.body);
}

#[test]
fn test_full_process_day_flow() {
    let mut game = make_game(1, 31); // Jan 31
    
    // Ensure snapshot (simulating game load)
    training_report::ensure_initial_snapshot(&mut game);
    
    // Simulate process_day for Feb 1
    game.clock.current_date = Utc.with_ymd_and_hms(2027, 2, 1, 12, 0, 0).unwrap();
    
    // Modify a player
    game.players.iter_mut().find(|p| p.id == "p1").unwrap().attributes.pace = 74;
    
    training_report::generate_monthly_training_report(&mut game);
    
    let training_msgs: Vec<_> = game.messages.iter()
        .filter(|m| m.category == domain::message::MessageCategory::Training)
        .collect();
    
    assert!(!training_msgs.is_empty(), "Training messages should exist");
    eprintln!("SUCCESS! Found {} training message(s)", training_msgs.len());
    eprintln!("Subject: {}", training_msgs[0].subject);
}

#[test]
fn test_report_with_identical_attributes() {
    let mut game = make_game(2, 1); // Feb 1
    
    // Create first snapshot
    training_report::ensure_initial_snapshot(&mut game);
    
    // Mar 1 with NO attribute changes
    game.clock.current_date = Utc.with_ymd_and_hms(2027, 3, 1, 12, 0, 0).unwrap();
    training_report::generate_monthly_training_report(&mut game);
    
    // Should still generate a "no changes" message
    let training_msgs: Vec<_> = game.messages.iter()
        .filter(|m| m.category == domain::message::MessageCategory::Training)
        .collect();
    
    assert_eq!(training_msgs.len(), 1, "Should have 1 training message even with no changes");
    eprintln!("SUCCESS! No-changes report body:\n{}", training_msgs[0].body);
}
