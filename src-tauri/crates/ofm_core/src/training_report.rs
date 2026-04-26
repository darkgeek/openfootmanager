//! Monthly Training Report System
//! 
//! Each month, generates a report comparing current player attributes
//! with the previous month's snapshot, showing training progress.

use crate::game::Game;
use domain::message::{InboxMessage, MessageCategory, MessagePriority};
use domain::player::Player;
use serde::{Deserialize, Serialize};

/// A snapshot of a single player's attributes at a point in time
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlayerAttributeSnapshot {
    pub player_id: String,
    pub pace: u8,
    pub stamina: u8,
    pub strength: u8,
    pub agility: u8,
    pub passing: u8,
    pub shooting: u8,
    pub tackling: u8,
    pub dribbling: u8,
    pub defending: u8,
    pub positioning: u8,
    pub vision: u8,
    pub decisions: u8,
    pub composure: u8,
    pub aggression: u8,
    pub teamwork: u8,
    pub leadership: u8,
    pub handling: u8,
    pub reflexes: u8,
    pub aerial: u8,
}

impl PlayerAttributeSnapshot {
    pub fn from_player(player: &Player) -> Self {
        Self {
            player_id: player.id.clone(),
            pace: player.attributes.pace,
            stamina: player.attributes.stamina,
            strength: player.attributes.strength,
            agility: player.attributes.agility,
            passing: player.attributes.passing,
            shooting: player.attributes.shooting,
            tackling: player.attributes.tackling,
            dribbling: player.attributes.dribbling,
            defending: player.attributes.defending,
            positioning: player.attributes.positioning,
            vision: player.attributes.vision,
            decisions: player.attributes.decisions,
            composure: player.attributes.composure,
            aggression: player.attributes.aggression,
            teamwork: player.attributes.teamwork,
            leadership: player.attributes.leadership,
            handling: player.attributes.handling,
            reflexes: player.attributes.reflexes,
            aerial: player.attributes.aerial,
        }
    }

    /// Calculate overall rating
    pub fn overall(&self) -> f64 {
        (self.pace as f64
            + self.stamina as f64
            + self.strength as f64
            + self.passing as f64
            + self.shooting as f64
            + self.tackling as f64
            + self.dribbling as f64
            + self.defending as f64
            + self.positioning as f64
            + self.vision as f64
            + self.decisions as f64)
            / 11.0
    }
}

/// Stores the attribute snapshots for all players on a team at a specific date
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TeamTrainingSnapshot {
    pub team_id: String,
    pub recorded_date: String,
    pub players: Vec<PlayerAttributeSnapshot>,
}

/// Attribute change for a single player
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlayerAttributeChange {
    pub player_id: String,
    pub player_name: String,
    pub position: String,
    pub overall_change: f64,
    pub changes: Vec<AttributeChange>,
}

/// A single attribute change
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AttributeChange {
    pub name: String,
    pub old_value: u8,
    pub new_value: u8,
    pub change: i8,
}

impl AttributeChange {
    pub fn format_change(&self) -> String {
        if self.change > 0 {
            format!("+{}", self.change)
        } else {
            self.change.to_string()
        }
    }
}

