"""Validate generated data against the Content Sync snapshots; exits non-zero on any conflict.

Run: python tools/check_data.py
"""
import json
import pathlib
import subprocess
import sys

from qf import plain, quran

ROOT = pathlib.Path(__file__).resolve().parent
REPO = ROOT.parent
errors = []


def expect(ok, message):
    if not ok:
        errors.append(message)


def main():
    q = quran()
    chapters, verses = q.chapters, q.verses
    counts = {c["chapter_number"]: c["verses_count"] for c in chapters}
    expect(len(chapters) == 114, f"{len(chapters)} chapters")
    expect(len(verses) == 6236 == sum(counts.values()), f"{len(verses)} verses vs {sum(counts.values())} counted")
    for field, total in [("juz_number", 30), ("page", 604), ("rub_el_hizb_number", 240), ("manzil_number", 7)]:
        values = [v[field] for v in verses]
        expect(values == sorted(values) and len(set(values)) == total, f"{field}: not 1..{total} in order")
    ruku = [v["ruku_number"] for v in verses]
    expect(ruku == sorted(ruku), "ruku' numbers out of order")
    starts = [v for i, v in enumerate(verses) if i == 0 or v["ruku_number"] != verses[i - 1]["ruku_number"]]
    surah_of = lambda v: int(v["verse_key"].split(":")[0])
    for s in range(1, 115):
        expect(any(surah_of(v) == s and v["verse_number"] == 1 for v in starts), f"surah {s} does not open a ruku'")

    generated = subprocess.run([sys.executable, str(ROOT / "gen_data.py")], capture_output=True, text=True, check=True).stdout
    committed = (REPO / "core" / "src" / "data.rs").read_text(encoding="utf-8")
    expect(generated.strip() == committed.strip(), "core/src/data.rs is out of date: run tools/gen_data.py")

    similar = json.loads((REPO / "explore" / "assets" / "similar.json").read_text(encoding="utf-8"))
    words = [len(row[2].split(" ")) for row in similar["a"]]
    for f, (length, members) in enumerate(similar["f"]):
        expect(len(members) >= 2, f"family {f} has {len(members)} member")
        for a, start in members:
            expect(start + length <= words[a], f"family {f} runs past ayah {similar['a'][a][:2]}")
    empty = [row[:2] for row in similar["a"] for w in row[2].split(" ") if not plain(w)]
    expect(not empty, f"similar.json has sign-only words in {empty[:3]}")
    keys = {(int(v["verse_key"].split(":")[0]), v["verse_number"]) for v in verses}
    expect(all((row[0], row[1]) in keys for row in similar["a"]), "similar.json references an unknown ayah")

    themes = json.loads((REPO / "explore" / "assets" / "themes.json").read_text(encoding="utf-8"))
    ruku_starts = {(surah_of(v), v["verse_number"]) for v in starts}
    covered = {}
    for s, a, b, kind, _ in themes:
        expect(1 <= a <= b <= counts[s], f"theme {s}:{a}-{b} out of range")
        expect(not any((s, x) in ruku_starts for x in range(a + 1, b + 1)), f"theme {s}:{a}-{b} crosses a ruku'")
        for x in range(a, b + 1):
            expect((s, x) not in covered, f"themes overlap at {s}:{x}")
            covered[(s, x)] = kind
    expect(len(covered) == 6236, f"themes cover {len(covered)} of 6236 ayat")

    if errors:
        print("data check failed:")
        for e in errors[:40]:
            print("  -", e)
        sys.exit(1)
    print(f"data check passed: {len(verses)} ayat, {len(similar['f'])} families, {len(themes)} theme entries, "
          f"{len(starts)} ruku'")


if __name__ == "__main__":
    main()
