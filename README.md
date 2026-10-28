# Custom WordCraft for Android

> [!WARNING]
> **This project is made entirely by AI.** All of the code, notes and documentation in this
> repository's Android work were written by AI coding agents (Anthropic's Claude models), directed
> by a person who did not write or review the code line by line. It has not had human code
> review or security review, and only light testing on an emulator and one Samsung phone. Expect bugs. Do not
> trust it with documents you can't afford to lose.

> [!IMPORTANT]
> **This is an unofficial, personal fork. It is not made, endorsed or supported by the ArtCraft
> team or by storytold.** It is not the official WordCraft, and it is not an ArtCraft project.
> Please don't report problems with this fork to the original project.

## What this is

**In short: a UI redesign for Android on top of the WordCraft engine.**
[WordCraft](https://github.com/storytold/wordcraft) is an open-source Rust word processor by the
ArtCraft team. Everything that makes it a word processor (the document model, editing commands,
layout and pagination, .docx and other file formats, proofing, rendering and PDF) is the original
WordCraft engine, unchanged in purpose. This fork adds a phone interface around it and the Android
plumbing to host it as a **native app** (no WebView): a GameActivity with Kotlin bridges for the
keyboard, the system file picker, sharing, the clipboard, text-to-speech, Back, dark mode and
recovery after the app is killed.

The few changes it makes below the interface are listed, with the reason for each, in
[docs/ENGINE-CHANGES.md](docs/ENGINE-CHANGES.md). The fork keeps the original project's history,
so newer WordCraft versions can be merged in; see [docs/UPDATING.md](docs/UPDATING.md). It is
currently based on WordCraft engine **0.11.0**.

## Download

Builds are on the [Releases page](https://github.com/Murdock011/custom-wordcraft-android/releases).
The latest is **0.2.0** (WordCraft engine 0.11.0): `custom-wordcraft-android-arm64.apk`, for 64-bit ARM phones
(arm64-v8a) only. It is signed with a debug key, so Android asks you to allow installs from your
browser or file manager. It installs as `ai.storyteller.wordcraft.native`, next to any other
WordCraft build.

## What works

- Typing with Gboard and Samsung Keyboard (Backspace, Enter, the word being typed stays in the document), long-press
  and double-tap selection with handles, an icon menu for Cut, Copy and Paste.
- Opening, saving and saving in place through the system file picker, Recent files, Open with and
  Share for .docx, .doc, .rtf, .odt, .txt and .md.
- Recovery of unsaved work after Android closes the app.
- A phone layout: ribbon tabs as sheets, full-width icon bars, dialogs as bottom sheets with large
  buttons, Options as a list of pages with toggle switches.
- Read Aloud with Android's text-to-speech, dark mode, rotation.

Status: tested on an Android emulator and briefly by hand on one Samsung phone. Work in progress and
known gaps are logged in [notes/native-spike-results.md](notes/native-spike-results.md).

## What this fork adds

- `apps/wordcraft-android/`: the native Android host (Rust).
- `packaging/android-native/`: the Gradle project, `build-native.sh` and the app icon
  (`icon-source/`).
- `crates/ui-egui/src/phone.rs` and changes across `crates/ui-egui`: the phone layout. They only
  apply in the phone layout.
- `packaging/android/`: an earlier WebView-based Android wrapper.
- `notes/`: the plan and step-by-step results of the Android work
  (`notes/native-spike-plan.md`, `notes/native-spike-results.md`) and design mockups.

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
Asset credits are in [ATTRIBUTION.md](ATTRIBUTION.md). The app icon includes the Android robot,
reproduced or modified from work created and shared by Google and used according to terms
described in the Creative Commons 3.0 Attribution License. The original project's README is kept
in [docs/UPSTREAM-README.md](docs/UPSTREAM-README.md).

Names, logos and branding of WordCraft and ArtCraft belong to their owners; their use here only
identifies where the code came from.
