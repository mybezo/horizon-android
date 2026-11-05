//! MocLG's control layout and editor on the original Vasysik Android event loop.
use super::*;
use horizon_client::controls::{Claim, Controls, Event, Screen as ControlScreen};
use horizon_client::controls::{
    editor::{Editor, Outcome},
    menu::{Choice, Menu, Settings as MenuSettings},
    storage,
};
use std::{cell::RefCell, collections::HashMap, path::PathBuf};
use winit::event::{Touch, TouchPhase};
struct State {
    home: PathBuf,
    stamp: Option<std::time::SystemTime>,
    next_reload: Instant,
    controls: Controls,
    editor: Option<Editor>,
    menu: Menu,
    contacts: HashMap<u64, Claim>,
    menu_fingers: std::collections::HashSet<u64>,
    events: Vec<Event>,
    choices: Vec<Choice>,
    held_keys: std::collections::HashSet<KeyCode>,
    held_buttons: std::collections::HashSet<MouseButton>,
    opacity: f32,
    hidden: bool,
    virtual_mouse: bool,
    pointer_open: bool,
    size: Option<f32>,
    scrolling: Option<(f32, Instant)>,
}
impl State {
    fn new(home: PathBuf) -> Self {
        let controls = storage::load(&home)
            .map(Controls::new)
            .unwrap_or_else(|_| Controls::default_layout());
        let stamp = std::fs::metadata(storage::path(&home))
            .and_then(|m| m.modified())
            .ok();
        let display = std::fs::read(home.join("controls/display.json"))
            .ok()
            .and_then(|b| serde_json::from_slice::<serde_json::Value>(&b).ok())
            .unwrap_or_default();
        let opacity = display["opacity"]
            .as_f64()
            .map_or(1.0, |v| v.clamp(0.1, 1.0) as f32);
        let size = display["size"].as_f64().map(|v| v.clamp(0.5, 1.2) as f32);
        let hidden = display["hidden"].as_bool().unwrap_or(false);
        let ball = (
            display["ball"][0].as_f64().unwrap_or(0.95).clamp(0.0, 1.0) as f32,
            display["ball"][1].as_f64().unwrap_or(0.12).clamp(0.0, 1.0) as f32,
        );
        Self {
            home,
            stamp,
            next_reload: Instant::now(),
            controls,
            editor: None,
            menu: Menu::new(ball),
            contacts: HashMap::new(),
            menu_fingers: Default::default(),
            events: vec![],
            choices: vec![],
            held_keys: Default::default(),
            held_buttons: Default::default(),
            opacity,
            hidden,
            virtual_mouse: display["virtual_mouse"].as_bool().unwrap_or(true),
            pointer_open: false,
            size,
            scrolling: None,
        }
    }
    fn persist_display(&self) {
        let path = self.home.join("controls/display.json");
        let result = (|| -> std::io::Result<()> {
            std::fs::create_dir_all(path.parent().unwrap())?;
            let partial = path.with_extension("partial");
            std::fs::write(
                &partial,
                serde_json::to_vec(
                    &serde_json::json!({"opacity":self.opacity,"size":self.size,"hidden":self.hidden,"virtual_mouse":self.virtual_mouse,"ball":[self.menu.ball.0,self.menu.ball.1]}),
                )?,
            )?;
            std::fs::rename(partial, path)
        })();
        if let Err(error) = result {
            log::warn!("Controller display settings: {error}");
        }
    }
    fn release(&mut self) {
        self.controls.release_all();
        self.events.extend(self.controls.take_events());
        self.scrolling = None;
        self.contacts.clear();
        self.menu_fingers.clear();
        let open = self.menu.open;
        self.menu = Menu::new(self.menu.ball);
        self.menu.open = open;
    }
    fn reload(&mut self) {
        if Instant::now() < self.next_reload || self.editor.is_some() || !self.contacts.is_empty() {
            return;
        }
        self.next_reload = Instant::now() + Duration::from_millis(250);
        let stamp = std::fs::metadata(storage::path(&self.home))
            .and_then(|m| m.modified())
            .ok();
        if stamp != self.stamp && self.editor.is_none() && self.contacts.is_empty() {
            if let Ok(layout) = storage::load(&self.home) {
                self.release();
                self.controls = Controls::new(layout);
                self.stamp = stamp;
            }
        }
    }
}
thread_local! { static STATE: RefCell<Option<State>> = const { RefCell::new(None) }; }
fn with<R>(f: impl FnOnce(&mut State) -> R) -> R {
    STATE.with(|slot| {
        let home = std::env::current_dir().unwrap_or_default();
        let mut slot = slot.borrow_mut();
        if slot.as_ref().is_none_or(|s| s.home != home) {
            *slot = Some(State::new(home));
        }
        let state = slot.as_mut().unwrap();
        state.reload();
        f(state)
    })
}
pub(super) fn any_held_key(mut matches: impl FnMut(KeyCode) -> bool) -> bool {
    with(|s| s.held_keys.iter().copied().any(&mut matches))
}
pub(super) fn held_mouse(button: MouseButton) -> bool {
    with(|s| s.held_buttons.contains(&button))
}
pub(super) fn density(window: &Window) -> f32 {
    let [x, y] = winit::platform::android::native_input_scale();
    (window.scale_factor() * x.min(y)) as f32
}
fn pointer_blocked() -> bool {
    with(|s| s.editor.is_some() || s.menu.open)
}
pub(super) fn virtual_mouse() -> bool {
    with(|s| s.virtual_mouse)
}
fn enter_pointer(open: bool) -> bool {
    with(|s| {
        let entered = open && !s.pointer_open;
        s.pointer_open = open;
        entered
    })
}
pub(super) fn editing() -> bool {
    with(|s| s.editor.is_some())
}
pub(super) fn open_editor() {
    with(|s| {
        s.release();
        s.menu.open = false;
        if s.editor.is_none() {
            let controls = std::mem::replace(&mut s.controls, Controls::default_layout());
            s.editor = Some(Editor::new(controls));
        }
    });
}
pub(super) fn release() {
    with(State::release);
}
pub(super) fn back() -> bool {
    with(|s| {
        if let Some(editor) = &mut s.editor {
            if !editor.finish_typing() {
                s.editor = None;
                s.controls = storage::load(&s.home)
                    .map(Controls::new)
                    .unwrap_or_else(|_| Controls::default_layout());
            }
            crate::platform::hide();
            true
        } else if s.menu.open {
            s.menu.open = false;
            true
        } else {
            false
        }
    })
}
fn screen(size: [u32; 2], density: f32) -> ControlScreen {
    ControlScreen {
        width: size[0].max(1) as f32,
        height: size[1].max(1) as f32,
        density: density.max(0.5),
        dark: true,
    }
}
pub(super) fn touch(t: &Touch, size: [u32; 2], density: f32, playing: bool) -> bool {
    with(|s| {
        let screen = screen(size, density);
        let at = (t.location.x as f32, t.location.y as f32);
        if let Some(editor) = &mut s.editor {
            let outcome = match t.phase {
                TouchPhase::Started => {
                    editor.down(t.id, at, screen);
                    Outcome::None
                }
                TouchPhase::Moved => {
                    editor.moved(t.id, at, screen);
                    Outcome::None
                }
                TouchPhase::Ended => editor.up(t.id, at, screen),
                TouchPhase::Cancelled => Outcome::None,
            };
            match outcome {
                Outcome::Keyboard(true) => crate::platform::show(""),
                Outcome::Keyboard(false) => crate::platform::hide(),
                Outcome::Save => match storage::save(&s.home, editor.layout()) {
                    Ok(()) => {
                        s.controls = s.editor.take().unwrap().into_controls();
                        crate::platform::hide();
                    }
                    Err(error) => {
                        crate::documents::notice(format!("Controller save failed: {error}"), false)
                    }
                },
                Outcome::Exit => {
                    s.editor = None;
                    s.controls = storage::load(&s.home)
                        .map(Controls::new)
                        .unwrap_or_else(|_| Controls::default_layout());
                    crate::platform::hide();
                }
                Outcome::None => {}
            }
            return true;
        }
        if t.phase == TouchPhase::Started && s.menu.down(t.id, at, screen) {
            s.menu_fingers.insert(t.id);
            return true;
        }
        if s.menu_fingers.contains(&t.id) {
            match t.phase {
                TouchPhase::Moved => s.menu.moved(t.id, at, screen),
                TouchPhase::Ended => {
                    if let Some(choice) = s.menu.up(t.id, at) {
                        s.choices.push(choice);
                    }
                    s.menu_fingers.remove(&t.id);
                    if s.menu.open {
                        s.release();
                    }
                    s.persist_display();
                }
                TouchPhase::Cancelled => {
                    s.menu_fingers.remove(&t.id);
                }
                _ => {}
            }
            return true;
        }
        if s.menu.open {
            return true;
        }
        if s.hidden {
            return false;
        }
        let claim = match t.phase {
            TouchPhase::Started => {
                let c = s.controls.down(t.id, at, playing, screen);
                s.contacts.insert(t.id, c);
                c
            }
            TouchPhase::Moved => {
                s.controls.moved(t.id, at, playing, screen);
                *s.contacts.get(&t.id).unwrap_or(&Claim::None)
            }
            TouchPhase::Ended => {
                s.controls.up(t.id, screen);
                s.contacts.remove(&t.id).unwrap_or(Claim::None)
            }
            TouchPhase::Cancelled => {
                let c = s.contacts.get(&t.id).copied().unwrap_or(Claim::None);
                s.release();
                c
            }
        };
        s.events.extend(s.controls.take_events());
        claim == Claim::Taken
    })
}
pub(super) fn editor_key(event: &winit::event::KeyEvent) -> bool {
    if !editing() {
        return false;
    }
    if event.state == ElementState::Pressed {
        with(|s| {
            if let Some(editor) = &mut s.editor {
                match event.physical_key {
                    PhysicalKey::Code(KeyCode::Backspace) => editor.backspace(),
                    PhysicalKey::Code(KeyCode::Enter | KeyCode::NumpadEnter) => {
                        editor.finish_typing();
                        crate::platform::hide();
                    }
                    _ => {
                        if let Some(text) = &event.text
                            && !text.chars().any(char::is_control)
                        {
                            editor.type_text(text);
                        }
                    }
                }
            }
        });
    }
    true
}
pub(super) fn poll_editor() -> bool {
    if !editing() {
        return false;
    }
    with(|s| {
        if let Some(editor) = &mut s.editor {
            for output in crate::platform::poll() {
                match output {
                    horizon_client::osk::Output::Text(text) => editor.type_text(&text),
                    horizon_client::osk::Output::Backspace => editor.backspace(),
                    horizon_client::osk::Output::Enter | horizon_client::osk::Output::Close => {
                        editor.finish_typing();
                        crate::platform::hide();
                    }
                }
            }
        }
    });
    true
}
pub(super) fn draw(
    batch: &mut HudBatch,
    atlas: &horizon_client::hud::HudAtlas,
    density: f32,
    playing: bool,
    settings: &Settings,
) {
    with(|s| {
        let screen = screen(
            [
                (batch.width * batch.scale) as u32,
                (batch.height * batch.scale) as u32,
            ],
            density,
        );
        if let Some(editor) = &mut s.editor {
            editor.draw(batch, atlas, screen);
            return;
        }
        s.controls.language = settings.language.replace('_', "-");
        let fitted = s.controls.fitting_size(screen);
        s.controls.size = s.size.unwrap_or(fitted);
        if !s.hidden {
            s.controls.draw(batch, atlas, playing, screen, s.opacity);
        }
        s.menu.draw(
            batch,
            atlas,
            screen,
            MenuSettings {
                gestures: settings.touch.crosshair_gestures,
                gyroscope: settings.touch.gyroscope,
                opacity: s.opacity,
                controls_hidden: s.hidden,
                size: s.size,
                fitted,
                larger_interface: false,
                debug_scale: 0,
            },
        );
    });
}
fn pending() -> (Vec<Event>, Vec<Choice>) {
    with(|s| {
        (
            std::mem::take(&mut s.events),
            std::mem::take(&mut s.choices),
        )
    })
}
fn choice(choice: Choice) -> bool {
    let handled = match choice {
        Choice::EditControls => {
            open_editor();
            true
        }
        Choice::VirtualMouse => {
            with(|s| {
                s.virtual_mouse = !s.virtual_mouse;
            });
            true
        }
        Choice::ImportControls => {
            if let Err(e) =
                crate::documents::begin_zalith(std::env::current_dir().unwrap_or_default())
            {
                crate::documents::notice(e, false);
            }
            true
        }
        Choice::Opacity => {
            with(|s| {
                s.opacity = if s.opacity > 0.3 {
                    s.opacity - 0.25
                } else {
                    1.0
                }
            });
            true
        }
        Choice::HideControls => {
            with(|s| {
                s.hidden = !s.hidden;
                s.release();
            });
            true
        }
        Choice::Size => {
            with(|s| {
                s.controls.size = if s.controls.size > 0.65 {
                    s.controls.size - 0.1
                } else {
                    1.2
                }
            });
            true
        }
        Choice::Keyboard => {
            if crate::platform::active() {
                crate::platform::hide()
            } else {
                crate::platform::show("")
            };
            true
        }
        Choice::Close => {
            with(|s| s.menu.open = false);
            true
        }
        _ => false,
    };
    if handled {
        with(|s| s.persist_display());
    }
    handled
}
impl Game<'_> {
    pub(super) fn zalith_pointer(&mut self) {
        self.pad.touch_ui.virtual_mouse = virtual_mouse();
        if pointer_blocked() {
            self.pad.touch_ui.cancel_relative();
        }
        let show = self.pad_in_menu()
            && self.pad.touch_ui.virtual_mouse
            && crate::touch_visible()
            && !editing();
        if enter_pointer(show)
            && let Some(batch) = self.gui_scale()
        {
            self.cursor = (batch.width / 2.0, batch.height / 2.0);
        }
    }

    fn zalith_scroll(&mut self, events: &dyn super::event_loop::EventLoopCtl, lines: f32) {
        let id = self.window.as_ref().map_or(WindowId::dummy(), |w| w.id());
        self.handle_window_event(
            events,
            id,
            WindowEvent::MouseWheel {
                device_id: DeviceId::dummy(),
                delta: winit::event::MouseScrollDelta::LineDelta(0.0, lines),
                phase: TouchPhase::Moved,
            },
        );
    }

    pub(super) fn zalith_events(&mut self, events: &dyn super::event_loop::EventLoopCtl) {
        let (pending, choices) = pending();
        for control in pending {
            match control {
                Event::Key { name, pressed } | Event::Launcher { name, pressed }
                    if mouse_button(&name).is_some() =>
                {
                    let button = mouse_button(&name).unwrap();
                    if pressed {
                        self.pad.touch_ui.suppress_relative_click();
                    }
                    with(|s| {
                        if pressed {
                            s.held_buttons.insert(button);
                        } else {
                            s.held_buttons.remove(&button);
                        }
                    });
                    let id = self.window.as_ref().map_or(WindowId::dummy(), |w| w.id());
                    let result = if matches!(button, MouseButton::Left | MouseButton::Right) {
                        self.android_menu_mouse(events, id, button == MouseButton::Right, pressed)
                    } else if !self.pad_in_menu() {
                        self.play_mouse(button, pressed)
                    } else {
                        self.handle_window_event(
                            events,
                            id,
                            WindowEvent::MouseInput {
                                device_id: DeviceId::dummy(),
                                button,
                                state: if pressed {
                                    ElementState::Pressed
                                } else {
                                    ElementState::Released
                                },
                            },
                        );
                        Ok(())
                    };
                    if let Err(error) = result {
                        log::warn!("Zalith mouse: {error}");
                    }
                }
                Event::Key { name, pressed } => {
                    if let Some(key) = glfw_key(&name) {
                        with(|s| {
                            if pressed {
                                s.held_keys.insert(key);
                            } else {
                                s.held_keys.remove(&key);
                            }
                        });

                        if pressed {
                            self.pad.custom_keys.insert(key);
                        } else {
                            self.pad.custom_keys.remove(&key);
                        }
                        if self.pad.hardware_keys.contains(&key) {
                            continue;
                        }
                        if pressed {
                            self.physical_keys.insert(key);
                        } else {
                            self.physical_keys.remove(&key);
                        }
                        self.mod_key(key, pressed, false);
                        self.native_key_event(key, pressed, false);
                        let action = if pressed {
                            horizon_client::debug_keys::KeyAction::Press
                        } else {
                            horizon_client::debug_keys::KeyAction::Release
                        };
                        if self.debug_key(key, action) {
                            continue;
                        }
                        if key == KeyCode::Escape {
                            if pressed {
                                let _ = self.pad_back();
                            }
                        } else {
                            for mapped in self.settings.keys.translate_all(key) {
                                match mapped {
                                    Ok(code) => {
                                        let _ = self.mapped_key(code, pressed, false);
                                    }
                                    Err(button) => {
                                        if pressed {
                                            self.pad.touch_ui.suppress_relative_click();
                                        }
                                        let _ = self.android_custom_mouse(button, pressed);
                                    }
                                }
                            }
                        }
                        if pressed && self.screen == Screen::Chat {
                            self.android_show_keyboard();
                        }
                    }
                }
                Event::SendText(text) => {
                    if self.screen != Screen::Chat {
                        let _ = self.key_pressed(KeyCode::KeyT);
                    }
                    self.pad_osk_output(horizon_client::osk::Output::Text(text));
                    self.pad_osk_output(horizon_client::osk::Output::Enter);
                }
                Event::Launcher { name, pressed } => match name.as_str() {
                    "launcher.event.switch_ime" if pressed => {
                        if crate::platform::active() {
                            crate::platform::hide()
                        } else {
                            self.android_show_keyboard()
                        }
                    }
                    "launcher.event.switch_menu" if pressed => with(|s| {
                        s.menu.open = !s.menu.open;
                        if s.menu.open {
                            s.release();
                        }
                    }),
                    "launcher.event.scroll_up" | "launcher.event.scroll_down" => with(|s| {
                        s.scrolling = pressed.then(|| {
                            (
                                if name.ends_with("up") { 1.0 } else { -1.0 },
                                Instant::now(),
                            )
                        });
                    }),
                    "launcher.event.scroll_up.single" if pressed => self.zalith_scroll(events, 1.0),
                    "launcher.event.scroll_down.single" if pressed => {
                        self.zalith_scroll(events, -1.0)
                    }
                    _ => {}
                },
            }
        }
        let scroll = with(|s| {
            if let Some((lines, next)) = &mut s.scrolling
                && Instant::now() >= *next
            {
                *next = Instant::now() + Duration::from_millis(80);
                Some(*lines)
            } else {
                None
            }
        });
        if let Some(lines) = scroll {
            self.zalith_scroll(events, lines);
        }
        for c in choices {
            if c == Choice::EditControls && !self.pad_in_menu() {
                let _ = self.pad_back();
            }
            if choice(c) {
                continue;
            }
            match c {
                Choice::Gestures => {
                    self.settings.touch.crosshair_gestures = !self.settings.touch.crosshair_gestures
                }
                Choice::Gyroscope => self.settings.touch.gyroscope = !self.settings.touch.gyroscope,
                Choice::Pause => {
                    let _ = self.pad_back();
                }
                _ => {}
            }
            self.settings.save(&Self::options_path());
        }
        let _ = events;
    }
}

