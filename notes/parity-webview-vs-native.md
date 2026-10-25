# Parity audit: WebView app vs native app (2026-10-25)

WebView: `ai.storyteller.wordcraft/.MainActivity`. Native: `ai.storyteller.wordcraft.native/ai.storyteller.wordcraft.nativeapp.MainActivity`.
Emulator wc35, 1080x2400, Android 15, Gboard, installed APKs as found (not reinstalled). Audit only: no source changed.
Screenshots: `~/.cache/wc-spike/parity/<item><w|n>_<what>.png` (`_s.png` = 360x800). Helper: `~/.cache/wc-spike/parity/s.sh`.
Checklist built from notes/phone-ui-fixes.md, save-fix.md, edit-print-view.md, final-verification.md, word-tabs-verification.md and native-spike-plan.md Phase 1.

## Summary
- Counts (71 items): SAME 55, DIFFERENT 9 (4 of them native better: 13, 36, 43, 71), BROKEN IN NATIVE 2, MISSING IN NATIVE 1, NOT RUN 4 (37 and 54 ran on native only). Shared bugs (broken in both) are counted as SAME with a note; 50, 51, 52 are SAME for what was run (rows, stroke), not the full feature.
- Blockers:
  1. Item 61: after Back leaves the native app (activity finished, process alive), launching it again hangs on the splash and ends in "WordCraft Native isn't responding". Reproduced twice (61n_relaunch2b, 33n_lp2 = ANR dialog). logcat: `Activity destroy timeout` for the old MainActivity, then `Input dispatching timed out (Application does not have a focused window)`. Only force-stop recovers.
  2. Item 66 (shared, but worse on native): Android Back while a dialog (Font, Save-changes prompt) is open does not close the dialog; it leaves the app and the unsaved document is lost. On native the next launch then hangs (blocker 1).
- Should fix: plain Save always asks where (62); no Print button (9); Recent cannot reopen in either app (65); long-press word selection drops the last letter in both apps ("peopl", "otherwis") (33/40).
- Native better than WebView: dark mode followed and document kept (WebView recreates the activity and loses the open document, 71); system fonts listed (43); first Backspace not lost (13); spelling strip appeared on long press (36).
- Test-environment note: the emulator process died twice when the scripted switch force-stopped one app and started the other (`gfxstream ... error null ctx` in the emulator log). Switching with HOME + `am start` was stable; a later `am force-stop` of the native app did not kill the emulator. Not proven to be an app bug.

## Results

