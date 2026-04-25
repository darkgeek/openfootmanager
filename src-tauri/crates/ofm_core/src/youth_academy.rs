//! Youth Academy Recruitment System
//! 
//! Each month, the youth academy generates recommendations for new young players
//! that the club can recruit. Recruitment cost depends on player quality and
//! the club's youth facilities.

use crate::game::Game;
use domain::player::{Player, PlayerAttributes, Position};
use rand::{Rng, RngExt};
use uuid::Uuid;

/// A recommended youth player that the club can recruit
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct YouthRecommendation {
    pub id: String,
    pub player: Player,
    pub recruitment_cost: i64,
    pub expires_at: String, // Date when this recommendation expires
    pub facility_bonus: i32, // How much youth facilities influenced the recommendation
}

/// Generate new youth recommendations for the current month.
/// Called from process_day when it's the first day of a new month.
pub fn generate_monthly_recommendations(game: &mut Game) {
    let current_month = game.clock.current_date.format("%Y-%m").to_string();
    
    // Check if we already have recommendations for this month
    // The recommendation ID contains the month it was generated in
    if game.youth_recommendations.iter().any(|r| {
        // Extract month from ID (format: "2026-08_0_uuid")
        r.id.split('_').next().map_or(false, |m| m == current_month)
    }) {
        return; // Already generated this month
    }
    
    let user_team_id = match &game.manager.team_id {
        Some(id) => id.clone(),
        None => return,
    };
    
    let team = match game.teams.iter().find(|t| t.id == user_team_id) {
        Some(t) => t,
        None => return,
    };
    
    // Youth facilities affect how many recommendations and quality
    let youth_facilities = team.facilities.youth;
    let facility_factor = (youth_facilities as f64 / 20.0).min(1.0);
    
    // Number of recommendations: 1-3 based on facilities (better facilities = more)
    let num_recommendations = (1.0 + (facility_factor * 2.0)) as usize;
    let num_recommendations = num_recommendations.min(3);
    
    // Team reputation affects the potential of recommended players
    let team_reputation = team.reputation as f64 / 1000.0;
    
    let mut rng = rand::rng();
    
    for i in 0..num_recommendations {
        // Generate a young player (15-17 years old)
        let age = rng.random_range(15..=17);
        
        // Player potential is influenced by team reputation and some randomness
        let base_potential = 50.0 + (team_reputation * 30.0);
        let potential_bonus = rng.random_range(0.0..20.0);
        let potential = (base_potential + potential_bonus).min(95.0) as u8;
        
        // Generate player
        let player = generate_youth_player(
            &format!("youth_{}_{}", current_month, i),
            age,
            potential,
            &team.country,
            &mut rng,
        );
        
        // Recruitment cost based on potential and youth facilities
        let base_cost = ((potential as i64 - 50) * 50000).max(10000); // Minimum 10k
        let facility_discount = youth_facilities as i64 * 10000; // Better facilities = cheaper
        let recruitment_cost = (base_cost - facility_discount).max(10000);
        
        // Recommendation expires in 30 days
        let expires_at = (game.clock.current_date + chrono::Duration::days(30))
            .format("%Y-%m-%d")
            .to_string();
        
        let recommendation = YouthRecommendation {
            id: format!("{}_{}_{}", current_month, i, Uuid::new_v4()),
            player,
            recruitment_cost,
            expires_at,
            facility_bonus: youth_facilities as i32,
        };
        
        game.youth_recommendations.push(recommendation);
    }
}

