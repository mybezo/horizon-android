//! The game menu, after Zalith Launcher 2's floating ball: a ball over the
//! game that drags anywhere and, tapped, opens the menu (the layout editor,
//! gestures, the gyroscope, the controls' opacity, the keyboard).
use super::widgets::{self, Hits, MUTED, TEXT};
use super::*;
use crate::ui_theme::{Face, color::hex, mybezo};

/// The player's touch settings the menu changes.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Settings {
    /// A tap on the world uses, a hold attacks (Zalith's gesture control).
    pub gestures: bool,
    pub gyroscope: bool,
    pub virtual_mouse: bool,
    pub camera_sensitivity: f32,
    /// The controls' opacity, 0..1.
    pub opacity: f32,
    pub controls_hidden: bool,
    /// The controls' size; none fits the layout to the screen.
    pub size: Option<f32>,
    /// The size Auto picked, to show.
    pub fitted: f32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Choice {
    EditControls,
    ImportControls,
    VirtualMouse,
    Gestures,
    Gyroscope,
    Opacity(u8),
    CameraSensitivity(u16),
    HideControls,
    Size,
    Keyboard,
    Pause,
    Minimize,
    Close,
}

#[derive(Clone, Copy, PartialEq)]
enum Slider {
    Opacity,
    Camera,
}

impl Slider {
    fn value(self, fraction: f32) -> Choice {
        match self {
            Self::Opacity => Choice::Opacity((10.0 + fraction * 90.0).round() as u8),
            Self::Camera => Choice::CameraSensitivity((1.0 + fraction * 999.0).round() as u16),
        }
    }
}

struct Drag {
    finger: u64,
    kind: Slider,
    track: Rect,
    x: f32,
}

impl Drag {
    fn fraction(&self) -> f32 {
        ((self.x - self.track.x) / self.track.w).clamp(0.0, 1.0)
    }
}

pub struct Menu {
    /// The ball's centre as a share of the screen.
    pub ball: (f32, f32),
    pub open: bool,
    hits: Hits<Choice>,
    /// A finger on the ball: where it landed, and whether it dragged.
    on_ball: Option<(u64, (f32, f32), bool)>,
    pending: Option<(u64, Choice)>,
    sliders: Vec<(Slider, Rect)>,
    drag: Option<Drag>,
}

impl Menu {
    pub fn new(ball: (f32, f32)) -> Self {
        Self {
            ball,
            open: false,
            hits: Hits::new(),
            on_ball: None,
            pending: None,
            sliders: Vec::with_capacity(2),
            drag: None,
        }
    }

    fn ball_disc(&self, screen: Screen) -> (f32, f32, f32) {
        (
            self.ball.0 * screen.width,
            self.ball.1 * screen.height,
            20.0 * screen.density,
        )
    }

    /// Whether the menu takes a finger landing at `at`.
    pub fn down(&mut self, finger: u64, at: (f32, f32), screen: Screen) -> bool {
        if self.drag.is_some() || self.on_ball.is_some() || self.pending.is_some() {
            return true;
        }
        if self.open {
            if let Some(&(kind, track)) = self.sliders.iter().find(|(_, track)| track.contains(at))
            {
                self.drag = Some(Drag {
                    finger,
                    kind,
                    track,
                    x: at.0,
                });
                return true;
            }
            if let Some(choice) = self.hits.at(at) {
                self.pending = Some((finger, choice));
                return true;
            }
        }
        let (x, y, r) = self.ball_disc(screen);
        if (at.0 - x).hypot(at.1 - y) <= r * 1.3 {
            self.on_ball = Some((finger, at, false));
            return true;
        }
        self.open
    }

    pub fn moved(&mut self, finger: u64, at: (f32, f32), screen: Screen) {
        if let Some(drag) = &mut self.drag
            && drag.finger == finger
        {
            drag.x = at.0;
            return;
        }
        if let Some((f, start, dragged)) = &mut self.on_ball
            && *f == finger
        {
            if !*dragged && (at.0 - start.0).hypot(at.1 - start.1) > 8.0 * screen.density {
                *dragged = true;
            }
            if *dragged {
                self.ball = (
                    (at.0 / screen.width).clamp(0.02, 0.98),
                    (at.1 / screen.height).clamp(0.03, 0.97),
                );
            }
        }
    }

