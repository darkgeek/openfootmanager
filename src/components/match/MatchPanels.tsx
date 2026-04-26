import { useTranslation } from "react-i18next";
import { MatchSnapshot, MatchEvent, EnginePlayerData } from "./types";
import { getEventDisplay, getEventDescription, getPlayerName, getPlayerPosition, getEventCategoryClasses } from "./helpers";
import { Badge } from "../ui";
import { translatePositionAbbreviation } from "../squad/SquadTab.helpers";
import { Circle } from "lucide-react";

export function EventFeed({
  events,
  snapshot,
  feedRef,
}: {
  events: MatchEvent[];
  snapshot: MatchSnapshot;
  feedRef: React.RefObject<HTMLDivElement | null>;
}) {
  const { t } = useTranslation();
  
  if (events.length === 0) {
    return (
      <div className="flex items-center justify-center h-40">
        <div className="text-center">
          <div className="w-12 h-12 mx-auto mb-3 rounded-full bg-gray-100 dark:bg-navy-800 flex items-center justify-center">
            <Circle className="w-6 h-6 text-gray-400" />
          </div>
          <p className="font-heading text-sm uppercase tracking-wider text-gray-600 dark:text-gray-500">
            {t("match.waitingKickoff")}
          </p>
        </div>
      </div>
    );
  }
  
  return (
    <div ref={feedRef} className="flex flex-col gap-1.5">
      {events.slice().reverse().map((evt, i) => {
        const display = getEventDisplay(evt);
        const description = getEventDescription(evt, snapshot);
        const isHome = evt.side === "Home";
        const categoryClasses = getEventCategoryClasses(display.category);
        const playerPosition = evt.player_id ? getPlayerPosition(snapshot, evt.player_id) : "";
        const posAbbr = playerPosition ? translatePositionAbbreviation(t, playerPosition) : "";
        
        // Determine card events that need special treatment
        const isCardEvent = display.category === "card";
        const isGoalEvent = display.category === "goal";
        const isImportantEvent = display.important;
        
        return (
          <div
            key={i}
            className={`
              relative overflow-hidden rounded-lg border transition-all duration-200
              ${isGoalEvent 
                ? `${display.bgColor} ${display.borderColor} shadow-sm ring-1 ring-${display.borderColor.includes('yellow') ? 'yellow' : 'amber'}-200/50 dark:ring-yellow-700/30` 
                : isCardEvent 
                  ? `${display.bgColor} ${display.borderColor} shadow-sm` 
                  : isImportantEvent 
                    ? `${display.bgColor} ${display.borderColor} shadow-sm` 
                    : "bg-white/60 dark:bg-navy-800/40 border-gray-200/80 dark:border-navy-700/60 hover:bg-white/80 dark:hover:bg-navy-800/60"
              }
            `}
          >
            {/* Accent stripe for important events */}
            {isImportantEvent && (
              <div 
                className={`absolute left-0 top-0 bottom-0 w-1 rounded-l-lg ${display.color.replace('text-', 'bg-')}`}
              />
            )}
            
            <div className="flex items-start gap-2.5 px-3 py-2.5 pl-3">
              {/* Minute badge */}
              <div className={`
                flex-shrink-0 w-10 h-10 rounded-lg flex flex-col items-center justify-center
                ${isGoalEvent 
                  ? "bg-gradient-to-br from-yellow-100 to-amber-100 dark:from-yellow-900/50 dark:to-amber-900/50" 
                  : isCardEvent 
                    ? "bg-gradient-to-br from-red-100 to-orange-100 dark:from-red-900/50 dark:to-orange-900/50" 
                    : isImportantEvent
                      ? "bg-gradient-to-br from-gray-100 to-gray-50 dark:from-navy-700/50 dark:to-navy-800/50"
                      : "bg-gray-100 dark:bg-navy-700/50"
                }
              `}>
                <span className={`font-heading font-bold text-sm tabular-nums leading-none ${display.color}`}>
                  {evt.minute}'
                </span>
                {isGoalEvent && (
                  <span className="text-[8px] font-heading uppercase text-yellow-600 dark:text-yellow-400 mt-0.5">GOAL</span>
                )}
                {isCardEvent && (
                  <span className="text-[8px] font-heading uppercase text-red-600 dark:text-red-400 mt-0.5">CARD</span>
                )}
              </div>
              
              {/* Event icon */}
              <div className={`
                flex-shrink-0 w-9 h-9 rounded-full flex items-center justify-center
                ${isGoalEvent 
                  ? "bg-yellow-100 dark:bg-yellow-900/40" 
                  : isCardEvent 
                    ? "bg-red-100 dark:bg-red-900/40" 
                    : isImportantEvent
                      ? "bg-gray-100 dark:bg-navy-700/40"
                      : "bg-gray-50 dark:bg-navy-800/40"
                }
              `}>
                <span className={display.color}>
                  {display.icon}
                </span>
              </div>
              
              {/* Content */}
              <div className="flex-1 min-w-0">
                <div className="flex items-center gap-2 flex-wrap">
                  {/* Team badge */}
                  <span
                    className={`
                      inline-flex items-center px-1.5 py-0.5 rounded text-[10px] font-heading font-bold uppercase tracking-wider
                      ${isHome 
                        ? "bg-primary-100 text-primary-700 dark:bg-primary-900/40 dark:text-primary-300" 
                        : "bg-indigo-100 text-indigo-700 dark:bg-indigo-900/40 dark:text-indigo-300"
                      }
                    `}
                  >
                    {isHome ? snapshot.home_team.name.substring(0, 3) : snapshot.away_team.name.substring(0, 3)}
                  </span>
                  
                  {/* Position badge for events with players */}
                  {posAbbr && (
                    <span className="inline-flex items-center px-1.5 py-0.5 rounded text-[10px] font-heading bg-gray-100 text-gray-600 dark:bg-navy-700 dark:text-gray-400">
                      {posAbbr}
                    </span>
                  )}
                  
                  {/* Event label */}
                  <span className={`
                    inline-flex items-center px-1.5 py-0.5 rounded text-[10px] font-heading font-medium uppercase tracking-wider
                    ${categoryClasses.badge}
                  `}>
                    {display.label}
                  </span>
                </div>
                
                {/* Description */}
                <p className={`
                  mt-0.5 text-sm leading-snug
                  ${isGoalEvent 
                    ? "font-semibold text-gray-900 dark:text-gray-100" 
                    : isCardEvent 
                      ? "font-medium text-gray-800 dark:text-gray-200" 
                      : isImportantEvent
                        ? "font-medium text-gray-700 dark:text-gray-300"
                        : "text-gray-600 dark:text-gray-400"
                  }
                `}>
                  {description}
                </p>
                
                {/* Secondary player for goal events (assists) */}
                {evt.secondary_player_id && isGoalEvent && (
                  <div className="mt-1 flex items-center gap-1 text-xs text-gray-500 dark:text-gray-400">
                    <span>Assist:</span>
                    <span className="font-medium text-gray-600 dark:text-gray-300">
                      {getPlayerName(snapshot, evt.secondary_player_id)}
                    </span>
                  </div>
                )}
                
                {/* Secondary player for substitution events */}
                {evt.secondary_player_id && evt.event_type === "Substitution" && (
                  <div className="mt-0.5 text-xs text-blue-600 dark:text-blue-400 font-medium">
                    ↪ {getPlayerName(snapshot, evt.secondary_player_id)} comes on
                  </div>
                )}
              </div>
            </div>
          </div>
        );
      })}
    </div>
  );
}

