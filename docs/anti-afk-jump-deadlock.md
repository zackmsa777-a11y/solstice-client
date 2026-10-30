# Anti AFK Jump deadlock

With Anti AFK enabled and Jump on (its default), `afk_keys` held the player
state mutex and called `rand`, which tried to acquire the same non-reentrant
mutex. The first eligible jump tick could therefore stop the client thread.
The random helper now accepts the already-locked state; callers outside that
lock still use the existing `rand` wrapper. Jump timing, random state updates,
settings and other Anti AFK actions retain their existing behavior.

[Issue #3](https://github.com/PointOfPressure/sunset-client/issues/3) reports a
hang when expanding Anti AFK. Its attachment shows Windows reporting that the
client is not responding, with Anti AFK enabled. A host-free reproduction of
the original Jump path also hangs. This establishes the jump-tick deadlock;
it does not establish a separate failure in settings expansion.

Run `python tools/test_anti_afk.py --rustc <rustc>` for three host-free tests.
The harness extracts the actual production state, mutex, random helpers and
`afk_keys`, supplies only key/setting doubles, and uses a ten-second process
watchdog. It checks 1,024 default Jump calls against fixed sequence fixtures,
the held-jump random short circuit, unrelated state preservation, seven RNG
seed/modulus fixtures, and Jump-disabled behavior. These tests do not exercise
the real tick hook or GUI. An isolated client check should enable Anti AFK
with default Jump, keep playing for several ticks, expand and collapse its
settings, then disable it and confirm the client remains responsive.
