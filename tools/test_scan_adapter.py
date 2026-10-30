#!/usr/bin/env python3
# SPDX-License-Identifier: MIT
# AI-assisted production-adapter regression harness; no Horizon, Cargo or game data.
import argparse
import os
from pathlib import Path
import subprocess
import tempfile

root = Path(__file__).resolve().parents[1]
parser = argparse.ArgumentParser()
parser.add_argument('--rustc', default=os.environ.get('RUSTC', 'rustc'))
args = parser.parse_args()
runtime = (root / 'mixins/solstice/runtime.rs').read_text()
start = runtime.index('// Shared render-scan adapters:')
adapters = runtime[start:runtime.index('// End shared render-scan adapters.', start)]
start = runtime.index('pub(super) fn block_colour(')
colour = runtime[start:runtime.index('// Shared render-scan adapters:', start)]
start = runtime.index('pub(super) fn post_tick(')
guard = runtime[start:runtime.index('    let injected', start)].replace('fn post_tick(', 'fn post_tick_guard_prefix(') + '}\n'
fixture = (root / 'tests/scan_adapter_fixture.rs').read_text()
harness = f'''#![allow(dead_code)]
#[path = r"{root / 'mixins/solstice/scan.rs'}"] mod solstice_scan;
mod runtime {{
{fixture}
{colour}
{adapters}
{guard}
}}
'''
with tempfile.TemporaryDirectory(prefix='sunset-scan-adapter-') as folder:
    folder = Path(folder)
    source = folder / 'tests.rs'
    binary = folder / ('adapter-tests.exe' if os.name == 'nt' else 'adapter-tests')
    source.write_text(harness)
    subprocess.run([args.rustc, '--edition=2024', '--test', str(source), '-o', str(binary)], check=True)
    subprocess.run([str(binary)], check=True)
