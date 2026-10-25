#!/usr/bin/env sh
# Builds the browser app (apps/wordcraft-web) and copies it into the Android project's assets, where
# MainActivity serves it from https://appassets.androidplatform.net/assets/web/index.html.
# Needs trunk and the wasm32-unknown-unknown target.
set -eu
root="$(cd "$(dirname "$0")/../.." && pwd)"
(cd "$root/apps/wordcraft-web" && trunk build --release)
out="$root/packaging/android/app/src/main/assets/web"
rm -rf "$out"
mkdir -p "$out"
cp -R "$root/dist/web/." "$out/"
echo "web build copied to $out"
