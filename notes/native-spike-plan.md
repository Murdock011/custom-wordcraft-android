# WordCraft Android: native migration plan and spike (2026-10-24)

> Note: the WebView build in packaging/android/ was removed after 0.2.0; paths below are historical.

Status: plan only. No source, build or emulator changes were made to write this. Decision (from the coordinator): option A, native egui. B and C are fallbacks only.

## 0. Corrections to the current picture
- `wordcraft-ui-egui::Services` is a struct of optional hooks (`pick_open`, `pick_save`, `download`, `clipboard_picture`, `print`, ...), not a trait. A native host fills the same struct. Hooks left `None` fall back to in-app behaviour.
- The desktop app `apps/wordcraft` cannot be reused as the Android host. It pulls arboard, rfd, winit X11, `control_server` (TCP), `default_apps`, and macOS/Windows code. Only `clipboard.rs` already excludes `target_os = "android"`.
- `crates/fonts` system-font discovery is `cfg(not(wasm32))` and `unix`. On Android that scans Linux paths, not `/system/fonts`. It needs an Android branch.
- Wasm gates that matter: `crates/engine/src/io.rs`, `crates/ui-egui/src/file_dialogs.rs`, `theme.rs`, `crates/render`, `crates/layout`. Portable core with no OS gate: geom, doc, layout, docx, formats, render, pdf, proof, engine, docbin, metafile, zotero, control-key. Fonts and ui-egui have native-only parts.
- Desktop host with eframe/winit exists: `apps/wordcraft` (eframe 0.36, features `wgpu`, `accesskit`, `wayland`, `x11`). Web host: `apps/wordcraft-web`, which runs on Android today inside a WebView.

## 1. Goal and "done"
Goal: WordCraft runs as a native Android app (no WebView, no wasm) with the same document engine and egui UI as desktop and the phone layout.

Done means all of these, verified on the `wc35` emulator and one physical Samsung phone:
1. Open, edit, Save, Save As and Open a .docx through SAF, with no WebView in the process.
2. Gboard and Samsung keyboard composition, suggestions and backspace behave at least as well as the WebView build (baseline: `notes/phone-ui-fixes.md`).
3. Back, rotation, dark mode and process death do not lose work.
4. Release APK per ABI, signed, with size recorded.

## 2. Options compared

| | A. egui native (android-activity / winit) | B. Compose UI + Rust engine (JNI or UniFFI) | C. Stay on WebView, harden |
|---|---|---|---|
| Effort | Medium-high: host crate, input, IME, SAF bridge, packaging. UI reused. | High: rewrite the UI (ribbon, phone layout, dialogs, canvas hit testing) in Kotlin. | Low. Already works. |
| Risk | Medium-high. eframe/winit Android IME is the unknown (see §2 notes). | Medium. Canvas and text editing in Compose from scratch. | Low build risk; high ceiling risk (WebView IME, no native selection). |
| Reused | All of `ui-egui`, `engine`, `layout`, `render`, `docx`, `formats`, `fonts`, `doc`, `geom`, `pdf`, `proof`, `docbin`. | `engine` and below, via UniFFI or JNI. | Everything, as is. |
| Rewritten | New `apps/wordcraft-android` host, a `fonts` discovery branch, an Android clipboard branch, SAF bridge, packaging. | All UI. | None. |
| Verdict | Recommended (decided). | Fallback if spike step 0.3 fails for the input layer. | Fallback, and the shipping build until A reaches parity. |

Sources read (current state, not verified in depth): android-activity PR #214 (IME editor-action groundwork, Jan 2026): https://github.com/rust-mobile/android-activity/pull/214 . android-activity activity choice and examples: https://docs.rs/crate/android-activity/^0.5 . Google's NativeActivity to GameActivity guide: https://developer.android.com/games/agdk/game-activity/migrate-native-activity . UniFFI-based Compose framework (third party; not evidence of production use): https://docs.rs/crate/jffi/0.4.18 .

Uncertainty, stated plainly: the search did not find a 2026 eframe or egui Android IME report, and found nothing on cargo-ndk with egui. The claim that NativeActivity gives poor soft-keyboard composition, and that GameActivity fixes it, is background knowledge. It must be checked in step 0.3. PR #214 also notes keyboard-visibility tracking is heuristic with no plan to support it, so the app must not depend on a "keyboard shown" event.

## 3. Recommended path: phased migration

### Phase 0: time-boxed spike (3 to 5 working days; stop at the first hard fail)
Each step has a pass/fail criterion. A fail on 0.3 or 0.4 triggers the fallback (option B for the input layer, or stay on C).

0.1 Toolchain. Install the Android Rust targets and cargo-ndk; use the current NDK LTS. Build a hello `cdylib` for `aarch64-linux-android` and `x86_64-linux-android` (the emulator).
  Pass: `cargo ndk -t arm64-v8a -t x86_64 build` produces `.so` files. Fail: pinned workspace crates do not build for Android (record the list; no fork without a note).

