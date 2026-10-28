# Vendored: egui-winit 0.36.2

egui-winit 0.36.2 by Emil Ernerfeldt and contributors, MIT OR Apache-2.0 (https://github.com/emilk/egui).

Vendored unmodified except for one patch, applied by WordCraft:

- `src/lib.rs`: on Android, every `window.set_ime_allowed(..)` call is skipped (the IME toggle, the interrupt-composition false/true pair, and `ViewportCommand::IMEAllowed`). On Android the app's own keyboard bridge (MainActivity) owns the soft keyboard; GameActivity's toggling made it flicker. Other platforms are unchanged.

Wired in through the root `Cargo.toml` `[patch.crates-io]` table. Remove it when upstream egui-winit ships this fix.
