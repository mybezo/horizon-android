use super::*;
use horizon_client::{
    osk::Output,
    ui_theme::color,
};
use winit::{
    event::{KeyEvent, Touch},
    keyboard::{KeyCode, PhysicalKey},
};

#[derive(Clone, Copy, PartialEq)]
pub(super) enum Action {
    Page(Page),
    Play,
    Back,
    SettingsSection(SettingsSection),
    Search,
    ShaderSource(crate::shader_catalog::Origin),
    ModSource(crate::shader_catalog::Origin),
    Field(&'static str),
    FinishNameEdit,
    Install(usize),
    Import(horizon_client::content_files::Kind),
    OpenFolder(horizon_client::content_files::Kind),
    ToggleSection(usize),
    ToggleLocal(usize),
    DeleteLocal(usize),
    ConfirmDeleteLocal,
    ConfirmMinimize,
    Next,
    Previous,
    Profile,
    Logs,
    RecordPerformance,
    ExportPerformance,
    ExportDiagnostics,
    Account,
    SignIn,
    ElySignIn,
    MicrosoftAccount,
    OfflineAccount,
    SubmitEly,
    SavedAccounts,
    SavedPage(bool),
    SelectAccount(usize),
    SelectOffline,
    WardrobeKind(bool),
    WardrobeSelect(usize),
    WardrobeImport,
    WardrobeCloud,
    WardrobeUpload,
    WardrobeActivateCape(usize),
    WardrobeUseCloud,
    WardrobeModel,
    WardrobeRotate(i8),
    WardrobeZoom(bool),
    WardrobeHideCape,
    DeviceSignIn,
    CopyLoginCode,
    CancelLogin,
    ReviewApp,
    Repository,
    SignOut,
    LoginLink,
    CloseLogin,
    Renderer,
    RenderScale,
    ChooseRenderScale(crate::display::RenderScale),
    Orientation,
    InputHost,
    ChooseInputHost(crate::catalog::InputHost),
    ChooseOrientation(crate::catalog::Orientation),
    ChooseRenderer(crate::catalog::RendererMode),
    ChooseProfile(u32),
    Dismiss,
    Immersive,
    Cutout,
    Aspect,
    ChooseAspect(crate::display::Aspect),
    DisplayHz,
    ChooseDisplayHz(u16),
    CacheBudget,
    ChooseCacheBudget(u32),
    ToggleBundled(&'static str),
    WorldFiles,
    GameFiles,
    WorkerLimit,
    ChooseWorkerLimit(u32),
    PerformanceHints,
    GpuProfile,
}

pub(super) const BACKGROUND_SPRITE: &str = "mybezo/launcher-background";

/// Shared launcher surfaces used by the catalogue, account dialog and settings.
pub(super) mod style {
    use horizon_client::ui_theme::color::hex;
    /// Cards and secondary buttons.
    pub const PANEL: u32 = hex(0x25292d, 255);
    pub const SURFACE: u32 = hex(0x34393c, 255);
    pub const SURFACE_HI: u32 = hex(0x3d4549, 255);
    /// Fields and switch tracks.
    pub const WELL: u32 = hex(0x25292d, 255);
    pub const LINE: u32 = hex(0x536167, 255);
    pub const LINE_SOFT: u32 = hex(0x536167, 150);
    pub const SHADOW: u32 = hex(0x000000, 96);
    pub const SHEEN: u32 = hex(0xffffff, 26);
    pub const GLOW: u32 = hex(0x97cff0, 46);
    pub const GLOW_SOFT: u32 = hex(0x97cff0, 20);
    pub const TEXT_SOFT: u32 = hex(0xbdc7cd, 255);
    pub const SUCCESS: u32 = hex(0x45c27a, 255);
    pub const DANGER: u32 = hex(0xf07a68, 255);
}

fn draw_play_background(ui: &mut Ui, w: f32, h: f32) {
    let Some(mut source) = ui.atlas.sprites.get(BACKGROUND_SPRITE).copied() else {
        return;
    };
    let aspect = w / h.max(1.0);
    let image_aspect = source.w as f32 / source.h as f32;
    if aspect > image_aspect {
        // Keep the eyes in view when a wider phone crops the top and bottom.
        let trim = (source.v1 - source.v0) * (1.0 - image_aspect / aspect) * 0.5;
        source.v0 += trim;
        source.v1 -= trim;
    } else {
        let trim = (source.u1 - source.u0) * (1.0 - aspect / image_aspect) * 0.5;
        source.u0 += trim;
        source.u1 -= trim;
    }
    let scale = ui.density / ui.batch.scale;
    ui.batch
        .rect_uv(0.0, 0.0, w * scale, h * scale, source, u32::MAX);

    ui.rounded(0.0, 0.0, w, h, 0.0, color::hex(0x100817, 67));
    // A soft vignette at the top (under the header) and the bottom (under the footer).
    for i in 0..16u8 {
        let step = i as f32;
        let alpha = 200u8.saturating_sub(i * 12);
        ui.rounded(0.0, step * 9.0, w, 9.0, 0.0, color::hex(0x0e0915, alpha));
        ui.rounded(0.0, h - (step + 1.0) * 7.0, w, 7.0, 0.0, color::hex(0x0e0915, alpha));
    }
}

/// A glass card: shadow, translucent plate, hairline border and a top sheen.
pub(super) fn card(ui: &mut Ui, x: f32, y: f32, w: f32, h: f32, radius: f32) {
    ui.rounded(x, y + 4.0, w, h, radius, style::SHADOW);
    ui.rounded(x, y, w, h, radius, style::SURFACE);
    ui.border(x, y, w, h, radius, style::LINE_SOFT);
    ui.rounded(x + radius, y + 1.0, (w - 2.0 * radius).max(0.0), 1.0, 0.0, style::SHEEN);
}

/// A small rounded label; returns its width.
pub(super) fn chip(ui: &mut Ui, x: f32, y: f32, text: &str, fg: u32, bg: u32) -> f32 {
    let size: f32 = 11.5;
    let width = ui.measure(Face::Medium, size, text) + 20.0;
    ui.rounded(x, y, width, 22.0, 11.0, bg);
    ui.text(
        Face::Medium,
        size,
        x + 10.0,
        y + (22.0 - (size * 1.45f32).round()) * 0.5,
        fg,
        text,
    );
    width
}

/// The icon a button shows beside its label.
fn action_icon(action: Action) -> Option<&'static str> {
    Some(match action {
        Action::Play => "play",
        Action::Search => "search",
        Action::OpenFolder(_) | Action::GameFiles | Action::WorldFiles => "folder",
        Action::Import(_) | Action::WardrobeImport => "plus",
        Action::Install(_) | Action::WardrobeCloud => "download",
        Action::DeleteLocal(_) | Action::ConfirmDeleteLocal => "trash",
        Action::Repository | Action::LoginLink | Action::ReviewApp => "external-link",
        Action::Logs | Action::ExportDiagnostics => "bug",
        Action::ExportPerformance => "terminal",
        Action::Account
        | Action::SignIn
        | Action::MicrosoftAccount
        | Action::OfflineAccount
        | Action::SavedAccounts
        | Action::DeviceSignIn => "user",
        Action::Previous => "arrow-left",
        Action::Next => "chevron-right",
        Action::Dismiss | Action::CancelLogin | Action::CloseLogin => "close",
        Action::ConfirmMinimize => "check",
        Action::ToggleBundled(_) => "zap",
        Action::SignOut => "arrow-left",
        Action::WardrobeUpload => "reload",
        _ => return None,
    })
}

fn page_icon(page: Page) -> &'static str {
    match page {
        Page::Play => "home",
        Page::Game => "gamepad",
        Page::Mods => "package",
        Page::Packs => "image",
        Page::Shaders => "zap",
        Page::Installed => "archive",
        Page::Settings => "settings",
        Page::Skins => "user",
    }
}

#[derive(Clone, Copy, PartialEq)]
pub(super) enum SettingsSection {
    Renderer,
    Game,
    Controls,
    Launcher,
    About,
}

impl SettingsSection {
    const ALL: [Self; 5] = [Self::Renderer, Self::Game, Self::Controls, Self::Launcher, Self::About];

    fn label(self) -> &'static str {
        match self {
            Self::Renderer => "Perender",
            Self::Game => "Game",
            Self::Controls => "Kontrol",
            Self::Launcher => "Launcher",
            Self::About => "Tentang",
        }
    }

    fn icon(self) -> &'static str {
        match self {
            Self::Renderer => "monitor",
            Self::Game => "play",
            Self::Controls => "gamepad",
            Self::Launcher => "package",
            Self::About => "info-box",
        }
    }

    fn for_action(action: Action) -> Self {
        match action {
            Action::Renderer | Action::RenderScale | Action::DisplayHz | Action::GpuProfile => Self::Renderer,
            Action::InputHost => Self::Controls,
            Action::Orientation | Action::Immersive | Action::Cutout | Action::Aspect => Self::Launcher,
            Action::Repository | Action::Logs | Action::ExportDiagnostics | Action::ExportPerformance => Self::About,
            _ => Self::Game,
        }
    }
}

pub(super) const SHELL: u32 = color::hex(0x1d2024, 255);
const SHEET: u32 = color::hex(0x34393c, 255);
const SELECTED: u32 = color::hex(0x364b57, 255);
pub(super) const LAUNCH_BLUE: u32 = color::hex(0x97cff0, 255);
const LAUNCH_INK: u32 = color::hex(0x10242f, 255);

fn center_text(ui: &mut Ui, face: Face, size: f32, center: f32, y: f32, width: f32, fg: u32, text: &str) {
    let label = ui.fit_text(face, size, text, width);
    let x = center - ui.measure(face, size, &label) * 0.5;
    ui.text(face, size, x, y, fg, &label);
}

