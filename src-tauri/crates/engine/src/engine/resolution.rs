use rand::{Rng, RngExt};

use crate::event::{ApplauseReason, EventType, MatchEvent};
use crate::shared::{PlayStylePhase, TraitContext, home_mod, play_style_modifier, trait_bonus};
use crate::types::{Position, Side, Zone};

use super::MatchContext;
use super::fouls::maybe_foul;
use super::snap_player;

// ---------------------------------------------------------------------------
// Action resolution per zone
// ---------------------------------------------------------------------------

pub(super) fn resolve_action<R: Rng>(ctx: &mut MatchContext, minute: u8, rng: &mut R) {
    let att_side = ctx.possession;
    let def_side = att_side.opposite();
    let zone = ctx.ball_zone;

    // Generate atmosphere events periodically
    if minute % 5 == 0 || rng.random_range(0.0..1.0f64) < 0.15 {
        generate_atmosphere_event(ctx, minute, rng);
    }

    if zone.is_box_for(att_side) {
        resolve_shot(ctx, minute, att_side, rng);
    } else if zone == Zone::attacking_third(att_side) {
        resolve_attacking_third(ctx, minute, att_side, def_side, rng);
    } else if zone == Zone::Midfield {
        resolve_midfield(ctx, minute, att_side, def_side, rng);
    } else {
        resolve_buildup(ctx, minute, att_side, def_side, rng);
    }
}

// ---------------------------------------------------------------------------
// Atmosphere events
// ---------------------------------------------------------------------------

fn generate_atmosphere_event<R: Rng>(ctx: &mut MatchContext, minute: u8, rng: &mut R) {
    let home_score = ctx.home_score();
    let away_score = ctx.away_score();
    let is_close_game = (home_score as i16 - away_score as i16).abs() <= 1;
    let is_goal_difference = home_score != away_score;

    // Check recent events for context
    let recent_events = ctx.recent_events(3);
    let had_shot = recent_events.iter().any(|e| e.event_type.is_shot());
    let had_tackle = recent_events.iter().any(|e| matches!(e.event_type, EventType::Tackle | EventType::Interception));

    // Determine atmosphere type
    let atm_type = if is_close_game && minute > 70 {
        // Tense end-game moment
        EventType::Tension
    } else if is_goal_difference && minute > 60 {
        // Leading team atmosphere
        if rng.random_range(0.0..1.0f64) < 0.5 {
            EventType::Chants
        } else {
            EventType::Applause
        }
    } else if had_shot {
        // Excitement after a shot
        EventType::Tension
    } else if had_tackle {
        // Appreciation for defensive effort
        EventType::Applause
    } else if minute > 80 && home_score == away_score {
        // Penalties anticipation
        EventType::Tension
    } else {
        // General crowd noise
        if rng.random_range(0.0..1.0f64) < 0.7 {
            EventType::Atmosphere
        } else if rng.random_range(0.0..1.0f64) < 0.5 {
            EventType::Chants
        } else {
            EventType::Atmosphere
        }
    };

    ctx.emit(MatchEvent::new(minute, atm_type, Side::Home, Zone::Midfield));
}

// ---------------------------------------------------------------------------
// Zone-specific resolution
// ---------------------------------------------------------------------------

fn resolve_buildup<R: Rng>(
    ctx: &mut MatchContext,
    minute: u8,
    att_side: Side,
    def_side: Side,
    rng: &mut R,
) {
    let passer = snap_player(ctx, att_side, Position::Defender, rng);
    let pass_skill = (passer.passing as f64
        + passer.vision as f64
        + passer.composure as f64
        + passer.teamwork as f64)
        / 4.0
        * trait_bonus(&passer, TraitContext::Passing);
    let press = effective_press(ctx, def_side);
    let ball_zone = ctx.ball_zone;

    let success_chance = (pass_skill * 1.3) / (pass_skill * 1.3 + press);
    if rng.random_range(0.0..1.0f64) < success_chance {
        // Check for key pass (long ball or through ball)
        if rng.random_range(0.0..1.0f64) < 0.15 {
            ctx.emit(
                MatchEvent::new(minute, EventType::ThroughBall, att_side, ball_zone)
                    .with_player(&passer.id),
            );
        } else {
            ctx.emit(
                MatchEvent::new(minute, EventType::PassCompleted, att_side, ball_zone)
                    .with_player(&passer.id),
            );
        }
        ctx.ball_zone = Zone::Midfield;

        // Counter-attack chance
        if rng.random_range(0.0..1.0f64) < 0.1 {
            ctx.emit(
                MatchEvent::new(minute, EventType::CounterAttack, att_side, ball_zone)
                    .with_player(&passer.id),
            );
        }
    } else {
        let interceptor = snap_player(ctx, def_side, Position::Midfielder, rng);
        ctx.emit(
            MatchEvent::new(minute, EventType::PassIntercepted, att_side, ball_zone)
                .with_player(&passer.id),
        );
        ctx.emit(
            MatchEvent::new(minute, EventType::Interception, def_side, ball_zone)
                .with_player(&interceptor.id),
        );
        // Appreciation for the interception
        ctx.emit(
            MatchEvent::new(minute, EventType::Applause, def_side, ball_zone)
                .with_player(&interceptor.id)
                .with_applause_reason(ApplauseReason::Interception),
        );
        ctx.possession = def_side;
    }
}

