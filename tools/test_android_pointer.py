#!/usr/bin/env python3
"""Run the existing pointer state-machine methods and tests without GPU/OS bindings.

Only dependency types unused by these tests are inert declarations. All pointer
logic and test bodies are extracted unchanged from the production source.
"""
from pathlib import Path
import os
import subprocess
ROOT=Path(__file__).resolve().parents[1]
def block(source,needle):
    start=source.index(needle)
    end=source.index('{',start)+1
    depth=1
    while depth:
        if source[end]=='{':depth+=1
        elif source[end]=='}':depth-=1
        end+=1
    return source[start:end]
def main():
    source=(ROOT/'crates/horizon-client/src/touch_ui.rs').read_text()
    work=ROOT/'.local/android/pointer-tests';work.mkdir(parents=True,exist_ok=True)
    text='''#![allow(dead_code)]
use std::{collections::BTreeMap,time::{Instant,Duration}};
type KeyCode=u32;
type Keyboard=();
#[derive(Clone,Copy,PartialEq,Eq)]
pub enum TouchPhase {Started,Moved,Ended,Cancelled}
#[derive(Default)]
'''+block(source,'pub struct TouchUi {')+'\n'+block(source,'struct RelativeTouch {')+'\nimpl TouchUi {\n'
    for name in ['suppress_relative_click','press_mouse','release_mouse','mouse_held','cancel','cancel_relative','relative_touch','relative_click']:
        text+=block(source,'    pub fn '+name+'(')+'\n'
    text+='}\n#[cfg(test)] mod tests {use super::*;\n'
    for name in ['virtual_mouse_scales_motion_from_one_to_one_thousand_percent','explicit_mouse_drag_does_not_generate_an_extra_tap_or_hold_click','virtual_mouse_moves_relatively_and_dragging_never_clicks','virtual_mouse_tap_hold_cancel_and_click_preference','layout_mouse_hold_keeps_relative_motion_without_an_extra_click']:
        text+='#[test]\n'+block(source,'    fn '+name+'(')+'\n'
    text+='}\n'
    file=work/'pointer.rs';file.write_text(text)
    compiler=os.environ.get('RUSTC','rustc')
    subprocess.run([compiler,'--edition=2024','--test',str(file),'-o',str(work/'pointer-tests')],check=True,cwd=ROOT)
    subprocess.run([str(work/'pointer-tests')],check=True,cwd=ROOT)
if __name__=='__main__':main()