fn nav_item(
    ui: &mut Ui,
    targets: &mut Vec<([f32; 4], Action)>,
    rect: [f32; 4],
    icon: &str,
    label: Option<&str>,
    active: bool,
    action: Action,
) {
    let [x, y, w, h] = rect;
    if active {
        ui.rounded(x, y, w, h, h * 0.5, SELECTED);
    }
    let ix = if label.is_some() { x + 13.0 } else { x + (w - 23.0) * 0.5 };
    ui.icon(icon, ix, y + (h - 23.0) * 0.5, 23.0, color::TEXT);
    if let Some(label) = label {
        ui.text(Face::Medium, 14.0, x + 42.0, y + (h - 20.0) * 0.5, color::TEXT, label);
    }
    targets.push((rect, action));
}

fn rail_item(
    ui: &mut Ui,
    targets: &mut Vec<([f32; 4], Action)>,
    y: f32,
    step: f32,
    icon: &str,
    label: &str,
    active: bool,
    action: Action,
) {
    let icon_height = (step - 20.0).clamp(22.0, 36.0);
    nav_item(ui, targets, [14.0, y, 64.0, icon_height], icon, None, active, action);
    if step >= 42.0 {
        center_text(ui, Face::Regular, 11.5, 46.0, y + icon_height + 2.0, 84.0, color::TEXT, label);
    }
    if let Some((rect, _)) = targets.last_mut() {
        *rect = [4.0, y, 84.0, (step - 1.0).max(icon_height)];
    }
}

fn avatar(ui: &mut Ui, x: f32, y: f32, size: f32) {
    // Generic pixel-art avatar used before a player skin is available here.
    let palette = [0xa66239, 0xd98b4e, 0xefbf95, 0xffdec0, 0xfffaf0, 0x4b6553, 0xd59482];
    let pixels = [
        [0, 1, 1, 1, 1, 1, 1, 0],
        [1, 1, 1, 1, 1, 2, 1, 1],
        [1, 1, 1, 2, 3, 3, 1, 1],
        [1, 2, 3, 3, 3, 3, 3, 1],
        [2, 4, 5, 3, 3, 5, 4, 2],
        [2, 3, 3, 3, 3, 3, 3, 2],
        [2, 3, 3, 6, 6, 3, 3, 2],
        [2, 2, 3, 3, 3, 3, 2, 2],
    ];
    let cell = size / 8.0;
    for (row, pixels) in pixels.iter().enumerate() {
        for (col, &pixel) in pixels.iter().enumerate() {
            ui.rounded(x + col as f32 * cell, y + row as f32 * cell, cell, cell, 0.0, color::hex(palette[pixel], 255));
        }
    }
}

/// A page link inside a dialog or content panel.
fn tab(ui: &mut Ui, rect: [f32; 4], label: &str, page: Page, active: bool) {
    let [x, y, w, h] = rect;
    let radius = (h * 0.5).min(16.0);
    if active {
        ui.rounded(x, y, w, h, radius, SELECTED);
    }
    let fg = if active { color::TEXT } else { style::TEXT_SOFT };
    let icon = page_icon(page);
    if w < 100.0 {
        let size: f32 = 16.0;
        ui.icon(icon, x + (w - size) * 0.5, y + 5.0, size, fg);
        let font: f32 = 9.5;
        let text = ui.fit_text(Face::Medium, font, label, w - 6.0);
        let width = ui.measure(Face::Medium, font, &text);
        ui.text(Face::Medium, font, x + (w - width) * 0.5, y + h - 17.0, fg, &text);
    } else {
        let size: f32 = 18.0;
        let font: f32 = 14.0;
        let text = ui.fit_text(Face::Medium, font, label, w - size - 28.0);
        let width = ui.measure(Face::Medium, font, &text);
        let left = x + (w - size - 8.0 - width) * 0.5;
        ui.icon(icon, left, y + (h - size) * 0.5, size, fg);
        ui.text(
            Face::Medium,
            font,
            left + size + 8.0,
            y + (h - (font * 1.45f32).round()) * 0.5,
            fg,
            &text,
        );
    }
}

pub(super) fn button(
    ui: &mut Ui,
    targets: &mut Vec<([f32; 4], Action)>,
    rect: [f32; 4],
    label: &str,
    action: Action,
    accent: bool,
) {
    let [x, y, w, h] = rect;
    if let Action::Page(page) = action {
        tab(ui, rect, label, page, accent);
        targets.push((rect, action));
        return;
    }
    let field = matches!(action, Action::Field(_));
    // Launch settings mark their dropdowns with a trailing " v": draw a chevron instead.
    let (label, dropdown) = match label.strip_suffix(" v") {
        Some(rest) => (rest, true),
        None => (label.trim_end(), false),
    };
    // Section headers start with "+ " (closed) or "- " (open).
    let (label, section) = if matches!(action, Action::ToggleSection(_)) {
        if let Some(rest) = label.strip_prefix("- ") {
            (rest, Some(true))
        } else if let Some(rest) = label.strip_prefix("+ ") {
            (rest, Some(false))
        } else {
            (label, None)
        }
    } else {
        (label, None)
    };
    let radius = (h * 0.5).min(18.0);

    let fill = if accent {
        LAUNCH_BLUE
    } else if field {
        style::WELL
    } else {
        style::SURFACE_HI
    };
    ui.rounded(x, y, w, h, radius, fill);
    if field {
        ui.border(x, y, w, h, radius, style::LINE_SOFT);
    }
    let fg = if accent {
        LAUNCH_INK
    } else if field {
        style::TEXT_SOFT
    } else {
        color::TEXT
    };
    let icon = match section {
        Some(true) => Some("chevron-down"),
        Some(false) => Some("chevron-right"),
        None => action_icon(action),
    };
    let icon_size = (h * 0.42).clamp(12.0, 20.0);
    if field {
        // The caret is placed at x + 12 + the Medium 16 width of the fitted text.
        let text = ui.fit_text(Face::Medium, 16.0, label, w - 24.0);
        let width = ui.measure(Face::Medium, 16.0, &text);
        ui.text(
            Face::Medium,
            16.0,
            x + 12.0,
            y + ((h - 23.0) * 0.5).max(0.0),
            fg,
            &text,
        );
        if width + icon_size + 36.0 < w {
            ui.icon(
                "search",
                x + w - 14.0 - icon_size,
                y + (h - icon_size) * 0.5,
                icon_size,
                style::TEXT_SOFT,
            );
        }
        targets.push((rect, action));
        return;
    }
    let font: f32 = if w < 80.0 {
        13.0
    } else if h >= 52.0 {
        18.0
    } else {
        15.0
    };
    let padding = (w * 0.1).clamp(6.0, 16.0);
    let show_icon = icon.is_some() && w >= 72.0;
    let mut room = w - 2.0 * padding;
    if show_icon {
        room -= icon_size + 8.0;
    }
    if dropdown {
        room -= 24.0;
    }
    let text = ui.fit_text(Face::Medium, font, label, room.max(8.0));
    let width = ui.measure(Face::Medium, font, &text);
    let content = width + if show_icon { icon_size + 8.0 } else { 0.0 };
    let mut left = if dropdown || section.is_some() {
        x + padding
    } else {
        x + ((w - content) * 0.5).max(padding.min(w * 0.5))
    };
    if show_icon && let Some(name) = icon {
        ui.icon(name, left, y + (h - icon_size) * 0.5, icon_size, fg);
        left += icon_size + 8.0;
    }
    ui.text(
        Face::Medium,
        font,
        left,
        y + ((h - (font * 1.45f32).round()) * 0.5).max(0.0),
        fg,
        &text,
    );
    if dropdown {
        ui.icon(
            "chevron-down",
            x + w - padding - 16.0,
            y + (h - 16.0) * 0.5,
            16.0,
            style::TEXT_SOFT,
        );
    }
    targets.push((rect, action));
}

/// A switch: an accent track with a lit knob when on, a sunk well when off.
fn toggle(
    ui: &mut Ui,
    targets: &mut Vec<([f32; 4], Action)>,
    rect: [f32; 4],
    on: bool,
    action: Action,
) {
    let [x, y, w, h] = rect;
    ui.text(
        Face::Medium,
        14.0,
        x,
        y + (h - 20.0) * 0.5,
        if on { color::TEXT } else { style::TEXT_SOFT },
        if on { "On" } else { "Off" },
    );
    let (tw, th): (f32, f32) = (54.0, 30.0);
    let tx = x + w - tw - 2.0;
    let ty = y + (h - th) * 0.5;
    if on {
        ui.rounded(tx - 2.0, ty - 2.0, tw + 4.0, th + 4.0, th * 0.5 + 2.0, style::GLOW);
        ui.rounded(tx, ty, tw, th, th * 0.5, LAUNCH_BLUE);
    } else {
        ui.rounded(tx, ty, tw, th, th * 0.5, style::WELL);
        ui.border(tx, ty, tw, th, th * 0.5, style::LINE);
    }
    let knob = th - 6.0;
    let kx = if on { tx + tw - knob - 3.0 } else { tx + 3.0 };
    ui.rounded(kx, ty + 5.0, knob, knob, knob * 0.5, style::SHADOW);
    ui.rounded(
        kx,
        ty + 3.0,
        knob,
        knob,
        knob * 0.5,
        if on { color::hex(0xffffff, 255) } else { style::TEXT_SOFT },
    );
    targets.push(([x, y, w, h], action));
}


