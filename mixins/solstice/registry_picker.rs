// SPDX-License-Identifier: MIT
// Original registry picker model, assisted by OpenAI Codex (GPT-6.1 Sol).
// Only explicit draft changes are made here; the caller owns Apply/Cancel.
use super::solstice_text_edit::{Draft, EditKey, validate};
use std::sync::{Arc, Weak};

pub(super) const ROWS: usize = 8;
const QUERY_BYTES: usize = 128;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Kind { Blocks, BlockItems, Items }

// Match actual consumers, not a setting-name heuristic or a plugin's unknown field.
pub(super) fn field_kind(module: &str, setting: &str) -> Option<Kind> {
    match (module, setting) {
        ("Xray", "Blocks") => Some(Kind::Blocks),
        ("Surround" | "Self Trap" | "Auto Trap" | "Hole Filler" | "Spawn Proofer", "Blocks")
        | ("Liquid Filler", "Whitelist" | "Blacklist") => Some(Kind::BlockItems),
        ("Fast Use", "Items") => Some(Kind::Items),
        _ => None,
    }
}

#[derive(Debug)]
pub(super) struct Catalog { blocks: Vec<String>, block_items: Vec<String>, items: Vec<String> }
impl Catalog {
    pub fn new(blocks: impl IntoIterator<Item = String>, items: impl IntoIterator<Item = (String, bool)>) -> Self {
        fn unique(mut ids: Vec<String>) -> Vec<String> { ids.sort(); ids.dedup(); ids }
        let (mut all, mut block_items) = (Vec::new(), Vec::new());
        for (id, block) in items { if block { block_items.push(id.clone()); } all.push(id); }
        Self { blocks: unique(blocks.into_iter().collect()), block_items: unique(block_items), items: unique(all) }
    }
    fn ids(&self, kind: Kind) -> &[String] {
        match kind { Kind::Blocks => &self.blocks, Kind::BlockItems => &self.block_items, Kind::Items => &self.items }
    }
}

// Weak identity prevents a freed resource's reused address from returning an old catalog.
pub(super) struct Cache<T> { saved: Option<(Weak<T>, Arc<Catalog>)> }
impl<T> Cache<T> {
    pub const fn new() -> Self { Self { saved: None } }
    pub fn get(&mut self, resource: &Arc<T>, build: impl FnOnce() -> Catalog) -> Arc<Catalog> {
        if let Some((weak, catalog)) = &self.saved
            && let Some(previous) = weak.upgrade()
            && Arc::ptr_eq(&previous, resource)
        { return catalog.clone(); }
        let catalog = Arc::new(build());
        self.saved = Some((Arc::downgrade(resource), catalog.clone()));
        catalog
    }
}

fn short(id: &str) -> &str { id.strip_prefix("minecraft:").unwrap_or(id) }
pub(super) fn contains(raw: &str, id: &str) -> bool {
    raw.split(',').any(|token| short(token.trim()) == short(id))
}
pub(super) fn toggled(raw: &str, id: &str) -> Result<String, &'static str> {
    validate(raw)?;
    let next = if contains(raw, id) {
        // An explicit removal removes equivalent aliases/duplicates, leaving every
        // surviving token's bytes and order intact, including unknown namespaces.
        raw.split(',').filter(|token| short(token.trim()) != short(id)).collect::<Vec<_>>().join(",")
    } else {
        let separator = if raw.is_empty() || raw.trim_end().ends_with(',') { "" } else { "," };
        format!("{raw}{separator}{id}")
    };
    validate(&next)?;
    Ok(next)
}

#[derive(Clone, Debug)]
pub(super) struct Picker {
    catalog: Arc<Catalog>, kind: Kind,
    pub query: Draft,
    pub search_focus: bool,
    matches: Arc<[usize]>,
    pub cursor: usize,
}
impl Picker {
    pub fn new(catalog: Arc<Catalog>, kind: Kind) -> Self {
        let mut picker = Self { catalog, kind, query: Draft::new("").unwrap(), search_focus: false, matches: Arc::from([]), cursor: 0 };
        picker.refresh(); picker
    }
    fn refresh(&mut self) {
        let query = self.query.text().to_ascii_lowercase();
        self.matches = self.catalog.ids(self.kind).iter().enumerate()
            .filter_map(|(i, id)| id.to_ascii_lowercase().contains(&query).then_some(i)).collect();
        self.cursor = 0;
    }
    pub fn insert_query(&mut self, text: &str) {
        let retained = if self.query.selected() { 0 } else { self.query.text().len() };
        if text.len() > QUERY_BYTES.saturating_sub(retained) { self.query.error = Some("Search: maximum 128 UTF-8 bytes"); return; }
        let mut next = self.query.clone(); next.insert(text);
        let changed = next.text() != self.query.text(); self.query = next;
        if changed { self.refresh(); }
    }
    pub fn key_query(&mut self, key: EditKey) {
        let before = self.query.text().to_owned(); self.query.key(key);
        if before != self.query.text() { self.refresh(); }
    }
    pub fn move_cursor(&mut self, delta: isize) {
        self.cursor = self.cursor.saturating_add_signed(delta).min(self.matches.len().saturating_sub(1));
    }
    pub fn count(&self) -> usize { self.matches.len() }
    pub fn id(&self, index: usize) -> Option<&str> {
        self.matches.get(index).map(|&i| self.catalog.ids(self.kind)[i].as_str())
    }
    pub fn visible(&self, rows: usize) -> impl Iterator<Item = (usize, &str)> {
        let rows = rows.clamp(1, ROWS);
        let start = self.cursor / rows * rows;
        (start..(start + rows).min(self.count())).filter_map(move |i| self.id(i).map(|id| (i, id)))
    }
    pub fn toggle(&self, index: usize, draft: &mut Draft) {
        let Some(id) = self.id(index) else { return; };
        match toggled(draft.text(), id).and_then(|text| Draft::new(&text)) {
            Ok(next) => *draft = next,
            Err(error) => draft.error = Some(error),
        }
    }
}

