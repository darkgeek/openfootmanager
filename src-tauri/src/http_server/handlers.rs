//! HTTP handlers for web version
//! Direct implementations of game logic for HTTP API access

use axum::{extract::State, Json};
use chrono::Datelike;
use log::{debug, info};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::AppState;

use ofm_core::game::Game;
use ofm_core::clock::GameClock;
use ofm_core::season_context::refresh_game_context;
use ofm_core::turn::process_day_with_capture;
use ofm_core::generator::generate_world;
use ofm_core::generator::load_world_from_json;
use ofm_core::messages;
use ofm_core::news;
use ofm_core::player_events;
use ofm_core::schedule;
use ofm_core::end_of_season::{is_season_complete, process_end_of_season};
use ofm_core::firing::check_manager_firing;
use ofm_core::live_match_manager::{self, MatchMode};
use ofm_core::contracts::{
    propose_renewal as propose_renewal_service,
    RenewalOffer,
    RenewalDecision,
    DelegatedRenewalOptions,
    delegate_renewals as delegate_renewals_service,
    DelegatedRenewalReport,
};
use domain::negotiation::NegotiationFeedback;
use domain::player::RenewalSessionStatus;
use ofm_core::turn;

use domain::manager::Manager;
use domain::stats::StatsState;
use domain::league::FixtureStatus;

use chrono::{Duration, TimeZone};

// ===== Helper Functions =====

/// Convert snake_case string to camelCase
fn snake_to_camel(s: &str) -> String {
    let mut result = String::new();
    let mut capitalize_next = false;
    for c in s.chars() {
        if c == '_' {
            capitalize_next = true;
        } else if capitalize_next {
            result.push(c.to_ascii_uppercase());
            capitalize_next = false;
        } else {
            result.push(c);
        }
    }
    result
}

/// Recursively convert all keys in a JSON value from snake_case to camelCase
fn convert_keys_to_camel(value: &Value) -> Value {
    match value {
        Value::Object(map) => {
            let converted: serde_json::Map<String, Value> = map.iter()
                .map(|(k, v)| (snake_to_camel(k), convert_keys_to_camel(v)))
                .collect();
            Value::Object(converted)
        }
        Value::Array(arr) => {
            Value::Array(arr.iter().map(convert_keys_to_camel).collect())
        }
        _ => value.clone(),
    }
}

/// Balance team strengths for a fair challenge.
/// - User's team: reduced overall ratings (target bottom 3-5 in league)
/// - AI teams: boosted overall ratings to create competition
fn balance_team_strengths(game: &mut Game, user_team_id: &str) {
    use domain::player::Player;
    
    // Calculate current average team strength
    let mut team_strengths: Vec<(String, f64)> = game.teams.iter()
        .map(|t| {
            let team_players: Vec<&Player> = game.players.iter()
                .filter(|p| p.team_id.as_ref() == Some(&t.id))
                .collect();
            let avg_ovr = if team_players.is_empty() {
                50.0
            } else {
                team_players.iter()
                    .map(|p| calculate_player_overall(&p.attributes))
                    .sum::<f64>() / team_players.len() as f64
            };
            (t.id.clone(), avg_ovr)
        })
        .collect();
    
    // Sort by strength to find the weakest teams
    team_strengths.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap());
    
    // Target: user's team should be around 15th-17th (lower mid-table)
    // Weaker teams should have lower ratings
    let user_strength_idx = team_strengths.iter()
        .position(|(id, _)| id == user_team_id)
        .unwrap_or(0);
    
    // Adjust player attributes to balance
    for player in game.players.iter_mut() {
        if player.team_id.is_none() {
            continue;
        }
        
        let is_user_team = player.team_id.as_ref().map(|s| s.as_str()) == Some(&*user_team_id);
        let team_idx = team_strengths.iter()
            .position(|(id, _)| id == player.team_id.as_ref().unwrap());
        
        // Determine adjustment based on position in strength ranking
        let adjustment = if is_user_team {
            // User's team: reduce by 5-10 points
            -6
        } else {
            // AI teams: boost based on their position
            // Weaker teams get smaller boosts, stronger teams get bigger boosts
            let team_idx = team_idx.unwrap_or(0) as f64;
            let total_teams = team_strengths.len() as f64;
            // Teams ranked lower get more boost to catch up
            let boost = ((total_teams - team_idx) / total_teams * 8.0) as i8;
            boost.min(5).max(1)
        };
        
        if adjustment == 0 {
            continue;
        }
        
        // Apply adjustment to key attributes
        let attrs = &mut player.attributes;
        if adjustment > 0 {
            // Boost: distribute across 3-4 attributes
            let attrs_to_boost = if attrs.pace < 90 { &mut attrs.pace } else { &mut attrs.shooting };
            *attrs_to_boost = ((*attrs_to_boost as i8) + adjustment).clamp(1, 99) as u8;
            
            let attrs_to_boost2 = if attrs.passing < 90 { &mut attrs.passing } else { &mut attrs.dribbling };
            *attrs_to_boost2 = ((*attrs_to_boost2 as i8) + adjustment).clamp(1, 99) as u8;
            
            let attrs_to_boost3 = if attrs.stamina < 90 { &mut attrs.stamina } else { &mut attrs.strength };
            *attrs_to_boost3 = ((*attrs_to_boost3 as i8) + adjustment / 2).clamp(1, 99) as u8;
        } else {
            // Reduce: distribute across 3-4 attributes
            let attrs_to_reduce = if attrs.pace > 20 { &mut attrs.pace } else { &mut attrs.shooting };
            *attrs_to_reduce = ((*attrs_to_reduce as i8) + adjustment).clamp(1, 99) as u8;
            
            let attrs_to_reduce2 = if attrs.passing > 20 { &mut attrs.passing } else { &mut attrs.dribbling };
            *attrs_to_reduce2 = ((*attrs_to_reduce2 as i8) + adjustment).clamp(1, 99) as u8;
            
            let attrs_to_reduce3 = if attrs.stamina > 20 { &mut attrs.stamina } else { &mut attrs.strength };
            *attrs_to_reduce3 = ((*attrs_to_reduce3 as i8) + adjustment / 2).clamp(1, 99) as u8;
        }
        
        // Update market value based on new overall
        let new_overall = calculate_player_overall(&player.attributes);
        let age = estimate_player_age(&player.date_of_birth);
        let age_factor = if age <= 23 { 1.5 } else if age <= 28 { 1.2 } else if age <= 32 { 0.8 } else { 0.4 };
        player.market_value = ((new_overall as f64).powi(2) * 500.0 * age_factor) as u64;
        player.wage = (player.market_value / 200).max(500) as u32;
    }
    
    log::info!("[balance] Team strengths adjusted: user team weakened, AI teams boosted");
}

fn calculate_player_overall(attrs: &domain::player::PlayerAttributes) -> f64 {
    (attrs.pace as f64 + attrs.stamina as f64 + attrs.strength as f64
        + attrs.passing as f64 + attrs.shooting as f64 + attrs.tackling as f64
        + attrs.dribbling as f64 + attrs.defending as f64 + attrs.positioning as f64
        + attrs.vision as f64 + attrs.decisions as f64) / 11.0
}

fn estimate_player_age(dob: &str) -> u32 {
    let parts: Vec<&str> = dob.split('-').collect();
    if parts.is_empty() {
        return 25;
    }
    let birth_year: u32 = parts[0].parse().unwrap_or(2000);
    2026u32.saturating_sub(birth_year)
}

/// Serialize a value to JSON and convert keys to camelCase for frontend compatibility
fn to_camel_json<T: Serialize>(value: &T) -> Result<Value, String> {
    let json = serde_json::to_value(value).map_err(|e| e.to_string())?;
    Ok(convert_keys_to_camel(&json))
}

// ===== Helper Types =====

#[derive(Debug, Serialize)]
pub struct BlockerData {
    pub id: String,
    pub severity: String,
    pub text: String,
    pub tab: String,
}

// Params structs for HTTP handlers - using Value for flexibility with camelCase/snake_case
#[derive(Debug, Deserialize)]
pub struct SelectTeamParams {
    #[serde(alias = "teamId", alias = "team_id")]
    team_id: String,
}

#[derive(Debug, Deserialize)]
pub struct LoadGameParams {
    #[serde(alias = "saveId", alias = "save_id")]
    save_id: String,
}

#[derive(Debug, Deserialize)]
pub struct AdvanceTimeModeParams {
    #[serde(alias = "mode", default)]
    mode: Option<String>,
}

// ===== Game Management =====

pub async fn get_saves(
    State(state): State<AppState>,
) -> Result<Json<Vec<Value>>, String> {
    let sm = state.save_manager.lock().map_err(|e: std::sync::PoisonError<_>| e.to_string())?;
    let saves: Vec<Value> = sm
        .list_saves()
        .iter()
        .map(|s| {
            serde_json::json!({
                "id": s.id,
                "name": s.name,
                "manager_name": s.manager_name,
                "db_filename": s.db_filename,
                "checksum": s.checksum,
                "created_at": s.created_at,
                "last_played_at": s.last_played_at
            })
        })
        .collect();
    Ok(Json(saves))
}

pub async fn delete_save(
    State(state): State<AppState>,
    Json(params): Json<Value>,
) -> Result<Json<bool>, String> {
    let save_id: String = params.get("saveId")
        .and_then(|v| v.as_str())
        .ok_or("Missing saveId")?
        .to_string();
    
    let mut sm = state.save_manager.lock().map_err(|e: std::sync::PoisonError<_>| e.to_string())?;
    let result = sm.delete_save(&save_id).map_err(|e| e.to_string())?;
    Ok(Json(result))
}

pub async fn start_new_game(
    State(state): State<AppState>,
    Json(params): Json<Value>,
) -> Result<Json<Game>, String> {
    // Extract fields from JSON directly (flexible for camelCase or snake_case)
    let first_name = params.get("firstName")
        .or_else(|| params.get("first_name"))
        .and_then(|v| v.as_str())
        .ok_or("First name is required")?
        .trim()
        .to_string();
    let last_name = params.get("lastName")
        .or_else(|| params.get("last_name"))
        .and_then(|v| v.as_str())
        .ok_or("Last name is required")?
        .trim()
        .to_string();
    
    if first_name.is_empty() || last_name.is_empty() {
        return Err("First name and last name are required.".to_string());
    }
    if first_name.len() > 30 || last_name.len() > 30 {
        return Err("First name and last name must not exceed 30 characters.".to_string());
    }
    
    let dob = params.get("dob")
        .or_else(|| params.get("managerDob"))
        .and_then(|v| v.as_str())
        .ok_or("Date of birth is required")?
        .to_string();
    
    let nationality = params.get("nationality")
        .and_then(|v| v.as_str())
        .ok_or("Nationality is required")?
        .trim()
        .to_string();
    if nationality.is_empty() {
        return Err("Nationality is required.".to_string());
    }

    // Validate DOB
    let birth_date = chrono::NaiveDate::parse_from_str(&dob, "%Y-%m-%d")
        .map_err(|_| "Invalid date of birth. Use YYYY-MM-DD format.".to_string())?;
    let today = chrono::Utc::now().date_naive();
    let age = today.signed_duration_since(birth_date).num_days() / 365;
    if age < 30 {
        return Err("Manager must be at least 30 years old.".to_string());
    }
    if age > 99 {
        return Err("Invalid date of birth.".to_string());
    }

    let manager = Manager::new(
        "mgr_user".to_string(),
        first_name,
        last_name,
        dob.clone(),
        nationality,
    );

    let start_date = chrono::Utc.with_ymd_and_hms(2026, 7, 1, 0, 0, 0).unwrap();
    let clock = GameClock::new(start_date);

    let world_source = params.get("worldSource")
        .and_then(|v| v.as_str())
        .unwrap_or("random");
    
    // Check for inline JSON data first (for imported worlds)
    let inline_json = params.get("worldJson")
        .and_then(|v| v.as_str());
    
    log::info!("[start_new_game] world_source={}, inline_json present={}", 
        world_source, inline_json.is_some());
    
    // Use Option<League> for the imported world's league
    let mut world_league: Option<domain::league::League> = None;
    let (teams, players, staff) = if world_source == "random" && inline_json.is_none() {
        log::info!("[start_new_game] Using random world generation");
        generate_world(None)
    } else if let Some(json_str) = inline_json {
        // Use inline JSON data directly
        log::info!("[start_new_game] Loading inline JSON world, length={}", json_str.len());
        let world = load_world_from_json(json_str)?;
        log::info!("[start_new_game] Loaded world with {} teams, {} players, league={:?}", 
            world.teams.len(), world.players.len(), world.league.as_ref().map(|l| &l.name));
        world_league = world.league.clone();
        (world.teams, world.players, world.staff)
    } else {
        let path = world_source.strip_prefix("file:").unwrap_or(&world_source);
        log::info!("[start_new_game] Loading world from path: {}", path);
        let json = std::fs::read_to_string(path)
            .map_err(|e| format!("Failed to read world database: {}", e))?;
        let world = load_world_from_json(&json)?;
        world_league = world.league.clone();
        (world.teams, world.players, world.staff)
    };

    let mut new_game = Game::new(clock, manager, teams, players, staff, vec![]);
    if let Some(league) = world_league {
        new_game.league = Some(league);
    }
    // Randomize AI team training focuses for variety
    let _user_team_id = new_game.manager.team_id.clone().unwrap_or_default();
    // AI training focus randomisation (to be restored in Phase 2)
    state.state_manager.set_game(new_game.clone());
    state.state_manager.set_stats_state(StatsState::default());
    Ok(Json(new_game))
}