| # | Behaviour | How to test | Result | Severity | Notes / root-cause guess |
|---|---|---|---|---|---|
| 1 | Launch screen | cold start | DIFFERENT | minor | WebView opens Backstage Home (01w_launch); native opens the sample document with the keyboard up (00n_launch). `apps/wordcraft-android/src/app.rs` `Shell::new` loads `sample_document()`. |
| 2 | Open sample from Home | Home > Studio handbook | SAME | | 02w_sample; native via Home (68n_pre). |
| 3 | Open blank document | Home > Blank document | SAME | | 03w_blank, 03n_blank. |
| 4 | Backstage menu list | file icon, hamburger | SAME | | 04w_menu, 04n_menu: Home New Open Info Recover Save Save As Print Export Close Options. |
| 5 | Backstage New | | SAME | | 05w_new, 05n_new. |
| 6 | Backstage Open | | DIFFERENT | minor | Native adds "Browse in WordCraft..." (in-app file browser) (06n_open vs 06w_open). |
| 7 | Backstage Info | | SAME | | 07w_info, 07n_info. Both say Pages: 1 while Print says 2 of 2 pages (shared oddity). |
| 8 | Backstage Save As | | SAME | | 08w_saveas, 08n_saveas. |
| 9 | Backstage Print | | MISSING IN NATIVE | should fix | WebView has a "Print" button (system print dialog) (09w_print); native shows only Save as PDF and export formats (09n_print). Native `Services` sets no `print` hook (app.rs ~line 37). |
| 10 | Tap text: caret + keyboard | tap a word | SAME | | 10w_tap, 10n_tap. Both put the caret inside "Thursda\|y". |
| 11 | Typing at caret | adb text | SAME | | 11w_type, 11n_type. |
| 12 | Gboard key taps | tap h, i | SAME | | 12w_keys, 12n_keys. |
| 13 | Backspace | Gboard backspace | DIFFERENT | minor (WebView) | Native deletes on the first press (13n_bksp). WebView ignored the first press after Gboard letters; two presses deleted one char (13w_bksp, 13w_bksp2). Native better. |
| 14 | Enter splits paragraph | Gboard action key | SAME | | 14w_enter, 14n_enter. Gboard shows a "→" action key on WebView, "↵" on native. |
| 15 | Auto-capital after Enter | Enter, type | NOT RUN | | adb text bypasses Gboard auto-capitalisation; not tested by key taps. |
| 16 | Back hides keyboard | BACK with keyboard up | SAME | | 16w_kbhidden, 16n_kbhidden. Caution in both: BACK with the keyboard already hidden leaves the app. |
| 17 | Header bar | | SAME | | file icon, title, undo, redo, search in both. |
| 18 | Undo | header undo | SAME | | 18w_undo, 18n_undo (step granularity not compared, inputs differed). |
| 19 | Redo | header redo | SAME | | 19w_redo, 19n_redo. |
| 20 | Search opens | header search | SAME | | 20w_search, 20n_search (Search Commands). |
| 21 | Find a word | type Studio, Find | SAME | | 21w_find2, 21n_find2 select "studio". Native moves the dialog to the top while typing (minor). |
| 22 | Pill above keyboard | | SAME | | 02w_sample, 00n_launch. |
| 23 | Pill, keyboard hidden | | SAME | | 16w_kbhidden, 16n_kbhidden. |
| 24 | Pill scrolls | swipe pill | SAME | | 24w_pillscroll, 24n_pillscroll. |
| 25 | Bold on selection | | SAME | | 25w_bold, 25n_bold. |
| 26 | Italic | | SAME | | 26w, 26n. |
| 27 | Underline | | SAME | | 27w, 27n. |
| 28 | Highlight from pill | | SAME | | 28w_hl, 28n_hl. |
| 29 | Font colour from pill | | SAME | | Opens a Colors dialog in both; in both the dialog is wider than the screen and cut off at the left (29w_color, 29n_color). Shared layout bug. |
| 30 | Bullets | | SAME | | 30w, 30n. |
| 31 | Numbering | | SAME | | 31w, 31n. |
| 32 | Bold at caret then type | | SAME (caveat) | | WebView: word not bold, B bolds the word and "Bo" types bold (32w_boldcaret). Native: caret was in a bold word, B unbolded it and "Bo" typed plain (32n_boldcaret). Same rule, different starting state. |
| 33 | Long press: word + menu | hold 0.9 s | DIFFERENT | minor | Both select the word and show Cut/Copy/Paste/Select All/New Comment/More. WebView raises the keyboard and puts the menu near the top (33w_longpress); native keeps the keyboard down (33n_lp2). Shared bug: the selection often misses the last letter ("peopl" in 34n_more and 59w_back1; native "otherwis", "bo" of "borrows"). Guess: `canvas.rs` `long_press_word` end offset. |
| 34 | Context menu More | | SAME | | 34w_more, 34n_more. |
| 35 | Mini font popup | | SAME | | above the menu in both. |
| 36 | Spelling strip | long press "ceramicist" | DIFFERENT | should fix (WebView) | Native shows "ceramics, ceramist, Ignore All, Add" (36n_spell). WebView showed no strip in two tries (36w_spell, 36w_spell2), after its keyboard rose on the long press. Native better here. |
| 37 | Spelling chip replace | tap chip | NOT RUN (WebView) | | Native PASS ("ceramist", 37n_chip). WebView had no strip. |
| 38 | One-finger scroll | slow swipe | SAME | | 38w_scroll, 38n_scroll. |
| 39 | Selection handles | | SAME | | No handles visible in either app. |
| 40 | Copy / Paste | menu Copy, Paste | SAME | | Both go through the Android clipboard (system overlay), paste works (40w_paste, 40n_paste). Same off-by-one as 33. |
| 41 | Pill More opens sheet | | SAME | | 41w_sheet, 41n_sheet. |
| 42 | Home sheet font row | | SAME | | name, − size +, B I U ab x2 x2 clear highlight. |
| 43 | Font list page | font name | DIFFERENT | (native better) | Native lists Android system families (AndroidClock, Carrois Gothic SC, ...) (43n_fontlist); WebView lists bundled fonts only (43w_fontlist). Font apply not re-tested. |
| 44 | Size stepper + | | SAME | | 44w (28→36), 44n (12→14); both step to the next listed size. |
| 45 | Highlight swatch page | | SAME | | 45w, 45n. |
| 46 | Font colour grid | | SAME | | 46w, 46n. |
| 47 | Strikethrough / clear | | NOT RUN | | time. |
| 48 | Tab chip | | SAME | | Home Insert Layout Review View Draw (48w, 48n). |
| 49 | Insert tab rows | | SAME | | 49w, 49n. |
| 50 | Layout tab | | SAME (rows only) | | 50w, 50n. Margins sub-page not opened. |
| 51 | Review tab | | SAME (rows only) | | 51w, 51n. Track Changes not toggled. |
| 52 | Draw pen stroke | Pen, swipe | SAME | | 52w_stroke, 52n_stroke. In both the keyboard rises after the stroke (shared). Done not checked. |
| 53 | Print view | View > Print view | SAME | | 53w_print, 53n_print. |
| 54 | Page card + page number | | NOT RUN (WebView) | | Native footer "2" on page 2 (53n_print). WebView not scrolled to a page end. |
| 55 | Zoom in | View > Zoom in | SAME | | 55w, 55n. |
| 56 | Edit view resets zoom | | SAME | | 56w_edit2, 56n_edit2. |
| 57 | Back: sub-page → Home sheet | | SAME | | 57w_back, 57n_back. |
| 58 | Back: sheet → closed | | SAME | | 58w_back, 58n_back. |
| 59 | Back: context menu → closed | | SAME | | 59w_back1, 59n_back1 (closes the whole menu from the More sub-list). |
| 60 | Back: Backstage → document | | SAME | | 60w_back, 60n_back. |
| 61 | Back: nothing open | | BROKEN IN NATIVE | blocker | Both leave the app and drop unsaved edits with no prompt (shared). Native: the next launch hangs on the splash, ANR (61n_relaunch2b). `MainActivity.kt` Back handler calls `finish()`; the process stays alive and a second GameActivity instance never gets a running eframe loop (`android.rs` `android_main` → `eframe::run_native`; winit/eframe cannot run twice per process). Guess fix: `moveTaskToBack(true)` instead of `finish()`, or end the process when `android_main` returns. WebView relaunch is fine (fresh Home). |
| 62 | Save | Backstage > Save | DIFFERENT | should fix | First Save: both open the system picker with the document name and write the file. Second Save: WebView rewrites silently (Document1.docx mtime 05:28:39, 62w_save2); native opens the picker again (62n_save2). Native `download` hook ignores `save_as` and keeps no URI (ponytail note in app.rs). Once, native Open then asked "Do you want to save changes" after a cancelled second Save and a cancelled Save As (64n_opened first run); a clean Save then Close did not ask (62n_dirty). Not reproduced cleanly. |
| 63 | Save As → picker | Save As > Browse | SAME | | 63w_saveas, 63n_saveas; Back cancels to the document. |
| 64 | Open → picker → document | Open > Browse > quokka.docx | SAME | | 64w_opened, 64n_opened, title "quokka". Native picker shows all types (Images/Audio chips), minor. |
| 65 | Recent | Home > Recent entry | SAME (broken in both) | should fix | Both list quokka.docx; tapping fails: WebView "files are opened through the browser on the web" (65w_reopen), native "No such file or directory (os error 2)" (65n_reopen). SAF URIs not persisted. |
| 66 | Font dialog (+ Back) | menu More > Font | BROKEN IN NATIVE (shared cause) | blocker | Dialog opens in both (66w_font, 66n_font). BACK on the open dialog leaves the app in both (66w_dlgback, 66n_dlgback), losing the document; on native this leads into item 61's hang. `WordApp::can_go_back` (`crates/ui-egui/src/lib.rs:2419`) counts phone sheet, Backstage and context menu, not modal dialogs. |
| 67 | Paragraph dialog | menu More > Paragraph | SAME | | 67w_para, 67n_para (both slightly wider than the screen). |
| 68 | Insert table | Insert > Table > OK | SAME | | 68w_table2, 68n_table2 (5x2, replaces the selected word in both). |
| 69 | Table renders | scroll to table | SAME | | 38w_scroll, 38n_scroll. |
| 70 | Rotation | user_rotation 1 then 0 | DIFFERENT | minor | Both keep the document and the phone layout in landscape. WebView shot had the keyboard down (full-width pill, 70w_land); native had the keyboard up, leaving a ~1-line page band (70n_land). Not the same starting state; landscape-with-keyboard is a known gap in both (edit-print-view.md round 4). |
| 71 | Dark mode | cmd uimode night yes/no | DIFFERENT | should fix (WebView) | Native follows night (dark header, pill and bars; page stays white) and keeps the document (71n_dark, 71n_light). WebView stays light and the activity is recreated: the open document is gone and Home shows (71w_dark). Native better. |

## Not run / not seen
- 15 auto-capital, 47 strike/clear, 54 on WebView, 37 on WebView, Margins/Track Changes actions, Draw Done.
- Gboard composing underline, suggestion tap: not part of this run.
- Physical phone: nothing.