/// Generate a youth player with given age and potential
fn generate_youth_player(
    player_id: &str,
    age: u8,
    potential: u8,
    country: &str,
    rng: &mut impl Rng,
) -> Player {
    // Positions distributed across the squad
    let positions = [
        Position::Goalkeeper,
        Position::CenterBack,
        Position::CenterBack,
        Position::LeftBack,
        Position::RightBack,
        Position::DefensiveMidfielder,
        Position::CentralMidfielder,
        Position::CentralMidfielder,
        Position::AttackingMidfielder,
        Position::LeftWinger,
        Position::RightWinger,
        Position::Striker,
    ];
    let position = positions[rng.random_range(0..positions.len())].clone();
    
    // Generate name based on country
    let (first_name, last_name) = generate_name(country, rng);
    let full_name = format!("{} {}", first_name, last_name);
    let match_name = last_name.clone();
    
    // Calculate birth date
    let birth_year = 2026 - age as u32;
    let birth_month = rng.random_range(1..=12);
    let birth_day = rng.random_range(1..=28);
    let dob = format!("{:04}-{:02}-{:02}", birth_year, birth_month, birth_day);
    
    // Generate attributes based on potential
    let base_ovr = potential as f64 * rng.random_range(0.5..0.7);
    let current_ovr = base_ovr as u8;
    
    let attributes = generate_attributes(position.clone(), current_ovr, rng);
    
    // Create player
    let mut player = Player::new(
        player_id.to_string(),
        match_name,
        full_name,
        dob,
        country.to_string(),
        position,
        attributes,
    );
    
    player.condition = rng.random_range(85..=100);
    player.morale = rng.random_range(60..=85);
    
    // Market value for youth player
    let age_factor = 1.5; // Young players have higher value multiplier
    let base_value = (current_ovr as f64).powi(2) * 500.0;
    player.market_value = (base_value * age_factor) as u64;
    player.wage = (player.market_value / 200).max(1000) as u32;
    player.contract_end = Some(format!("{}-06-30", 2026 + rng.random_range(2..=5)));
    
    player
}

/// Generate player attributes based on position and overall rating
fn generate_attributes(
    position: Position,
    target_ovr: u8,
    rng: &mut impl Rng,
) -> PlayerAttributes {
    let is_gk = matches!(position, Position::Goalkeeper);
    let is_def = matches!(position, Position::Defender | Position::CenterBack | Position::LeftBack | Position::RightBack);
    let is_fwd = matches!(position, Position::Forward | Position::Striker | Position::LeftWinger | Position::RightWinger);
    
    // Generate base attributes with some randomness around target
    let variance = 10;
    
    let pace = if is_gk { rng.random_range(30..=50) } else { rng.random_range(target_ovr - variance..=target_ovr + variance).min(99) };
    let stamina = rng.random_range(target_ovr - variance..=target_ovr + variance).min(99);
    let strength = rng.random_range(target_ovr - variance..=target_ovr + variance).min(99);
    let agility = rng.random_range(target_ovr - variance..=target_ovr + variance).min(99);
    let passing = rng.random_range(target_ovr - variance..=target_ovr + variance).min(99);
    let shooting = if is_gk { rng.random_range(20..=50) } else { rng.random_range(target_ovr - variance..=target_ovr + variance).min(99) };
    let tackling = if is_gk || is_fwd { rng.random_range(20..=60) } else { rng.random_range(target_ovr - variance..=target_ovr + variance).min(99) };
    let dribbling = if is_gk { rng.random_range(20..=50) } else { rng.random_range(target_ovr - variance..=target_ovr + variance).min(99) };
    let defending = if is_gk { rng.random_range(25..=55) } else if is_def { rng.random_range(target_ovr - variance..=target_ovr + variance).min(99) } else { rng.random_range(30..=60) };
    let positioning = rng.random_range(target_ovr - variance..=target_ovr + variance).min(99);
    let vision = rng.random_range(target_ovr - variance..=target_ovr + variance).min(99);
    let decisions = rng.random_range(target_ovr - variance..=target_ovr + variance).min(99);
    let composure = rng.random_range(target_ovr - variance..=target_ovr + variance).min(99);
    let aggression = rng.random_range(30..=90);
    let teamwork = rng.random_range(45..=95);
    let leadership = rng.random_range(30..=90);
    let handling = if is_gk { rng.random_range(target_ovr - variance..=target_ovr + variance).min(99) } else { rng.random_range(10..=35) };
    let reflexes = if is_gk { rng.random_range(target_ovr - variance..=target_ovr + variance).min(99) } else { rng.random_range(20..=50) };
    let aerial = if is_gk { rng.random_range(target_ovr - variance..=target_ovr + variance).min(99) } else if is_def { rng.random_range(target_ovr - variance..=target_ovr + variance).min(99) } else { rng.random_range(30..=75) };
    
    PlayerAttributes {
        pace: pace.max(20),
        stamina: stamina.max(20),
        strength: strength.max(20),
        agility: agility.max(20),
        passing: passing.max(20),
        shooting: shooting.max(20),
        tackling: tackling.max(20),
        dribbling: dribbling.max(20),
        defending: defending.max(20),
        positioning: positioning.max(20),
        vision: vision.max(20),
        decisions: decisions.max(20),
        composure: composure.max(20),
        aggression: aggression.max(20),
        teamwork: teamwork.max(20),
        leadership: leadership.max(20),
        handling: handling.max(20),
        reflexes: reflexes.max(20),
        aerial: aerial.max(20),
    }
}