/// Generate a monthly training report comparing current attributes with last month's snapshot.
/// Called from process_day when it's the first day of a new month.
pub fn generate_monthly_training_report(game: &mut Game) {
    let user_team_id = match &game.manager.team_id {
        Some(id) => id.clone(),
        None => return,
    };

    let current_date = game.clock.current_date.format("%Y-%m-%d").to_string();
    let current_month = game.clock.current_date.format("%Y-%m").to_string();

    log::info!("[training_report] Processing for team {} on {} (month: {})", 
        user_team_id, current_date, current_month);
    log::info!("[training_report] Current snapshots count: {}", game.training_snapshots.len());

    // Collect player data before any mutable borrow
    let team_player_ids: Vec<String> = game.players.iter()
        .filter(|p| p.team_id.as_deref() == Some(&user_team_id))
        .map(|p| p.id.clone())
        .collect();

    if team_player_ids.is_empty() {
        log::info!("[training_report] No players found for team {}", user_team_id);
        return;
    }

    // Get or create snapshot for this team
    let existing_idx = game.training_snapshots.iter()
        .position(|s| s.team_id == user_team_id);

    match existing_idx {
        Some(idx) => {
            // Clone the recorded date before any mutable borrow
            let last_month = game.training_snapshots[idx].recorded_date.clone();
            let snapshot = &game.training_snapshots[idx];
            
            log::info!("[training_report] Found existing snapshot from {} with {} players", 
                last_month, snapshot.players.len());
            
            // Check if we already have a snapshot for this month
            if snapshot.recorded_date.starts_with(&current_month) {
                log::info!("[training_report] Already have snapshot for this month, skipping");
                return; // Already generated this month
            }

            // Calculate changes using player IDs and snapshot
            let changes = calculate_changes_for_team(&team_player_ids, &game.players, snapshot);
            log::info!("[training_report] Calculated {} changes", changes.len());

            // Generate report message
            generate_report_message(game, &changes, &current_date, &last_month);

            // Update snapshot to current values
            let current_date_clone = current_date.clone();
            let snapshot_data: Vec<PlayerAttributeSnapshot> = game.players.iter()
                .filter(|p| p.team_id.as_deref() == Some(&user_team_id))
                .map(|p| PlayerAttributeSnapshot::from_player(p))
                .collect();
            game.training_snapshots[idx] = TeamTrainingSnapshot {
                team_id: user_team_id.clone(),
                recorded_date: current_date_clone,
                players: snapshot_data,
            };
        }
        None => {
            // First time - create new snapshot (no report for first month)
            log::info!("[training_report] Creating first snapshot for team {} on {} with {} players", 
                user_team_id, current_date, team_player_ids.len());
            let snapshot_data: Vec<PlayerAttributeSnapshot> = game.players.iter()
                .filter(|p| p.team_id.as_deref() == Some(&user_team_id))
                .map(|p| PlayerAttributeSnapshot::from_player(p))
                .collect();
            let snapshot = TeamTrainingSnapshot {
                team_id: user_team_id.clone(),
                recorded_date: current_date.clone(),
                players: snapshot_data,
            };
            game.training_snapshots.push(snapshot);
        }
    }
}

/// Calculate attribute changes for a team using player IDs
fn calculate_changes_for_team(
    player_ids: &[String],
    all_players: &[Player],
    snapshot: &TeamTrainingSnapshot,
) -> Vec<PlayerAttributeChange> {
    let mut changes = Vec::new();

    for player_id in player_ids {
        // Find current player
        if let Some(player) = all_players.iter().find(|p| &p.id == player_id) {
            // Find snapshot for this player
            if let Some(snap) = snapshot.players.iter().find(|s| s.player_id == *player_id) {
                let player_changes = calculate_single_player_changes(snap, player);
                
                // Only include if there's any change
                if player_changes.changes.iter().any(|c| c.change != 0) {
                    changes.push(player_changes);
                }
            }
        }
    }

    changes
}