0.2 Host crate `apps/wordcraft-android` (new, `crate-type = ["cdylib"]`, Android-only dependency on eframe with its android feature). Depends on `wordcraft-ui-egui` and `wordcraft-engine`. Add to workspace `members` only if `cargo xtask ci` still passes.
  Pass: APK shows a blank egui window with a label, no crash for 60 s on `wc35`.

0.3 Activity and IME choice. Spike both NativeActivity (`android:hasCode="false"`, no Java) and GameActivity (Gradle dependency, `game-activity` feature). Type into a native text field with Gboard: letters, suggestion tap, space, backspace, Enter, and hardware keys.
  Pass: composition shown inline and commits correctly on the chosen activity. Record the choice here.
  Fail on both: stop Phase 0 and open option B for the input layer.

0.4 Render a real page. Load the sample document through `wordcraft-engine`, lay it out, and draw one page through the existing canvas path.
  Pass: page text and a table render within 1 s of open. Pixels match the desktop `ui_shot` output within a tolerance stated before the test.

0.5 Touch. Scroll, tap to place the caret, long-press word selection, drag selection with handles, double-tap.
  Pass: caret lands within one character of the tap point; selection and handles match the WebView build on the same document. Record the measured offset.

0.6 SAF Save and Open through JNI. Use `ndk-context` and the `jni` crate to start `ACTION_CREATE_DOCUMENT` and `ACTION_OPEN_DOCUMENT`, then read and write through `ContentResolver` streams. Route results to `Services.pick_save` / `pick_open` / `download`.
  Pass: save a new .docx on the emulator, pull it with `adb`, it is a valid zip with the typed text. Open it back.

0.7 Back. Route the back gesture into the existing Escape path (`phone::back_step`, `WordApp::can_go_back`).
  Pass: the 4 cases in `notes/phone-ui-fixes.md` round 6 pass on the native host.

0.8 Clipboard. Plain text copy and paste through JNI `ClipboardManager`. HTML and pictures are out of spike scope.
  Pass: copy in WordCraft, paste into a system text field, and back.

0.9 Fonts. Add Android font discovery over `/system/fonts` and `/product/fonts`. Bundled fonts already load through `include_bytes!`.
  Pass: "Aptos" falls back to a metric substitute, no crash, sample renders in Inter.

0.10 APK size. Release `.so` per ABI, stripped. Compare with the WebView APK (about 22 MB debug build; record the release figure).
  Pass: report only. Go/no-go thresholds are in §5.

Spike exit: this file gets a short result with each step marked PASS, FAIL or NOT RUN, plus the activity choice.

### Phase 1: parity with the WebView app
1. Keyboard: Gboard and Samsung composition, Enter, IME actions. Files: `apps/wordcraft-android/src/input.rs`. Check: type "Hello world" on both keyboards, screenshot.
2. Touch parity: handles, long-press menu. Files: `crates/ui-egui/src/canvas.rs`, only where the touch path needs it. Check: step 0.5 list.
3. SAF: Save, Save As, Open, reopen recent with persistable grants. Files: `apps/wordcraft-android/src/saf.rs`; `crates/ui-egui/src/lib.rs` only if a hook is missing. Check: the 6 cases in `notes/save-fix.md`.
4. Back: all cases in `notes/phone-ui-fixes.md` round 6. Files: `apps/wordcraft-android/src/main.rs`.
5. Clipboard: text, HTML and picture. Files: `crates/ui-egui/src/system_clipboard.rs` (the `put` path), Android side in `apps/wordcraft-android/src/clipboard.rs`. Check: paste from Chrome.
6. Fonts: finish 0.9 so the Font dialog lists system families. Check: Roboto and bundled fonts appear.
7. Rotation: keep document, caret and scroll across `configChanged`. Check: rotate with a document open.
8. Dark mode: follow the system mode in `theme.rs`. Check: toggle emulator dark theme, screenshot both.
9. Print: `print` hook through `PrintManager`, or PDF share (phase 2). Check: a PDF is produced and opens.
10. Phone layout at 1080x2400 and a tablet size against the checklist in `notes/phone-ui-fixes.md`.

Exit: the parity items above pass with the same results as the WebView build.

### Phase 2: native-only features
1. Spellcheck: optional system spell service (`SpellCheckerSession`) beside the built-in proofing, plus a personal dictionary file. Check: a misspelling gets a system suggestion.
2. Share and open-with: `ACTION_SEND` for the current document; `ACTION_VIEW` intake of .docx. Files: `apps/wordcraft-android/src/intents.rs`. Check: share from Files; open a .docx sent from Chrome.
3. Accessibility: TalkBack labels for ribbon and pill buttons through accesskit in eframe. Check: TalkBack reads the Bold button.
4. Autosave and recovery on process death: save on `onStop`, offer restore at launch, reusing `crates/engine/src/recovery.rs`. Check: `adb shell am kill` mid-edit, relaunch, text restored.
5. Native selection feel: handle size and haptics at Android defaults. Check: side by side with Google Docs on the same phone.

