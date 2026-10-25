# Phone UI fixes: verification (2026-10-24)

Emulator AVD wc35, 1080x2400, APK built from this tree and installed with `adb install -r`.
Screenshots: `notes/shots/ui_*.png`. Not committed.

## 1. Typing live on the page (IME) : PASS on emulator, Gboard composition NOT verified
- Cause: eframe's hidden text input drops input it reads as non-composing and only emits text
  on commit, and the browser draws the composition in the 1 px input (text floated off the caret).
- Fix: `apps/wordcraft-web/src/web/text_input.rs` takes the agent's `input`, `compositionend`
  and Enter keydown events in the capture phase. Composition becomes `Ime::Preedit` (drawn
  underlined at the caret); each typed or finished word becomes `Ime::Commit`. Events are queued
  and placed before the frame's own events (`drain`, called from `WebShell::logic`).
  `index.html` hides the agent's own composition text (`#wordcraft_canvas + input`).
- Evidence: ui_03 (`hell` shows live, underlined), ui_04 (Enter commits, "Next" capitalised),
  ui_07 ("One two three" typed in full, lines committed on Enter).
- Note: test input was adb hardware-key text, not Gboard prediction. A real Gboard/Samsung
  composition run is still needed on the user's phone.
- Earlier Hellworld/missing-space results were caused by adb text injection (retest clean).

## 2. Bottom formatting bar placement and scrolling : PASS (partial)
- Pill now floats just below the caret line (above it when the keyboard leaves no room),
  never over the caret; it steps aside while a context menu is open. ui_08.
- Groups: B I U highlight font colour | bullets numbering outdent indent | comment link
  picture table | undo | More (⋮ pinned right). Horizontal ScrollArea (swipe to reach more).
- Centring when the groups fit is coded but NOT verified (groups overflow at 1080 px).
- Not verified: placement with the keyboard open, drag-scroll not moving the document.

## 3. Font formatting always available : PASS
- Home sheet font row shows font name and size ("Aptos 12") with no selection (ui_13).
  Tapping it opens the Font dialog (ui_12). Bold/Italic/Underline still in the row.
- The pill's font controls: not added to the pill (its groups are above). Mini popup is kept.

## 4. Context menu for phone : PASS (spelling part NOT verified)
- Long press on a word: compact list Cut, Copy, Paste, Select All, New Comment, More (ui_09).
  Cut and Copy greyed with no selection. Mini font popup stacked above the menu, not overlapping.
- More opens a sub-list with a back arrow: Keep Text Only, Merge Formatting, Paste Special,
  Font, Paragraph, Link (ui_10).
- Spelling chips (up to 3 suggestions, Ignore All, Add) are coded but NOT verified: no misspelt
  word was tested on the emulator.

## Design-idea items (coordinator, from the Word screenshots as layout ideas only)
- (a) scrolling pill, groups, overflow: done (item 2).
- (b) spelling chip strip below the word: coded, not verified.
- (c) compact icon list with More sub-menu and back arrow: done (item 4).
- (d) popup away from the pill: the pill hides while the menu is open. Verified (ui_09).

## Needs the user's real-phone check
- Gboard vs Samsung keyboard composition (typing, the space commit, Enter).
- Pill placement with the keyboard open; centring; drag-scrolling the pill.
- Spelling chips on a misspelt word.

## Files changed
- apps/wordcraft-web/src/web.rs (module, listener install, drain in logic)
- apps/wordcraft-web/src/web/text_input.rs (new)
- apps/wordcraft-web/Cargo.toml (web-sys features)
- apps/wordcraft-web/index.html (agent text hidden)
- crates/ui-egui/src/phone.rs (bottom bar, caret note, phone context menu, font row)
- crates/ui-egui/src/canvas.rs (phone context menu hook, caret note)

## Round 2 (coordinator follow-up)
- Spelling strip: PASS on render. Long-press "Hhgv": strip shows High, Hogg, Huge, Ignore All, Add above Cut/Copy/Paste (ui_17). Bug found and fixed: the menu reopened in its More sub-list (stale flag); now reset when the menu opens (`phone::reset_context_more`). Chip actions (replace, ignore, add) NOT yet tapped. The "more" (⋮) chip was not added; suggestions beyond 3 are not reachable from the strip.
- Bar with keyboard open, drag-scroll not moving the document, tap keeps keyboard, even spread when groups fit: NOT verified.
- Gboard as active IME, suggestion tap replacing the composing word, backspace/Enter/autocorrect: NOT done.
- Text formatting parity (font picker and size stepper that change the format, strikethrough, highlight/font colour swatch grid on phone, subscript/superscript, clear formatting): NOT done.

