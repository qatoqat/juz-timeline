"""Build explore/assets/relations.json: links between ayah themes, each with its Arabic evidence.

Cross-surah links score four signals per pair of themes, each scaled to 0..1:
  p  shared phrase families (similar.json), idf x sqrt(length/3), nested phrases counted once; P_FULL points = 1
  c  Quranic Arabic Corpus concepts (topics.json), tf-idf cosine smoothed with BETA so one mention weighs little
  h  heading words of the themes, tf-idf cosine with aliases (Moses -> musa)
  v  the passages' own words (light stem), tf-idf cosine; V_FULL = 1
A link is strong with two signals over their floors and a sum of SEED or more (or a twin), weaker with LOW or more and two
strong neighbours in common; each theme keeps its CAP best. Inside a surah only shared wording counts (tools/echoes.py).
Kinds: 0 story (a person or people shared), 1 wording, 2 subject, 3 echo inside one surah. See tools/twins.py for twins.

relations.json: v [ayat, families] of the similar.json it was built from; o opening words of each theme;
c [[arabic, english]] cited concepts; t twins [aFrom, aTo, bFrom, bTo, flagsA, flagsB, frame]; r refrains;
l links [i, j, kind, score 0-9, strong, [twins], [families], [concepts], [[ayahI, wordI, ayahJ, wordJ]]];
x [[ayah, text]] for shown ayat that similar.json lacks. Ayat are indices among the 6,236.
Run: python tools/relations.py
"""
import collections
import json
import math
import pathlib
import re

import echoes
import twins
from qf import plain, quran, vkey

ROOT = pathlib.Path(__file__).resolve().parent
ASSETS = ROOT.parent / "explore" / "assets"
BETA, P_FULL, V_FULL = 5.0, 24.0, 0.25
FLOOR = {"p": 0.25, "c": 0.2, "h": 0.15, "v": 0.3}
SEED, LOW, COMMON, MIN_SIGNALS = 0.9, 0.6, 2, 2
CAP, MIN_GAP, STORY_C, ECHO_FLOOR = 12, 2, 0.25, 6.0
EV_PHRASES, EV_CONCEPTS, EV_WORDS, EV_WORD_DF, OPEN_WORDS = 3, 2, 4, 60, 4
PEOPLE_ROOTS = ["Historic People", "Historic Person", "Messenger", "Prophet"]
NOT_PEOPLE = ["Muhammad"]
# Ontology tags that name someone absent from the passage (Shu'ayb's Madyan on Lut's story, Pharaoh on the Ibn Maryam
# passage). Ignored here only; tools/data/ontology.json keeps the source as it is.
FIXES = [("Madyan", (26, 160), (26, 173)), ("Madyan", (27, 54), (27, 57)), ("Pharaoh", (43, 57), (43, 86))]
STOP = set("""the and of to a in for who his is not be are their will those from on with that do they no has all one as him he
them it have but was there your against if shall were by this can its which what an or at about been only when how even
own did any most must after over who's whom then into out so may made make let being through i we our us my me
they're he's allah allah's al""".split())
ALIAS = {"moses": "musa", "moses'": "musa", "pharaoh": "fir'on", "pharaoh's": "fir'on", "fir'on's": "fir'on", "jesus": "isa",
         "mary": "maryem", "noah": "nuh", "abraham": "ibrahim", "salah's": "saleh", "saleh's": "saleh", "iblees": "shaitan",
         "satan": "shaitan", "joseph": "yusuf", "solomon": "sulaiman", "david": "dawood", "john": "yahya", "jonah": "yunus",
         "job": "ayub", "israelites": "israel", "jews": "israel", "a'd": "ad", "lut's": "lut", "nuh's": "nuh", "hud's": "hud",
         "musa's": "musa", "ibrahim's": "ibrahim", "adam's": "adam", "yusuf's": "yusuf", "shu'aib's": "shu'aib",
         "haroon": "harun", "aaron": "harun"}


def stem(w):
    w = w[1:] if len(w) > 3 and w[0] in "وف" else w
    w = w[1:] if len(w) > 4 and w[0] in "بلك" else w
    return w[2:] if len(w) > 4 and w.startswith("ال") else w


