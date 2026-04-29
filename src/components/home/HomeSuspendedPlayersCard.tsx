import { Ban } from "lucide-react";
import { useTranslation } from "react-i18next";

import type { PlayerData } from "../../store/gameStore";
import { Badge, Card, CardBody, CardHeader } from "../ui";

interface HomeSuspendedPlayersCardProps {
  players: PlayerData[];
  onNavigate?: (tab: string) => void;
}

export default function HomeSuspendedPlayersCard({
  players,
  onNavigate,
}: HomeSuspendedPlayersCardProps) {
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
            className="text-amber-600 dark:text-amber-400 text-xs font-heading font-bold uppercase tracking-wider hover:text-amber-700 dark:hover:text-amber-300 transition-colors"
          >
            {t("dashboard.squad")}
          </button>
        }
      >
        <div className="flex items-center gap-2">
          <Ban className="w-4 h-4 text-amber-500" />
          {t("home.suspendedPlayers", { count: players.length })}
        </div>
      </CardHeader>
      <CardBody>
        <div className="flex flex-col gap-2.5">
          {players.map((player) => (
            <div
              key={player.id}
              className="flex flex-col gap-2 rounded-lg border border-amber-100 bg-amber-50/50 px-3 py-2.5 dark:border-amber-900/30 dark:bg-amber-900/10 sm:flex-row sm:items-center sm:justify-between"
            >
              <div className="min-w-0">
                <div className="flex flex-wrap items-center gap-2">
                  <span className="truncate text-sm font-heading font-bold text-gray-800 dark:text-gray-200">
                    {player.full_name}
                  </span>
                  <Badge variant="neutral" size="sm">
                    {t("common.suspended")}
                  </Badge>
                </div>
                <p className="mt-1 text-xs text-amber-600 dark:text-amber-400">
                  {t("home.gameSuspended", {
                    count: player.suspension_games_remaining,
                  })}
                </p>
              </div>
              <div className="text-xs font-heading font-bold uppercase tracking-wider text-gray-400 dark:text-gray-500">
                {t(`common.positions.${player.position}`, {
                  defaultValue: player.position,
                })}
              </div>
            </div>
          ))}
        </div>
      </CardBody>
    </Card>
  );
}