fn resolve_midfield<R: Rng>(
    ctx: &mut MatchContext,
    minute: u8,
    att_side: Side,
    def_side: Side,
    rng: &mut R,
) {
    let attacker = snap_player(ctx, att_side, Position::Midfielder, rng);
    let defender = snap_player(ctx, def_side, Position::Midfielder, rng);

    let att_rating = (attacker.dribbling as f64
        + attacker.passing as f64
        + attacker.vision as f64
        + attacker.teamwork as f64)
        / 4.0
        * trait_bonus(&attacker, TraitContext::Midfield);
    let def_rating = (defender.tackling as f64
        + defender.positioning as f64
        + defender.decisions as f64
        + defender.teamwork as f64)
        / 4.0
        * trait_bonus(&defender, TraitContext::Tackling);

    let att_mod = play_style_modifier(
        ctx.team(att_side).play_style,
        PlayStylePhase::Midfield,
        true,
    );
    let def_mod = play_style_modifier(
        ctx.team(def_side).play_style,
        PlayStylePhase::Midfield,
        false,
    );
    let att_eff = att_rating * att_mod * home_mod(att_side, ctx.config);
    let def_eff = def_rating * def_mod * home_mod(def_side, ctx.config);
    let success = att_eff / (att_eff + def_eff);

    if rng.random_range(0.0..1.0f64) < success {
        // Check for key pass (vision-based playmaking)
        if attacker.vision > 75 && rng.random_range(0.0..1.0f64) < 0.25 {
            ctx.emit(
                MatchEvent::new(minute, EventType::KeyPass, att_side, Zone::Midfield)
                    .with_player(&attacker.id),
            );
        } else {
            ctx.emit(
                MatchEvent::new(minute, EventType::PassCompleted, att_side, Zone::Midfield)
                    .with_player(&attacker.id),
            );
        }

        // Counter-attack opportunity
        if rng.random_range(0.0..1.0f64) < 0.12 {
            ctx.emit(
                MatchEvent::new(minute, EventType::CounterAttack, att_side, Zone::Midfield)
                    .with_player(&attacker.id),
            );
        }

        ctx.ball_zone = Zone::attacking_third(att_side);
    } else {
        if rng.random_range(0.0..1.0f64) < 0.6 {
            ctx.emit(
                MatchEvent::new(minute, EventType::Tackle, def_side, Zone::Midfield)
                    .with_player(&defender.id),
            );
            ctx.emit(
                MatchEvent::new(minute, EventType::Applause, def_side, Zone::Midfield)
                    .with_player(&defender.id)
                    .with_applause_reason(ApplauseReason::Tackle),
            );
            maybe_foul(
                ctx,
                minute,
                def_side,
                &attacker,
                &defender,
                Zone::Midfield,
                rng,
            );
        } else {
            ctx.emit(
                MatchEvent::new(minute, EventType::Interception, def_side, Zone::Midfield)
                    .with_player(&defender.id),
            );
        }
        ctx.possession = def_side;
        ctx.ball_zone = Zone::Midfield;
    }
}

