//! Convert a ZalithLauncher2 export into an editable native GoyControl preset.
use super::{Control, ControlStyle, CustomButton, Mode, Placement, Settings};
use serde::Deserialize;
use std::collections::BTreeMap;

pub const MAX_BYTES: usize = 4 * 1024 * 1024;

#[derive(Clone, Copy)]
pub struct Screen {
    pub width: u32,
    pub height: u32,
    pub density: f32,
}

pub struct Import {
    pub preset: Settings,
    pub buttons: usize,
    pub joystick: bool,
    pub skipped: usize,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Layout {
    editor_version: u32,
    info: Info,
    layers: Vec<Layer>,
    #[serde(default)]
    styles: Vec<Style>,
}

#[derive(Deserialize)]
struct Info {
    name: Label,
}

#[derive(Default, Deserialize)]
struct Label {
    default: String,
}

#[derive(Clone, Copy, Default, Deserialize)]
#[serde(rename_all = "snake_case")]
enum Visibility {
    #[default]
    Always,
    InGame,
    InMenu,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Layer {
    #[serde(default)]
    hide: bool,
    #[serde(default)]
    visibility_type: Visibility,
    #[serde(default)]
    normal_buttons: Vec<Button>,
    #[serde(default)]
    joystick_buttons: Vec<Joystick>,
    #[serde(default)]
    text_boxes: Vec<serde_json::Value>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Button {
    #[serde(default)]
    text: Label,
    position: Position,
    button_size: Size,
    button_style: Option<String>,
    #[serde(default)]
    visibility_type: Visibility,
    #[serde(default)]
    click_events: Vec<Event>,
    #[serde(default)]
    is_swipple: bool,
    #[serde(default)]
    is_penetrable: bool,
    #[serde(default)]
    is_toggleable: bool,
}

#[derive(Deserialize)]
struct Event {
    #[serde(rename = "type")]
    kind: String,
    key: String,
}

#[derive(Clone, Copy, Deserialize)]
struct Position {
    x: i32,
    y: i32,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Size {
    #[serde(rename = "type")]
    kind: String,
    #[serde(default)]
    width_dp: f32,
    #[serde(default)]
    height_dp: f32,
    #[serde(default)]
    width_percentage: i32,
    #[serde(default)]
    height_percentage: i32,
    #[serde(default)]
    width_reference: String,
    #[serde(default)]
    height_reference: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Style {
    uuid: String,
    #[serde(default)]
    common_style: Option<bool>,
    light_style: Option<Palette>,
    dark_style: Option<Palette>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Palette {
    alpha: Option<f32>,
    background_color: Option<i64>,
    pressed_background_color: Option<i64>,
    border_color: Option<i64>,
    border_width: Option<f32>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Joystick {
    position: Position,
    size_type: Option<String>,
    size_dp: Option<f32>,
    size_percentage: Option<i32>,
    #[serde(default)]
    visibility_type: Visibility,
    direction_events: Option<BTreeMap<String, Vec<Event>>>,
    lock_events: Option<Vec<Event>>,
}

fn axes(layer: Visibility, element: Visibility) -> (bool, bool) {
    (
        !matches!(layer, Visibility::InMenu) && !matches!(element, Visibility::InMenu),
        !matches!(layer, Visibility::InGame) && !matches!(element, Visibility::InGame),
    )
}

fn placement(screen: Screen, position: Position, width: f32, height: f32) -> Placement {
    let screen_w = screen.width.max(1) as f32;
    let screen_h = screen.height.max(1) as f32;
    let width = width.clamp(1.0, screen_w);
    let height = height.clamp(1.0, screen_h);
    let center = |position: i32, size: f32, full: f32| {
        let anchor = (position as f32 / 10_000.0).clamp(0.0, 1.0);
        ((full - size) * anchor + size * 0.5) / full
    };
    let size = (height / screen_h).clamp(0.045, 0.45);
    let mut p = Placement::new(
        center(position.x, width, screen_w),
        center(position.y, height, screen_h),
        size,
    );
    p.width_ratio = width / (size * screen_h);
    p.height_ratio = height / (size * screen_h);
    p
}

fn binding(event: &Event) -> Option<String> {
    if !matches!(event.kind.as_str(), "key" | "launcher_event") {
        return None;
    }
    let token = match event.key.as_str() {
        "GLFW_MOUSE_BUTTON_LEFT" | "GLFW_MOUSE_BUTTON_1" => "key.mouse.left",
        "GLFW_MOUSE_BUTTON_RIGHT" | "GLFW_MOUSE_BUTTON_2" => "key.mouse.right",
        "GLFW_MOUSE_BUTTON_MIDDLE" | "GLFW_MOUSE_BUTTON_3" => "key.mouse.middle",
        "GLFW_MOUSE_BUTTON_4" => "key.mouse.4",
        "GLFW_MOUSE_BUTTON_5" => "key.mouse.5",
        "launcher.event.scroll_up" | "launcher.event.scroll_up.single" => "mouse.scroll.up",
        "launcher.event.scroll_down" | "launcher.event.scroll_down.single" => "mouse.scroll.down",
        "launcher.event.switch_ime" => "keyboard.toggle",
        key => {
            let key = key.strip_prefix("GLFW_KEY_")?;
            let suffix = match key {
                "LEFT_SUPER" => "left.win".into(),
                "RIGHT_SUPER" => "right.win".into(),
                "KP_DECIMAL" => "keypad.period".into(),
                key if key.starts_with("KP_") => format!("keypad.{}", key[3..].to_ascii_lowercase()),
                key => key.to_ascii_lowercase().replace('_', "."),
            };
            let token = format!("key.keyboard.{suffix}");
            return crate::keybinds::key_code(&token).map(|_| token);
        }
    };
    Some(token.into())
}

fn joystick_supported(stick: &Joystick) -> bool {
    const WASD: [(&str, &[&str]); 8] = [
        ("north", &["GLFW_KEY_W"]),
        ("south", &["GLFW_KEY_S"]),
        ("west", &["GLFW_KEY_A"]),
        ("east", &["GLFW_KEY_D"]),
        ("north_east", &["GLFW_KEY_W", "GLFW_KEY_D"]),
        ("north_west", &["GLFW_KEY_W", "GLFW_KEY_A"]),
        ("south_east", &["GLFW_KEY_S", "GLFW_KEY_D"]),
        ("south_west", &["GLFW_KEY_S", "GLFW_KEY_A"]),
    ];
    stick.lock_events.as_ref().is_none_or(|events| {
        events.iter().all(|event| event.kind == "key" && event.key == "GLFW_KEY_LEFT_CONTROL")
    }) && stick.direction_events.as_ref().is_none_or(|directions| {
        WASD.iter().all(|(direction, expected)| {
            directions.get(*direction).is_some_and(|events| {
                events.len() == expected.len()
                    && expected.iter().all(|key| events.iter().any(|event| {
                        event.kind == "key" && event.key == *key
                    }))
            })
        }) && directions.get("none").is_none_or(Vec::is_empty)
    })
}

fn clean_label(label: &str) -> String {
    label
        .trim()
        .chars()
        .filter(|character| !character.is_control())
        .take(40)
        .collect()
}

pub fn convert(bytes: &[u8], screen: Screen) -> Result<Import, String> {
    if bytes.len() > MAX_BYTES {
        return Err("Zalith layout exceeds 4 MiB".into());
    }
    let layout: Layout = serde_json::from_slice(bytes)
        .map_err(|error| format!("Invalid ZalithLauncher2 JSON: {error}"))?;
    if !(1..=12).contains(&layout.editor_version) {
        return Err(format!(
            "Unsupported Zalith editor version {}",
            layout.editor_version,
        ));
    }
    let mut preset = Settings::default_for_style(ControlStyle::GoyControl);
    preset.preset_id = "zalith-import".into();
    preset.preset_name = clean_label(&layout.info.name.default);
    if preset.preset_name.is_empty() {
        preset.preset_name = "Zalith Layout".into();
    } else if preset.preset_name.eq_ignore_ascii_case("default") {
        preset.preset_name = "Zalith Default".into();
    }
    preset.preset_description = "Imported from ZalithLauncher2".into();
    let native_mouse = preset
        .custom_buttons
        .drain(..)
        .find(|button| button.key == "mouse.virtual.toggle");
    let button_limit = preset.control_style.custom_button_limit();

    let mut joystick = false;
    let mut skipped = 0;
    let height = screen.height.max(1) as f32;
    let density = screen.density.clamp(0.5, 10.0);
    for layer in layout.layers {
        skipped += layer.text_boxes.len();
        if layer.hide {
            skipped += layer.normal_buttons.len() + layer.joystick_buttons.len();
            continue;
        }
        for button in layer.normal_buttons {
            let (in_game, in_menu) = axes(layer.visibility_type, button.visibility_type);
            if (!in_game && !in_menu) || preset.custom_buttons.len() >= button_limit {
                skipped += 1;
                continue;
            }
            let mut tokens = Vec::new();
            for event in &button.click_events {
                match binding(event) {
                    Some(token) if tokens.contains(&token) => {}
                    Some(token) if tokens.len() < 3 => tokens.push(token),
                    _ => skipped += 1,
                }
            }
            if tokens.is_empty() {
                if button.click_events.is_empty() {
                    skipped += 1;
                }
                continue;
            }
            let size = &button.button_size;
            let reference = |axis: &str| {
                if axis == "screen_width" {
                    screen.width.max(1) as f32
                } else {
                    height
                }
            };
            let (width, button_height) = match size.kind.as_str() {
                "percentage" => (
                    reference(&size.width_reference) * size.width_percentage as f32 / 10_000.0,
                    reference(&size.height_reference) * size.height_percentage as f32 / 10_000.0,
                ),
                "dp" => (size.width_dp * density, size.height_dp * density),
                "wrap_content" => {
                    let chars = button.text.default.chars().count().min(40) as f32;
                    let label_width = (chars * 8.0 + 20.0).max(36.0);
                    (label_width * density, 40.0 * density)
                }
                _ => {
                    skipped += 1;
                    continue;
                }
            };
            let mut p = placement(screen, button.position, width, button_height);
            p.visible_in_game = in_game;
            p.visible_in_menu = in_menu;
            p.passthrough = button.is_penetrable;
            p.toggleable = button.is_toggleable;
            p.swipeable = button.is_swipple;
            p.use_goy_style = true;
            if let Some(style) = layout
                .styles
                .iter()
                .find(|style| Some(&style.uuid) == button.button_style.as_ref())
            {
                let palette = if style.common_style.unwrap_or(true) {
                    style.light_style.as_ref()
                } else {
                    style.dark_style.as_ref().or(style.light_style.as_ref())
                };
                if let Some(palette) = palette {
                    p.opacity = palette.alpha.unwrap_or(1.0);
                    p.button_color = palette.background_color.map_or(p.button_color, |c| c as u32);
                    p.on_click_color = palette
                        .pressed_background_color
                        .map_or(p.on_click_color, |c| c as u32);
                    p.stroke_color = palette.border_color.map_or(p.stroke_color, |c| c as u32);
                    p.stroke_size = palette.border_width.unwrap_or(p.stroke_size);
                }
            }
            let mut tokens = tokens.into_iter();
            let mut custom = CustomButton::new(tokens.next().unwrap(), preset.custom_buttons.len());
            custom.second_key = tokens.next().unwrap_or_default();
            custom.third_key = tokens.next().unwrap_or_default();
            custom.custom_label = clean_label(&button.text.default);
            custom.always_visible = in_menu;
            custom.layouts = [p; 3];
            preset.custom_buttons.push(custom);
        }
        for stick in layer.joystick_buttons {
            if joystick
                || !axes(layer.visibility_type, stick.visibility_type).0
                || !joystick_supported(&stick)
            {
                skipped += 1;
                continue;
            }
            let side = match stick.size_type.as_deref() {
                None | Some("percentage") => {
                    height * stick.size_percentage.unwrap_or(2500).max(2000) as f32 / 10_000.0
                }
                Some("dp") => stick.size_dp.unwrap_or(200.0).max(20.0) * density,
                Some("wrap_content") => 200.0 * density,
                _ => {
                    skipped += 1;
                    continue;
                }
            };
            let mut p = placement(screen, stick.position, side, side);
            // The native movement control is square and handles only world input.
            p.width_ratio = 1.0;
            p.height_ratio = 1.0;
            for slots in &mut preset.layouts {
                slots[Control::Move as usize] = p;
            }
            preset.mode = Mode::Crosshair;
            joystick = true;
        }
    }

    let buttons = preset.custom_buttons.len();
    if buttons == 0 && !joystick {
        return Err("This layout has no controls supported by the native editor".into());
    }
    preset.button_visibility[Control::Move as usize] = if joystick { 1 } else { 2 };
    preset.sanitize();
    if buttons < button_limit
        && let Some(mut mouse) = native_mouse
    {
        let aspect = screen.width.max(1) as f32 / screen.height.max(1) as f32;
        let occupied: Vec<_> = preset
            .custom_buttons
            .iter()
            .map(|button| button.layouts[preset.mode.index()].rect(aspect))
            .chain(joystick.then(|| preset.placement(Control::Move).rect(aspect)))
            .collect();
        // Keep the native cursor toggle clear of imported controls and the top toolbar.
        let centers = [0.18, 0.30, 0.42, 0.54, 0.66, 0.78].into_iter().flat_map(|y| {
            [0.60, 0.36, 0.72, 0.24, 0.84, 0.12, 0.50]
                .into_iter()
                .map(move |x| [x, y])
        });
        let mut p = mouse.layouts[0];
        for center in centers {
            p.center = center;
            let [x, y, w, h] = p.rect(aspect);
            if x >= 0.0
                && x + w <= 1.0
                && !occupied.iter().any(|&[ox, oy, ow, oh]| {
                    x <= ox + ow && x + w >= ox && y <= oy + oh && y + h >= oy
                })
            {
                mouse.layouts = [p; 3];
                preset.custom_buttons.push(mouse);
                break;
            }
        }
    }
    Ok(Import {
        preset,
        buttons,
        joystick,
        skipped,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const SCREEN: Screen = Screen {
        width: 1600,
        height: 800,
        density: 2.0,
    };

    #[test]
    fn zalith_scroll_buttons_move_the_hotbar_in_opposite_directions() {
        use crate::{mouse::ScrollWheelHandler, touch::custom::scroll_wheel_delta};

        for (key, expected, wrapped) in [
            ("launcher.event.scroll_up", 3, (0, 8)),
            ("launcher.event.scroll_up.single", 3, (0, 8)),
            ("launcher.event.scroll_down", 5, (8, 0)),
            ("launcher.event.scroll_down.single", 5, (8, 0)),
        ] {
            let token = binding(&Event {
                kind: "launcher_event".into(),
                key: key.into(),
            })
            .unwrap();
            let delta = scroll_wheel_delta(&token).unwrap();
            assert_eq!(ScrollWheelHandler::next_selection(delta, 4, 9), expected);
            assert_eq!(
                ScrollWheelHandler::next_selection(delta, wrapped.0, 9),
                wrapped.1
            );
        }
    }

    #[test]
    fn imports_bindings_and_geometry_as_native_controls() {
        let bytes = br#"{
            "editorVersion":12,
            "info":{"name":{"default":"My HUD"}},
            "styles":[{
                "uuid":"accent",
                "lightStyle":{"alpha":0.5,"backgroundColor":-16711936}
            }],
            "layers":[{
                "visibilityType":"in_game",
                "normalButtons":[{
                    "text":{"default":"Use"},
                    "position":{"x":10000,"y":5000},
                    "buttonSize":{
                        "type":"percentage","widthPercentage":1000,"heightPercentage":1000,
                        "widthReference":"screen_width","heightReference":"screen_height"
                    },
                    "buttonStyle":"accent",
                    "isPenetrable":true,
                    "clickEvents":[
                        {"type":"launcher_event","key":"GLFW_MOUSE_BUTTON_RIGHT"},
                        {"type":"key","key":"GLFW_KEY_LEFT_SHIFT"},
                        {"type":"switch_layer","key":"another-layer"}
                    ]
                }],
                "joystickButtons":[{"position":{"x":0,"y":10000},"sizePercentage":2000}]
            }]
        }"#;
        let imported = convert(bytes, SCREEN).unwrap();
        assert_eq!(imported.preset.preset_name, "My HUD");
        assert_eq!(imported.preset.control_style, ControlStyle::GoyControl);
        assert_eq!(imported.buttons, 1);
        assert_eq!(imported.skipped, 1);
        assert!(imported.joystick);
        assert!(imported.preset.visible(Control::Move));
        let button = &imported.preset.custom_buttons[0];
        assert_eq!(button.active_tokens(), ["key.mouse.right", "key.keyboard.left.shift"]);
        let p = button.layouts[0];
        assert!(p.visible_in_game && !p.visible_in_menu && p.passthrough);
        assert!((p.center[0] - 0.95).abs() < 1e-6);
        assert_eq!(p.center[1], 0.5);
        assert_eq!(p.width_ratio, 2.0);
        assert_eq!(p.opacity, 0.5);
        assert_eq!(p.button_color, 0xff00ff00);
        let joystick = imported.preset.layouts[0][Control::Move as usize];
        assert!((joystick.center[0] - 0.05).abs() < 1e-6);
        assert!((joystick.center[1] - 0.9).abs() < 1e-6);
    }

    #[test]
    fn rejects_non_zalith_and_layouts_with_no_supported_controls() {
        for bytes in [
            br#"{"custom_buttons":[]}"#.as_slice(),
            br#"{"editorVersion":12,"info":{"name":{"default":"Empty"}},"layers":[]}"#,
        ] {
            assert!(convert(bytes, SCREEN).is_err());
        }
    }

    #[test]
    fn layout_without_a_joystick_does_not_add_the_native_analog() {
        let bytes = br#"{
            "editorVersion":12,
            "info":{"name":{"default":"D-pad"}},
            "layers":[{"normalButtons":[{
                "position":{"x":1500,"y":7000},
                "buttonSize":{"type":"dp","widthDp":60,"heightDp":60},
                "clickEvents":[{"type":"key","key":"GLFW_KEY_W"}]
            }]}]
        }"#;
        let imported = convert(bytes, SCREEN).unwrap();
        assert!(!imported.joystick);
        assert!(!imported.preset.visible(Control::Move));
        assert_eq!(imported.preset.custom_buttons[0].key, "key.keyboard.w");
        let mut controls = crate::touch::TouchControls::default();
        controls.settings = imported.preset;
        controls.open_editor();
        let editor = controls.editor.unwrap();
        assert_eq!(editor.selected, None);
        assert!(!editor.show_hidden);
    }

    #[test]
    fn imports_open_keyboard_after_the_first_32_buttons() {
        let mut layout: serde_json::Value = serde_json::from_str(
            r#"{
            "editorVersion":12,
            "info":{"name":{"default":"Keyboard"}},
            "layers":[{"normalButtons":[{
                "text":{"default":"Keyboard"},
                "position":{"x":8500,"y":1500},
                "buttonSize":{"type":"dp","widthDp":60,"heightDp":35},
                "clickEvents":[{"type":"launcher_event","key":"launcher.event.switch_ime"}]
            }]}]
        }"#,
        )
        .unwrap();
        let buttons = layout["layers"][0]["normalButtons"].as_array_mut().unwrap();
        let keyboard = buttons.pop().unwrap();
        let mut key = keyboard.clone();
        key["clickEvents"][0] = serde_json::json!({"type":"key","key":"GLFW_KEY_W"});
        buttons.resize(32, key);
        buttons.push(keyboard);

        let imported = convert(&serde_json::to_vec(&layout).unwrap(), SCREEN).unwrap();
        assert_eq!(imported.buttons, 33);
        assert_eq!(imported.skipped, 0);
        let button = &imported.preset.custom_buttons[32];
        assert_eq!(button.active_tokens(), ["keyboard.toggle"]);
        assert_eq!(button.label(), "Keyboard");
    }

    #[test]
    fn imported_layout_can_toggle_the_native_virtual_mouse() {
        let bytes = br#"{
            "editorVersion":12,
            "info":{"name":{"default":"Layout"}},
            "layers":[{"normalButtons":[{
                "position":{"x":6000,"y":1200},
                "buttonSize":{"type":"dp","widthDp":240,"heightDp":80},
                "clickEvents":[{"type":"key","key":"GLFW_KEY_W"}]
            }]}]
        }"#;
        let imported = convert(bytes, SCREEN).unwrap();
        let mut controls = crate::touch::TouchControls::default();
        controls.viewport(SCREEN.width, SCREEN.height);
        controls.settings = imported.preset;
        let mouse = controls.custom_placement(1);
        assert!(mouse.visible_in_game && mouse.visible_in_menu);
        assert_ne!(mouse.center, [0.60, 0.18]);
        controls.update(1, mouse.center, true, false);
        assert_eq!(controls.key_events(), [("mouse.virtual.toggle".into(), true)]);
        controls.update(1, mouse.center, false, true);
        assert_eq!(controls.key_events(), [("mouse.virtual.toggle".into(), false)]);
    }
}
