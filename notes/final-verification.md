# Final verification (2026-10-24) - partial

Build: /tmp/wc_build_install.sh OK (emulator-5554, Android 15, 1080x2400), rebuilt after the Done fix.
Tab route: keyboard hidden (BACK), tap ⋮ at the right of the pill (936,2222), tap the tab chip (174,1220), pick the tab (Review at 120,1764; Draw at 112,2036).

## A. Draw tab
- Open Draw tab: PASS (shots/fv_drawtab.png).
- Highlighter stroke: PASS (shots/fv_A_highlight.png).
- Draw mode persists after the sheet closes: PASS (shots/fv_A_modecheck.png).
- Done returns to normal touch: FAIL before the fix (shots/fv_A_done.png): after Done, a drag moved the stroke and showed a selection box.
  Cause: a caret next to an ink object counts as selecting it (objects::selected_all), and the stroke's insertion point is that caret.
  Fix: draw.select (Done) records the dismissed object in DrawState.dismissed and clears the lasso; selected_all skips a dismissed object at a collapsed caret.
  Test: cmd::draw::tests::done_drops_the_selection_left_by_a_stroke PASS (all 21 draw tests pass).
  Emulator after fix: PASS (shots/fv_A_done2.png): after Done and a drag below the page, no selection box and the stroke did not move.
  Not checked: scrolling over text (the test document is one page).
- Lasso select, colour change, thickness change, redo, Save and reopen with ink: NOT RUN.

## B. Review
- Review tab rows: PASS (shots/fv_B_review.png).
- Track Changes on, typed "tracked": PASS, magenta insertion markup (shots/fv_B_track.png).
- Accept All Changes: the magenta markup goes, but "tracked" keeps a blue double underline: FAIL, the reported bug reproduces (shots/fv_B_acceptall.png). Not investigated yet.
- NOT RUN: Next/Previous Change, Reject, single Accept, Next/Previous Comment, Spelling & Grammar, View > One Page, a second comment in the list.

## C. Save/Open: NOT RUN
## D. Phone UX: NOT RUN

## E. cargo xtask ci
- fmt: ok (cargo fmt ran).
- clippy: FAIL, stops at the first failing crate. Fixed two in files we did not otherwise change: crates/doc/src/math_linear.rs:559 (nonminimal_bool), crates/proof/src/affix.rs:313 (collapsible match arm).
  Still failing, in files we did not change (pre-existing with rustc 1.95 clippy):
  - crates/docbin/src/officeart.rs:316 and :325 (collapsible if)
  - crates/docx/src/read/story.rs:735 (collapsible if)
  - crates/layout/src/lib.rs:3121 and :4424 (nonminimal_bool)
  Clippy stops at the first crate, so more may follow after these.
- test, i18n-report, doctest, assets, layers, wasm: skipped by the failing clippy step, not run.
