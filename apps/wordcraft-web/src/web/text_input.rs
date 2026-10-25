//! Typing through the browser's hidden text input (eframe's text agent), taken over for
//! input methods and mobile keyboards.
//!
//! eframe's agent drops input it reads as non-composing (Gboard's composing text reports
//! `isComposing` false in Android WebViews), so typed words only reached the page when a space
//! committed them, and the browser drew the composition over the caret. This module takes the
//! agent's `input` and `compositionend` events in the capture phase, so eframe never sees them,
//! and turns them into egui events: the composition becomes `Ime::Preedit` (drawn inline at the
//! caret), and each commit becomes `Ime::Commit` as soon as the keyboard sends it.

use std::cell::RefCell;

use wasm_bindgen::JsCast as _;
use wasm_bindgen::closure::Closure;

thread_local! {
    /// Events waiting for the next frame, placed before the frame's own events.
    static QUEUE: RefCell<Vec<egui::Event>> = const { RefCell::new(Vec::new()) };
    /// The egui context, so a queued event wakes the app.
    static CTX: RefCell<Option<egui::Context>> = const { RefCell::new(None) };
}

/// Installs the listeners. The page's canvas is the agent's previous sibling, as eframe inserts it.
pub fn listen(canvas_id: &'static str, ctx: egui::Context) -> Result<(), String> {
    CTX.with(|c| *c.borrow_mut() = Some(ctx));
    let window = web_sys::window().ok_or("no window")?;
    let agent_of = move |target: Option<web_sys::EventTarget>| -> Option<web_sys::HtmlInputElement> {
        let input: web_sys::HtmlInputElement = target?.dyn_into().ok()?;
        (input.previous_element_sibling()?.id() == canvas_id).then_some(input)
    };
    let agent_of_input = agent_of.clone();
    let on_input = Closure::<dyn FnMut(web_sys::InputEvent)>::new(move |e: web_sys::InputEvent| {
        if let Some(input) = agent_of_input(e.target()) {
            e.stop_immediate_propagation();
            on_input(&e, &input);
        }
    });
    let agent_of_end = agent_of.clone();
    let on_end = Closure::<dyn FnMut(web_sys::CompositionEvent)>::new(move |e: web_sys::CompositionEvent| {
        if let Some(input) = agent_of_end(e.target()) {
            e.stop_immediate_propagation();
            commit(&input);
        }
    });
    let on_key = Closure::<dyn FnMut(web_sys::KeyboardEvent)>::new(move |e: web_sys::KeyboardEvent| {
        // Enter ends the word being composed: commit it before the Enter reaches the page.
        if e.key() == "Enter" {
            if let Some(input) = agent_of(e.target()) {
                commit(&input);
            }
        }
    });
    window.add_event_listener_with_callback_and_bool("input", on_input.as_ref().unchecked_ref(), true).map_err(|e| format!("{e:?}"))?;
    window.add_event_listener_with_callback_and_bool("compositionend", on_end.as_ref().unchecked_ref(), true).map_err(|e| format!("{e:?}"))?;
    window.add_event_listener_with_callback_and_bool("keydown", on_key.as_ref().unchecked_ref(), true).map_err(|e| format!("{e:?}"))?;
    // The listeners live as long as the page.
    on_input.forget();
    on_end.forget();
    on_key.forget();
    Ok(())
}

/// Moves the queued events to the front of this frame's events. Call at the start of a frame.
pub fn drain(ctx: &egui::Context) {
    CTX.with(|c| *c.borrow_mut() = Some(ctx.clone()));
    let queued = QUEUE.with(|q| std::mem::take(&mut *q.borrow_mut()));
    if !queued.is_empty() {
        ctx.input_mut(|i| {
            i.events.splice(0..0, queued);
        });
    }
}

fn push(e: egui::Event) {
    QUEUE.with(|q| q.borrow_mut().push(e));
    CTX.with(|c| {
        if let Some(ctx) = c.borrow().as_ref() {
            ctx.request_repaint();
        }
    });
}

/// Commits what the agent holds (the word the keyboard has finished) and empties it.
fn commit(input: &web_sys::HtmlInputElement) {
    let text = input.value();
    if !text.is_empty() {
        push(egui::Event::Ime(egui::ImeEvent::Commit(text)));
        input.set_value("");
    }
}

fn on_input(e: &web_sys::InputEvent, input: &web_sys::HtmlInputElement) {
    let text = input.value();
    let kind = e.input_type();
    if e.is_composing() || kind.starts_with("insertComposition") || kind.starts_with("deleteComposition") {
        // The composition so far: drawn inline at the caret, underlined.
        push(egui::Event::Ime(egui::ImeEvent::Preedit { text, active_range_chars: None }));
    } else if kind.starts_with("delete") {
        if text.is_empty() {
            // Backspace (Android reports keyCode 229, so the keydown does not delete) with
            // nothing composing: delete the character before the caret.
            match kind.as_str() {
                "deleteContentBackward" => push(egui::Event::Ime(egui::ImeEvent::DeleteSurrounding { before_chars: 1, after_chars: 0 })),
                "deleteContentForward" => push(egui::Event::Ime(egui::ImeEvent::DeleteSurrounding { before_chars: 0, after_chars: 1 })),
                _ => {}
            }
        } else {
            push(egui::Event::Ime(egui::ImeEvent::Preedit { text, active_range_chars: None }));
        }
    } else {
        // A typed or inserted character, or a finished word: it is text now.
        commit(input);
    }
}