fn resolve_attacking_third<R: Rng>(
    ctx: &mut MatchContext,
    minute: u8,
    att_side: Side,
    def_side: Side,
    rng: &mut R,
) {
    let attacker = snap_player(ctx, att_side, Position::Forward, rng);
    let defender = snap_player(ctx, def_side, Position::Defender, rng);

    let att_rating = (attacker.dribbling as f64
        + attacker.pace as f64
        + attacker.agility as f64
        + attacker.composure as f64)
        / 4.0
        * trait_bonus(&attacker, TraitContext::Dribbling);
    let def_rating = (defender.defending as f64
        + defender.tackling as f64
        + defender.positioning as f64
        + defender.aerial as f64)
        / 4.0
        * trait_bonus(&defender, TraitContext::Tackling);

    let att_mod = play_style_modifier(ctx.team(att_side).play_style, PlayStylePhase::Attack, true);
    let def_mod = play_style_modifier(
        ctx.team(def_side).play_style,
        PlayStylePhase::Defense,
        false,
    );
    let att_eff = att_rating * att_mod * home_mod(att_side, ctx.config);
    let def_eff = def_rating * def_mod * home_mod(def_side, ctx.config);
    let success = att_eff / (att_eff + def_eff);
    let zone = Zone::attacking_third(att_side);

    if rng.random_range(0.0..1.0f64) < success {
        ctx.emit(
            MatchEvent::new(minute, EventType::Dribble, att_side, zone).with_player(&attacker.id),
        );

        // Try cross or continue dribbling into box
        if attacker.passing > 70 && rng.random_range(0.0..1.0f64) < 0.4 {
            if rng.random_range(0.0..1.0f64) < 0.5 {
                ctx.emit(
                    MatchEvent::new(minute, EventType::Cross, att_side, zone)
                        .with_player(&attacker.id),
                );
            } else {
                ctx.emit(
                    MatchEvent::new(minute, EventType::CrossCompleted, att_side, zone)
                        .with_player(&attacker.id),
                );
            }
        }

        ctx.ball_zone = Zone::attacking_box(att_side);
    } else {
        let is_tackle = rng.random_range(0.0..1.0f64) < 0.5;
        if is_tackle {
            ctx.emit(
                MatchEvent::new(minute, EventType::DribbleTackled, att_side, zone)
                    .with_player(&attacker.id)
                    .with_secondary(&defender.id),
            );
            ctx.emit(
                MatchEvent::new(minute, EventType::Tackle, def_side, zone)
                    .with_player(&defender.id),
            );
            ctx.emit(
                MatchEvent::new(minute, EventType::Applause, def_side, zone)
                    .with_player(&defender.id)
                    .with_applause_reason(ApplauseReason::Tackle),
            );
            maybe_foul(ctx, minute, def_side, &attacker, &defender, zone, rng);
        } else {
            ctx.emit(
                MatchEvent::new(minute, EventType::Clearance, def_side, zone)
                    .with_player(&defender.id),
            );
        }

        // Corner or offside trap
        if rng.random_range(0.0..1.0f64) < 0.2 {
            ctx.emit(MatchEvent::new(minute, EventType::Corner, att_side, zone));
            if rng.random_range(0.0..1.0f64) < 0.30 {
                ctx.ball_zone = Zone::attacking_box(att_side);
                return;
            }
        } else if rng.random_range(0.0..1.0f64) < 0.1 {
            // Offside trap
            ctx.emit(MatchEvent::new(minute, EventType::Offside, att_side, zone));
            ctx.emit(
                MatchEvent::new(minute, EventType::Applause, def_side, zone)
                    .with_applause_reason(ApplauseReason::General),
            );
        }

        ctx.possession = def_side;
        ctx.ball_zone = Zone::defensive_third(att_side);
    }
}

