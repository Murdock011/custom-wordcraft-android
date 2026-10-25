# Custom WordCraft for Android

> [!WARNING]
> **This project is made entirely by AI.** All of the code, notes and documentation in this
> repository's Android work were written by AI coding agents (Anthropic's Claude models), directed
> by a person who did not write or review the code line by line. It has not had human code
> review, security review or real-world testing beyond an Android emulator. Expect bugs. Do not
> trust it with documents you can't afford to lose.

> [!IMPORTANT]
> **This is an unofficial, personal fork. It is not made, endorsed or supported by the ArtCraft
> team or by storytold.** It is not the official WordCraft, and it is not an ArtCraft project.
> Please don't report problems with this fork to the original project.

## What this is

**In short: a UI redesign for Android on top of the WordCraft engine.** Everything that makes
WordCraft a word processor (the document model, editing commands, layout and pagination,
.docx and other file formats, proofing, rendering and PDF) is the original WordCraft engine,
unchanged in purpose. What this fork adds is a phone interface around it and the Android plumbing
to host it.

An experiment in running [WordCraft](https://github.com/storytold/wordcraft), an open-source,
Rust word processor by the ArtCraft team, as a **native Android app**: the same document engine
and egui interface as the desktop build, hosted in a GameActivity with Kotlin bridges for the
keyboard, the system file picker, the clipboard, Back, dark mode and recovery after the app is
killed. No WebView.

What was added here, on top of the original project:

- `apps/wordcraft-android/`: the native Android host (Rust).
- `packaging/android-native/`: the Gradle project and `build-native.sh`.
- `packaging/android/`: an earlier WebView-based Android wrapper.
- `crates/ui-egui/src/phone.rs` and changes across `crates/ui-egui`: a phone layout (bottom
  formatting bar, ribbon as a sheet, long-press selection with handles, a selection menu).
- `notes/`: the plan and step-by-step results of the native Android work
  (`notes/native-spike-plan.md`, `notes/native-spike-results.md`).

Status: an emulator-tested spike. It has not been tested on a physical phone.

## Building

Needs the Android SDK and NDK 27, `cargo-ndk`, and the Rust Android targets.

```sh
rustup target add aarch64-linux-android x86_64-linux-android
cargo install cargo-ndk
sh packaging/android-native/build-native.sh arm64-v8a      # phone
sh packaging/android-native/build-native.sh x86_64         # emulator
```

The debug APK is written to `packaging/android-native/app/build/outputs/apk/debug/`.

## Credit and licence

All of WordCraft itself, the document engine, layout, file formats and desktop interface, is the
work of the ArtCraft team and the WordCraft contributors. This fork keeps their licence:
MIT OR Apache-2.0, at your option ([LICENSE-MIT](LICENSE-MIT), [LICENSE-APACHE](LICENSE-APACHE)).
Asset credits are in [ATTRIBUTION.md](ATTRIBUTION.md). The original project's README is kept in
[docs/UPSTREAM-README.md](docs/UPSTREAM-README.md).

Names, logos and branding of WordCraft and ArtCraft belong to their owners; their use here only
identifies where the code came from.
