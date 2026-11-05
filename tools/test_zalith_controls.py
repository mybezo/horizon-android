#!/usr/bin/env python3
"""Run the actual layout/storage/document modules without graphics or player data."""
import argparse
import json
import os
from pathlib import Path
import subprocess
ROOT=Path(__file__).resolve().parents[1]
def main():
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--layout',type=Path,help='Also check an external user-supplied controller')
    args=parser.parse_args()
    work=ROOT/'.local/android/zalith-tests'
    (work/'src').mkdir(parents=True,exist_ok=True)
    (work/'Cargo.toml').write_text('''[package]
name="zalith-controls-tests"
version="0.1.0"
edition="2024"
[workspace]
[dependencies]
serde={version="1",features=["derive"]}
serde_json="1"
''')
    files={'layout':'crates/horizon-client/src/controls/layout.rs','storage':'crates/horizon-client/src/controls/storage.rs','document_store':'crates/horizon-client/src/android/document_store.rs'}
    source='\n'.join(f'#[allow(dead_code)]\n#[path={json.dumps(str(ROOT/path))}]\nmod {name};' for name,path in files.items())
    if args.layout:
        source+='''\n#[test]
fn supplied_controller_roundtrips_without_conversion() {
    let bytes=std::fs::read('''+json.dumps(str(args.layout.resolve()))+''').unwrap();
    let value=storage::parse(&bytes).unwrap();
    assert_eq!(value.editor_version,12);
    assert_eq!(value,layout::Layout::parse(&value.to_json()).unwrap());
}
'''
    (work/'src/lib.rs').write_text(source)
    env=dict(os.environ)
    env.setdefault('CARGO_TARGET_DIR',str(ROOT/'.local/android/test-target'))
    subprocess.run(['cargo','test','--manifest-path',str(work/'Cargo.toml'),'--','--test-threads=1'],env=env,cwd=ROOT,check=True)
if __name__=='__main__':main()
