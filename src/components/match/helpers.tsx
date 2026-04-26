import React from "react";
import { MatchEvent, MatchSnapshot, ApplauseReason } from "./types";
import type { FixtureData, GameStateData } from "../../store/gameStore";
import {
  Circle,
  CircleOff,
  Square,
  ArrowLeftRight,
  Cross,
  Play,
  Pause,
  Flag,
  Hand,
  ArrowUpRight,
  Shield,
  CornerDownRight,
  Ruler,
  AlertTriangle,
  Zap,
  CircleDot,
  Target,
  Sparkles,
  ThumbsUp,
  Volume2,
  Megaphone,
  Frown,
  Save,
  HandMetal,
  FlagOff,
  Footprints,
  Swords,
  Eye,
  TrendingUp,
  Zap as ZapIcon,
  Trophy,
} from "lucide-react";

// Event category for visual styling
export type EventCategory = "goal" | "card" | "substitution" | "shot" | "foul" | "defensive" | "passing" | "atmosphere" | "structural" | "other";

interface EventDisplayConfig {
  icon: React.ReactNode;
  color: string;
  bgColor: string;
  borderColor: string;
  important: boolean;
  category: EventCategory;
  label?: string;
}

export const EVENT_ICONS: Record<string, EventDisplayConfig> = {
  Goal: {
    icon: <Trophy className="w-4 h-4" />,
    color: "text-yellow-600 dark:text-yellow-400",
    bgColor: "bg-yellow-50 dark:bg-yellow-900/30",
    borderColor: "border-yellow-300 dark:border-yellow-700",
    important: true,
    category: "goal",
    label: "Goal!",
  },
  PenaltyGoal: {
    icon: <CircleDot className="w-4 h-4" />,
    color: "text-yellow-600 dark:text-yellow-400",
    bgColor: "bg-yellow-50 dark:bg-yellow-900/30",
    borderColor: "border-yellow-300 dark:border-yellow-700",
    important: true,
    category: "goal",
    label: "Penalty Goal!",
  },
  PenaltyMiss: {
    icon: <CircleOff className="w-4 h-4" />,
    color: "text-red-500",
    bgColor: "bg-red-50 dark:bg-red-900/30",
    borderColor: "border-red-300 dark:border-red-700",
    important: true,
    category: "goal",
    label: "Penalty Miss!",
  },
  YellowCard: {
    icon: <Square className="w-3.5 h-3.5 fill-yellow-400 text-yellow-400" />,
    color: "text-yellow-500",
    bgColor: "bg-yellow-50 dark:bg-yellow-900/20",
    borderColor: "border-yellow-300 dark:border-yellow-700",
    important: true,
    category: "card",
    label: "Yellow Card",
  },
  RedCard: {
    icon: <Square className="w-3.5 h-3.5 fill-red-500 text-red-500" />,
    color: "text-red-500",
    bgColor: "bg-red-50 dark:bg-red-900/30",
    borderColor: "border-red-400 dark:border-red-700",
    important: true,
    category: "card",
    label: "Red Card",
  },
  SecondYellow: {
    icon: <Square className="w-3.5 h-3.5 fill-red-500 text-red-500" />,
    color: "text-red-500",
    bgColor: "bg-red-50 dark:bg-red-900/30",
    borderColor: "border-red-400 dark:border-red-700",
    important: true,
    category: "card",
    label: "Second Yellow → Red",
  },
  Substitution: {
    icon: <ArrowLeftRight className="w-4 h-4" />,
    color: "text-blue-500",
    bgColor: "bg-blue-50 dark:bg-blue-900/20",
    borderColor: "border-blue-300 dark:border-blue-700",
    important: true,
    category: "substitution",
    label: "Substitution",
  },
  Injury: {
    icon: <Cross className="w-4 h-4" />,
    color: "text-orange-500",
    bgColor: "bg-orange-50 dark:bg-orange-900/30",
    borderColor: "border-orange-300 dark:border-orange-700",
    important: true,
    category: "other",
    label: "Injury",
  },
  KickOff: {
    icon: <Play className="w-3.5 h-3.5 fill-current" />,
    color: "text-emerald-600 dark:text-emerald-400",
    bgColor: "bg-emerald-50 dark:bg-emerald-900/20",
    borderColor: "border-emerald-300 dark:border-emerald-700",
    important: true,
    category: "structural",
    label: "Kick Off",
  },
  HalfTime: {
    icon: <Pause className="w-3.5 h-3.5" />,
    color: "text-gray-600 dark:text-gray-400",
    bgColor: "bg-gray-100 dark:bg-navy-800",
    borderColor: "border-gray-300 dark:border-navy-600",
    important: true,
    category: "structural",
    label: "Half Time",
  },
  SecondHalfStart: {
    icon: <Play className="w-3.5 h-3.5 fill-current" />,
    color: "text-emerald-600 dark:text-emerald-400",
    bgColor: "bg-emerald-50 dark:bg-emerald-900/20",
    borderColor: "border-emerald-300 dark:border-emerald-700",
    important: true,
    category: "structural",
    label: "Second Half",
  },
  FullTime: {
    icon: <Flag className="w-4 h-4" />,
    color: "text-gray-600 dark:text-gray-400",
    bgColor: "bg-gray-100 dark:bg-navy-800",
    borderColor: "border-gray-300 dark:border-navy-600",
    important: true,
    category: "structural",
    label: "Full Time",
  },
  ShotSaved: {
    icon: <Hand className="w-4 h-4" />,
    color: "text-cyan-600 dark:text-cyan-400",
    bgColor: "bg-cyan-50 dark:bg-cyan-900/20",
    borderColor: "border-cyan-300 dark:border-cyan-700",
    important: false,
    category: "shot",
    label: "Shot Saved",
  },
  ShotOffTarget: {
    icon: <ArrowUpRight className="w-4 h-4" />,
    color: "text-gray-500 dark:text-gray-500",
    bgColor: "bg-gray-50 dark:bg-navy-800/50",
    borderColor: "border-gray-200 dark:border-navy-700",
    important: false,
    category: "shot",
    label: "Off Target",
  },
  ShotBlocked: {
    icon: <Shield className="w-4 h-4" />,
    color: "text-slate-600 dark:text-slate-400",
    bgColor: "bg-slate-50 dark:bg-slate-800/50",
    borderColor: "border-slate-300 dark:border-slate-700",
    important: false,
    category: "shot",
    label: "Shot Blocked",
  },
  Corner: {
    icon: <CornerDownRight className="w-4 h-4" />,
    color: "text-amber-600 dark:text-amber-400",
    bgColor: "bg-amber-50 dark:bg-amber-900/20",
    borderColor: "border-amber-300 dark:border-amber-700",
    important: false,
    category: "other",
    label: "Corner",
  },
  FreeKick: {
    icon: <Ruler className="w-4 h-4" />,
    color: "text-amber-600 dark:text-amber-400",
    bgColor: "bg-amber-50 dark:bg-amber-900/20",
    borderColor: "border-amber-300 dark:border-amber-700",
    important: false,
    category: "other",
    label: "Free Kick",
  },
  Foul: {
    icon: <AlertTriangle className="w-4 h-4" />,
    color: "text-amber-600 dark:text-amber-400",
    bgColor: "bg-amber-50 dark:bg-amber-900/20",
    borderColor: "border-amber-300 dark:border-amber-700",
    important: false,
    category: "foul",
    label: "Foul",
  },
  PenaltyAwarded: {
    icon: <Zap className="w-4 h-4" />,
    color: "text-orange-500",
    bgColor: "bg-orange-50 dark:bg-orange-900/30",
    borderColor: "border-orange-300 dark:border-orange-700",
    important: true,
    category: "goal",
    label: "Penalty Awarded!",
  },
  GreatChance: {
    icon: <Target className="w-4 h-4" />,
    color: "text-orange-500",
    bgColor: "bg-orange-50 dark:bg-orange-900/30",
    borderColor: "border-orange-300 dark:border-orange-700",
    important: true,
    category: "shot",
    label: "Great Chance!",
  },
  CloseCall: {
    icon: <Eye className="w-4 h-4" />,
    color: "text-orange-400",
    bgColor: "bg-orange-50/50 dark:bg-orange-900/20",
    borderColor: "border-orange-200 dark:border-orange-800",
    important: true,
    category: "shot",
    label: "Close Call!",
  },
  GreatSave: {
    icon: <Save className="w-4 h-4" />,
    color: "text-emerald-600 dark:text-emerald-400",
    bgColor: "bg-emerald-50 dark:bg-emerald-900/20",
    borderColor: "border-emerald-300 dark:border-emerald-700",
    important: true,
    category: "defensive",
    label: "Great Save!",
  },
  Celebration: {
    icon: <Sparkles className="w-4 h-4" />,
    color: "text-yellow-500",
    bgColor: "bg-yellow-50/50 dark:bg-yellow-900/20",
    borderColor: "border-yellow-200 dark:border-yellow-800",
    important: true,
    category: "goal",
    label: "Celebration",
  },
  Tension: {
    icon: <AlertTriangle className="w-4 h-4" />,
    color: "text-orange-400",
    bgColor: "bg-orange-50/50 dark:bg-orange-900/20",
    borderColor: "border-orange-200 dark:border-orange-800",
    important: true,
    category: "atmosphere",
    label: "Tension Rising",
  },
  Diving: {
    icon: <AlertTriangle className="w-4 h-4" />,
    color: "text-red-400",
    bgColor: "bg-red-50/50 dark:bg-red-900/20",
    borderColor: "border-red-200 dark:border-red-800",
    important: true,
    category: "foul",
    label: "Simulation!",
  },
  ShotOnTarget: {
    icon: <Target className="w-4 h-4" />,
    color: "text-cyan-600 dark:text-cyan-400",
    bgColor: "bg-cyan-50 dark:bg-cyan-900/20",
    borderColor: "border-cyan-300 dark:border-cyan-700",
    important: false,
    category: "shot",
    label: "On Target",
  },
  Applause: {
    icon: <ThumbsUp className="w-4 h-4" />,
    color: "text-emerald-600 dark:text-emerald-400",
    bgColor: "bg-emerald-50/50 dark:bg-emerald-900/20",
    borderColor: "border-emerald-200 dark:border-emerald-800",
    important: false,
    category: "atmosphere",
  },
  Chants: {
    icon: <Megaphone className="w-4 h-4" />,
    color: "text-blue-500",
    bgColor: "bg-blue-50/50 dark:bg-blue-900/20",
    borderColor: "border-blue-200 dark:border-blue-800",
    important: true,
    category: "atmosphere",
    label: "Chants",
  },
  Groans: {
    icon: <Frown className="w-4 h-4" />,
    color: "text-gray-500",
    bgColor: "bg-gray-50/50 dark:bg-gray-800/50",
    borderColor: "border-gray-200 dark:border-gray-700",
    important: true,
    category: "atmosphere",
    label: "Groans",
  },
  Atmosphere: {
    icon: <Volume2 className="w-4 h-4" />,
    color: "text-gray-500",
    bgColor: "bg-gray-50/30 dark:bg-navy-800/30",
    borderColor: "border-gray-200 dark:border-navy-700",
    important: false,
    category: "atmosphere",
    label: "Crowd Noise",
  },
  GoalkeeperPunch: {
    icon: <HandMetal className="w-4 h-4" />,
    color: "text-slate-600 dark:text-slate-400",
    bgColor: "bg-slate-50 dark:bg-slate-800/50",
    borderColor: "border-slate-300 dark:border-slate-700",
    important: false,
    category: "defensive",
    label: "GK Punch",
  },
  GoalkeeperCatch: {
    icon: <Hand className="w-4 h-4" />,
    color: "text-slate-600 dark:text-slate-400",
    bgColor: "bg-slate-50 dark:bg-slate-800/50",
    borderColor: "border-slate-300 dark:border-slate-700",
    important: false,
    category: "defensive",
    label: "GK Catch",
  },
  Offside: {
    icon: <FlagOff className="w-4 h-4" />,
    color: "text-gray-500 dark:text-gray-500",
    bgColor: "bg-gray-50/50 dark:bg-gray-800/50",
    borderColor: "border-gray-200 dark:border-gray-700",
    important: false,
    category: "other",
    label: "Offside",
  },
  Dribble: {
    icon: <Footprints className="w-4 h-4" />,
    color: "text-violet-500",
    bgColor: "bg-violet-50 dark:bg-violet-900/20",
    borderColor: "border-violet-300 dark:border-violet-700",
    important: false,
    category: "passing",
    label: "Dribble",
  },
  DribbleTackled: {
    icon: <Swords className="w-4 h-4" />,
    color: "text-rose-500",
    bgColor: "bg-rose-50 dark:bg-rose-900/20",
    borderColor: "border-rose-300 dark:border-rose-700",
    important: false,
    category: "defensive",
    label: "Dribble Lost",
  },
  PassCompleted: {
    icon: <ArrowLeftRight className="w-3.5 h-3.5" />,
    color: "text-blue-500 dark:text-blue-400",
    bgColor: "bg-blue-50/50 dark:bg-blue-900/20",
    borderColor: "border-blue-200 dark:border-blue-800",
    important: false,
    category: "passing",
    label: "Pass",
  },
  PassIntercepted: {
    icon: <Hand className="w-4 h-4" />,
    color: "text-amber-500 dark:text-amber-400",
    bgColor: "bg-amber-50/50 dark:bg-amber-900/20",
    borderColor: "border-amber-200 dark:border-amber-800",
    important: false,
    category: "defensive",
    label: "Intercepted",
  },
  KeyPass: {
    icon: <Eye className="w-4 h-4" />,
    color: "text-purple-500",
    bgColor: "bg-purple-50 dark:bg-purple-900/20",
    borderColor: "border-purple-300 dark:border-purple-700",
    important: true,
    category: "passing",
    label: "Key Pass!",
  },
  ThroughBall: {
    icon: <TrendingUp className="w-4 h-4" />,
    color: "text-pink-500",
    bgColor: "bg-pink-50 dark:bg-pink-900/20",
    borderColor: "border-pink-300 dark:border-pink-700",
    important: true,
    category: "passing",
    label: "Through Ball!",
  },
  Cross: {
    icon: <ArrowUpRight className="w-4 h-4" />,
    color: "text-cyan-500",
    bgColor: "bg-cyan-50 dark:bg-cyan-900/20",
    borderColor: "border-cyan-300 dark:border-cyan-700",
    important: false,
    category: "passing",
    label: "Cross",
  },
  CrossCompleted: {
    icon: <ArrowUpRight className="w-4 h-4" />,
    color: "text-emerald-500",
    bgColor: "bg-emerald-50 dark:bg-emerald-900/20",
    borderColor: "border-emerald-300 dark:border-emerald-700",
    important: false,
    category: "passing",
    label: "Cross Connected",
  },
  CounterAttack: {
    icon: <ZapIcon className="w-4 h-4" />,
    color: "text-red-500",
    bgColor: "bg-red-50 dark:bg-red-900/20",
    borderColor: "border-red-300 dark:border-red-700",
    important: true,
    category: "passing",
    label: "Counter Attack!",
  },
  Tackle: {
    icon: <Shield className="w-4 h-4" />,
    color: "text-emerald-600 dark:text-emerald-400",
    bgColor: "bg-emerald-50 dark:bg-emerald-900/20",
    borderColor: "border-emerald-300 dark:border-emerald-700",
    important: false,
    category: "defensive",
    label: "Tackle",
  },
  Interception: {
    icon: <Hand className="w-4 h-4" />,
    color: "text-cyan-600 dark:text-cyan-400",
    bgColor: "bg-cyan-50 dark:bg-cyan-900/20",
    borderColor: "border-cyan-300 dark:border-cyan-700",
    important: false,
    category: "defensive",
    label: "Interception",
  },
  Clearance: {
    icon: <Shield className="w-4 h-4" />,
    color: "text-slate-500 dark:text-slate-400",
    bgColor: "bg-slate-50 dark:bg-slate-800/50",
    borderColor: "border-slate-200 dark:border-slate-700",
    important: false,
    category: "defensive",
    label: "Clearance",
  },
  FreeKickShot: {
    icon: <Target className="w-4 h-4" />,
    color: "text-amber-500",
    bgColor: "bg-amber-50 dark:bg-amber-900/20",
    borderColor: "border-amber-300 dark:border-amber-700",
    important: false,
    category: "shot",
    label: "Free Kick Shot",
  },
  GoalKick: {
    icon: <Flag className="w-4 h-4" />,
    color: "text-gray-500",
    bgColor: "bg-gray-50/50 dark:bg-gray-800/50",
    borderColor: "border-gray-200 dark:border-gray-700",
    important: false,
    category: "other",
    label: "Goal Kick",
  },
  ThrowIn: {
    icon: <Flag className="w-4 h-4" />,
    color: "text-gray-500",
    bgColor: "bg-gray-50/50 dark:bg-gray-800/50",
    borderColor: "border-gray-200 dark:border-gray-700",
    important: false,
    category: "other",
    label: "Throw In",
  },
};