class Base:
    def __init__(self):
        self.verses = quran().verses
        self.keys = [tuple(v["key"]) for v in self.verses]
        self.slot = {k: i for i, k in enumerate(self.keys)}
        load = lambda name: json.loads((ASSETS / name).read_text(encoding="utf-8"))
        self.themes, self.similar, self.topics = load("themes.json"), load("similar.json"), load("topics.json")
        self.n = len(self.themes)
        self.theme_of = {}
        for t, (s, a, b, _, _) in enumerate(self.themes):
            for x in range(a, b + 1):
                self.theme_of[self.slot[(s, x)]] = t
        self.surah = [r[0] for r in self.themes]
        self.real = [r[3] != 2 for r in self.themes]
        self.span = [(self.slot[(s, a)], self.slot[(s, b)]) for s, a, b, _, _ in self.themes]
        self.ayah = [self.slot[(s, a)] for s, a, _ in self.similar["a"]]


def tfidf(docs, n_docs, min_df=1):
    df = collections.Counter(w for d in docs for w in set(d))
    vecs = [{w: (1 + math.log(k)) * math.log(n_docs / df[w]) for w, k in sorted(collections.Counter(d).items()) if df[w] >= min_df}
            for d in docs]
    return vecs, df


def cosine_pairs(vecs, beta=0.0):
    norm = [sum(x * x for x in v.values()) + beta * beta for v in vecs]
    inv = collections.defaultdict(list)
    for t, v in enumerate(vecs):
        for w, x in v.items():
            inv[w].append((t, x))
    dot = collections.defaultdict(float)
    for w in sorted(inv):
        post = inv[w]
        for x, (i, a) in enumerate(post):
            for j, b in post[x + 1:]:
                dot[(i, j)] += a * b
    return {p: s / math.sqrt(norm[p[0]] * norm[p[1]]) for p, s in dot.items() if norm[p[0]] and norm[p[1]]}


def phrase_signal(B):
    fam_len = [f[0] for f in B.similar["f"]]
    fam_at = []
    for _, members in B.similar["f"]:
        d = collections.defaultdict(list)
        for i, start in members:
            d[B.theme_of[B.ayah[i]]].append((B.ayah[i], start))
        fam_at.append(d)
    weight = [math.log(B.n / len(d)) * math.sqrt(fam_len[f] / 3) for f, d in enumerate(fam_at)]
    shared = collections.defaultdict(list)
    for f, d in enumerate(fam_at):
        ts = sorted(d)
        if 2 <= len(ts) <= 40:
            for x, i in enumerate(ts):
                for j in ts[x + 1:]:
                    shared[(i, j)].append(f)

    def inside(f, g, t):
        return all(any(a == b and gs <= s and s + fam_len[f] <= gs + fam_len[g] for b, gs in fam_at[g][t]) for a, s in fam_at[f][t])

    phrases = {}
    for (i, j), fams in shared.items():
        kept = []
        for f in sorted(fams, key=lambda f: (-fam_len[f], f)):
            if not any(inside(f, g, i) and inside(f, g, j) for g in kept):
                kept.append(f)
        kept.sort(key=lambda f: (-weight[f], f))
        phrases[(i, j)] = (min(sum(weight[f] for f in kept) / P_FULL, 1.0), kept)
    return phrases, fam_at