impl Launcher {
    /// The same spatial focus algorithm as the game's controller menus.
    pub(super) fn navigate(&mut self, key: &KeyEvent) -> bool {
        self.navigate_key(&key.logical_key)
    }
    pub(super) fn navigate_key(&mut self, key: &winit::keyboard::Key) -> bool {
        use horizon_client::gamepad::{self, Button};
        use winit::keyboard::{Key, NamedKey};
        if self.pending_focus.is_some() {
            return true;
        }
        if self.catalog.editing.is_some() || self.targets.is_empty() {
            return false;
        }
        let button = match key {
            Key::Named(NamedKey::ArrowUp) => Button::DPadUp,
            Key::Named(NamedKey::ArrowDown) => Button::DPadDown,
            Key::Named(NamedKey::ArrowLeft) => Button::DPadLeft,
            Key::Named(NamedKey::ArrowRight) => Button::DPadRight,
            Key::Named(NamedKey::Tab) => {
                self.focus = Some(self.focus.map_or(0, |i| (i + 1) % self.targets.len()));
                return true;
            }
            _ => return false,
        };
        if matches!(button, Button::DPadUp | Button::DPadDown)
            && let Some((_, action)) = self.focus.and_then(|i| self.targets.get(i))
            && let Some(index) = self.list_targets.iter().position(|(_, a)| a == action)
        {
            let rects: Vec<_> = self.list_targets.iter().map(|(r, _)| *r).collect();
            if let Some(next) = gamepad::focus_step_strict(rects[index], button, &rects)
                && let Some(view) = self.list_view
            {
                let (rect, action) = self.list_targets[next];
                let scroll = if let Some(chooser) = &mut self.chooser {
                    &mut chooser.scroll
                } else {
                    &mut self.catalog.scroll
                };
                *scroll = super::gesture::reveal(*scroll, rect, view);
                self.pending_focus = Some(action);
                self.focus = None;
                if let Some(window) = &self.window {
                    window.request_redraw();
                }
                return true;
            }
        }
        self.focus = Some(
            if let Some(index) = self.focus.filter(|i| *i < self.targets.len()) {
                let rects: Vec<_> = self.targets.iter().map(|(rect, _)| *rect).collect();
                gamepad::focus_step_rect(rects[index], button, &rects).unwrap_or(index)
            } else {
                0
            },
        );
        true
    }