pub async fn list_world_databases(
    State(_state): State<AppState>,
) -> Result<Json<Vec<Value>>, String> {
    let databases = vec![
        serde_json::json!({
            "id": "random",
            "name": "Random World",
            "description": "Generate a random world with teams, players, and staff",
            "team_count": 8,
            "player_count": 160,
            "source": "builtin",
            "path": ""
        })
    ];
    Ok(Json(databases))
}

pub async fn export_world_database(
    State(_state): State<AppState>,
    Json(params): Json<Value>,
) -> Result<String, String> {
    let _world_id: String = params.get("worldId")
        .and_then(|v| v.as_str())
        .ok_or("Missing worldId")?
        .to_string();
    
    let world = generate_world(None);
    let json = serde_json::json!({
        "name": "Exported World",
        "description": "Exported world database",
        "teams": world.0,
        "players": world.1,
        "staff": world.2
    });
    serde_json::to_string_pretty(&json).map_err(|e| e.to_string())
}

pub async fn write_temp_database(
    State(_state): State<AppState>,
    Json(params): Json<Value>,
) -> Result<String, String> {
    let json = params.get("json")
        .and_then(|v| v.as_str())
        .ok_or("Missing json parameter")?;
    
    // Validate the JSON is valid WorldData
    let _: ofm_core::generator::WorldData = serde_json::from_str(json)
        .map_err(|e| format!("Invalid world database JSON: {}", e))?;
    
    // Write to a temp file
    let data_dir = dirs::data_local_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join("openfootmanager")
        .join("worlds");
    std::fs::create_dir_all(&data_dir).map_err(|e| e.to_string())?;
    
    // Generate unique filename using timestamp
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|e| e.to_string())?
        .as_nanos();
    let filename = format!("custom_{}.json", timestamp);
    let path = data_dir.join(&filename);
    
    std::fs::write(&path, json).map_err(|e| e.to_string())?;
    
    Ok(path.to_string_lossy().to_string())
}

pub async fn select_team(
    State(state): State<AppState>,
    Json(params): Json<SelectTeamParams>,
) -> Result<Json<Game>, String> {
    let team_id = params.team_id;
    
    let mut game = state.state_manager
        .get_game(|g: &Game| g.clone())
        .ok_or("No active game session".to_string())?;

    let team = game.teams.iter()
        .find(|t| t.id == team_id)
        .ok_or("Team not found".to_string())?
        .clone();
    let team_name = team.name.clone();

    game.manager.hire(team_id.clone());
    if let Some(t) = game.teams.iter_mut().find(|t| t.id == team_id) {
        t.manager_id = Some(game.manager.id.clone());
    }

    // Balance teams: weaken user's team and boost AI teams
    balance_team_strengths(&mut game, &team_id);

    let season_start = game.clock.current_date + Duration::days(30);
    let team_ids: Vec<String> = game.teams.iter().map(|t| t.id.clone()).collect();
    let mut league = schedule::generate_league("Premier Division", 2026, &team_ids, season_start);
    // Generate preseason friendlies INCLUDING user's team
    let friendlies = schedule::generate_preseason_friendlies(&team_ids, season_start, 3);
    schedule::append_fixtures(&mut league, friendlies);
    game.league = Some(league);
    refresh_game_context(&mut game);

    // Youth academy features (to be integrated with scouting system)
    // if game.clock.current_date.date_naive().day() == 1 {
    //     ofm_core::youth_academy::generate_monthly_recommendations(&mut game);
    // }
    // ofm_core::youth_academy::cleanup_expired_recommendations(&mut game);

    let date_str = game.clock.current_date.to_rfc3339();
    let welcome_msg = messages::welcome_message(&team_name, &team_id, &date_str);
    game.messages.push(welcome_msg);

    let season_msg = messages::season_schedule_message(
        "Premier Division",
        &season_start.format("%B %d, %Y").to_string(),
        &date_str,
    );
    game.messages.push(season_msg);

    let team_names: Vec<String> = game.teams.iter().map(|team| team.name.clone()).collect();
    game.news.push(news::season_preview_article(&team_names, &date_str));

    let staff_msg = messages::staff_advice_message(&team_name, &team_id, &date_str);
    game.messages.push(staff_msg);

    player_events::generate_contract_concern_messages(&mut game, false);

    // Apply game difficulty settings (board_firing_enabled)
    if state.settings_path.exists() {
        if let Ok(json) = std::fs::read_to_string(&state.settings_path) {
            if let Ok(settings) = serde_json::from_str::<crate::commands::settings::AppSettings>(&json) {
                game.board_firing_enabled = settings.board_firing_enabled;
            }
        }
    }

    let manager_name = format!("{} {}", game.manager.first_name, game.manager.last_name);
    let save_name = format!("{}'s Career", manager_name);

    let mut sm = state.save_manager.lock().map_err(|e: std::sync::PoisonError<_>| e.to_string())?;
    let save_id = sm.create_save(&game, &save_name)?;
    state.state_manager.set_save_id(save_id);

    state.state_manager.set_game(game.clone());
    state.state_manager.set_stats_state(StatsState::default());
    Ok(Json(game))
}

pub async fn load_game(
    State(state): State<AppState>,
    Json(params): Json<LoadGameParams>,
) -> Result<Json<String>, String> {
    let mut sm = state.save_manager.lock().map_err(|e: std::sync::PoisonError<_>| e.to_string())?;
    let mut game = sm.load_game(&params.save_id)
        .map_err(|e| e.to_string())?;
    let stats_state = sm.load_stats_state(&params.save_id)
        .map_err(|e| e.to_string())?;
    refresh_game_context(&mut game);

    // Generate monthly training report if it's the first day of a new month
    if game.clock.current_date.date_naive().day() == 1 {
        info!("[load_game] Generating monthly training report for day 1");
        ofm_core::training_report::generate_monthly_training_report(&mut game);
    }
    
    // Recalculate positions for all teams based on their formations.
    // This fixes positions for saved games created before the formation fix.
    recalculate_all_positions(&mut game);
    
    info!("[load_game] Game loaded, messages: {}",
          game.messages.len());

    let mgr_name = format!("{} {}", game.manager.first_name, game.manager.last_name);

    state.state_manager.set_save_id(params.save_id);
    state.state_manager.set_game(game.clone());
    state.state_manager.set_stats_state(stats_state);

    // Apply board_firing_enabled from settings.json so mid-game setting changes
    // (made in a previous run) are reflected immediately on load.
    if state.settings_path.exists() {
        if let Ok(json) = std::fs::read_to_string(&state.settings_path) {
            if let Ok(settings) = serde_json::from_str::<crate::commands::settings::AppSettings>(&json) {
                if let Some(mut g) = state.state_manager.get_game(|g| g.clone()) {
                    g.board_firing_enabled = settings.board_firing_enabled;
                    state.state_manager.set_game(g);
                }
            }
        }
    }

    // Bootstrap transfer market if it's nearly empty — this creates initial
    // market supply so the user can browse players immediately after loading.
    if let Some(mut g) = state.state_manager.get_game(|g| g.clone()) {
        ofm_core::ai_team_management::initialize_transfer_market(&mut g);
        ofm_core::ai_team_management::ai_replenish_squad(&mut g);
        state.state_manager.set_game(g);
    }

    Ok(Json(mgr_name))
}

pub async fn get_active_game(
    State(state): State<AppState>,
) -> Result<Json<Game>, String> {
    state.state_manager
        .get_game(|g| g.clone())
        .ok_or("No active game session".to_string())
        .map(Json)
}

pub async fn save_game(
    State(state): State<AppState>,
) -> Result<Json<()>, String> {
    let game = state.state_manager
        .get_game(|g| g.clone())
        .ok_or("No active game session".to_string())?;

    let save_id = state.state_manager
        .get_save_id()
        .ok_or("No active save session".to_string())?;

    let mut sm = state.save_manager.lock().map_err(|e: std::sync::PoisonError<_>| e.to_string())?;
    sm.save_game(&game, &save_id)?;
    let stats_state = state.state_manager
        .get_stats_state(|stats| stats.clone())
        .unwrap_or_default();
    sm.save_stats_state(&stats_state, &save_id)?;
    
    Ok(Json(()))
}

pub async fn exit_to_menu(
    State(state): State<AppState>,
) -> Result<Json<()>, String> {
    let game = state.state_manager
        .get_game(|g| g.clone())
        .ok_or("No active game session")?;

    if let Some(save_id) = state.state_manager.get_save_id() {
        let mut sm = state.save_manager.lock().map_err(|e: std::sync::PoisonError<_>| e.to_string())?;
        sm.save_game(&game, &save_id)?;
        let stats_state = state.state_manager
            .get_stats_state(|stats| stats.clone())
            .unwrap_or_default();
        sm.save_stats_state(&stats_state, &save_id)?;
    }

    state.state_manager.clear_game();
    state.state_manager.clear_save_id();
    
    Ok(Json(()))
}

// ===== Time Advancement =====

pub async fn advance_time(
    State(state): State<AppState>,
    Json(_params): Json<Value>,
) -> Result<Json<Value>, String> {
    // Auto-abandon any stale live match (user probably left the match page without finishing)
    // This resets the fixture to Scheduled so it will be simulated on next call
    if state.state_manager.with_live_match(|_| ()).is_some() {
        if let Err(e) = abandon_match_logic(&state).await {
            println!("Warning: Failed to abandon stale live match: {}", e);
        }
        // Continue with normal processing below
    }
    
    let mut game = state.state_manager
        .get_game(|g| g.clone())
        .ok_or("No active game session".to_string())?;

    // Normal day processing - simulate all matches (including user's)
    let mut captures = Vec::new();
    process_day_with_capture(&mut game, &mut |capture| {
        captures.push(capture);
    });
    for capture in captures {
        state.state_manager.append_stats_state(capture);
    }

    state.state_manager.set_game(game.clone());
    
    Ok(Json(serde_json::json!({
        "game": game
    })))
}

