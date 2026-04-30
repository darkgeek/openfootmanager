import { useTranslation } from "react-i18next";
import type { PlayerData } from "../../store/gameStore";

interface PlayerAttributesTooltipProps {
  player: PlayerData;
  activePosition: string;
}

interface AttributeDisplay {
  name: string;
  value: number;
}

interface AttributeGroupDisplay {
  label: string;
  attrs: AttributeDisplay[];
}

export function PlayerAttributesTooltip({ player, activePosition }: PlayerAttributesTooltipProps) {
  const { t } = useTranslation();
  
  const attributes = player.attributes;
  
  const groups: AttributeGroupDisplay[] = [
    {
      label: t("playerProfile.physical", "Physical"),
      attrs: [
        { name: t("common.attributes.pace", "Pace"), value: attributes.pace },
        { name: t("common.attributes.stamina", "Stamina"), value: attributes.stamina },
        { name: t("common.attributes.strength", "Strength"), value: attributes.strength },
        { name: t("common.attributes.agility", "Agility"), value: attributes.agility },
      ],
    },
    {
      label: t("playerProfile.offensive", "Offensive"),
      attrs: [
        { name: t("common.attributes.shooting", "Shooting"), value: attributes.shooting },
        { name: t("common.attributes.passing", "Passing"), value: attributes.passing },
        { name: t("common.attributes.dribbling", "Dribbling"), value: attributes.dribbling },
        { name: t("common.attributes.composure", "Composure"), value: attributes.composure },
      ],
    },
    {
      label: t("playerProfile.defensive", "Defensive"),
      attrs: [
        { name: t("common.attributes.tackling", "Tackling"), value: attributes.tackling },
        { name: t("common.attributes.defending", "Defending"), value: attributes.defending },
        { name: t("common.attributes.positioning", "Positioning"), value: attributes.positioning },
        { name: t("common.attributes.aggression", "Aggression"), value: attributes.aggression },
      ],
    },
    {
      label: t("playerProfile.mental", "Mental"),
      attrs: [
        { name: t("common.attributes.vision", "Vision"), value: attributes.vision },
        { name: t("common.attributes.decisions", "Decisions"), value: attributes.decisions },
        { name: t("common.attributes.teamwork", "Teamwork"), value: attributes.teamwork },
        { name: t("common.attributes.leadership", "Leadership"), value: attributes.leadership },
      ],
    },
    {
      label: t("playerProfile.goalkeeper", "Goalkeeper"),
      attrs: [
        { name: t("common.attributes.handling", "Handling"), value: attributes.handling },
        { name: t("common.attributes.reflexes", "Reflexes"), value: attributes.reflexes },
        { name: t("common.attributes.aerial", "Aerial"), value: attributes.aerial },
      ],
    },
  ];

  const getBarColor = (value: number) => {
    if (value >= 80) return "bg-green-500";
    if (value >= 60) return "bg-blue-500";
    if (value >= 40) return "bg-yellow-500";
    return "bg-gray-400";
  };

  return (
    <div className="space-y-2 min-w-[200px]">
      <div className="font-heading font-bold text-sm border-b border-gray-700 pb-2 mb-2">
        {player.full_name}
        <span className="text-gray-400 font-normal ml-2 text-xs">
          ({activePosition})
        </span>
      </div>
      
      <div className="grid grid-cols-2 gap-x-4 gap-y-1.5">
        {groups.map((group) =>
          group.attrs.map((attr) => (
            <div key={attr.name} className="flex items-center gap-2">
              <span className="text-gray-400 text-xs w-16 truncate">
                {attr.name}
              </span>
              <div className="flex-1 h-1.5 bg-gray-700 rounded-full overflow-hidden">
                <div
                  className={`h-full ${getBarColor(attr.value)} rounded-full`}
                  style={{ width: `${attr.value}%` }}
                />
              </div>
              <span className="text-white font-bold text-xs w-5 text-right tabular-nums">
                {attr.value}
              </span>
            </div>
          ))
        )}
      </div>
    </div>
  );
}