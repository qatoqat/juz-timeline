#!/bin/sh
# Refresh every generated data file from the quran.com API and check it.
#   tools/rebuild_data.sh           fetch fresh data, regenerate, check
#   tools/rebuild_data.sh --cached  regenerate from the existing cache only
set -e
cd "$(dirname "$0")"
[ "$1" = "--cached" ] || ./fetch_quran_com.sh
python -I gen_data.py > ../core/src/data.rs
python -I similar.py
python -I themes.py
python -I check_data.py
