// SPDX-License-Identifier: MIT
// Host vocabulary only; adapter bodies, classification and startup guard are extracted verbatim.
use std::cell::{Cell, RefCell};
use std::marker::PhantomData;
use super::solstice_scan::Shared;
const ID: &str = "solstice";
#[derive(Clone)]
struct Module { name: &'static str, on: bool, range: f64, preset: &'static str, fill: bool }
impl Module {
    fn n(&self, name: &str) -> f64 { assert_eq!(name,"Range"); self.range }
    fn p(&self, name: &str) -> &str { assert_eq!(name,"Blocks"); self.preset }
}
struct Fact { name: &'static str }
struct Facts { entries: Vec<Fact> }
impl Facts {
    fn is_air(&self, state: usize) -> bool { state==0 }
    fn get(&self, state: usize) -> &Fact { &self.entries[state] }
}
struct Resources { facts: Facts, accesses: Cell<usize> }
impl Resources { fn facts(&self) -> &Facts { self.accesses.set(self.accesses.get()+1); &self.facts } }
struct World { reads: Cell<usize>, material: Cell<usize> }
impl World {
    fn block(&self, x: i32, y: i32, z: i32) -> usize {
        self.reads.set(self.reads.get()+1);
        if x.rem_euclid(2)==0 && z.rem_euclid(2)==0 && y.rem_euclid(3)!=0 { 0 } else { self.material.get() }
    }
}
struct NativePlayer<'a> { x: f64, y: f64, z: f64, dimension: &'a str }
pub(super) struct Game<'a> {
    client: bool, enabled: bool, player: Option<[f64;3]>, title: Option<()>, dimension: String,
    world: World, res: Resources, config_loads: Cell<usize>, lifetime: PhantomData<&'a ()>,
}
impl Game<'_> {
    fn native_window(&self) -> bool { self.client }
    fn native_on(&self, id: &str) -> bool { assert_eq!(id,ID); self.enabled }
    fn native_player(&self) -> Option<NativePlayer<'_>> {
        let [x,y,z]=self.player?; Some(NativePlayer{x,y,z,dimension:&self.dimension})
    }
}
fn game() -> Game<'static> {
    Game { client:true, enabled:true, player:Some([0.;3]),title:None,dimension:"overworld".into(),
        world:World{reads:Cell::new(0),material:Cell::new(1)},
        res:Resources{facts:Facts{entries:["minecraft:air","minecraft:stone","minecraft:diamond_ore","minecraft:crying_obsidian","minecraft:bedrock","minecraft:tall_grass","minecraft:water","custom:foo_obsidian"].map(|name|Fact{name}).into()},accesses:Cell::new(0)},
        config_loads:Cell::new(0),lifetime:PhantomData }
}
struct State { scans: Shared, modules: Vec<Module> }
thread_local! { static STATE: RefCell<State> = RefCell::new(State{scans:Shared::default(),modules:vec![]}); }
fn with<R>(f: impl FnOnce(&mut State)->R) -> R { STATE.with(|state|f(&mut state.borrow_mut())) }
fn snapshot() -> Vec<Module> { with(|state|state.modules.clone()) }
fn ensure_loaded(game: &Game<'_>) { game.config_loads.set(game.config_loads.get()+1); }
fn modules() -> Vec<Module> {
    vec![Module{name:"Block ESP",on:true,range:8.,preset:"Ores",fill:true},Module{name:"Hole ESP",on:true,range:3.,preset:"",fill:true}]
}
fn reset() { with(|state| { state.scans=Shared::default(); state.modules=modules(); }); }

#[test]
fn actual_startup_prefix_never_loads_config_for_disabled_or_server_games() {
    reset(); let mut game=game(); game.client=false;
    post_tick_guard_prefix(&mut game); assert_eq!(game.config_loads.get(),0);
    game.client=true; game.enabled=false;
    post_tick_guard_prefix(&mut game); assert_eq!(game.config_loads.get(),0);
    game.enabled=true; post_tick_guard_prefix(&mut game); assert_eq!(game.config_loads.get(),1);
}
#[test]
fn actual_adapters_are_read_free_on_frame_draw_and_ineligible_tick_contexts() {
    reset(); let mut game=game();
    render_scan_frame(&game); observe_render_scans(&game,&snapshot());
    assert_eq!(game.world.reads.get(),0); assert_eq!(game.res.accesses.get(),0);
    for reason in 0..6 {
        with(|state|state.modules=modules());
        game.client=reason!=0; game.enabled=reason!=1;
        game.player=if reason==2 {None} else if reason==5 {Some([f64::NAN,0.,0.])} else {Some([0.;3])};
        game.title=(reason==3).then_some(());
        if reason==4 { with(|state|state.modules.iter_mut().for_each(|module|module.on=false)); }
        render_scan_frame(&game); observe_render_scans(&game,&snapshot()); render_scan_tick(&game,&snapshot(),reason+1);
        assert_eq!(game.world.reads.get(),0,"reason {reason}");
        assert_eq!(game.res.accesses.get(),0,"reason {reason}");
        assert_eq!(game.config_loads.get(),0);
    }
}
#[test]
fn actual_tick_adapter_invokes_one_shared_allowance_and_render_changes_only_observe() {
    reset(); let game=game(); let modules=snapshot();
    render_scan_tick(&game,&modules,1);
    let before=game.world.reads.get(); assert!(before>0 && before<=4096);
    render_scan_tick(&game,&modules,1); assert_eq!(game.world.reads.get(),before);
    with(|state| { state.scans.block.found=vec![([1;3],1)]; state.scans.hole.found=vec![([2;3],2)]; state.modules[1].fill=false; });
    observe_render_scans(&game,&snapshot());
    with(|state| { assert_eq!(state.scans.hole.found.len(),1); state.modules[0].range=12.; });
    render_scan_frame(&game);
    with(|state| { assert!(state.scans.block.found.is_empty()); assert_eq!(state.scans.hole.found.len(),1); state.modules[1].on=false; });
    render_scan_frame(&game);
    with(|state|assert!(state.scans.hole.found.is_empty()));
    assert_eq!(game.world.reads.get(),before);
    clear_render_scans();
    with(|state|assert_eq!(state.scans.epoch(),1));
}
#[test]
fn actual_hole_classifier_preserves_suffix_safety_and_nonair_plants_fluids() {
    for (material,expected) in [(1,0xFFFF_FF55),(3,0xFF55_FF55),(4,0xFF55_FF55),(5,0xFFFF_FF55),(6,0xFFFF_FF55),(7,0xFF55_FF55)] {
        reset(); with(|state|state.modules[0].on=false);
        let game=game(); game.world.material.set(material);
        let mut complete=false;
        for tick in 1..1000 {
            render_scan_tick(&game,&snapshot(),tick);
            if with(|state|!state.scans.hole.found.is_empty()) { complete=true; break; }
        }
        assert!(complete); with(|state|assert!(state.scans.hole.found.iter().all(|(_,colour)|*colour==expected)));
    }
}
