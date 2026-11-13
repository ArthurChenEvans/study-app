#!/usr/bin/env python3
"""Writes latest.json, the feed the app checks for updates (updates.feature, R165).

usage: latest-json.py VERSION RPM_PATH DOWNLOAD_URL NOTES_FILE OUT

The app looks for the "linux-x86_64-rpm" entry first, so an installed .rpm only ever
updates to an .rpm. The signature is the .sig file the build writes next to the package.
"""
import datetime, json, pathlib, sys

version, rpm, url, notes, out = sys.argv[1:6]
signature = pathlib.Path(rpm + ".sig").read_text().strip()
feed = {
    "version": version,
    "notes": pathlib.Path(notes).read_text().strip() if pathlib.Path(notes).exists() else "",
    "pub_date": datetime.datetime.now(datetime.timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ"),
    "platforms": {"linux-x86_64-rpm": {"signature": signature, "url": url}},
}
pathlib.Path(out).write_text(json.dumps(feed, indent=2) + "\n")
print(json.dumps(feed, indent=2))