pub async fn advance_time_with_mode(
    State(state): State<AppState>,
    Json(params): Json<AdvanceTimeModeParams>,
) -> Result<Json<Value>, String> {
    let mode = params.mode.as_deref().unwrap_or("normal");
    
    // Auto-abandon any stale live match (user probably left the match page without finishing)
    if state.state_manager.with_live_match(|_| ()).is_some() {
        if let Err(e) = abandon_match_logic(&state).await {
            println!("Warning: Failed to abandon stale live match: {}", e);
        }
        // Continue with normal processing below
    }
    
    let mut game = state.state_manager
        .get_game(|g| g.clone())
        .ok_or("No active game session".to_string())?;
    
    let today = game.clock.current_date.format("%Y-%m-%d").to_string();
    
    // Check if it's a match day for the user's team
    let manager_team_id = game.manager.team_id.as_ref().cloned();
    let user_fixture_index = if let Some(ref tid) = manager_team_id {
        game.league.as_ref().and_then(|league| {
            league.fixtures.iter().enumerate().find(|(_, f)| {
                f.date == today 
                && f.status == FixtureStatus::Scheduled
                && (f.home_team_id == *tid || f.away_team_id == *tid)
            }).map(|(i, _)| i)
        })
    } else {
        None
    };
    
    match mode {
        "instant" => {
            // Simulate everything instantly including user's match
            let mut captures = Vec::new();
            process_day_with_capture(&mut game, &mut |capture| {
                captures.push(capture);
            });
            for capture in captures {
                state.state_manager.append_stats_state(capture);
            }
            
            // Auto-save after instant simulation (includes all match suspensions/results)
            if let Some(ref save_id) = state.state_manager.get_save_id() {
                let mut sm = state.save_manager.lock().map_err(|e: std::sync::PoisonError<_>| e.to_string())?;
                sm.save_game(&game, save_id).map_err(|e| format!("Failed to auto-save: {}", e))?;
                let stats_state = state.state_manager.get_stats_state(|s| s.clone()).unwrap_or_default();
                sm.save_stats_state(&stats_state, save_id).map_err(|e| format!("Failed to auto-save stats: {}", e))?;
            }
            state.state_manager.set_game(game.clone());
            
            // Return the game state with instant action
            return Ok(Json(serde_json::json!({
                "action": "advanced",
                "game": game
            })));
        },
        _ => {
            // Normal mode: skip user's match if it's a match day
            if let Some(fixture_idx) = user_fixture_index {
                turn::simulate_other_matches_with_capture(&mut game, &today, Some(fixture_idx), &mut |_| {});
                
                if let Some(league) = game.league.as_mut() {
                    if let Some(fixture) = league.fixtures.get_mut(fixture_idx) {
                        fixture.status = FixtureStatus::InProgress;
                    }
                }
                
                ofm_core::turn::generate_matchday_news(&mut game, &today);
                state.state_manager.set_game(game.clone());
                
                // Return live_match action with snapshot for navigation to match page
                let match_mode = match mode {
                    "spectator" => "spectator",
                    _ => "live",
                };
                
                // Create a live match session to get the initial snapshot
                let match_mode_enum = if mode == "spectator" {
                    MatchMode::Spectator
                } else {
                    MatchMode::Live
                };
                
                let session = live_match_manager::create_live_match(&game, fixture_idx, match_mode_enum, false)
                    .map_err(|e| e.to_string())?;
                let snapshot = session.snapshot();
                log::info!("[match] formation debug: home_team={:?}, home_formation={:?}, home_players={:?}",
                    snapshot.home_team.name, snapshot.home_team.formation,
                    snapshot.home_team.players.iter().map(|p| &p.position).collect::<Vec<_>>());
                state.state_manager.set_live_match(session);
                
                return Ok(Json(serde_json::json!({
                    "action": "live_match",
                    "fixture_index": fixture_idx,
                    "mode": match_mode,
                    "snapshot": serde_json::to_value(&snapshot).map_err(|e| e.to_string())?
                })));
            } else {
                let mut captures = Vec::new();
                process_day_with_capture(&mut game, &mut |capture| {
                    captures.push(capture);
                });
                for capture in captures {
                    state.state_manager.append_stats_state(capture);
                }
                state.state_manager.set_game(game.clone());
            }
        }
    }

    Ok(Json(serde_json::json!({
        "action": "advanced",
        "game": game
    })))
}

pub async fn skip_to_match_day(
    State(state): State<AppState>,
) -> Result<Json<Value>, String> {
    let mut game = state.state_manager
        .get_game(|g| g.clone())
        .ok_or("No active game session")?;

    let user_team_id = game.manager.team_id.clone().ok_or("No team assigned")?;
    let mut days_skipped = 0u32;
    
    loop {
        if days_skipped >= 60 {
            break;
        }

        let today = game.clock.current_date.format("%Y-%m-%d").to_string();
        let has_match = game.league.as_ref().is_some_and(|league| {
            league.fixtures.iter().any(|fixture| {
                fixture.date == today
                    && fixture.status == FixtureStatus::Scheduled
                    && (fixture.home_team_id == user_team_id || fixture.away_team_id == user_team_id)
            })
        });

        if has_match {
            break;
        }

        let mut captures = Vec::new();
        process_day_with_capture(&mut game, &mut |capture| {
            captures.push(capture);
        });
        for capture in captures {
            state.state_manager.append_stats_state(capture);
        }
        days_skipped += 1;

        if game.manager.team_id.is_none() {
            state.state_manager.set_game(game.clone());
            return Ok(Json(serde_json::json!({
                "action": "fired",
                "game": game,
                "days_skipped": days_skipped
            })));
        }

        // Simplified blocker check
        let blockers: Vec<Value> = Vec::new();
        if !blockers.is_empty() {
            state.state_manager.set_game(game.clone());
            return Ok(Json(serde_json::json!({
                "action": "blocked",
                "game": game,
                "blockers": blockers,
                "days_skipped": days_skipped
            })));
        }
    }

    state.state_manager.set_game(game.clone());
    Ok(Json(serde_json::json!({
        "action": "arrived",
        "game": game,
        "days_skipped": days_skipped
    })))
}

pub async fn check_blocking_actions(
    State(state): State<AppState>,
) -> Result<Json<Vec<BlockerData>>, String> {
    let _game = state.state_manager
        .get_game(|g| g.clone())
        .ok_or("No active game session")?;

    // Simplified - return empty blockers
    Ok(Json(vec![]))
}

// ===== Placeholder implementations for other handlers =====

pub async fn get_squad(State(state): State<AppState>, Json(_params): Json<Value>) -> Result<Json<Game>, String> {
    state.state_manager
        .get_game(|g| g.clone())
        .ok_or("No active game session".to_string())
        .map(Json)
}

pub async fn get_league(State(state): State<AppState>) -> Result<Json<Value>, String> {
    let game = state.state_manager
        .get_game(|g| g.clone())
        .ok_or("No active game session")?;
    Ok(Json(serde_json::json!({ "league": game.league })))
}

pub async fn upgrade_facility(State(state): State<AppState>, Json(_params): Json<Value>) -> Result<Json<Game>, String> {
    state.state_manager
        .get_game(|g| g.clone())
        .ok_or("No active game session".to_string())
        .map(Json)
}

#[derive(Debug, Clone, Serialize)]
pub struct RenewalCommandResponse {
    pub outcome: RenewalDecision,
    pub game: Game,
    pub suggested_wage: Option<u32>,
    pub suggested_years: Option<u32>,
    pub sessionStatus: RenewalSessionStatus,
    pub isTerminal: bool,
    pub cooledOff: bool,
    pub feedback: Option<NegotiationFeedback>,
}