    pub(super) fn draw_pages(&mut self) -> Result<()> {
        let (Some(r), Some(gui)) = (&mut self.renderer, &self.gui) else {
            return Ok(());
        };
        if r.suspended {
            return Ok(());
        }
        self.targets.clear();
        self.list_targets.clear();
        self.list_view = None;
        let mut b = HudBatch::new([r.config.width, r.config.height]);
        let mut ui = Ui::mobile(&gui.atlas, &mut b, (0.0, 0.0));
        // OEM navigation bars can remain visible even in fullscreen (keypad phones).
        // Inset the launcher layout only; keep surface and input coordinates intact.
        if let Some(window) = &self.window {
            use winit::platform::android::WindowExtAndroid;
            let rect = window.content_rect();
            let input = winit::platform::android::native_input_scale();
            if rect.bottom > 0 {
                ui.h = ui.h.min(rect.bottom as f32 * input[1] as f32 / ui.density);
            }
            if rect.right > 0 {
                ui.w = ui.w.min(rect.right as f32 * input[0] as f32 / ui.density);
            }
        }
        if let Some(bottom) = crate::native_ime::viewport_bottom() {
            ui.h = ui.h.min(bottom as f32 / ui.density);
        }
        let (w, h) = (ui.w, ui.h);
        let page = self.catalog.page;
        let home = page == Page::Play;
        let settings = page == Page::Settings;
        let download = page == Page::Game || page == Page::Installed || page.is_catalog();
        let left = if settings || download { 104.0 } else { 0.0 };
        let compact = w - left < 700.0;
        ui.rounded(0.0, 0.0, w, h, 0.0, SHELL);
        if home {
            if self.foreground
                && let Some(patch) = self.motion.patch(&gui.atlas, BACKGROUND_SPRITE)
            {
                ui.batch.patches.push(patch);
            }
            draw_play_background(&mut ui, w, h);
        } else {
            self.motion.pause();
        }
        if !home {
            nav_item(&mut ui, &mut self.targets, [9.0, 5.0, 45.0, 40.0],
                "arrow-left", None, false, Action::Back);
            nav_item(&mut ui, &mut self.targets, [57.0, 5.0, 45.0, 40.0],
                "home", None, false, Action::Page(Page::Play));
        }
        let title = if settings {
            format!("Pengaturan - {}", self.settings_section.label())
        } else if download {
            format!("Unduh - {}", page.name())
        } else if home {
            "Horizon".into()
        } else {
            page.name().into()
        };
        let mut right = w - 12.0;
        for (icon, label, selected, action) in [
            ("settings", "Pengaturan", settings, Action::Page(Page::Settings)),
            ("download", "Unduh", download, Action::Page(Page::Game)),
            ("user", "Akun", false, Action::Account),
        ] {
            let show_label = selected && w >= 540.0;
            let width = if show_label { 125.0 } else { 45.0 };
            right -= width;
            nav_item(&mut ui, &mut self.targets, [right, 5.0, width, 39.0],
                icon, show_label.then_some(label), selected, action);
            right -= 5.0;
        }
        let title_x = if home { 17.0 } else { 112.0 };
        let title = ui.fit_text(Face::Medium, 20.0, &title, (right - title_x - 4.0).max(0.0));
        ui.text(Face::Medium, 20.0, title_x, 10.0, color::TEXT, &title);
        if settings || download {
            let spacing = ((h - 64.0) / 6.0).clamp(24.0, 76.0);
            if settings {
                for (i, section) in SettingsSection::ALL.into_iter().enumerate() {
                    rail_item(&mut ui, &mut self.targets, 57.0 + i as f32 * spacing, spacing,
                        section.icon(), section.label(), self.settings_section == section,
                        Action::SettingsSection(section));
                }
            } else {
                for (i, item) in [Page::Game, Page::Mods, Page::Packs, Page::Shaders, Page::Installed, Page::Skins]
                    .into_iter().enumerate()
                {
                    rail_item(&mut ui, &mut self.targets, 57.0 + i as f32 * spacing, spacing,
                        page_icon(item), item.name(), page == item, Action::Page(item));
                }
            }
        }

        if home || page == Page::Game {
            use horizon_client::official_install::InstallStage;
            let installed = horizon_client::official_install::ready(&self.home);
            let progress = *self.install.lock().unwrap();
            let installing = self.worker.is_some();
            let (x, y, width, height) = if home {
                let width = if w >= h { (w * 0.287).max(170.0) } else { w - 28.0 };
                let height = if w >= h { h - 69.0 } else { (h * 0.50).min(370.0) };
                (w - width - 14.0, h - height - 14.0, width, height)
            } else {
                (left + 7.0, 63.0, w - left - 21.0, 78.0)
            };
            let center = x + width * 0.5;
            let launch = if installed { "Luncurkan" } else if self.error.is_some() { "Coba lagi" } else { "Unduh" };
            ui.rounded(x, y, width, height, if home { 32.0 } else { 17.0 },
                if home { color::hex(0x34393c, 244) } else { SHEET });
            let version = format!("Minecraft {}", horizon_core::VANILLA_VERSION);
            if home {
                let account_end = y + height - 106.0;
                let account_space = (account_end - y).max(0.0);
                let size = 65.0f32.min((account_space - 47.0).max(0.0));
                let avatar_y = y + (account_space - size - 47.0).max(0.0) * 0.5;
                if size >= 16.0 {
                    avatar(&mut ui, center - size * 0.5, avatar_y, size);
                }
                let name = self.login.profile.as_deref().or_else(|| {
                    let name = self.catalog.config.name.trim();
                    (!name.is_empty()).then_some(name)
                }).unwrap_or("Masuk ke akun");
                let provider = if self.login.pending() { "Sedang masuk…" }
                    else if self.login.signed_in {
                        self.login.saved_accounts.iter().find(|a| a.name == name)
                            .map_or("Akun tersimpan", |a| match a.provider {
                                crate::account::Provider::Microsoft => "Microsoft",
                                crate::account::Provider::Ely => "Ely.by",
                            })
                    } else if self.login.profile.is_some() { "Perlu masuk kembali" }
                    else { "Offline" };
                let text_y = avatar_y + size + 8.0;
                center_text(&mut ui, Face::Medium, 17.0, center, text_y, width - 22.0, color::TEXT, name);
                center_text(&mut ui, Face::Regular, 13.0, center, text_y + 22.0, width - 22.0, color::MUTED, provider);
                self.targets.push(([x + 10.0, y + 4.0, width - 20.0, (account_space - 4.0).max(0.0)], Action::Account));

                let vy = y + height - 106.0;
                ui.icon("package", x + 17.0, vy + 13.0, 22.0, color::MUTED);
                let version = ui.fit_text(Face::Medium, 15.0, &version, width - 99.0);
                ui.text(Face::Medium, 15.0, x + 48.0, vy + 4.0, color::TEXT, &version);
                ui.text(Face::Regular, 12.0, x + 48.0, vy + 26.0, color::MUTED, "Horizon");
                nav_item(&mut ui, &mut self.targets, [x + width - 46.0, vy + 3.0, 38.0, 40.0],
                    "settings", None, false, Action::Page(Page::Settings));
            } else {
                ui.icon("package", x + 15.0, y + 23.0, 27.0, color::TEXT);
                let version_space = width - 78.0 - (width * 0.38).min(142.0);
                let version = ui.fit_text(Face::Medium, 17.0, &version, version_space.max(10.0));
                let title_width = ui.text(Face::Medium, 17.0, x + 55.0, y + 13.0, color::TEXT, &version);
                if version_space - title_width >= 60.0 {
                    chip(&mut ui, x + 65.0 + title_width, y + 14.0,
                        "Rilis", color::TEXT, SELECTED);
                }
                ui.text(Face::Regular, 13.0, x + 55.0, y + 38.0, color::MUTED,
                    if installed { "Terpasang · Horizon" } else { "Tersedia untuk Horizon" });
            }

            let [bx, by, bw, bh] = if home {
                [x + 13.0, y + height - 58.0, width - 26.0, 44.0]
            } else {
                let width = (width * 0.38).min(142.0);
                [w - width - 25.0, y + 18.0, width, 42.0]
            };
            if installing {
                let step = InstallStage::ALL.iter().position(|s| *s == progress.stage).unwrap_or(0);
                let fraction = if progress.total > 0 {
                    (progress.done as f32 / progress.total as f32).clamp(0.0, 1.0)
                } else { 0.0 };
                let overall = (step as f32 + fraction) / InstallStage::ALL.len() as f32;
                center_text(&mut ui, Face::Medium, 12.0, bx + bw * 0.5, by, bw, color::TEXT, progress.stage.label());
                ui.rounded(bx, by + 25.0, bw, 5.0, 2.5, SELECTED);
                ui.rounded(bx, by + 25.0, bw * overall, 5.0, 2.5, LAUNCH_BLUE);
            } else {
                ui.rounded(bx, by, bw, bh, bh * 0.5, LAUNCH_BLUE);
                center_text(&mut ui, Face::Medium, 16.0, bx + bw * 0.5, by + (bh - 23.0) * 0.5,
                    bw - 14.0, LAUNCH_INK, launch);
                self.targets.push(([bx, by, bw, bh], Action::Play));
            }
        } else if page == Page::Skins {
            self.wardrobe
                .draw(&mut ui, &mut self.targets, self.catalog.scroll);
            self.list_view = Some(self.wardrobe.view);
            self.catalog.scroll = self.catalog.scroll.min(self.wardrobe.view[2]);
        } else if page.is_catalog() {
            ui.icon(page_icon(page), left + 32.0, 61.0, 20.0, LAUNCH_BLUE);
            ui.text(
                Face::SemiBold,
                18.0,
                left + 60.0,
                58.0,
                color::TEXT,
                if page == Page::Mods {
                    "Horizon Mods"
                } else if page == Page::Shaders {
                    if w < 360.0 { "Shaders" } else { "Shader Packs" }
                } else {
                    "Resource Packs"
                },
            );
            let search_width = ((w - left - 74.0) * 0.38).clamp(52.0, 158.0);
            button(
                &mut ui,
                &mut self.targets,
                [left + 22.0, 90.0, (w - left - 62.0 - search_width).max(20.0), 44.0],
                &format!(
                    "{}{}",
                    if page == Page::Mods {
                        "Search / .hmod URL: "
                    } else {
                        "Search: "
                    },
                    self.catalog.query
                ),
                Action::Field("search"),
                false,
            );
            button(
                &mut ui,
                &mut self.targets,
                [w - 30.0 - search_width, 90.0, search_width, 44.0],
                if page == Page::Mods && self.catalog.query.trim().starts_with("https://") {
                    "Import .hmod"
                } else {
                    "Search"
                },
                Action::Search,
                true,
            );
            if matches!(page, Page::Shaders | Page::Mods) {
                use crate::shader_catalog::Origin;
                let width = ((w - 185.0) / 2.0).clamp(58.0, 120.0);
                for (i, origin) in [Origin::Horizon, Origin::Modrinth].into_iter().enumerate() {
                    let rect = [w - 32.0 - (2 - i) as f32 * width, 56.0, width - 5.0, 30.0];
                    let selected = (if page == Page::Mods {
                        self.catalog.config.mod_source
                    } else {
                        self.catalog.config.shader_source
                    }) == origin;
                    button(
                        &mut ui,
                        &mut self.targets,
                        rect,
                        origin.source().label(),
                        if page == Page::Mods {
                            Action::ModSource(origin)
                        } else {
                            Action::ShaderSource(origin)
                        },
                        selected,
                    );
                }
            }
            let hint = match page {
                Page::Mods
                    if self.catalog.config.mod_source
                        == crate::shader_catalog::Origin::Modrinth =>
                {
                    "Modrinth Java mods are browse-only; Fabric/Forge/NeoForge cannot run here"
                }
                Page::Mods => "Search Horizon mods or paste an HTTPS .hmod link",
                Page::Shaders
                    if self.catalog.config.shader_source
                        == crate::shader_catalog::Origin::Modrinth =>
                {
                    "Modrinth shader ZIPs / experimental compatibility; some packs may not work"
                }
                Page::Shaders => "Horizon native .hshader packs",
                _ => "Verified downloads / enable in Installed",
            };
            let hint = ui.fit_text(Face::Regular, 14.0, hint, w - left - 64.0);
            ui.text(Face::Regular, 14.0, left + 32.0, 186.0, color::MUTED, &hint);
            let kind = match page {
                Page::Mods => horizon_client::content_files::Kind::Mods,
                Page::Shaders => horizon_client::content_files::Kind::Shaders,
                _ => horizon_client::content_files::Kind::Packs,
            };
            let file_button_width = ((w - left - 72.0) / 2.0).min(158.0);
            button(
                &mut ui,
                &mut self.targets,
                [left + 32.0, 138.0, file_button_width, 38.0],
                "Open folder",
                Action::OpenFolder(kind),
                false,
            );
            button(
                &mut ui,
                &mut self.targets,
                [w - 32.0 - file_button_width, 138.0, file_button_width, 38.0],
                "Import file",
                Action::Import(kind),
                false,
            );
            let bundled: Vec<_> = if page == Page::Mods
                && self.catalog.config.mod_source == crate::shader_catalog::Origin::Horizon
            {
                let query = self.catalog.query.to_lowercase();
                horizon_client::mod_list::ANDROID_BUNDLED
                    .into_iter()
                    .filter(|(id, title, _)| {
                        id.contains(&query) || title.to_lowercase().contains(&query)
                    })
                    .collect()
            } else {
                Vec::new()
            };
            let catalog_rows: Vec<_> = self
                .catalog
                .results
                .iter()
                .enumerate()
                .filter(|(_, p)| {
                    !(page == Page::Mods
                        && p.source == crate::browser::SourceId::Horizon
                        && horizon_client::mod_list::ANDROID_BUNDLED
                            .iter()
                            .any(|(id, _, _)| *id == p.id))
                })
                .collect();
            let row_count = bundled.len() + catalog_rows.len();
            let top = 212.0;
            let bottom = h - 112.0;
            self.catalog.scroll = self
                .catalog
                .scroll
                .clamp(0.0, (row_count as f32 * 78.0 - (bottom - top)).max(0.0));
            self.list_view = Some([
                top,
                bottom,
                (row_count as f32 * 78.0 - (bottom - top)).max(0.0),
            ]);
            for (row, (id, title, description)) in bundled.iter().enumerate() {
                let y = top + row as f32 * 78.0 - self.catalog.scroll;
                if y < top - 1.0 || y + 68.0 > bottom {
                    continue;
                }
                let on = self.catalog.bundled_enabled.contains(*id);
                card(&mut ui, left + 22.0, y - 4.0, w - left - 44.0, 70.0, 16.0);
                ui.rounded(left + 32.0, y + 7.0, 44.0, 44.0, 12.0, color::hex(0x97cff0, if on { 70 } else { 26 }));
                ui.icon("zap", left + 42.0, y + 17.0, 24.0, if on { LAUNCH_BLUE } else { style::TEXT_SOFT });
                let title = ui.fit_text(Face::Medium, 18.0, title, (w - left - 340.0).max(60.0));
                let summary = ui.fit_text(Face::Regular, 13.5, description, (w - left - 262.0).max(24.0));
                let title_w = ui.text(Face::Medium, 18.0, left + 90.0, y + 4.0, color::TEXT, &title);
                chip(
                    &mut ui,
                    left + 96.0 + title_w,
                    y + 6.0,
                    if on { "ON" } else { "BUILT-IN" },
                    if on { LAUNCH_INK } else { style::TEXT_SOFT },
                    if on { style::SUCCESS } else { style::LINE_SOFT },
                );
                ui.text(Face::Regular, 13.5, left + 90.0, y + 32.0, color::MUTED, &summary);
                button(
                    &mut ui,
                    &mut self.targets,
                    [w - 154.0, y + 9.0, 118.0, 40.0],
                    if on { "Disable" } else { "Enable" },
                    Action::ToggleBundled(id),
                    !on,
                );
            }
            for (row, (i, p)) in catalog_rows.into_iter().enumerate() {
                let y = top + (row + bundled.len()) as f32 * 78.0 - self.catalog.scroll;
                let download = self.catalog.download_label(p);
                if download.is_none()
                    && !crate::catalog::source_only_mod(p)
                    && !(p.kind == crate::browser::Kind::Mod
                        && p.source == crate::browser::SourceId::Modrinth)
                {
                    self.list_targets
                        .push(([w - 150.0, y, 118.0, 70.0], Action::Install(i)));
                }
                if y < top || y + 70.0 > bottom {
                    continue;
                }
                card(&mut ui, left + 22.0, y - 4.0, w - left - 44.0, 70.0, 16.0);
                ui.rounded(left + 32.0, y + 7.0, 44.0, 44.0, 12.0, style::SURFACE_HI);
                ui.border(left + 32.0, y + 7.0, 44.0, 44.0, 12.0, style::LINE);
                let initial: String = p
                    .title
                    .chars()
                    .find(|c| c.is_alphanumeric())
                    .map_or_else(|| "?".into(), |c| c.to_uppercase().collect());
                let initial_w = ui.measure(Face::SemiBold, 20.0, &initial);
                ui.text(
                    Face::SemiBold,
                    20.0,
                    left + 32.0 + (44.0 - initial_w) * 0.5,
                    y + 15.0,
                    LAUNCH_BLUE,
                    &initial,
                );
                let title = ui.fit_text(Face::Medium, 18.0, &p.title, (w - left - 262.0).max(24.0));
                let summary = ui.fit_text(Face::Regular, 13.5, &p.summary, (w - left - 262.0).max(24.0));
                ui.text(Face::Medium, 18.0, left + 90.0, y + 4.0, color::TEXT, &title);
                ui.text(Face::Regular, 13.5, left + 90.0, y + 32.0, color::MUTED, &summary);
                if let Some(state) = download {
                    ui.text(Face::Medium, 15.0, w - 154.0, y + 19.0, LAUNCH_BLUE, &state);
                } else if p.kind == crate::browser::Kind::Mod
                    && p.source == crate::browser::SourceId::Modrinth
                {
                    ui.text(
                        Face::Regular,
                        14.0,
                        w - 154.0,
                        y + 20.0,
                        color::MUTED,
                        "Java mod",
                    );
                } else if crate::catalog::source_only_mod(p) {
                    ui.text(
                        Face::Regular,
                        14.0,
                        w - 154.0,
                        y + 20.0,
                        color::MUTED,
                        ".hnmod: later",
                    );
                } else if p.kind == crate::browser::Kind::Mod && !cfg!(target_pointer_width = "64")
                {
                    ui.text(
                        Face::Regular,
                        14.0,
                        w - 154.0,
                        y + 20.0,
                        color::MUTED,
                        "64-bit required",
                    );
                } else {
                    button(
                        &mut ui,
                        &mut self.targets,
                        [w - 154.0, y + 9.0, 118.0, 40.0],
                        self.catalog.install_button(p),
                        Action::Install(i),
                        false,
                    );
                }
            }
            button(
                &mut ui,
                &mut self.targets,
                [left + 32.0, h - 100.0, 120.0, 40.0],
                "Previous",
                Action::Previous,
                false,
            );
            button(
                &mut ui,
                &mut self.targets,
                [left + 166.0, h - 100.0, 120.0, 40.0],
                "Next",
                Action::Next,
                false,
            );
        } else if page == Page::Installed {
            use horizon_client::content_files::Kind;
            let top = 60.0;
            let bottom = h - 100.0;
            let row_height = if compact { 110.0 } else { 72.0 };
            let total = Kind::ALL
                .iter()
                .enumerate()
                .map(|(i, k)| {
                    48.0 + if self.catalog.sections[i] {
                        self.catalog.local.iter().filter(|e| e.kind == *k).count() as f32
                            * row_height
                    } else {
                        0.0
                    }
                })
                .sum::<f32>();
            let maximum = (total - (bottom - top)).max(0.0);
            self.catalog.scroll = self.catalog.scroll.clamp(0.0, maximum);
            self.list_view = Some([top, bottom, maximum]);
            let mut offset = 0.0;
            for (section, kind) in Kind::ALL.into_iter().enumerate() {
                let count = self.catalog.local.iter().filter(|e| e.kind == kind).count();
                let y = top + offset - self.catalog.scroll;
                let label = format!(
                    "{} {} ({count})",
                    if self.catalog.sections[section] {
                        "-"
                    } else {
                        "+"
                    },
                    kind.label()
                );
                self.list_targets
                    .push(([left + 32.0, y, w - left - 210.0, 40.0], Action::ToggleSection(section)));
                self.list_targets
                    .push(([w - 165.0, y, 133.0, 40.0], Action::Import(kind)));
                if y >= top && y + 40.0 <= bottom {
                    button(
                        &mut ui,
                        &mut self.targets,
                        [left + 32.0, y, w - left - 210.0, 40.0],
                        &label,
                        Action::ToggleSection(section),
                        false,
                    );
                    button(
                        &mut ui,
                        &mut self.targets,
                        [w - 165.0, y, 133.0, 40.0],
                        "Import",
                        Action::Import(kind),
                        false,
                    );
                }
                offset += 48.0;
                if !self.catalog.sections[section] {
                    continue;
                }
                for (i, item) in self
                    .catalog
                    .local
                    .iter()
                    .enumerate()
                    .filter(|(_, e)| e.kind == kind)
                {
                    let y = top + offset - self.catalog.scroll;
                    offset += row_height;
                    self.list_targets.push((
                        [
                            if compact { left + 32.0 } else { w - 300.0 },
                            y,
                            120.0,
                            row_height - 4.0,
                        ],
                        Action::ToggleLocal(i),
                    ));
                    self.list_targets.push((
                        [w - 165.0, y, 133.0, row_height - 4.0],
                        Action::DeleteLocal(i),
                    ));
                    if y < top || y + row_height - 4.0 > bottom {
                        continue;
                    }
                    card(&mut ui, left + 22.0, y - 6.0, w - left - 44.0, row_height - 6.0, 14.0);
                    ui.rounded(
                        left + 22.0,
                        y + 6.0,
                        3.0,
                        row_height - 30.0,
                        1.5,
                        if item.enabled { LAUNCH_BLUE } else { style::LINE },
                    );
                    let title = ui.fit_text(
                        Face::Medium,
                        19.0,
                        &item.title,
                        if compact { w - left - 64.0 } else { w - left - 364.0 },
                    );
                    ui.text(Face::Medium, 19.0, left + 32.0, y, color::TEXT, &title);
                    ui.text(
                        Face::Regular,
                        14.0,
                        left + 32.0,
                        y + 28.0,
                        color::MUTED,
                        kind.label(),
                    );
                    if kind == Kind::Mods && !cfg!(target_pointer_width = "64") {
                        ui.text(
                            Face::Regular,
                            14.0,
                            if compact { left + 32.0 } else { w - 300.0 },
                            y + if compact { 62.0 } else { 12.0 },
                            color::MUTED,
                            "64-bit required",
                        );
                    } else {
                        toggle(
                            &mut ui,
                            &mut self.targets,
                            [
                                if compact { left + 32.0 } else { w - 300.0 },
                                y + if compact { 55.0 } else { 0.0 },
                                120.0,
                                40.0,
                            ],
                            item.enabled,
                            Action::ToggleLocal(i),
                        );
                    }
                    button(
                        &mut ui,
                        &mut self.targets,
                        [w - 165.0, y + if compact { 55.0 } else { 0.0 }, 133.0, 40.0],
                        "Remove",
                        Action::DeleteLocal(i),
                        false,
                    );
                }
            }
        } else {
            let c = &self.catalog.config;
            let rows = [
                ("Perender Global", c.renderer.label().to_owned(), Action::Renderer),
                (
                    "Skala Resolusi",
                    if self.catalog.editing == Some("render_scale") {
                        self.catalog.render_scale_text.clone()
                    } else {
                        format!("{}%", c.render_scale.percent())
                    },
                    Action::RenderScale,
                ),
                (
                    "Batas Worker Chunk",
                    if c.worker_limit == 0 {
                        "No additional limit".into()
                    } else {
                        c.worker_limit.to_string()
                    },
                    Action::WorkerLimit,
                ),
                (
                    "Cache RAM",
                    if c.cache_budget_mib == 0 {
                        "Engine default (80 MiB)".into()
                    } else {
                        format!("{} MiB", c.cache_budget_mib)
                    },
                    Action::CacheBudget,
                ),
                (
                    "File Game",
                    "Open in system Files".into(),
                    Action::GameFiles,
                ),
                (
                    "File Dunia",
                    "Import / copy / back up in Files".into(),
                    Action::WorldFiles,
                ),
                ("Profil", format!("Profile {}", c.profile), Action::Profile),
                (
                    "Orientasi",
                    c.orientation.label().to_owned(),
                    Action::Orientation,
                ),
                ("Mode Input", c.input_host.label().to_owned(), Action::InputHost),
                (
                    "Layar Penuh",
                    if c.screen.immersive { "On" } else { "Off" }.into(),
                    Action::Immersive,
                ),
                (
                    "Hindari Area Kamera",
                    if c.screen.avoid_cutout { "On" } else { "Off" }.into(),
                    Action::Cutout,
                ),
                (
                    "Rasio Layar",
                    c.screen.aspect.label().into(),
                    Action::Aspect,
                ),
                (
                    "Refresh Rate",
                    c.device.display_label(),
                    Action::DisplayHz,
                ),
                (
                    "Optimasi Performa",
                    if c.device.performance_hints {
                        "On (ADPF)"
                    } else {
                        "Off"
                    }
                    .into(),
                    Action::PerformanceHints,
                ),
                (
                    "Profil GPU",
                    if c.device.gpu_profile {
                        "On / included in report"
                    } else {
                        "Off"
                    }
                    .into(),
                    Action::GpuProfile,
                ),
                (
                    "Rekam Performa",
                    if c.record_performance {
                        "On / up to 30 min"
                    } else {
                        "Off"
                    }
                    .into(),
                    Action::RecordPerformance,
                ),
                (
                    "Laporan Diagnostik",
                    "Export one ZIP to share".into(),
                    Action::ExportDiagnostics,
                ),
                (
                    "Data Performa Mentah",
                    "Advanced / JSONL in Reports".into(),
                    Action::ExportPerformance,
                ),
                (
                    "Server",
                    if c.server.is_empty() {
                        "Singleplayer / tap to enter server".into()
                    } else {
                        c.server.clone()
                    },
                    Action::Field("server"),
                ),
                ("Diagnostik", "Lihat status dan log".into(), Action::Logs),
                ("GitHub", "Buka repositori Horizon".into(), Action::Repository),
            ];
            let top = 58.0;
            let bottom = h - 59.0;
            let row_height = 77.0;
            let section = self.settings_section;
            let rows = rows.into_iter().filter(|(_, _, action)| SettingsSection::for_action(*action) == section);
            let maximum = (rows.clone().count() as f32 * row_height - (bottom - top)).max(0.0);
            self.catalog.scroll = self.catalog.scroll.clamp(0.0, maximum);
            self.list_view = Some([top, bottom, maximum]);
            for (i, (title, value, action)) in rows.enumerate() {
                let y = top + i as f32 * row_height - self.catalog.scroll;
                let rect = [left + 7.0, y, w - left - 21.0, row_height - 3.0];
                self.list_targets.push((rect, action));
                if y < top || y + rect[3] > bottom {
                    continue;
                }
                ui.rounded(rect[0], y, rect[2], rect[3], if i == 0 { 21.0 } else { 5.0 }, SHEET);
                let text_x = rect[0] + 18.0;
                let title = ui.fit_text(Face::Medium, 17.0, title, rect[2] - 62.0);
                ui.text(Face::Medium, 17.0, text_x, y + 11.0, color::TEXT, &title);
                let value = ui.fit_text(Face::Regular, 14.0, &value, rect[2] - 62.0);
                ui.text(Face::Regular, 14.0, text_x, y + 39.0, color::MUTED, &value);
                let icon = match action {
                    Action::Renderer | Action::RenderScale | Action::DisplayHz
                    | Action::Orientation | Action::InputHost | Action::Aspect | Action::Profile
                    | Action::WorkerLimit | Action::CacheBudget => "chevron-down",
                    _ => "chevron-right",
                };
                ui.icon(icon, w - 47.0, y + 25.0, 22.0, color::TEXT);
                self.targets.push((rect, action));
            }
        }
        let status = self.error.as_deref().unwrap_or(&self.catalog.status);
        if !status.trim().is_empty() && (!home || self.error.is_some()) {
            let (x, width) = if home && w >= h {
                (14.0, (w * 0.69 - 28.0).max(90.0))
            } else {
                (left + 7.0, w - left - 21.0)
            };
            let y = h - 48.0;
            let foreground = if self.error.is_some() { style::DANGER } else { color::TEXT };
            ui.rounded(x, y, width, 36.0, 12.0, SHEET);
            ui.icon("info-box", x + 10.0, y + 10.0, 16.0, foreground);
            let label = ui.fit_text(Face::Regular, 13.0, status, width - 46.0);
            ui.text(Face::Regular, 13.0, x + 35.0, y + 9.0, foreground, &label);
        }
        if self.login.visible {
            self.list_targets.clear();
            self.list_view = None;
            self.login.draw(
                &mut ui,
                &mut self.targets,
                &self.catalog.config.name,
                self.catalog.editing,
            );
        }
        if let Some(chooser) = &mut self.chooser {
            chooser.draw(&mut ui, &mut self.targets);
            self.list_targets = chooser.navigation(ui.w);
            self.list_view = Some(chooser.viewport(ui.h));
        }
        if let Some([top, bottom, maximum]) = self.list_view
            && maximum > 0.0
            && bottom > top
        {
            let scroll = self
                .chooser
                .as_ref()
                .map_or(self.catalog.scroll, |c| c.scroll);
            let height = bottom - top;
            let thumb = (height * height / (height + maximum)).max(20.0).min(height);
            let y = top + (height - thumb) * (scroll / maximum).clamp(0.0, 1.0);
            ui.rounded(w - 12.0, top, 4.0, height, 2.0, style::LINE_SOFT);
            ui.rounded(w - 13.0, y, 6.0, thumb, 3.0, LAUNCH_BLUE);
        }
        let caret_visible = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .is_ok_and(|t| t.as_millis() % 1000 < 500);
        if caret_visible && let Some(name) = self.catalog.editing {
            let value = match name {
                "name" => self.catalog.config.name.clone(),
                "server" => self.catalog.config.server.clone(),
                "render_scale" => self.catalog.render_scale_text.clone(),
                _ => format!("Search: {}", self.catalog.query),
            };
            for ([x, y, w, _], action) in &self.targets {
                if *action == Action::Field(name)
                    || (name == "render_scale" && *action == Action::RenderScale)
                {
                    let text = ui.fit_text(Face::Medium, 16.0, &value, w - 24.0);
                    let caret_x = x + 12.0 + ui.measure(Face::Medium, 16.0, &text);
                    ui.rounded(
                        caret_x.min(x + w - 10.0),
                        y + 12.0,
                        1.0,
                        18.0,
                        0.0,
                        color::TEXT,
                    );
                }
            }
        }
        if self.exit_confirmation {
            self.targets.clear();
            self.list_targets.clear();
            self.list_view = None;
            ui.rounded(0.0, 0.0, w, h, 0.0, horizon_client::hud::rgba(0, 0, 0, 170));
            let width = (w - 32.0).min(380.0);
            let x = (w - width) * 0.5;
            let y = (h - 168.0) * 0.5;
            ui.rounded(x - 3.0, y - 3.0, width + 6.0, 174.0, 25.0, style::GLOW_SOFT);
            ui.rounded(x, y + 6.0, width, 168.0, 22.0, style::SHADOW);
            ui.rounded(x, y, width, 168.0, 22.0, style::PANEL);
            ui.border(x, y, width, 168.0, 22.0, style::LINE);
            ui.rounded(x + 20.0, y + 20.0, 40.0, 40.0, 12.0, color::hex(0x97cff0, 40));
            ui.icon("arrow-left", x + 28.0, y + 28.0, 24.0, LAUNCH_BLUE);
            ui.text(
                Face::SemiBold,
                21.0,
                x + 74.0,
                y + 18.0,
                color::TEXT,
                "Minimize Horizon?",
            );
            ui.text(
                Face::Regular,
                13.5,
                x + 74.0,
                y + 46.0,
                style::TEXT_SOFT,
                &ui.fit_text(
                    Face::Regular,
                    13.5,
                    "The launcher keeps running in the background.",
                    width - 94.0,
                ),
            );
            let bw = (width - 52.0) * 0.5;
            button(
                &mut ui,
                &mut self.targets,
                [x + 20.0, y + 102.0, bw, 46.0],
                "Cancel",
                Action::Dismiss,
                false,
            );
            button(
                &mut ui,
                &mut self.targets,
                [x + 32.0 + bw, y + 102.0, bw, 46.0],
                "Minimize",
                Action::ConfirmMinimize,
                true,
            );
        }
        if let Some(action) = self.pending_focus.take() {
            self.focus = self.targets.iter().position(|(_, a)| *a == action);
        }
        if let Some(index) = self.focus
            && let Some(([x, y, w, h], _)) = self.targets.get(index)
        {
            let radius = (*h * 0.5).min(18.0) + 3.0;
            ui.border(*x - 3.0, *y - 3.0, *w + 6.0, *h + 6.0, radius, LAUNCH_BLUE);
            ui.border(*x - 4.0, *y - 4.0, *w + 8.0, *h + 8.0, radius + 1.0, style::GLOW);
            self.cursor = (
                f64::from((x + w / 2.0) * ui.density),
                f64::from((y + h / 2.0) * ui.density),
            );
        }
        if self.mouse_down || self.drag.as_ref().is_some_and(|drag| !drag.moved) {
            let point = [
                self.cursor.0 as f32 / ui.density,
                self.cursor.1 as f32 / ui.density,
            ];
            if let Some(([x, y, w, h], _)) = self.targets.iter().rev().find(|(rect, _)| {
                point[0] >= rect[0]
                    && point[0] <= rect[0] + rect[2]
                    && point[1] >= rect[1]
                    && point[1] <= rect[1] + rect[3]
            }) {
                ui.rounded(
                    *x,
                    *y,
                    *w,
                    *h,
                    (*h * 0.5).min(18.0),
                    horizon_client::hud::rgba(255, 255, 255, 44),
                );
            }
        }
        if !self.mouse_down
            && self.drag.is_none()
            && let Some((action, time)) = self.feedback
            && time.elapsed() < std::time::Duration::from_millis(160)
            && let Some(([x, y, w, h], _)) = self.targets.iter().find(|(_, a)| *a == action)
        {
            ui.rounded(
                *x,
                *y,
                *w,
                *h,
                (*h * 0.5).min(18.0),
                horizon_client::hud::rgba(255, 255, 255, 44),
            );
        }
        crate::documents::draw_ui(&mut ui);
        ui.finish();

        if !r.render(&game::loading_frame(&b))? && !b.patches.is_empty() {
            self.motion.invalidate();
            // A suspended/lost surface skips the frame, including its glyph
            // uploads. Rebuild them next time instead of caching invisible text.
            horizon_client::ui_theme::reset_atlas();
        }
        Ok(())
    }
    pub(super) fn activate(&mut self, events: &ActiveEventLoop) -> Result<()> {
        if self.session_worker.is_some() && self.chooser.is_none() && !self.exit_confirmation {
            return Ok(());
        }
        let Some(r) = &self.renderer else {
            return Ok(());
        };
        let mut b = HudBatch::new([r.config.width, r.config.height]);
        let ui = Ui::mobile(&self.gui.as_ref().unwrap().atlas, &mut b, (0.0, 0.0));
        let p = self.focus.and_then(|index| self.targets.get(index)).map_or(
            [
                self.cursor.0 as f32 / ui.density,
                self.cursor.1 as f32 / ui.density,
            ],
            |([x, y, w, h], _)| [x + w / 2.0, y + h / 2.0],
        );
        let action = self
            .targets
            .iter()
            .find(|([x, y, w, h], _)| {
                (*x..*x + *w).contains(&p[0]) && (*y..*y + *h).contains(&p[1])
            })
            .map(|(_, a)| *a);
        if self.catalog.busy() || crate::documents::busy() {
            return Ok(());
        }
        self.feedback = action.map(|action| (action, std::time::Instant::now()));
        self.focus = None;
        if action.is_some() && horizon_client::official_install::ready(&self.home) {
            let audio = self.audio.get_or_insert_with(|| {
                let mut audio = horizon_client::audio::Audio::new(
                    &self.home.join(".local/horizon-client/26.3"),
                );
                let settings = horizon_client::settings::Settings::load(
                    &self
                        .catalog
                        .config
                        .game_home(&self.home)
                        .join("options.txt"),
                );
                audio.master = settings.volumes[0];
                let mut volumes = settings.volumes;
                volumes[0] = 1.0;
                horizon_client::settings::set_source_volumes(volumes);
                audio
            });
            audio.play_button_click();
        }
        match action {
            Some(Action::ConfirmMinimize) => {
                self.exit_confirmation = false;
                self.focus = None;
                crate::platform::hide();
                crate::native_ime::minimize();
            }
            Some(Action::Immersive) => {
                self.catalog.config.screen.immersive ^= true;
                self.catalog.config.save(&self.home)?;
                self.configure_screen();
            }
            Some(Action::Cutout) => {
                self.catalog.config.screen.avoid_cutout ^= true;
                self.catalog.config.save(&self.home)?;
                self.configure_screen();
            }
            Some(Action::Aspect) => self.open_chooser(chooser::Kind::Aspect),
            Some(Action::ChooseAspect(aspect)) => {
                self.catalog.config.screen.aspect = aspect;
                self.catalog.config.save(&self.home)?;
                self.configure_screen();
                self.chooser = None;
            }
            Some(Action::CacheBudget) => self.open_chooser(chooser::Kind::CacheBudget),
            Some(Action::ChooseCacheBudget(mib)) => {
                self.catalog.config.cache_budget_mib = mib.min(1024);
                self.catalog.config.save(&self.home)?;
                self.chooser = None;
                self.catalog.status = "Disposable cache budget saved. Applies on next Play; not a total RAM or VRAM limit.".into();
            }
            Some(Action::ToggleBundled(id)) => {
                let game = self.catalog.config.game_home(&self.home);
                let mut list = horizon_client::mod_list::ModList::load(&game);
                let enabled = list.enabled.contains(id) && !list.disabled.contains(id);
                if enabled {
                    list.enabled.remove(id);
                    list.disabled.insert(id.into());
                } else {
                    list.disabled.remove(id);
                    list.enabled.insert(id.into());
                }
                std::fs::create_dir_all(&game)?;
                std::fs::write(game.join(horizon_client::mod_list::FILE), list.serialize())?;
                self.catalog.refresh(&self.home);
                self.catalog.status = format!(
                    "Bundled {id} {}. Applies on Play; settings are in Options > Mods.",
                    if enabled { "disabled" } else { "enabled" }
                );
            }
            Some(Action::GameFiles) => {
                self.catalog.status = if crate::platform::open_documents(crate::document_store::ROOT) {
                    "Game files opened. Worlds, mods, packs, shaders and reports are available here."
                } else { "Open system Files > Horizon Android." }.into();
            }
            Some(Action::WorldFiles) => {
                let id = crate::document_store::folder_id(self.catalog.config.profile, "worlds");
                self.catalog.status = if crate::platform::open_documents(&id) {
                    "Copy a complete world folder to back up or import. Finish copying before Play; never replace an existing world silently."
                } else { "Open system Files > Horizon Android > Worlds. Copy complete world folders while the game is closed." }.into();
            }
            Some(Action::WorkerLimit) => self.open_chooser(chooser::Kind::WorkerLimit),
            Some(Action::ChooseWorkerLimit(limit)) => {
                self.catalog.config.worker_limit = limit.min(64);
                self.catalog.config.save(&self.home)?;
                self.chooser = None;
                self.catalog.status = "Chunk worker ceiling saved. Applies on the next Play; other engine threads are unchanged.".into();
            }
            Some(Action::DisplayHz) => self.open_chooser(chooser::Kind::DisplayHz),
            Some(Action::ChooseDisplayHz(hz)) => {
                self.catalog.config.device.display_hz = hz;
                self.catalog.config.device.sanitize();
                self.catalog.config.save(&self.home)?;
                crate::device_runtime::configure(self.catalog.config.device);
                self.chooser = None;
            }
            Some(Action::PerformanceHints) => {
                self.catalog.config.device.performance_hints ^= true;
                self.catalog.status="ADPF gives Android CPU frame-work hints on supported devices. It does not cap FPS or guarantee a speedup.".into();
                self.catalog.config.save(&self.home)?;
                crate::device_runtime::configure(self.catalog.config.device);
            }
            Some(Action::GpuProfile) => {
                self.catalog.config.device.gpu_profile ^= true;
                self.catalog.status="Detailed GPU profiling records diagnostics and has overhead. Reports remain on this device.".into();
                self.catalog.config.save(&self.home)?;
                crate::device_runtime::configure(self.catalog.config.device);
            }

            Some(Action::ModSource(origin)) => {
                self.catalog.config.mod_source = origin;
                self.catalog.config.save(&self.home)?;
                self.catalog.offset = 0;
                self.catalog.browse(&self.home);
            }
            Some(Action::ShaderSource(origin)) => {
                self.catalog.config.shader_source = origin;
                self.catalog.config.save(&self.home)?;
                self.catalog.offset = 0;
                self.catalog.browse(&self.home);
            }
            Some(Action::OpenFolder(kind)) => {
                crate::platform::hide();
                let folder = match kind {
                    horizon_client::content_files::Kind::Mods => "mods",
                    horizon_client::content_files::Kind::Packs => "packs",
                    horizon_client::content_files::Kind::Shaders => "shaders",
                };
                let id = crate::document_store::folder_id(self.catalog.config.profile, folder);
                self.catalog.status = if crate::platform::open_documents(&id) {
                    "Opened in Files. Return here after copying files.".into()
                } else {
                    "Open the system Files app and choose Horizon Android.".into()
                };
            }
            Some(Action::Import(kind)) => {
                crate::platform::hide();
                let home = self.catalog.config.game_home(&self.home);
                if let Err(e) = crate::documents::begin(kind, home) {
                    self.catalog.status = e;
                }
            }
            Some(Action::ToggleSection(i)) => {
                self.catalog.sections[i] = !self.catalog.sections[i];
                self.catalog.scroll = 0.0;
            }
            Some(Action::ToggleLocal(i)) => self.catalog.toggle_local(&self.home, i),
            Some(Action::DeleteLocal(i)) => {
                self.catalog.pending_delete = self.catalog.local.get(i).cloned();
                if let Some(item) = &self.catalog.pending_delete {
                    self.chooser = Some(super::chooser::Chooser::confirm_delete(item.file.clone()));
                }
            }
            Some(Action::ConfirmDeleteLocal) => {
                self.catalog.remove_local(&self.home);
                self.chooser = None;
            }
            Some(Action::Repository) => {
                self.catalog.status = if crate::platform::open_repository() {
                    "Repository opened in browser."
                } else {
                    "No browser available to open the repository."
                }
                .into();
            }
            Some(Action::Account) => {
                self.catalog.editing = None;
                crate::platform::hide();
                self.login.providers = true;
                self.login.offline = false;
                self.login.ely = false;
                self.login.choosing = false;
                self.login.visible = true;
            }
            Some(Action::OfflineAccount | Action::MicrosoftAccount) => {
                self.login.providers = false;
                self.login.offline = matches!(action, Some(Action::OfflineAccount));
                self.login.ely = false;
                self.login.choosing = false;
                self.login.visible = true;
            }
            Some(Action::SavedAccounts) => {
                self.login.providers = false;
                self.login.refresh_status();
                self.login.choosing = true;
                self.login.ely = false;
            }
            Some(Action::SavedPage(next)) => {
                if next && self.login.account_offset + 5 < self.login.saved_accounts.len() {
                    self.login.account_offset += 5;
                } else if !next {
                    self.login.account_offset = self.login.account_offset.saturating_sub(5);
                }
            }
            Some(Action::SelectAccount(index)) => {
                let mut accounts = crate::account::Accounts::load();
                if let Some(account) = self.login.saved_accounts.get(index)
                    && accounts.accounts.iter().any(|a| a.key() == account.key())
                {
                    accounts.selected = Some(account.key());
                    accounts.save()?;
                }
                self.session_ready = false;
                self.prepared_session = None;
                self.session_worker = None;
                self.login.refresh_status();
                if self.catalog.page == Page::Skins {
                    self.wardrobe.open(
                        &self.catalog.config.name,
                        &self.catalog.config.microsoft_client_id,
                    );
                }
            }
            Some(Action::SelectOffline) => {
                self.catalog.config.save(&self.home)?;
                self.catalog.editing = None;
                crate::platform::hide();
                self.login.hide();
                let mut accounts = crate::account::Accounts::load();
                accounts.selected = None;
                accounts.save()?;
                self.session_ready = false;
                self.prepared_session = None;
                self.session_worker = None;
                self.login.choosing = false;
                self.login.refresh_status();
                if self.catalog.page == Page::Skins {
                    self.wardrobe.open(
                        &self.catalog.config.name,
                        &self.catalog.config.microsoft_client_id,
                    );
                }
            }
            Some(Action::ElySignIn) => {
                self.login.providers = false;
                self.login.close();
                self.login.ely = true;
                self.login.visible = true;
            }
            Some(Action::SubmitEly) => {
                self.catalog.editing = None;
                crate::platform::hide();
                self.session_ready = false;
                self.prepared_session = None;
                self.session_worker = None;
                if !self.login.pending() {
                    self.login.start_ely();
                }
            }
            Some(Action::CloseLogin) => {
                self.login.hide();
                crate::platform::hide();
                self.catalog.editing = None;
                self.refresh_wardrobe_account();
            }
            Some(Action::DeviceSignIn) => {
                self.session_ready = false;
                self.prepared_session = None;
                self.session_worker = None;
                self.login
                    .start_device(&self.catalog.config.microsoft_client_id);
            }
            Some(Action::CopyLoginCode) => self.login.copy_code(),
            Some(Action::CancelLogin) => {
                self.login.close();
                self.login.visible = true;
            }
            Some(Action::ReviewApp) => self.login.review_app(),
            Some(Action::SignIn) => {
                self.session_ready = false;
                self.prepared_session = None;
                self.session_worker = None;
                self.login.start(&self.catalog.config.microsoft_client_id);
            }
            Some(Action::SignOut) => {
                self.session_ready = false;
                self.prepared_session = None;
                self.session_worker = None;
                self.login.sign_out();
                self.refresh_wardrobe_account();
            }
            Some(Action::LoginLink) => self.login.open_link(),
            Some(Action::Back) => self.request_back(),
            Some(Action::SettingsSection(section)) => {
                self.settings_section = section;
                self.catalog.scroll = 0.0;
                self.catalog.editing = None;
                crate::platform::hide();
            }
            Some(Action::Page(page)) => {
                if page == Page::Skins {
                    self.wardrobe.open(
                        &self.catalog.config.name,
                        &self.catalog.config.microsoft_client_id,
                    );
                }
                self.catalog.editing = None;
                if crate::platform::active() {
                    crate::platform::hide();
                }
                self.catalog.page = page;
                self.catalog.scroll = 0.0;
                self.error = None;
                self.catalog.offset = 0;
                self.catalog.browse(&self.home);
            }
            Some(Action::Play) => self.action(events)?,
            Some(Action::WardrobeKind(cape)) => {
                self.wardrobe.capes = cape;
                self.catalog.scroll = 0.0;
            }
            Some(Action::WardrobeSelect(index)) => self.wardrobe.select(index),
            Some(Action::WardrobeImport) => self.wardrobe.import(),
            Some(Action::WardrobeCloud) => self
                .wardrobe
                .cloud(&self.catalog.config.microsoft_client_id, None),
            Some(Action::WardrobeUpload) => self.wardrobe.cloud(
                &self.catalog.config.microsoft_client_id,
                Some(wardrobe::CloudAction::Upload),
            ),
            Some(Action::WardrobeActivateCape(index)) => self.wardrobe.cloud(
                &self.catalog.config.microsoft_client_id,
                Some(wardrobe::CloudAction::Cape(index)),
            ),
            Some(Action::WardrobeUseCloud) => self.wardrobe.use_cloud(),
            Some(Action::WardrobeModel) => self.wardrobe.model(),
            Some(Action::WardrobeRotate(direction)) => {
                self.wardrobe.camera.rotate(direction as f32 * 25.0, 0.0);
                self.wardrobe.preview();
            }
            Some(Action::WardrobeZoom(inward)) => {
                self.wardrobe
                    .camera
                    .zoom(if inward { 1.15 } else { 1.0 / 1.15 });
                self.wardrobe.preview();
            }
            Some(Action::WardrobeHideCape) => self.wardrobe.hide_cape(),
            Some(Action::Search) => {
                crate::platform::hide();
                self.catalog.offset = 0;
                self.catalog.search(&self.home);
            }
            Some(Action::FinishNameEdit) => {
                if self.catalog.editing == Some("name") {
                    self.refresh_wardrobe_account();
                }
                self.catalog.editing = None;
                crate::platform::hide();
                self.catalog.config.save(&self.home)?;
            }
            Some(Action::Field(name)) => {
                if let Some(value) = self.login.field(name) {
                    self.catalog.editing = Some(name);
                    if name == "ely_password" || name == "ely_otp" {
                        crate::platform::show_secret(value);
                    } else {
                        crate::platform::show(value);
                    }
                    return Ok(());
                }
                if name == "render_scale" {
                    self.catalog.render_scale_text =
                        self.catalog.config.render_scale.percent().to_string();
                    self.chooser = None;
                }
                self.catalog.editing = Some(name);
                let initial = match name {
                    "name" => &self.catalog.config.name,
                    "server" => &self.catalog.config.server,
                    "render_scale" => &self.catalog.render_scale_text,
                    "client_id" => &self.catalog.config.microsoft_client_id,
                    _ => &self.catalog.query,
                };
                crate::platform::show(initial);
            }
            Some(Action::Install(i)) => self.catalog.install(&self.home, i),
            Some(Action::Next) => {
                self.catalog.offset += 12;
                self.catalog.search(&self.home);
            }
            Some(Action::Previous) => {
                self.catalog.offset = self.catalog.offset.saturating_sub(12);
                self.catalog.search(&self.home);
            }
            Some(Action::Profile) => self.open_chooser(chooser::Kind::Profile),
            Some(Action::Renderer) => self.open_chooser(chooser::Kind::Renderer),
            Some(Action::RenderScale) => self.open_chooser(chooser::Kind::RenderScale),
            Some(Action::ChooseRenderScale(scale)) => {
                self.catalog.config.render_scale = scale;
                self.catalog.config.save(&self.home)?;
                self.chooser = None;
                crate::display::set_scale(scale);
            }
            Some(Action::Orientation) => self.open_chooser(chooser::Kind::Orientation),
            Some(Action::InputHost) => self.open_chooser(chooser::Kind::InputHost),
            Some(Action::ChooseInputHost(mode)) => {
                self.catalog.config.input_host = mode;
                self.catalog.config.save(&self.home)?;
                self.chooser = None;
                crate::configure_input_mode(mode);
            }
            Some(Action::ChooseOrientation(mode)) => {
                self.catalog.config.orientation = mode;
                self.catalog.config.save(&self.home)?;
                self.chooser = None;
                crate::platform::set_orientation(mode);
            }
            Some(Action::ChooseRenderer(mode)) => {
                self.catalog.config.renderer = mode;
                self.catalog.config.save(&self.home)?;
                self.chooser = None;
            }
            Some(Action::ChooseProfile(profile)) => {
                std::fs::create_dir_all(if profile == 0 {
                    self.home.clone()
                } else {
                    self.home
                        .join("profiles")
                        .join(format!("profile-{profile}"))
                })?;
                self.catalog.config.profile = profile;
                self.catalog.config.save(&self.home)?;
                self.catalog.refresh(&self.home);
                self.chooser = None;
            }
            Some(Action::Dismiss) => {
                self.exit_confirmation = false;
                self.chooser = None;
                if self.session_worker.is_none() {
                    self.session_ready = false;
                    self.prepared_session = None;
                    self.session_worker = None;
                }
            }
            Some(Action::Logs) => {
                self.catalog.status = crate::diagnostics::summary(&self.home);
                if let Ok(error) = std::fs::read_to_string(self.home.join("android-error.txt")) {
                    self.catalog.status = format!(
                        "Last handled error: {}. {}",
                        crate::diagnostics::redact(&error),
                        self.catalog.status
                    );
                }
            }
            Some(Action::ExportDiagnostics) if self.diagnostic_export.is_none() => {
                let home = self.home.clone();
                let (sender, receiver) = mpsc::channel();
                self.diagnostic_export = Some(receiver);
                self.catalog.status =
                    "Preparing local diagnostic ZIP. No upload; review before sharing.".into();
                std::thread::spawn(move || {
                    let result = crate::diagnostics::export(&home).map(|p| {
                        p.file_name()
                            .unwrap_or_default()
                            .to_string_lossy()
                            .into_owned()
                    });
                    let _ = sender.send(result);
                });
            }
            Some(Action::ExportDiagnostics) => {}
            Some(Action::RecordPerformance) => {
                self.catalog.config.record_performance = !self.catalog.config.record_performance;
                if !self.catalog.config.record_performance {
                    crate::performance::stop();
                }
                self.catalog.config.save(&self.home)?;
                self.catalog.status = if self.catalog.config.record_performance {
                    "Recording starts on Play. Detailed timings add overhead. Return here to save the report."
                } else { "Performance recording off." }.into();
            }
            Some(Action::ExportPerformance) => {
                self.catalog.status = match crate::performance::export(&self.home) {
                    Ok(()) => "Reports opened in Files. Copy or share a report from there.".into(),
                    Err(e) => e,
                };
            }
            None => {}
        }
        Ok(())
    }
    pub(super) fn ui_density(&self) -> f32 {
        let (Some(r), Some(gui)) = (&self.renderer, &self.gui) else {
            return 1.0;
        };
        let mut batch = HudBatch::new([r.config.width, r.config.height]);
        Ui::mobile(&gui.atlas, &mut batch, (0.0, 0.0)).density
    }

