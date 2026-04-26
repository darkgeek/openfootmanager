import { Crosshair, Dumbbell, Shield, Trophy, Users } from "lucide-react";

import { Card, CardBody, CardHeader } from "../ui";
import type { TeamData } from "../../store/gameStore";
import type { TeamProfileTranslate } from "./TeamProfile.types";
import { InfoRow } from "./TeamProfile.primitives";

interface TeamProfileClubDetailsCardProps {
  team: TeamData;
  t: TeamProfileTranslate;
}

// Format training focus for display (e.g., "Physical" -> "Physical Training")
function formatTrainingFocus(focus: string): string {
  if (!focus) return "-";
  // Add "Training" suffix if not already present
  if (focus.toLowerCase().includes("training")) return focus;
  return `${focus} Training`;
}

// Format training intensity for display
function formatTrainingIntensity(intensity: string): string {
  if (!intensity) return "-";
  // Convert to title case
  return intensity.charAt(0).toUpperCase() + intensity.slice(1).toLowerCase();
}

export default function TeamProfileClubDetailsCard({
  team,
  t,
}: TeamProfileClubDetailsCardProps) {
  return (
    <Card>
      <CardHeader>{t("teamProfile.clubInfo")}</CardHeader>
      <CardBody>
        <div className="flex flex-col gap-3">
          <InfoRow
            icon={<Shield className="w-4 h-4" />}
            label={t("teamProfile.stadium")}
            value={team.stadium_name}
          />
          <InfoRow
            icon={<Users className="w-4 h-4" />}
            label={t("teamProfile.capacity")}
            value={team.stadium_capacity.toLocaleString()}
          />
          <InfoRow
            icon={<Crosshair className="w-4 h-4" />}
            label={t("tactics.formation")}
            value={team.formation}
          />
          <InfoRow
            icon={<Trophy className="w-4 h-4" />}
            label={t("tactics.playStyle")}
            value={team.play_style}
          />
          {/* Training Info - shown for all teams */}
          <div className="border-t border-gray-200 dark:border-gray-700 pt-3 mt-1">
            <InfoRow
              icon={<Dumbbell className="w-4 h-4" />}
              label={t("training.trainingFocus")}
              value={formatTrainingFocus(team.training_focus)}
            />
            <InfoRow
              icon={<Dumbbell className="w-4 h-4" />}
              label={t("training.intensity")}
              value={formatTrainingIntensity(team.training_intensity)}
            />
          </div>
        </div>
      </CardBody>
    </Card>
  );
}
