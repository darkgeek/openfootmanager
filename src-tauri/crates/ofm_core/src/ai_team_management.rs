//! AI Team Management Module
//! 
//! Handles AI team squad replenishment and transfers:
//! - Minimum squad requirements (3 GK, 20 outfield)
//! - Sign youth players when below minimum (outside transfer window)
//! - AI team transfers during transfer window
//! - End of season replenishment

use crate::game::Game;
use domain::player::{Player, PlayerAttributes, Position};
use domain::season::TransferWindowStatus;
use log::{info, warn};
use rand::seq::SliceRandom;
use rand::RngExt;

/// Minimum squad size requirements
const MIN_GOALKEEPERS: usize = 3;
const MIN_OUTFIELD_PLAYERS: usize = 20;

/// Check if transfer window is open
pub fn transfer_window_is_open(game: &Game) -> bool {
    matches!(
        game.season_context.transfer_window.status,
        TransferWindowStatus::Open | TransferWindowStatus::DeadlineDay
    )
}

/// Count goalkeepers and outfield players for a team
fn count_squad_size(game: &Game, team_id: &str) -> (usize, usize) {
    let team_players: Vec<&Player> = game.players.iter()
        .filter(|p| p.team_id.as_deref() == Some(team_id))
        .collect();
    
    let gks = team_players.iter().filter(|p| p.position == Position::Goalkeeper).count();
    let outfield = team_players.len() - gks;
    
    (gks, outfield)
}

/// Find free agents available for AI teams
fn find_available_free_agents<'a>(game: &'a Game, position: Option<&'a Position>) -> Vec<&'a Player> {
    game.players.iter()
        .filter(|p| {
            p.team_id.is_none()
            && position.map(|pos| &p.position == pos).unwrap_or(true)
        })
        .collect()
}

/// Calculate youth player attributes
fn generate_youth_attributes(base_ovr: u8) -> PlayerAttributes {
    let mut rng = rand::rng();
    
    PlayerAttributes {
        pace: (base_ovr as i16 + rng.random_range(-8..=8)).clamp(40, 85) as u8,
        stamina: (base_ovr as i16 + rng.random_range(-8..=8)).clamp(40, 85) as u8,
        strength: (base_ovr as i16 + rng.random_range(-8..=8)).clamp(40, 85) as u8,
        agility: (base_ovr as i16 + rng.random_range(-8..=8)).clamp(40, 85) as u8,
        passing: (base_ovr as i16 + rng.random_range(-8..=8)).clamp(40, 85) as u8,
        shooting: (base_ovr as i16 + rng.random_range(-8..=8)).clamp(40, 85) as u8,
        tackling: (base_ovr as i16 + rng.random_range(-8..=8)).clamp(40, 85) as u8,
        dribbling: (base_ovr as i16 + rng.random_range(-8..=8)).clamp(40, 85) as u8,
        defending: (base_ovr as i16 + rng.random_range(-8..=8)).clamp(40, 85) as u8,
        positioning: (base_ovr as i16 + rng.random_range(-8..=8)).clamp(40, 85) as u8,
        vision: (base_ovr as i16 + rng.random_range(-8..=8)).clamp(40, 85) as u8,
        decisions: (base_ovr as i16 + rng.random_range(-8..=8)).clamp(40, 85) as u8,
        composure: (base_ovr as i16 + rng.random_range(-8..=8)).clamp(40, 85) as u8,
        aggression: (base_ovr as i16 + rng.random_range(-8..=8)).clamp(40, 85) as u8,
        teamwork: (base_ovr as i16 + rng.random_range(-8..=8)).clamp(40, 85) as u8,
        leadership: rng.random_range(20..60),
        handling: if base_ovr < 60 { (base_ovr as i16 + rng.random_range(-8..=8)).clamp(40, 85) as u8 } else { 30 },
        reflexes: if base_ovr < 60 { (base_ovr as i16 + rng.random_range(-8..=8)).clamp(40, 85) as u8 } else { 30 },
        aerial: (base_ovr as i16 + rng.random_range(-8..=8)).clamp(40, 85) as u8,
    }
}

