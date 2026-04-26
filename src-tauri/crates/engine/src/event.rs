use crate::types::{Side, Zone};
use serde::{Deserialize, Serialize};

/// Reason for applause events
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ApplauseReason {
    /// Applause for a successful tackle
    Tackle,
    /// Applause for a successful interception
    Interception,
    /// Applause for a clearance
    Clearance,
    /// Applause for a goalkeeper save
    Save,
    /// Applause for a great save specifically
    GreatSave,
    /// General crowd appreciation (no specific action)
    General,
}

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
    /// Reason for applause events
    pub applause_reason: Option<ApplauseReason>,
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
            applause_reason: None,
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

    pub fn with_applause_reason(mut self, reason: ApplauseReason) -> Self {
        self.applause_reason = Some(reason);
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
