# Keybinds status HUD

**Keybinds HUD** is an optional Render module, off by default. It shows modules
with configured binds in their registration order, using separate columns for
module name, the normal key label, and current **ON/OFF** state. It reads the
actual module `on` flag: a temporary Hold activation remains visible as ON when
this feature is combined with the bind contribution. It does not persist that
state or change inputs, binds, settings, activation, or any module lifecycle.

Settings use the existing module configuration:

- **Only active** defaults off. When enabled, only bound modules currently ON
  count as eligible. Otherwise, bound OFF modules remain visible too.
- **Max rows** defaults to 8, with a range of 1–20 and menu steps of 1.
  Fractional config values round down; non-finite model inputs fall back to 8.
- **Corner** defaults to Bottom left; all four corners are available.

The collector reads only names, keys, and effective states under the existing
state mutex. It copies at most 20 rows; no Module or settings collection is
cloned for this display. Each copied field is bounded to 512 UTF-8 bytes plus
an explicit `...` marker, and invalid line/control characters display as visible
escapes. Additional matching modules are counted without copying their fields.
Unknown key identifiers remain visible through the existing `key_label`
helper; there is no new key map or guessed fallback key. A configured empty
key is explicitly labeled `(empty)`. Menu shortcuts are outside this module
list and are not added as invented rows.

Text measurement runs after releasing the mutex because the native font hook
may read settings. The existing native font, gold title, white module names,
gray keys/OFF states, green ON states, and translucent HUD background are used
without new assets, resampling, clipping, effects, sounds, or network work.

The compact panel is at most 240 GUI pixels wide. Names and labels are fitted
with measured ASCII `...`; the ON/OFF status column is never shortened. The
panel reduces the displayed rows to fit available corner height and reports
the exact omitted count, including both Max rows and screen-height overflow.
Very short corners can show only the title and count. Empty results explicitly
say No bound modules or No active binds. If truthful content cannot fit, the
panel remains hidden instead of drawing outside the viewport.

The pinned host's Corners API has allocation but no remaining-space query.
The adapter reserves a zero-height anchor, consuming its normal 2-pixel gap,
then sizes the real group using that edge and the allocator's next gap. It
checks the final group rectangle against the viewport before emitting shapes.
If there is insufficient space, the anchor can leave a 2-pixel gap without a
visible panel. F1, reserved debug corners, title/no player, native disablement,
and non-window Games suppress the HUD. This is an independent corner group,
not an addition to draggable HUD-editor ownership.

Host-free validation includes the actual production selection/layout model,
adapter, and extracted normal key-label helper:

```powershell
python tools/test_keybinds_hud.py --rustc D:/Codex-Meteor-Build/toolchains/1.99.1-x86_64-pc-windows-msvc/bin/rustc.exe
```

The suite covers row/filter ordering, allocation and count bounds, escaped and
unknown keys, measured ellipses, complete status columns, small/exhausted
corners, visibility gates, and read-only effective state. Its adapter fixture
fails if a Module is cloned or text is measured while the state mutex is held.
The fixture supplies host vocabulary and simple font metrics; it does not prove
live glyph appearance, real Winit behavior, or game API compatibility. The
separate exact-host Cargo check verifies the real adapter API.

For the owner's isolated local-game validation:

1. With fresh settings confirm the module is off. Bind two modules to different
   keys, leave one ON and one OFF, then enable Keybinds HUD. Verify labels and
   status against `.modules` and bind settings; capture the panel.
2. Toggle each module normally. The next HUD frame should show the current
   state. With the bind contribution present, verify a supported Hold appears
   ON while held and returns to its prior state on release. Verify mouse labels
   use that contribution's existing formatter. This HUD must trigger no action.
3. Enable Only active; verify only eligible ON modules count. Bind additional
   modules and lower Max rows; verify stable order and the exact +N more count.
4. Check long key/module labels and a smaller GUI viewport. Confirm measured
   ellipses, readable complete ON/OFF, row reduction/counts, and no screen-edge
   spill. Cycle all corners with existing HUD groups and status effects visible.
5. Test F1, F3/top-corner and chart/bottom-corner reservations, native disablement,
   title return, and disconnect. No panel should remain in a forbidden context.
6. Relaunch with the same isolated configuration and verify the module settings
   persist. It must not alter any bind or another module's enabled state.

This feature was written from the functional requirement and pinned host APIs,
without reading another client's implementation. New source, tests, and this
note are MIT licensed under the repository license and were implemented with
Codex (OpenAI) assistance.