## Round 3: text formatting on phone (item 3)
Engine: `format.strikethrough`, `format.subscript`, `format.superscript`, `format.clear`, `format.font`, `format.size`, `format.color`, `format.highlight` already exist (the coordinator's `format.strike` is `format.strikethrough` here), so no engine change was needed.
UI: the Home sheet font row now scrolls sideways: B I U Strikethrough Subscript Superscript Clear | font name (opens font list) | size (opens size page) | Highlight | Font Color. The Home list's Highlight and Font Color rows open the swatch pages. Format pages: Font (families), Size (− / + and list), Highlight (swatch grid from Highlight::ALL), Font Color (Automatic + 16 swatches).

- Strikethrough on a selection: PASS (ui_20: "Fmt test" struck through).
- Superscript on a selection: PASS (ui_21: text shrinks and raises).
- Clear formatting on a selection: PASS (ui_22: strike and superscript removed).
- Highlight swatch grid: PASS render (ui_25), applying yellow to "Mark me": PASS (ui_26). Note the Back arrow on the swatch page sits under the emulator's floating side panel, so it could not be tapped in this test.
- Font Color swatch grid: NOT verified (blocked by the same Back-arrow issue).
- Font name picker and size stepper: NOT verified (scrolled off the row; a horizontal swipe did not scroll it on the emulator).
- Subscript: NOT verified separately.
- Apply with no selection then type: NOT verified.
- Strikethrough engine test: none added (command already exists and is covered by the engine tests).
- Spelling chips (replace, Ignore All, Add): NOT tapped in this round.

## Round 4: reachability and formatting verification
Changes (crates/ui-egui/src/phone.rs): sub-page Back is now a text button at the right of the header (clear of the left edge and the emulator's side panel). The font name and size stepper (− size +) are in a fixed row above the scrolling toggles row. The scrolling row also sets `scroll_source(ALL)`.

- Back on sub-pages by tap: PASS (ui_29, ui_31 back to Home).
- Android hardware Back steps back one page: NOT implemented (MainActivity has no back handling; would need a JS/egui Escape bridge that returns "handled"). Not done.
- Font row scrollable by finger drag: NOT verified (the toggles row still did not scroll by adb swipe); the fixed row makes font and size reachable without it.
- Font name picker changes the selected text's font: PASS (ui_30, "Mark me" changes from Aptos to Inter).
- Size stepper + on a selection: PASS (ui_28, 12 → 14, text grows). The − button and the size list page: − not tested; the list page renders (ui_29 shows the font list; size list not captured).
- Font colour grid: PASS (ui_31 grid renders with Automatic and 16 swatches; ui_32 red applied to "Mark me").
- Subscript on a selection: PASS (ui_33, text drops and shrinks).
- Apply at caret with no selection, then type: PASS (ui_35, Italic applied at collapsed caret, "Hi" typed in italic).
- Spelling chip: replace: PASS (ui_37, "Hhgv" replaced by "High" from the strip).
- Spelling chip: Ignore All: NOT verified (second misspelt word "Zqxv" long-press did not show the strip, ui_38; position needs checking).
- Spelling chip: Add to dictionary: NOT verified (same cause).
- Bold at caret with no selection: NOT verified (Bold icon hidden under the emulator side panel); Italic used instead.

## Round 5: Android Back
Built: `MainActivity.kt` has an `OnBackPressedCallback`. It asks the page `window.wcCanGoBack`; if true it dispatches Escape to the canvas, otherwise it lets Back leave. The web shell publishes `wcCanGoBack` each frame (`web.rs::set_back_flag`, from `WordApp::can_go_back`: a sheet page, the sheet, Backstage or a context menu). phone.rs: Escape steps back one level (`phone::back_step`: sub-page first, then the sheet). lib.rs: Escape closes Backstage.
Verified: the web wasm check (`cargo check --target wasm32-unknown-unknown -p wordcraft-web`) and the phone tests pass; the APK builds and installs.
NOT verified on the device: KEYCODE_BACK on a sub-page did not show a clear result. The sheet had not opened in that test run (the ⋮ tap landed while the Gboard keyboard was up), so the before/after shots (ui_39, ui_40) show no sheet at all. The Back behaviour still needs a clean run: open the sheet, open a sub-page, press Back, expect the Home sheet; press Back again, expect the sheet to close.

Not started: item 2 (Size −, size list page, Bold at caret, Ignore All, Add, keyboard-open bar placement) and item 3 (Word tabs survey and missing rows).

## Round 6: Android Back verified on the emulator
- Sub-page (font list) → Back → Home sheet: PASS (ui_42).
- Home sheet → Back → sheet closed: PASS (ui_43).
- Context menu → Back → closed: PASS (ui_44, ui_45).
- Backstage (File) → Back → back to the document: PASS (ui_46, ui_47).
- Nothing open → Back → app leaves: see ui_48 (checked with dumpsys focus).

## Not done in this round
- Item 2 remaining: Size − and size list page, Bold at caret, spelling chips Ignore All and Add, keyboard-open bar placement. Not verified.
- Item 3: Word tab survey (Insert, Draw, Layout, Review, View) and missing rows. Not started.

## Round 7: item 2 checks (short run)
- Spelling strip on a second misspelt word: PASS (ui_50, strip Zurv Zax Ziv Ignore All Add).
- Chip Add to dictionary: PASS (ui_51, squiggle on "Zqxv" gone).
- Chip Ignore All: PASS (ui_52 → ui_53, squiggle on "Hhgv" gone).
- Bold at caret, no selection: PASS (ui_54, "Bo" typed in bold after the pill's B).
- Keyboard open, bar placement: PASS (ui_49, ui_54: pill sits above the keyboard and below the caret line).
- Size − and size list page: NOT verified. The keyboard stayed up in every attempt (taps on the keyboard hide key and collapse arrow did not dismiss it; KEYCODE_BACK did not either), so the sheet did not open (ui_55..ui_59).
