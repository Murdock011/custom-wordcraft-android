# Vendored: egui-winit (storytold/egui rev 6d518fc, egui 0.37)

egui-winit 0.37.0 by Emil Ernerfeldt and contributors, MIT OR Apache-2.0 (https://github.com/emilk/egui),
from the storytold/egui fork at rev 6d518fc799681571963842f95b55a6b68431b15a. Inheritance from its workspace is resolved to concrete values so it builds standalone.

Patched by WordCraft:

- `src/lib.rs`: on Android, every `window.set_ime_allowed(..)` call is skipped (the IME toggle, the interrupt-composition false/true pair, and `ViewportCommand::IMEAllowed`). On Android the app's own keyboard bridge (MainActivity) owns the soft keyboard; GameActivity's toggling made it flicker. Other platforms are unchanged.

Wired in through the root `Cargo.toml` `[patch.crates-io]` table. Remove it when upstream egui-winit ships this fix.
