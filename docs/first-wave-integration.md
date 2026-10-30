# Local first-wave integration

`overnight-integration` combines the independent contributions on top of
`d48f0ea`. It preserves the Solstice name, version 0.8.0, and root MIT license.
This is a local composition branch, not an upstream merge or release.

The source commits are retained as separate cherry-picks with original commit
references: Radar `b33ffa0`; profiles `e44f9ce` and `561500d`; text editing
`574bbcf`, `d478c7b`, and its runtime evidence `de88ce5`; binds `d0f0109`
and the visible-value label follow-up `eccd1d5`;
HUD positioning `b5bc5f9` and `9ffa6df`; bounded scans `5b3406e`; Armor
Durability `5a1c06a`; Low Health `d0db40b`; and the registry picker `9a4c435`
with its review correction `c06dc09`. Feature source branches remain
unchanged. Integration-only commits hold the composition fixes and tests.

## Conflict decisions

- Manifest conflicts retain every module and injection. The separate bind and
  text `HEAD` observers both target `follow_keymap`; neither replaces the other.
  The host-free runner checks their presence, unique IDs, and referenced files.
- Frame conflicts preserve bind synchronization, HUD editor lifecycle,
  block-scan lifecycle, Low Health sampling, and the existing render callback.
  The Low Health native-disabled guard remains ahead of config loading.
- The profiles serializer iterates the proposed module clone with its existing
  module indices. It uses `Inputs.saved_on(index, proposed.on)` when persisting
  enabled states, so a temporary Hold does not become a saved enabled module.
  Tests also verify the proposed setting value is saved instead of the old one.
- Profiles accept exact text padding because the normal config loader now
  preserves the full text RHS. Both save and load still reject controls,
  U+2028/U+2029, and text over 4096 UTF-8 bytes. Profile/config/resave tests
  preserve padded Unicode, empty strings, spaces, commas, pipes, and equals.
- `gui_key` gives HUD placement priority, then consumes text-modal keys before
  bind capture/search. `text_event` clears stale text/registry drafts while
  HUD placement is active. Mouse dispatch similarly gives the HUD editor
  priority before modal and panel handling. Opening/closing the GUI clears
  text drafts and pending consumed state.
- Registry search keeps its separate query focus and logical-key navigation.
  Text still uses raw Winit layout text and explicit Apply/Cancel. Bind releases
  and focus loss still reach the bind observer while text consumes key presses.
  Repeated Enter does not repeat Add/Remove; Tab intentionally retains standard
  repeating focus navigation, and arrow/edit-key repeats remain supported.
  Liquid Filler suggestions use its actual vanilla-only lookup domain; other
  placement fields keep namespaced catalogs and custom raw tokens are retained.
- Watermark, module list, and info retain the HUD position editor's ownership.
  Radar, Armor Durability, and Low Health keep their independent Corners
  placement and reservation checks; no expanded draggable-editor claim is made.
- Host-free fixtures select settings by name because Bind activation is now
  the first setting. They include the real production Inputs model wherever
  the production serializer or Module constructor depends on it.

## Verification scope

Run the complete host-free suite with the explicitly pinned compiler:

```powershell
python tools/test_integration.py --rustc D:/Codex-Meteor-Build/toolchains/1.99.1-x86_64-pc-windows-msvc/bin/rustc.exe --report docs/integration-hostfree-checks.json
```

The report lists each suite, every test identity, compiler version, manifest
checks, and source hashes. Executed instances and distinct tests are separate
counts: imported bind, draft, picker, and HUD model tests run in multiple
harnesses and are deduplicated by their production source and test name.

The event harness extracts the actual production observer/restoration bodies,
then supplies a small host vocabulary and the actual Inputs, Draft, Picker,
and HUD editor models. It verifies both observer orders, swallowed releases,
Escape consumption, focus-loss cleanup, HUD/modal ownership, and independent
registry/raw logical navigation. The host doubles do not establish real Winit
API compatibility or live appearance.
The actual-event Enter regression failed against the preliminary registry
source and passes with `c06dc09`; companion cases retain the documented Tab,
arrow, Home, and Backspace repeat behavior.
The event harness does not extract `gui_key`, native/mapped-key handling, or
actual window dispatch. Its modifier fixture is neutral; Ctrl/Alt behavior
remains covered by source review and separate feature validation, not this
composition event harness.

Integration preparation does not stage the AST, invoke Cargo, build or launch
a game, push a branch, or create a PR. The owner performs the final fresh AST
transformation, serial host build, and unified isolated game validation.

Integration fixes, tests, and this note are MIT licensed under the repository
license and were written with Codex (OpenAI) assistance.
