//! HTTP handlers for web version
//! Direct implementations of game logic for HTTP API access

use axum::{extract::State, Json};
use chrono::Datelike;
use log::info;
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
    
    let (teams, players, staff) = if world_source == "random" && inline_json.is_none() {
        generate_world(None)
    } else if let Some(json_str) = inline_json {
        // Use inline JSON data directly
        let world = load_world_from_json(json_str)?;
        (world.teams, world.players, world.staff)
    } else {
        let path = world_source.strip_prefix("file:").unwrap_or(&world_source);
        let json = std::fs::read_to_string(path)
            .map_err(|e| format!("Failed to read world database: {}", e))?;
        let world = load_world_from_json(&json)?;
        (world.teams, world.players, world.staff)
    };

    let new_game = Game::new(clock, manager, teams, players, staff, vec![]);
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

    let season_start = game.clock.current_date + Duration::days(30);
    let team_ids: Vec<String> = game.teams.iter().map(|t| t.id.clone()).collect();
    let mut league = schedule::generate_league("Premier Division", 2026, &team_ids, season_start);
    let opponents: Vec<String> = team_ids
        .iter()
        .filter(|candidate_team_id| candidate_team_id.as_str() != team_id)
        .cloned()
        .collect();
    let friendlies = schedule::generate_preseason_friendlies(&team_id, &opponents, season_start, 3);
    schedule::append_fixtures(&mut league, friendlies);
    game.league = Some(league);
    refresh_game_context(&mut game);

    // Generate youth recommendations if it's the first day of a new month
    if game.clock.current_date.date_naive().day() == 1 {
        ofm_core::youth_academy::generate_monthly_recommendations(&mut game);
    }
    ofm_core::youth_academy::cleanup_expired_recommendations(&mut game);

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

    // Generate youth recommendations if it's the first day of a new month
    if game.clock.current_date.date_naive().day() == 1 {
        ofm_core::youth_academy::generate_monthly_recommendations(&mut game);
    }
    ofm_core::youth_academy::cleanup_expired_recommendations(&mut game);

    let mgr_name = format!("{} {}", game.manager.first_name, game.manager.last_name);

    state.state_manager.set_save_id(params.save_id);
    state.state_manager.set_game(game);
    state.state_manager.set_stats_state(stats_state);
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
    
    // Update manager's team formation
    if let Some(ref team_id) = game.manager.team_id {
        if let Some(team) = game.teams.iter_mut().find(|t| t.id == *team_id) {
            team.formation = formation;
        }
    }
    
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
    
    if let Some(msg) = game.messages.iter_mut().find(|m| m.id == message_id) {
        if let Some(action) = msg.actions.iter_mut().find(|a| a.id == action_id) {
            action.resolved = true;
        }
    }
    
    state.state_manager.set_game(game.clone());
    
    Ok(Json(serde_json::json!({
        "game": game,
        "effect": null,
        "effect_i18n_key": null,
        "effect_i18n_params": null
    })))
}

pub async fn auto_select_set_pieces(State(state): State<AppState>, Json(_params): Json<Value>) -> Result<Json<Game>, String> {
    state.state_manager
        .get_game(|g| g.clone())
        .ok_or("No active game session".to_string())
        .map(Json)
}

pub async fn toggle_transfer_list(State(state): State<AppState>, Json(_params): Json<Value>) -> Result<Json<Game>, String> {
    state.state_manager
        .get_game(|g| g.clone())
        .ok_or("No active game session".to_string())
        .map(Json)
}

pub async fn toggle_loan_list(State(state): State<AppState>, Json(_params): Json<Value>) -> Result<Json<Game>, String> {
    state.state_manager
        .get_game(|g| g.clone())
        .ok_or("No active game session".to_string())
        .map(Json)
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

pub async fn respond_to_offer(State(state): State<AppState>, Json(_params): Json<Value>) -> Result<Json<Game>, String> {
    state.state_manager
        .get_game(|g| g.clone())
        .ok_or("No active game session".to_string())
        .map(Json)
}

pub async fn counter_offer(State(state): State<AppState>, Json(_params): Json<Value>) -> Result<Json<Value>, String> {
    let _game = state.state_manager
        .get_game(|g| g.clone())
        .ok_or("No active game session".to_string())?;
    Ok(Json(serde_json::json!({})))
}

pub async fn send_scout(State(state): State<AppState>, Json(_params): Json<Value>) -> Result<Json<Game>, String> {
    state.state_manager
        .get_game(|g| g.clone())
        .ok_or("No active game session".to_string())
        .map(Json)
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

pub async fn advance_to_next_season(State(state): State<AppState>) -> Result<Json<Game>, String> {
    state.state_manager
        .get_game(|g| g.clone())
        .ok_or("No active game session".to_string())
        .map(Json)
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
    
    // Return the snapshot directly (engine types are already Serialize)
    Ok(Json(to_camel_json(&snapshot)?))
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
    
    // Save the updated game
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

pub async fn get_settings(State(_state): State<AppState>) -> Result<Json<Value>, String> {
    // Return default settings
    Ok(Json(serde_json::json!({
        "language": "en",
        "default_match_mode": "live",
        "volume": 0.8
    })))
}

pub async fn save_settings(State(_state): State<AppState>, Json(_params): Json<Value>) -> Result<Json<()>, String> {
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
// Youth Academy Handlers
// ============================================================================

pub async fn get_youth_recommendations(
    State(state): State<AppState>,
) -> Result<Json<Vec<Value>>, String> {
    let game = state.state_manager
        .get_game(|g| g.clone())
        .ok_or("No game in progress")?;
    
    let recommendations = ofm_core::youth_academy::get_current_recommendations(&game);
    
    let result: Vec<Value> = recommendations.iter().map(|r| {
        let player = &r.player;
        serde_json::json!({
            "id": r.id,
            "playerId": player.id.clone(),
            "fullName": player.full_name.clone(),
            "matchName": player.match_name.clone(),
            "position": format!("{:?}", player.position),
            "age": calculate_age(&player.date_of_birth),
            "overallRating": calculate_player_ovr(player),
            "potentialRating": calculate_player_ovr(player) + 10, // Youth players have growth potential
            "recruitmentCost": r.recruitment_cost,
            "expiresAt": r.expires_at,
            "facilityBonus": r.facility_bonus,
        })
    }).collect();
    
    Ok(Json(result))
}

pub async fn recruit_youth_player(
    State(state): State<AppState>,
    Json(params): Json<Value>,
) -> Result<Json<Game>, String> {
    let recommendation_id = params.get("recommendationId")
        .and_then(|v| v.as_str())
        .ok_or("Missing recommendationId")?;

    let mut game = state.state_manager
        .get_game(|g| g.clone())
        .ok_or("No active game session".to_string())?;

    // Recruit the youth player
    let _result = ofm_core::youth_academy::recruit_youth_player(&mut game, recommendation_id)
        .map_err(|e| e.to_string())?;


    // Save the game immediately so the new player is persisted
    if let Some(save_id) = state.state_manager.get_save_id() {
        let mut sm = state.save_manager.lock().map_err(|e: std::sync::PoisonError<_>| e.to_string())?;
        sm.save_game(&game, &save_id)?;
    }

    // Update game state in memory
    state.state_manager.set_game(game.clone());

    // Return the complete game state
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
