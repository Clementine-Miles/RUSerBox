//! The Android activity: GameActivity through android-activity.
//!
//! - **Lifecycle**: the Vulkan surface is created on `InitWindow` and dropped
//!   on `TerminateWindow` (pause, rotation, process going to the back); the
//!   device and the app state outlive it. `LowMemory` trims GPU caches.
//! - **Frames** are drawn only when egui asks (input, a background task's
//!   repaint request, an animation), paced by FIFO presentation; between them
//!   the thread sleeps in `poll_events`.
//! - **Input**: touch and mouse from `MotionEvent`, hardware keys from
//!   `KeyEvent` (characters through the device's key map), and the soft
//!   keyboard through GameActivity's `GameTextInput` (`TextEvent`), which
//!   carries IME composition: the composing region is egui's preedit, and
//!   committed text is sent once and cleared from the input state.
//! - **Platform services** (clipboard, file picker, saving, the Keystore,
//!   the session service, widgets) are `MainActivity` methods ([`super::jni`]).

use std::sync::{Arc, Mutex, OnceLock};
use std::time::{Duration, Instant};

use android_activity::input::{InputEvent, KeyAction, KeyMapChar, Keycode, MotionAction, TextInputState, TextSpan, ToolType};
use android_activity::{AndroidApp, AndroidAppWaker, InputStatus, MainEvent, PollEvent};
use egui::{Event, Pos2, RawInput, Vec2};
use raw_window_handle::{AndroidDisplayHandle, AndroidNdkWindowHandle, RawDisplayHandle, RawWindowHandle};

use super::jni::Activity;
use super::keyfile::{self, Sealer};
use crate::core::lock;
use crate::gfx::{Gpu, GfxError};
use crate::ui::{App, Inbox, Pick, Picked, Platform};

/// Pick requests in flight, by the request code handed to Java.
static PICKS: Mutex<Vec<(i32, Pick)>> = Mutex::new(Vec::new());
static INBOX: OnceLock<Inbox> = OnceLock::new();
static WAKER: OnceLock<AndroidAppWaker> = OnceLock::new();

pub(super) fn file_picked(request: i32, name: String, data: Vec<u8>) {
    let purpose = {
        let mut p = lock(&PICKS);
        let i = p.iter().position(|(r, _)| *r == request);
        i.map(|i| p.remove(i).1)
    };
    if let (Some(purpose), Some(inbox)) = (purpose, INBOX.get()) {
        lock(inbox).push_back(Picked { purpose, name, data });
        if let Some(w) = WAKER.get() {
            w.wake();
        }
    }
}

struct Android {
    activity: Activity,
    next_request: std::cell::Cell<i32>,
    alive: std::cell::Cell<bool>,
}

impl Platform for Android {
    fn set_clipboard(&self, text: &str) {
        self.activity.set_clipboard(text);
    }
    fn clipboard(&self) -> Option<String> {
        self.activity.clipboard()
    }
    fn pick_file(&self, purpose: Pick) {
        let r = self.next_request.get() + 1;
        self.next_request.set(r);
        lock(&PICKS).push((r, purpose));
        self.activity.pick_file(r);
    }
    fn save_file(&self, name: &str, data: Vec<u8>) {
        self.activity.save_file(name, &data);
    }
    fn keep_alive(&self, on: bool) {
        if self.alive.get() != on {
            self.alive.set(on);
            self.activity.keep_alive(on);
        }
    }
    fn system_dark(&self) -> bool {
        self.activity.is_dark()
    }
    fn publish_widgets(&self, payload: &str) {
        self.activity.publish_widgets(payload);
    }
}

/// The store key sealed by the Android Keystore.
struct Keystore<'a>(&'a Activity);

impl Sealer for Keystore<'_> {
    fn seal(&self, key: &[u8; 32]) -> Result<Vec<u8>, String> {
        self.0.seal(key).ok_or_else(|| "the Android Keystore refused to seal the key".into())
    }
    fn open(&self, sealed: &[u8]) -> Result<[u8; 32], String> {
        self.0.open(sealed).and_then(|v| v.try_into().ok()).ok_or_else(|| "the Android Keystore could not open the key".into())
    }
    fn legacy(&self) -> Option<[u8; 32]> {
        self.0.legacy_key().and_then(|v| v.try_into().ok())
    }
}

