# Save fix (Android, Save / Save As)

> Note: the WebView build in packaging/android/ was removed after 0.2.0; paths below are historical.

## Flows found (before)
- Web build (`apps/wordcraft-web/src/web.rs`): `download()` tried `showSaveFilePicker` first, then fell back to a blob anchor click. The Rust side marked the save done right after the click, so nothing reported the real write result.
- Android shell (`MainActivity.kt`): a JS shim turned blob clicks into `AndroidSave.save(name, base64, mime)`, which wrote through MediaStore and asked Replace / Keep both / Cancel for an existing name. Failures were logged only as toasts, and the page never heard about them.
- Save As (Backstage > Save As > Browse): `file_dialogs.rs` `AfterPick::SaveAs` called `file.save`. On the web, `pick_save` returns the default name at once, so Browse saved silently without ever showing a picker. This is why Save As "did nothing" on the device.
- Save: `file.save` with no path uses the document's own name; on the web it goes through `download()`.
- Backstage phone view: Save = `file.save`, Save As = Browse (`save_as_page`), Save as Word Template = `ui.saveAsTemplate`.

## Root causes (believed)
1. Save As never reached a picker on the web: `pick_save` answers immediately, and Save As ran `file.save`.
2. The Android bridge could not tell Save from Save As and never reported a real outcome, so a failed write was either silent or showed as an unrelated warning.
3. MediaStore insert/update on files from earlier sessions (name collisions, rows not owned by the app) is fragile across Android versions. Replaced with the system picker (Storage Access Framework).
4. The web download path could try `showSaveFilePicker` first on a WebView that exposes it without a usable picker. The Android path now skips it.

## What changed
- `apps/wordcraft-web/src/web.rs`: `download(..., save_as)`. On Android (`window.AndroidSave` present) it starts a save ticket, sets `data-save-as` on the anchor and clicks it. `window.wcSaveResult(name, status, error)` finishes the ticket: saved, cancelled, or failed with the error text. `trigger_anchor_download` takes `save_as`.
- `crates/ui-egui/src/lib.rs`: `Services.download` gets a `save_as: bool` (true for Save As). The call site passes `id == "file.saveAs"`.
- `crates/ui-egui/src/backstage.rs`, `lib.rs` tests: closures take the new argument.
- `crates/ui-egui/src/file_dialogs.rs`: Save As on the web runs `file.saveAs` so the host shows its picker.
- `packaging/android/.../MainActivity.kt`: MediaStore code removed. `save(name, base64, mime, saveAs)`: Save writes to the URI last saved for that name (this session), otherwise (Save As, first save, or a failed write) opens `ACTION_CREATE_DOCUMENT` with the suggested name and the file's MIME type. The URI gets a persistable grant when the provider allows it. Results go to `wcSaveResult`. Errors are logged under tag `WordCraftSave` with the exception class and message. The Replace / Keep both / Cancel dialog is gone (the picker handles overwrite and renames collisions itself).
- Phone pill (`crates/ui-egui/src/phone.rs`): `bar_top` now docks the pill at the bottom of the visible area (above the keyboard when it is up); the caret-following code and `note_caret` are removed. `bar_clearance()` gives the canvas the room the pill takes.
- `crates/ui-egui/src/canvas.rs`: the caret scroll target keeps the pill's height clear at the bottom on the phone; the `note_caret` call is removed.

## Tests on the emulator (wc35, Android 15, build of 2026-10-24)
- PASS  Launch and open Blank document: app starts, Home shows, document opens.
- PASS  First Save: system picker opens with `Document1.docx` in Downloads/WordCraft. Pulled file is a valid .docx and holds the typed text ("SaveTest one").
- PASS  Second Save after an edit: silent write to the same file (picked in the same session). File text became "SaveTest oneMoreText".
- PASS  Name collision: `Document1.docx` already existed from earlier sessions. The picker saved as `Document1 (3).docx`; the file contains the new text. No error.
- PASS  Save As to a new name (Backstage > Save As > Browse): picker opens with the default name; renamed to `SaveAsTest.docx`, saved; pulled file contains "SaveAsNew". Before the Rust change this path wrote silently with the default name.
- PASS  Cancel the picker (Back twice): no error warning, document text intact, log shows "cancelled at the picker".
- PASS  Pill docked above the keyboard with the keyboard up (screenshot).
- NOT VERIFIED  Pill with the keyboard down, and the caret staying above the pill while typing near the bottom of the view.
- NOT VERIFIED  Save in a new session on an existing file (save bridge forgets the URI after restart, so Save asks where; expected, but only the Save As case was run after a reinstall).
- NOT VERIFIED  Open file from the device still works (the Open path was not exercised in this pass).
- NOT VERIFIED  Samsung picker and the user's exact phone version; the failure the user sees may also come from the old `showSaveFilePicker` branch on that WebView, which Android now skips.

## Could only be checked on the user's real phone
- Save, Save As, and Save again on the Samsung device (picker look, provider behaviour, `Downloads` naming).
- Whether the phone's WebView exposes `showSaveFilePicker` (this build bypasses it on Android).

## Known limits
- The URI is kept only for this app session; after a restart Save asks where again.
- Two documents with the same name share one saved URI for Save.
- `Services.download` now has a bool; any other host must pass it.
