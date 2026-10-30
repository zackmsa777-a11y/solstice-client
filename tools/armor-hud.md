# Armor Durability validation

The Render module **Armor Durability** defaults off and persists through the
existing module settings. It has Corner (default Bottom left) and Display
(Both, Remaining, Percent) settings. It is independent of the main HUD toggle.
It uses the existing native font, gold title, durability colors and translucent
HUD background. It supplies one group to the host Corners allocator; no HUD
editor integration is claimed.

Source semantics use only the pinned Horizon revision
`ba6344a59cb9e755b730dcf02285fc0d33b97d3c`:

- `game/agent.rs` identifies armor slots 500–503 as head, chest, legs, feet.
- `items::ItemStack` supplies current item/count/damage. `items()` confirms the
  item exists in the loaded facts. No material durability values are embedded.
- `item_render::stack_view` preserves saved component patches and removals.
  `ItemModels::components` merges those with the exported item prototype.
  The adapter reads the resulting `minecraft:max_damage`, `minecraft:damage`
  and `minecraft:unbreakable`, so custom maximums apply.
- `native_player` supplies the in-world check; F1, native-enabled, client-window
  and `native_corner_shown` checks hide the group when appropriate. The host
  additionally suppresses its HUD hook under F1.

Empty slots, zero-count stacks and air read **empty**. A missing max-damage or
damage component reads **no durability**. An unbreakable component reads
**unbreakable** without a misleading finite amount. Missing item facts or
prototypes, malformed components, zero/negative/oversized maximums, negative
damage, or damage above maximum read **unknown**. Data already normalized or
discarded by Horizon cannot be reconstructed by the HUD. Percentages are
floored; one point of damage cannot still display 100%.

Host-free tests compile the actual production model:

```sh
rustc --edition 2024 --test tools/armor_policy_tests.rs -o armor-tests
./armor-tests
```

Functional validation for an isolated local world:

1. With fresh settings, confirm Armor Durability is off. Enable it in Render
   or `.toggle armordurability`; four empty rows should appear bottom left.
2. Equip four ordinary pieces with different wear. Compare the displayed
   remaining/max against inventory and the authoritative inventory stack data.
   Damage and repair a piece, swap it and unequip it; the next rendered frame
   should reflect each change. Capture a screenshot showing all four rows.
3. Equip custom max-damage/damage equipment, an unbreakable piece, and a
   nondamageable item in an equipment slot. Confirm the custom arithmetic and
   explicit status labels. No invented vanilla durability should appear.
4. Cycle all Display choices and corners, checking the existing watermark,
   module list, minimap and effect rows share the allocated space. Capture
   the Both view and a Percent view with the module list enabled.
5. Press F1: no armor group. Restore it, choose a top corner and press F3:
   the group disappears in the reserved corner. Check bottom chart reservation
   similarly. Disable the native mod and disconnect to title: no armor group.
6. Relaunch with the same isolated settings and confirm enabled state, corner
   and Display persist. Disable Armor Durability: the group disappears.

The policy tests exercise invalid values without injecting malformed data into
a live world. Visual placement and equipment updates require the local game
validation described above; host compilation alone does not verify appearance.

New armor source and tests are MIT licensed under the repository license and
were implemented with Codex (OpenAI) assistance.
