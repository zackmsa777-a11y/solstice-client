#!/usr/bin/env python3
# SPDX-License-Identifier: MIT
# AI-assisted implementation: Codex (OpenAI).
"""Test the actual HUD adapter, production model and normal key_label helper."""
import argparse
from pathlib import Path
import subprocess
import tempfile

parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument('--rustc', required=True)
args = parser.parse_args()
root = Path(__file__).resolve().parents[1]
runtime = (root / 'mixins/solstice/runtime.rs').read_text(encoding='utf-8')
start = runtime.index('pub(super) fn key_label(')
label = runtime[start:runtime.index('// A line in chat:', start)]
fixture = (root / 'tests/keybinds_hud_adapter.rs').read_text(encoding='utf-8')
source = '#![allow(dead_code)]\nextern crate self as horizon_client;\n'
source += f'#[path = r"{root / "mixins/solstice/keybinds_model.rs"}"] mod solstice_keybinds_model;\n'
source += f'#[path = r"{root / "mixins/solstice/keybinds_hud.rs"}"] mod adapter;\n'
source += fixture.replace('// PRODUCTION_KEY_LABEL', label)
with tempfile.TemporaryDirectory(prefix='sunset-keybinds-hud-') as folder:
    folder = Path(folder)
    rust = folder / 'tests.rs'
    binary = folder / 'tests.exe'
    rust.write_text(source, encoding='utf-8')
    subprocess.run([args.rustc, '--edition=2024', '--test', str(rust), '-o', str(binary)], check=True)
    subprocess.run([str(binary), '--test-threads=1'], check=True)