/// Generate a random name
/// Generate a random name appropriate for the given country.
/// Uses the nationality name pools from the generator if available,
/// falling back to simple English defaults for unrecognised countries.
fn generate_random_name(country: &str) -> String {
    // Map country to ISO code to pick the right name pool
    let code = crate::generator::country_to_iso(country);
    // For CN, return Chinese-format name: surname + given name (no space)
    if code == "CN" {
        let last_names = ["王", "李", "张", "刘", "陈", "杨", "赵", "黄", "周", "吴",
            "徐", "孙", "胡", "朱", "高", "林", "何", "郭", "马", "罗"];
        let first_names = ["志强", "伟杰", "明辉", "浩宇", "俊杰", "建平", "志明", "永强", "建国", "文杰",
            "海涛", "卫东", "建华", "志伟", "嘉诚", "瑞华", "晓明", "洪波", "泽宇",
            "天佑", "宇轩", "子涵", "梓豪", "雨泽", "俊豪", "博文", "鹏飞", "伟国", "振华"];
        let mut rng = rand::rng();
        let last = last_names[rng.random_range(0..last_names.len())];
        let first = first_names[rng.random_range(0..first_names.len())];
        return format!("{}{}", last, first);
    }
    let first_names = ["James", "Marcus", "Oliver", "Luke", "Harry", "Jack", "Thomas", "William", "Daniel", "Ryan"];
    let last_names = ["Smith", "Jones", "Williams", "Brown", "Taylor", "Wilson", "Davies", "Evans", "Thomas", "Roberts"];
    let mut rng = rand::rng();
    let first = first_names[rng.random_range(0..first_names.len())];
    let last = last_names[rng.random_range(0..last_names.len())];
    format!("{} {}", first, last)
}

/// Sign a youth player to an AI team
fn sign_youth_player(game: &mut Game, team_id: &str, position: Position) -> Option<String> {
    let team = game.teams.iter().find(|t| t.id == team_id)?;
    let team_name = &team.name;
    
    let available: Vec<&Player> = find_available_free_agents(game, Some(&position));
    
    if available.is_empty() {
        let base_ovr = rand::rng().random_range(55..=70);
        let attrs = generate_youth_attributes(base_ovr);
        
        let mut rng = rand::rng();
        let player_id = format!("ai_youth_{}_{}", team_id, rng.random::<u64>());
        
        let full_name = generate_random_name(&team.country);
        
        let birth_year = game.clock.current_date.format("%Y").to_string().parse::<u32>().unwrap_or(2026);
        let age = rng.random_range(15..=18);
        let birth_date = format!("{}-01-01", birth_year - age);

        let mut new_player = Player::new(
            player_id.clone(),
            full_name.clone(),
            full_name,
            birth_date,
            team.country.clone(),
            position.clone(),
            attrs,
        );
        new_player.team_id = Some(team_id.to_string());
        new_player.contract_end = Some(format!("{}-06-30", game.clock.current_date.format("%Y")));
        new_player.market_value = (base_ovr as u64 * 100_000);
        new_player.wage = (base_ovr as u32 * 1000);
        
        game.players.push(new_player);
        info!("[AI Team] {} signed youth player for {:?}", team_name, position);
        return Some(player_id);
    }
    
    let player_id = available[0].id.clone();
    if let Some(player) = game.players.iter_mut().find(|p| p.id == player_id) {
        player.team_id = Some(team_id.to_string());
        player.contract_end = Some(format!("{}-06-30", game.clock.current_date.format("%Y")));
        info!("[AI Team] {} signed free agent {}", team_name, player.match_name);
    }
    
    Some(player_id)
}

