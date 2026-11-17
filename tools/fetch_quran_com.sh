#!/bin/sh
# Cache quran.com API data used by tools/gen_data.py and tools/similar.py.
set -e
cd "$(dirname "$0")"
mkdir -p .cache/quran-com
API=https://api.quran.com/api/v4
FIELDS=text_imlaei,text_imlaei_simple,juz_number,page_number,rub_el_hizb_number,ruku_number,manzil_number,sajdah_number
curl -sS --retry 3 -m 60 -o .cache/quran-com/chapters.json "$API/chapters?language=en"
seq 1 114 | xargs -P 8 -I{} curl -sS --retry 3 -m 120 -o .cache/quran-com/{}.json \
  "$API/verses/by_chapter/{}?per_page=300&fields=$FIELDS&words=true&word_fields=line_number,page_number"
date -u +%Y-%m-%d > .cache/quran-com/fetched.txt
echo "cached $(ls .cache/quran-com | wc -l) files"
