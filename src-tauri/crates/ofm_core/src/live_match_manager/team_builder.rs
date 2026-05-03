use crate::game::Game;
use crate::player_rating::{effective_rating_for_assignment, formation_slots, natural_ovr};
use domain::player::Position as DomainPosition;
use engine::{PlayStyle, PlayerData, Position as EnginePosition, TeamData};

// ---------------------------------------------------------------------------
// Domain → Engine conversion with starting XI / bench split
// ---------------------------------------------------------------------------

pub(super) fn build_team_with_bench(game: &Game, team_id: &str) -> (TeamData, Vec<PlayerData>) {
    let team = game.teams.iter().find(|t| t.id == team_id);
    let (name, formation, play_style) = match team {
        Some(t) => (
            t.name.clone(),
            t.formation.clone(),
            match t.play_style {
                domain::team::PlayStyle::Attacking => PlayStyle::Attacking,
                domain::team::PlayStyle::Defensive => PlayStyle::Defensive,
                domain::team::PlayStyle::Possession => PlayStyle::Possession,
                domain::team::PlayStyle::Counter => PlayStyle::Counter,
                domain::team::PlayStyle::HighPress => PlayStyle::HighPress,
                _ => PlayStyle::Balanced,
            },
        ),
        None => ("Unknown".into(), "4-4-2".into(), PlayStyle::Balanced),
    };

    // Collect all available (non-injured, non-suspended) players for this team
    let available_players: Vec<&domain::player::Player> = game
        .players
        .iter()
        .filter(|p| {
            p.team_id.as_deref() == Some(team_id)
                && p.injury.is_none()
                && p.suspension_games_remaining == 0
        })
        .collect();

    // Respect the user's saved starting XI from Tactics/Squad if available.
    // Fall back to auto-generation only when the saved list is empty or too short.
    let saved_xi_ids: Vec<&str> = team
        .as_ref()
        .map(|t| t.starting_xi_ids.iter().map(|s| s.as_str()).collect())
        .unwrap_or_default();

    let mut used_ids = std::collections::HashSet::new();
    let mut starting_xi = Vec::with_capacity(11);

    // If the user has saved a meaningful XI (>= 8 players), use it.
    // This preserves their lineup choices made in the Tactics screen.
    if saved_xi_ids.len() >= 8 {
        let slots = formation_slots(&formation);
        // Group slots by domain position group so we can assign players correctly
        let gk_slots: Vec<_> = slots.iter().take_while(|s| matches!(s, DomainPosition::Goalkeeper)).collect();
        let def_slots: Vec<_> = slots.iter().skip(gk_slots.len()).take_while(|s| s.to_group_position() == DomainPosition::Defender).collect();
        let mid_slots: Vec<_> = slots.iter().skip(gk_slots.len() + def_slots.len()).take_while(|s| s.to_group_position() == DomainPosition::Midfielder).collect();
        let fwd_slots: Vec<_> = slots.iter().skip(gk_slots.len() + def_slots.len() + mid_slots.len()).collect();
        // Interleave back into group order
        let ordered_slots: Vec<&DomainPosition> = gk_slots.iter()
            .chain(def_slots.iter())
            .chain(mid_slots.iter())
            .chain(fwd_slots.iter())
            .copied()
            .collect();

        for (slot_idx, slot) in ordered_slots.iter().enumerate().take(11) {
            let slot_group = slot.to_group_position();

            // Try: 1) saved player for this slot index, 2) any other saved player,
            // 3) best available from matching group, 4) any best available
            let player_opt: Option<&domain::player::Player> = saved_xi_ids
                .get(slot_idx)
                .and_then(|id| available_players.iter().find(|p| p.id.as_str() == *id && !used_ids.contains(&p.id)))
                .copied();

            let player_opt = player_opt.or_else(|| {
                saved_xi_ids.iter()
                    .find(|id| available_players.iter().any(|p| p.id.as_str() == **id && !used_ids.contains(&p.id)))
                    .and_then(|id| available_players.iter().find(|p| p.id.as_str() == *id && !used_ids.contains(&p.id)))
                    .copied()
            });

            let player_opt = player_opt.or_else(|| {
                available_players.iter()
                    .filter(|p| !used_ids.contains(&p.id))
                    .filter(|p| p.position.to_group_position() == slot_group)
                    .max_by(|left, right| {
                        effective_rating_for_assignment(left, slot)
                            .partial_cmp(&effective_rating_for_assignment(right, slot))
                            .unwrap_or(std::cmp::Ordering::Equal)
                    })
                    .copied()
            });

            let player_opt = player_opt.or_else(|| {
                available_players.iter()
                    .filter(|p| !used_ids.contains(&p.id))
                    .max_by(|left, right| {
                        effective_rating_for_assignment(left, slot)
                            .partial_cmp(&effective_rating_for_assignment(right, slot))
                            .unwrap_or(std::cmp::Ordering::Equal)
                    })
                    .copied()
            });

            let Some(player) = player_opt else { break };
            used_ids.insert(player.id.clone());
            starting_xi.push(to_engine_player_for_slot(player, slot_group));
        }
    }

    // If we don't have 11 yet (no saved XI, or not enough valid players in it),
    // fill the remaining slots using auto-selection.
    if starting_xi.len() < 11 {
        fill_remaining_slots(
            &available_players,
            &mut used_ids,
            &mut starting_xi,
            &formation,
        );
    }

    let mut bench_domain: Vec<&domain::player::Player> = available_players
        .into_iter()
        .filter(|player| !used_ids.contains(&player.id))
        .collect();
    bench_domain.sort_by(|left, right| {
        natural_ovr(right)
            .partial_cmp(&natural_ovr(left))
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    let bench = bench_domain.into_iter().map(to_engine_player).collect();

    let team_data = TeamData {
        id: team_id.to_string(),
        name,
        formation,
        play_style,
        players: starting_xi,
    };

    (team_data, bench)
}

fn fill_remaining_slots(
    available_players: &[&domain::player::Player],
    used_ids: &mut std::collections::HashSet<String>,
    starting_xi: &mut Vec<PlayerData>,
    formation: &str,
) {
    let slots = formation_slots(formation);
    let gk_slots: Vec<_> = slots.iter().take_while(|s| matches!(s, DomainPosition::Goalkeeper)).collect();
    let def_slots: Vec<_> = slots.iter().skip(gk_slots.len()).take_while(|s| s.to_group_position() == DomainPosition::Defender).collect();
    let mid_slots: Vec<_> = slots.iter().skip(gk_slots.len() + def_slots.len()).take_while(|s| s.to_group_position() == DomainPosition::Midfielder).collect();
    let fwd_slots: Vec<_> = slots.iter().skip(gk_slots.len() + def_slots.len() + mid_slots.len()).collect();
    let ordered_slots: Vec<&DomainPosition> = gk_slots.iter()
        .chain(def_slots.iter())
        .chain(mid_slots.iter())
        .chain(fwd_slots.iter())
        .copied()
        .collect();

    for slot in ordered_slots.iter().skip(starting_xi.len()).take(11) {
        let slot_group = slot.to_group_position();
        // available_players is &[&Player], so *p is &Player, need to clone id
        let best_player = available_players
            .iter()
            .filter(|p| !used_ids.contains(&p.id))
            .filter(|p| p.position.to_group_position() == slot_group)
            .max_by(|left, right| {
                effective_rating_for_assignment(left, *slot)
                    .partial_cmp(&effective_rating_for_assignment(right, *slot))
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .or_else(|| {
                available_players.iter()
                    .filter(|p| !used_ids.contains(&p.id))
                    .max_by(|left, right| {
                        effective_rating_for_assignment(left, *slot)
                            .partial_cmp(&effective_rating_for_assignment(right, *slot))
                            .unwrap_or(std::cmp::Ordering::Equal)
                    })
            });

        if let Some(player) = best_player {
            used_ids.insert(player.id.clone());
            starting_xi.push(to_engine_player_for_slot(*player, slot_group));
        } else {
            break;
        }
    }
}

fn to_engine_player_for_slot(
    p: &domain::player::Player,
    slot_group: DomainPosition,
) -> PlayerData {
    let pos = match slot_group {
        DomainPosition::Goalkeeper => EnginePosition::Goalkeeper,
        DomainPosition::Defender => EnginePosition::Defender,
        DomainPosition::Midfielder => EnginePosition::Midfielder,
        DomainPosition::Forward => EnginePosition::Forward,
        _ => EnginePosition::Midfielder,
    };
    log::debug!(
        "[team_builder] player {}: {} natural={:?} slot_group={:?} -> {:?}",
        p.id,
        p.match_name,
        p.position.to_group_position(),
        slot_group,
        pos
    );

    PlayerData {
        id: p.id.clone(),
        name: p.match_name.clone(),
        position: pos,
        condition: p.condition,
        fitness: p.fitness,
        pace: p.attributes.pace,
        stamina: p.attributes.stamina,
        strength: p.attributes.strength,
        agility: p.attributes.agility,
        passing: p.attributes.passing,
        shooting: p.attributes.shooting,
        tackling: p.attributes.tackling,
        dribbling: p.attributes.dribbling,
        defending: p.attributes.defending,
        positioning: p.attributes.positioning,
        vision: p.attributes.vision,
        decisions: p.attributes.decisions,
        composure: p.attributes.composure,
        aggression: p.attributes.aggression,
        teamwork: p.attributes.teamwork,
        leadership: p.attributes.leadership,
        handling: p.attributes.handling,
        reflexes: p.attributes.reflexes,
        aerial: p.attributes.aerial,
        traits: p.traits.iter().map(|t| format!("{:?}", t)).collect(),
    }
}

fn to_engine_player(p: &domain::player::Player) -> PlayerData {
    let pos = match p.position.to_group_position() {
        DomainPosition::Goalkeeper => EnginePosition::Goalkeeper,
        DomainPosition::Defender => EnginePosition::Defender,
        DomainPosition::Midfielder => EnginePosition::Midfielder,
        DomainPosition::Forward => EnginePosition::Forward,
        _ => EnginePosition::Midfielder,
    };
    log::debug!("[team_builder] player {}: {} -> {:?}", p.id, p.match_name, pos);

    PlayerData {
        id: p.id.clone(),
        name: p.match_name.clone(),
        position: pos,
        condition: p.condition,
        fitness: p.fitness,
        pace: p.attributes.pace,
        stamina: p.attributes.stamina,
        strength: p.attributes.strength,
        agility: p.attributes.agility,
        passing: p.attributes.passing,
        shooting: p.attributes.shooting,
        tackling: p.attributes.tackling,
        dribbling: p.attributes.dribbling,
        defending: p.attributes.defending,
        positioning: p.attributes.positioning,
        vision: p.attributes.vision,
        decisions: p.attributes.decisions,
        composure: p.attributes.composure,
        aggression: p.attributes.aggression,
        teamwork: p.attributes.teamwork,
        leadership: p.attributes.leadership,
        handling: p.attributes.handling,
        reflexes: p.attributes.reflexes,
        aerial: p.attributes.aerial,
        traits: p.traits.iter().map(|t| format!("{:?}", t)).collect(),
    }
}

/// Auto-select set-piece takers from a set of player IDs.
/// Returns (captain_id, penalty_taker_id, free_kick_taker_id, corner_taker_id).
pub fn auto_select_set_pieces(
    game: &Game,
    player_ids: &[String],
) -> (
    Option<String>,
    Option<String>,
    Option<String>,
    Option<String>,
) {
    let players: Vec<&domain::player::Player> = player_ids
        .iter()
        .filter_map(|id| game.players.iter().find(|p| &p.id == id))
        .collect();

    if players.is_empty() {
        return (None, None, None, None);
    }

    // Captain: highest leadership + teamwork
    let captain = players
        .iter()
        .max_by_key(|p| (p.attributes.leadership as u16) + (p.attributes.teamwork as u16))
        .map(|p| p.id.clone());

    // Penalty taker: highest shooting + composure (exclude GK)
    let penalty = players
        .iter()
        .filter(|p| p.position != DomainPosition::Goalkeeper)
        .max_by_key(|p| (p.attributes.shooting as u16) + (p.attributes.composure as u16))
        .map(|p| p.id.clone());

    // Free kick taker: highest passing + vision + shooting (exclude GK)
    let free_kick = players
        .iter()
        .filter(|p| p.position != DomainPosition::Goalkeeper)
        .max_by_key(|p| {
            (p.attributes.passing as u16)
                + (p.attributes.vision as u16)
                + (p.attributes.shooting as u16) / 2
        })
        .map(|p| p.id.clone());

    // Corner taker: highest passing + vision (exclude GK, prefer different from FK)
    let corner = players
        .iter()
        .filter(|p| p.position != DomainPosition::Goalkeeper)
        .max_by_key(|p| {
            let base = (p.attributes.passing as u16) + (p.attributes.vision as u16);
            // Small penalty if same as free kick taker to encourage variety
            if free_kick.as_ref() == Some(&p.id) {
                base.saturating_sub(5)
            } else {
                base
            }
        })
        .map(|p| p.id.clone());

    (captain, penalty, free_kick, corner)
}
