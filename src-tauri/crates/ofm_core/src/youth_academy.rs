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
    let (_surname, full_name) = generate_name(country, rng);
    let match_name = full_name.clone();
    
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

/// Generate a random Chinese name (surname + given name)
/// Returns (surname, full_name) where full_name is surname + given_name
/// ~5% of names are from minority ethnicities
fn generate_chinese_name(rng: &mut impl Rng) -> (String, String) {
    // 5% chance for minority ethnic name
    if rng.random_range(0..20) == 0 {
        return generate_minority_name(rng);
    }
    
    // Common Chinese surnames (ordered by popularity)
    let surnames = [
        "王", "李", "张", "刘", "陈", "杨", "赵", "黄", "周", "吴",
        "徐", "孙", "马", "朱", "胡", "郭", "何", "高", "林", "罗",
        "郑", "梁", "谢", "宋", "唐", "许", "韩", "冯", "邓", "曹",
        "彭", "曾", "肖", "田", "董", "袁", "潘", "于", "蒋", "蔡",
    ];
    
    // Common single-character given names (male)
    let single_char_names = [
        "伟", "强", "磊", "浩", "杰", "鹏", "飞", "超", "龙", "凯",
        "文", "勇", "波", "峰", "华", "刚", "洋", "军", "涛", "明",
        "东", "雷", "宇", "晨", "辉", "松", "健", "斌", "威", "海",
        "川", "博", "然", "程", "思", "雨", "泽", "睿", "鑫", "帆",
        "俊", "锋", "亮", "康", "志", "豪", "瑞", "林", "森", "霖",
        "鹏", "飞", "骞", "腾", "超", "越", "钧", "鑫", "皓", "然",
    ];
    
    // Common two-character given names (male)
    let double_char_names = [
        "建国", "建军", "志强", "志伟", "志明", "志刚", "志华", "志勇",
        "建华", "建平", "建业", "文博", "文才", "文辉", "文涛",
        "明辉", "明华", "明远", "明星", "晓东", "晓峰", "晓光", "晓军",
        "海东", "海峰", "海明", "海超", "振华", "振宇", "振国", "振强",
        "俊杰", "俊峰", "俊豪", "俊才", "伟东", "伟明", "伟峰", "伟豪",
        "伟男", "伟民", "思远", "思雨", "思博", "思远",
        "浩然", "浩宇", "浩轩", "浩天", "浩明", "浩博",
        "子轩", "子墨", "子龙", "子豪", "子健", "子涵",
        "一鸣", "一凡", "一航", "一凡", "天宇", "天翔", "天赐", "天佑", "天成", "天瑞",
        "雨泽", "雨轩", "雨晨", "雨航", "宇航", "宇轩", "宇辰",
        "诗豪", "诗杰", "博涛", "博远", "博涵", "博轩",
        "冠宇", "冠豪", "冠杰", "冠霖", "冠东", "冠希",
        "文轩", "文豪", "文杰", "文博", "文涛", "文博",
        "睿渊", "睿诚", "睿智", "睿明", "睿轩", "睿材",
        "泽宇", "泽轩", "泽雨", "泽恩", "泽豪", "泽瑞",
        "俊杰", "俊贤", "俊豪", "俊逸", "俊健", "俊朗",
        "子涵", "子轩", "子墨", "子龙", "子睿", "子琪",
        "浩宇", "浩然", "浩轩", "浩天", "浩宇", "浩泽",
        "明轩", "明远", "明哲", "明志", "明锐", "明健",
        "梓轩", "梓豪", "梓涵", "梓杰", "梓霖", "梓博",
        "浩然", "浩宇", "浩轩", "浩天", "浩明", "浩然大",
        "天宇", "天翔", "天赐", "天佑", "天成", "天瑞",
        "旭尧", "旭东", "旭阳", "旭豪", "旭辉", "旭彬",
        "逸凡", "逸尘", "逸群", "逸才", "逸飞", "逸安",
        "晨逸", "晨轩", "晨浩", "晨宇", "晨飞", "晨阳",
        "承运", "承宇", "承轩", "承志", "承远", "承瑞",
        "德润", "德宇", "德轩", "德志", "德明", "德豪",
        "冠廷", "冠宇", "冠豪", "冠杰", "冠霖", "冠希",
        "嘉祥", "嘉懿", "嘉言", "嘉行", "嘉瑞", "嘉豪",
        "建安", "建白", "建业", "建成", "建华", "建元",
        "晋鹏", "晋浩", "晋宇", "晋轩", "晋明", "晋才",
        "经赋", "经国", "经纬", "经略", "经略", "经略",
        "景曜", "景福", "景龙", "景明", "景天", "景和",
        "乐生", "乐圣", "乐天", "乐成", "乐意", "乐康",
        "明轩", "明远", "明哲", "明志", "明锐", "明健",
        "天宇", "天翔", "天赐", "天佑", "天成", "天瑞",
    ];
    
    let surname = surnames[rng.random_range(0..surnames.len())].to_string();
    
    // 70% chance for single character name, 30% for double character
    let given_name = if rng.random_range(0..10) < 7 {
        // 70% single character
        single_char_names[rng.random_range(0..single_char_names.len())].to_string()
    } else {
        // 30% double character
        double_char_names[rng.random_range(0..double_char_names.len())].to_string()
    };
    
    let full_name = format!("{}{}", surname, given_name);
    
    (surname, full_name)
}