const DEFAULT_DISPLAY: EventDisplayConfig = {
  icon: <Circle className="w-3 h-3" />,
  color: "text-gray-700 dark:text-gray-400",
  bgColor: "bg-gray-50/50 dark:bg-navy-800/50",
  borderColor: "border-gray-200 dark:border-navy-700",
  important: false,
  category: "other",
  label: "Event",
};

export function getEventDisplay(evt: MatchEvent): EventDisplayConfig {
  return EVENT_ICONS[evt.event_type] || DEFAULT_DISPLAY;
}

/**
 * Generate a descriptive text for an event, including player names and context
 */
export function getEventDescription(
  evt: MatchEvent,
  snapshot: MatchSnapshot,
): string {
  const playerName = evt.player_id ? getPlayerName(snapshot, evt.player_id) : null;
  const secondaryName = evt.secondary_player_id ? getPlayerName(snapshot, evt.secondary_player_id) : null;
  
  const zoneName = getZoneName(evt.zone);
  
  switch (evt.event_type) {
    case "Goal":
      if (secondaryName) {
        return `${playerName} scores! Assisted by ${secondaryName}`;
      }
      return `${playerName} scores!`;
    
    case "PenaltyGoal":
      if (secondaryName) {
        return `${playerName} converts the penalty! Assisted by ${secondaryName}`;
      }
      return `${playerName} converts the penalty!`;
    
    case "PenaltyMiss":
      return `${playerName} misses the penalty!`;
    
    case "GreatChance":
      if (secondaryName && secondaryName !== playerName) {
        return `${playerName} with a great chance! Played through by ${secondaryName}`;
      }
      return `${playerName} creates a great chance!`;
    
    case "ShotSaved":
      return `${playerName}'s shot is saved`;
    
    case "ShotOffTarget":
      return `${playerName} fires wide`;
    
    case "ShotBlocked":
      return `${playerName}'s shot is blocked`;
    
    case "ShotOnTarget":
      return `${playerName} shoots on target`;
    
    case "CloseCall":
      return `${playerName} goes close!`;
    
    case "GreatSave":
      return `${playerName} with a brilliant save!`;
    
    case "GoalkeeperPunch":
      return `${playerName} punches the ball away`;
    
    case "GoalkeeperCatch":
      return `${playerName} catches the ball safely`;
    
    case "Dribble":
      return `${playerName} dribbles past a defender`;
    
    case "DribbleTackled":
      if (secondaryName) {
        return `${playerName} loses the ball to ${secondaryName}`;
      }
      return `${playerName} loses the ball`;
    
    case "PassCompleted":
      return `${playerName} completes a pass${zoneName ? ` in ${zoneName}` : ""}`;
    
    case "PassIntercepted":
      if (secondaryName) {
        return `${playerName}'s pass intercepted by ${secondaryName}`;
      }
      return `Pass intercepted`;
    
    case "KeyPass":
      return `${playerName} plays a key pass!`;
    
    case "ThroughBall":
      return `${playerName} plays a through ball!`;
    
    case "Cross":
      return `${playerName} delivers a cross`;
    
    case "CrossCompleted":
      return `${playerName}'s cross finds a teammate`;
    
    case "CounterAttack":
      return `${playerName} starts a counter attack!`;
    
    case "Tackle":
      return `${playerName} wins the ball`;
    
    case "Interception":
      return `${playerName} intercepts the pass`;
    
    case "Clearance":
      return `${playerName} clears the ball`;
    
    case "Foul":
      if (secondaryName) {
        return `${playerName} commits a foul on ${secondaryName}`;
      }
      return `${playerName} commits a foul`;
    
    case "YellowCard":
      return `${playerName} is shown a yellow card`;
    
    case "RedCard":
      return `${playerName} is sent off!`;
    
    case "SecondYellow":
      return `${playerName} receives a second yellow - RED CARD!`;
    
    case "Diving":
      return `${playerName} is booked for simulation!`;
    
    case "Injury":
      return `${playerName} is down injured`;
    
    case "Corner":
      return `${playerName} takes the corner`;
    
    case "FreeKick":
      return `${playerName} takes the free kick`;
    
    case "FreeKickShot":
      return `${playerName} shoots from the free kick`;
    
    case "PenaltyAwarded":
      return `Penalty awarded!`;
    
    case "Offside":
      return `${playerName} is caught offside`;
    
    case "GoalKick":
      return `Goal kick`;
    
    case "ThrowIn":
      return `Throw in`;
    
    case "Substitution":
      if (secondaryName) {
        return `Off: ${playerName} → On: ${secondaryName}`;
      }
      return `${playerName} comes on`;
    
    case "Tension":
      return `The atmosphere grows tense`;
    
    case "Celebration":
      if (playerName) {
        return `${playerName} celebrates!`;
      }
      return `Goal celebration!`;
    
    case "Applause":
      // Applause reason indicates why the crowd is applauding
      return getApplauseDescription(evt.applause_reason, playerName);
    
    case "Chants":
      return `The fans are chanting`;
    
    case "Groans":
      return `The crowd groans`;
    
    case "Atmosphere":
      return `The home crowd is making some noise`;
    
    case "KickOff":
      return `The match kicks off!`;
    
    case "HalfTime":
      return `Half time`;
    
    case "SecondHalfStart":
      return `Second half begins`;
    
    case "FullTime":
      return `Full time - match over`;
    
    default:
      return evt.event_type.replace(/([A-Z])/g, " $1").trim();
  }
}

