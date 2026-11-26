#!/usr/bin/env bash
# Repacks the built .deb as a Void Linux .xbps (R222). Tauri can't build xbps itself, so this
# takes the .deb's files as they are and adds the xbps package info. Runs inside a Void
# container (ghcr.io/void-linux/void-glibc-full) with xbps, binutils and bsdtar installed.
#
#   scripts/make-xbps.sh <the .deb> <version> <output folder>
#
# Dependencies aren't named by hand. The shared libraries the binary links against are looked
# up in Void's repo index (which records the libraries each package provides), and those
# packages become the dependencies. xbps won't fetch a package just to satisfy a library, so
# shlib-requires alone isn't enough. Needs `xbps-install -S` first, so the index is local.
# Updates come from xbps, not the app, so the package carries a marker that turns the in-app
# updater off (src-tauri/src/updates.rs).
set -euo pipefail

deb=$(realpath "$1")
version=$2
out=$(realpath -m "$3")
work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT

mkdir -p "$work/deb" "$work/root" "$out"
(cd "$work/deb" && ar x "$deb")
bsdtar -xf "$work"/deb/data.tar.* -C "$work/root"

mkdir -p "$work/root/usr/share/shudhana"
echo "Installed by xbps. Updates come from xbps, not the app." > "$work/root/usr/share/shudhana/installed-by-xbps"

# Every library any binary in the package needs, minus ones the package ships itself.
shipped=$(find "$work/root" -name '*.so*' -printf '%f\n' | sort -u)
needed=$(find "$work/root/usr" -type f -exec sh -c 'head -c4 "$1" | grep -q "ELF" && objdump -p "$1"' _ {} \; \
  | awk '/NEEDED/ { print $2 }' | sort -u | grep -vxF "$shipped" || true)
if [ -z "$needed" ]; then
  echo "no shared libraries found in the package, the .deb looks wrong" >&2
  exit 1
fi
echo "shlib-requires:" $needed

# Library -> package, from every synced repo index (index.plist inside *-repodata). Keys in
# each package's dict are sorted, so pkgver comes before shlib-provides.
providers=$(for repodata in /var/db/xbps/*/x86_64-repodata; do bsdtar -xOf "$repodata" index.plist; done | awk '
  /<key>pkgver<\/key>/ { want_pkgver = 1; next }
  want_pkgver && /<string>/ { gsub(/.*<string>|<\/string>.*/, ""); pkgver = $0; want_pkgver = 0; next }
  /<key>shlib-provides<\/key>/ { in_shlibs = 1; next }
  in_shlibs && /<\/array>/ { in_shlibs = 0; next }
  in_shlibs && /<string>/ { gsub(/.*<string>|<\/string>.*/, ""); name = pkgver; sub(/-[^-]*$/, "", name); print $0, name }
')
depends=""
missing=""
for lib in $needed; do
  # A here-string, not a pipe: with pipefail, awk finishing early would fail the echo (SIGPIPE).
  package=$(awk -v lib="$lib" '$1 == lib && !found { print $2; found = 1 }' <<< "$providers")
  if [ -z "$package" ]; then missing="$missing $lib"; else depends="$depends $package>=0"; fi
done
if [ -n "$missing" ]; then
  echo "no Void package provides:$missing (run xbps-install -S first?)" >&2
  exit 1
fi
depends=$(echo $depends | tr ' ' '\n' | sort -u | tr '\n' ' ')
echo "depends:" $depends

(cd "$out" && xbps-create \
  --architecture x86_64 \
  --pkgver "shudhana-${version}_1" \
  --desc "All-in-one study app for quizzes, flashcards and free recall" \
  --license "Proprietary" \
  --homepage "https://github.com/ArthurChenEvans/study-app" \
  --maintainer "Arthur Evans" \
  --dependencies "$depends" \
  --shlib-requires "$(echo $needed)" \
  "$work/root")
ls "$out"/shudhana-"${version}"_1.x86_64.xbps
