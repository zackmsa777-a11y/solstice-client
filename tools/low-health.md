# Low Health warning

**Low Health** is an optional Render module, off by default. It displays one
steady native-font line such as `Low health: 5.0 HP (threshold 6)` in an
allocated HUD corner. It is independent of the main HUD toggle and Auto Log.
It has no chat notifications, game actions, disconnects, sound, flashing,
screen effects, network operations, or telemetry.

The threshold defaults to 6 HP, configurable from 1 to 40 HP. Hysteresis
defaults to 2 HP, configurable from 0 to 20 HP. Both menu controls step by
0.5 HP. The warning activates at or below the threshold and clears strictly
above threshold plus hysteresis. At the default values it activates at 6 HP
and clears above 8 HP. The strict recovery boundary keeps zero hysteresis
stable when health is exactly the threshold. Changing either setting clears
the old latch and immediately evaluates the new policy. HP is displayed to
one decimal place; threshold comparisons use the actual values.

The adapter reads `game.player.health.health()` from the pinned Horizon host.
These are raw local base health points (20 HP is ten ordinary hearts).
Absorption is deliberately excluded: temporary absorption hearts cannot mask
low base health. The current HP is sampled every frame, including while F1 or
a debug corner hides the display, and again at drawing to avoid stale text.
It does not infer health from a screen, tab list or another player.

The latch resets on module/native disablement, death, title return, a missing
player, invalid health/configuration, or changed health settings. The window's
frame hook runs outside the HUD, so these resets also occur while F1 hides
HUD rendering. Server Games do not mutate the window's latch. Native-disabled
updates reset only the warning latch and return before loading settings, which
can synchronize other modules' input or packet flags. The host's
world lifecycle hook is explicitly server-side and is not used for this
client display. Disconnect/open paths clear the local player or enter title;
the next window frame resets the latch. Health must be finite, positive and
representable by the host's f32 health API; settings have finite bounded
ranges. There are no periodic notifications or accumulated events.
The adapter also checks the host's `dead_or_dying()` flag and death screen,
so positive HP on a dead player cannot display the warning.

Corner defaults to Bottom right, with the other three corners available.
The display uses Corners, existing native text measurement, the existing
translucent HUD background and steady red text. It yields to F3/debug chart
reservations through `native_corner_shown`, and stays hidden under F1, without
a living player/world, on title, or with the native mod disabled. It does not
claim draggable HUD-editor integration.

Host-free tests include the actual production model:

```sh
rustc --edition 2024 --test tools/low_health_policy_tests.rs -o low-health-tests
./low-health-tests
rustc --edition 2024 --test tools/low_health_adapter_tests.rs -o low-health-adapter-tests
./low-health-adapter-tests
```

The six policy tests cover threshold/recovery transitions, zero-hysteresis
boundary stability, repeated sampling, lifecycle and disable gaps, setting
changes, invalid finite bounds, and recovery while the HUD is hidden. The
adapter harness includes that same production model and actual production
adapter with a minimal host test double. Four additional tests verify native
disablement prevents config-loading side effects, server Games cannot reset
window state, absorption does not mask base HP, hidden-HUD recovery, and the
adapter's death/lifecycle/disable guards. The real pinned-host Cargo check
separately verifies actual API types. These tests do not verify live visuals.

For an isolated local-game functional check:

1. Leave Auto Log and Auto Respawn off. With fresh settings confirm Low Health
   is off; enable it using the Render menu or `.toggle lowhealth`.
2. With threshold 6 and hysteresis 2, set local base HP to 7, 6, 7, 8, and 9.
   Expect hidden, visible, visible, visible, hidden. Capture the 6 HP and held
   7 HP views, including the displayed threshold and normal HUD groups.
3. Add absorption at 5 base HP. Confirm the warning remains visible. Heal base
   health above 8 and confirm it clears. No disconnect or action should occur.
4. Set hysteresis to zero and keep health exactly 6 for several frames: the
   warning stays steady. Change threshold while warned and confirm it is
   reevaluated immediately. Cycle all corners and verify shared HUD placement.
5. Press F1, recover above the boundary while hidden, then restore the HUD:
   no stale warning. Choose a top corner and open F3; that corner yields.
   Test a bottom debug-chart reservation similarly.
6. While warned, die, disconnect to title, disable the module, and disable the
   native mod in separate trials. After each reset, return alive at 7 HP:
   the old warning must remain cleared. Relaunch with isolated settings and
   verify the module and setting choices persist; warning latch is transient.

This implementation was written from the functional requirement and pinned
host APIs, without reading or importing another client's implementation.
New source and tests are MIT licensed under the repository license and were
implemented with Codex (OpenAI) assistance.