def signals(B):
    phrases, fam_at = phrase_signal(B)
    names = [row[3] for row in B.topics["t"]]
    parent = [row[1] for row in B.topics["t"]]
    people = set()
    for c in range(len(names)):
        p = parent[c]
        while p >= 0:
            if names[p] in PEOPLE_ROOTS:
                people.add(c)
            p = parent[p]
    people -= {names.index(x) for x in NOT_PEOPLE}
    people.add(names.index("Iblis"))
    bad = collections.defaultdict(set)
    for name, lo, hi in FIXES:
        bad[names.index(name)] |= set(range(B.slot[lo], B.slot[hi] + 1))
    tf = [collections.Counter() for _ in range(B.n)]
    for c, ayat in enumerate(B.topics["a"]):
        for a in ayat:
            if a not in bad[c]:
                tf[B.theme_of[a]][c] += 1
    cdf = collections.Counter(c for d in tf for c in d)
    cvec = [{c: (1 + math.log(k)) * math.log(B.n / cdf[c]) for c, k in sorted(d.items())} for d in tf]

    def heading(text):
        ws = [ALIAS.get(w, w) for w in re.findall(r"[a-z']+", text.lower())]
        return [w for w in ws if w not in STOP and len(w) > 2]

    hvec, _ = tfidf([heading(r[4]) if r[3] != 2 else [] for r in B.themes], B.n)
    docs = [[] for _ in range(B.n)]
    B.forms = [collections.defaultdict(dict) for _ in range(B.n)]
    for g, verse in enumerate(B.verses):
        t = B.theme_of[g]
        for wi, w in enumerate(verse["words"]):
            k = stem(plain(w))
            docs[t].append(k)
            B.forms[t][k].setdefault(vkey(w), (g, wi))
    vvec, vdf = tfidf(docs, B.n, min_df=2)
    sig = {
        "p": {k: v[0] for k, v in phrases.items()},
        "c": cosine_pairs(cvec, BETA),
        "h": cosine_pairs(hvec),
        "v": {k: min(s / V_FULL, 1.0) for k, s in cosine_pairs(vvec).items() if s >= 0.05},
    }
    return dict(sig=sig, phrases=phrases, fam_at=fam_at, cvec=cvec, vvec=vvec, vdf=vdf, people=people)


def cross_links(B, S, forced):
    feats = {}
    for i, j in sorted(set().union(*S["sig"].values()) | forced):
        if not (B.real[i] and B.real[j]) or B.surah[i] == B.surah[j]:
            continue
        f = {k: S["sig"][k].get((i, j), 0.0) for k in "pchv"}
        f["s"] = sum(f.values())
        f["n"] = sum(1 for k in "pchv" if f[k] >= FLOOR[k])
        f["twin"] = (i, j) in forced
        if (f["n"] >= MIN_SIGNALS and f["s"] >= LOW) or f["twin"]:
            feats[(i, j)] = f
    seeds = {p for p, f in feats.items() if (f["s"] >= SEED and f["n"] >= MIN_SIGNALS) or f["twin"]}
    near = collections.defaultdict(set)
    for i, j in seeds:
        near[i].add(j)
        near[j].add(i)
    links = {p for p in feats if p in seeds or len(near[p[0]] & near[p[1]]) >= COMMON}
    ranked = collections.defaultdict(list)
    for i, j in sorted(links):
        key = (-(feats[(i, j)]["s"] + (2 if feats[(i, j)]["twin"] else 0)),)
        ranked[i].append((key, j))
        ranked[j].append((key, i))
    kept = {(min(i, j), max(i, j)) for i, lst in ranked.items() for _, j in sorted(lst)[:CAP]}
    kept |= {p for p in forced if p in feats}
    return feats, seeds, kept


def kind_of(S, f, i, j):
    cvec = S["cvec"]
    both = sorted(((cvec[i][c] * cvec[j][c], c) for c in cvec[i] if c in cvec[j]), key=lambda x: (-x[0], x[1]))
    top = [c for _, c in both[:EV_CONCEPTS]] if f["c"] >= FLOOR["c"] else []
    if f["c"] >= STORY_C and any(c in S["people"] for c in top):
        top.sort(key=lambda c: c not in S["people"])
        return 0, top
    if f["p"] >= FLOOR["p"] and f["p"] >= max(f["c"], f["h"]):
        return 1, top
    return 2, top


def evidence_words(B, S, i, j, taken):
    """Up to EV_WORDS shared words (light stem scored) with an identically vowelled form in both passages."""
    vvec, vdf = S["vvec"], S["vdf"]
    cand = sorted(((vvec[i][w] * vvec[j][w], w) for w in vvec[i] if w in vvec[j] and vdf[w] <= EV_WORD_DF), key=lambda x: (-x[0], x[1]))
    out = []
    for _, w in cand:
        if len(out) >= EV_WORDS:
            break
        fa, fb = B.forms[i].get(w, {}), B.forms[j].get(w, {})
        common = sorted(set(fa) & set(fb))
        if not common:
            continue
        a, b = fa[common[0]], fb[common[0]]
        if (a[0], b[0]) not in taken:
            out.append([a[0], a[1], b[0], b[1]])
    return out