export function MatchStats({ snapshot }: { snapshot: MatchSnapshot }) {
  const { t } = useTranslation();
  const homeEvents = snapshot.events.filter((e) => e.side === "Home");
  const awayEvents = snapshot.events.filter((e) => e.side === "Away");
  const ct = (events: MatchEvent[], type: string) =>
    events.filter((e) => e.event_type === type).length;

  const stats = [
    {
      label: t("match.possession"),
      home: `${snapshot.home_possession_pct.toFixed(0)}%`,
      away: `${snapshot.away_possession_pct.toFixed(0)}%`,
      homePct: snapshot.home_possession_pct,
    },
    {
      label: t("match.shots"),
      home:
        ct(homeEvents, "Goal") +
        ct(homeEvents, "PenaltyGoal") +
        ct(homeEvents, "ShotSaved") +
        ct(homeEvents, "ShotOffTarget") +
        ct(homeEvents, "ShotBlocked"),
      away:
        ct(awayEvents, "Goal") +
        ct(awayEvents, "PenaltyGoal") +
        ct(awayEvents, "ShotSaved") +
        ct(awayEvents, "ShotOffTarget") +
        ct(awayEvents, "ShotBlocked"),
    },
    {
      label: t("match.shotsOnTarget"),
      home:
        ct(homeEvents, "Goal") +
        ct(homeEvents, "PenaltyGoal") +
        ct(homeEvents, "ShotSaved"),
      away:
        ct(awayEvents, "Goal") +
        ct(awayEvents, "PenaltyGoal") +
        ct(awayEvents, "ShotSaved"),
    },
    {
      label: t("match.fouls"),
      home: ct(homeEvents, "Foul"),
      away: ct(awayEvents, "Foul"),
    },
    {
      label: t("match.corners"),
      home: ct(homeEvents, "Corner"),
      away: ct(awayEvents, "Corner"),
    },
    {
      label: t("match.yellowCards"),
      home: Object.keys(snapshot.home_yellows).length,
      away: Object.keys(snapshot.away_yellows).length,
    },
  ];

  return (
    <div className="max-w-lg mx-auto flex flex-col gap-3">
      {stats.map((stat, i) => {
        const hv = typeof stat.home === "number" ? stat.home : 0;
        const av = typeof stat.away === "number" ? stat.away : 0;
        const total = hv + av || 1;
        const pct = stat.homePct ?? (hv / total) * 100;
        return (
          <div key={i}>
            <div className="flex justify-between text-xs mb-1">
              <span className="font-heading font-bold text-primary-400 tabular-nums">
                {stat.home}
              </span>
                <span className="text-gray-500 dark:text-gray-400 font-heading uppercase tracking-wider text-[10px]">
                {stat.label}
              </span>
              <span className="font-heading font-bold text-indigo-400 tabular-nums">
                {stat.away}
              </span>
            </div>
             <div className="flex h-1.5 bg-gray-300 dark:bg-navy-700 rounded-full overflow-hidden transition-colors duration-300">
              <div
                className="h-full bg-primary-500 transition-all duration-500"
                style={{ width: `${pct}%` }}
              />
              <div
                className="h-full bg-indigo-500 transition-all duration-500"
                style={{ width: `${100 - pct}%` }}
              />
            </div>
          </div>
        );
      })}
    </div>
  );
}

