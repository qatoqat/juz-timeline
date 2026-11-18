"""Build explore/assets/topics.json from the ontology (tools/data/ontology.json) and Content Sync, for tools/relations.py.

topics.json:
  t: [[qul_id, parent_index | -1, arabic, english]]
  a: [[ayah indices]] per concept, sorted (0-based, mushaf order, 0..6235).
Run: python tools/topics.py
"""
import json
import pathlib

from qf import quran

ROOT = pathlib.Path(__file__).resolve().parent
OUT = ROOT.parent / "explore" / "assets" / "topics.json"


def main():
    concepts = json.loads((ROOT / "data" / "ontology.json").read_text(encoding="utf-8"))
    slot = {v["verse_key"]: n for n, v in enumerate(quran().verses)}
    index = {c["id"]: i for i, c in enumerate(concepts)}
    missing = sorted({k for c in concepts for k in c["ayahs"] if k not in slot})
    assert not missing, f"ontology ayat not in Content Sync: {missing[:10]}"
    topics = {
        "t": [[c["id"], index.get(c["parent"], -1), c["arabic"], c["name"]] for c in concepts],
        "a": [sorted({slot[k] for k in c["ayahs"]}) for c in concepts],
    }
    OUT.write_text(json.dumps(topics, ensure_ascii=False, separators=(",", ":")), encoding="utf-8")
    print(f"{len(concepts)} concepts, {sum(len(a) for a in topics['a'])} concept ayat; topics.json {OUT.stat().st_size // 1024} KB")


if __name__ == "__main__":
    main()