fn mouse_button(name: &str) -> Option<MouseButton> {
    Some(match name {
        "GLFW_MOUSE_BUTTON_LEFT" | "GLFW_MOUSE_BUTTON_1" => MouseButton::Left,
        "GLFW_MOUSE_BUTTON_RIGHT" | "GLFW_MOUSE_BUTTON_2" => MouseButton::Right,
        "GLFW_MOUSE_BUTTON_MIDDLE" | "GLFW_MOUSE_BUTTON_3" => MouseButton::Middle,
        "GLFW_MOUSE_BUTTON_4" => MouseButton::Back,
        "GLFW_MOUSE_BUTTON_5" => MouseButton::Forward,
        _ => return None,
    })
}

/// A GLFW key name (`GLFW_KEY_E`) as the key it names.
fn glfw_key(name: &str) -> Option<KeyCode> {
    use KeyCode as K;
    let key = name.strip_prefix("GLFW_KEY_")?;
    if key.len() == 1 {
        let c = key.as_bytes()[0];
        return Some(match c {
            b'A'..=b'Z' => [
                K::KeyA,
                K::KeyB,
                K::KeyC,
                K::KeyD,
                K::KeyE,
                K::KeyF,
                K::KeyG,
                K::KeyH,
                K::KeyI,
                K::KeyJ,
                K::KeyK,
                K::KeyL,
                K::KeyM,
                K::KeyN,
                K::KeyO,
                K::KeyP,
                K::KeyQ,
                K::KeyR,
                K::KeyS,
                K::KeyT,
                K::KeyU,
                K::KeyV,
                K::KeyW,
                K::KeyX,
                K::KeyY,
                K::KeyZ,
            ][usize::from(c - b'A')],
            b'0'..=b'9' => [
                K::Digit0,
                K::Digit1,
                K::Digit2,
                K::Digit3,
                K::Digit4,
                K::Digit5,
                K::Digit6,
                K::Digit7,
                K::Digit8,
                K::Digit9,
            ][usize::from(c - b'0')],
            _ => return None,
        });
    }
    if let Some(n) = key.strip_prefix('F').and_then(|n| n.parse::<usize>().ok()) {
        return [
            K::F1,
            K::F2,
            K::F3,
            K::F4,
            K::F5,
            K::F6,
            K::F7,
            K::F8,
            K::F9,
            K::F10,
            K::F11,
            K::F12,
            K::F13,
            K::F14,
            K::F15,
            K::F16,
            K::F17,
            K::F18,
            K::F19,
            K::F20,
            K::F21,
            K::F22,
            K::F23,
            K::F24,
            K::F25,
        ]
        .get(n.wrapping_sub(1))
        .copied();
    }
    if let Some(n) = key
        .strip_prefix("KP_")
        .and_then(|n| n.parse::<usize>().ok())
    {
        return [
            K::Numpad0,
            K::Numpad1,
            K::Numpad2,
            K::Numpad3,
            K::Numpad4,
            K::Numpad5,
            K::Numpad6,
            K::Numpad7,
            K::Numpad8,
            K::Numpad9,
        ]
        .get(n)
        .copied();
    }
    Some(match key {
        "SPACE" => K::Space,
        "APOSTROPHE" => K::Quote,
        "COMMA" => K::Comma,
        "MINUS" => K::Minus,
        "PERIOD" => K::Period,
        "SLASH" => K::Slash,
        "SEMICOLON" => K::Semicolon,
        "EQUAL" => K::Equal,
        "LEFT_BRACKET" => K::BracketLeft,
        "BACKSLASH" => K::Backslash,
        "RIGHT_BRACKET" => K::BracketRight,
        "GRAVE_ACCENT" => K::Backquote,
        "WORLD_1" | "WORLD_2" => K::IntlBackslash,
        "ESCAPE" => K::Escape,
        "ENTER" => K::Enter,
        "TAB" => K::Tab,
        "BACKSPACE" => K::Backspace,
        "INSERT" => K::Insert,
        "DELETE" => K::Delete,
        "RIGHT" => K::ArrowRight,
        "LEFT" => K::ArrowLeft,
        "DOWN" => K::ArrowDown,
        "UP" => K::ArrowUp,
        "PAGE_UP" => K::PageUp,
        "PAGE_DOWN" => K::PageDown,
        "HOME" => K::Home,
        "END" => K::End,
        "CAPS_LOCK" => K::CapsLock,
        "SCROLL_LOCK" => K::ScrollLock,
        "NUM_LOCK" => K::NumLock,
        "PRINT_SCREEN" => K::PrintScreen,
        "PAUSE" => K::Pause,
        "KP_DECIMAL" => K::NumpadDecimal,
        "KP_DIVIDE" => K::NumpadDivide,
        "KP_MULTIPLY" => K::NumpadMultiply,
        "KP_SUBTRACT" => K::NumpadSubtract,
        "KP_ADD" => K::NumpadAdd,
        "KP_ENTER" => K::NumpadEnter,
        "KP_EQUAL" => K::NumpadEqual,
        "LEFT_SHIFT" => K::ShiftLeft,
        "LEFT_CONTROL" => K::ControlLeft,
        "LEFT_ALT" => K::AltLeft,
        "LEFT_SUPER" => K::SuperLeft,
        "RIGHT_SHIFT" => K::ShiftRight,
        "RIGHT_CONTROL" => K::ControlRight,
        "RIGHT_ALT" => K::AltRight,
        "RIGHT_SUPER" => K::SuperRight,
        "MENU" => K::ContextMenu,
        _ => return None,
    })
}