/// Replenish AI team squad to meet minimum requirements
pub fn ai_replenish_squad_offseason(game: &mut Game) {
    let user_team_id = game.manager.team_id.clone().unwrap_or_default();
    
    // Collect team IDs to avoid borrow issues
    let team_ids: Vec<String> = game.teams.iter()
        .filter(|t| t.id != user_team_id)
        .map(|t| t.id.clone())
        .collect();
    
    for team_id in team_ids {
        let team_name = game.teams.iter().find(|t| t.id == team_id).map(|t| t.name.clone()).unwrap_or_default();
        let (gks, outfield) = count_squad_size(game, &team_id);
        
        if gks < MIN_GOALKEEPERS {
            let needed = MIN_GOALKEEPERS - gks;
            info!("[AI Team] {} needs {} more goalkeeper(s)", team_name, needed);
            for _ in 0..needed {
                let _ = sign_youth_player(game, &team_id, Position::Goalkeeper);
            }
        }
        
        if outfield < MIN_OUTFIELD_PLAYERS {
            let needed = MIN_OUTFIELD_PLAYERS - outfield;
            info!("[AI Team] {} needs {} more outfield player(s)", team_name, needed);
            
            let positions = vec![
                Position::Defender,
                Position::Midfielder,
                Position::Forward,
            ];
            
            for i in 0..needed {
                let pos = positions[i % positions.len()].clone();
                let _ = sign_youth_player(game, &team_id, pos);
            }
        }
    }
}


/// Replenish any AI team that is below minimum squad size.
/// This is the authoritative fallback: it signs youth/free-agent players
/// to guarantee every AI team has at least MIN_GOALKEEPERS GK and
/// MIN_OUTFIELD_PLAYERS outfield players regardless of transfer budget.
///
/// Called at the end of every ai_transfer_activity() and separately on
/// match days when ai_transfer_activity is skipped, to ensure no AI
/// team goes under-staffed.
pub fn ai_replenish_squad(game: &mut Game) {
    let user_team_id = game.manager.team_id.clone().unwrap_or_default();
    
    let team_ids: Vec<String> = game.teams.iter()
        .filter(|t| t.id != user_team_id)
        .map(|t| t.id.clone())
        .collect();
    
    for team_id in team_ids {
        let team_name = game.teams.iter().find(|t| t.id == team_id).map(|t| t.name.clone()).unwrap_or_default();
        let (gks, outfield) = count_squad_size(game, &team_id);
        
        let mut signed = false;
        
        if gks < MIN_GOALKEEPERS {
            let needed = MIN_GOALKEEPERS - gks;
            for _ in 0..needed {
                let _ = sign_youth_player(game, &team_id, Position::Goalkeeper);
                signed = true;
            }
        }
        
        if outfield < MIN_OUTFIELD_PLAYERS {
            let needed = MIN_OUTFIELD_PLAYERS - outfield;
            let positions = [
                Position::Defender,
                Position::Midfielder,
                Position::Forward,
            ];
            for i in 0..needed {
                let pos = positions[i % positions.len()].clone();
                let _ = sign_youth_player(game, &team_id, pos);
                signed = true;
            }
        }
        
        if signed {
            let (new_gks, new_outfield) = count_squad_size(game, &team_id);
            info!("[AI Squad] {} replenished to {} GK / {} outfield (was {} / {})",
                team_name, new_gks, new_outfield, gks, outfield);
        }
    }
}

/// Handle AI team transfers during transfer window
pub fn ai_transfer_activity(game: &mut Game) {
    log::info!("[ai_transfer_activity] called: transfer_window_open={}", transfer_window_is_open(game));
    if !transfer_window_is_open(game) {
        return;
    }
    
    let user_team_id = game.manager.team_id.clone().unwrap_or_default();
    
    // Collect team IDs to avoid borrow issues
    let team_ids: Vec<String> = game.teams.iter()
        .filter(|t| t.id != user_team_id)
        .map(|t| t.id.clone())
        .collect();
    
    for team_id in team_ids {
        let team = game.teams.iter().find(|t| t.id == team_id).cloned();
        let Some(team) = team else { continue; };
        
        let (gks, outfield) = count_squad_size(game, &team_id);
        
        if gks >= MIN_GOALKEEPERS && outfield >= MIN_OUTFIELD_PLAYERS {
            continue;
        }
        
        // Collect candidate IDs to avoid borrow issues
        let candidate_ids: Vec<(String, u64)> = game.players.iter()
            .filter(|p| {
                p.team_id.as_ref().map(|tid| tid != &team_id).unwrap_or(false)
                && p.team_id.as_ref() != Some(&user_team_id)
                && p.transfer_listed
            })
            .map(|p| (p.id.clone(), p.market_value))
            .collect();
        
        let mut rng = rand::rng();
        let mut candidates = candidate_ids;
        candidates.shuffle(&mut rng);
        
        let mut bought = 0;
        let mut current_gks = gks;
        let mut current_outfield = outfield;
        
        for (candidate_id, fee) in candidates {
            if current_gks < MIN_GOALKEEPERS {
                if team.transfer_budget >= fee as i64 {
                    execute_ai_purchase(game, &team_id, &candidate_id, fee);
                    current_gks += 1;
                    bought += 1;
                }
            } else if current_outfield < MIN_OUTFIELD_PLAYERS {
                if team.transfer_budget >= fee as i64 {
                    execute_ai_purchase(game, &team_id, &candidate_id, fee);
                    current_outfield += 1;
                    bought += 1;
                }
            }
            
            if current_gks >= MIN_GOALKEEPERS && current_outfield >= MIN_OUTFIELD_PLAYERS {
                break;
            }
            
            if bought >= 3 {
                break;
            }
        }
    }
    
    // Replenish squad size for any AI team still below minimum.
    // ai_transfer_activity only buys when budget allows; ai_replenish_squad
    // signs youth players unconditionally to ensure no team goes understaffed.
    ai_replenish_squad(game);
}

