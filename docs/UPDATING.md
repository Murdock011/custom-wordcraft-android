# Updating to a newer WordCraft engine

This fork keeps the full history of [storytold/wordcraft](https://github.com/storytold/wordcraft),
so a newer upstream version comes in with an ordinary `git merge`. Everything upstream adds arrives
with it; Git only stops on lines that both upstream and this fork changed.

## One-time setup

```sh
git remote add upstream https://github.com/storytold/wordcraft.git
git remote set-url --push upstream DISABLED   # this fork never pushes to the original project
```

## Each update

```sh
git fetch upstream
git switch main
git switch -c update-upstream            # do the merge on a branch
git merge upstream/main
```

If Git reports conflicts, they can only be in files this fork changed:

- the engine files listed in [ENGINE-CHANGES.md](ENGINE-CHANGES.md), and
- the shared interface crate `crates/ui-egui` (mostly `lib.rs`, `canvas.rs`, `keys.rs`,
  `backstage.rs`, `mini_toolbar.rs`) and the web host's `Cargo.toml`.

Resolve each conflict by **keeping upstream's change and re-applying this fork's change on top**,
never by dropping upstream's side. `crates/ui-egui/src/phone.rs`, `apps/wordcraft-android/` and
`packaging/android-native/` are this fork's own files and don't conflict.

Then check that nothing broke:

```sh
cargo test -p wordcraft-engine -p wordcraft-ui-egui -p wordcraft-android
cargo xtask wasm
sh packaging/android-native/build-native.sh x86_64    # and try it on an emulator
```

When it builds and the tests pass, merge the branch into `main`, push to this fork, and update
[ENGINE-CHANGES.md](ENGINE-CHANGES.md) if any engine change was dropped (upstream fixed it
themselves) or had to be reworked.

## Keeping updates easy

- Keep changes below the interface rare and small, and list each one in
  [ENGINE-CHANGES.md](ENGINE-CHANGES.md).
- Put new phone behaviour in this fork's own files (`phone.rs`, `apps/wordcraft-android/`) and
  keep edits to upstream's shared files to small hooks.
- Merge upstream often: a few weeks of changes conflict far less than a few months.

On 2026-10-28 the fork was merged with upstream v0.11.0; the conflicts were in `ATTRIBUTION.md`, `apps/wordcraft-web/Cargo.toml`, `crates/ui-egui/src/canvas.rs`, `crates/ui-egui/src/dialogs.rs` and `crates/ui-egui/src/lib.rs`.

The vendored egui and egui-winit in `vendor/` (IME patches, see their `LICENSE-NOTE.md`) must be re-vendored from the new versions, or removed together with their `[patch.crates-io]` entries, whenever upstream bumps egui.
