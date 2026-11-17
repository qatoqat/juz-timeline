# Juz timeline

A right-to-left timeline of the Quran for hifz: all 30 juz with their surahs, sajdah positions, the فمي بشوق manzil marks, and a zoomed detail view with rub' al-hizb lengths in pages. Mark progress and keep reading bookmarks; both are saved in your browser.

Live: https://qatoqat.com/juz-timeline/ · Explore: https://qatoqat.com/juz-timeline/explore/

## Layout

- `core` — shared mushaf data (surah, juz, page, rub', sajdah, manzil) and positions.
- `timeline` — the timeline page, kept lightweight.
- `explore` — divisions and relation datasets, the base for ring patterns, themes and links between ayat.

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

- Surah, juz, page, rub' and sajdah positions: [Tanzil Quran metadata](https://tanzil.net/docs/quran_metadata) (CC BY 3.0).
- Line positions for rub' lengths (Madani 15-line layout): [quran.com API](https://api-docs.quran.com).
