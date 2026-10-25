//! Headless UI screenshots: renders the whole WordCraft window offscreen (wgpu), so it works
//! with a locked screen or a hidden window.
//!
//! `cargo run -p wordcraft-ui-egui --example ui_shot -- script.jsonl`
//!
//! The script is JSON lines: control-channel requests (`{"method": …, "params": …}`, see
//! `docs/control-protocol.md`), `{"shot": "/abs/out.png"}` to save the window as PNG, or
//! `{"steps": n}` to run extra frames, or `{"wait": ms}` to keep running frames for a while (for
//! background work such as a Zotero session). The window is 1440×900 pt at 2× and opens the sample
//! document unless the first line is `{"empty": true}`; `{"tap": [x, y]}` taps a phone point; `{"empty": true, "startScreen": true}`
//! starts the way the desktop app does with nothing to open (File › Home, #766).

use wordcraft_engine::Session;
use wordcraft_ui_egui::{ControlRequest, Services, WordApp};

static READY: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
/// The phone tab sheet to open on the first frame (`{"phoneSheet": tab}`), or -1.
static SHEET_TAB: std::sync::atomic::AtomicI32 = std::sync::atomic::AtomicI32::new(-1);

fn main() {
    let path = std::env::args().nth(1).expect("usage: ui_shot script.jsonl");
    let script = std::fs::read_to_string(&path).expect("read script");
    let (tx, rx) = std::sync::mpsc::channel::<ControlRequest>();
    let lines: Vec<serde_json::Value> =
        script.lines().filter(|l| !l.trim().is_empty()).map(|l| serde_json::from_str(l).expect("json line")).collect();
    let doc = if lines.first().is_some_and(|l| l.get("empty").is_some()) {
        wordcraft_doc::Document::new()
    } else {
        wordcraft_engine::sample::sample_document()
    };
    let mut app = WordApp::new(Session::new(doc), Services::default()).with_control(rx);
    app.integrated_titlebar = true;
    if lines.first().is_some_and(|l| l.get("startScreen").is_some()) {
        app.start_screen();
    }
    let w = lines.iter().find_map(|l| l.get("width").and_then(|v| v.as_f64())).unwrap_or(1440.0) as f32;
    let h = lines.iter().find_map(|l| l.get("height").and_then(|v| v.as_f64())).unwrap_or(900.0) as f32;
    // `{"scale": 2.625}` on the first line sets the pixels per point (a phone at 412x915 pt).
    let scale = lines.iter().find_map(|l| l.get("scale").and_then(|v| v.as_f64())).unwrap_or(2.0) as f32;
    let mut harness =
        egui_kittest::Harness::builder().with_size(egui::vec2(w, h)).with_pixels_per_point(scale).with_max_steps(1_000_000).wgpu().build_ui_state(
            |ui, app: &mut WordApp| {
                if !READY.load(std::sync::atomic::Ordering::Relaxed) {
                    return;
                }
                let ctx = ui.ctx().clone();
                // `{"phoneSheet": tab}` opens the phone's tab sheet once, on the first frame.
                let tab = SHEET_TAB.swap(-1, std::sync::atomic::Ordering::Relaxed);
                if tab >= 0 {
                    app.open_phone_sheet(&ctx, tab as usize);
                }
                app.logic(&ctx);
                app.ui(ui);
            },
            app,
        );
    harness.input_mut().max_texture_side = Some(8192);
    if let Some(t) = lines.iter().find_map(|l| l.get("phoneSheet").and_then(|v| v.as_u64())) {
        SHEET_TAB.store(t as i32, std::sync::atomic::Ordering::Relaxed);
    }
    READY.store(true, std::sync::atomic::Ordering::Relaxed);
    step_n(&mut harness, 6);
    for l in lines {
        if let Some(p) = l.get("shot").and_then(|v| v.as_str()) {
            step_n(&mut harness, 4);
            match harness.render() {
                Ok(img) => {
                    img.save(p).expect("save png");
                    println!("{p}");
                }
                Err(e) => eprintln!("render failed: {e}"),
            }
        } else if let Some(n) = l.get("steps").and_then(|v| v.as_u64()) {
            step_n(&mut harness, n as usize);
        } else if let Some(ms) = l.get("wait").and_then(|v| v.as_u64()) {
            let end = std::time::Instant::now() + std::time::Duration::from_millis(ms);
            while std::time::Instant::now() < end {
                step(&mut harness);
                std::thread::sleep(std::time::Duration::from_millis(16));
            }
        } else if let Some(p) = l.get("tap").and_then(|v| v.as_array()) {
            // A phone tap at [x, y] points: pressed for a few frames, then released.
            let at = egui::pos2(p.first().and_then(|v| v.as_f64()).unwrap_or(0.0) as f32, p.get(1).and_then(|v| v.as_f64()).unwrap_or(0.0) as f32);
            let button =
                |pressed| egui::Event::PointerButton { pos: at, button: egui::PointerButton::Primary, pressed, modifiers: Default::default() };
            harness.input_mut().events.push(egui::Event::PointerMoved(at));
            step(&mut harness);
            harness.input_mut().events.push(button(true));
            step_n(&mut harness, 3);
            harness.input_mut().events.push(button(false));
            step_n(&mut harness, 6);
        } else if let Some(p) = l.get("longPress").and_then(|v| v.as_array()) {
            // A phone long press at [x, y] points: the press is held about 0.7 s, then released.
            let at = egui::pos2(p.first().and_then(|v| v.as_f64()).unwrap_or(0.0) as f32, p.get(1).and_then(|v| v.as_f64()).unwrap_or(0.0) as f32);
            let button =
                |pressed| egui::Event::PointerButton { pos: at, button: egui::PointerButton::Primary, pressed, modifiers: Default::default() };
            harness.input_mut().events.push(egui::Event::PointerMoved(at));
            step(&mut harness);
            harness.input_mut().events.push(button(true));
            step_n(&mut harness, 40);
            harness.input_mut().events.push(button(false));
            step_n(&mut harness, 4);
        } else if let Some(m) = l.get("method").and_then(|v| v.as_str()) {
            let (req, reply) = ControlRequest::new(m, l.get("params").cloned().unwrap_or_default());
            tx.send(req).expect("send");
            for _ in 0..60 {
                step(&mut harness);
                if let Ok(r) = reply.try_recv() {
                    let s = r.to_string();
                    // `UI_SHOT_FULL=1` prints whole replies (e.g. `document.inspect` for scripted checks).
                    let cut = if std::env::var_os("UI_SHOT_FULL").is_some() { s.len() } else { s.len().min(300) };
                    println!("{m}: {}", s.get(..cut).unwrap_or(&s));
                    break;
                }
            }
            step_n(&mut harness, 2);
        }
    }
}

fn step(harness: &mut egui_kittest::Harness<'_, WordApp>) {
    let mut raw = std::mem::take(harness.input_mut());
    harness.state_mut().raw_input_hook(&mut raw);
    *harness.input_mut() = raw;
    harness.step();
}

fn step_n(harness: &mut egui_kittest::Harness<'_, WordApp>, n: usize) {
    for _ in 0..n {
        step(harness);
    }
}