/// Execute a player purchase between AI teams
fn execute_ai_purchase(game: &mut Game, buyer_id: &str, player_id: &str, fee: u64) {
    // Find and update player first
    let player = game.players.iter_mut().find(|p| p.id == player_id);
    let Some(player) = player else { return; };
    
    let seller_team_id = player.team_id.clone();
    let player_name = player.full_name.clone();
    
    // Update player
    player.team_id = Some(buyer_id.to_string());
    player.transfer_listed = false;
    player.transfer_offers.clear();
    
    let season_year: u32 = game.clock.current_date.format("%Y").to_string().parse().unwrap_or(2026);
    player.contract_end = Some(format!("{}-06-30", season_year + 2));
    player.wage = (player.wage as f32 * 1.05) as u32;
    player.market_value = (player.market_value as f32 * 1.1) as u64;
    
    // Update buyer team
    if let Some(buyer) = game.teams.iter_mut().find(|t| t.id == buyer_id) {
        buyer.finance -= fee as i64;
        buyer.transfer_budget -= fee as i64;
    }
    
    // Update seller team
    if let Some(seller_id) = seller_team_id {
        if let Some(seller) = game.teams.iter_mut().find(|t| t.id == seller_id) {
            seller.finance += fee as i64;
        }
    }
    
    info!("[AI Transfer] {} bought {} for ${}", buyer_id, player_name, fee);
}

/// End of season AI team replenishment
pub fn ai_end_of_season_replenishment(game: &mut Game) {
    info!("[AI Team] Running end of season replenishment");
    
    let user_team_id = game.manager.team_id.clone().unwrap_or_default();
    
    // Collect team IDs to avoid borrow issues
    let team_ids: Vec<String> = game.teams.iter()
        .filter(|t| t.id != user_team_id)
        .map(|t| t.id.clone())
        .collect();
    
    for team_id in team_ids {
        let team_name = game.teams.iter().find(|t| t.id == team_id).map(|t| t.name.clone()).unwrap_or_default();
        let (gks, outfield) = count_squad_size(game, &team_id);
        
        let mut total_needed = 0;
        
        if gks < MIN_GOALKEEPERS {
            let needed = MIN_GOALKEEPERS - gks;
            total_needed += needed;
            for _ in 0..needed {
                let _ = sign_youth_player(game, &team_id, Position::Goalkeeper);
            }
        }
        
        if outfield < MIN_OUTFIELD_PLAYERS {
            let needed = MIN_OUTFIELD_PLAYERS - outfield;
            total_needed += needed;
            
            let positions = vec![
                Position::Defender,
                Position::Midfielder,
                Position::Forward,
            ];
            
            for i in 0..needed {
                let pos = positions[i % positions.len()].clone();
                let _ = sign_youth_player(game, &team_id, pos);
            }
        }
        
        if total_needed > 0 {
            info!("[AI Team] {} signed {} youth players for new season", team_name, total_needed);
        }
        
        // List some squad players for transfer to generate market activity.
        // This creates supply for the AI teams to buy and for the user to browse.
        let squad_ids: Vec<String> = game.players.iter()
            .filter(|p| p.team_id.as_deref() == Some(&team_id) && p.position != Position::Goalkeeper)
            .map(|p| p.id.clone())
            .collect();
        if squad_ids.len() > 5 {
            // List up to 20% of the non-GK squad
            let list_count = (squad_ids.len() as f32 * 0.2).ceil() as usize;
            for player_id in squad_ids.iter().take(list_count) {
                if let Some(player) = game.players.iter_mut().find(|p| &p.id == player_id) {
                    if !player.transfer_listed && !player.loan_listed {
                        player.transfer_listed = true;
                        info!("[AI Team] {} listed {} on transfer market", team_name, player.full_name);
                    }
                }
            }
        }
    }
}

