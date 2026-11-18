#!/bin/sh
set -e
cd "$(dirname "$0")"
[ -f core/src/data.rs ] || { echo "no Quran data: run tools/rebuild_data.sh first" >&2; exit 1; }
cargo build --release --target wasm32-unknown-unknown -p quran-timeline -p quran-explore
v=$(date +%s)

html() {
  name=$1 out=$2 title=$3 lang=$4 base=${5:-.}
  mkdir -p "$out"
  cat > "$out/index.html" <<HTML
<!doctype html>
<html lang="$lang">
<head><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1"><title>$title</title></head>
<body><div id="main"></div>
<script type="module">import init from "$base/$name.js?v=$v"; init({ module_or_path: "$base/${name}_bg.wasm?v=$v" });</script>
</body>
</html>
HTML
}

page() {
  wasm-bindgen --target web --no-typescript --out-dir "$2" "target/wasm32-unknown-unknown/release/$1.wasm"
  html "$@"
}

page quran-timeline dist "Juz timeline" ar
page quran-explore dist/explore "Explore · Juz timeline" en
html quran-explore dist/explore/similar-ayah "Similar ayah · Explore" ar ..
cp explore/assets/similar.json dist/explore/similar-ayah/
html quran-explore dist/explore/ayah-theme "Ayah themes · Explore" ar ..
cp explore/assets/themes.json explore/assets/relations.json dist/explore/ayah-theme/
