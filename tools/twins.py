"""Twin passages and refrains for tools/relations.py.

A twin is two stretches of ayat whose shared phrases (similar.json families) occur in the same order, at most GAP ayat
apart on each side, word-aligned and kept when enough of the shorter stretch matches. Each word gets a flag:
. same, ^ same letters with other vowels, ~ same but for a leading wa/fa, * unmatched.
A refrain is a phrase that mostly lives in one surah, repeated at the start or end of 3+ of its ayat.
"""
import collections
import re

from qf import plain
from similar import aligned, norm, stem

GAP, MIN_RUNGS, MIN_RUNG_WORDS, MIN_SAME, MIN_DENSITY = 3, 2, 10, 10, 0.30
REFRAIN_N, REFRAIN_MAX_N, REFRAIN_MIN_AYAT = 3, 30, 3
WAQF = re.compile("[ۖ-ۭـ  ]")


def voc(w):
    """Vowelled form: waqf signs and tatweel dropped, and an assimilation shadda on the first letter."""
    return re.sub("^(.)ّ", r"\1", WAQF.sub("", w))


class Words:
    def __init__(self, B):
        self.B = B
        self.shown = [v["words"] for v in B.verses]
        self.match = [[stem(norm(plain(w))) for w in ws] for ws in self.shown]
        self.exact = [[norm(plain(w)) for w in ws] for ws in self.shown]
        self.surah = [k[0] for k in B.keys]


def anchors(B):
    out = []
    for f, (length, members) in enumerate(B.similar["f"]):
        occ = sorted((B.ayah[m], start) for m, start in members)
        for x in range(len(occ)):
            for y in range(x + 1, len(occ)):
                if occ[x][0] != occ[y][0]:
                    out.append((occ[x][0], occ[x][1], occ[y][0], occ[y][1], length, f))
    out.sort()
    return out


def chain_scores(remaining, surah):
    """Best chain ending at each anchor: anchors in order on both sides, each side within one surah and GAP ayat."""
    best = [None] * len(remaining)
    by_g1 = collections.defaultdict(list)
    for k, (g1, w1, g2, w2, ln, _) in enumerate(remaining):
        score, prev, first = ln, None, g2
        for d in range(GAP + 1):
            for p in by_g1.get(g1 - d, ()):
                a1, x1, a2, x2, aln, _ = remaining[p]
                if surah[a1] != surah[g1] or surah[a2] != surah[g2] or not 0 <= g2 - a2 <= GAP:
                    continue
                if (a1, x1 + aln) > (g1, w1) or (a2, x2 + aln) > (g2, w2) or g1 >= best[p][2]:
                    continue
                if best[p][0] + ln > score:
                    score, prev, first = best[p][0] + ln, p, best[p][2]
        best[k] = (score, prev, first)
        by_g1[g1].append(k)
    return best


def runs(B, surah):
    """Chains taken greedily by phrase words; anchors between a taken chain's two stretches are dropped."""
    remaining = anchors(B)
    found = []
    while True:
        best = chain_scores(remaining, surah)
        order = sorted(range(len(remaining)), key=lambda k: -best[k][0])
        by_g1 = collections.defaultdict(list)
        for k, a in enumerate(remaining):
            by_g1[a[0]].append(k)
        blocked, took = set(), False
        for k in order:
            chain, x = [], k
            while x is not None:
                chain.append(x)
                x = best[x][1]
            if len(chain) < 2 or any(c in blocked for c in chain):
                continue
            rungs = [remaining[c] for c in chain[::-1]]
            a_lo, a_hi = min(r[0] for r in rungs), max(r[0] for r in rungs)
            b_lo, b_hi = min(r[2] for r in rungs), max(r[2] for r in rungs)
            for g in range(a_lo, a_hi + 1):
                for c in by_g1[g]:
                    if b_lo <= remaining[c][2] <= b_hi:
                        blocked.add(c)
            found.append(rungs)
            took = True
        if not took:
            return found
        remaining = [a for k, a in enumerate(remaining) if k not in blocked]


def side(chain, which):
    gs = [a[0] if which == 0 else a[2] for a in chain]
    return min(gs), max(gs)


def ladder(W, chain):
    """Word alignment of the two stretches; isolated one-word matches are dropped as noise."""
    (a_lo, a_hi), (b_lo, b_hi) = side(chain, 0), side(chain, 1)
    A = [(g, k) for g in range(a_lo, a_hi + 1) for k in range(len(W.shown[g]))]
    Bw = [(g, k) for g in range(b_lo, b_hi + 1) for k in range(len(W.shown[g]))]
    same_a, _ = aligned([W.match[g][k] for g, k in A], [W.match[g][k] for g, k in Bw])
    keep = {i: j for i, j in same_a.items() if same_a.get(i - 1) == j - 1 or same_a.get(i + 1) == j + 1}
    return A, Bw, keep


def flags(W, chain):
    A, Bw, keep = ladder(W, chain)
    fa, fb = ["*"] * len(A), ["*"] * len(Bw)
    for x, y in keep.items():
        (ga, ka), (gb, kb) = A[x], Bw[y]
        if W.exact[ga][ka] != W.exact[gb][kb]:
            f = "~"
        elif voc(W.shown[ga][ka]) != voc(W.shown[gb][kb]):
            f = "^"
        else:
            f = "."
        fa[x] = fb[y] = f
    return "".join(fa), "".join(fb), len(keep), len(A), len(Bw)


