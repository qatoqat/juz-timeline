"""Fingerprint the generated data so a deploy can tell when Quran Foundation data changed.

The lock (tools/data.lock.json) holds only hashes and counts, never the data itself.
  python tools/fingerprint.py           compare with the lock; exit 3 and list changes if they differ
  python tools/fingerprint.py --write   update the lock
"""
import hashlib
import json
import pathlib
import re
import sys

REPO = pathlib.Path(__file__).resolve().parent.parent
LOCK = REPO / "tools" / "data.lock.json"


def digest(value):
    return hashlib.sha256(json.dumps(value, ensure_ascii=False).encode()).hexdigest()[:16]


def components():
    rust = (REPO / "core" / "src" / "data.rs").read_text(encoding="utf-8")
    out = {}
    for name, body in re.findall(r"pub const (\w+): [^=]+= (.*?);\n", rust, re.S):
        if name != "DATA_FETCHED":
            out[name] = {"hash": digest(body), "count": body.count("(") or body.count(",") + 1}
    similar = json.loads((REPO / "explore" / "assets" / "similar.json").read_text(encoding="utf-8"))
    for key, name in [("a", "similar ayat"), ("f", "similar families"), ("v", "close variants")]:
        out[name] = {"hash": digest(similar[key]), "count": len(similar[key])}
    themes = json.loads((REPO / "explore" / "assets" / "themes.json").read_text(encoding="utf-8"))
    out["themes"] = {"hash": digest(themes), "count": len(themes)}
    topics = json.loads((REPO / "explore" / "assets" / "topics.json").read_text(encoding="utf-8"))
    for key, name in [("t", "concepts"), ("a", "concept ayat"), ("l", "concept links"), ("c", "concept co-occurrence")]:
        out[name] = {"hash": digest(topics[key]), "count": len(topics[key])}
    text = json.loads((REPO / "explore" / "assets" / "topic-ayat.json").read_text(encoding="utf-8"))
    out["concept ayah text"] = {"hash": digest(text), "count": len(text["k"])}
    rel = json.loads((REPO / "explore" / "assets" / "relations.json").read_text(encoding="utf-8"))
    for key, name in [("o", "theme openings"), ("t", "theme twins"), ("r", "refrains"), ("l", "theme links"), ("x", "relation ayah text")]:
        out[name] = {"hash": digest(rel[key]), "count": len(rel[key])}
    return out


def main():
    now = components()
    if "--write" in sys.argv:
        LOCK.write_text(json.dumps(now, indent=2) + "\n")
        return
    old = json.loads(LOCK.read_text()) if LOCK.exists() else {}
    changed = [n for n in sorted(now.keys() | old.keys()) if now.get(n) != old.get(n)]
    if not changed:
        print("data unchanged since the last deploy")
        return
    print("| Data | Before | After |\n|---|---|---|")
    for n in changed:
        count = lambda c: f"{c['count']} ({c['hash'][:8]})" if c else "missing"
        print(f"| {n} | {count(old.get(n))} | {count(now.get(n))} |")
    sys.exit(3)


if __name__ == "__main__":
    main()
