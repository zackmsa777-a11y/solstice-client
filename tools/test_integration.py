#!/usr/bin/env python3
# SPDX-License-Identifier: MIT
# AI-assisted integration verification by Codex (OpenAI), 2026.
"""Run host-free production tests only; never Cargo, the game, or network tools."""
import argparse
import hashlib
import json
from pathlib import Path
import re
import subprocess
import sys
import tempfile
import tomllib

parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument('--rustc', required=True)
parser.add_argument('--report', type=Path)
args = parser.parse_args()
root = Path(__file__).resolve().parents[1]
payload = root / 'mixins/solstice'
manifest = tomllib.loads((payload / 'horizon-mixin.toml').read_text(encoding='utf-8'))
assert manifest['mod']['version'] == '0.8.0'
assert manifest['compatibility']['horizon_revision'] == 'ba6344a59cb9e755b730dcf02285fc0d33b97d3c'
for group, field in [('module', 'source'), ('injection', 'helper')]:
    entries = manifest[group]
    assert len({entry['id'] for entry in entries}) == len(entries), f'duplicate {group} id'
    assert all((payload / entry[field]).is_file() for entry in entries)
observers = [entry for entry in manifest['injection'] if entry['target'] == 'horizon_client::game::follow_keymap']
assert {entry['helper'] for entry in observers} == {'bind_event.rs', 'text_event.rs'}
assert len(observers) == 2 and all(entry['kind'] == 'HEAD' for entry in observers)

aliases = {
    'bind_state': 'bind_state.rs', 'solstice_bind_state': 'bind_state.rs',
    'hud_layout': 'hud_layout.rs', 'solstice_hud_layout': 'hud_layout.rs',
    'text_edit': 'text_edit.rs', 'solstice_text_edit': 'text_edit.rs',
    'registry_picker': 'registry_picker.rs', 'solstice_registry_picker': 'registry_picker.rs',
}
def identity(suite, name):
    first, separator, rest = name.partition('::')
    return f'{aliases[first]}::{rest}' if first in aliases else f'{suite}::{name}'

version = subprocess.check_output([args.rustc, '--version'], text=True).strip()
assert version.startswith('rustc 1.99.1 '), version
records = []
with tempfile.TemporaryDirectory(prefix='sunset-first-wave-tests-') as folder:
    folder = Path(folder)
    commands = [(name, [sys.executable, str(root / 'tools' / script), '--rustc', args.rustc]) for name, script in [
        ('profiles', 'test_profiles.py'), ('text-settings', 'test_text_settings.py'),
        ('binds', 'test_binds.py'), ('hud-layout', 'test_hud_layout.py'),
        ('integration-events', 'test_integration_events.py'),
    ]]
    for name, source in [
        ('draft', 'test_text_edit.rs'), ('registry-picker', 'test_registry_picker.rs'),
        ('block-scan', 'block_scan_tests.rs'), ('armor', 'armor_policy_tests.rs'),
        ('low-health', 'low_health_adapter_tests.rs'),
    ]:
        binary = folder / f'{name}.exe'
        commands.append((name, [args.rustc, '--edition=2024', '--test', str(root / 'tools' / source), '-o', str(binary)]))
        commands.append((name + '-run', [str(binary)]))
    radar = folder / 'radar.rs'
    radar.write_text(f'#[path = r"{payload / "radar_math.rs"}"] mod radar_math;\n', encoding='utf-8')
    binary = folder / 'radar.exe'
    commands += [('radar-compile', [args.rustc, '--edition=2024', '--test', str(radar), '-o', str(binary)]), ('radar', [str(binary)])]
    for name, command in commands:
        result = subprocess.run(command, capture_output=True, text=True, encoding='utf-8', errors='replace')
        output = result.stdout + result.stderr
        tests = re.findall(r'^test (\S+) \.\.\. ok$', output, re.M)
        records.append({'suite': name, 'exit_code': result.returncode, 'passed': len(tests),
                        'test_identities': [identity(name.removesuffix('-run'), test) for test in tests]})
        print(f'{name}: exit {result.returncode}; {len(tests)} passed', flush=True)
        if result.returncode:
            print(output, flush=True)
            raise SystemExit(result.returncode)

all_tests = [test for record in records for test in record['test_identities']]
sources = sorted(set(payload.glob('*')) | set((root / 'tests').glob('*.rs')) | set((root / 'tools').glob('*.py')) | set((root / 'tools').glob('*.rs')))
report = {
    'scope': 'Local host-free integration only; no AST staging, Cargo, game, push or PR.',
    'compiler': version,
    'modules': len(manifest['module']), 'injections': len(manifest['injection']),
    'shared_head_observers': [entry['helper'] for entry in observers],
    'executed_test_instances': len(all_tests), 'unique_test_identities': len(set(all_tests)),
    'shared_model_counting': 'Repeated bind/input, draft, picker and HUD model tests are deduplicated by production source and test name.',
    'suites': records,
    'source_sha256': {path.relative_to(root).as_posix(): hashlib.sha256(path.read_bytes()).hexdigest() for path in sources if path.is_file()},
}
if args.report:
    destination = args.report if args.report.is_absolute() else root / args.report
    destination.parent.mkdir(parents=True, exist_ok=True)
    destination.write_text(json.dumps(report, indent=2) + '\n', encoding='utf-8')
print(f'PASS: {len(all_tests)} test instances; {len(set(all_tests))} unique test identities.', flush=True)
