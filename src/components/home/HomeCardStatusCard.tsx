import { AlertCircle } from "lucide-react";
import { useTranslation } from "react-i18next";

import type { PlayerData } from "../../store/gameStore";
import { Badge, Card, CardBody, CardHeader } from "../ui";

interface HomeCardStatusCardProps {
  players: PlayerData[];
  onNavigate?: (tab: string) => void;
}

export default function HomeCardStatusCard({
  players,
  onNavigate,
}: HomeCardStatusCardProps) {
  const { t } = useTranslation();

  if (players.length === 0) {
    return null;
  }

  return (
    <Card>
      <CardHeader
        action={
          <button
            onClick={() => onNavigate?.("Squad")}
            className="text-red-600 dark:text-red-400 text-xs font-heading font-bold uppercase tracking-wider hover:text-red-700 dark:hover:text-red-300 transition-colors"
          >
            {t("dashboard.squad")}
          </button>
        }
      >
        <div className="flex items-center gap-2">
          <AlertCircle className="w-4 h-4 text-red-500" />
          {t("home.playersWithCards", { count: players.length })}
        </div>
      </CardHeader>
      <CardBody>
        <div className="flex flex-col gap-2.5">
          {players.map((player) => {
            const isSuspended = player.suspension_games_remaining > 0;
            const yellowCards = player.accumulated_yellow_cards || 0;
            
            return (
              <div
                key={player.id}
                className={`flex flex-col gap-2 rounded-lg border px-3 py-2.5 dark:border-navy-700 sm:flex-row sm:items-center sm:justify-between ${
                  isSuspended 
                    ? "border-red-200 bg-red-50/50 dark:border-red-900/30 dark:bg-red-900/10" 
                    : "border-gray-100 bg-gray-50/50 dark:border-navy-700 dark:bg-navy-800/30"
                }`}
              >
                <div className="min-w-0">
                  <div className="flex flex-wrap items-center gap-2">
                    <span className="truncate text-sm font-heading font-bold text-gray-800 dark:text-gray-200">
                      {player.full_name}
                    </span>
                    {isSuspended ? (
                      <Badge variant="danger" size="sm">
                        {t("common.suspended")}
                      </Badge>
                    ) : (
                      <Badge variant="neutral" size="sm">
                        {t("home.yellowCards", { count: yellowCards })}
                      </Badge>
                    )}
                  </div>
                  <p className={`mt-1 text-xs ${
                    isSuspended 
                      ? "text-red-600 dark:text-red-400" 
                      : "text-amber-600 dark:text-amber-400"
                  }`}>
                    {isSuspended ? (
                      t("home.gameSuspended", { 
                        count: player.suspension_games_remaining 
                      })
                    ) : yellowCards >= 2 ? (
                      t("home.warningNearSuspension", { 
                        count: yellowCards 
                      })
                    ) : (
                      t("home.yellowCardsAccumulated", { 
                        count: yellowCards 
                      })
                    )}
                  </p>
                </div>
                <div className="text-xs font-heading font-bold uppercase tracking-wider text-gray-400 dark:text-gray-500">
                  {t(`common.positions.${player.position}`, {
                    defaultValue: player.position,
                  })}
                </div>
              </div>
            );
          })}
        </div>
      </CardBody>
    </Card>
  );
}