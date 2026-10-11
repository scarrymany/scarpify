#!/usr/bin/env bash
# A successful archive build is insufficient if WebKit helpers were not found.
set -euo pipefail
archive=$(realpath "${1:?Pass the built AppImage}")
scratch=$(mktemp -d)
trap 'rm -rf "$scratch"' EXIT
cd "$scratch"
"$archive" --appimage-extract >/dev/null
for helper in WebKitNetworkProcess WebKitWebProcess; do
  test -n "$(find squashfs-root -type f -name "$helper" -print -quit)" || {
    echo "AppImage is missing $helper; build with the matching WebKit runtime installed." >&2
    exit 1
  }
done
test -n "$(find squashfs-root -type f -name libwebkit2gtkinjectedbundle.so -print -quit)"
