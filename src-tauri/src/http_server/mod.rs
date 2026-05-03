pub mod handlers;
pub mod models;

use axum::{routing::{post, get}, Router};
use std::sync::{Arc, Mutex};
use tower_http::cors::{Any, CorsLayer};

use db::save_manager::SaveManager;
use ofm_core::state::StateManager;

pub type AppState = Arc<HttpAppState>;

pub struct HttpAppState {
    pub state_manager: StateManager,
    pub save_manager: Mutex<SaveManager>,
    /// Path to the settings.json file (set on startup).
    pub settings_path: std::path::PathBuf,
}

pub async fn start_server(port: u16) -> Result<(), Box<dyn std::error::Error>> {
    use crate::http_server::handlers::*;

    // Initialize state
    let state_manager = StateManager::new();
    let saves_dir = dirs::data_local_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join("openfootmanager")
        .join("saves");
    std::fs::create_dir_all(&saves_dir)?;
    let save_manager = SaveManager::init(&saves_dir)?;

    let settings_dir = dirs::data_local_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join("openfootmanager");
    std::fs::create_dir_all(&settings_dir)?;

    let app_state = Arc::new(HttpAppState {
        state_manager,
        save_manager: std::sync::Mutex::new(save_manager),
        settings_path: settings_dir.join("settings.json"),
    });

    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    let app = Router::new()
        // Game management
        .route("/api/get_saves", get(get_saves).post(get_saves))
        .route("/api/delete_save", post(delete_save))
        .route("/api/start_new_game", post(start_new_game))
        .route("/api/list_world_databases", get(list_world_databases).post(list_world_databases))
        .route("/api/export_world_database", post(export_world_database))
        .route("/api/write_temp_database", post(write_temp_database))
        .route("/api/select_team", post(select_team))
        .route("/api/load_game", post(load_game))
        .route("/api/get_active_game", post(get_active_game))
        .route("/api/save_game", post(save_game))
        .route("/api/exit_to_menu", post(exit_to_menu))
        
        // Time management
        .route("/api/advance_time", post(advance_time))
        .route("/api/advance_time_with_mode", post(advance_time_with_mode))
        .route("/api/skip_to_match_day", post(skip_to_match_day))
        .route("/api/check_blocking_actions", post(check_blocking_actions))
        
        // Squad & team
        .route("/api/get_squad", post(get_squad))
        .route("/api/get_league", get(get_league).post(get_league))
        .route("/api/upgrade_facility", post(upgrade_facility))
        .route("/api/set_formation", post(set_formation))
        .route("/api/recalculate_positions", post(recalculate_positions))
        .route("/api/set_starting_xi", post(set_starting_xi))
        .route("/api/set_play_style", post(set_play_style))
        .route("/api/set_team_match_roles", post(set_team_match_roles))
        
        // Contracts
        .route("/api/propose_renewal", post(propose_renewal))
        .route("/api/delegate_renewals", post(delegate_renewals))
        .route("/api/preview_renewal_financial_impact", post(preview_renewal_financial_impact))
        
        // Training
        .route("/api/set_training", post(set_training))
        .route("/api/set_training_schedule", post(set_training_schedule))
        .route("/api/set_training_groups", post(set_training_groups))
        .route("/api/set_player_training_focus", post(set_player_training_focus))
        
        // Staff
        .route("/api/hire_staff", post(hire_staff))
        .route("/api/release_staff", post(release_staff))
        
        // Messages
        .route("/api/mark_message_read", post(mark_message_read))
        .route("/api/delete_message", post(delete_message))
        .route("/api/delete_messages", post(delete_messages))
        .route("/api/mark_all_messages_read", post(mark_all_messages_read))
        .route("/api/clear_old_messages", post(clear_old_messages))
        .route("/api/resolve_message_action", post(resolve_message_action))
        
        // Transfers
        .route("/api/auto_select_set_pieces", post(auto_select_set_pieces))
        .route("/api/toggle_transfer_list", post(toggle_transfer_list))
        .route("/api/toggle_loan_list", post(toggle_loan_list))
        .route("/api/make_transfer_bid", post(make_transfer_bid))
        .route("/api/preview_transfer_bid_financial_impact", post(preview_transfer_bid_financial_impact))
        .route("/api/respond_to_offer", post(respond_to_offer))
        .route("/api/counter_offer", post(counter_offer))
        
        // Scouting
        .route("/api/send_scout", post(send_scout))
        
        // Season
        .route("/api/check_season_complete", post(check_season_complete))
        .route("/api/advance_to_next_season", post(advance_to_next_season))
        .route("/api/get_season_awards", post(get_season_awards))
        
        // Match
        .route("/api/start_live_match", post(start_live_match))
        .route("/api/get_player_match_history", post(get_player_match_history))
        .route("/api/get_player_stats_overview", post(get_player_stats_overview))
        .route("/api/get_team_match_history", post(get_team_match_history))
        .route("/api/get_team_stats_overview", post(get_team_stats_overview))
        .route("/api/step_live_match", post(step_live_match))
        .route("/api/apply_match_command", post(apply_match_command))
        .route("/api/get_match_snapshot", post(get_match_snapshot))
        .route("/api/finish_live_match", post(finish_live_match))
        .route("/api/abandon_match", post(abandon_match))
        .route("/api/apply_team_talk", post(apply_team_talk))
        .route("/api/submit_press_conference", post(submit_press_conference))
        
        // Settings
        .route("/api/get_settings", get(get_settings).post(get_settings))
        .route("/api/save_settings", post(save_settings))
        .route("/api/clear_all_saves", post(clear_all_saves))
        
        // Jobs
        .route("/api/get_available_jobs", post(get_available_jobs))
        .route("/api/apply_for_job", post(apply_for_job))
        
        // Youth Academy
        .route("/api/get_youth_recommendations", post(get_youth_recommendations))
        .route("/api/recruit_youth_player", post(recruit_youth_player))
        
        // Health check
        .route("/api/health", get(health_check))
        
        .with_state(app_state)
        .layer(cors);

    let addr = format!("0.0.0.0:{}", port);
    println!("Starting HTTP server on http://{}", addr);
    println!("Press Ctrl+C to stop");

    let listener = tokio::net::TcpListener::bind(&addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
