#!/usr/bin/env python3
# SPDX-License-Identifier: MIT
# Original regression harness assisted by OpenAI Codex.
"""Compile actual Anti AFK key/RNG functions and bound execution with a watchdog."""
import argparse
import os
from pathlib import Path
import subprocess
import tempfile

root = Path(__file__).resolve().parents[1]
parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument('--rustc', default=os.environ.get('RUSTC', 'rustc'))
args = parser.parse_args()
source = (root / 'mixins/solstice/player.rs').read_text(encoding='utf-8')
state = source[source.index('#[derive(Default)]\nstruct Ps'):source.index("// Reach's extra")]
keys = source[source.index('pub(super) fn afk_keys('):source.index('\nfn anti_afk(')]
# Only the host key vocabulary and setting accessors are doubles. State, mutex,
# random helpers and key policy below are extracted verbatim from production.
fixture = r'''
#![allow(dead_code)]
use std::sync::Mutex;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum KeyCode { Space, ShiftLeft, KeyA, KeyD }
struct Module { jump: bool, sneak: bool, strafe: bool }
impl Module {
    fn b(&self, name: &str) -> bool {
        match name { "Jump" => self.jump, "Sneak" => self.sneak, "Strafe" => self.strafe, _ => panic!("unexpected setting") }
    }
    fn n(&self, name: &str) -> f64 { assert_eq!(name, "Sneak time"); 5.0 }
}
mod production {
use super::*;
''' + state + keys + r'''
#[test]
fn default_jump_returns_repeatedly_preserving_sequence_and_unrelated_state() {
    ps(|p| *p = Ps { fish_state: 7, spam_timer: 23, spam_index: 4,
        afk_sneak: 9, afk_strafe: 17, afk_right: true, afk_key: Some(KeyCode::KeyD),
        afk_yaw: Some(37.0), afk_message: 19, afk_index: 3, ..Ps::default() });
    let m = Module { jump: true, sneak: false, strafe: false };
    let mut jumps = Vec::new();
    for tick in 0..1024 {
        let held_before = ps(|p| (p.afk_jump, p.rng));
        let keys = afk_keys(&m);
        if held_before.0 {
            assert!(keys.is_empty());
            assert_eq!(ps(|p| p.rng), held_before.1, "held jump must short-circuit RNG");
        }
        if keys == [KeyCode::Space] { jumps.push(tick); }
        else { assert!(keys.is_empty()); }
    }
    // Fixed seed-zero outputs of the pre-fix RNG algorithm, including the
    // no-RNG tick immediately following every jump.
    assert_eq!(jumps, [23, 33, 219, 302, 402, 462, 466, 689, 711, 729, 820]);
    ps(|p| {
        assert_eq!(p.rng, 0xfbfe5adde6fb704a);
        assert!(!p.afk_jump);
        assert_eq!((p.fish_state, p.cast_left, p.catch_left, p.gap_eating, p.gap_prev), (7, 0, 0, false, 0));
        assert_eq!((p.spam_timer, p.spam_index), (23, 4));
        assert_eq!((p.afk_sneak, p.afk_strafe, p.afk_right, p.afk_key), (9, 17, true, Some(KeyCode::KeyD)));
        assert_eq!((p.afk_yaw, p.afk_message, p.afk_index), (Some(37.0), 19, 3));
    });
}
#[test]
fn rand_wrapper_matches_original_algorithm_seed_fixtures() {
    for (seed, n, expected_state, expected_value) in [
        (0x0, 99, 0x14057b7ef76781b2, 89),
        (0x1, 99, 0x6c576fac43fd00df, 23),
        (0xffffffffffffffff, 99, 0xbbb38751aad20285, 58),
        (0x123456789abcdef0, 26, 0xa1e095c2dee3c199, 1),
        (0x0, 0, 0x14057b7ef767814f, 0),
        (0x2a, 1, 0x91778aed87ee5eb2, 0),
        (0x8000000000000000, 2, 0x94057b7ef7678151, 1),
    ] {
        ps(|p| *p = Ps { rng: seed, ..Ps::default() });
        assert_eq!(rand(n), expected_value);
        assert_eq!(ps(|p| p.rng), expected_state);
    }
}
#[test]
fn jump_disabled_does_not_touch_jump_or_random_state() {
    ps(|p| *p = Ps { afk_jump: true, rng: 42, ..Ps::default() });
    assert!(afk_keys(&Module { jump: false, sneak: false, strafe: false }).is_empty());
    assert_eq!(ps(|p| (p.afk_jump, p.rng)), (true, 42));
}
}
'''
with tempfile.TemporaryDirectory(prefix='sunset-anti-afk-tests-') as directory:
    directory = Path(directory)
    wrapper = directory / 'tests.rs'
    wrapper.write_text(fixture, encoding='utf-8')
    binary = directory / ('anti-afk-tests.exe' if os.name == 'nt' else 'anti-afk-tests')
    subprocess.run([args.rustc, '--edition=2024', '--test', str(wrapper), '-o', str(binary)], check=True)
    # The original bug blocks forever on a mutex, so a test assertion alone
    # cannot catch it. Serialize tests sharing production PS and bound process time.
    try:
        subprocess.run([str(binary), '--test-threads=1'], check=True, timeout=10)
    except subprocess.TimeoutExpired:
        raise SystemExit('Anti AFK tests exceeded 10 seconds: possible mutex deadlock')