/// Generate a random name based on country
fn generate_name(country: &str, rng: &mut impl Rng) -> (String, String) {
    // Simple name pools for Chinese names
    let chinese_first = ["伟", "强", "磊", "浩", "杰", "鹏", "飞", "超", "龙", "凯", "文", "勇", "波", "峰", "华", "刚", "洋", "军", "涛", "明", "东", "雷", "宇", "晨", "辉", "松", "健", "斌", "威", "林", "海", "川", "博", "然", "程", "思", "雨", "泽", "睿", "鑫"];
    let chinese_last = ["王", "李", "张", "刘", "陈", "杨", "黄", "赵", "周", "吴", "徐", "孙", "马", "朱", "胡", "郭", "何", "高", "林", "罗", "郑", "梁", "谢", "宋", "唐", "许", "韩", "冯", "邓", "曹", "彭", "曾", "肖", "田", "董", "袁", "潘", "于", "蒋", "蔡", "余", "杜", "叶", "程", "苏", "魏", "吕", "丁", "任", "沈"];
    
    let first = chinese_first[rng.random_range(0..chinese_first.len())].to_string();
    let last = chinese_last[rng.random_range(0..chinese_last.len())].to_string();
    
    (first, last)
}

/// Recruit a youth player from recommendations
pub fn recruit_youth_player(game: &mut Game, recommendation_id: &str) -> Result<Player, String> {
    let user_team_id = game.manager.team_id.as_ref()
        .ok_or("No team selected")?;
    
    let team = game.teams.iter_mut()
        .find(|t| t.id == *user_team_id)
        .ok_or("Team not found")?;
    
    // Find and remove the recommendation
    let idx = game.youth_recommendations.iter()
        .position(|r| r.id == recommendation_id)
        .ok_or("Recommendation not found")?;
    
    let recommendation = game.youth_recommendations.remove(idx);
    
    // Check if we can afford it
    if team.finance < recommendation.recruitment_cost {
        return Err("Insufficient funds".to_string());
    }
    
    // Deduct recruitment cost
    team.finance -= recommendation.recruitment_cost;
    
    // Assign player to team
    let mut player = recommendation.player;
    player.team_id = Some(user_team_id.clone());
    
    // Add to game's player list
    game.players.push(player.clone());
    
    Ok(player)
}

/// Clean up expired recommendations
pub fn cleanup_expired_recommendations(game: &mut Game) {
    let today = game.clock.current_date.format("%Y-%m-%d").to_string();
    game.youth_recommendations.retain(|r| r.expires_at >= today);
}

/// Get current recommendations for display
pub fn get_current_recommendations(game: &Game) -> Vec<&YouthRecommendation> {
    let today = game.clock.current_date.format("%Y-%m-%d").to_string();
    game.youth_recommendations.iter()
        .filter(|r| r.expires_at >= today)
        .collect()
}