def build():
    B = Base()
    S = signals(B)
    echoes.prepare(B)
    twin_rows = twins.build_twins(B)
    refrain_rows = twins.refrains(B)
    forced = {r["pair"] for r in twin_rows if B.real[r["pair"][0]] and B.real[r["pair"][1]] and B.surah[r["pair"][0]] != B.surah[r["pair"][1]]}
    feats, seeds, kept = cross_links(B, S, forced)
    twins_of = collections.defaultdict(list)
    for k, r in enumerate(twin_rows):
        twins_of[r["pair"]].append(k)
    concept_ids, rows = {}, []
    for i, j in sorted(kept):
        f = feats[(i, j)]
        kind, top = kind_of(S, f, i, j)
        fams = S["phrases"][(i, j)][1][:EV_PHRASES] if (i, j) in S["phrases"] else []
        taken = {(a, b) for fam in fams for a, _ in S["fam_at"][fam][i] for b, _ in S["fam_at"][fam][j]}
        rows.append([i, j, kind, min(9, int(f["s"] * 3)), 1 if (i, j) in seeds else 0, twins_of.get((i, j), []), fams,
                     [concept_ids.setdefault(c, len(concept_ids)) for c in top], evidence_words(B, S, i, j, taken)])
    echo_rows = {}
    for s in range(1, 115):
        for score, x, y, ev in echoes.echoes(B, s):
            if score >= ECHO_FLOOR:
                echo_rows[(x, y)] = [x, y, 3, min(9, int(score / 3)), 1, [], [e[6] for e in ev if e[1] == "p"], [],
                                     [[e[3][0], e[3][1], e[4][0], e[4][1]] for e in ev if e[1] == "w"]]
    for k, r in enumerate(twin_rows):
        i, j = r["pair"]
        if B.real[i] and B.real[j] and B.surah[i] == B.surah[j] and j - i >= MIN_GAP:
            echo_rows.setdefault((i, j), [i, j, 3, 9, 1, [], [], [], []])[5].append(k)
    rows = sorted(rows + list(echo_rows.values()))
    last = {g for g in range(len(B.keys)) if g + 1 == len(B.keys) or B.keys[g + 1][0] != B.keys[g][0]}
    need = set()
    for r in twin_rows:
        for lo, hi in (r["a"], r["b"]):
            need.update(range(lo, hi + 1))
            if hi not in last:
                need.add(hi + 1)
    for _, members in refrain_rows:
        for g, _ in members:
            need.add(g)
            if g not in last:
                need.add(g + 1)
    need |= {g for row in rows for a, _, b, _ in row[8] for g in (a, b)}
    extra = sorted(need - set(B.ayah))
    return {
        "v": [len(B.similar["a"]), len(B.similar["f"])],
        "o": [" ".join(B.verses[B.span[t][0]]["words"][:OPEN_WORDS]) for t in range(B.n)],
        "c": [[B.topics["t"][c][2], B.topics["t"][c][3]] for c in concept_ids],
        "t": [[r["a"][0], r["a"][1], r["b"][0], r["b"][1], r["fa"], r["fb"], r["frame"]] for r in twin_rows],
        "r": refrain_rows,
        "l": rows,
        "x": [[g, " ".join(B.verses[g]["words"])] for g in extra],
    }


def main():
    data = build()
    text = json.dumps(data, ensure_ascii=False, separators=(",", ":"))
    (ASSETS / "relations.json").write_text(text, encoding="utf-8")
    kinds = collections.Counter(r[2] for r in data["l"])
    print(f"{len(data['t'])} twins, {len(data['r'])} refrains, {len(data['l'])} links (story {kinds[0]}, wording {kinds[1]}, "
          f"subject {kinds[2]}, echo {kinds[3]}), {len(data['x'])} extra ayat; relations.json {len(text.encode()) // 1024} KB")


if __name__ == "__main__":
    main()