function getZoneName(zone: string): string | null {
  const zoneMap: Record<string, string> = {
    HomeBox: "the home penalty area",
    AwayBox: "the away penalty area",
    HomeDefense: "home defensive third",
    AwayDefense: "away defensive third",
    Midfield: "midfield",
    HomeAttack: "home attacking third",
    AwayAttack: "away attacking third",
  };
  return zoneMap[zone] || null;
}

export function getPlayerName(
  snapshot: MatchSnapshot,
  playerId: string | null,
): string {
  if (!playerId) return "";
  for (const p of snapshot.home_team.players) {
    if (p.id === playerId) return p.name;
  }
  for (const p of snapshot.away_team.players) {
    if (p.id === playerId) return p.name;
  }
  // Also check bench players
  if (snapshot.home_bench) {
    for (const p of snapshot.home_bench) {
      if (p.id === playerId) return p.name;
    }
  }
  if (snapshot.away_bench) {
    for (const p of snapshot.away_bench) {
      if (p.id === playerId) return p.name;
    }
  }
  return playerId;
}

/**
 * Get player position from snapshot
 */
export function getPlayerPosition(
  snapshot: MatchSnapshot,
  playerId: string | null,
): string {
  if (!playerId) return "";
  for (const p of snapshot.home_team.players) {
    if (p.id === playerId) return p.position;
  }
  for (const p of snapshot.away_team.players) {
    if (p.id === playerId) return p.position;
  }
  if (snapshot.home_bench) {
    for (const p of snapshot.home_bench) {
      if (p.id === playerId) return p.position;
    }
  }
  if (snapshot.away_bench) {
    for (const p of snapshot.away_bench) {
      if (p.id === playerId) return p.position;
    }
  }
  return "";
}