/// Bootstrap the transfer market at game start or load.
/// If the market has too few listed players, list squad players from AI teams.
/// This runs once to create initial market supply for the user to browse.
pub fn initialize_transfer_market(game: &mut Game) {
    let market_count = game.players.iter().filter(|p| p.transfer_listed).count();
    if market_count < 5 {
        info!("[ai_transfer_activity] bootstrap: market has only {} listed players, seeding...", market_count);
        let user_team_id = game.manager.team_id.clone().unwrap_or_default();
        for team in game.teams.iter() {
            if team.id == user_team_id {
                continue;
            }
            // List up to 5 non-GK squad players for each AI team
            let candidates: Vec<String> = game.players.iter()
                .filter(|p| {
                    p.team_id.as_deref() == Some(&team.id)
                        && p.position != Position::Goalkeeper
                        && !p.transfer_listed
                        && !p.loan_listed
                })
                .map(|p| p.id.clone())
                .collect();
            let list_count = 5.min(candidates.len());
            for pid in candidates.iter().take(list_count) {
                if let Some(p) = game.players.iter_mut().find(|pl| pl.id == *pid) {
                    p.transfer_listed = true;
                    info!("[ai_transfer_activity] bootstrap: listed {} ({:?})", p.full_name, p.position);
                }
            }
            if list_count > 0 {
                info!("[ai_transfer_activity] bootstrap: {} listed {} players", team.name, list_count);
            }
        }
    } else {
        info!("[ai_transfer_activity] bootstrap: market already has {} listed players, skipping", market_count);
    }
}
mod tests {
    use super::*;
    use crate::game::Game;
    use crate::clock::GameClock;
    use domain::manager::Manager;
    use domain::player::Player;
    use domain::player::PlayerAttributes;
    use domain::team::Team;
    use chrono::TimeZone;

    fn make_test_game_with_team(team_id: &str) -> Game {
        let clock = GameClock::new(chrono::Utc.with_ymd_and_hms(2025, 6, 15, 12, 0, 0).unwrap());
        let mut manager = Manager::new(
            "mgr1".to_string(),
            "Test".to_string(),
            "Manager".to_string(),
            "1980-01-01".to_string(),
            "England".to_string(),
        );
        manager.hire(team_id.to_string());

        let team = Team::new(
            team_id.to_string(),
            "Test FC".to_string(),
            "TST".to_string(),
            "England".to_string(),
            "London".to_string(),
            "Test Ground".to_string(),
            30_000,
        );

        let attrs = PlayerAttributes {
            pace: 50,
            stamina: 50,
            strength: 50,
            agility: 50,
            passing: 50,
            shooting: 50,
            tackling: 50,
            dribbling: 50,
            defending: 50,
            positioning: 50,
            vision: 50,
            decisions: 50,
            composure: 50,
            aggression: 50,
            teamwork: 50,
            leadership: 50,
            handling: 75,
            reflexes: 75,
            aerial: 50,
        };
        let gk = Player::new(
            format!("{}_gk", team_id),
            "Test".to_string(),
            "GK".to_string(),
            "2000-01-01".to_string(),
            "England".to_string(),
            Position::Goalkeeper,
            attrs,
        );

        Game::new(clock, manager, vec![team], vec![gk], vec![], vec![])
    }