fn surface_target(window: &ndk::native_window::NativeWindow) -> wgpu::SurfaceTargetUnsafe {
    let handle = AndroidNdkWindowHandle::new(std::ptr::NonNull::new(window.ptr().as_ptr().cast()).expect("window pointer"));
    wgpu::SurfaceTargetUnsafe::RawHandle { raw_display_handle: Some(RawDisplayHandle::Android(AndroidDisplayHandle::new())), raw_window_handle: RawWindowHandle::AndroidNdk(handle) }
}

/// What the app shows when it cannot start (no Vulkan, an unreadable store).
enum Screen {
    App(Box<App>),
    Error(String),
}

struct Input {
    events: Vec<Event>,
    /// The pointer egui is following, and where it is.
    primary: Option<i32>,
    pos: Pos2,
    /// IME text already handed to egui, so a repeated state adds nothing.
    committed: String,
    back: bool,
    combining: Option<char>,
}

pub fn main(app: AndroidApp) {
    android_logger::init_once(android_logger::Config::default().with_max_level(log::LevelFilter::Info).with_tag("RUSerBox"));
    let _ = WAKER.set(app.create_waker());
    // SAFETY: android-activity's VM and activity global reference.
    let activity = unsafe { Activity::new(app.vm_as_ptr(), app.activity_as_ptr()) };
    if let Some(tag) = activity.locale() {
        crate::i18n::set_locale(&tag);
    }
    let ctx = egui::Context::default();
    let waker = app.create_waker();
    ctx.set_request_repaint_callback(move |_| waker.wake());

    let screen = (|| -> Result<Screen, String> {
        let dir = activity.store_dir().map(std::path::PathBuf::from).or_else(|| app.internal_data_path()).ok_or("no app data directory")?;
        std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
        let key = keyfile::load(&dir, &Keystore(&activity)).map_err(|e| e.to_string())?;
        let store = sbm_store::Store::open(&dir, &key).map_err(|e| e.to_string())?;
        let app = App::new(Arc::new(store), ctx.clone());
        let _ = INBOX.set(app.inbox.clone());
        Ok(Screen::App(Box::new(app)))
    })();
    let mut screen = screen.unwrap_or_else(Screen::Error);
    let platform = Android { activity, next_request: std::cell::Cell::new(0), alive: std::cell::Cell::new(false) };

    let mut gpu: Option<Gpu> = None;
    let mut input = Input { events: vec![], primary: None, pos: Pos2::ZERO, committed: String::new(), back: false, combining: None };
    let mut ime_shown = false;
    let mut redraw_at: Option<Instant> = Some(Instant::now());
    let mut quit = false;
    let started = Instant::now();

    while !quit {
        let timeout = redraw_at.map(|t| t.saturating_duration_since(Instant::now()));
        app.poll_events(timeout, |event| match event {
            PollEvent::Main(MainEvent::InitWindow { .. }) => {
                if let Some(w) = app.native_window() {
                    let (wd, ht) = (w.width() as u32, w.height() as u32);
                    let target = surface_target(&w);
                    match &mut gpu {
                        // SAFETY: the window lives until TerminateWindow, where the surface goes.
                        Some(g) => {
                            if let Err(e) = unsafe { g.attach(target, wd, ht) } {
                                screen = Screen::Error(e.to_string());
                            }
                        }
                        None => match crate::gfx::block_on(Gpu::new(Some(target))) {
                            Ok(mut g) => {
                                g.resize(wd, ht);
                                gpu = Some(g);
                            }
                            Err(GfxError::NoVulkan) => screen = Screen::Error(GfxError::NoVulkan.to_string()),
                            Err(e) => screen = Screen::Error(e.to_string()),
                        },
                    }
                }
                redraw_at = Some(Instant::now());
            }
            PollEvent::Main(MainEvent::TerminateWindow { .. }) => {
                if let Some(g) = &mut gpu {
                    g.detach();
                }
            }
            PollEvent::Main(MainEvent::WindowResized { .. } | MainEvent::ContentRectChanged { .. } | MainEvent::ConfigChanged { .. } | MainEvent::RedrawNeeded { .. } | MainEvent::InsetsChanged { .. }) => {
                if let (Some(g), Some(w)) = (&mut gpu, app.native_window()) {
                    g.resize(w.width() as u32, w.height() as u32);
                }
                redraw_at = Some(Instant::now());
            }
            PollEvent::Main(MainEvent::LowMemory) => {
                if let Some(g) = &mut gpu {
                    g.trim();
                }
            }
            PollEvent::Main(MainEvent::Resume { .. } | MainEvent::GainedFocus) => redraw_at = Some(Instant::now()),
            PollEvent::Main(MainEvent::Destroy) => quit = true,
            PollEvent::Wake | PollEvent::Main(MainEvent::InputAvailable) => redraw_at = Some(Instant::now()),
            _ => {}
        });
        // Input since the last poll.
        if let Ok(mut iter) = app.input_events_iter() {
            loop {
                let more = iter.next(|ev| handle_input(&app, ev, &mut input));
                if !more {
                    break;
                }
            }
        }
        if !input.events.is_empty() || input.back {
            redraw_at = Some(Instant::now());
        }
        if input.back {
            input.back = false;
            if let Screen::App(a) = &mut screen
                && !a.back()
            {
                platform.activity.move_to_back();
            }
        }
        let due = redraw_at.is_some_and(|t| t <= Instant::now());
        let Some(g) = gpu.as_mut().filter(|g| g.has_surface()) else {
            redraw_at = None;
            continue;
        };
        if !due {
            continue;
        }
        let Some(w) = app.native_window() else { continue };
        let density = app.config().density().unwrap_or(160) as f32;
        let ppp = (density / 160.0).max(0.75);
        let size = Vec2::new(w.width() as f32, w.height() as f32) / ppp;
        let [l, t, r, b] = platform.activity.insets();
        let raw = RawInput {
            screen_rect: Some(egui::Rect::from_min_size(Pos2::ZERO, size)),
            safe_area_insets: Some(egui::SafeAreaInsets(egui::epaint::MarginF32 { left: l as f32 / ppp, top: t as f32 / ppp, right: r as f32 / ppp, bottom: b as f32 / ppp })),
            viewports: std::iter::once((egui::ViewportId::ROOT, egui::ViewportInfo { native_pixels_per_point: Some(ppp), focused: Some(true), ..Default::default() })).collect(),
            events: std::mem::take(&mut input.events),
            time: Some(started.elapsed().as_secs_f64()),
            focused: true,
            ..Default::default()
        };
        let out = ctx.run_ui(raw, |ui| match &mut screen {
            Screen::App(a) => a.update(ui, &platform),
            Screen::Error(msg) => {
                egui::CentralPanel::default().show(ui, |ui| {
                    ui.add_space(40.0);
                    ui.heading("RUSerBox");
                    ui.label(msg.as_str());
                });
            }
        });
        // The soft keyboard follows whether a text widget wants input.
        let wants_ime = out.platform_output.ime.is_some();
        if wants_ime != ime_shown {
            ime_shown = wants_ime;
            if wants_ime {
                app.show_soft_input(true);
            } else {
                app.hide_soft_input(true);
                input.committed.clear();
                app.set_text_input_state(TextInputState { text: String::new(), selection: TextSpan { start: 0, end: 0 }, compose_region: None });
            }
        }
        for cmd in &out.platform_output.commands {
            if let egui::OutputCommand::CopyText(text) = cmd {
                platform.set_clipboard(text);
            }
        }
        let delay = out.viewport_output.get(&egui::ViewportId::ROOT).map(|v| v.repaint_delay).unwrap_or(Duration::MAX);
        let ppp_out = out.pixels_per_point;
        g.render(&ctx, out, ppp_out);
        redraw_at = if delay.is_zero() { Some(Instant::now() + Duration::from_millis(16)) } else if delay < Duration::from_secs(3600) { Some(Instant::now() + delay) } else { None };
    }
}

