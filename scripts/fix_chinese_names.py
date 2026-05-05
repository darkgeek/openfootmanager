#!/usr/bin/env python3
"""
Fix Chinese player names: remove spaces between surname and given name.
Chinese names should be like "王志强" not "王 志强".

Usage:
    python3 scripts/fix_chinese_names.py [path_to_save.db]
"""

import sqlite3
import sys
from pathlib import Path


def find_save_file(path=None):
    if path:
        p = Path(path)
        if p.exists():
            return p
        print(f"File not found: {path}")
        sys.exit(1)

    candidates = [
        Path.home() / ".local/share/openfootmanager/saves",
        Path.home() / "Library/Application Support/openfootmanager/saves",
    ]
    for saves_dir in candidates:
        if saves_dir.exists():
            dbs = list(saves_dir.glob("*.db"))
            if dbs:
                return dbs[0]

    print("No save file found. Provide path manually.")
    sys.exit(1)


def fix_players(db_path):
    conn = sqlite3.connect(str(db_path))
    cursor = conn.cursor()

    cursor.execute("""
        UPDATE players
        SET full_name = REPLACE(full_name, ' ', ''),
            match_name = REPLACE(match_name, ' ', '')
        WHERE nationality = 'China'
    """)
    affected = cursor.rowcount
    conn.commit()
    conn.close()

    print(f"Fixed {affected} player(s) ✅")


if __name__ == "__main__":
    db_path = find_save_file(sys.argv[1] if len(sys.argv) > 1 else None)
    print(f"Fixing players in: {db_path}")
    fix_players(db_path)
