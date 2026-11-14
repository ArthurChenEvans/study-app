#!/usr/bin/env python3
"""Writes latest.json, the feed the app checks for updates (updates.feature, R165).

usage: latest-json.py VERSION NOTES_FILE OUT PLATFORM=PACKAGE=URL [PLATFORM=PACKAGE=URL ...]

PLATFORM is the updater's key: "linux-x86_64-rpm" for the .rpm and "windows-x86_64-nsis"
for the Windows installer. The app looks for its own installer's key first, so an installed
.rpm only ever updates to an .rpm. Each package's signature is the .sig file next to it.
"""
import datetime, json, pathlib, sys

if len(sys.argv) < 5:
    sys.exit(__doc__)
version, notes, out = sys.argv[1:4]
platforms = {}
for entry in sys.argv[4:]:
    key, package, url = entry.split("=", 2)
    signature = pathlib.Path(package + ".sig")
    if not signature.exists():
        sys.exit(f"{package} has no signature ({signature} is missing)")
    platforms[key] = {"signature": signature.read_text().strip(), "url": url}

feed = {
    "version": version,
    "notes": pathlib.Path(notes).read_text().strip() if pathlib.Path(notes).exists() else "",
    "pub_date": datetime.datetime.now(datetime.timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ"),
    "platforms": platforms,
}
pathlib.Path(out).write_text(json.dumps(feed, indent=2) + "\n")
print(json.dumps(feed, indent=2))
