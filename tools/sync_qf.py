"""Fetch the Quran Foundation Content Sync snapshots used by the generators.

Credentials come from QF_CLIENT_ID / QF_CLIENT_SECRET, or from
~/.config/juz-timeline/quran_foundation_credentials.txt (CLIENT_ID=..., CLIENT_SECRET=...).
Each run bootstraps a fresh snapshot of every resource, which applies all published changes;
the Quran Foundation developer terms require this at least every 7 days.
Run: python tools/sync_qf.py
"""
import datetime
import json
import os
import pathlib
import subprocess

ROOT = pathlib.Path(__file__).resolve().parent
OUT = ROOT / ".cache" / "qf-sync"
CREDENTIALS = pathlib.Path.home() / ".config" / "juz-timeline" / "quran_foundation_credentials.txt"
TOKEN_URL = "https://oauth2.quran.foundation/oauth2/token"
API = "https://apis.quran.foundation/content/api/v4"
ORIGIN = "https://apis.quran.foundation/content"
SCOPES = "content.quran.read content.sync.read"
# quran_core: verses, words (Imla'i text), juz/hizb/rub'/ruku'/manzil/sajdah; mushafs:1: QCF V2,
# the King Fahd Complex 1441 AH print, for page and line numbers.
RESOURCES = ["quran_core:1", "mushafs:1"]


def credentials():
    if os.environ.get("QF_CLIENT_ID"):
        return os.environ["QF_CLIENT_ID"], os.environ["QF_CLIENT_SECRET"]
    pairs = dict(line.split("=", 1) for line in CREDENTIALS.read_text().split() if "=" in line)
    return pairs["CLIENT_ID"].strip('"'), pairs["CLIENT_SECRET"].strip('"')


def curl(args, config=""):
    """Runs curl; secrets go through a config on stdin so they never appear in the process list."""
    base = ["curl", "-sS", "--fail-with-body", "--retry", "3", "-m", "600", "-K", "-"]
    return subprocess.run(base + args, input=config.encode(), capture_output=True, check=True).stdout


def main():
    client, secret = credentials()
    login = f'user = "{client}:{secret}"\n'
    token = json.loads(curl(["-d", f"grant_type=client_credentials&scope={SCOPES}", TOKEN_URL], login))["access_token"]
    auth = f'header = "x-auth-token: {token}"\nheader = "x-client-id: {client}"\nuser-agent = "juz-timeline-sync"\n'
    OUT.mkdir(parents=True, exist_ok=True)
    meta = {"synced": datetime.date.today().isoformat(), "resources": {}}
    for resource in RESOURCES:
        sync = json.loads(curl([f"{API}/resources/sync?bootstrap=true&resources={resource}"], auth))["sync"]
        create = next(m for m in sync["mutations"] if m["type"] == "RESOURCE_CREATE")
        group, rid = resource.split(":")
        path = OUT / f"{group}_{rid}.json"
        path.write_bytes(curl([ORIGIN + create["snapshot_url"]], auth))
        meta["resources"][resource] = {"sequence": sync["sync_until_sequence"], "next_sync_token": sync["next_sync_token"]}
        print(f"{resource}: {path.stat().st_size // 1024} KB, sequence {sync['sync_until_sequence']}")
    (OUT / "sync.json").write_text(json.dumps(meta, indent=2))


if __name__ == "__main__":
    main()
