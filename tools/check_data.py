"""Validate generated data against the Content Sync snapshots; exits non-zero on any conflict.

Run: python tools/check_data.py
"""
import collections
import json
import pathlib
import subprocess
import sys

from qf import plain, quran, vkey
from similar import norm, stem
from twins import voc

ROOT = pathlib.Path(__file__).resolve().parent
REPO = ROOT.parent
errors = []


def expect(ok, message):
    if not ok:
        errors.append(message)


def check_twin(k, twin, words, surah):
    a0, a1, b0, b1, fa, fb, frame = twin
    N = len(words)
    ok = 0 <= a0 <= a1 < N and 0 <= b0 <= b1 < N and surah[a0] == surah[a1] and surah[b0] == surah[b1] and (a1 < b0 or b1 < a0)
    expect(ok and frame >= -1, f"relations: twin {k} has a bad stretch")
    if not ok:
        return
    A = [w for g in range(a0, a1 + 1) for w in words[g]]
    B = [w for g in range(b0, b1 + 1) for w in words[g]]
    expect(len(fa) == len(A) and len(fb) == len(B) and set(fa + fb) <= set(".^~*"), f"relations: twin {k} flags do not fit its words")
    marks = [f for f in fa if f != "*"]
    expect(marks == [f for f in fb if f != "*"], f"relations: twin {k} sides disagree on matched words")
    for x, y, f in zip([w for w, f in zip(A, fa) if f != "*"], [w for w, f in zip(B, fb) if f != "*"], marks):
        px, py = norm(plain(x)), norm(plain(y))
        good = (f == "." and px == py and voc(x) == voc(y)) or (f == "^" and px == py and voc(x) != voc(y)) or \
            (f == "~" and px != py and stem(px) == stem(py))
        expect(good, f"relations: twin {k} pairs {x} with {y} as '{f}'")
    expect(len(marks) >= 10 and len(marks) >= 0.3 * min(len(A), len(B)), f"relations: twin {k} is below the twin threshold")