    /// The choice a lifted finger made, if any. A tap on the ball opens or closes the menu.
    pub fn up(&mut self, finger: u64, at: (f32, f32)) -> Option<Choice> {
        if self.drag.as_ref().is_some_and(|drag| drag.finger == finger) {
            let mut drag = self.drag.take().unwrap();
            drag.x = at.0;
            return Some(drag.kind.value(drag.fraction()));
        }
        if let Some((f, _, dragged)) = self.on_ball
            && f == finger
        {
            self.on_ball = None;
            if !dragged {
                self.open = !self.open;
            }
            return None;
        }
        let (f, choice) = self.pending?;
        if f != finger {
            return None;
        }
        self.pending = None;
        (self.hits.at(at) == Some(choice)).then_some(choice)
    }

    pub fn cancel(&mut self, finger: u64) {
        if self.holds(finger) {
            self.on_ball = None;
            self.pending = None;
            self.drag = None;
        }
    }

    pub fn holds(&self, finger: u64) -> bool {
        self.on_ball.is_some_and(|(f, _, _)| f == finger)
            || self.pending.is_some_and(|(f, _)| f == finger)
            || self.drag.as_ref().is_some_and(|drag| drag.finger == finger)
    }

    pub fn draw(
        &mut self,
        batch: &mut HudBatch,
        atlas: &HudAtlas,
        screen: Screen,
        settings: Settings,
    ) {
        let mut ui = Ui::new(atlas, batch, (0.0, 0.0));
        self.hits.clear();
        self.sliders.clear();
        let (x, y, r) = self.ball_disc(screen);
        let ball = Rect {
            x: x - r,
            y: y - r,
            w: 2.0 * r,
            h: 2.0 * r,
        };
        widgets::plate(&mut ui, ball, r * 0.36, mybezo::LIME);
        // The gear is drawn as geometry, so it works with every bundled font.
        for tooth in 0..8 {
            let a = tooth as f32 * std::f32::consts::FRAC_PI_4;
            let size = r * 0.23;
            widgets::plate(
                &mut ui,
                Rect {
                    x: x + a.cos() * r * 0.49 - size / 2.0,
                    y: y + a.sin() * r * 0.49 - size / 2.0,
                    w: size,
                    h: size,
                },
                size * 0.16,
                mybezo::INK,
            );
        }
        for (radius, color) in [(r * 0.46, mybezo::INK), (r * 0.20, mybezo::LIME)] {
            widgets::plate(
                &mut ui,
                Rect {
                    x: x - radius,
                    y: y - radius,
                    w: 2.0 * radius,
                    h: 2.0 * radius,
                },
                radius,
                color,
            );
        }
        if self.open {
            self.draw_panel(&mut ui, screen, settings);
        }
        ui.finish();
    }

