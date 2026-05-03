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
use log::info;
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
fn generate_random_name() -> String {
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
        
        let full_name = generate_random_name();
        
        let mut new_player = Player::new(
            player_id.clone(),
            full_name.clone(),
            full_name,
            "2002-01-01".to_string(),
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

/// Handle AI team transfers during transfer window
pub fn ai_transfer_activity(game: &mut Game) {
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
    }
}

#[cfg(test)]
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
}
