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


FATHA, DAMMA, KASRA, SUKUN, SHADDA = "َ", "ُ", "ِ", "ْ", "ّ"
TANWEEN = "ًٌٍ"
VOWEL_MARKS = set(FATHA + DAMMA + KASRA + SUKUN + SHADDA + TANWEEN + "ٰٕٓٔ")


def _units(word):
    """[(letter, sorted marks)] without waqf signs, tatweel and spaces; alif wasla read as alif."""
    out = []
    for c in word:
        if 0x06D6 <= ord(c) <= 0x06ED or c in "ـ ۞" or c.isspace():
            continue
        if c in VOWEL_MARKS:
            if out:
                out[-1][1].append(c)
        else:
            out.append(["ا" if c == "ٱ" else c, []])
    return [(letter, "".join(sorted(marks))) for letter, marks in out]


def vkey(word, min_left=3):
    """Vowel-aware word key: drops the vowelled proclitics (wa/fa, bi/li/ka, al-) and the case ending, keeps every other vowel.
    Joins وَقَطَّعْنَ ~ قَطَّعْنَ and النِّسْوَةِ ~ نِسْوَةٌ, never بُكْمٌ ~ بِكُمْ."""
    u = _units(word)
    if len(u) > min_left and u[0][0] in "وف" and u[0][1] == FATHA:
        u = u[1:]
    if len(u) > min_left and ((u[0][0] in "بل" and u[0][1] == KASRA) or (u[0][0] == "ك" and u[0][1] == FATHA)):
        rest = u[1:]
        if rest[0][0] == "ل" and rest[0][1] in ("", SUKUN) and len(rest) > min_left:
            nxt = rest[1]
            if rest[0][1] == SUKUN or SHADDA in nxt[1]:
                rest = [(nxt[0], nxt[1].replace(SHADDA, ""))] + rest[2:]
        u = rest
    if len(u) > min_left + 1 and u[0] == ("ا", "") and u[1][0] == "ل" and u[1][1] in ("", SUKUN):
        nxt = u[2]
        if u[1][1] == SUKUN or SHADDA in nxt[1]:
            u = [(nxt[0], nxt[1].replace(SHADDA, ""))] + u[3:]
    if len(u) >= 2 and u[-1] == ("ا", "") and any(t in u[-2][1] for t in TANWEEN):
        u = u[:-1]
    if u and u[-1][0] == "ا" and any(t in u[-1][1] for t in TANWEEN):
        u = u[:-1]
    if u:
        letter, marks = u[-1]
        u = u[:-1] + [(letter, "".join(c for c in marks if c not in TANWEEN + FATHA + DAMMA + KASRA))]
    return "".join(letter + marks for letter, marks in u)
