#!/bin/sh
# Refresh every generated data file from Quran Foundation Content Sync and check it.
#   tools/rebuild_data.sh           sync fresh snapshots, regenerate, check
#   tools/rebuild_data.sh --cached  regenerate from the existing snapshots only
set -e
cd "$(dirname "$0")"
[ "$1" = "--cached" ] || python sync_qf.py
python gen_data.py > ../core/src/data.rs.tmp && mv ../core/src/data.rs.tmp ../core/src/data.rs
python similar.py
python themes.py
python topics.py
python relations.py
python check_data.py
