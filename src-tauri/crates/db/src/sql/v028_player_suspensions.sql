ALTER TABLE players ADD COLUMN suspension_games_remaining INTEGER NOT NULL DEFAULT 0;
ALTER TABLE players ADD COLUMN accumulated_yellow_cards INTEGER NOT NULL DEFAULT 0;