### Phase 3: packaging and release
1. ABI splits: ship `arm64-v8a`; `x86_64` for the emulator only. Files: `packaging/android/app/build.gradle.kts` (`splits.abi`). Check: `unzip -l` lists the expected `.so` set.
2. Replace the WebView launcher (`MainActivity.kt`, `build-web.sh`, WebView assets) with the native activity chosen in 0.3. Files: `packaging/android/app/src/main/AndroidManifest.xml`, `packaging/android/build-native.sh`. Keep the WebView build on a separate tag until parity is signed off.
3. Signing: release keystore from environment variables, never committed. Check: `apksigner verify`.
4. Size report: APK size per ABI recorded in this file.
5. Attribution: any new font or icon gets a row in `ATTRIBUTION.md` (AGENTS.md rule).

## 4. WebView-only limitations and the native answer

| WebView limitation | Native answer |
|---|---|
| Hidden text input; composition drawn off the caret (`text_input.rs` workaround) | Native IME through the chosen activity (0.3). The workaround is deleted. |
| Blob download cannot save (`AndroidSave` JS shim) | SAF `ACTION_CREATE_DOCUMENT` through JNI (0.6). |
| Back goes to the page through a `wcCanGoBack` poll | Native back callback reads `WordApp::can_go_back` (0.7). |
| Keyboard inset handled by a padding container | Insets from the native window. |
| Only bundled fonts for lookup | Native `/system/fonts` scan (0.9). |
| Clipboard only through browser APIs | JNI `ClipboardManager` (0.8). |
| Two-stage tap read as double-click (`max_double_click_delay = 0`) | Native touch events; keep the setting only if 0.5 shows it is still needed. |
| WASM download size and start time; 4096 texture cap on software GPU | Native wgpu/GL. |
| No TalkBack for canvas content | accesskit through eframe (phase 2, item 3). |

## 5. Risks and go/no-go

| Risk | Signal | Go/no-go |
|---|---|---|
| IME composition in eframe/winit on Android is missing or wrong | 0.3 fails on both activities | No-go for A's input layer. Fall back to B for input, or stay on C. |
| Pinned workspace crates fail to build for Android | 0.1 | Go if fixes are small and upstream; no-go if a fork is needed. |
| Font discovery and shaping differ from WebView | 0.9 | Go on bundled fonts if system fallback works. |
| `.so` / APK size far above WebView build | 0.10 | Go up to 1.5x the current release size; above that, strip features or keep C. |
| Touch selection worse than WebView | 0.5 | Go if within one character; otherwise tune before Phase 1. |
| Keyboard visibility is heuristic (PR #214) | Layout jumps when the keyboard opens | Do not key UI on a "keyboard shown" event; use insets. |

Overall go: 0.1 to 0.4 and 0.6 pass. Other failures need a written reason and still allow Phase 1 work on the rest.

## 6. Work breakdown for small agents
Each task touches one file set and has one check on `wc35`. Use a separate `CARGO_TARGET_DIR` per agent, not the shared build directory. Run `cargo xtask layers` after any `crates/*` edit, and `cargo xtask wasm` if a shared crate changes.

- T1 (cargo-ndk setup): add `packaging/android/build-native.sh` running `cargo ndk`. Check: `.so` exists for both ABIs.
- T2 (host skeleton): `apps/wordcraft-android/Cargo.toml` and `src/lib.rs` with an eframe `android_main`. Check: blank window on `wc35`.
- T3 (activity probe): two throwaway manifests (NativeActivity, GameActivity). Check: both launch; record which gets IME (0.3).
- T4 (page render): call the engine sample load and the canvas draw. Check: screenshot against desktop `ui_shot`.
- T5 (touch probe): log tap, long-press and drag coordinates. Check: caret offset within one character.
- T6 (SAF bridge): `src/saf.rs` with JNI create and open. Check: 0.6.
- T7 (back and clipboard): back callback in `src/main.rs`; JNI text clipboard in `src/clipboard.rs`. Check: 0.7 and 0.8.
- T8 (fonts): Android branch in `crates/fonts` discovery (`#[cfg(target_os = "android")]`). Check: 0.9 and `cargo test -p wordcraft-fonts`.
- T9 (size): release build per ABI, stripped. Check: `ls -l` and `unzip -l`; numbers into this file.
- T10 (spike result): fill PASS/FAIL per step here. Check: no "NOT RUN" without a reason.

## 7. Sources
- https://github.com/rust-mobile/android-activity/pull/214
- https://docs.rs/crate/android-activity/^0.5
- https://developer.android.com/games/agdk/game-activity/migrate-native-activity
- https://docs.rs/crate/jffi/0.4.18
- Repo: `crates/ui-egui/src/lib.rs` (`Services`), `apps/wordcraft-web/src/web.rs`, `apps/wordcraft-web/src/web/text_input.rs`, `packaging/android/app/src/main/java/ai/storyteller/wordcraft/MainActivity.kt`, `notes/save-fix.md`, `notes/phone-ui-fixes.md`.
