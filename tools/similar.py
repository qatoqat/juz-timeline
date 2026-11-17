"""Build explore/assets/similar.json: shared-phrase families and close variants.

Text: quran.com API (text_imlaei for display, text_imlaei_simple for matching), cached by tools/fetch_quran_com.sh.
Run: python tools/similar.py
"""
import collections
import json
import pathlib
import re

ROOT = pathlib.Path(__file__).resolve().parent
CACHE = ROOT / ".cache"
OUT = ROOT.parent / "explore" / "assets" / "similar.json"

MIN_WORDS, MAX_WORDS = 3, 30
MAX_MEMBERS = 12
COMMON_DF = 150
RARE_DF = 40
VARIANT_RATIO = 0.7
VARIANT_MIN_WORDS = 4


def is_mark(token):
    return all(0x064B <= ord(c) <= 0x065F or 0x06D6 <= ord(c) <= 0x06ED for c in token)


def load():
    """(surah, ayah, display words, plain words) from the quran.com cache (tools/fetch_quran_com.sh)."""
    rows = []
    for s in range(1, 115):
        for v in json.loads((CACHE / "quran-com" / f"{s}.json").read_text(encoding="utf-8"))["verses"]:
            shown = [w for w in v["text_imlaei"].split() if not is_mark(w)]
            plain = v["text_imlaei_simple"].split()
            assert len(shown) == len(plain), v["verse_key"]
            rows.append((s, v["verse_number"], shown, plain))
    return rows


def norm(w):
    return re.sub("[أإآٱ]", "ا", w).replace("ى", "ي")


def stem(w):
    return w[1:] if len(w) > 3 and w[0] in "وف" else w


def informative(g, df):
    return len(g) > 3 or any(df[w] <= COMMON_DF for w in g)


def phrase_families(words):
    df = collections.Counter(w for ws in words for w in set(ws))
    grams = {}
    for n in range(MIN_WORDS, MAX_WORDS + 1):
        index = collections.defaultdict(set)
        for i, ws in enumerate(words):
            for k in range(len(ws) - n + 1):
                index[tuple(ws[k:k + n])].add(i)
        grams[n] = {g: s for g, s in index.items() if len(s) >= 2}
    maximal = {}
    for n in range(MIN_WORDS, MAX_WORDS + 1):
        longer = collections.defaultdict(list)
        for g, s in grams.get(n + 1, {}).items():
            longer[g[:-1]].append(s)
            longer[g[1:]].append(s)
        for g, s in grams[n].items():
            if len(s) <= MAX_MEMBERS and informative(g, df) and not any(t == s for t in longer.get(g, [])):
                maximal[g] = s
    return maximal


def find(ws, g):
    return next(k for k in range(len(ws) - len(g) + 1) if tuple(ws[k:k + len(g)]) == g)


def lcs(a, b):
    row = [0] * (len(b) + 1)
    for x in a:
        prev = 0
        for j, y in enumerate(b):
            cur = row[j + 1]
            row[j + 1] = prev + 1 if x == y else max(row[j + 1], row[j])
            prev = cur
    return row[-1]


def aligned(a, b):
    n, m = len(a), len(b)
    t = [[0] * (m + 1) for _ in range(n + 1)]
    for i in range(n - 1, -1, -1):
        for j in range(m - 1, -1, -1):
            t[i][j] = t[i + 1][j + 1] + 1 if a[i] == b[j] else max(t[i + 1][j], t[i][j + 1])
    same_a, same_b, i, j = {}, {}, 0, 0
    while i < n and j < m:
        if a[i] == b[j]:
            same_a[i] = j
            same_b[j] = i
            i, j = i + 1, j + 1
        elif t[i + 1][j] >= t[i][j + 1]:
            i += 1
        else:
            j += 1
    return same_a, same_b


def variants(words, families):
    stems = [[stem(w) for w in ws] for ws in words]
    df = collections.Counter(w for ws in stems for w in set(ws))
    index = collections.defaultdict(list)
    for i, ws in enumerate(stems):
        for w in set(ws):
            if df[w] <= RARE_DF:
                index[w].append(i)
    shared = collections.Counter()
    for ids in index.values():
        for x in range(len(ids)):
            for y in range(x + 1, len(ids)):
                shared[ids[x], ids[y]] += 1
    covered = collections.defaultdict(int)
    for g, s in families.items():
        for a in s:
            for b in s:
                if a < b:
                    covered[a, b] = max(covered[a, b], len(g))
    out = []
    for (a, b), n in shared.items():
        la, lb = len(stems[a]), len(stems[b])
        if n < 2 or min(la, lb) < VARIANT_MIN_WORDS or words[a] == words[b]:
            continue
        common = lcs(stems[a], stems[b])
        ratio = 2 * common / (la + lb)
        if ratio >= VARIANT_RATIO and covered[a, b] < 0.8 * common:
            out.append((a, b, ratio))
    return out


def main():
    rows = load()
    words = [[norm(w) for w in plain] for _, _, _, plain in rows]
    keys = [[s, a] for s, a, _, _ in rows]
    text = [shown for _, _, shown, _ in rows]

    families = phrase_families(words)
    ordered = sorted(families.items(), key=lambda kv: (-len(kv[1]), -len(kv[0])))
    pairs = sorted(variants(words, families), key=lambda p: -p[2])

    used = sorted({i for _, m in ordered for i in m} | {i for a, b, _ in pairs for i in (a, b)})
    slot = {i: n for n, i in enumerate(used)}
    out = {"a": [keys[i] + [" ".join(text[i])] for i in used], "f": [], "v": []}
    for g, members in ordered:
        out["f"].append([len(g), [[slot[i], find(words[i], g)] for i in sorted(members)]])

    for a, b, ratio in pairs:
        same_a, same_b = aligned([stem(w) for w in words[a]], [stem(w) for w in words[b]])

        def flags(i, other, same):
            return "".join("*" if j not in same else "~" if words[i][j] != words[other][same[j]] else "." for j in range(len(text[i])))

        out["v"].append([round(ratio, 2), [[slot[a], flags(a, b, same_a)], [slot[b], flags(b, a, same_b)]]])

    OUT.write_text(json.dumps(out, ensure_ascii=False, separators=(",", ":")), encoding="utf-8")
    print(f"{len(used)} ayat, {len(families)} phrase families, {len(pairs)} variants, {OUT.stat().st_size // 1024} KB")


if __name__ == "__main__":
    main()
