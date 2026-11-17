#!/bin/sh
set -e
cd "$(dirname "$0")"
cargo build --release --target wasm32-unknown-unknown -p quran-timeline -p quran-explore
v=$(date +%s)

page() {
  name=$1 out=$2 title=$3 lang=$4
  mkdir -p "$out"
  wasm-bindgen --target web --no-typescript --out-dir "$out" "target/wasm32-unknown-unknown/release/$name.wasm"
  cat > "$out/index.html" <<HTML
<!doctype html>
<html lang="$lang">
<head><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1"><title>$title</title></head>
<body><div id="main"></div>
<script type="module">import init from "./$name.js?v=$v"; init({ module_or_path: "./${name}_bg.wasm?v=$v" });</script>
</body>
</html>
HTML
}

page quran-timeline dist "Juz timeline" ar
page quran-explore dist/explore "Explore · Juz timeline" en
