#!/usr/bin/env python3
# SPDX-License-Identifier: MIT
# AI-assisted integration regression harness, Codex (OpenAI), 2026.
"""Compile actual text/bind event observers with production editor/input models."""
import argparse
from pathlib import Path
import subprocess
import tempfile

parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument('--rustc', required=True)
args = parser.parse_args()
root = Path(__file__).resolve().parents[1]
runtime = (root / 'mixins/solstice/runtime.rs').read_text(encoding='utf-8')

def section(start, end):
    return runtime[runtime.index(start):runtime.index(end, runtime.index(start))]

parts = [
    section('fn restore_holds(', 'fn release_changed_binds('),
    section('fn release_binds(', 'fn set_bind('),
    section('pub(super) fn bind_event(', '// The next GUI click'),
    section('fn text_edit_key(', '// The named native-key API'),
    section('pub(super) fn text_event(', '// A key while the GUI is open'),
]
modules = ''.join(f'#[path = r"{root / "mixins/solstice" / file}"] mod {name};\n' for name, file in [
    ('solstice_text_edit', 'text_edit.rs'), ('solstice_bind_state', 'bind_state.rs'),
    ('solstice_hud_layout', 'hud_layout.rs'), ('solstice_registry_picker', 'registry_picker.rs'),
])
fixture = (root / 'tests/integration_events.rs').read_text(encoding='utf-8')
source = '#![allow(dead_code)]\n' + modules + fixture.replace('// PRODUCTION_EVENT_OBSERVERS', '\n'.join(parts))
with tempfile.TemporaryDirectory(prefix='sunset-integration-events-') as folder:
    folder = Path(folder)
    rust = folder / 'events.rs'
    binary = folder / 'events.exe'
    rust.write_text(source, encoding='utf-8')
    subprocess.run([args.rustc, '--edition=2024', '--test', str(rust), '-o', str(binary)], check=True)
    subprocess.run([str(binary), '--test-threads=1'], check=True)
