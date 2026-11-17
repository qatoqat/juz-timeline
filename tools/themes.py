"""Build explore/assets/themes.json from QUL's ayah-theme export, aligned to quran.com's ruku'.

Inputs: tools/.cache/ayah-themes.db.zip (QUL sqlite download) and the quran.com cache
(tools/fetch_quran_com.sh). Themes that cross a quran.com ruku' boundary are split there, and
ayat with no theme get a "no theme" entry carrying the opening words of their first ayah, so
every ayah belongs to exactly one entry and every entry sits inside one ruku'.

Output rows: [surah, from, to, kind, text] with kind 0 = theme, 1 = continued theme, 2 = no theme.
Run: python tools/themes.py
"""
import json
import pathlib
import sqlite3
import zipfile

ROOT = pathlib.Path(__file__).resolve().parent
CACHE = ROOT / ".cache"
ZIP = CACHE / "ayah-themes.db.zip"
DB = CACHE / "themes" / "ayah-themes.db"
OUT = ROOT.parent / "explore" / "assets" / "themes.json"
OPENING_WORDS = 6
THEME, CONTINUED, NONE = 0, 1, 2


def quran_com():
    verses, counts, ruku = {}, {}, set()
    for s in range(1, 115):
        prev = None
        for v in json.loads((CACHE / "quran-com" / f"{s}.json").read_text(encoding="utf-8"))["verses"]:
            a = v["verse_number"]
            verses[(s, a)] = v["text_imlaei"]
            counts[s] = a
            if v["ruku_number"] != prev:
                ruku.add((s, a))
                prev = v["ruku_number"]
    return verses, counts, ruku


def opening(text):
    words = [w for w in text.split() if not all(0x064B <= ord(c) <= 0x065F or 0x06D6 <= ord(c) <= 0x06ED for c in w)]
    return " ".join(words[:OPENING_WORDS]) + (" …" if len(words) > OPENING_WORDS else "")


def split_at_ruku(s, a, b, ruku):
    cuts = [x for x in range(a + 1, b + 1) if (s, x) in ruku]
    bounds = [a] + cuts + [b + 1]
    return [(bounds[i], bounds[i + 1] - 1) for i in range(len(bounds) - 1)]


def build():
    if not DB.exists():
        zipfile.ZipFile(ZIP).extractall(DB.parent)
    themes = sqlite3.connect(DB).execute(
        "select distinct surah_number, ayah_from, ayah_to, theme from themes order by surah_number, ayah_from"
    ).fetchall()
    verses, counts, ruku = quran_com()
    covered = {}
    rows = []
    for s, a, b, text in themes:
        for n, (x, y) in enumerate(split_at_ruku(s, a, min(b, counts[s]), ruku)):
            rows.append([s, x, y, CONTINUED if n else THEME, " ".join(text.split())])
            covered.update({(s, k): True for k in range(x, y + 1)})
    for s in range(1, 115):
        a = 1
        while a <= counts[s]:
            if (s, a) in covered:
                a += 1
                continue
            b = a
            while b + 1 <= counts[s] and (s, b + 1) not in covered:
                b += 1
            for x, y in split_at_ruku(s, a, b, ruku):
                rows.append([s, x, y, NONE, opening(verses[(s, x)])])
            a = b + 1
    rows.sort()
    return rows, counts, ruku


def check(rows, counts, ruku):
    seen = {}
    for s, a, b, _, _ in rows:
        assert a <= b <= counts[s], (s, a, b)
        for x in range(a, b + 1):
            assert (s, x) not in seen, f"overlap at {s}:{x}"
            seen[(s, x)] = True
        assert not any((s, x) in ruku for x in range(a + 1, b + 1)), f"{s}:{a}-{b} crosses a ruku'"
    missing = [(s, a) for s in counts for a in range(1, counts[s] + 1) if (s, a) not in seen]
    assert not missing, f"uncovered ayat: {missing[:5]}"


def main():
    rows, counts, ruku = build()
    check(rows, counts, ruku)
    OUT.write_text(json.dumps(rows, ensure_ascii=False, separators=(",", ":")), encoding="utf-8")
    kinds = [r[3] for r in rows]
    print(f"{kinds.count(THEME)} themes, {kinds.count(CONTINUED)} continued at a ruku', {kinds.count(NONE)} no-theme gaps, "
          f"{sum(counts.values())} ayat covered, {OUT.stat().st_size // 1024} KB")


if __name__ == "__main__":
    main()