#[derive(Debug, Clone, Serialize)]
pub struct DelegatedRenewalCommandResponse {
    pub game: Game,
    pub report: DelegatedRenewalReport,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RenewalParams {
    player_id: String,
    weekly_wage: u32,
    contract_years: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DelegatedRenewalParams {
    player_ids: Option<Vec<String>>,
    max_wage_increase_pct: u32,
    max_contract_years: u32,
}

pub async fn propose_renewal(State(state): State<AppState>, Json(params): Json<RenewalParams>) -> Result<Json<RenewalCommandResponse>, String> {
    info!("[http] propose_renewal: player_id={}, weekly_wage={}, contract_years={}",
          params.player_id, params.weekly_wage, params.contract_years);

    let mut game = state.state_manager
        .get_game(|g| g.clone())
        .ok_or("No active game session")?;


    let outcome = propose_renewal_service(
        &mut game,
        &params.player_id,
        RenewalOffer {
            weekly_wage: params.weekly_wage,
            contract_years: params.contract_years,
        },
    )?;

    state.state_manager.set_game(game.clone());


    Ok(Json(RenewalCommandResponse {
        outcome: outcome.decision,
        game,
        suggested_wage: outcome.suggested_wage,
        suggested_years: outcome.suggested_years,
        sessionStatus: outcome.session_status,
        isTerminal: outcome.is_terminal,
        cooledOff: outcome.cooled_off,
        feedback: outcome.feedback,
    }))
}

pub async fn delegate_renewals(State(state): State<AppState>, Json(params): Json<DelegatedRenewalParams>) -> Result<Json<DelegatedRenewalCommandResponse>, String> {
    info!("[http] delegate_renewals: player_ids={:?}, max_wage_increase_pct={}, max_contract_years={}",
          params.player_ids, params.max_wage_increase_pct, params.max_contract_years);

    let mut game = state.state_manager
        .get_game(|g| g.clone())
        .ok_or("No active game session")?;

    let report = delegate_renewals_service(
        &mut game,
        DelegatedRenewalOptions {
            player_ids: params.player_ids,
            max_wage_increase_pct: params.max_wage_increase_pct,
            max_contract_years: params.max_contract_years,
        },
    )?;

    state.state_manager.set_game(game.clone());

    Ok(Json(DelegatedRenewalCommandResponse {
        game,
        report,
    }))
}

pub async fn preview_renewal_financial_impact(State(_state): State<AppState>, Json(_params): Json<Value>) -> Result<Json<Value>, String> {
    // TODO: Implement financial projection
    Ok(Json(serde_json::json!({
        "weeklyWageImpact": 0,
        "annualWageImpact": 0,
        "budgetImpactPct": 0
    })))
}

pub async fn set_formation(State(state): State<AppState>, Json(params): Json<Value>) -> Result<Json<Game>, String> {
    let formation = params.get("formation")
        .and_then(|v| v.as_str())
        .ok_or("Missing formation")?
        .to_string();
    
    let mut game = state.state_manager
        .get_game(|g| g.clone())
        .ok_or("No active game session".to_string())?;
    
    // Parse formation into (def, mid, fwd) counts
    let parts: Vec<usize> = formation
        .split('-')
        .filter_map(|s| s.parse().ok())
        .collect();
    let (num_def, num_mid, num_fwd) = match parts.len() {
        3 => (parts[0], parts[1], parts[2]),
        4 => (parts[0], parts[1] + parts[2], parts[3]),
        _ => (4, 4, 2),
    };
    
    info!("[http] set_formation: {} -> def={}, mid={}, fwd={}", formation, num_def, num_mid, num_fwd);
    
    // Update manager's team formation and reassign player positions
    if let Some(ref team_id) = game.manager.team_id {
        if let Some(team) = game.teams.iter_mut().find(|t| t.id == *team_id) {
            team.formation = formation.clone();
        }
        
        // Reassign positions for outfield players
        let player_ids: Vec<String> = game
            .players
            .iter()
            .filter(|p| {
                p.team_id.as_deref() == Some(team_id)
                    && p.position != domain::player::Position::Goalkeeper
            })
            .map(|p| p.id.clone())
            .collect();
        
        // Sort by defensive ability (most defensive first)
        let mut sorted_ids = player_ids.clone();
        sorted_ids.sort_by(|a_id, b_id| {
            let pa = game.players.iter().find(|p| p.id == *a_id).unwrap();
            let pb = game.players.iter().find(|p| p.id == *b_id).unwrap();
            let def_a = pa.attributes.defending as u16
                + pa.attributes.tackling as u16
                + pa.attributes.strength as u16;
            let def_b = pb.attributes.defending as u16
                + pb.attributes.tackling as u16
                + pb.attributes.strength as u16;
            def_b.cmp(&def_a)
        });
        
        // Count positions before
        let mut before_counts = std::collections::HashMap::new();
        for p in game.players.iter().filter(|p| p.team_id.as_deref() == Some(team_id)) {
            *before_counts.entry(format!("{:?}", p.position)).or_insert(0) += 1;
        }
        info!("[http] set_formation: before positions: {:?}", before_counts);
        
        // Assign positions
        for (slot, pid) in sorted_ids.iter().enumerate() {
            let new_pos = if slot < num_def {
                domain::player::Position::Defender
            } else if slot < num_def + num_mid {
                domain::player::Position::Midfielder
            } else if slot < num_def + num_mid + num_fwd {
                domain::player::Position::Forward
            } else {
                continue;
            };
            if let Some(player) = game.players.iter_mut().find(|p| p.id == *pid) {
                info!("[http] set_formation: player {} {} -> {:?}", player.match_name, format!("{:?}", player.position), new_pos);
                player.position = new_pos;
            }
        }
        
        // Count positions after
        let mut after_counts = std::collections::HashMap::new();
        for p in game.players.iter().filter(|p| p.team_id.as_deref() == Some(team_id)) {
            *after_counts.entry(format!("{:?}", p.position)).or_insert(0) += 1;
        }
        info!("[http] set_formation: after positions: {:?}", after_counts);
    }
    
    state.state_manager.set_game(game.clone());
    Ok(Json(game))
}

/// Recalculate positions for ALL teams in the game based on their formations.
/// This fixes positions for saved games that were created before the formation fix was applied.
/// Call this after loading a game from the database.
pub fn recalculate_all_positions(game: &mut Game) {
    for team in game.teams.iter_mut() {
        let formation = &team.formation;
        let parts: Vec<usize> = formation
            .split('-')
            .filter_map(|s| s.parse().ok())
            .collect();
        let (num_def, num_mid, num_fwd) = match parts.len() {
            3 => (parts[0], parts[1], parts[2]),
            4 => (parts[0], parts[1] + parts[2], parts[3]),
            _ => continue,
        };
        
        // Get outfield players for this team
        let mut player_ids: Vec<String> = game
            .players
            .iter()
            .filter(|p| {
                p.team_id.as_deref() == Some(&team.id)
                    && p.position != domain::player::Position::Goalkeeper
            })
            .map(|p| p.id.clone())
            .collect();
        
        // Sort by defensive ability (most defensive first)
        player_ids.sort_by(|a_id, b_id| {
            let pa = game.players.iter().find(|p| p.id == *a_id).unwrap();
            let pb = game.players.iter().find(|p| p.id == *b_id).unwrap();
            let def_a = pa.attributes.defending as u16 + pa.attributes.tackling as u16 + pa.attributes.strength as u16;
            let def_b = pb.attributes.defending as u16 + pb.attributes.tackling as u16 + pb.attributes.strength as u16;
            def_b.cmp(&def_a)
        });
        
        // Assign positions
        for (slot, pid) in player_ids.iter().enumerate() {
            let new_pos = if slot < num_def {
                domain::player::Position::Defender
            } else if slot < num_def + num_mid {
                domain::player::Position::Midfielder
            } else if slot < num_def + num_mid + num_fwd {
                domain::player::Position::Forward
            } else {
                continue;
            };
            if let Some(player) = game.players.iter_mut().find(|p| p.id == *pid) {
                player.position = new_pos;
            }
        }
        
        // Log position counts
        let mut pos_counts = std::collections::HashMap::new();
        for p in game.players.iter().filter(|p| p.team_id.as_deref() == Some(&team.id)) {
            *pos_counts.entry(format!("{:?}", p.position)).or_insert(0) += 1;
        }
        info!("[recalculate_all_positions] {} ({}) -> {:?}", team.name, formation, pos_counts);
    }
}

/// Recalculate positions for all teams based on their formations
/// This fixes positions for saved games that were created before the formation fix
pub async fn recalculate_positions(State(state): State<AppState>) -> Result<Json<Game>, String> {
    let mut game = state.state_manager
        .get_game(|g| g.clone())
        .ok_or("No active game session".to_string())?;
    
    info!("[http] recalculate_positions: recalculating for {} teams", game.teams.len());
    recalculate_all_positions(&mut game);
    
    // Save to database so changes persist
    let save_id = state.state_manager
        .get_save_id()
        .ok_or("No active save session".to_string())?;
    {
        let mut sm = state.save_manager.lock().map_err(|e: std::sync::PoisonError<_>| e.to_string())?;
        sm.save_game(&game, &save_id)?;
        let stats_state = state.state_manager
            .get_stats_state(|stats| stats.clone())
            .unwrap_or_default();
        sm.save_stats_state(&stats_state, &save_id)?;
    }
    info!("[http] recalculate_positions: saved to database");
    
    state.state_manager.set_game(game.clone());
    Ok(Json(game))
}

pub async fn set_starting_xi(State(state): State<AppState>, Json(params): Json<Value>) -> Result<Json<Game>, String> {
    let player_ids = params.get("playerIds")
        .and_then(|v| v.as_array())
        .map(|arr| arr.iter().filter_map(|v| v.as_str()).map(String::from).collect())
        .ok_or("Missing or invalid playerIds")?;
    
    let mut game = state.state_manager
        .get_game(|g| g.clone())
        .ok_or("No active game session".to_string())?;
    
    // Update manager's team starting XI
    if let Some(ref team_id) = game.manager.team_id {
        if let Some(team) = game.teams.iter_mut().find(|t| t.id == *team_id) {
            team.starting_xi_ids = player_ids;
        }
    }
    
    state.state_manager.set_game(game.clone());
    Ok(Json(game))
}

pub async fn set_play_style(State(state): State<AppState>, Json(params): Json<Value>) -> Result<Json<Game>, String> {
    let play_style_str = params.get("playStyle")
        .or_else(|| params.get("play_style"))
        .and_then(|v| v.as_str())
        .ok_or("Missing playStyle")?
        .to_string();
    
    let play_style: domain::team::PlayStyle = match play_style_str.as_str() {
        "Balanced" => domain::team::PlayStyle::Balanced,
        "Attacking" => domain::team::PlayStyle::Attacking,
        "Defensive" => domain::team::PlayStyle::Defensive,
        "Possession" => domain::team::PlayStyle::Possession,
        "Counter" => domain::team::PlayStyle::Counter,
        "HighPress" => domain::team::PlayStyle::HighPress,
        _ => return Err("Invalid play style".to_string()),
    };
    
    let mut game = state.state_manager
        .get_game(|g| g.clone())
        .ok_or("No active game session".to_string())?;
    
    // Update manager's team play style
    if let Some(ref team_id) = game.manager.team_id {
        if let Some(team) = game.teams.iter_mut().find(|t| t.id == *team_id) {
            team.play_style = play_style;
        }
    }
    
    state.state_manager.set_game(game.clone());
    Ok(Json(game))
}

pub async fn set_team_match_roles(State(state): State<AppState>, Json(params): Json<Value>) -> Result<Json<Game>, String> {
    let match_roles_params = params.get("matchRoles")
        .and_then(|v| v.as_object())
        .ok_or("Missing or invalid matchRoles")?;
    
    let mut game = state.state_manager
        .get_game(|g| g.clone())
        .ok_or("No active game session".to_string())?;
    
    // Update manager's team match roles
    if let Some(ref team_id) = game.manager.team_id {
        if let Some(team) = game.teams.iter_mut().find(|t| t.id == *team_id) {
            if let Some(captain) = match_roles_params.get("captain").and_then(|v| v.as_str()) {
                team.match_roles.captain = Some(captain.to_string());
            }
            if let Some(vice_captain) = match_roles_params.get("vice_captain").and_then(|v| v.as_str()) {
                team.match_roles.vice_captain = Some(vice_captain.to_string());
            }
            if let Some(penalty_taker) = match_roles_params.get("penalty_taker").and_then(|v| v.as_str()) {
                team.match_roles.penalty_taker = Some(penalty_taker.to_string());
            }
            if let Some(free_kick_taker) = match_roles_params.get("free_kick_taker").and_then(|v| v.as_str()) {
                team.match_roles.free_kick_taker = Some(free_kick_taker.to_string());
            }
            if let Some(corner_taker) = match_roles_params.get("corner_taker").and_then(|v| v.as_str()) {
                team.match_roles.corner_taker = Some(corner_taker.to_string());
            }
        }
    }
    
    state.state_manager.set_game(game.clone());
    Ok(Json(game))
}

pub async fn set_training(State(state): State<AppState>, Json(params): Json<Value>) -> Result<Json<Game>, String> {
    let focus_str = params.get("focus")
        .and_then(|v| v.as_str())
        .ok_or("Missing focus")?;
    let intensity_str = params.get("intensity")
        .and_then(|v| v.as_str())
        .ok_or("Missing intensity")?;

    let focus: domain::team::TrainingFocus = match focus_str {
        "Physical" => domain::team::TrainingFocus::Physical,
        "Technical" => domain::team::TrainingFocus::Technical,
        "Tactical" => domain::team::TrainingFocus::Tactical,
        "Defending" => domain::team::TrainingFocus::Defending,
        "Attacking" => domain::team::TrainingFocus::Attacking,
        "Goalkeeping" => domain::team::TrainingFocus::Goalkeeping,
        "Recovery" => domain::team::TrainingFocus::Recovery,
        _ => return Err("Invalid training focus".to_string()),
    };

    let intensity: domain::team::TrainingIntensity = match intensity_str {
        "Low" => domain::team::TrainingIntensity::Low,
        "Medium" => domain::team::TrainingIntensity::Medium,
        "High" => domain::team::TrainingIntensity::High,
        _ => return Err("Invalid training intensity".to_string()),
    };

    let mut game = state.state_manager
        .get_game(|g| g.clone())
        .ok_or("No active game session".to_string())?;


    if let Some(ref team_id) = game.manager.team_id {
        if let Some(team) = game.teams.iter_mut().find(|t| t.id == *team_id) {
            team.training_focus = focus;
            team.training_intensity = intensity;
        }
    }

    state.state_manager.set_game(game.clone());
    Ok(Json(game))
}

pub async fn set_training_schedule(State(state): State<AppState>, Json(params): Json<Value>) -> Result<Json<Game>, String> {
    let schedule_str = params.get("schedule")
        .and_then(|v| v.as_str())
        .ok_or("Missing schedule")?;


    let schedule: domain::team::TrainingSchedule = match schedule_str {
        "Intense" => domain::team::TrainingSchedule::Intense,
        "Balanced" => domain::team::TrainingSchedule::Balanced,
        "Light" => domain::team::TrainingSchedule::Light,
        _ => return Err("Invalid training schedule".to_string()),
    };


    let mut game = state.state_manager
        .get_game(|g| g.clone())
        .ok_or("No active game session".to_string())?;

    if let Some(ref team_id) = game.manager.team_id {
        if let Some(team) = game.teams.iter_mut().find(|t| t.id == *team_id) {
            team.training_schedule = schedule;
        }
    }

    state.state_manager.set_game(game.clone());
    Ok(Json(game))
}

pub async fn set_training_groups(State(state): State<AppState>, Json(params): Json<Value>) -> Result<Json<Game>, String> {
    let groups_array = params.get("groups")
        .and_then(|v| v.as_array())
        .ok_or("Missing groups")?;

    let mut game = state.state_manager
        .get_game(|g| g.clone())
        .ok_or("No active game session".to_string())?;


    if let Some(ref team_id) = game.manager.team_id {
        if let Some(team) = game.teams.iter_mut().find(|t| t.id == *team_id) {
            let mut training_groups = Vec::new();
            for (i, g) in groups_array.iter().enumerate() {
                let id = g.get("id").and_then(|v| v.as_str()).unwrap_or(&format!("group_{}", i)).to_string();
                let name = g.get("name").and_then(|v| v.as_str()).unwrap_or("Group").to_string();
                let focus_str = g.get("focus").and_then(|v| v.as_str()).unwrap_or("Technical");
                let focus: domain::team::TrainingFocus = match focus_str {
                    "Physical" => domain::team::TrainingFocus::Physical,
                    "Technical" => domain::team::TrainingFocus::Technical,
                    "Tactical" => domain::team::TrainingFocus::Tactical,
                    "Defending" => domain::team::TrainingFocus::Defending,
                    "Attacking" => domain::team::TrainingFocus::Attacking,
                    "Goalkeeping" => domain::team::TrainingFocus::Goalkeeping,
                    "Recovery" => domain::team::TrainingFocus::Recovery,
                    _ => domain::team::TrainingFocus::Technical,
                };
                let player_ids: Vec<String> = g.get("player_ids")
                    .and_then(|v| v.as_array())
                    .map(|arr| arr.iter().filter_map(|v| v.as_str().map(String::from)).collect())
                    .unwrap_or_default();

                training_groups.push(domain::team::TrainingGroup {
                    id,
                    name,
                    focus,
                    player_ids,
                });
            }
            team.training_groups = training_groups;
        }
    }

    state.state_manager.set_game(game.clone());
    Ok(Json(game))
}


pub async fn set_player_training_focus(State(state): State<AppState>, Json(params): Json<Value>) -> Result<Json<Game>, String> {
    let player_id = params.get("playerId")
        .and_then(|v| v.as_str())
        .ok_or("Missing playerId")?;
    let focus_str = params.get("focus").and_then(|v| v.as_str());

    let focus: Option<domain::team::TrainingFocus> = focus_str.map(|s| match s {
        "Physical" => domain::team::TrainingFocus::Physical,
        "Technical" => domain::team::TrainingFocus::Technical,
        "Tactical" => domain::team::TrainingFocus::Tactical,
        "Defending" => domain::team::TrainingFocus::Defending,
        "Attacking" => domain::team::TrainingFocus::Attacking,
        "Recovery" => domain::team::TrainingFocus::Recovery,
        _ => domain::team::TrainingFocus::Technical,
    });

    let mut game = state.state_manager
        .get_game(|g| g.clone())
        .ok_or("No active game session".to_string())?;

    if let Some(player) = game.players.iter_mut().find(|p| p.id == player_id) {
        player.training_focus = focus;
    }

    state.state_manager.set_game(game.clone());
    Ok(Json(game))
}

pub async fn hire_staff(State(state): State<AppState>, Json(_params): Json<Value>) -> Result<Json<Game>, String> {
    state.state_manager
        .get_game(|g| g.clone())
        .ok_or("No active game session".to_string())
        .map(Json)
}

pub async fn release_staff(State(state): State<AppState>, Json(_params): Json<Value>) -> Result<Json<Game>, String> {
    state.state_manager
        .get_game(|g| g.clone())
        .ok_or("No active game session".to_string())
        .map(Json)
}

pub async fn mark_message_read(State(state): State<AppState>, Json(params): Json<Value>) -> Result<Json<Game>, String> {
    let mut game = state.state_manager
        .get_game(|g| g.clone())
        .ok_or("No active game session".to_string())?;
    
    if let Some(msg_id) = params.get("messageId").and_then(|v| v.as_str()) {
        if let Some(msg) = game.messages.iter_mut().find(|m| m.id == msg_id) {
            msg.read = true;
        }
        state.state_manager.set_game(game.clone());
    }
    
    Ok(Json(game))
}

pub async fn delete_message(State(state): State<AppState>, Json(params): Json<Value>) -> Result<Json<Game>, String> {
    let mut game = state.state_manager
        .get_game(|g| g.clone())
        .ok_or("No active game session".to_string())?;
    
    if let Some(msg_id) = params.get("messageId").and_then(|v| v.as_str()) {
        game.messages.retain(|m| m.id != msg_id);
        state.state_manager.set_game(game.clone());
    }
    
    Ok(Json(game))
}

pub async fn delete_messages(State(state): State<AppState>, Json(params): Json<Value>) -> Result<Json<Game>, String> {
    let mut game = state.state_manager
        .get_game(|g| g.clone())
        .ok_or("No active game session".to_string())?;
    
    if let Some(msg_ids) = params.get("messageIds").and_then(|v| v.as_array()) {
        let ids: Vec<String> = msg_ids.iter().filter_map(|v| v.as_str().map(String::from)).collect();
        game.messages.retain(|m| !ids.contains(&m.id));
        state.state_manager.set_game(game.clone());
    }
    
    Ok(Json(game))
}

pub async fn mark_all_messages_read(State(state): State<AppState>) -> Result<Json<Game>, String> {
    let mut game = state.state_manager
        .get_game(|g| g.clone())
        .ok_or("No active game session".to_string())?;
    
    for msg in game.messages.iter_mut() {
        msg.read = true;
    }
    state.state_manager.set_game(game.clone());
    
    Ok(Json(game))
}

pub async fn clear_old_messages(State(state): State<AppState>) -> Result<Json<Game>, String> {
    let mut game = state.state_manager
        .get_game(|g| g.clone())
        .ok_or("No active game session".to_string())?;
    
    let current_date = game.clock.current_date.format("%Y-%m-%d").to_string();
    game.messages.retain(|m| {
        if !m.read { return true; }
        if m.actions.iter().any(|a| !a.resolved) { return true; }
        if let Ok(msg_date) = chrono::NaiveDate::parse_from_str(&m.date, "%Y-%m-%d") {
            if let Ok(cur_date) = chrono::NaiveDate::parse_from_str(&current_date, "%Y-%m-%d") {
                return (cur_date - msg_date).num_days() <= 14;
            }
        }
        false
    });
    
    state.state_manager.set_game(game.clone());
    Ok(Json(game))
}

pub async fn resolve_message_action(State(state): State<AppState>, Json(params): Json<Value>) -> Result<Json<Value>, String> {
    let mut game = state.state_manager
        .get_game(|g| g.clone())
        .ok_or("No active game session".to_string())?;
    
    let message_id = params.get("messageId").and_then(|v| v.as_str()).unwrap_or("");
    let action_id = params.get("actionId").and_then(|v| v.as_str()).unwrap_or("");
    let option_id = params.get("optionId").and_then(|v| v.as_str());
    
    // Try youth recruitment response first
    let mut effect: Option<ofm_core::scouting::YouthRecruitmentEffect> = None;
    if !message_id.is_empty() && !action_id.is_empty() {
        if let Some(opt) = option_id {
            effect = ofm_core::scouting::apply_youth_recruitment_response(
                &mut game, message_id, action_id, opt,
            );
        }
    }
    
    // Fallback: mark action as resolved if not handled by youth recruitment
    if effect.is_none() {
        if let Some(msg) = game.messages.iter_mut().find(|m| m.id == message_id) {
            if let Some(action) = msg.actions.iter_mut().find(|a| a.id == action_id) {
                action.resolved = true;
            }
        }
    }
    
    state.state_manager.set_game(game.clone());
    
    if let Some(e) = effect {
        Ok(Json(serde_json::json!({
            "game": game,
            "effect": e.message,
            "effect_i18n_key": e.i18n_key,
            "effect_i18n_params": e.i18n_params
        })))
    } else {
        Ok(Json(serde_json::json!({
            "game": game,
            "effect": null,
            "effect_i18n_key": null,
            "effect_i18n_params": null
        })))
    }
}

pub async fn auto_select_set_pieces(State(state): State<AppState>, Json(_params): Json<Value>) -> Result<Json<Game>, String> {
    state.state_manager
        .get_game(|g| g.clone())
        .ok_or("No active game session".to_string())
        .map(Json)
}

pub async fn toggle_transfer_list(State(state): State<AppState>, Json(params): Json<Value>) -> Result<Json<Game>, String> {
    let player_id = params["playerId"]
        .as_str()
        .ok_or("Missing playerId")?
        .to_string();
    let mut game = state
        .state_manager
        .get_game(|g| g.clone())
        .ok_or("No active game session")?;
    
    if let Some(p) = game.players.iter_mut().find(|p| p.id == player_id) {
        p.transfer_listed = !p.transfer_listed;
    } else {
        return Err("Player not found".into());
    }
    state.state_manager.set_game(game.clone());
    Ok(Json(game))
}

pub async fn toggle_loan_list(State(state): State<AppState>, Json(params): Json<Value>) -> Result<Json<Game>, String> {
    let player_id = params["playerId"]
        .as_str()
        .ok_or("Missing playerId")?
        .to_string();
    let mut game = state
        .state_manager
        .get_game(|g| g.clone())
        .ok_or("No active game session")?;
    
    if let Some(p) = game.players.iter_mut().find(|p| p.id == player_id) {
        p.loan_listed = !p.loan_listed;
    } else {
        return Err("Player not found".into());
    }
    state.state_manager.set_game(game.clone());
    Ok(Json(game))
}

pub async fn make_transfer_bid(State(state): State<AppState>, Json(_params): Json<Value>) -> Result<Json<Value>, String> {
    let _game = state.state_manager
        .get_game(|g| g.clone())
        .ok_or("No active game session".to_string())?;
    Ok(Json(serde_json::json!({})))
}

pub async fn preview_transfer_bid_financial_impact(State(state): State<AppState>, Json(_params): Json<Value>) -> Result<Json<Value>, String> {
    let _game = state.state_manager
        .get_game(|g| g.clone())
        .ok_or("No active game session".to_string())?;
    Ok(Json(serde_json::json!({})))
}

pub async fn respond_to_offer(State(state): State<AppState>, Json(params): Json<Value>) -> Result<Json<Game>, String> {
    let player_id = params.get("playerId").and_then(|v| v.as_str()).ok_or("missing playerId")?;
    let offer_id = params.get("offerId").and_then(|v| v.as_str()).ok_or("missing offerId")?;
    let accept = params.get("accept").and_then(|v| v.as_bool()).unwrap_or(false);

    info!(
        "[http] respond_to_offer: player_id={}, offer_id={}, accept={}",
        player_id, offer_id, accept
    );

    let mut game = state.state_manager
        .get_game(|g| g.clone())
        .ok_or("No active game session".to_string())?;

    ofm_core::transfers::respond_to_offer(&mut game, player_id, offer_id, accept)?;
    state.state_manager.set_game(game.clone());

    Ok(Json(game))
}

pub async fn counter_offer(State(state): State<AppState>, Json(_params): Json<Value>) -> Result<Json<Value>, String> {
    let _game = state.state_manager
        .get_game(|g| g.clone())
        .ok_or("No active game session".to_string())?;
    Ok(Json(serde_json::json!({})))
}

pub async fn send_scout(State(state): State<AppState>, Json(params): Json<Value>) -> Result<Json<Game>, String> {
    let scout_id = params.get("scoutId")
        .or_else(|| params.get("scout_id"))
        .and_then(|v| v.as_str())
        .ok_or("Missing scoutId")?;
    let player_id = params.get("playerId")
        .or_else(|| params.get("player_id"))
        .and_then(|v| v.as_str())
        .ok_or("Missing playerId")?;
    
    let mut game = state.state_manager
        .get_game(|g| g.clone())
        .ok_or("No active game session".to_string())?;
    
    ofm_core::scouting::send_scout(&mut game, scout_id, player_id)?;
    
    state.state_manager.set_game(game.clone());
    Ok(Json(game))
}

pub async fn check_season_complete(State(state): State<AppState>) -> Result<Json<bool>, String> {
    let game = state.state_manager
        .get_game(|g| g.clone())
        .ok_or("No active game session".to_string())?;
    
    let complete = game.league.as_ref().is_some_and(|league| {
        league.fixtures.iter().all(|f| f.status != FixtureStatus::Scheduled)
    });
    
    Ok(Json(complete))
}

pub async fn advance_to_next_season(State(state): State<AppState>) -> Result<Json<Value>, String> {
    let mut game = state.state_manager
        .get_game(|g| g.clone())
        .ok_or("No active game session".to_string())?;

    if !is_season_complete(&game) {
        return Err("Season is not yet complete".to_string());
    }

    let summary = process_end_of_season(&mut game);

    // End-of-season objective evaluation may have dropped satisfaction — check firing
    check_manager_firing(&mut game);

    state.state_manager.set_game(game.clone());

    let response = serde_json::json!({
        "game": game,
        "summary": summary,
    });

    Ok(Json(response))
}

pub async fn get_season_awards(State(state): State<AppState>) -> Result<Json<Value>, String> {
    let game = state.state_manager
        .get_game(|g| g.clone())
        .ok_or("No active game session".to_string())?;
    
    let awards = ofm_core::season_awards::compute_season_awards(&game);
    
    Ok(Json(serde_json::to_value(&awards).map_err(|e| e.to_string())?))
}

pub async fn start_live_match(State(state): State<AppState>, Json(params): Json<Value>) -> Result<Json<Value>, String> {
    let game = state.state_manager
        .get_game(|g| g.clone())
        .ok_or("No active game session".to_string())?;
    
    let fixture_index = params.get("fixtureIndex")
        .and_then(|v| v.as_u64())
        .map(|v| v as usize)
        .ok_or("Missing fixtureIndex".to_string())?;
    let mode_str = params.get("mode")
        .and_then(|v| v.as_str())
        .unwrap_or("live");
    let allows_extra_time = params.get("allowsExtraTime")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    
    // Convert mode string to MatchMode
    let mode = match mode_str {
        "spectator" => MatchMode::Spectator,
        "instant" => MatchMode::Instant,
        _ => MatchMode::Live,
    };
    
    // Create the live match session using the engine
    let session = live_match_manager::create_live_match(&game, fixture_index, mode, allows_extra_time)
        .map_err(|e| e.to_string())?;
    
    let snapshot = session.snapshot();
    
    // Store the session in state
    state.state_manager.set_live_match(session);
    
    // Update fixture status to InProgress
    {
        let mut game = state.state_manager
            .get_game(|g| g.clone())
            .ok_or("No active game session".to_string())?;
        if let Some(league) = game.league.as_mut() {
            if let Some(fixture) = league.fixtures.get_mut(fixture_index) {
                fixture.status = FixtureStatus::InProgress;
            }
        }
        state.state_manager.set_game(game);
    }
    
    // Return the snapshot with snake_case keys (frontend MatchSnapshot type expects snake_case)
    Ok(Json(serde_json::to_value(&snapshot).map_err(|e| e.to_string())?))
}

pub async fn get_player_match_history(State(state): State<AppState>, Json(_params): Json<Value>) -> Result<Json<Vec<Value>>, String> {
    let _game = state.state_manager
        .get_game(|g| g.clone())
        .ok_or("No active game session".to_string())?;
    Ok(Json(vec![]))
}

pub async fn get_player_stats_overview(State(state): State<AppState>, Json(_params): Json<Value>) -> Result<Json<Value>, String> {
    let _game = state.state_manager
        .get_game(|g| g.clone())
        .ok_or("No active game session".to_string())?;
    
    // Return mock stats in the expected camelCase format
    // In a full implementation, this would query match history from the DB
    Ok(Json(serde_json::json!({
        "percentileEligible": false,
        "metrics": {
            "shots": { "total": 0, "per90": null, "percentile": null },
            "shotsOnTarget": { "total": 0, "per90": null, "percentile": null },
            "passes": { "completed": 0, "attempted": 0, "accuracy": null, "percentile": null },
            "tacklesWon": { "total": 0, "per90": null, "percentile": null },
            "interceptions": { "total": 0, "per90": null, "percentile": null },
            "foulsCommitted": { "total": 0, "per90": null, "percentile": null }
        }
    })))
}

pub async fn get_team_match_history(State(state): State<AppState>, Json(_params): Json<Value>) -> Result<Json<Vec<Value>>, String> {
    let _game = state.state_manager
        .get_game(|g| g.clone())
        .ok_or("No active game session".to_string())?;
    Ok(Json(vec![]))
}

pub async fn get_team_stats_overview(State(state): State<AppState>, Json(_params): Json<Value>) -> Result<Json<Value>, String> {
    let _game = state.state_manager
        .get_game(|g| g.clone())
        .ok_or("No active game session".to_string())?;
    
    // Return mock stats in the expected camelCase format
    // In a full implementation, this would query match history from the DB
    Ok(Json(serde_json::json!({
        "matchesPlayed": 0,
        "goalsFor": 0,
        "goalsAgainst": 0,
        "goalDifference": 0,
        "possessionAverage": null,
        "metrics": {
            "shots": { "total": 0, "perMatch": null },
            "shotsOnTarget": { "total": 0, "perMatch": null },
            "passes": { "completed": 0, "attempted": 0, "accuracy": null },
            "tacklesWon": { "total": 0, "perMatch": null },
            "interceptions": { "total": 0, "perMatch": null },
            "foulsCommitted": { "total": 0, "perMatch": null }
        }
    })))
}

pub async fn step_live_match(State(state): State<AppState>, Json(params): Json<Value>) -> Result<Json<Value>, String> {
    let minutes = params.get("minutes")
        .and_then(|v| v.as_u64())
        .map(|v| v as u16)
        .unwrap_or(1);
    
    let results = state.state_manager
        .with_live_match(|session| {
            if minutes <= 1 {
                vec![session.step()]
            } else {
                session.step_many(minutes)
            }
        })
        .ok_or_else(|| "No active live match".to_string())?;
    
    // Return the results with snake_case keys for frontend compatibility
    Ok(Json(serde_json::to_value(&results).map_err(|e| e.to_string())?))
}

pub async fn apply_match_command(State(state): State<AppState>, Json(params): Json<Value>) -> Result<Json<Value>, String> {
    // Parse the command from params
    // Support two formats:
    // 1. { type: "ChangeFormation", side: "Home", formation: "4-3-3" }
    // 2. { command: { ChangeFormation: { side: "Home", formation: "4-3-3" } } }
    
    let command: engine::MatchCommand = if let Some(cmd_obj) = params.get("command").and_then(|v| v.as_object()) {
        // Format 2: { command: { ChangeFormation: {...} } }
        let (cmd_name, cmd_data) = cmd_obj.iter().next()
            .ok_or("Invalid command format")?;
        let data = cmd_data.as_object().ok_or("Invalid command data")?;
        let side_str = data.get("side").and_then(|v| v.as_str()).unwrap_or("Home");
        let side = if side_str == "Home" { engine::Side::Home } else { engine::Side::Away };
        
        match cmd_name.as_str() {
            "Substitution" | "Substitute" => {
                let player_on_id = data.get("playerInId")
                    .or_else(|| data.get("player_on_id"))
                    .and_then(|v| v.as_str())
                    .unwrap_or_default()
                    .to_string();
                let player_off_id = data.get("playerOutId")
                    .or_else(|| data.get("player_off_id"))
                    .and_then(|v| v.as_str())
                    .unwrap_or_default()
                    .to_string();
                engine::MatchCommand::Substitute { side, player_off_id, player_on_id }
            },
            "SetTactic" | "ChangeFormation" => {
                let formation = data.get("formation")
                    .or_else(|| data.get("tactic"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("4-4-2")
                    .to_string();
                engine::MatchCommand::ChangeFormation { side, formation }
            },
            "ChangePlayStyle" | "SetPlayStyle" => {
                let play_style_str = data.get("playStyle")
                    .or_else(|| data.get("play_style"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("Balanced");
                let play_style = match play_style_str {
                    "Attacking" => engine::PlayStyle::Attacking,
                    "Defensive" => engine::PlayStyle::Defensive,
                    "Possession" => engine::PlayStyle::Possession,
                    "Counter" => engine::PlayStyle::Counter,
                    "HighPress" => engine::PlayStyle::HighPress,
                    _ => engine::PlayStyle::Balanced,
                };
                engine::MatchCommand::ChangePlayStyle { side, play_style }
            },
            "SetCaptain" => {
                let player_id = data.get("playerId")
                    .or_else(|| data.get("player_id"))
                    .and_then(|v| v.as_str())
                    .unwrap_or_default()
                    .to_string();
                engine::MatchCommand::SetCaptain { side, player_id }
            },
            "SetPenaltyTaker" => {
                let player_id = data.get("playerId")
                    .or_else(|| data.get("player_id"))
                    .and_then(|v| v.as_str())
                    .unwrap_or_default()
                    .to_string();
                engine::MatchCommand::SetPenaltyTaker { side, player_id }
            },
            "SetFreeKickTaker" => {
                let player_id = data.get("playerId")
                    .or_else(|| data.get("player_id"))
                    .and_then(|v| v.as_str())
                    .unwrap_or_default()
                    .to_string();
                engine::MatchCommand::SetFreeKickTaker { side, player_id }
            },
            "SetCornerTaker" => {
                let player_id = data.get("playerId")
                    .or_else(|| data.get("player_id"))
                    .and_then(|v| v.as_str())
                    .unwrap_or_default()
                    .to_string();
                engine::MatchCommand::SetCornerTaker { side, player_id }
            },
            _ => return Err(format!("Unknown command type: {}", cmd_name)),
        }
    } else {
        // Format 1: { type: "ChangeFormation", side: "Home", formation: "4-3-3" }
        let command_type = params.get("type")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        let side_str = params.get("side")
            .and_then(|v| v.as_str())
            .unwrap_or("Home");
        let side = if side_str == "Home" { engine::Side::Home } else { engine::Side::Away };
        
        match command_type {
            "Substitution" | "Substitute" => {
                let player_on_id = params.get("playerInId")
                    .or_else(|| params.get("player_on_id"))
                    .and_then(|v| v.as_str())
                    .unwrap_or_default()
                    .to_string();
                let player_off_id = params.get("playerOutId")
                    .or_else(|| params.get("player_off_id"))
                    .and_then(|v| v.as_str())
                    .unwrap_or_default()
                    .to_string();
                engine::MatchCommand::Substitute { side, player_off_id, player_on_id }
            },
            "SetTactic" | "ChangeFormation" => {
                let formation = params.get("formation")
                    .or_else(|| params.get("tactic"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("4-4-2")
                    .to_string();
                engine::MatchCommand::ChangeFormation { side, formation }
            },
            "ChangePlayStyle" | "SetPlayStyle" => {
                let play_style_str = params.get("playStyle")
                    .or_else(|| params.get("play_style"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("Balanced");
                let play_style = match play_style_str {
                    "Attacking" => engine::PlayStyle::Attacking,
                    "Defensive" => engine::PlayStyle::Defensive,
                    "Possession" => engine::PlayStyle::Possession,
                    "Counter" => engine::PlayStyle::Counter,
                    "HighPress" => engine::PlayStyle::HighPress,
                    _ => engine::PlayStyle::Balanced,
                };
                engine::MatchCommand::ChangePlayStyle { side, play_style }
            },
            "SetCaptain" => {
                let player_id = params.get("playerId")
                    .or_else(|| params.get("player_id"))
                    .and_then(|v| v.as_str())
                    .unwrap_or_default()
                    .to_string();
                engine::MatchCommand::SetCaptain { side, player_id }
            },
            "SetPenaltyTaker" => {
                let player_id = params.get("playerId")
                    .or_else(|| params.get("player_id"))
                    .and_then(|v| v.as_str())
                    .unwrap_or_default()
                    .to_string();
                engine::MatchCommand::SetPenaltyTaker { side, player_id }
            },
            "SetFreeKickTaker" => {
                let player_id = params.get("playerId")
                    .or_else(|| params.get("player_id"))
                    .and_then(|v| v.as_str())
                    .unwrap_or_default()
                    .to_string();
                engine::MatchCommand::SetFreeKickTaker { side, player_id }
            },
            "SetCornerTaker" => {
                let player_id = params.get("playerId")
                    .or_else(|| params.get("player_id"))
                    .and_then(|v| v.as_str())
                    .unwrap_or_default()
                    .to_string();
                engine::MatchCommand::SetCornerTaker { side, player_id }
            },
            _ => return Err(format!("Unknown command type: {}", command_type)),
        }
    };
    
    // Apply the command to the live match
    let result = state.state_manager
        .with_live_match(|session| {
            session.apply_command(command)?;
            Ok::<engine::MatchSnapshot, String>(session.snapshot())
        })
        .ok_or_else(|| "No active live match".to_string())?;
    
    let snapshot = result.map_err(|e| e.to_string())?;
    
    // Return the snapshot with snake_case keys for frontend compatibility
    Ok(Json(serde_json::to_value(&snapshot).map_err(|e| e.to_string())?))
}

pub async fn get_match_snapshot(State(state): State<AppState>) -> Result<Json<Value>, String> {
    // First, check if there's an active live match
    if let Some(snapshot) = state.state_manager.with_live_match(|session| session.snapshot()) {
        // Return the snapshot with snake_case keys for frontend compatibility
        return Ok(Json(serde_json::to_value(&snapshot).map_err(|e| e.to_string())?));
    }
    
    // No active live match - return the last completed match info
    let game = state.state_manager
        .get_game(|g| g.clone())
        .ok_or("No active game session".to_string())?;
    
    // Find the user's team
    let manager_team_id = match game.manager.team_id.as_ref() {
        Some(id) => id,
        None => return Err("No team assigned".to_string()),
    };
    
    // Default values
    let mut home_team_id = manager_team_id.clone();
    let mut away_team_id = manager_team_id.clone();
    let mut home_score: u8 = 0;
    let mut away_score: u8 = 0;
    let mut phase = "prematch".to_string();
    let mut current_minute: u8 = 0;
    
    // Find the most recent completed match involving user's team
    if let Some(league) = game.league.as_ref() {
        if let Some(fixture) = league.fixtures.iter().rev().find(|f| {
            (f.home_team_id == *manager_team_id || f.away_team_id == *manager_team_id)
            && f.status == domain::league::FixtureStatus::Completed
        }) {
            home_team_id = fixture.home_team_id.clone();
            away_team_id = fixture.away_team_id.clone();
            home_score = fixture.result.as_ref()
                .map(|r| r.home_goals)
                .unwrap_or(0);
            away_score = fixture.result.as_ref()
                .map(|r| r.away_goals)
                .unwrap_or(0);
            phase = "Finished".to_string();
            current_minute = 90;
        } else {
            // If no completed match, look for today's scheduled match
            let today = game.clock.current_date.format("%Y-%m-%d").to_string();
            if let Some(fixture) = league.fixtures.iter().find(|f| {
                f.date == today && f.status == domain::league::FixtureStatus::Scheduled
            }) {
                home_team_id = fixture.home_team_id.clone();
                away_team_id = fixture.away_team_id.clone();
            }
        }
    };
    
    // Get team names
    let home_team = game.teams.iter().find(|t| t.id == home_team_id);
    let away_team = game.teams.iter().find(|t| t.id == away_team_id);
    
    Ok(Json(serde_json::json!({
        "phase": phase,
        "current_minute": current_minute,
        "home_score": home_score,
        "away_score": away_score,
        "possession": "Home",
        "ball_zone": "defensive_third",
        "home_team": { 
            "id": home_team_id, 
            "name": home_team.map(|t| t.name.clone()).unwrap_or_else(|| "Home".to_string()), 
            "players": [] 
        },
        "away_team": { 
            "id": away_team_id, 
            "name": away_team.map(|t| t.name.clone()).unwrap_or_else(|| "Away".to_string()), 
            "players": [] 
        },
        "home_bench": [],
        "away_bench": [],
        "home_possession_pct": 50,
        "away_possession_pct": 50,
        "events": [],
        "home_subs_made": 0,
        "away_subs_made": 0,
        "max_subs": 5,
        "home_set_pieces": { "corners": null, "freeKicks": null, "penalties": null },
        "away_set_pieces": { "corners": null, "freeKicks": null, "penalties": null },
        "substitutions": [],
        "allows_extra_time": false,
        "home_yellows": {},
        "away_yellows": {},
        "sent_off": []
    })))
}

pub async fn finish_live_match(State(state): State<AppState>) -> Result<Json<Value>, String> {
    // Take the live match session
    let session = state.state_manager
        .take_live_match()
        .ok_or("No active live match".to_string())?;
    
    let fixture_index = session.fixture_index;
    let home_team_id = session.home_team_id.clone();
    let away_team_id = session.away_team_id.clone();
    let round_matchday = session.round_matchday;
    let _round_previous_standings = session.round_previous_standings.clone();
    
    // Get the match report
    let report = session.match_state.into_report();
    
    // Get and update the game
    let mut game = state.state_manager
        .get_game(|g| g.clone())
        .ok_or("No active game session".to_string())?;
    
    // Apply the match report to the game
    let mut captures = Vec::new();
    turn::apply_match_report_with_capture(
        &mut game,
        fixture_index,
        &home_team_id,
        &away_team_id,
        &report,
        &mut |capture| captures.push(capture),
    );
    for capture in captures {
        state.state_manager.append_stats_state(capture);
    }
    
    // Finish the live match day (advance time, generate news, etc.)
    turn::finish_live_match_day(&mut game);
    
    // Save the game to the database so suspensions/results persist across server restarts
    if let Some(ref save_id) = state.state_manager.get_save_id() {
        let mut sm = state.save_manager.lock().map_err(|e: std::sync::PoisonError<_>| e.to_string())?;
        sm.save_game(&game, save_id).map_err(|e| format!("Failed to auto-save: {}", e))?;
        let stats_state = state.state_manager.get_stats_state(|s| s.clone()).unwrap_or_default();
        sm.save_stats_state(&stats_state, save_id).map_err(|e| format!("Failed to auto-save stats: {}", e))?;
        debug!("[finish_live_match] auto-saved game to {}", save_id);
    }
    
    // Update in-memory state
    state.state_manager.set_game(game.clone());
    
    // Build round summary
    let standings = game.league.as_ref()
        .map(|l| l.standings.clone())
        .unwrap_or_default();
    
    let completed_results: Vec<Value> = game.league.as_ref()
        .map(|l| l.fixtures.iter()
            .filter(|f| f.status == FixtureStatus::Completed && f.matchday == round_matchday)
            .map(|f| {
                let home_name = game.teams.iter()
                    .find(|t| t.id == f.home_team_id)
                    .map(|t| t.name.clone())
                    .unwrap_or_default();
                let away_name = game.teams.iter()
                    .find(|t| t.id == f.away_team_id)
                    .map(|t| t.name.clone())
                    .unwrap_or_default();
                serde_json::json!({
                    "homeTeam": { "name": home_name },
                    "awayTeam": { "name": away_name },
                    "homeGoals": f.result.as_ref().map(|r| r.home_goals).unwrap_or(0),
                    "awayGoals": f.result.as_ref().map(|r| r.away_goals).unwrap_or(0)
                })
            })
            .collect())
        .unwrap_or_default();
    
    Ok(Json(serde_json::json!({
        "game": game,
        "roundSummary": {
            "matchday": round_matchday,
            "isComplete": true,
            "pendingFixtureCount": 0,
            "completedResults": completed_results,
            "standings": standings
        }
    })))
}

/// Abandon the current live match (e.g., when user leaves the match page without finishing).
/// This cancels the match and allows time to advance again.
pub async fn abandon_match(State(state): State<AppState>) -> Result<Json<Value>, String> {
    abandon_match_logic(&state).await?;
    
    let game = state.state_manager
        .get_game(|g| g.clone())
        .ok_or("No active game session".to_string())?;
    
    Ok(Json(serde_json::json!({
        "success": true,
        "message": "Match abandoned",
        "game": game
    })))
}

/// Internal helper for abandoning a live match.
async fn abandon_match_logic(state: &AppState) -> Result<(), String> {
    // Take and discard the live match session
    let session = state.state_manager
        .take_live_match()
        .ok_or("No active live match to abandon".to_string())?;
    
    let fixture_index = session.fixture_index;
    
    // Get the game and revert the fixture status
    let mut game = state.state_manager
        .get_game(|g| g.clone())
        .ok_or("No active game session".to_string())?;
    
    // Revert the fixture status back to Scheduled so it can be simulated normally
    if let Some(league) = game.league.as_mut() {
        if let Some(fixture) = league.fixtures.get_mut(fixture_index) {
            fixture.status = FixtureStatus::Scheduled;
            fixture.result = None;
        }
    }
    
    // Don't advance the clock - the fixture will be simulated on the next advance_time call
    
    state.state_manager.set_game(game);
    
    Ok(())
}

pub async fn apply_team_talk(State(state): State<AppState>, Json(_params): Json<Value>) -> Result<Json<Value>, String> {
    let game = state.state_manager
        .get_game(|g| g.clone())
        .ok_or("No active game session".to_string())?;
    Ok(Json(serde_json::json!({ "game": game })))
}

pub async fn submit_press_conference(State(state): State<AppState>, Json(_params): Json<Value>) -> Result<Json<Value>, String> {
    let game = state.state_manager
        .get_game(|g| g.clone())
        .ok_or("No active game session".to_string())?;
    Ok(Json(serde_json::json!({ "game": game })))
}

pub async fn get_settings(State(state): State<AppState>) -> Result<Json<Value>, String> {
    use crate::commands::settings::AppSettings;
    if !state.settings_path.exists() {
        return Ok(Json(serde_json::to_value(&AppSettings::default()).unwrap()));
    }
    let json = std::fs::read_to_string(&state.settings_path).map_err(|e| e.to_string())?;
    let settings: AppSettings = serde_json::from_str(&json).map_err(|e| format!("Failed to parse settings: {}", e))?;
    serde_json::to_value(&settings).map(Json).map_err(|e| e.to_string())
}

pub async fn save_settings(State(state): State<AppState>, Json(params): Json<Value>) -> Result<Json<()>, String> {
    use crate::commands::settings::AppSettings;
    // Frontend sends { settings: AppSettings } — try unwrapping first, then fall back to direct.
    let settings: AppSettings = if let Some(settings_obj) = params.get("settings") {
        serde_json::from_value(settings_obj.clone())
            .map_err(|e| format!("Failed to parse settings object: {}", e))?
    } else {
        serde_json::from_value(params.clone())
            .map_err(|e| format!("Failed to parse settings: {}", e))?
    };
    let json = serde_json::to_string_pretty(&settings).map_err(|e| e.to_string())?;
    std::fs::write(&state.settings_path, json).map_err(|e| format!("Failed to save settings: {}", e))?;

    // If a game is currently active, apply the new board_firing_enabled to it
    // and save the updated game so the change persists.
    if let Some(mut game) = state.state_manager.get_game(|g| g.clone()) {
        game.board_firing_enabled = settings.board_firing_enabled;
        state.state_manager.set_game(game.clone());
        if let Some(save_id) = state.state_manager.get_save_id() {
            if let Ok(mut sm) = state.save_manager.lock() {
                let _ = sm.save_game(&game, &save_id);
            }
        }
    }

    Ok(Json(()))
}

pub async fn clear_all_saves(State(_state): State<AppState>) -> Result<Json<()>, String> {
    // Simplified: just return success
    Ok(Json(()))
}

pub async fn get_available_jobs(State(state): State<AppState>) -> Result<Json<Vec<Value>>, String> {
    let _game = state.state_manager
        .get_game(|g| g.clone())
        .ok_or("No active game session".to_string())?;
    Ok(Json(vec![]))
}

pub async fn apply_for_job(State(_state): State<AppState>, Json(_params): Json<Value>) -> Result<Json<Game>, String> {
    Err("Job application not implemented".to_string())
}

// ============================================================================
// Youth Academy Handlers (stubs — replaced by scouting system)
// ============================================================================

pub async fn get_youth_recommendations(
    State(state): State<AppState>,
) -> Result<Json<Vec<Value>>, String> {
    // Replaced by develop's youth scouting system
    Ok(Json(vec![]))
}

pub async fn recruit_youth_player(
    State(state): State<AppState>,
    Json(_params): Json<Value>,
) -> Result<Json<Game>, String> {
    let game = state.state_manager
        .get_game(|g| g.clone())
        .ok_or("No active game session".to_string())?;
    Ok(Json(game))
}

fn calculate_age(dob: &str) -> u32 {
    use chrono::NaiveDate;
    if let Ok(birth) = NaiveDate::parse_from_str(dob, "%Y-%m-%d") {
        let today = chrono::Utc::now().date_naive();
        let years = today.signed_duration_since(birth).num_days() / 365;
        years as u32
    } else {
        20
    }
}

fn calculate_player_ovr(player: &domain::player::Player) -> u8 {
    let attrs = &player.attributes;
    ((attrs.pace as u32
        + attrs.stamina as u32
        + attrs.strength as u32
        + attrs.passing as u32
        + attrs.shooting as u32
        + attrs.tackling as u32
        + attrs.dribbling as u32
        + attrs.defending as u32
        + attrs.positioning as u32
        + attrs.vision as u32
        + attrs.decisions as u32) / 11) as u8
}

pub async fn health_check() -> &'static str {
    "OK"
}

// ============================================================================
// Additional Handlers (from develop commands)
// ============================================================================

pub async fn get_finance_snapshot(
    State(state): State<AppState>,
    Json(_params): Json<Value>,
) -> Result<Json<Value>, String> {
    let game = state.state_manager
        .get_game(|g| g.clone())
        .ok_or("No active game session".to_string())?;
    
    // Return team budget info from the manager's team
    let budget_info = game.manager.team_id.as_ref().and_then(|tid| {
        game.teams.iter().find(|t| t.id == *tid).map(|team| {
            serde_json::json!({
                "budget": team.finance,
                "wageBudget": team.wage_budget,
                "balance": team.finance,
            })
        })
    }).unwrap_or(serde_json::json!({}));
    
    Ok(Json(budget_info))
}

pub async fn request_board_support(
    State(state): State<AppState>,
) -> Result<Json<Value>, String> {
    let game = state.state_manager
        .get_game(|g| g.clone())
        .ok_or("No active game session".to_string())?;
    Ok(Json(serde_json::json!({ "game": game })))
}

pub async fn request_sponsor_pitch(
    State(state): State<AppState>,
) -> Result<Json<Value>, String> {
    let game = state.state_manager
        .get_game(|g| g.clone())
        .ok_or("No active game session".to_string())?;
    Ok(Json(serde_json::json!({ "game": game })))
}

pub async fn request_marketing_campaign(
    State(state): State<AppState>,
) -> Result<Json<Value>, String> {
    let game = state.state_manager
        .get_game(|g| g.clone())
        .ok_or("No active game session".to_string())?;
    Ok(Json(serde_json::json!({ "game": game })))
}

pub async fn offer_free_agent_contract(
    State(state): State<AppState>,
    Json(params): Json<Value>,
) -> Result<Json<Value>, String> {
    let _player_id = params.get("playerId")
        .or_else(|| params.get("player_id"))
        .and_then(|v| v.as_str())
        .map(String::from);
    let _weekly_wage = params.get("weeklyWage")
        .or_else(|| params.get("weekly_wage"))
        .and_then(|v| v.as_u64());
    let _contract_years = params.get("contractYears")
        .or_else(|| params.get("contract_years"))
        .and_then(|v| v.as_u64());
    
    let game = state.state_manager
        .get_game(|g| g.clone())
        .ok_or("No active game session".to_string())?;
    Ok(Json(serde_json::json!({ "game": game })))
}

pub async fn preview_free_agent_contract_impact(
    State(state): State<AppState>,
    Json(params): Json<Value>,
) -> Result<Json<Value>, String> {
    let _player_id = params.get("playerId")
        .or_else(|| params.get("player_id"))
        .and_then(|v| v.as_str())
        .map(String::from);
    let _weekly_wage = params.get("weeklyWage")
        .or_else(|| params.get("weekly_wage"))
        .and_then(|v| v.as_u64());
    let _contract_years = params.get("contractYears")
        .or_else(|| params.get("contract_years"))
        .and_then(|v| v.as_u64());
    
    Ok(Json(serde_json::json!({
        "weeklyWageImpact": 0,
        "annualWageImpact": 0,
        "budgetImpactPct": 0
    })))
}

pub async fn set_contract_exit_intent(
    State(state): State<AppState>,
    Json(params): Json<Value>,
) -> Result<Json<Value>, String> {
    let _player_id = params.get("playerId")
        .or_else(|| params.get("player_id"))
        .and_then(|v| v.as_str())
        .map(String::from);
    
    let mut game = state.state_manager
        .get_game(|g| g.clone())
        .ok_or("No active game session".to_string())?;
    
    if let Some(pid) = _player_id {
        if let Some(player) = game.players.iter_mut().find(|p| p.id == pid) {
            player.transfer_listed = true;
        }
    }
    
    state.state_manager.set_game(game.clone());
    Ok(Json(serde_json::json!({ "game": game })))
}

pub async fn clear_contract_exit_intent(
    State(state): State<AppState>,
    Json(params): Json<Value>,
) -> Result<Json<Value>, String> {
    let _player_id = params.get("playerId")
        .or_else(|| params.get("player_id"))
        .and_then(|v| v.as_str())
        .map(String::from);
    
    let mut game = state.state_manager
        .get_game(|g| g.clone())
        .ok_or("No active game session".to_string())?;
    
    if let Some(pid) = _player_id {
        if let Some(player) = game.players.iter_mut().find(|p| p.id == pid) {
            player.transfer_listed = false;
        }
    }
    
    state.state_manager.set_game(game.clone());
    Ok(Json(serde_json::json!({ "game": game })))
}

pub async fn preview_contract_termination(
    State(state): State<AppState>,
    Json(params): Json<Value>,
) -> Result<Json<Value>, String> {
    let player_id = params.get("playerId")
        .or_else(|| params.get("player_id"))
        .and_then(|v| v.as_str())
        .map(String::from);
    
    let game = state.state_manager
        .get_game(|g| g.clone())
        .ok_or("No active game session".to_string())?;
    
    let (pid, pname) = player_id.as_ref().and_then(|pid| {
        game.players.iter().find(|p| p.id == *pid).map(|p| (p.id.clone(), p.match_name.clone()))
    }).unwrap_or_default();
    
    let team_id = player_id.as_ref().and_then(|pid| {
        game.players.iter().find(|p| p.id == *pid).and_then(|p| p.team_id.clone())
    });
    
    // Count squad health
    let healthy_players = team_id.as_ref().map(|tid| {
        game.players.iter().filter(|p| p.team_id.as_deref() == Some(tid) && p.injury.is_none() && !p.retired).count()
    }).unwrap_or(0);
    let healthy_gks = team_id.as_ref().map(|tid| {
        game.players.iter().filter(|p| p.team_id.as_deref() == Some(tid) && p.injury.is_none() && !p.retired && matches!(p.position, domain::player::Position::Goalkeeper)).count()
    }).unwrap_or(0);
    let effective_xi = healthy_players.min(11);
    let can_field = healthy_players >= 11 && healthy_gks >= 1;
    
    Ok(Json(serde_json::json!({
        "preview": {
            "player_id": pid,
            "player_name": pname,
            "severance_cost": 0,
            "squad_safety": {
                "team_id": team_id.unwrap_or_default(),
                "projected_roster_size": healthy_players,
                "healthy_players": healthy_players,
                "healthy_goalkeepers": healthy_gks,
                "effective_xi_size": effective_xi,
                "can_field_matchday_squad": can_field,
                "missing_reasons": []
            }
        }
    })))
}

pub async fn terminate_contract_now(
    State(state): State<AppState>,
    Json(params): Json<Value>,
) -> Result<Json<Value>, String> {
    let _player_id = params.get("playerId")
        .or_else(|| params.get("player_id"))
        .and_then(|v| v.as_str())
        .map(String::from);
    
    let mut game = state.state_manager
        .get_game(|g| g.clone())
        .ok_or("No active game session".to_string())?;
    
    let mut removed_team_id: Option<String> = None;
    if let Some(pid) = _player_id {
        if let Some(player) = game.players.iter_mut().find(|p| p.id == pid) {
            removed_team_id = player.team_id.clone();
            player.team_id = None;
            player.contract_end = None;
            if let Some(tid) = &removed_team_id {
                if let Some(team) = game.teams.iter_mut().find(|t| t.id == *tid) {
                    team.starting_xi_ids.retain(|id| id != &pid);
                }
            }
        }
    }
    
    state.state_manager.set_game(game.clone());
    Ok(Json(serde_json::json!({
        "game": game,
        "severance_cost": 0,
        "squad_safety": {
            "team_id": removed_team_id.unwrap_or_default(),
            "projected_roster_size": 0,
            "healthy_players": 0,
            "healthy_goalkeepers": 0,
            "effective_xi_size": 0,
            "can_field_matchday_squad": true,
            "missing_reasons": []
        }
    })))
}

pub async fn set_player_squad_role(
    State(state): State<AppState>,
    Json(params): Json<Value>,
) -> Result<Json<Game>, String> {
    let player_id = params.get("playerId")
        .or_else(|| params.get("player_id"))
        .and_then(|v| v.as_str())
        .map(String::from);
    let squad_role = params.get("squadRole")
        .or_else(|| params.get("squad_role"))
        .and_then(|v| v.as_str())
        .map(String::from);
    
    let mut game = state.state_manager
        .get_game(|g| g.clone())
        .ok_or("No active game session".to_string())?;
    
    if let (Some(pid), Some(role)) = (&player_id, &squad_role) {
        if let Some(player) = game.players.iter_mut().find(|p| p.id == *pid) {
            // Convert string to SquadRole
            let role_enum = match role.as_str() {
                "Senior" => domain::player::SquadRole::Senior,
                "Youth" => domain::player::SquadRole::Youth,
                "KeyPlayer" | "Key Player" => domain::player::SquadRole::Senior,
                "Regular" => domain::player::SquadRole::Senior,
                "Rotation" => domain::player::SquadRole::Senior,
                "Prospect" => domain::player::SquadRole::Youth,
                "Captain" => domain::player::SquadRole::Senior,
                _ => domain::player::SquadRole::Senior,
            };
            player.squad_role = role_enum;
        }
    }
    
    state.state_manager.set_game(game.clone());
    Ok(Json(game))
}

pub async fn start_youth_scouting(
    State(state): State<AppState>,
    Json(params): Json<Value>,
) -> Result<Json<Game>, String> {
    let scout_id = params.get("scoutId")
        .or_else(|| params.get("scout_id"))
        .and_then(|v| v.as_str())
        .ok_or("Missing scoutId")?;
    let region_str = params.get("region")
        .and_then(|v| v.as_str())
        .unwrap_or("Domestic");
    let objective_str = params.get("objective")
        .and_then(|v| v.as_str())
        .unwrap_or("Balanced");
    let target_position = params.get("targetPosition")
        .or_else(|| params.get("target_position"))
        .and_then(|v| v.as_str())
        .map(|s| match s {
            "Goalkeeper" => domain::player::Position::Goalkeeper,
            "Defender" => domain::player::Position::Defender,
            "Midfielder" => domain::player::Position::Midfielder,
            "Forward" => domain::player::Position::Forward,
            _ => domain::player::Position::Midfielder,
        });
    
    let region = match region_str {
        "International" => ofm_core::game::YouthScoutingRegion::International,
        _ => ofm_core::game::YouthScoutingRegion::Domestic,
    };
    let objective = match objective_str {
        "HighPotential" => ofm_core::game::YouthScoutingObjective::HighPotential,
        "ReadySoon" => ofm_core::game::YouthScoutingObjective::ReadySoon,
        _ => ofm_core::game::YouthScoutingObjective::Balanced,
    };
    
    let mut game = state.state_manager
        .get_game(|g| g.clone())
        .ok_or("No active game session".to_string())?;
    
    ofm_core::scouting::start_youth_scouting(
        &mut game,
        scout_id,
        region,
        objective,
        target_position,
    )?;
    
    state.state_manager.set_game(game.clone());
    Ok(Json(game))
}

pub async fn cancel_youth_scouting(
    State(state): State<AppState>,
    Json(params): Json<Value>,
) -> Result<Json<Game>, String> {
    let assignment_id = params.get("assignmentId")
        .or_else(|| params.get("assignment_id"))
        .and_then(|v| v.as_str())
        .ok_or("Missing assignmentId")?;
    
    let mut game = state.state_manager
        .get_game(|g| g.clone())
        .ok_or("No active game session".to_string())?;
    
    ofm_core::scouting::cancel_youth_scouting(&mut game, assignment_id)?;
    
    state.state_manager.set_game(game.clone());
    Ok(Json(game))
}

pub async fn reassign_youth_scouting(
    State(state): State<AppState>,
    Json(params): Json<Value>,
) -> Result<Json<Game>, String> {
    let assignment_id = params.get("assignmentId")
        .or_else(|| params.get("assignment_id"))
        .and_then(|v| v.as_str())
        .ok_or("Missing assignmentId")?;
    let scout_id = params.get("scoutId")
        .or_else(|| params.get("scout_id"))
        .and_then(|v| v.as_str())
        .ok_or("Missing scoutId")?;
    
    let mut game = state.state_manager
        .get_game(|g| g.clone())
        .ok_or("No active game session".to_string())?;
    
    ofm_core::scouting::reassign_youth_scouting(&mut game, assignment_id, scout_id)?;
    
    state.state_manager.set_game(game.clone());
    Ok(Json(game))
}
