"""Build explore/assets/topics.json and topic-ayat.json from the ontology (tools/data/ontology.json) and Content Sync.

topics.json:
  t: [[qul_id, parent_index | -1, arabic, english, description, wikipedia_path]]
     description is plain text; links to other concepts are written {index|label}.
  a: [[ayah indices]] per concept, sorted (0-based, mushaf order, 0..6235).
  l: [[i, j, kind]] links between concepts outside the parent tree: 0 = named in i's description, 1 = listed as related.
  c: [[i, j, shared]] concepts that appear in the same ayat (Jaccard), strongest first per concept.
topic-ayat.json: {"k": [ayah indices], "t": [text]}, the Imla'i text of every ayah a concept appears in.
Run: python tools/topics.py
"""
import html
import json
import pathlib
import re

from qf import quran

ROOT = pathlib.Path(__file__).resolve().parent
ASSETS = ROOT.parent / "explore" / "assets"
# "mentioned in the Quran" closes almost every description, so links to the Quran concept are not relations.
BOILERPLATE = {61}
CO_PER_TOPIC = 6
CO_MIN_SHARED = 2


def targets(text, by_name):
    """Concept ids a description links to, by data-id or, failing that, by name."""
    found = [int(i) if i else by_name.get(label.lower()) for i, label in re.findall(r'<topic(?: data-id="(\d+)")?>([^<]*)</topic>', text)]
    return [t for t in found if t is not None]


def describe(text, index, by_name):
    text = re.sub(r"</?(b|span)[^>]*>", "", text)

    def link(m):
        target = int(m.group(1)) if m.group(1) else by_name.get(m.group(2).lower())
        label = html.unescape(m.group(2))
        return f"{{{index[target]}|{label}}}" if target in index else label

    text = re.sub(r'<topic(?: data-id="(\d+)")?>([^<]*)</topic>', link, text)
    assert "<" not in text, text
    return html.unescape(text).strip()


def lineage(i, parents):
    out = set()
    while parents[i] >= 0:
        i = parents[i]
        out.add(i)
    return out


def main():
    concepts = json.loads((ROOT / "data" / "ontology.json").read_text(encoding="utf-8"))
    q = quran()
    slot = {v["verse_key"]: n for n, v in enumerate(q.verses)}
    index = {c["id"]: i for i, c in enumerate(concepts)}
    by_name = {c["name"].lower(): c["id"] for c in concepts}
    parents = [index.get(c["parent"], -1) for c in concepts]
    missing = sorted({k for c in concepts for k in c["ayahs"] if k not in slot})
    assert not missing, f"ontology ayat not in Content Sync: {missing[:10]}"
    ayat = [sorted({slot[k] for k in c["ayahs"]}) for c in concepts]

    family = [lineage(i, parents) | {j for j in range(len(concepts)) if i in lineage(j, parents)} for i in range(len(concepts))]
    links = {}
    for i, c in enumerate(concepts):
        named = targets(c["description"], by_name)
        for kind, ids in [(0, named), (1, c["related"])]:
            for t in ids:
                j = index.get(t)
                if j is None or j == i or t in BOILERPLATE or j in family[i]:
                    continue
                pair = (min(i, j), max(i, j))
                links.setdefault(pair, (i, j, kind))

    sets = [set(a) for a in ayat]
    co = {}
    for i in range(len(concepts)):
        scored = []
        for j in range(len(concepts)):
            if j == i or j in family[i] or (min(i, j), max(i, j)) in links:
                continue
            shared = len(sets[i] & sets[j])
            if shared >= CO_MIN_SHARED:
                scored.append((shared / len(sets[i] | sets[j]), shared, j))
        for _, shared, j in sorted(scored, reverse=True)[:CO_PER_TOPIC]:
            co[(min(i, j), max(i, j))] = shared

    # Some source paths are placeholders (Samiri's points at the article "Wikipedia"); those are dropped.
    wiki = lambda w: "" if (path := re.sub(r"^(https?://)?en\.wikipedia\.org/+wiki/", "", w)).lower() in ("", "wikipedia") else path
    topics = {
        "t": [[c["id"], parents[i], c["arabic"], c["name"], describe(c["description"], index, by_name), wiki(c["wiki"])] for i, c in enumerate(concepts)],
        "a": ayat,
        "l": [list(v) for _, v in sorted(links.items())],
        "c": [[i, j, n] for (i, j), n in sorted(co.items(), key=lambda kv: -kv[1])],
    }
    used = sorted({a for list_ in ayat for a in list_})
    text = {"k": used, "t": [" ".join(q.verses[a]["words"]) for a in used]}
    for name, data in [("topics.json", topics), ("topic-ayat.json", text)]:
        (ASSETS / name).write_text(json.dumps(data, ensure_ascii=False, separators=(",", ":")), encoding="utf-8")
    size = lambda n: (ASSETS / n).stat().st_size // 1024
    print(f"{len(concepts)} concepts, {len(links)} links, {len(co)} co-occurrences, {len(used)} ayat; "
          f"topics.json {size('topics.json')} KB, topic-ayat.json {size('topic-ayat.json')} KB")


if __name__ == "__main__":
    main()