#[derive(Clone, Copy)]
pub(super) struct Panel { pub rect: [f32; 4], pub browse: bool, pub rows: usize }
impl Panel {
    pub fn new(w: f32, h: f32, picker: bool) -> Self {
        let browse = picker && w >= 180.0 && h >= 146.0;
        let width = (w - 8.0).clamp(1.0, 320.0);
        let height = if browse { (h - 8.0).min(232.0) } else { 76.0 };
        let rows = if browse { ((height - 127.0) / 13.0).floor().max(1.0) as usize } else { 0 }.min(ROWS);
        Self { rect: [(w - width) / 2.0, ((h - height) / 2.0).max(0.0), width, height], browse, rows }
    }
    pub fn raw(&self) -> [f32; 4] { let [x,y,w,_] = self.rect; [x+6.0,y+20.0,w-12.0,14.0] }
    pub fn search(&self) -> [f32; 4] { let [x,y,w,_] = self.rect; [x+6.0,y+49.0,w-12.0,14.0] }
    pub fn row(&self, i: usize) -> [f32; 4] { let [x,y,w,_] = self.rect; [x+6.0,y+69.0+i as f32*13.0,w-12.0,12.0] }
    pub fn buttons(&self) -> ([f32;4],[f32;4]) {
        let [x,y,w,h] = self.rect; let width = ((w-18.0)/2.0).clamp(1.0,48.0);
        let top = y + if self.browse { h-33.0 } else { 42.0 };
        ([x+6.0,top,width,13.0],[x+12.0+width,top,width,13.0])
    }
    pub fn pages(&self) -> ([f32;4],[f32;4]) {
        let [x,y,_,h] = self.rect;
        ([x+6.0,y+h-51.0,48.0,13.0],[x+60.0,y+h-51.0,48.0,13.0])
    }
    pub fn footer(&self) -> f32 { self.rect[1] + self.rect[3] - 15.0 }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn catalog() -> Arc<Catalog> {
        Arc::new(Catalog::new(
            ["minecraft:wall_torch", "minecraft:water", "minecraft:wall_torch"].map(str::to_owned),
            [("minecraft:torch", true), ("minecraft:water_bucket", false)].map(|(id, block)| (id.to_owned(), block)),
        ))
    }
    #[test]
    fn actual_consumer_policy_separates_blocks_block_items_and_plain_text() {
        assert_eq!(field_kind("Xray", "Blocks"), Some(Kind::Blocks));
        for name in ["Surround", "Self Trap", "Auto Trap", "Hole Filler", "Spawn Proofer"] {
            assert_eq!(field_kind(name, "Blocks"), Some(Kind::BlockItems));
        }
        assert_eq!(field_kind("Liquid Filler", "Whitelist"), Some(Kind::BlockItems));
        assert_eq!(field_kind("Liquid Filler", "Blacklist"), Some(Kind::BlockItems));
        assert_eq!(field_kind("Fast Use", "Items"), Some(Kind::Items));
        assert_eq!(field_kind("Spam", "Messages"), None);
        assert_eq!(field_kind("Auto Mount", "Entities"), None);
        assert_eq!(field_kind("Unknown plugin", "Blocks"), None);
    }
    #[test]
    fn catalog_deduplicates_state_variants_without_inventing_block_items() {
        let catalog = catalog();
        assert_eq!(Picker::new(catalog.clone(), Kind::Blocks).count(), 2);
        let block_items = Picker::new(catalog.clone(), Kind::BlockItems);
        assert_eq!(block_items.count(), 1);
        assert_eq!(block_items.id(0), Some("minecraft:torch"));
        let items = Picker::new(catalog, Kind::Items);
        assert_eq!(items.count(), 2);
        assert_eq!(items.id(1), Some("minecraft:water_bucket"));
    }
    #[test]
    fn catalog_cache_builds_only_for_a_new_live_resource_identity() {
        let mut cache = Cache::new();
        let first = Arc::new(());
        let mut builds = 0;
        let mut build = || { builds += 1; Catalog::new([], []) };
        let a = cache.get(&first, &mut build);
        let b = cache.get(&first.clone(), &mut build);
        assert!(Arc::ptr_eq(&a, &b));
        let second = Arc::new(());
        let c = cache.get(&second, &mut build);
        assert!(!Arc::ptr_eq(&a, &c));
        drop(first); drop(second);
        let third = Arc::new(());
        cache.get(&third, &mut build);
        assert_eq!(builds, 3);
    }
    #[test]
    fn explicit_toggle_preserves_unknown_tokens_namespaces_order_and_whitespace() {
        let raw = " obsidian , custom:obsidian , #minecraft:logs ,minecraft:obsidian,  keep=this  ";
        assert!(contains(raw, "minecraft:obsidian"));
        assert!(contains(raw, "custom:obsidian"));
        let removed = toggled(raw, "minecraft:obsidian").unwrap();
        assert_eq!(removed, " custom:obsidian , #minecraft:logs ,  keep=this  ");
        assert!(!contains(&removed, "minecraft:obsidian"));
        let added = toggled(&removed, "minecraft:stone").unwrap();
        assert_eq!(added, format!("{removed},minecraft:stone"));
        assert_eq!(toggled("stone,  ", "minecraft:torch").unwrap(), "stone,  minecraft:torch");
        assert_eq!(toggled("  ", "minecraft:torch").unwrap(), "  ,minecraft:torch");
        assert!(!contains("STONE,custom:stone", "minecraft:stone")); // Consumer comparison is case-sensitive.
    }
    #[test]
    fn picker_changes_only_draft_and_failed_inserts_never_truncate() {
        let picker = Picker::new(catalog(), Kind::Items);
        let stored = "unknown:custom,  alias ";
        let mut draft = Draft::new(stored).unwrap();
        picker.toggle(0, &mut draft);
        assert_eq!(stored, "unknown:custom,  alias ");
        assert_eq!(draft.text(), "unknown:custom,  alias ,minecraft:torch");
        drop(draft); // Cancel leaves the setting untouched.
        let raw = "x".repeat(4096);
        let mut draft = Draft::new(&raw).unwrap();
        picker.toggle(0, &mut draft);
        assert_eq!(draft.text(), raw);
        assert!(draft.error.is_some());
        assert!(toggled("invalid\ntext", "minecraft:torch").is_err());
    }
    #[test]
    fn search_is_separate_cached_bounded_and_keyboard_pages_are_reachable() {
        let catalog = Arc::new(Catalog::new((0..25).map(|i| format!("minecraft:block_{i:02}")), []));
        let mut picker = Picker::new(catalog, Kind::Blocks);
        assert_eq!(picker.visible(1000).count(), ROWS);
        picker.move_cursor(24);
        assert_eq!(picker.visible(ROWS).collect::<Vec<_>>(), vec![(24, "minecraft:block_24")]);
        picker.move_cursor(1000);
        assert_eq!(picker.cursor, 24);
        picker.insert_query("BLOCK_1");
        assert_eq!(picker.count(), 10);
        assert_eq!(picker.cursor, 0);
        assert!(picker.visible(ROWS).all(|(_, id)| id.contains("block_1")));
        picker.key_query(EditKey::SelectAll); picker.insert_query("not-present");
        assert_eq!(picker.count(), 0);
        let mut draft = Draft::new("unknown:preserved").unwrap();
        picker.toggle(0, &mut draft);
        assert_eq!(draft.text(), "unknown:preserved");
    }
    #[test]
    fn search_byte_limit_rejects_whole_insert_and_allows_recovery() {
        let mut picker = Picker::new(catalog(), Kind::Items);
        picker.insert_query(&"é".repeat(64));
        let before = picker.query.text().to_owned();
        picker.insert_query("x");
        assert_eq!(picker.query.text(), before);
        assert!(picker.query.error.is_some());
        picker.insert_query(&"x".repeat(5000));
        assert_eq!(picker.query.error, Some("Search: maximum 128 UTF-8 bytes"));
        assert_eq!(picker.query.text(), before);
        picker.key_query(EditKey::SelectAll); picker.insert_query("torch");
        assert_eq!(picker.query.text(), "torch");
        assert_eq!(picker.id(0), Some("minecraft:torch"));
    }
    #[test]
    fn compact_panel_bounds_rows_and_falls_back_to_raw_for_tiny_viewports() {
        for (w, h) in [(180.,146.), (320.,180.), (427.,240.), (1920.,1080.)] {
            let panel = Panel::new(w,h,true);
            assert!(panel.browse && (1..=ROWS).contains(&panel.rows));
            let (apply,cancel) = panel.buttons(); let (previous,next) = panel.pages();
            for rect in [panel.raw(),panel.search(),panel.row(panel.rows-1),apply,cancel,previous,next] {
                assert!(rect[0] >= panel.rect[0] && rect[1] >= panel.rect[1]);
                assert!(rect[0]+rect[2] <= panel.rect[0]+panel.rect[2]);
                assert!(rect[1]+rect[3] <= panel.rect[1]+panel.rect[3]);
            }
            assert!(panel.row(panel.rows-1)[1]+12.0 < previous[1]);
        }
        assert!(!Panel::new(160.,120.,true).browse);
        assert!(!Panel::new(427.,240.,false).browse);
    }
}
