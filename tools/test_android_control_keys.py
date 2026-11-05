#!/usr/bin/env python3
"""Exercise production latch recovery with synthetic touch/OS input, without a window."""
import json
import os
from pathlib import Path
import subprocess
from test_android_pointer import block

ROOT = Path(__file__).resolve().parents[1]

def main():
    work = ROOT / '.local/android/control-key-tests'
    (work / 'src').mkdir(parents=True, exist_ok=True)
    (work / 'Cargo.toml').write_text('''[package]
name="android-control-key-tests"
version="0.1.0"
edition="2024"
[workspace]
[dependencies]
horizon-client-input={path=INPUT_PATH}
winit="0.30.13"
serde={version="1",features=["derive"]}
serde_json="1"
'''.replace('INPUT_PATH', json.dumps(str(ROOT / 'crates/horizon-client-input'))))
    source = '#![allow(dead_code)]\nextern crate self as horizon_client;\n'
    source += 'pub use horizon_client_input::{keybinds,keymapping,modifiers};\n'
    # Keep the production settings module; its unrelated UI tests need a renderer.
    layout = (ROOT / 'crates/horizon-client-render/src/touch/layout.rs').read_text()
    (work / 'src/touch_settings.rs').write_text(layout.split('#[cfg(test)]')[0])
    source += 'mod touch_settings;\n'
    custom = (ROOT / 'crates/horizon-client-render/src/touch/custom.rs').read_text()
    source += '#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]\n' + block(custom, 'pub struct CustomButton {') + '\n'
    source += 'use touch_settings::Placement;\nimpl CustomButton {' + block(custom, '    pub fn valid_token(') + '}\n'
    source += '''
use std::collections::HashSet;
use winit::keyboard::{KeyCode, ModifiersState};
use keymapping::ToggleKeys;
struct Settings { keys:keybinds::KeyBinds, controller:(), toggle_crouch:bool, toggle_sprint:bool }
#[derive(Default)] struct TouchUi;
impl TouchUi {fn modifier_latched(&self,_:KeyCode)->bool {false}}
#[derive(Default)] struct Touch {inventory_shift:bool}
#[derive(Default)] struct Pad {custom_keys:HashSet<KeyCode>, touch_ui:TouchUi, touch:Touch}
impl Pad {fn holds_action(&self,_:&(),_:&str)->bool {false}}
struct Game {settings:Settings,pad:Pad,physical_keys:HashSet<KeyCode>,keys:HashSet<KeyCode>,modifiers:ModifiersState,toggles:ToggleKeys}
impl Game {
 fn new()->Self {Self {settings:Settings {keys:Default::default(),controller:(),toggle_crouch:false,toggle_sprint:false},pad:Default::default(),physical_keys:Default::default(),keys:Default::default(),modifiers:ModifiersState::empty(),toggles:Default::default()}}
 // Only the input provider is synthetic: production key translation is retained.
 fn android_custom_key_held(&self,key:KeyCode)->bool {self.pad.custom_keys.iter().any(|k|self.settings.keys.translate_all(*k).contains(&Ok(key)))}
 fn android_inventory_controls(&self)->bool {false}
 fn press(&mut self,key:KeyCode) {self.pad.custom_keys.insert(key);self.physical_keys.insert(key);for mapped in self.settings.keys.translate_all(key) {if let Ok(KeyCode::ShiftLeft)=mapped {self.toggles.key(0,keymapping::KeyAction::Press,false,false);self.sync_toggle(0);}}}
 fn release(&mut self,key:KeyCode) {self.pad.custom_keys.remove(&key);self.physical_keys.remove(&key);self.toggles.key(0,keymapping::KeyAction::Release,false,false);self.sync_toggle(0);}
'''
    game = (ROOT / 'crates/horizon-client/src/game.rs').read_text()
    for method in ['held_control_action', 'heal_hold_latches', 'sync_toggle', 'has_shift_down']:
        needle = ('    pub(super) fn ' if method == 'has_shift_down' else '    fn ') + method + '('
        source += block(game, needle).replace('#[cfg(target_os = "android")]', '').replace('pub(super) fn', 'fn') + '\n'
    source += '''}
#[test] fn shift_hold_survives_frames_and_unrelated_os_modifiers_then_releases() {
 let mut g=Game::new();g.press(KeyCode::ShiftLeft);
 for _ in 0..5 {g.heal_hold_latches();assert!(g.keys.contains(&KeyCode::ShiftLeft));assert!(g.has_shift_down());}
 let held=[g.held_control_action("key.sneak"),g.held_control_action("key.sprint")];
 modifiers::reconcile(ModifiersState::ALT,&mut g.physical_keys,&g.settings.keys,&mut g.toggles,[false;2],held);
 g.sync_toggle(0);g.heal_hold_latches();assert!(g.keys.contains(&KeyCode::ShiftLeft));assert!(g.has_shift_down());
 g.release(KeyCode::ShiftLeft);g.heal_hold_latches();assert!(!g.keys.contains(&KeyCode::ShiftLeft));assert!(!g.has_shift_down());
}
#[test] fn rebound_touch_sneak_is_preserved_without_claiming_inventory_shift() {
 let mut g=Game::new();g.settings.keys.set("key.sneak","key.keyboard.c");g.press(KeyCode::KeyC);
 for _ in 0..5 {g.heal_hold_latches();assert!(g.keys.contains(&KeyCode::ShiftLeft));assert!(!g.has_shift_down());}
 g.release(KeyCode::KeyC);g.heal_hold_latches();assert!(!g.keys.contains(&KeyCode::ShiftLeft));
}
#[test] fn missing_hardware_release_still_heals() {
 let mut g=Game::new();g.physical_keys.insert(KeyCode::ShiftLeft);g.toggles.key(0,keymapping::KeyAction::Press,false,false);g.sync_toggle(0);
 g.heal_hold_latches();assert!(!g.keys.contains(&KeyCode::ShiftLeft));assert!(!g.physical_keys.contains(&KeyCode::ShiftLeft));
}
#[test] fn mouse_sensitivity_survives_profile_json_reload_and_sanitization() {
 for sensitivity in [0.01,1.0,3.01,10.0] {let mut settings=touch_settings::Settings::default();settings.virtual_mouse_sensitivity=sensitivity;
 let bytes=serde_json::to_vec(&settings).unwrap();let mut loaded:touch_settings::Settings=serde_json::from_slice(&bytes).unwrap();loaded.sanitize();assert_eq!(loaded.virtual_mouse_sensitivity,sensitivity);}
 for (value,want) in [(-1.0,0.01),(20.0,10.0),(f32::NAN,1.0)] {let mut s=touch_settings::Settings::default();s.virtual_mouse_sensitivity=value;s.sanitize();assert_eq!(s.virtual_mouse_sensitivity,want);}
}
'''
    (work / 'src/lib.rs').write_text(source)
    env = dict(os.environ)
    env.setdefault('CARGO_BUILD_JOBS', '2')
    env.setdefault('CARGO_TARGET_DIR', str(ROOT / '.local/android/test-target'))
    subprocess.run(['cargo', 'test', '--manifest-path', str(work / 'Cargo.toml')], cwd=ROOT, env=env, check=True)

if __name__ == '__main__':
    main()
