# Juz timeline

A right-to-left timeline of the Quran for hifz: all 30 juz with their surahs, sajdah positions, the فمي بشوق manzil marks, and a zoomed detail view with rub' al-hizb lengths in pages. Mark progress and keep reading bookmarks; both are saved in your browser.

Live: https://qatoqat.com/juz-timeline/ · Explore: https://qatoqat.com/juz-timeline/explore/

## Layout

- `core` — shared mushaf data (surah, juz, page, rub', sajdah, manzil) and positions.
- `ui` — the timeline component: rows, detail panel, search, bookmarks, done marks.
- `timeline` — the timeline page, kept lightweight.
- `explore` — relation datasets (similar ayah, mutashabihat, themes, topics); each opens a page built on the timeline, with ruku' marks (ع) in the detail view.

Each page builds its own WebAssembly, so the explore page never adds weight to the timeline.

Built with [Dioxus](https://dioxuslabs.com) and compiled to WebAssembly.

## Build

```sh
rustup target add wasm32-unknown-unknown
cargo install wasm-bindgen-cli --version 0.2.130
tools/rebuild_data.sh # sync Quran data (needs credentials, see Data)
./build.sh            # dist/ (timeline) and dist/explore/
python -m http.server -d dist
```

## Data

All Quran data comes from **Quran Foundation Content Sync** (the quran.com content API), under the [developer terms](https://api-docs.quran.foundation/legal/developer-terms/):

- `tools/sync_qf.py` downloads fresh snapshots of `quran_core:1` (Imla'i text, juz, hizb, rub', ruku', manzil, sajdah) and `mushafs:1` (QCF V2, the King Fahd Complex **1441 AH** print, for page and line numbers). It needs a confidential client with `content.quran.read` and `content.sync.read`: set `QF_CLIENT_ID` / `QF_CLIENT_SECRET`, or put `CLIENT_ID=` / `CLIENT_SECRET=` lines in `~/.config/juz-timeline/quran_foundation_credentials.txt`.
- `tools/rebuild_data.sh` syncs, regenerates `core/src/data.rs` and the Explore assets (`similar.json`, `themes.json`, `topics.json`, `topic-ayat.json`, `relations.json`), then runs `tools/check_data.py`, which fails on any overlap, gap, ruku' crossing, stale generated code or bad reference. `--cached` skips the sync.
- The generated files are not committed. `.github/workflows/pages.yml` syncs and regenerates them on every deploy and on a twice-weekly schedule, with the credentials stored as repository secrets, so no copy is older than the terms' 7 days. A failed check stops the deploy; data that differs from the last deploy (`tools/fingerprint.py`, hashes and counts in `tools/data.lock.json`) opens an issue. If the deployed data passes 7 days old (failed syncs), the app shows a dismissible warning at the top until the next successful deploy.
- The Quran text is shown as delivered (waqf signs kept); only matching uses a normalised copy.
- Sajdah: quran.com lists 14; 22:77, which the Madani mushaf marks and the Shafi'i and Hanbali schools count, is added and flagged (white symbol, note in its menu).
- Similar ayah: `tools/similar.py` finds shared phrases of 3+ words in 2–12 ayat plus close variants aligned word by word. Large families are drawn as a two-sided right-to-left word tree (after the DoubleTree design by Culy & Lyding), small ones as aligned cards.
- Ayah themes: the ruku' headings of Muhammad Farooq-i-Azam Malik's translation, as digitised in QUL's ayah-theme dataset ([qul.tarteel.ai](https://qul.tarteel.ai/resources/ayah-theme)), kept in `tools/data/`. `tools/themes.py` aligns them to quran.com's ruku' (splitting any theme that crosses one) and marks ayat without a theme with their opening words.
- Concepts: the 284 concepts of the [Quranic Arabic Corpus](https://corpus.quran.com/ontology.jsp) ontology (GNU GPL), extracted from QUL's ayah-topics SQLite by `tools/ontology.py` into `tools/data/ontology.json` (QUL's thematic and general topics, from other sources, are left out). `tools/topics.py` maps them onto Content Sync ayat, keeps the corpus links between concepts and adds concepts that share ayat, and writes the ayah text the page loads on demand.
- Theme relations (Ayah themes page): `tools/relations.py` links themes that are told or worded alike, each link with its Arabic evidence.
  - Cross-surah links fuse four signals per pair of themes: shared phrase families, shared corpus concepts, heading words and the passages' own words. A link needs two signals over their floors; links are typed story (a shared person or people), wording or subject, with weaker ones kept apart. Three ontology tags that name someone absent from the passage are ignored here only.
  - Look-alike passages (`tools/twins.py`): shared phrases in the same order on both sides, word-aligned, with each word flagged same, other vowels, و/ف only, or unmatched. The page draws them as a ladder: shared wording once, forks side by side, then what follows.
  - Refrains: phrases a surah repeats at the start or end of 3+ of its ayat. In-surah echoes (`tools/echoes.py`): shared phrases or words found in at most 10 ayat of the Quran, drawn as arcs under the theme band and labelled as computed echoes of wording, not as claims about structure.
  - `tools/check_data.py` checks every evidence item against the text and gates on `tools/data/relations_gold.json`, a hand-built evaluation set (24 retold stories, must-link and must-not-link pairs) that is never shown.