export function Lineups({ snapshot }: { snapshot: MatchSnapshot }) {
  const { t } = useTranslation();
  const renderTeam = (
    team: MatchSnapshot["home_team"],
    bench: EnginePlayerData[],
    side: "Home" | "Away",
    yellows: Record<string, number>,
    sentOff: string[],
  ) => {
    const positions = ["Goalkeeper", "Defender", "Midfielder", "Forward"];
    const subbedOnIds = new Set(
      snapshot.substitutions
        .filter((s) => s.side === side)
        .map((s) => s.player_on_id),
    );
    const subbedOffIds = new Set(
      snapshot.substitutions
        .filter((s) => s.side === side)
        .map((s) => s.player_off_id),
    );
    return (
      <div className="flex-1">
        <h4
          className={`font-heading font-bold text-sm uppercase tracking-wider mb-3 ${side === "Home" ? "text-primary-400" : "text-indigo-400"}`}
        >
          {team.name}{" "}
            <span className="text-gray-600 dark:text-gray-500 font-normal text-xs">
            ({team.formation})
          </span>
        </h4>
        {positions.map((pos) => {
          const players = team.players.filter((p) => p.position === pos);
          if (players.length === 0) return null;
          return (
            <div key={pos} className="mb-3">
              <p className="text-[10px] font-heading uppercase tracking-widest text-gray-600 dark:text-gray-500 mb-1">
                {pos}s
              </p>
              {players.map((p) => {
                const isOff = sentOff.includes(p.id);
                const yc = yellows[p.id] || 0;
                const isSubOn = subbedOnIds.has(p.id);
                const condColor =
                  p.condition >= 70
                    ? "bg-primary-500"
                    : p.condition >= 40
                      ? "bg-yellow-500"
                      : "bg-red-500";
                return (
                  <div
                    key={p.id}
                    className={`flex items-center gap-2 py-1 px-2 rounded text-xs ${isOff ? "opacity-40" : ""}`}
                  >
                    {isSubOn && (
                      <span className="text-green-400 text-[10px]">▲</span>
                    )}
                    <span
                       className={`font-medium flex-1 truncate ${isOff ? "line-through text-gray-600 dark:text-gray-500" : "text-gray-700 dark:text-gray-300"}`}
                    >
                      {p.name}
                    </span>
                    {yc > 0 && (
                      <span className="w-3 h-4 rounded-sm bg-yellow-400 text-navy-900 text-[8px] flex items-center justify-center font-bold">
                        {yc > 1 ? yc : ""}
                      </span>
                    )}
                    {isOff && (
                      <span className="w-3 h-4 rounded-sm bg-red-500" />
                    )}
                    <div className="w-14 flex items-center gap-1">
                       <div className="flex-1 h-1.5 bg-gray-300 dark:bg-navy-600 rounded-full overflow-hidden transition-colors duration-300">
                        <div
                          className={`h-full ${condColor} rounded-full transition-all`}
                          style={{ width: `${p.condition}%` }}
                        />
                      </div>
                       <span className="text-gray-500 dark:text-gray-400 tabular-nums text-[10px] w-6 text-right">
                        {Math.round(p.condition)}
                      </span>
                    </div>
                  </div>
                );
              })}
            </div>
          );
        })}

        {/* Bench */}
        {bench.length > 0 && (
           <div className="mt-3 pt-3 border-t border-gray-200 dark:border-navy-700">
             <p className="text-[10px] font-heading uppercase tracking-widest text-gray-600 dark:text-gray-500 mb-1">
              {t("match.bench")}
            </p>
            {bench.map((p) => {
              const wasSubbedOff = subbedOffIds.has(p.id);
              return (
                <div
                  key={p.id}
                  className={`flex items-center gap-2 py-1 px-2 rounded text-xs ${wasSubbedOff ? "opacity-50" : ""}`}
                >
                  {wasSubbedOff && (
                    <span className="text-red-400 text-[10px]">▼</span>
                  )}
                   <span className="text-gray-600 dark:text-gray-400 font-medium flex-1 truncate">
                    {p.name}
                  </span>
                  <Badge variant="neutral" size="sm">
                    {translatePositionAbbreviation(t, p.position)}
                  </Badge>
                   <span className="text-gray-500 dark:text-gray-400 tabular-nums text-[10px] w-6 text-right">
                    {Math.round(p.condition)}
                  </span>
                </div>
              );
            })}
          </div>
        )}

        {/* Sub History */}
        {snapshot.substitutions.filter((s) => s.side === side).length > 0 && (
           <div className="mt-3 pt-3 border-t border-gray-200 dark:border-navy-700">
             <p className="text-[10px] font-heading uppercase tracking-widest text-gray-600 dark:text-gray-500 mb-1">
              {t("match.substitutions")}
            </p>
            {snapshot.substitutions
              .filter((s) => s.side === side)
              .map((sub, i) => (
                <div
                  key={i}
                  className="flex items-center gap-1.5 py-0.5 text-[11px]"
                >
                   <span className="text-gray-600 dark:text-gray-500 tabular-nums w-5 text-right font-heading">
                    {sub.minute}'
                  </span>
                  <span className="text-green-400">▲</span>
                   <span className="text-gray-700 dark:text-gray-300 truncate">
                    {getPlayerName(snapshot, sub.player_on_id)}
                  </span>
                  <span className="text-red-400">▼</span>
                   <span className="text-gray-500 dark:text-gray-400 truncate">
                    {getPlayerName(snapshot, sub.player_off_id)}
                  </span>
                </div>
              ))}
          </div>
        )}
      </div>
    );
  };
  return (
    <div className="flex gap-6">
      {renderTeam(
        snapshot.home_team,
        snapshot.home_bench,
        "Home",
        snapshot.home_yellows,
        snapshot.sent_off,
      )}
      <div className="w-px bg-gray-200 dark:bg-navy-700 transition-colors duration-300" />
      {renderTeam(
        snapshot.away_team,
        snapshot.away_bench,
        "Away",
        snapshot.away_yellows,
        snapshot.sent_off,
      )}
    </div>
  );
}
