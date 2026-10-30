# HUD group positions

Type `.hudedit` in a world to move the existing watermark, active module list,
and combined coordinates/info group. Enable the HUD and its desired settings
first: only groups with visible content receive gold outlines and drag bounds.
Drag anywhere inside an outlined group with the left button.

- **Apply** saves the draft and closes the editor. A save error keeps the draft
  open so you can retry or cancel.
- **Cancel**, Escape, or the menu key discards the draft.
- **Reset** puts all three groups back into automatic corner stacking in the
  draft. Choose Apply to keep that reset, or Cancel to retain saved positions.
- Tab / Shift+Tab selects an action; Enter activates that action. Other keys
  cannot toggle modules or edit the click-menu search. The wheel is consumed.

Positions are optional per group. Until a group is dragged, its original
corner placement (including host minimap/status-effect space) remains exact.
A dragged group anchors to its nearest horizontal and vertical viewport edge,
keeping that pixel inset as the viewport or content size changes. The origin
is clamped to the current viewport. Content larger than the viewport starts at
the edge and may overflow; text is never scaled or resampled by this editor.
A resize ends an active drag to avoid reusing an old pointer offset.

F1 hides the HUD during normal play; while the editor is open, its input gate
consumes F1. Editing cannot start while hidden, and an external HUD visibility
change ends the draft. Opening another screen, leaving the world, changing dimension,
disabling the mod, or losing focus cancels editing and clears drag state.
The editor shares the click GUI's input gate and suppresses automatic movement
key injection, Freecam movement keys, and Free Look arrow keys in both menus.
Already enabled modules remain enabled; this is a position editor, not a pause
or a module toggle.

Manual positions deliberately permit overlaps with other groups and host
extras. The editor does not reserve the old corner location for a moved group
or attempt collision avoidance. Debug visibility follows the chosen anchor's
corner, so F3 hides groups anchored in its top corners and charts hide their
bottom corners. A manually positioned group spanning a reserved region can
still overlap it. The toolbar takes precedence when it overlaps a group; for
overlapping groups the last drawn group (info, modules, watermark) is picked.

Storage is separate from module configuration: the fixed `hud-layout-v1` file
inside the existing Solstice data directory. Schema 1 starts with
`sunset-hud-layout=1`, followed by exactly one `watermark`, `modules`, and `info`
entry. Each entry is `auto` or `left|right,top|bottom,x,y`. Offsets must be finite
and between 0 and 32768 GUI pixels. Unknown schemas, duplicate/missing groups,
invalid offsets, and files above 1024 bytes are rejected as a whole and use
automatic defaults. Apply uses a fixed sibling temporary file and replaces the
layout only after a successful write; no command accepts a path.

Run `python tools/test_hud_layout.py --rustc /path/to/rustc` for the host-free
anchor, resize, clamp, draft, input and schema tests. Runtime rendering and
input integration also need the pinned Horizon/Mixins staged build and live
client validation. This implementation is original MIT-licensed source,
written with Codex (OpenAI); no other client's implementation was consulted.
