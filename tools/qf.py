"""Shared reader for the Content Sync snapshots written by tools/sync_qf.py."""
import functools
import json
import pathlib

SYNC = pathlib.Path(__file__).resolve().parent / ".cache" / "qf-sync"
NBSP = " "


def is_mark(c):
    return 0x064B <= ord(c) <= 0x065F or ord(c) == 0x0670 or 0x06D6 <= ord(c) <= 0x06ED or ord(c) == 0x0640


def plain(word):
    """Matching form of a word: no harakat, waqf signs or tatweel; alif and ya forms unified."""
    stripped = "".join(c for c in word if not is_mark(c) and not c.isspace())
    return stripped.translate(str.maketrans({"أ": "ا", "إ": "ا", "آ": "ا", "ٱ": "ا", "ى": "ي"}))


def with_marks(text):
    """Space-separated words, each sign (waqf, ۞) kept and attached to the word before it, or after it at the start."""
    out, lead = [], ""
    for token in text.split():
        if all(is_mark(c) for c in token):
            if out:
                out[-1] += NBSP + token
            else:
                lead += token + NBSP
        else:
            out.append(lead + token)
            lead = ""
    return out


class Quran:
    def __init__(self):
        core = json.loads((SYNC / "quran_core_1.json").read_text(encoding="utf-8"))["records"]
        mushaf = json.loads((SYNC / "mushafs_1.json").read_text(encoding="utf-8"))["records"]
        self.synced = json.loads((SYNC / "sync.json").read_text())["synced"]
        self.chapters = sorted((r for r in core if r["record_type"] == "chapter"), key=lambda r: r["chapter_number"])
        self.verses = sorted((r for r in core if r["record_type"] == "verse"), key=lambda r: r["id"])
        words = {}
        for r in core:
            if r["record_type"] == "word" and r["char_type_name"] == "word":
                words.setdefault(r["verse_id"], []).append(r)
        place = {}
        self.page_lines = {}
        for r in mushaf:
            if r["record_type"] == "mushaf_word":
                self.page_lines[r["page_number"]] = max(self.page_lines.get(r["page_number"], 0), r["line_number"])
                if r["char_type_name"] == "word":
                    best = place.get(r["verse_id"])
                    if best is None or r["position_in_verse"] < best[2]:
                        place[r["verse_id"]] = (r["page_number"], r["line_number"], r["position_in_verse"])
        for v in self.verses:
            v["key"] = tuple(int(x) for x in v["verse_key"].split(":"))
            text = " ".join(w["text_imlaei"] for w in sorted(words[v["id"]], key=lambda w: w["position"]))
            v["words"] = with_marks(text)
            v["page"], v["line"], _ = place[v["id"]]


@functools.cache
def quran():
    return Quran()
