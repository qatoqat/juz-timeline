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

- Main source: the [quran.com API](https://api-docs.quran.com). `tools/fetch_quran_com.sh` caches every chapter and verse (text, page, juz, rub', ruku', manzil, sajdah, word line numbers); `tools/gen_data.py` builds `core/src/data.rs` from it.
- Sajdah: quran.com lists 14; 22:77, which the Madani mushaf marks and the Shafi'i and Hanbali schools count, is added and flagged (white symbol, note in the right-click menu).
- Similar ayah: `tools/similar.py` builds `explore/assets/similar.json` from quran.com's text (text_imlaei for display, text_imlaei_simple for matching): shared phrases of 3+ words found in 2–12 ayat, plus close variants aligned word by word. The page draws large families as a two-sided right-to-left word tree (after the DoubleTree design by Culy & Lyding) and small ones as aligned cards.
- Ayah themes: QUL ayah-theme dataset ([qul.tarteel.ai](https://qul.tarteel.ai/resources/ayah-theme)); QUL lists no licence or author for it. Built by `tools/themes.py` from the sqlite download.
