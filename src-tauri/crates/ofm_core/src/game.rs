use crate::clock::GameClock;
use domain::league::League;
use domain::manager::Manager;
use domain::message::InboxMessage;
use domain::news::NewsArticle;
use domain::player::Player;
use domain::season::SeasonContext;
use domain::staff::Staff;
use domain::team::Team;

use crate::training_report::TeamTrainingSnapshot;
use crate::youth_academy::YouthRecommendation;

use serde::{Deserialize, Serialize};

fn default_board_firing_enabled() -> bool { true }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ObjectiveType {
    LeaguePosition,
    Wins,
    GoalsScored,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BoardObjective {
    pub id: String,
    pub description: String,
    pub target: u32,
    pub objective_type: ObjectiveType,
    pub met: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScoutingAssignment {
    pub id: String,
    pub player_id: String,
    pub days_remaining: u32,
    pub scout_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Game {
    pub clock: GameClock,
    pub manager: Manager,
    pub teams: Vec<Team>,
    pub players: Vec<Player>,
    pub staff: Vec<Staff>,
    pub messages: Vec<InboxMessage>,
    #[serde(default)]
    pub news: Vec<NewsArticle>,
    pub league: Option<League>,
    #[serde(default)]
    pub scouting_assignments: Vec<ScoutingAssignment>,
    #[serde(default)]
    pub board_objectives: Vec<BoardObjective>,
    #[serde(default)]
    pub season_context: SeasonContext,
    #[serde(default)]
    pub days_since_last_job_offer: Option<u32>,
    #[serde(default)]
    pub youth_recommendations: Vec<YouthRecommendation>,
    #[serde(default)]
    pub training_snapshots: Vec<TeamTrainingSnapshot>,
    /// When false, the board will never fire the manager regardless of satisfaction.
    /// Set via game difficulty settings.
    #[serde(default = "default_board_firing_enabled")]
    pub board_firing_enabled: bool,
}

impl Game {
    pub fn new(
        clock: GameClock,
        manager: Manager,
        teams: Vec<Team>,
        players: Vec<Player>,
        staff: Vec<Staff>,
        messages: Vec<InboxMessage>,
    ) -> Self {
        let mut game = Self {
            clock,
            manager,
            teams,
            players,
            staff,
            messages,
            news: vec![],
            league: None,
            scouting_assignments: vec![],
            board_objectives: vec![],
            season_context: SeasonContext::default(),
            days_since_last_job_offer: None,
            youth_recommendations: vec![],
            training_snapshots: vec![],
            board_firing_enabled: true,
        };
        crate::football_identity::upgrade_game_football_identities(&mut game);
        crate::season_context::refresh_game_context(&mut game);
        game
    }
}