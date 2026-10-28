# Vendored: egui 0.36.2

egui 0.36.2 by Emil Ernerfeldt and contributors, MIT OR Apache-2.0 (https://github.com/emilk/egui).

Vendored unmodified except for one patch, applied by WordCraft:

- `src/widgets/text_edit/builder.rs`: a click in a text field that already has focus no longer calls `mem.request_focus(...)` again. Re-requesting focus interrupts the IME, which made Android hide and re-show the on-screen keyboard on every tap.

Wired in through the root `Cargo.toml` `[patch.crates-io]` table. Remove it when upstream egui ships this fix.