fn key_of(k: Keycode) -> Option<egui::Key> {
    use egui::Key as K;
    Some(match k {
        Keycode::Enter | Keycode::NumpadEnter => K::Enter,
        Keycode::Del => K::Backspace,
        Keycode::ForwardDel => K::Delete,
        Keycode::Tab => K::Tab,
        Keycode::Escape => K::Escape,
        Keycode::DpadUp => K::ArrowUp,
        Keycode::DpadDown => K::ArrowDown,
        Keycode::DpadLeft => K::ArrowLeft,
        Keycode::DpadRight => K::ArrowRight,
        Keycode::MoveHome => K::Home,
        Keycode::MoveEnd => K::End,
        Keycode::PageUp => K::PageUp,
        Keycode::PageDown => K::PageDown,
        Keycode::Insert => K::Insert,
        Keycode::F1 => K::F1,
        Keycode::F2 => K::F2,
        Keycode::F3 => K::F3,
        Keycode::F4 => K::F4,
        Keycode::F5 => K::F5,
        Keycode::F6 => K::F6,
        Keycode::F7 => K::F7,
        Keycode::F8 => K::F8,
        Keycode::F9 => K::F9,
        Keycode::F10 => K::F10,
        Keycode::F11 => K::F11,
        Keycode::F12 => K::F12,
        Keycode::A => K::A,
        Keycode::B => K::B,
        Keycode::C => K::C,
        Keycode::D => K::D,
        Keycode::E => K::E,
        Keycode::F => K::F,
        Keycode::G => K::G,
        Keycode::H => K::H,
        Keycode::I => K::I,
        Keycode::J => K::J,
        Keycode::K => K::K,
        Keycode::L => K::L,
        Keycode::M => K::M,
        Keycode::N => K::N,
        Keycode::O => K::O,
        Keycode::P => K::P,
        Keycode::Q => K::Q,
        Keycode::R => K::R,
        Keycode::S => K::S,
        Keycode::T => K::T,
        Keycode::U => K::U,
        Keycode::V => K::V,
        Keycode::W => K::W,
        Keycode::X => K::X,
        Keycode::Y => K::Y,
        Keycode::Z => K::Z,
        _ => return None,
    })
}

