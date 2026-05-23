-- Add suspension tracking fields to players table
-- - suspension_games_remaining: games left until player can play
-- - accumulated_yellow_cards: yellow cards this season (resets each season)

ALTER TABLE players ADD COLUMN suspension_games_remaining INTEGER NOT NULL DEFAULT 0;
ALTER TABLE players ADD COLUMN accumulated_yellow_cards INTEGER NOT NULL DEFAULT 0;