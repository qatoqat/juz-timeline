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
./build.sh            # dist/ (timeline) and dist/explore/
python -m http.server -d dist
```

## Data

- Surah, juz, page, rub', ruku' and sajdah positions: [Tanzil Quran metadata](https://tanzil.net/docs/quran_metadata) (CC BY 3.0).
- Line positions for rub' lengths (Madani 15-line layout): [quran.com API](https://api-docs.quran.com).
- Quran text for similar ayah matching: [Tanzil](https://tanzil.net) (simple and simple-clean). Families are built by `tools/similar.py` into `explore/assets/similar.json`: shared phrases of 3+ words found in 2–12 ayat, plus close variants aligned word by word.