def check_relations(verses, themes, similar, topics):
    rel = json.loads((REPO / "explore" / "assets" / "relations.json").read_text(encoding="utf-8"))
    gold = json.loads((ROOT / "data" / "relations_gold.json").read_text(encoding="utf-8"))
    words = [v["words"] for v in verses]
    surah = [v["key"][0] for v in verses]
    slot = {tuple(v["key"]): i for i, v in enumerate(verses)}
    T = len(themes)
    span = [(slot[(s, a)], slot[(s, b)]) for s, a, b, _, _ in themes]
    inside = lambda g, t: span[t][0] <= g <= span[t][1]
    touches = lambda lo, hi, t: lo <= span[t][1] and hi >= span[t][0]
    expect(list(rel) == ["v", "o", "c", "t", "r", "l", "x"], "relations: unexpected keys")
    expect(rel["v"] == [len(similar["a"]), len(similar["f"])], "relations: built from another similar.json")
    expect(rel["o"] == [" ".join(words[span[t][0]][:4]) for t in range(T)], "relations: theme openings differ from Content Sync")
    order = [tuple(t[:4]) for t in rel["t"]]
    expect(order == sorted(set(order)), "relations: twins not sorted and unique")
    for k, twin in enumerate(rel["t"]):
        check_twin(k, twin, words, surah)
    for k, (length, members) in enumerate(rel["r"]):
        ayat = [g for g, _ in members]
        expect(len(set(ayat)) >= 3 and len({surah[g] for g in ayat}) == 1 and ayat == sorted(ayat), f"relations: refrain {k} is not 3+ ayat of one surah")
        grams = {tuple(plain(w) for w in words[g][st:st + length]) for g, st in members}
        expect(len(grams) == 1 and all(len(x) == length for x in grams), f"relations: refrain {k} wording differs between its ayat")
    ayah = [slot[(s, a)] for s, a, _ in similar["a"]]
    fam = [[ayah[i] for i, _ in members] for _, members in similar["f"]]
    concept = {(row[2], row[3]): set(topics["a"][c]) for c, row in enumerate(topics["t"])}
    expect(all(tuple(c) in concept for c in rel["c"]), "relations: concept not in topics.json")
    pairs = [(r[0], r[1]) for r in rel["l"]]
    expect(pairs == sorted(set(pairs)), "relations: links not sorted and unique")
    bad = missing = 0
    for i, j, kind, score, strong, tw, fams, cons, ws in rel["l"]:
        expect(0 <= i < j < T and themes[i][3] != 2 and themes[j][3] != 2, f"relations: bad pair {i},{j}")
        same = themes[i][0] == themes[j][0]
        expect((kind == 3) == same and (not same or j - i >= 2), f"relations: link {i},{j} kind {kind} does not fit its surahs")
        expect(kind in (0, 1, 2, 3) and 0 <= score <= 9 and strong in (0, 1), f"relations: bad fields {i},{j}")
        missing += not (tw or fams or ws or (cons and kind != 3))
        for k in tw:
            a0, a1, b0, b1 = rel["t"][k][:4]
            bad += not ((touches(a0, a1, i) and touches(b0, b1, j)) or (touches(a0, a1, j) and touches(b0, b1, i)))
        for f in fams:
            bad += not (any(inside(g, i) for g in fam[f]) and any(inside(g, j) for g in fam[f]))
        for c in cons:
            a = concept[tuple(rel["c"][c])]
            bad += not (any(inside(g, i) for g in a) and any(inside(g, j) for g in a))
        for ga, wa, gb, wb in ws:
            bad += not (inside(ga, i) and inside(gb, j) and vkey(words[ga][wa]) == vkey(words[gb][wb]))
    expect(missing == 0, f"relations: {missing} links without evidence")
    expect(bad == 0, f"relations: {bad} evidence items do not hold in their passages")
    xs = [g for g, _ in rel["x"]]
    expect(xs == sorted(set(xs)) and not set(xs) & set(ayah), "relations: extra ayat not sorted, unique and absent from similar.json")
    expect(all(t == " ".join(words[g]) for g, t in rel["x"]), "relations: extra text differs from Content Sync")
    shown = {g for t in rel["t"] for lo, hi in ((t[0], t[1]), (t[2], t[3])) for g in range(lo, hi + 1)}
    shown |= {g for _, m in rel["r"] for g, _ in m} | {g for r in rel["l"] for w in r[8] for g in (w[0], w[2])}
    ends = {t[1] for t in rel["t"]} | {t[3] for t in rel["t"]} | {g for _, m in rel["r"] for g, _ in m}
    shown |= {g + 1 for g in ends if g + 1 < len(words) and surah[g + 1] == surah[g]}
    expect(shown <= set(xs) | set(ayah), f"relations: {len(shown - set(xs) - set(ayah))} shown ayat have no text")
    theme_at = {(s, x): t for t, (s, a, b, _, _) in enumerate(themes) for x in range(a, b + 1)}
    near = collections.defaultdict(set)
    for i, j in pairs:
        near[i].add(j)
        near[j].add(i)
    hit = total = 0
    for tells in gold["stories"].values():
        tells = [{theme_at[tuple(r)] for r in tell} for tell in tells]
        for x, tell in enumerate(tells):
            for i in tell:
                for y, other in enumerate(tells):
                    if y != x and i not in other:
                        total += 1
                        hit += bool(near[i] & other)
    expect(hit / total >= gold["min_recall"], f"relations: story recall {hit}/{total} fell below {gold['min_recall']}")
    linked = set(pairs)
    pair_of = lambda a, b: tuple(sorted((theme_at[tuple(a)], theme_at[tuple(b)])))
    for a, b in gold["must"]:
        expect(pair_of(a, b) in linked, f"relations: expected link {a} <-> {b} is missing")
    for a, b in gold["must_not"]:
        expect(pair_of(a, b) not in linked, f"relations: false friend {a} <-> {b} is linked")
    return len(pairs), len(rel["t"]), hit, total


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

    topics = json.loads((REPO / "explore" / "assets" / "topics.json").read_text(encoding="utf-8"))
    text = json.loads((REPO / "explore" / "assets" / "topic-ayat.json").read_text(encoding="utf-8"))
    n = len(topics["t"])
    expect(len(topics["a"]) == n, "topics: ayat list count differs from concept count")
    for i, (_, parent, _, english, _, _) in enumerate(topics["t"]):
        seen, p = {i}, parent
        while p >= 0:
            expect(p < n and p not in seen, f"topics: bad parent chain at {english}")
            if p >= n or p in seen:
                break
            seen.add(p)
            p = topics["t"][p][1]
    flat = [a for ayat in topics["a"] for a in ayat]
    expect(all(0 <= a < len(verses) for a in flat), "topics: ayah index out of range")
    expect(all(ayat == sorted(set(ayat)) for ayat in topics["a"]), "topics: ayat not sorted and unique")
    expect(all(0 <= i < n and 0 <= j < n and i != j for i, j, *_ in topics["l"] + topics["c"]), "topics: link to an unknown concept")
    expect(text["k"] == sorted(set(flat)) and len(text["t"]) == len(text["k"]), "topic-ayat.json does not match the concepts' ayat")
    expect(all(t == " ".join(verses[k]["words"]) for k, t in zip(text["k"], text["t"])), "topic-ayat.json text differs from Content Sync")

    links, twin_count, hit, total = check_relations(verses, themes, similar, topics)

    if errors:
        print("data check failed:")
        for e in errors[:40]:
            print("  -", e)
        sys.exit(1)
    print(f"data check passed: {len(verses)} ayat, {len(similar['f'])} families, {len(themes)} theme entries, "
          f"{len(starts)} ruku', {n} concepts, {links} theme links ({twin_count} twins, story recall {hit}/{total})")


if __name__ == "__main__":
    main()