    #[test]
    fn test_minimum_squad_requirements() {
        let team_id = "test_ai_team";
        let mut game = make_test_game_with_team(team_id);

        game.teams.push(Team::new(
            team_id.to_string(),
            "Test AI FC".to_string(),
            "TAI".to_string(),
            "England".to_string(),
            "London".to_string(),
            "Test Ground".to_string(),
            30_000,
        ));

        let (gks, outfield) = count_squad_size(&game, team_id);
        assert_eq!(gks, 0);
        assert_eq!(outfield, 0);

        sign_youth_player(&mut game, team_id, Position::Goalkeeper);
        let (gks, _) = count_squad_size(&game, team_id);
        assert_eq!(gks, 1);

        sign_youth_player(&mut game, team_id, Position::Midfielder);
        let (_, outfield) = count_squad_size(&game, team_id);
        assert_eq!(outfield, 1);
    }

    #[test]
    fn sign_youth_player_age_is_15_to_18() {
        let team_id = "test_age_team";
        let mut game = make_test_game_with_team(team_id);
        game.teams.push(Team::new(
            team_id.to_string(),
            "Test Age FC".to_string(),
            "TAG".to_string(),
            "England".to_string(),
            "London".to_string(),
            "Test Ground".to_string(),
            30_000,
        ));

        // Generate several youth players and verify each is 15-18
        let current_year: u32 = game.clock.current_date.format("%Y").to_string().parse().unwrap();
        for _ in 0..10 {
            sign_youth_player(&mut game, team_id, Position::Defender);
        }

        let team_players: Vec<&Player> = game.players.iter()
            .filter(|p| p.team_id.as_deref() == Some(team_id))
            .collect();

        assert!(team_players.len() >= 10, "Should have generated at least 10 youth players");

        for player in &team_players {
            let birth_year: u32 = player.date_of_birth.split('-').next()
                .and_then(|y| y.parse().ok())
                .unwrap_or(0);
            let age = current_year - birth_year;
            assert!(
                (15..=18).contains(&age),
                "Youth player {} age {} should be between 15 and 18",
                player.match_name, age
            );
        }
    }

    #[test]
    fn ai_end_of_season_lists_players_for_transfer() {
        let user_team_id = "user_team";
        let ai_team_id = "ai_team";
        let mut game = make_test_game_with_team(user_team_id);

        // Add a second AI team to process (ai_end_of_season_replenishment skips user team)
        game.teams.push(Team::new(
            ai_team_id.to_string(),
            "AI FC".to_string(),
            "AI".to_string(),
            "England".to_string(),
            "London".to_string(),
            "AI Ground".to_string(),
            30_000,
        ));

        // Add 10 outfield players to the AI team (enough to trigger listing)
        let attrs = PlayerAttributes {
            pace: 50, stamina: 50, strength: 50, agility: 50,
            passing: 50, shooting: 50, tackling: 50, dribbling: 50,
            defending: 50, positioning: 50, vision: 50, decisions: 50,
            composure: 50, aggression: 50, teamwork: 50, leadership: 30,
            handling: 30, reflexes: 30, aerial: 50,
        };
        for i in 0..10 {
            let mut p = Player::new(
                format!("ai_player_{}", i),
                format!("AIPlayer{}", i),
                "Surname".to_string(),
                "2000-01-01".to_string(),
                "England".to_string(),
                Position::Midfielder,
                attrs.clone(),
            );
            p.team_id = Some(ai_team_id.to_string());
            game.players.push(p);
        }

        // Before replenishment: no listed players
        let listed_before = game.players.iter().filter(|p| p.transfer_listed).count();
        assert_eq!(listed_before, 0);

        ai_end_of_season_replenishment(&mut game);

        // After replenishment: some players should be listed
        let listed_after = game.players.iter().filter(|p| p.transfer_listed).count();
        assert!(listed_after > 0, "Expected some players to be listed, but none were");

        // Listed players should belong to AI teams, not the user team
        for player in game.players.iter().filter(|p| p.transfer_listed) {
            assert_ne!(
                player.team_id.as_ref(),
                Some(&user_team_id.to_string()),
                "User's own players should not be listed by AI"
            );
        }
    }
}
