"""Build explore/assets/themes.json from QUL's ayah-theme sqlite export.

Place the download at tools/.cache/ayah-themes.db.zip (from https://qul.tarteel.ai/resources/ayah-theme).
Run: python tools/themes.py
"""
import json
import pathlib
import sqlite3
import zipfile

ROOT = pathlib.Path(__file__).resolve().parent
ZIP = ROOT / ".cache" / "ayah-themes.db.zip"
DB = ROOT / ".cache" / "themes" / "ayah-themes.db"
OUT = ROOT.parent / "explore" / "assets" / "themes.json"


def main():
    if not DB.exists():
        zipfile.ZipFile(ZIP).extractall(DB.parent)
    rows = sqlite3.connect(DB).execute(
        "select distinct surah_number, ayah_from, ayah_to, theme from themes order by surah_number, ayah_from"
    ).fetchall()
    data = [[s, a, b, " ".join(t.split())] for s, a, b, t in rows]
    OUT.write_text(json.dumps(data, ensure_ascii=False, separators=(",", ":")), encoding="utf-8")
    print(f"{len(data)} themes, {OUT.stat().st_size // 1024} KB")


if __name__ == "__main__":
    main()
