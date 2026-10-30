# Shared Block ESP and Hole ESP read budget

Block ESP and Hole ESP share a hard allowance of 4096 loaded-world block reads per
client tick and one soft 1 ms deadline. The fixed two-job coordinator alternates
64-read slices, with the first job alternating each new tick. Idle/not-due jobs
yield their allowance. A repeated or older tick stamp cannot grant more reads,
even if a lifecycle reset occurs between calls.

A Hole candidate is admitted only with seven remaining credits in its slice and
the total allowance. It completes in that invocation, charging one read for a
non-air foot, two for a non-air head, or seven for the full enclosure test. The
deadline is checked before every candidate; it may overrun while those probes
finish. Block slices check the same deadline before reading. Neither counter nor
deadline is restarted for the second job. Block can consume a final one-to-six
credits that cannot admit another Hole candidate.

The frozen Hole behavior is preserved: x/z/y traversal, inclusive Y from
`max(center_y-range,-63)` through `center_y+range` with no upper 320 clamp; two air
blocks and eager below/east/west/south/north probes; any non-air enclosure, including
plants/fluids; green when every enclosure name ends in obsidian or bedrock, otherwise
yellow. The original 400 check skips only the rest of that Y column. Later columns
can still add their first qualifying result, so this remains **not a global cap**.
Its conservative bound is 399 plus the number of X/Z columns, at most 1488 results.
Block retains its original first-2000-match cap. Drawing colors, footprints, Fill
flags and distance filters are unchanged.

Each cycle keeps its starting center while ordinary walking updates the next
request. Pending vectors remain private until complete; an ordinary refresh keeps
the prior publication visible. Hole starts a nominal refresh 20 ticks after
publication, Block after 100 ticks or six blocks of movement. Sharing and incremental
completion add latency; these are not once-per-second or atomic-world snapshots.
No measured frame-time improvement is claimed.

Range/config/dimension changes and large observed jumps invalidate the affected
job. Disabling one module clears only its job. Missing player, title/world exit or
native disable clears both; named world/remote lifecycle hooks also increment the
shared world epoch, including same-dimension replacements. The scan frame/draw
adapters only observe these tags and never advance a cursor or read a block. Server/disabled
startup guards run before config loading. Hole coordinates reject impossible I32
neighbor bounds instead of wrapping or merging probes; Block retains the pilot's
saturating bounds because it has no neighbor probes.

Only the existing resident `world.block` accessor is used. No chunks are requested,
no packets/actions/settings/dependencies are added, and other scanners are unchanged.

Tests compile the actual production models and extracted adapter/guard/classifier:

```text
rustc --edition=2024 --test tools/block_scan_tests.rs -o block-scan-tests
./block-scan-tests
python tools/test_scan_adapter.py
```

The legacy eight Block tests remain unchanged. New checks compare Hole results and
read counts with the frozen old-loop oracle for every range 3–16, negative and Y-boundary
poses, dense output exceeding 400, mixed enclosures, admission/read ordering, deadline
and fairness behavior, duplicate ticks, publication and invalidation. Adapter fixtures
replace only host vocabulary; the actual source bodies verify zero-read frame/draw
and disabled/server paths, no config load on disabled/server startup, one shared tick
allowance, suffix classification, plants/fluids and draw-only Fill changes.

This follow-up depends on the Block ESP pilot `5b3406e`. It needs isolated in-game
latency/footprint validation before making runtime claims. MIT; original implementation
and tests written with assistance from OpenAI Codex. No donor implementation was read.
