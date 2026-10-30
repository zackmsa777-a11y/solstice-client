# Registry picker for list settings

The click GUI's text editor offers registry suggestions for existing block/item lists.
The raw value remains editable. Click its value field or the separate **Registry search**
field; Tab switches between them. Search results update as you type. Click **Add** or
**Remove**, or use Up/Down and Enter while searching. Previous/Next and the wheel page
through results. Only eight result rows are rendered at once, with fewer in short viewports.

All changes stay in the current draft. **Apply** saves it; **Cancel**, Escape or focus
loss discards it. Searching changes no setting. If the viewport cannot fit the picker,
the original raw editor remains available. Messages, entity lists, plugin fields and
other text settings keep their plain text editor.

Sources follow the existing consumer:

- Xray / Blocks uses distinct block IDs from loaded registry-derived block facts.
- Surround, Self Trap, Auto Trap, Hole Filler and Spawn Proofer / Blocks, plus Liquid
  Filler / Whitelist and Blacklist, use loaded item IDs that have a block mapping.
- Fast Use / Items uses all loaded item IDs.

The catalog is built once per live resource identity from Horizon's loaded exports;
block-state variants are deduplicated. It does not read the world, request chunks,
bundle a guessed ID list, or rescan the registry every rendered frame. Filtered results
are cached until the search text changes. This feature depends on
[text editor PR #4](https://github.com/PointOfPressure/sunset-client/pull/4), including
its logical-key navigation follow-up (`d478c7b`, based on `574bbcf`).

Existing values are never silently cleaned or rewritten. `obsidian` and
`minecraft:obsidian` match as they do in the module's existing list consumer, while
`custom:obsidian` stays distinct. An explicit Remove removes equivalent occurrences
of that chosen ID; surviving tokens keep their order and exact bytes. Add appends a
full exported ID. Unknown namespaces, aliases, empty tokens and whitespace otherwise
remain unchanged. Raw text and `.set` remain available for values not in the catalog;
`.set` parsing and module behavior are unchanged. Invalid/oversized existing text is
still preserved by the underlying editor, with Apply unavailable instead of truncation.

Host-free tests compile the actual picker and draft:

```text
rustc --edition=2024 --test tools/test_registry_picker.rs -o registry-picker-tests
./registry-picker-tests
python tools/test_text_settings.py
```

The production model covers domain selection, registry deduplication, live resource
identity caching, token/namespace preservation, draft-only mutation, byte limits,
search/pagination bounds and compact layout. The inherited text tests cover actual
serializer/loader and logical navigation. Real registry API integration also needs the
pinned client compile check; physical GUI operation remains a separate runtime check.

MIT contribution, written with assistance from OpenAI Codex (GPT-6.1 Sol). No donor
implementation, new dependency, icon, font or image asset was used.