fn handle_input(app: &AndroidApp, ev: &InputEvent, inp: &mut Input) -> InputStatus {
    let density = app.config().density().unwrap_or(160) as f32;
    let ppp = (density / 160.0).max(0.75);
    match ev {
        InputEvent::MotionEvent(m) => {
            let action = m.action();
            let idx = m.pointer_index();
            let mouse = m.pointers().next().is_some_and(|p| p.tool_type() == ToolType::Mouse);
            if action == MotionAction::Scroll {
                let p = m.pointer_at_index(idx);
                let dy = p.axis_value(android_activity::input::Axis::Vscroll);
                let dx = p.axis_value(android_activity::input::Axis::Hscroll);
                inp.events.push(Event::MouseWheel { unit: egui::MouseWheelUnit::Line, delta: Vec2::new(dx, dy), phase: egui::TouchPhase::Move, modifiers: egui::Modifiers::NONE, source: egui::MouseWheelSource::Wheel });
                return InputStatus::Handled;
            }
            for p in m.pointers() {
                let pos = Pos2::new(p.x() / ppp, p.y() / ppp);
                let id = p.pointer_id();
                let phase = match action {
                    MotionAction::Down | MotionAction::PointerDown if p.pointer_index() == idx => egui::TouchPhase::Start,
                    MotionAction::Up | MotionAction::PointerUp if p.pointer_index() == idx => egui::TouchPhase::End,
                    MotionAction::Cancel => egui::TouchPhase::Cancel,
                    MotionAction::Move | MotionAction::HoverMove => egui::TouchPhase::Move,
                    _ => continue,
                };
                if !mouse {
                    inp.events.push(Event::Touch { device_id: egui::TouchDeviceId(0), id: egui::TouchId(id as u64), phase, pos, force: Some(p.axis_value(android_activity::input::Axis::Pressure)) });
                }
                // The first finger (or the mouse) also drives egui's pointer.
                if inp.primary.is_none() && phase == egui::TouchPhase::Start {
                    inp.primary = Some(id);
                }
                if inp.primary == Some(id) || mouse && action == MotionAction::HoverMove {
                    inp.pos = pos;
                    match phase {
                        egui::TouchPhase::Start => {
                            inp.events.push(Event::PointerMoved(pos));
                            let button = if mouse && m.button_state().0 & 2 != 0 { egui::PointerButton::Secondary } else { egui::PointerButton::Primary };
                            inp.events.push(Event::PointerButton { pos, button, pressed: true, modifiers: egui::Modifiers::NONE });
                        }
                        egui::TouchPhase::Move => inp.events.push(Event::PointerMoved(pos)),
                        egui::TouchPhase::End | egui::TouchPhase::Cancel => {
                            inp.events.push(Event::PointerButton { pos, button: egui::PointerButton::Primary, pressed: false, modifiers: egui::Modifiers::NONE });
                            if !mouse {
                                inp.events.push(Event::PointerGone);
                            }
                            inp.primary = None;
                        }
                    }
                }
            }
            InputStatus::Handled
        }
        InputEvent::KeyEvent(k) => {
            let pressed = k.action() == KeyAction::Down;
            let meta = k.meta_state();
            let modifiers = egui::Modifiers { alt: meta.alt_on(), ctrl: meta.ctrl_on(), shift: meta.shift_on(), mac_cmd: false, command: meta.ctrl_on() };
            if k.key_code() == Keycode::Back {
                if !pressed {
                    inp.back = true;
                }
                return InputStatus::Handled;
            }
            if let Some(key) = key_of(k.key_code()) {
                inp.events.push(Event::Key { key, physical_key: None, pressed, repeat: false, modifiers });
            }
            // Characters from a hardware keyboard, through its key map.
            if pressed
                && !modifiers.ctrl
                && let Ok(map) = app.device_key_character_map(k.device_id())
            {
                match map.get(k.key_code(), meta) {
                    Ok(KeyMapChar::Unicode(c)) if !c.is_control() => {
                        let c = match inp.combining.take() {
                            Some(accent) => map.get_dead_char(accent, c).ok().flatten().unwrap_or(c),
                            None => c,
                        };
                        inp.events.push(Event::Text(c.to_string()));
                    }
                    Ok(KeyMapChar::CombiningAccent(a)) => inp.combining = Some(a),
                    _ => {}
                }
            }
            InputStatus::Handled
        }
        InputEvent::TextEvent(state) => {
            // GameTextInput's whole buffer. The composing region is preedit;
            // anything outside it is committed, sent once, then cleared.
            match state.compose_region {
                Some(r) if r.end > r.start => {
                    let pre: String = state.text.chars().skip(r.start).take(r.end - r.start).collect();
                    let before: String = state.text.chars().take(r.start).collect();
                    if before.len() > inp.committed.len() && before.starts_with(&inp.committed) {
                        inp.events.push(Event::Ime(egui::ImeEvent::Commit(before[inp.committed.len()..].to_owned())));
                        inp.committed = before;
                    }
                    inp.events.push(Event::Ime(egui::ImeEvent::Preedit { text: pre, active_range_chars: None }));
                }
                _ => {
                    let text = state.text.clone();
                    if text.starts_with(&inp.committed) && text.len() > inp.committed.len() {
                        inp.events.push(Event::Ime(egui::ImeEvent::Commit(text[inp.committed.len()..].to_owned())));
                    } else {
                        inp.events.push(Event::Ime(egui::ImeEvent::Preedit { text: String::new(), active_range_chars: None }));
                    }
                    inp.committed.clear();
                    app.set_text_input_state(TextInputState { text: String::new(), selection: TextSpan { start: 0, end: 0 }, compose_region: None });
                }
            }
            InputStatus::Handled
        }
        InputEvent::TextAction(_) => {
            inp.events.push(Event::Key { key: egui::Key::Enter, physical_key: None, pressed: true, repeat: false, modifiers: egui::Modifiers::NONE });
            inp.events.push(Event::Key { key: egui::Key::Enter, physical_key: None, pressed: false, repeat: false, modifiers: egui::Modifiers::NONE });
            InputStatus::Handled
        }
        _ => InputStatus::Unhandled,
    }
}
