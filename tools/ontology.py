"""Extract the Quranic Arabic Corpus ontology concepts from QUL's ayah-topics SQLite into tools/data/ontology.json.

Only the ontology rows are kept (Quranic Arabic Corpus, corpus.quran.com, GNU GPL); QUL's thematic and
general topics come from other sources and are left out.
Run: python tools/ontology.py [path/to/topics.db.zip]
"""
import io
import json
import pathlib
import sqlite3
import sys
import tempfile
import zipfile

ROOT = pathlib.Path(__file__).resolve().parent
SOURCE = ROOT / ".cache" / "topics.db.zip"
OUT = ROOT / "data" / "ontology.json"


def rows(path):
    with zipfile.ZipFile(path) as z, tempfile.TemporaryDirectory() as tmp:
        db = pathlib.Path(tmp) / "topics.db"
        db.write_bytes(z.read("topics.db"))
        con = sqlite3.connect(db)
        con.row_factory = sqlite3.Row
        out = [dict(r) for r in con.execute("select * from topics where ontology = 1 order by topic_id")]
        con.close()
        return out


def keys(text):
    return [k.strip() for k in (text or "").split(",") if k.strip()]


def main():
    concepts = []
    for r in rows(sys.argv[1] if len(sys.argv) > 1 else SOURCE):
        concepts.append({
            "id": r["topic_id"],
            "name": r["name"],
            "arabic": r["arabic_name"] or "",
            "parent": r["ontology_parent_id"],
            "description": r["description"] or "",
            "wiki": r["wiki_link"] or "",
            "ayahs": keys(r["ayahs"]),
            "related": [int(x) for x in keys(r["related_topics"])],
        })
    buf = io.StringIO()
    json.dump(concepts, buf, ensure_ascii=False, indent=0)
    OUT.write_text(buf.getvalue() + "\n", encoding="utf-8")
    print(f"{len(concepts)} concepts, {sum(len(c['ayahs']) for c in concepts)} ayah links -> {OUT.relative_to(ROOT.parent)}")


if __name__ == "__main__":
    main()