def frames(B, rows):
    """Sides of same-surah twins that overlap by half of the shorter are one stretch; 3+ linked stretches form a frame."""
    sides = []
    for k, r in enumerate(rows):
        if B.keys[r["a"][0]][0] == B.keys[r["b"][0]][0]:
            sides.append((r["a"], k, 0))
            sides.append((r["b"], k, 1))
    parent = list(range(len(sides)))

    def find(x):
        while parent[x] != x:
            parent[x] = parent[parent[x]]
            x = parent[x]
        return x

    for x in range(len(sides)):
        for y in range(x + 1, len(sides)):
            (a, b), (c, d) = sides[x][0], sides[y][0]
            inter = min(b, d) - max(a, c) + 1
            if inter > 0 and inter >= 0.5 * min(b - a + 1, d - c + 1):
                parent[find(x)] = find(y)
    stretch = {(k, which): find(x) for x, (_, k, which) in enumerate(sides)}
    group = {s: s for s in set(stretch.values())}

    def gfind(x):
        while group[x] != x:
            group[x] = group[group[x]]
            x = group[x]
        return x

    for k in range(len(rows)):
        if (k, 0) in stretch:
            group[gfind(stretch[(k, 0)])] = gfind(stretch[(k, 1)])
    members = collections.defaultdict(set)
    for s in set(stretch.values()):
        members[gfind(s)].add(s)
    first = {g: min(sides[x][0] for x in range(len(sides)) if gfind(find(x)) == g) for g in members}
    frame_id = {}
    for g in sorted(members, key=lambda g: first[g]):
        if len(members[g]) >= 3:
            frame_id[g] = len(frame_id)
    for k, r in enumerate(rows):
        r["frame"] = frame_id.get(gfind(stretch[(k, 0)]), -1) if (k, 0) in stretch else -1


def build_twins(B):
    W = Words(B)
    kept = []
    for chain in runs(B, W.surah):
        if len(chain) < MIN_RUNGS or sum(a[4] for a in chain) < MIN_RUNG_WORDS:
            continue
        fa, fb, same, na, nb = flags(W, chain)
        if same / min(na, nb) >= MIN_DENSITY:
            kept.append((chain, fa, fb, same))
    kept.sort(key=lambda x: -x[3])
    rows = []
    for chain, fa, fb, same in kept:
        if same < MIN_SAME:
            continue
        ra, rb = collections.Counter(), collections.Counter()
        for g1, _, g2, _, ln, _ in chain:
            ra[B.theme_of[g1]] += ln
            rb[B.theme_of[g2]] += ln
        i = sorted(ra.items(), key=lambda x: (-x[1], x[0]))[0][0]
        j = sorted(rb.items(), key=lambda x: (-x[1], x[0]))[0][0]
        rows.append({"a": side(chain, 0), "b": side(chain, 1), "fa": fa, "fb": fb, "pair": (min(i, j), max(i, j))})
    rows.sort(key=lambda r: (r["a"], r["b"]))
    frames(B, rows)
    return rows


def refrains(B):
    """[[length, [[ayah, start word]]]] for phrases a surah repeats at the start or end of 3+ of its ayat."""
    words = [[plain(w) for w in v["words"]] for v in B.verses]
    by_surah = collections.defaultdict(list)
    for g, (s, _) in enumerate(B.keys):
        by_surah[s].append(g)
    total = collections.Counter()
    for ws in words:
        total.update({tuple(ws[k:k + n]) for n in range(REFRAIN_N, min(REFRAIN_MAX_N, len(ws)) + 1) for k in range(len(ws) - n + 1)})
    rows = []
    for s in range(1, 115):
        occ = collections.defaultdict(list)
        for g in by_surah[s]:
            ws = words[g]
            for n in range(REFRAIN_N, min(REFRAIN_MAX_N, len(ws)) + 1):
                for k in range(len(ws) - n + 1):
                    occ[tuple(ws[k:k + n])].append((g, k))
        cands = {gram: o for gram, o in occ.items() if len({g for g, _ in o}) >= REFRAIN_MIN_AYAT}
        for gram, o in cands.items():
            ayat = frozenset(g for g, _ in o)
            n, (g0, k0) = len(gram), o[0]
            ws = words[g0]
            if any(a >= 0 and b <= len(ws) and tuple(ws[a:b]) in cands and frozenset(x for x, _ in cands[tuple(ws[a:b])]) == ayat
                   for a, b in ((k0 - 1, k0 + n), (k0, k0 + n + 1))):
                continue
            count = len(ayat)
            whole = sum(1 for g, k in o if k == 0 and n == len(words[g]))
            edge = max(sum(1 for g, k in o if k + n == len(words[g])), sum(1 for g, k in o if k == 0)) / count
            if count / total[gram] >= 0.6 and edge >= 0.8 and (n >= 4 or whole == count):
                rows.append((s, count, gram, sorted(ayat), o[0]))
    rows.sort(key=lambda x: (x[0], -x[1] * len(x[2]), x[4]))
    kept = []
    for s, count, gram, ayat, first in rows:
        text = " ".join(gram)
        if not any(s2 == s and len(set(ayat) & set(a2)) >= 0.6 * count and (text in " ".join(g2) or " ".join(g2) in text)
                   for s2, _, g2, a2, _ in kept):
            kept.append((s, count, gram, ayat, first))
    out = []
    for _, _, gram, ayat, _ in kept:
        n = len(gram)
        out.append([n, [[g, next(x for x in range(len(words[g]) - n + 1) if tuple(words[g][x:x + n]) == gram)] for g in ayat]])
    return out
