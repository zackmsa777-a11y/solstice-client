# Block ESP bounded scanning pilot

Block ESP reads loaded blocks incrementally on the client tick. Its fixed defaults
are a hard limit of **4096 block reads per tick** and a **1 ms elapsed-time target**,
checked before the first read and then every 64 reads. A slow read or batch can
exceed the time target. This is a work limit, not a measured frame-time guarantee.
No chunks are requested or generated; the existing read-only `world.block` accessor
returns air for absent chunks. Other scanning modules retain their current behavior.

The existing presets and colours, inclusive cuboid range (8–48), Y clamp
(-64 through 320), x/z/y traversal order, first-2000-match cap, fill setting and
render-distance filtering are retained. Invalid/nonfinite ranges and player poses
clear the cache. Integer coordinate limits use saturating bounds instead of overflow.

A cycle keeps its original center while walking. Its partial results remain private;
only the completed cycle (or its first 2000 matches) replaces the displayed list.
The latest observed position is used when the next cycle starts. A new cycle is due
when the player is at least six blocks from the published center or 100 ticks have
passed since publication. A single observed move of at least twice the range clears
and restarts the cycle. Smaller continuous movement can therefore finish a cycle
centered behind the player; the existing render-distance filter hides distant boxes.
World blocks can change while a cycle runs: publication is coherent for one scan
request, but is not an atomic snapshot of the world.

Changing the preset/range/dimension, disabling Block ESP/the mod, or losing the player
clears both pending and displayed results. The frame and draw paths check this before
showing results, including when ticks are paused. Named remote leave/dimension hooks
also clear on reconnect/respawn in the same dimension. The API 3 world lifecycle hook
clears integrated-server transitions (that hook is server-side in the pinned host).

Spreading work adds detection latency. With no early result cap, the cell cap alone
requires at least 68 ticks for a full default-range cube, or 223 ticks at range 48
(about 3.4 or 11.2 seconds at 20 ticks/s). The elapsed-time target can make it longer.
Existing published results stay visible during an ordinary refresh; the initial scan
and invalidating changes display no results until completion. Dense matching regions
can reach the result cap sooner. No runtime speedup or frame-time improvement has
been measured; in-game responsiveness and detection latency still need validation.

Host-free regression tests use the actual scan model:

```powershell
rustc --edition 2024 --test tools/block_scan_tests.rs -o block_scan_tests.exe
.\block_scan_tests.exe
```

The implementation and tests are MIT licensed, like the repository, and were written
with AI assistance from Codex. No new dependencies or module settings were added.
