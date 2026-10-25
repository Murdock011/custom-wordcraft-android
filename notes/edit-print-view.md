# Edit view vs Print view (Word observed black-box, 2026-10-24)

Doc: four typed paragraphs, Aptos (Body) 12 pt default (Home ribbon shows 12).
Shots: notes/shots/ev_word_*.png

## Word, Mobile View (= our Edit view)
- Text runs nearly full width: ~16 px left margin on a 900 px-wide shot, no page card, no page edges, no grey surround.
- Line spacing is generous (about 1.5), paragraph gap about one line.
- Continuous scroll, no page breaks visible.
- Spelling squiggles shown. Bottom pill (B, I, U, highlight, colour, list, more) docked above the keyboard.
- Evidence: ev_word_edit_nokb.png.

## Word, Print View
- Page is a single white sheet filling the screen width at 100 %, thin grey border, small body text.
- Paragraphs wrap at page margins (about 1 in); page continues below the screen.
- View sheet offers: Mobile View, Print View, Headings, 100 %, Zoom In, Zoom Out.
- Evidence: ev_word_print.png, ev_word_view_tab.png.

## WordCraft (to compare, see below after the change)
- Edit view (Web Layout): white page card with grey surround and shadow, text small and cramped.
- Print view: whole page scaled to width (phone.printView = view.printLayout + view.zoom pageWidth).

## Conclusion
- Edit view should be an endless full-width page with small side padding, comfortable text size, no page edges.
- Print view should keep the page card, with a readable zoom and a zoom control.

## WordCraft results (sample doc "The Open Studio Handbook", emulator, 2026-10-24)

### Edit view (endless page)
- PASS: paper spans the full width, no shadow or page edge, text about 12 pt (about 16 dp) in from each edge. Shot ev_wc_edit_after2.png.
- PASS: table reflows to the full width in Edit view. Shot ev_pre_more.png.
- PASS: scrolling. Fix in crates/ui-egui/src/canvas.rs: on phone the canvas used Sense::click_and_drag, so a one-finger drag was taken by the canvas (selection) and never reached the ScrollArea. Now the canvas is click-only on phone until a long press (held still for 0.4 s); the drag then selects. A slow adb swipe (400 ms) on the text now scrolls: ev_scroll_fix2.png, ev_scroll_table.png.
- NOT verified: long-press selection on the emulator, selection handles, pinch zoom.

### Print view
- PASS: opens at page width; page card with grey surround; page-number footer shows (page 2 shows "2"). Shot ev_wc_print_view1.png.
- NOT verified: header text on page 1, page-break position, Edit -> Print -> Edit round trip (see below).

### Edit <-> Print switch
- Print -> Edit: after switching back, the Edit view text looked smaller than before (page-width zoom appears to carry over). Shot ev_wc_front2.png. OPEN BUG, not fixed.

### Not done
- Bold/italic typing in WordCraft: not tested.
- Picture with Wrap Square in Web Layout: not tested (needs the file chooser; sample has no picture).
- Rotation of WordCraft: not verified (a rotation shot captured Word, not WordCraft; user_rotation reset to 0).
- Stray text: an earlier mis-tap typed "TR" into the sample title; not saved, not fixed.

## Round 3 (coordinator: zoom carry-over, long press, bold/italic, rotation, tests)
- Zoom carry-over: FIXED. crates/engine/src/cmd/view.rs `mode()`: entering Web Layout from another view resets zoom to 100 % and clears the fit mode. Verified Edit -> Print -> Edit: text back to normal size (ev_zoom_edit2.png).
- Long press: NEW. Hold still 0.4 s on a word (adb swipe x y x y 700) places the caret and selects the word, with the mini format bar (ev_longpress2.png selects "print"). Implemented in canvas.rs (`long_press_word`, touch block). Handle drag NOT verified: adb cannot hold and drag in one gesture.
- Bold / italic on the selection: PASS (ev_bolditalic.png).
- Rotation: WordCraft in front (focus checked). Landscape (2400x1080) shows the desktop-width layout with ribbon; keyboard covers the page (ev_wc_rotated2.png). Layout sane but the keyboard hides most of the page.
- Picture with Wrap Square in Edit view: NOT done (needs Insert > Picture file chooser).
- `cargo test -p wordcraft-ui-egui phone`: PASS, 4 passed (re-run after the long-press change).

## Round 4 (landscape and Wrap Square)
- Phone mode now decided by the shorter side (phone::active, min(width, height) < 600 pt). Landscape keeps the phone UI: top bar and bottom pill (ev_landscape_phone3.png). Phone tests pass.
- Landscape with keyboard: NOT fixed. The visible page band is about 130 px tall (ev_landscape_keyboard.png) and the pill covers the text, so the caret line is not visible. Needs keyboard clearance in landscape (caret scroll above the pill and keyboard).
- Full suite (cargo test -p wordcraft-ui-egui): 826 passed, 1 failed: mini_toolbar::tests::context_menu_toolbar_fits_near_the_bottom_right_of_a_narrow_window ("the menu has Copy"). Fails alone too. Not caused by the short-side change (same width, phone either way) or by the canvas sense change (forced sense still fails). Not investigated further.
- Wrap Square picture in Edit view: NOT done.
- Emulator note: another agent's app (ai.storyteller.wordcraft.native) was in focus at the start of this round.
