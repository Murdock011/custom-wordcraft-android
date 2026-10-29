# Vendored: egui (storytold/egui rev 6d518fc, egui 0.37)

egui 0.37.0 by Emil Ernerfeldt and contributors, MIT OR Apache-2.0 (https://github.com/emilk/egui),
from the storytold/egui fork at rev 6d518fc799681571963842f95b55a6b68431b15a. Inheritance from its workspace is resolved to concrete values so it builds standalone.

Patched by WordCraft:

- `src/widgets/text_edit/builder.rs`: a focused text field that is off screen (scrolled away, or under the soft keyboard) still reports its IME output (`ime_context_generation != 0`, the field owns IME events, and its inner rect is not visible: it scrolls the field into view and emits the IME output). Without it the keyboard flickers: egui drops the output, Android hides the keyboard, the field is uncovered, the output returns, and Android shows it again.

Wired in through the root `Cargo.toml` `[patch.crates-io]` table. Remove it when upstream egui ships this fix.