    pub(super) fn touch_event(&mut self, events: &ActiveEventLoop, t: Touch) -> Result<()> {
        if self.catalog.page == Page::Skins
            && !self.login.visible
            && self.wardrobe.gesture(&t, self.ui_density())
        {
            if let Some(window) = &self.window {
                window.request_redraw();
            }
            return Ok(());
        }
        self.focus = None;
        self.pending_focus = None;
        if let Some(window) = &self.window {
            window.request_redraw();
        }
        self.cursor = (t.location.x, t.location.y);
        match t.phase {
            TouchPhase::Started => {
                self.drag = Some(super::gesture::Drag::new(t.location.x, t.location.y));
            }
            TouchPhase::Moved => {
                let density = self.ui_density() as f64;
                if let Some(drag) = &mut self.drag {
                    let delta = drag.update(t.location.x, t.location.y, density);
                    if drag.moved {
                        if let Some(chooser) = &mut self.chooser {
                            chooser.scroll = (chooser.scroll + delta).max(0.0);
                        } else if self
                            .list_view
                            .is_some_and(|view| drag.origin_y() / density >= view[0] as f64)
                        {
                            self.catalog.scroll = (self.catalog.scroll + delta).max(0.0);
                        }
                    }
                }
            }
            TouchPhase::Ended => {
                if self.drag.take().is_some_and(|drag| !drag.moved) {
                    self.activate(events)?;
                }
            }
            TouchPhase::Cancelled => {
                self.drag = None;
            }
        }
        Ok(())
    }
    pub(super) fn type_key(&mut self, key: KeyEvent) {
        if key.physical_key == PhysicalKey::Code(KeyCode::Escape)
            || key.logical_key
                == winit::keyboard::Key::Named(winit::keyboard::NamedKey::BrowserBack)
        {
            if !key.repeat {
                self.request_back();
            }
            return;
        }
        if self.catalog.editing.is_none() {
            return;
        }
        let out = match key.physical_key {
            PhysicalKey::Code(KeyCode::Backspace) => Some(Output::Backspace),
            PhysicalKey::Code(KeyCode::Enter) => {
                crate::platform::hide();
                Some(Output::Enter)
            }
            _ => key.text.map(|s| Output::Text(s.into())),
        };
        if let Some(out) = out {
            self.edit_text(out);
        }
    }
}