fn resolve_shot<R: Rng>(ctx: &mut MatchContext, minute: u8, att_side: Side, rng: &mut R) {
    let def_side = att_side.opposite();
    let shooter = snap_player(ctx, att_side, Position::Forward, rng);
    let assister = snap_player(ctx, att_side, Position::Midfielder, rng);
    let goalkeeper = snap_player(ctx, def_side, Position::Goalkeeper, rng);
    let zone = Zone::attacking_box(att_side);

    let shoot_rating =
        (shooter.shooting as f64 + shooter.composure as f64 + shooter.decisions as f64) / 3.0
            * trait_bonus(&shooter, TraitContext::Shooting);
    let gk_rating =
        (goalkeeper.handling as f64 + goalkeeper.reflexes as f64 + goalkeeper.positioning as f64)
            / 3.0
            * trait_bonus(&goalkeeper, TraitContext::Goalkeeping);

    // Determine shot type and accuracy
    let accuracy =
        (ctx.config.shot_accuracy_base + (shoot_rating - 50.0) / 200.0).clamp(0.15, 0.85);

    let shot_roll = rng.random_range(0.0..1.0f64);

    // Shot misses the target
    if shot_roll > accuracy {
        if rng.random_range(0.0..1.0f64) < 0.3 {
            // Close call - nearly scored
            ctx.emit(
                MatchEvent::new(minute, EventType::CloseCall, att_side, zone)
                    .with_player(&shooter.id),
            );
            ctx.emit(
                MatchEvent::new(minute, EventType::Tension, att_side, zone)
                    .with_player(&shooter.id),
            );
        } else if rng.random_range(0.0..1.0f64) < 0.4 {
            ctx.emit(
                MatchEvent::new(minute, EventType::ShotBlocked, att_side, zone)
                    .with_player(&shooter.id),
            );
        } else {
            ctx.emit(
                MatchEvent::new(minute, EventType::ShotOffTarget, att_side, zone)
                    .with_player(&shooter.id),
            );
            ctx.emit(MatchEvent::new(minute, EventType::Groans, def_side, zone));
        }
        ctx.possession = def_side;
        return;
    }

    // Shot is on target
    let conversion =
        (ctx.config.goal_conversion_base + (shoot_rating - gk_rating) / 150.0).clamp(0.10, 0.70);
    let goal_roll = rng.random_range(0.0..1.0f64);

    // Great chance - high quality shot setup
    if shoot_rating > 80.0 && goal_roll < conversion * 1.2 {
        ctx.emit(
            MatchEvent::new(minute, EventType::GreatChance, att_side, zone)
                .with_player(&shooter.id)
                .with_secondary(&assister.id),
        );
    } else {
        // Regular on-target shot
        ctx.emit(
            MatchEvent::new(minute, EventType::ShotOnTarget, att_side, zone)
                .with_player(&shooter.id),
        );
    }

    // Goal or saved
    if goal_roll < conversion {
        ctx.emit(
            MatchEvent::new(minute, EventType::Goal, att_side, zone)
                .with_player(&shooter.id)
                .with_secondary(&assister.id),
        );
        ctx.emit(
            MatchEvent::new(minute, EventType::Celebration, att_side, zone)
                .with_player(&shooter.id),
        );
        ctx.add_goal(att_side);
    } else {
        // Check for great save vs regular save
        let gk_quality = (goalkeeper.reflexes as f64 + goalkeeper.handling as f64) / 2.0;
        if gk_quality > 75.0 && rng.random_range(0.0..1.0f64) < 0.5 {
            ctx.emit(
                MatchEvent::new(minute, EventType::GreatSave, def_side, zone)
                    .with_player(&goalkeeper.id),
            );
            ctx.emit(
                MatchEvent::new(minute, EventType::Applause, def_side, zone)
                    .with_player(&goalkeeper.id)
                    .with_applause_reason(ApplauseReason::Save),
            );
        } else {
            ctx.emit(
                MatchEvent::new(minute, EventType::ShotSaved, att_side, zone)
                    .with_player(&shooter.id),
            );
        }

        // Goalkeeper punch or catch on corner
        if rng.random_range(0.0..1.0f64) < 0.2 {
            ctx.emit(
                MatchEvent::new(minute, EventType::GoalkeeperPunch, def_side, zone)
                    .with_player(&goalkeeper.id),
            );
        } else {
            ctx.emit(
                MatchEvent::new(minute, EventType::GoalkeeperCatch, def_side, zone)
                    .with_player(&goalkeeper.id),
            );
        }
    }

    ctx.possession = def_side;
}

// ---------------------------------------------------------------------------
// Rating helpers
// ---------------------------------------------------------------------------

pub(super) fn effective_midfield(ctx: &MatchContext, side: Side) -> f64 {
    let base = ctx.team(side).midfield_rating();
    let modifier = play_style_modifier(ctx.team(side).play_style, PlayStylePhase::Midfield, true);
    base * modifier * home_mod(side, ctx.config)
}

fn effective_press(ctx: &MatchContext, pressing_side: Side) -> f64 {
    let team = ctx.team(pressing_side);
    let base = team.position_attr_avg(Position::Midfielder, |p| {
        ((p.stamina as u16 + p.tackling as u16 + p.pace as u16) / 3) as u8
    });
    let modifier = play_style_modifier(team.play_style, PlayStylePhase::Press, true);
    base * modifier * home_mod(pressing_side, ctx.config)
}
