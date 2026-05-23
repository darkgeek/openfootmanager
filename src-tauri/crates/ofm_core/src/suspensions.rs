//! Suspension system for red and yellow cards
//! - Red card: automatic 1 match ban
//! - 3 yellow cards: 1 match ban (yellow cards reset each season)
//! - Friendly matches don't count towards suspensions

use crate::game::Game;

/// Get list of suspended players for a team.
pub fn get_suspended_players(
    game: &Game,
    team_id: &str,
) -> Vec<(String, String, u8)> {
    game.players
        .iter()
        .filter(|p| p.team_id.as_deref() == Some(team_id) && p.suspension_games_remaining > 0)
        .map(|p| (p.id.clone(), p.match_name.clone(), p.suspension_games_remaining))
        .collect()
}