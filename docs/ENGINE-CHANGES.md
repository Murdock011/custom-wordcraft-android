# Changes to the WordCraft engine in this fork

This fork is a UI redesign for Android on top of the WordCraft engine (see the
[README](../README.md)). This page lists every change it makes **below the user interface**, so
anyone can see exactly where it departs from the original engine. All of these were written by AI
coding agents, like the rest of the fork.

Baseline: this fork branches from [storytold/wordcraft](https://github.com/storytold/wordcraft)
at commit `7086f8a` and keeps its full history, so later upstream versions can be merged in (see
[UPDATING.md](UPDATING.md)). To see the code itself, against whichever upstream version was last
merged:

```sh
git remote add upstream https://github.com/storytold/wordcraft.git   # once
git fetch upstream
git diff $(git merge-base HEAD upstream/main) HEAD -- crates ':!crates/ui-egui' Cargo.toml
```

"Engine" here means every crate except `crates/ui-egui` (the interface) and `apps/` (the
programs that host it). Changes to the shared interface crate are the phone layout and are not
listed one by one; they only apply in the phone layout, except for those listed at the end.

## Behaviour changes

| Area | File | What changed | Why |
|---|---|---|---|
| Recovery | `crates/engine/src/recovery.rs` | New `Session::flush_recovery()` asks for a recovery copy now instead of at the next 10-second interval. | Android can kill a backgrounded app without warning; the Android host calls this when the app goes to the background, so the last edits are saved. Nothing calls it on desktop. |
| Recovery | `crates/engine/src/recovery/desktop.rs` | When the platform has no file locks at all (`std` reports `Unsupported`, as on Android), the recovery copy is opened without a lock instead of failing. Any other lock failure is still an error. New `Recovery::flush()`. | On Android every recovery write failed, so no copy was ever kept. Android runs one process, so the missing lock guards against nothing there. |
| Pictures | `crates/engine/src/cmd/insert.rs` (`place_picture`) | A new picture is fitted to the reflowed column (window width minus 36 pt) in Web Layout and Read Mode, instead of the printed page's text width. Print and Draft views are unchanged. | On a phone the picture came in about 1.7 times wider than the column. |
| Headers and footers | `crates/engine/src/cmd/insert.rs` (`hf_part`) | A new header or footer is also given to every section that has none of its own. | As in Word, a page number inserted after a section break should show in every section, not only the one with the caret. |
| View | `crates/engine/src/cmd/view.rs` | Switching into Web Layout resets zoom to 100% and clears the Print view's fit mode (Page Width, One Page…). | Web Layout is the phone's edit view; it was inheriting the Print view's page-width zoom. |
| Draw | `crates/engine/src/cmd/draw.rs`, `crates/engine/src/cmd/objects.rs` | Done (`draw.select`) clears the lasso and marks the ink object next to the caret as dismissed, so it no longer counts as selected until the caret moves. | After drawing and tapping Done, a drag moved the stroke instead of selecting text. |
| Review | `crates/engine/src/cmd/review.rs` (`list_comments`) | Each comment in the list also reports `anchorText`: the text it is attached to, within one paragraph, at most 200 characters. | Lets the phone's comment list show what each comment is about. Additive; existing fields are unchanged. |
| Fonts | `crates/fonts/src/fontdb.rs` | On Android, system fonts are looked up in `/system/fonts`, `/product/fonts`, `/system_ext/fonts` and `/vendor/fonts`, through a new `android_font_dirs()`. | Android keeps its fonts there, not in the Linux paths the code used. Other platforms are unchanged. |
| Read Aloud | `crates/engine/src/speech.rs` | New `Backend::Host`, a `SpeechHost` trait (speak, poll, stop) and `ReadAloud::use_host()`. The same worker and queue speak through the host instead of a system program. Desktop and web keep their backend; `Hold` (silence) is kept. | Android has no speech program to spawn. The Android host speaks through `TextToSpeech`, so the player works there without any change to desktop behaviour. |

## No behaviour change

| File | What changed |
|---|---|
| `crates/doc/src/math_linear.rs` | One condition rewritten to satisfy a newer clippy lint (`nonminimal_bool`). Same logic. |
| `crates/proof/src/affix.rs` | One `match` arm restructured for a clippy lint (collapsible match). Same logic. |
| `Cargo.toml` | The workspace excludes a stray tool folder (`apps/.claude-mpm`) so it isn't treated as a crate. |

## Tests added

- `crates/engine/src/tests.rs`: `picture_fits_the_web_layout_column`.
- `crates/engine/src/cmd/insert.rs`: `page_number_reaches_every_section`.
- `crates/engine/src/cmd/draw.rs`: `done_drops_the_selection_left_by_a_stroke`.
- `crates/engine/src/recovery/desktop.rs`: `flush_copies_the_latest_edit_before_the_interval`,
  `only_a_platform_without_file_locks_opens_a_record_unlocked`.
- `crates/fonts/src/tests_sysfonts.rs`: `android_font_folders_are_scanned`.

## Interface changes that also reach desktop

The phone layout lives in `crates/ui-egui`, which desktop and web share. Almost all of it is
switched on only in the phone layout. These apply everywhere:

- Pressing Escape while the right-click (context) menu is open now only closes the menu; it no
  longer also collapses the selection.
- Recent files can be opened through a host hook (`Services::open_location`) before the file
  system is tried. Desktop and web don't set the hook, so they behave as before.

Last updated: 2026-10-25.
