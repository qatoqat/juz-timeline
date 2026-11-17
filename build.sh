#!/bin/sh
set -e
cd "$(dirname "$0")"
cargo build --release --target wasm32-unknown-unknown
wasm-bindgen --target web --no-typescript --out-dir dist target/wasm32-unknown-unknown/release/quran-timeline.wasm
v=$(date +%s)
cat > dist/index.html <<HTML
<!doctype html>
<html lang="ar">
<head><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1"><title>Juz timeline</title></head>
<body><div id="main"></div>
<script type="module">import init from "./quran-timeline.js?v=$v"; init({ module_or_path: "./quran-timeline_bg.wasm?v=$v" });</script>
</body>
</html>
HTML