export function phaseLabel(phase: string): string {
  switch (phase) {
    case "PreKickOff":
      return "Pre-Match";
    case "FirstHalf":
      return "1st Half";
    case "HalfTime":
      return "Half Time";
    case "SecondHalf":
      return "2nd Half";
    case "FullTime":
      return "Full Time";
    case "ExtraTimeFirstHalf":
      return "ET 1st Half";
    case "ExtraTimeHalfTime":
      return "ET Half Time";
    case "ExtraTimeSecondHalf":
      return "ET 2nd Half";
    case "ExtraTimeEnd":
      return "ET End";
    case "PenaltyShootout":
      return "Penalties";
    case "Finished":
      return "Final";
    default:
      return phase;
  }
}

export function calcOvr(attrs: Record<string, number>): number {
  const vals = Object.values(attrs);
  if (vals.length === 0) return 0;
  return Math.round(vals.reduce((a, b) => a + b, 0) / vals.length);
}

export function resolveMatchFixture(
  gameState: GameStateData | null,
  snapshot: MatchSnapshot | null,
  fixtureIndex?: number,
): FixtureData | null {
  const fixtures = gameState?.league?.fixtures;
  if (!fixtures || !snapshot) return null;

  if (
    typeof fixtureIndex === "number" &&
    fixtureIndex >= 0 &&
    fixtureIndex < fixtures.length
  ) {
    return fixtures[fixtureIndex];
  }

  return (
    fixtures.find(
      (fixture) =>
        fixture.home_team_id === snapshot.home_team.id &&
        fixture.away_team_id === snapshot.away_team.id,
    ) || null
  );
}