fn calculate_single_player_changes(
    snap: &PlayerAttributeSnapshot,
    player: &Player,
) -> PlayerAttributeChange {
    let attrs = &player.attributes;
    
    let all_changes = vec![
        AttributeChange { name: "Pace".to_string(), old_value: snap.pace, new_value: attrs.pace, change: attrs.pace as i8 - snap.pace as i8 },
        AttributeChange { name: "Stamina".to_string(), old_value: snap.stamina, new_value: attrs.stamina, change: attrs.stamina as i8 - snap.stamina as i8 },
        AttributeChange { name: "Strength".to_string(), old_value: snap.strength, new_value: attrs.strength, change: attrs.strength as i8 - snap.strength as i8 },
        AttributeChange { name: "Agility".to_string(), old_value: snap.agility, new_value: attrs.agility, change: attrs.agility as i8 - snap.agility as i8 },
        AttributeChange { name: "Passing".to_string(), old_value: snap.passing, new_value: attrs.passing, change: attrs.passing as i8 - snap.passing as i8 },
        AttributeChange { name: "Shooting".to_string(), old_value: snap.shooting, new_value: attrs.shooting, change: attrs.shooting as i8 - snap.shooting as i8 },
        AttributeChange { name: "Tackling".to_string(), old_value: snap.tackling, new_value: attrs.tackling, change: attrs.tackling as i8 - snap.tackling as i8 },
        AttributeChange { name: "Dribbling".to_string(), old_value: snap.dribbling, new_value: attrs.dribbling, change: attrs.dribbling as i8 - snap.dribbling as i8 },
        AttributeChange { name: "Defending".to_string(), old_value: snap.defending, new_value: attrs.defending, change: attrs.defending as i8 - snap.defending as i8 },
        AttributeChange { name: "Positioning".to_string(), old_value: snap.positioning, new_value: attrs.positioning, change: attrs.positioning as i8 - snap.positioning as i8 },
        AttributeChange { name: "Vision".to_string(), old_value: snap.vision, new_value: attrs.vision, change: attrs.vision as i8 - snap.vision as i8 },
        AttributeChange { name: "Decisions".to_string(), old_value: snap.decisions, new_value: attrs.decisions, change: attrs.decisions as i8 - snap.decisions as i8 },
        AttributeChange { name: "Composure".to_string(), old_value: snap.composure, new_value: attrs.composure, change: attrs.composure as i8 - snap.composure as i8 },
        AttributeChange { name: "Aggression".to_string(), old_value: snap.aggression, new_value: attrs.aggression, change: attrs.aggression as i8 - snap.aggression as i8 },
        AttributeChange { name: "Teamwork".to_string(), old_value: snap.teamwork, new_value: attrs.teamwork, change: attrs.teamwork as i8 - snap.teamwork as i8 },
        AttributeChange { name: "Leadership".to_string(), old_value: snap.leadership, new_value: attrs.leadership, change: attrs.leadership as i8 - snap.leadership as i8 },
        AttributeChange { name: "Handling".to_string(), old_value: snap.handling, new_value: attrs.handling, change: attrs.handling as i8 - snap.handling as i8 },
        AttributeChange { name: "Reflexes".to_string(), old_value: snap.reflexes, new_value: attrs.reflexes, change: attrs.reflexes as i8 - snap.reflexes as i8 },
        AttributeChange { name: "Aerial".to_string(), old_value: snap.aerial, new_value: attrs.aerial, change: attrs.aerial as i8 - snap.aerial as i8 },
    ];

    let current_overall = (attrs.pace as f64
        + attrs.stamina as f64
        + attrs.strength as f64
        + attrs.passing as f64
        + attrs.shooting as f64
        + attrs.tackling as f64
        + attrs.dribbling as f64
        + attrs.defending as f64
        + attrs.positioning as f64
        + attrs.vision as f64
        + attrs.decisions as f64)
        / 11.0;

    let old_overall = snap.overall();

    PlayerAttributeChange {
        player_id: player.id.clone(),
        player_name: player.match_name.clone(),
        position: format!("{:?}", player.position),
        overall_change: current_overall - old_overall,
        changes: all_changes,
    }
}

/// Generate the inbox message with training report
fn generate_report_message(game: &mut Game, changes: &[PlayerAttributeChange], date: &str, _last_month: &str) {
    let mut body = String::new();
    body.push_str("## Monthly Training Report\n\n");
    body.push_str("Here's how your squad has developed this month:\n\n");

    // Sort by overall change (biggest improvement first)
    let mut sorted_changes = changes.to_vec();
    sorted_changes.sort_by(|a, b| {
        b.overall_change.partial_cmp(&a.overall_change).unwrap()
    });

    if sorted_changes.is_empty() {
        // No changes at all
        body.push_str("📋 No significant attribute changes this month.\n\n");
        body.push_str("Training continues - attribute improvements are gradual and may not show every month.\n");
    } else {
        for change in &sorted_changes {
            let direction = if change.overall_change > 0.0 { "📈" } else if change.overall_change < 0.0 { "📉" } else { "➡️" };
            body.push_str(&format!("### {} {} ({})\n", direction, change.player_name, change.position));
            body.push_str(&format!("**Overall: {:+.2}**\n\n", change.overall_change));

            // Show only attributes that changed
            let changed_attrs: Vec<_> = change.changes.iter()
                .filter(|c| c.change != 0)
                .collect();

            if changed_attrs.is_empty() {
                body.push_str("*No attribute changes this month*\n\n");
            } else {
                for attr in changed_attrs {
                    let arrow = if attr.change > 0 { "⬆️" } else { "⬇️" };
                    body.push_str(&format!("{} {}: {} → {} ({})\n", 
                        arrow,
                        attr.name,
                        attr.old_value,
                        attr.new_value,
                        attr.format_change()
                    ));
                }
            }
            body.push_str("---\n\n");
        }
    }

    // Create message
    let message = InboxMessage::new(
        format!("training_report_{}", date),
        format!("Monthly Training Report - {}", game.clock.current_date.format("%B %Y")),
        body,
        "Training Department".to_string(),
        date.to_string(),
    )
    .with_category(MessageCategory::Training)
    .with_priority(MessagePriority::Normal)
    .with_sender_role("Coach");

    game.messages.push(message);
}
