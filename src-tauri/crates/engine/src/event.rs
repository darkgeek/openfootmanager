use crate::types::{Side, Zone};
use serde::{Deserialize, Serialize};

/// A single event that occurred during the match.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MatchEvent {
    pub minute: u8,
    pub event_type: EventType,
    pub side: Side,
    pub zone: Zone,
    /// ID of the primary player involved (scorer, passer, fouler, etc.).
    pub player_id: Option<String>,
    /// ID of a secondary player (assist provider, fouled player, etc.).
    pub secondary_player_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum EventType {
    // --- Structural events ---
    KickOff,
    HalfTime,
    SecondHalfStart,
    FullTime,

    // --- Possession & passing ---
    PassCompleted,
    PassIntercepted,
    KeyPass,
    ThroughBall,

    // --- Attacking ---
    Dribble,
    DribbleTackled,
    Cross,
    CrossCompleted,
    CounterAttack,

    // --- Shooting ---
    ShotOnTarget,
    ShotOffTarget,
    ShotBlocked,
    ShotSaved,
    Goal,
    PenaltyAwarded,
    PenaltyGoal,
    PenaltyMiss,
    CloseCall,
    GreatChance,

    // --- Defending ---
    Tackle,
    Interception,
    Clearance,
    GoalkeeperPunch,
    Offside,

    // --- Fouls & discipline ---
    Foul,
    YellowCard,
    RedCard,
    SecondYellow,
    Diving,

    // --- Set pieces ---
    Corner,
    FreeKick,
    FreeKickShot,
    GoalKick,
    ThrowIn,

    // --- Atmosphere ---
    Atmosphere,
    Tension,
    Celebration,
    Applause,
    Chants,
    Groans,

    // --- Goalkeeper ---
    GreatSave,
    GoalkeeperCatch,

    // --- Other ---
    Injury,
    Substitution,
    TimeWasting,
}

impl MatchEvent {
    pub fn new(minute: u8, event_type: EventType, side: Side, zone: Zone) -> Self {
        Self {
            minute,
            event_type,
            side,
            zone,
            player_id: None,
            secondary_player_id: None,
        }
    }

    pub fn with_player(mut self, player_id: &str) -> Self {
        self.player_id = Some(player_id.to_string());
        self
    }

    pub fn with_secondary(mut self, player_id: &str) -> Self {
        self.secondary_player_id = Some(player_id.to_string());
        self
    }

    pub fn is_goal(&self) -> bool {
        matches!(self.event_type, EventType::Goal | EventType::PenaltyGoal)
    }
}

impl EventType {
    /// Returns true if this event type is important enough to show in key events
    pub fn is_key_event(&self) -> bool {
        matches!(
            self,
            EventType::Goal
                | EventType::PenaltyGoal
                | EventType::PenaltyMiss
                | EventType::YellowCard
                | EventType::RedCard
                | EventType::SecondYellow
                | EventType::Substitution
                | EventType::Injury
                | EventType::GreatChance
                | EventType::CloseCall
                | EventType::GreatSave
                | EventType::Diving
        )
    }

    /// Returns true if this event type is a shooting event
    pub fn is_shot(&self) -> bool {
        matches!(
            self,
            EventType::ShotOnTarget
                | EventType::ShotOffTarget
                | EventType::ShotBlocked
                | EventType::ShotSaved
                | EventType::CloseCall
                | EventType::GreatChance
        )
    }
}
