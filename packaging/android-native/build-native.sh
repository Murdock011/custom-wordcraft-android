#!/bin/sh
# Builds the native spike APK: Rust .so per ABI with cargo-ndk, then Gradle.
# Usage: sh packaging/android-native/build-native.sh [abis...]   (default: x86_64)
set -eu
ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
ABIS="${*:-x86_64}"
ARGS=""
for abi in $ABIS; do ARGS="$ARGS -t $abi"; done
export ANDROID_HOME="${ANDROID_HOME:-$HOME/Android/Sdk}"
export ANDROID_NDK_HOME="${ANDROID_NDK_HOME:-$ANDROID_HOME/ndk/27.2.12479018}"
cd "$ROOT"
# Spelling lists, hyphenation and interface translations are embedded from craft-i18n at the pinned commit.
I18N="${CRAFT_I18N_DIR:-$HOME/.cache/craft-i18n}"
REV="$(cat "$ROOT/craft-i18n.rev")"
[ -d "$I18N" ] || git clone -q https://github.com/storytold/craft-i18n "$I18N"
git -C "$I18N" cat-file -e "$REV^{commit}" 2>/dev/null || git -C "$I18N" fetch -q
git -C "$I18N" checkout -q "$REV"
export CRAFT_I18N_DIR="$I18N" CRAFT_I18N_REQUIRED=1
# shellcheck disable=SC2086
cargo ndk $ARGS -o "$ROOT/packaging/android-native/app/src/main/jniLibs" build --release -p wordcraft-android
cd "$ROOT/packaging/android-native"
./gradlew assembleDebug
