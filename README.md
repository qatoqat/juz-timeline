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
- `tools/rebuild_data.sh` syncs, regenerates `core/src/data.rs`, `explore/assets/similar.json` and `explore/assets/themes.json`, then runs `tools/check_data.py`, which fails on any overlap, gap, ruku' crossing, stale generated code or bad reference. `--cached` skips the sync.
- The generated files are not committed. `.github/workflows/pages.yml` syncs and regenerates them on every deploy and on a twice-weekly schedule, with the credentials stored as repository secrets, so no copy is older than the terms' 7 days. A failed check stops the deploy; data that differs from the last deploy (`tools/fingerprint.py`, hashes and counts in `tools/data.lock.json`) opens an issue. If the deployed data passes 7 days old (failed syncs), the app shows a dismissible warning at the top until the next successful deploy.
- The Quran text is shown as delivered (waqf signs kept); only matching uses a normalised copy.
- Sajdah: quran.com lists 14; 22:77, which the Madani mushaf marks and the Shafi'i and Hanbali schools count, is added and flagged (white symbol, note in its menu).
- Similar ayah: `tools/similar.py` finds shared phrases of 3+ words in 2–12 ayat plus close variants aligned word by word. Large families are drawn as a two-sided right-to-left word tree (after the DoubleTree design by Culy & Lyding), small ones as aligned cards.
- Ayah themes: the ruku' headings of Muhammad Farooq-i-Azam Malik's translation, as digitised in QUL's ayah-theme dataset ([qul.tarteel.ai](https://qul.tarteel.ai/resources/ayah-theme)), kept in `tools/data/`. `tools/themes.py` aligns them to quran.com's ruku' (splitting any theme that crosses one) and marks ayat without a theme with their opening words.
