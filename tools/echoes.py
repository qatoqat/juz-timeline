"""In-surah echoes for tools/relations.py: two themes of one surah, at least two apart, that share wording.

A unit is a similar.json phrase family with members in 2+ themes of the surah, or a word (vowel-aware key, 3+ letters)
found in at most RARE_AYAT ayat of the whole Quran. A unit's themes are grouped into runs of neighbours; each pair of
runs gets a vote. A pair is kept with a 4+ word phrase, two phrases, a unit found nowhere else in the Quran, or a
two-place unit plus another; at most CAP echoes per theme.
"""
import collections
import math

from qf import vkey

RARE_AYAT, MAX_RUNS, CAP = 10, 6, 4


def prepare(B):
    occ = collections.defaultdict(list)
    for g, v in enumerate(B.verses):
        for wi, w in enumerate(v["words"]):
            k = vkey(w)
            if sum(1 for c in k if "ء" <= c <= "ي") >= 3:
                occ[k].append((g, wi))
    B.word_occ = occ
    B.word_ayat = {k: len({g for g, _ in o}) for k, o in occ.items()}
    B.word_themes = {k: len({B.theme_of[g] for g, _ in o}) for k, o in occ.items()}
    B.fam_themes = [len({B.theme_of[B.ayah[m]] for m, _ in members}) for _, members in B.similar["f"]]


def neighbour_runs(themes):
    out = []
    for t in themes:
        if out and t - out[-1][-1] <= 1:
            out[-1].append(t)
        else:
            out.append([t])
    return out


def units(B, s):
    """(kind, id, weight, {theme: (ayah, word, length)}, share of the unit's occurrences that are in this surah)"""
    out = []
    for k in sorted(B.word_occ):
        if B.word_ayat[k] > RARE_AYAT:
            continue
        here = [(g, wi) for g, wi in B.word_occ[k] if B.keys[g][0] == s]
        if len({g for g, _ in here}) < 2:
            continue
        by = {}
        for g, wi in here:
            by.setdefault(B.theme_of[g], (g, wi, 1))
        if len(by) >= 2:
            out.append(("w", k, math.log(B.n / B.word_themes[k]), by, len({g for g, _ in here}) / B.word_ayat[k]))
    for f, (length, members) in enumerate(B.similar["f"]):
        ms = [(B.ayah[a], st) for a, st in members if B.keys[B.ayah[a]][0] == s]
        if len(ms) < 2:
            continue
        by = {}
        for g, st in sorted(ms):
            by.setdefault(B.theme_of[g], (g, st, length))
        if len(by) >= 2:
            out.append(("p", f, math.log(B.n / B.fam_themes[f]) * (1 + 0.25 * (length - 3)), by, len(ms) / len(members)))
    return out


def echoes(B, s):
    """[(score, theme x, theme y, evidence)] where evidence is up to four (vote, kind, runs, at x, at y, share, unit id)."""
    votes = collections.defaultdict(list)
    for kind, uid, w, by, share in units(B, s):
        rs = neighbour_runs(sorted(by))
        k = len(rs)
        if k > MAX_RUNS:
            continue
        for a in range(k):
            for b in range(a + 1, k):
                x, y = rs[a][0], rs[b][0]
                if y - rs[a][-1] >= 2 and B.real[x] and B.real[y]:
                    votes[(x, y)].append((w * 2 / k, kind, k, by[x], by[y], share, uid))
    out = []
    for (x, y), ev in sorted(votes.items()):
        ev.sort(key=lambda e: (-e[0], e[1], str(e[6])))
        phrases = [(e[3][0], e[3][1], e[3][1] + e[3][2]) for e in ev if e[1] == "p"]
        ev = [e for e in ev if e[1] == "p" or not any(e[3][0] == g and lo <= e[3][1] < hi for g, lo, hi in phrases)]
        seen, unique = set(), []
        for e in sorted(ev, key=lambda e: (e[1] != "p", -e[3][2], -e[0], str(e[6]))):
            if e[1] == "p":
                if (e[3][0], e[4][0]) in seen:
                    continue
                seen.add((e[3][0], e[4][0]))
            unique.append(e)
        ev = sorted(unique, key=lambda e: (-e[0], e[1], str(e[6])))
        long_phrase = any(e[1] == "p" and e[3][2] >= 4 for e in ev) or sum(1 for e in ev if e[1] == "p") >= 2
        exclusive = any(e[5] >= 0.999 and e[2] == 2 for e in ev)
        two_place = any(e[2] == 2 for e in ev) and len(ev) >= 2
        if long_phrase or exclusive or two_place:
            out.append((sum(e[0] for e in ev[:4]), x, y, ev[:4]))
    out.sort(key=lambda e: (-e[0], e[1], e[2]))
    degree, keep = collections.Counter(), []
    for e in out:
        if degree[e[1]] < CAP or degree[e[2]] < CAP:
            keep.append(e)
            degree[e[1]] += 1
            degree[e[2]] += 1
    return keep