    fn draw_panel(&mut self, ui: &mut Ui, screen: Screen, s: Settings) {
        let d = screen
            .density
            .min(screen.height / 352.0)
            .min(screen.width / 336.0);
        let text_scale = d / ui.density;
        let p = Rect {
            x: screen.width - 332.0 * d,
            y: (screen.height - 336.0 * d) * 0.5,
            w: 322.0 * d,
            h: 336.0 * d,
        };
        widgets::plate(ui, p, 11.0 * d, mybezo::PANEL);
        ui.border_px(
            p.x,
            p.y,
            p.x + p.w,
            p.y + p.h,
            [11.0 * d; 4],
            d.max(1.0),
            mybezo::BORDER,
        );
        let rect = |x: f32, y: f32, w: f32, h: f32| Rect {
            x: p.x + x * d,
            y: p.y + y * d,
            w: w * d,
            h: h * d,
        };
        widgets::text(
            ui,
            p.x + 14.0 * d,
            p.y + 10.0 * d,
            9.0 * text_scale,
            Face::SemiBold,
            mybezo::LIME,
            "MYBEZO EDITION",
        );
        widgets::text(
            ui,
            p.x + 14.0 * d,
            p.y + 23.0 * d,
            18.0 * text_scale,
            Face::SemiBold,
            TEXT,
            "Quick controls",
        );
        let close = rect(276.0, 8.0, 34.0, 34.0);
        self.action(ui, close, "×", Choice::Close, false, text_scale);
        for (index, (label, choice)) in [
            ("Edit controls", Choice::EditControls),
            ("Import layout", Choice::ImportControls),
        ]
        .into_iter()
        .enumerate()
        {
            self.action(
                ui,
                rect(12.0 + index as f32 * 152.0, 51.0, 146.0, 32.0),
                label,
                choice,
                index == 0,
                text_scale,
            );
        }
        for (index, (label, enabled, choice)) in [
            ("Virtual pointer", s.virtual_mouse, Choice::VirtualMouse),
            ("Show controls", !s.controls_hidden, Choice::HideControls),
            ("Gestures", s.gestures, Choice::Gestures),
            ("Gyroscope", s.gyroscope, Choice::Gyroscope),
        ]
        .into_iter()
        .enumerate()
        {
            let row = rect(12.0, 90.0 + index as f32 * 26.0, 298.0, 25.0);
            widgets::text(
                ui,
                row.x + 3.0 * d,
                row.y + 3.0 * d,
                12.0 * text_scale,
                Face::Medium,
                TEXT,
                label,
            );
            let x = row.x + row.w - 36.0 * d;
            let y = row.y + 4.0 * d;
            widgets::plate(
                ui,
                Rect {
                    x,
                    y,
                    w: 36.0 * d,
                    h: 16.0 * d,
                },
                8.0 * d,
                if enabled {
                    mybezo::LIME
                } else {
                    mybezo::RAISED
                },
            );
            widgets::plate(
                ui,
                Rect {
                    x: x + if enabled { 22.0 * d } else { 2.0 * d },
                    y: y + 2.0 * d,
                    w: 12.0 * d,
                    h: 12.0 * d,
                },
                6.0 * d,
                if enabled { mybezo::INK } else { MUTED },
            );
            self.hits.push(choice, row);
        }
        for (index, (kind, name, fraction)) in [
            (
                Slider::Opacity,
                "Controls opacity",
                (s.opacity * 100.0 - 10.0) / 90.0,
            ),
            (
                Slider::Camera,
                "Camera sensitivity",
                (s.camera_sensitivity * 100.0 - 1.0) / 999.0,
            ),
        ]
        .into_iter()
        .enumerate()
        {
            let row = rect(15.0, 195.0 + index as f32 * 34.0, 292.0, 33.0);
            let fraction = self
                .drag
                .as_ref()
                .filter(|drag| drag.kind == kind)
                .map_or(fraction.clamp(0.0, 1.0), Drag::fraction);
            let value = match kind.value(fraction) {
                Choice::Opacity(value) => format!("{value}%"),
                Choice::CameraSensitivity(value) => format!("{value}%"),
                _ => unreachable!(),
            };
            widgets::text(
                ui,
                row.x,
                row.y,
                11.0 * text_scale,
                Face::Medium,
                TEXT,
                name,
            );
            let value_w = widgets::text_width(ui, 11.0 * text_scale, Face::SemiBold, &value);
            widgets::text(
                ui,
                row.x + row.w - value_w,
                row.y,
                11.0 * text_scale,
                Face::SemiBold,
                mybezo::LIME,
                &value,
            );
            let y = row.y + 21.0 * d;
            widgets::plate(
                ui,
                Rect {
                    y,
                    h: 4.0 * d,
                    ..row
                },
                2.0 * d,
                mybezo::RAISED,
            );
            if fraction > 0.0 {
                widgets::plate(
                    ui,
                    Rect {
                        y,
                        w: row.w * fraction,
                        h: 4.0 * d,
                        ..row
                    },
                    2.0 * d,
                    mybezo::LIME,
                );
            }
            widgets::plate(
                ui,
                Rect {
                    x: row.x + row.w * fraction - 5.0 * d,
                    y: y - 3.0 * d,
                    w: 10.0 * d,
                    h: 10.0 * d,
                },
                5.0 * d,
                mybezo::LIME,
            );
            self.sliders.push((kind, row));
        }
        let size = match s.size {
            Some(size) => format!("Size {:.0}%", size * 100.0),
            None => format!("Size auto {:.0}%", s.fitted * 100.0),
        };
        for (i, (label, choice)) in [
            (size.as_str(), Choice::Size),
            ("Keyboard", Choice::Keyboard),
            ("Pause", Choice::Pause),
            ("Minimize", Choice::Minimize),
        ]
        .into_iter()
        .enumerate()
        {
            self.action(
                ui,
                rect(
                    12.0 + (i % 2) as f32 * 152.0,
                    264.0 + (i / 2) as f32 * 34.0,
                    146.0,
                    29.0,
                ),
                label,
                choice,
                false,
                text_scale,
            );
        }
    }

    fn action(
        &mut self,
        ui: &mut Ui,
        r: Rect,
        label: &str,
        choice: Choice,
        primary: bool,
        scale: f32,
    ) {
        widgets::plate(
            ui,
            r,
            r.h * 0.20,
            if primary {
                mybezo::LIME
            } else {
                hex(0x26372a, 255)
            },
        );
        widgets::centred(
            ui,
            r,
            12.0 * scale,
            Face::SemiBold,
            if primary { mybezo::INK } else { TEXT },
            label,
        );
        self.hits.push(choice, r);
    }
}