/**
 * Get CSS classes for event category badge styling
 */
export function getEventCategoryClasses(category: EventCategory): {
  badge: string;
  icon: string;
} {
  const styles: Record<EventCategory, { badge: string; icon: string }> = {
    goal: {
      badge: "bg-gradient-to-r from-yellow-100 to-amber-100 dark:from-yellow-900/40 dark:to-amber-900/40 text-yellow-800 dark:text-yellow-200",
      icon: "text-yellow-600 dark:text-yellow-400",
    },
    card: {
      badge: "bg-gradient-to-r from-red-100 to-orange-100 dark:from-red-900/40 dark:to-orange-900/40 text-red-800 dark:text-red-200",
      icon: "text-red-600 dark:text-red-400",
    },
    substitution: {
      badge: "bg-gradient-to-r from-blue-100 to-cyan-100 dark:from-blue-900/40 dark:to-cyan-900/40 text-blue-800 dark:text-blue-200",
      icon: "text-blue-600 dark:text-blue-400",
    },
    shot: {
      badge: "bg-gradient-to-r from-cyan-100 to-teal-100 dark:from-cyan-900/40 dark:to-teal-900/40 text-cyan-800 dark:text-cyan-200",
      icon: "text-cyan-600 dark:text-cyan-400",
    },
    foul: {
      badge: "bg-gradient-to-r from-amber-100 to-yellow-100 dark:from-amber-900/40 dark:to-yellow-900/40 text-amber-800 dark:text-amber-200",
      icon: "text-amber-600 dark:text-amber-400",
    },
    defensive: {
      badge: "bg-gradient-to-r from-emerald-100 to-green-100 dark:from-emerald-900/40 dark:to-green-900/40 text-emerald-800 dark:text-emerald-200",
      icon: "text-emerald-600 dark:text-emerald-400",
    },
    passing: {
      badge: "bg-gradient-to-r from-violet-100 to-purple-100 dark:from-violet-900/40 dark:to-purple-900/40 text-violet-800 dark:text-violet-200",
      icon: "text-violet-600 dark:text-violet-400",
    },
    atmosphere: {
      badge: "bg-gradient-to-r from-gray-100 to-slate-100 dark:from-gray-800/60 dark:to-slate-800/60 text-gray-700 dark:text-gray-300",
      icon: "text-gray-600 dark:text-gray-400",
    },
    structural: {
      badge: "bg-gradient-to-r from-gray-100 to-stone-100 dark:from-gray-800/60 dark:to-stone-800/60 text-gray-700 dark:text-gray-300",
      icon: "text-gray-600 dark:text-gray-400",
    },
    other: {
      badge: "bg-gradient-to-r from-gray-100 to-gray-100 dark:from-gray-800/60 dark:to-gray-800/60 text-gray-700 dark:text-gray-300",
      icon: "text-gray-600 dark:text-gray-400",
    },
  };
  return styles[category] || styles.other;
}

/**
 * Generate a descriptive text for applause events based on the reason
 */
function getApplauseDescription(
  reason: ApplauseReason | undefined,
  playerName: string | null
): string {
  if (!playerName) {
    return "";
  }

  switch (reason) {
    case "Tackle":
      return `${playerName} makes a great tackle!`;
    case "Interception":
      return `${playerName} with a crucial interception!`;
    case "Clearance":
      return `${playerName} clears the danger!`;
    case "Save":
      return `${playerName} makes a save!`;
    case "GreatSave":
      return `${playerName} with an incredible save!`;
    case "General":
      return `${playerName} wins the ball back!`;
    default:
      return `${playerName} impresses the crowd!`;
  }
}
