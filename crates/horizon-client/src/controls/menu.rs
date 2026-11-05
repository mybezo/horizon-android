//! The game menu, after Zalith Launcher 2's floating ball: a ball over the
//! game that drags anywhere and, tapped, opens the menu (the layout editor,
//! gestures, the gyroscope, the controls' opacity, the keyboard).
use super::widgets::{self, Hits, MUTED, TEXT};
use super::*;
use crate::ui_theme::{Face, color::with_alpha, mybezo};

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
    pub gear_opacity: f32,
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
    GearOpacity(u8),
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
    GearOpacity,
    Camera,
}

impl Slider {
    fn value(self, fraction: f32) -> Choice {
        match self {
            Self::Opacity => Choice::Opacity((10.0 + fraction * 90.0).round() as u8),
            Self::GearOpacity => Choice::GearOpacity((10.0 + fraction * 90.0).round() as u8),
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
            sliders: Vec::with_capacity(3),
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

    /// Apply the thumb position while dragging. Cancel drops the preview;
    /// lifting a finger commits the same value through Choice.
    pub fn preview_controls_opacity(&self, opacity: f32) -> f32 {
        self.opacity_preview(Slider::Opacity, opacity)
    }

    fn opacity_preview(&self, kind: Slider, opacity: f32) -> f32 {
        self.drag
            .as_ref()
            .filter(|drag| drag.kind == kind)
            .map_or(opacity, |drag| {
                (10.0 + drag.fraction() * 90.0).round() / 100.0
            })
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
        let opacity = self.opacity_preview(Slider::GearOpacity, settings.gear_opacity);
        Self::gear(&mut ui, x, y, r, opacity);
        if self.open {
            self.draw_panel(&mut ui, screen, settings);
        }
        ui.finish();
    }

    fn gear(ui: &mut Ui, x: f32, y: f32, r: f32, opacity: f32) {
        let gear_color = with_alpha(mybezo::ACCENT, opacity);
        let scale = ui.batch.scale;
        let atlas = ui.atlas;
        // A hollow gear drawn as one ring of quads. Its centre and surrounding
        // area are transparent, while ball_disc keeps a generous touch target.
        let ring_point = |i: usize, radius: f32| {
            let angle = i as f32 * std::f32::consts::TAU / 32.0;
            [
                (x + angle.cos() * radius) / scale,
                (y + angle.sin() * radius) / scale,
            ]
        };
        let tooth = |i: usize| r * if matches!(i % 4, 1 | 2) { 0.50 } else { 0.38 };
        for i in 0..32 {
            let next = (i + 1) % 32;
            ui.batch.fill_quad(
                atlas,
                [
                    ring_point(i, r * 0.19),
                    ring_point(i, tooth(i)),
                    ring_point(next, tooth(next)),
                    ring_point(next, r * 0.19),
                ],
                gear_color,
            );
        }
    }

    fn draw_panel(&mut self, ui: &mut Ui, screen: Screen, s: Settings) {
        let d = screen
            .density
            .min(screen.height / 386.0)
            .min(screen.width / 336.0);
        let text_scale = d / ui.density;
        let p = Rect {
            x: screen.width - 332.0 * d,
            y: (screen.height - 370.0 * d) * 0.5,
            w: 322.0 * d,
            h: 370.0 * d,
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
            mybezo::ACCENT,
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
                    mybezo::ACCENT
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
                Slider::GearOpacity,
                "Gear opacity",
                (s.gear_opacity * 100.0 - 10.0) / 90.0,
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
                Choice::Opacity(value) | Choice::GearOpacity(value) => format!("{value}%"),
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
                mybezo::ACCENT,
                &value,
            );
            if kind == Slider::GearOpacity {
                Self::gear(
                    ui,
                    row.x + row.w - value_w - 13.0 * d,
                    row.y + 7.0 * d,
                    12.0 * d,
                    self.opacity_preview(kind, s.gear_opacity),
                );
            }
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
                    mybezo::ACCENT,
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
                mybezo::ACCENT,
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
                    298.0 + (i / 2) as f32 * 34.0,
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
                mybezo::ACCENT
            } else {
                mybezo::RAISED
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

#[cfg(test)]
mod tests {
    use super::*;

    fn settings() -> Settings {
        Settings {
            gestures: false,
            gyroscope: false,
            virtual_mouse: false,
            camera_sensitivity: 1.0,
            opacity: 1.0,
            gear_opacity: 0.5,
            controls_hidden: false,
            size: None,
            fitted: 1.0,
        }
    }

    fn screen() -> Screen {
        Screen {
            width: 1280.0,
            height: 720.0,
            density: 2.0,
            dark: true,
        }
    }

    #[test]
    fn opacity_sliders_preview_independently_and_release_commits() {
        let atlas = HudAtlas::boot();
        let mut batch = HudBatch::new([1280, 720]);
        let mut menu = Menu::new((0.95, 0.12));
        menu.open = true;
        menu.draw(&mut batch, &atlas, screen(), settings());

        let hit = |menu: &Menu, kind| {
            menu.sliders
                .iter()
                .find(|(slider, _)| *slider == kind)
                .unwrap()
                .1
        };
        let controls = hit(&menu, Slider::Opacity);
        let y = controls.y + 2.0;
        assert!(menu.down(4, (controls.x, y), screen()));
        assert_eq!(menu.preview_controls_opacity(1.0), 0.1);
        menu.moved(4, (controls.x + controls.w / 2.0, y), screen());
        let preview = menu.preview_controls_opacity(1.0);
        assert_eq!(preview, 0.55);
        assert_eq!(menu.opacity_preview(Slider::GearOpacity, 0.5), 0.5);

        let mut controls_layout = Controls::default_layout();
        let mut max_alpha = |opacity| {
            let mut frame = HudBatch::new([1280, 720]);
            controls_layout.draw(&mut frame, &atlas, true, screen(), opacity);
            frame
                .vertices
                .iter()
                .map(|vertex| vertex.color >> 24)
                .max()
                .unwrap()
        };
        assert!(max_alpha(preview) < max_alpha(1.0));
        assert_eq!(
            menu.up(4, (controls.x + controls.w / 2.0, y)),
            Some(Choice::Opacity(55))
        );

        let gear = hit(&menu, Slider::GearOpacity);
        assert!(menu.down(5, (gear.x, gear.y + 2.0), screen()));
        assert_eq!(menu.opacity_preview(Slider::GearOpacity, 0.5), 0.1);
        assert_eq!(menu.preview_controls_opacity(0.55), 0.55);
        menu.cancel(5);
        assert_eq!(menu.opacity_preview(Slider::GearOpacity, 0.5), 0.5);
    }

    #[test]
    fn collapsed_gear_draws_only_its_faded_shape() {
        let atlas = HudAtlas::boot();
        let mut batch = HudBatch::new([1280, 720]);
        Menu::new((0.95, 0.12)).draw(&mut batch, &atlas, screen(), settings());
        assert!(!batch.vertices.is_empty());
        assert!(
            batch
                .vertices
                .iter()
                .all(|vertex| vertex.color >> 24 == 128)
        );
    }
}