/// Generate a minority ethnic Chinese name (Mongol, Tibetan, Uyghur, Zhuang, Miao, Hui, etc.)
/// Returns (surname, full_name) - all names are male
fn generate_minority_name(rng: &mut impl Rng) -> (String, String) {
    // Minority surnames (common among ethnic minorities) - male surnames
    let minority_surnames = [
        // Mongolian
        "乌兰", "巴图", "哈斯", "巴雅尔", "苏日", "德勒", "巴根", "哈图",
        // Tibetan  
        "多吉", "扎西", "旦增", "索朗", "平措", "格桑", "达瓦", "次仁", "洛桑", "仁青",
        // Uyghur
        "阿不都", "艾力", "买买提", "吾布力", "艾孜", "穆合塔尔", "阿迪力", "艾克拜尔",
        // Zhuang
        "韦", "蒙", "陆", "农", "莫", "覃", "卢", "谭",
        // Miao
        "龙", "吴", "杨", "田", "石", "麻", "张", "王",
        // Hui
        "马", "苏", "丁", "黑", "摆", "闪", "保", "虎",
    ];
    
    // Minority given names (single character) - male only
    let minority_single = [
        // Mongolian
        "乌兰", "巴图", "巴雅尔", "苏日", "哈斯", "巴根", "哈图", "格尔",
        // Tibetan
        "多吉", "扎西", "旦增", "索朗", "平措", "格桑", "达瓦", "次仁", "洛桑", "仁青", "土登",
        // Uyghur
        "阿迪", "艾克", "阿力", "穆合", "吾斯", "艾山", "买买", "阿合",
        // Zhuang/Miao/Hui
        "金", "银", "华", "强", "勇", "军", "龙", "海", "刚", "明",
    ];
    
    // Minority given names (double character) - male only
    let minority_double = [
        // Mongolian
        "乌兰夫", "巴图尔", "巴雅尔", "苏日格", "哈斯尔", "巴根尔",
        // Tibetan
        "多吉杰", "扎西顿", "旦增罗", "索朗多", "平措杰", "格桑多", "洛桑杰",
        // Uyghur
        "阿迪力", "艾克拜尔", "穆合塔尔", "阿不都热合曼", "买买提明", "吾布力山",
        // Zhuang/Miao/Hui
        "韦国强", "蒙海龙", "陆志刚", "农文华", "莫军强", "覃勇军",
        "龙国强", "杨勇军", "田文华", "石海军", "麻志刚", "马国强",
    ];
    
    // 80% chance for surname-style (Han-style with minority surname), 20% for pure minority
    let is_surname_style = rng.random_range(0..10) < 8;
    
    if is_surname_style {
        // Use a minority surname with given name
        let surname = minority_surnames[rng.random_range(0..minority_surnames.len())].to_string();
        
        // 50% minority given name, 50% Han-style given name
        let given = if rng.random_range(0..2) == 0 {
            if rng.random_range(0..2) == 0 {
                minority_single[rng.random_range(0..minority_single.len())].to_string()
            } else {
                minority_double[rng.random_range(0..minority_double.len())].to_string()
            }
        } else {
            // Han-style male given name
            let han_given = [
                "伟", "强", "磊", "浩", "杰", "鹏", "飞", "超", "龙", "凯",
                "勇", "峰", "华", "刚", "军", "涛", "明", "东", "辉", "健",
                "俊杰", "俊峰", "伟明", "志华", "志强", "志刚", "志勇",
                "浩然", "浩宇", "子轩", "子豪", "一凡", "天宇", "天翔",
            ];
            han_given[rng.random_range(0..han_given.len())].to_string()
        };
        
        let full_name = format!("{}{}", surname, given);
        return (surname, full_name);
    } else {
        // Pure minority-style name (single-name culture)
        let given = if rng.random_range(0..2) == 0 {
            minority_single[rng.random_range(0..minority_single.len())].to_string()
        } else {
            minority_double[rng.random_range(0..minority_double.len())].to_string()
        };
        return (given.clone(), given);
    }
}

/// Generate a random name based on country
fn generate_name(country: &str, rng: &mut impl Rng) -> (String, String) {
    match country {
        "China" | "Chinese" => generate_chinese_name(rng),
        _ => {
            // Fallback: simple English names
            let first_names = ["James", "John", "Michael", "David", "Robert", "William", "Richard", "Joseph", "Thomas", "Charles"];
            let last_names = ["Smith", "Johnson", "Williams", "Brown", "Jones", "Garcia", "Miller", "Davis", "Rodriguez", "Martinez"];
            let first = first_names[rng.random_range(0..first_names.len())].to_string();
            let last = last_names[rng.random_range(0..last_names.len())].to_string();
            let full_name = format!("{} {}", first, last);
            (last, full_name)
        }
    }
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
