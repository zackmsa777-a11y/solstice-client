// Bounded, host-free Block/Hole ESP scan model. MIT; AI-assisted implementation by Codex.
// The caller supplies read-only loaded-world access and a monotonic deadline.
pub(super) const CELLS_PER_TICK: usize = 4096;
pub(super) const CLOCK_BATCH: usize = 64;
pub(super) const MAX_RESULTS: usize = 2000;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct Request {
    pub dimension: String,
    pub preset: usize,
    pub range: i32,
    pub center: [i32; 3],
}

// Reject corrupt config/poses; valid UI ranges retain the old truncation semantics.
pub(super) fn scan_range(value: f64) -> Option<i32> {
    (value.is_finite() && (8.0..=48.0).contains(&value)).then_some(value as i32)
}
pub(super) fn scan_center(value: [f64; 3]) -> Option<[i32; 3]> {
    value
        .iter()
        .all(|v| {
            v.is_finite() && v.floor() >= f64::from(i32::MIN) && v.floor() <= f64::from(i32::MAX)
        })
        .then(|| value.map(|v| v.floor() as i32))
}
fn distance(a: [i32; 3], b: [i32; 3]) -> i64 {
    (0..3)
        .map(|i| (i64::from(a[i]) - i64::from(b[i])).abs())
        .max()
        .unwrap_or(0)
}
impl Request {
    fn same_config(&self, other: &Self) -> bool {
        self.dimension == other.dimension
            && self.preset == other.preset
            && self.range == other.range
    }
}

// Old enumeration order: x outermost, then z, then y; inclusive -64..320 clamp.
struct Cells {
    lo: [i32; 3],
    hi: [i32; 3],
    next: Option<[i32; 3]>,
}
impl Cells {
    fn new(request: &Request) -> Self {
        let r = request.range.clamp(8, 48);
        let lo = [
            request.center[0].saturating_sub(r),
            request.center[1].saturating_sub(r).max(-64),
            request.center[2].saturating_sub(r),
        ];
        let hi = [
            request.center[0].saturating_add(r),
            request.center[1].saturating_add(r).min(320),
            request.center[2].saturating_add(r),
        ];
        Self {
            lo,
            hi,
            next: (lo[1] <= hi[1]).then_some(lo),
        }
    }
    fn bounds(lo: [i32; 3], hi: [i32; 3]) -> Self {
        Self { lo, hi, next: (lo[1] <= hi[1]).then_some(lo) }
    }
    fn next_column(&mut self, at: [i32; 3]) {
        self.next = if at[2] < self.hi[2] {
            Some([at[0], self.lo[1], at[2] + 1])
        } else if at[0] < self.hi[0] {
            Some([at[0] + 1, self.lo[1], self.lo[2]])
        } else { None };
    }
    fn skip_column(&mut self, at: [i32; 3]) {
        if self.next.is_some_and(|next| next[0] == at[0] && next[2] == at[2]) {
            self.next_column(at);
        }
    }
    fn take(&mut self) -> Option<[i32; 3]> {
        let at = self.next?;
        if at[1] < self.hi[1] { self.next = Some([at[0], at[1] + 1, at[2]]); }
        else { self.next_column(at); }
        Some(at)
    }
}
struct Cycle {
    request: Request,
    cells: Cells,
    found: Vec<([i32; 3], u32)>,
}
#[derive(Default)]
pub(super) struct Scan {
    latest: Option<Request>,
    cycle: Option<Cycle>,
    published_for: Option<Request>,
    published_at: u64,
    pub found: Vec<([i32; 3], u32)>,
}
impl Scan {
    pub fn clear(&mut self) {
        *self = Self::default();
    }
    // Called before drawing as well as scanning: never draw old settings/dimensions.
    pub fn observe(&mut self, request: Request) {
        if self.latest.as_ref().is_some_and(|last| {
            !last.same_config(&request)
                || distance(last.center, request.center) >= i64::from(request.range) * 2
        }) {
            self.clear();
        }
        self.latest = Some(request);
    }
    fn slice(&mut self, ticks: u64, lease: usize, read: &mut impl FnMut([i32;3]) -> Option<u32>, expired: &mut impl FnMut() -> bool) -> Step {
        if lease == 0 { return Step::default(); }
        if self.cycle.is_none() {
            let Some(request) = self.latest.as_ref() else { return Step::default(); };
            let due = self.published_for.as_ref().is_none_or(|last| {
                self.published_at != ticks && (distance(last.center, request.center) >= 6 || ticks.saturating_sub(self.published_at) >= 100)
            });
            if !due { return Step::default(); }
            self.cycle = Some(Cycle { cells: Cells::new(request), request: request.clone(), found: Vec::new() });
        }
        let mut step = Step::default();
        let cycle = self.cycle.as_mut().unwrap();
        while step.reads < lease && cycle.cells.next.is_some() {
            if step.reads % CLOCK_BATCH == 0 && expired() { step.expired = true; break; }
            let at = cycle.cells.take().unwrap();
            step.reads += 1;
            if let Some(colour) = read(at) { cycle.found.push((at, colour)); }
            if cycle.found.len() == MAX_RESULTS { break; }
        }
        step.progress = step.reads != 0;
        if cycle.cells.next.is_none() || cycle.found.len() == MAX_RESULTS {
            let done = self.cycle.take().unwrap();
            self.found = done.found; self.published_for = Some(done.request); self.published_at = ticks;
            step.progress = true;
        }
        step
    }
    // Legacy pilot harness exercises the same production slice with one block-only lease.
    #[cfg(test)]
    pub fn tick(&mut self, request: Request, ticks: u64, mut read: impl FnMut([i32;3]) -> Option<u32>, mut expired: impl FnMut() -> bool) -> usize {
        self.observe(request);
        self.slice(ticks, CELLS_PER_TICK, &mut read, &mut expired).reads
    }
}

#[derive(Default)]
struct Step { reads: usize, progress: bool, expired: bool }

pub(super) fn hole_range(value: f64) -> Option<i32> {
    (value.is_finite() && (3.0..=16.0).contains(&value)).then_some(value as i32)
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct HoleRequest { pub dimension: String, pub range: i32, pub center: [i32;3] }
impl HoleRequest {
    pub fn valid(&self) -> bool { self.cells().is_some() }
    fn cells(&self) -> Option<Cells> {
        if !(3..=16).contains(&self.range) { return None; }
        let r = self.range;
        let lo = [self.center[0].checked_sub(r)?, self.center[1].checked_sub(r)?.max(-63), self.center[2].checked_sub(r)?];
        let hi = [self.center[0].checked_add(r)?, self.center[1].checked_add(r)?, self.center[2].checked_add(r)?];
        // Every admitted candidate's neighbor arithmetic is then safe and distinct.
        for i in 0..3 { lo[i].checked_sub(1)?; hi[i].checked_add(1)?; }
        Some(Cells::bounds(lo, hi))
    }
    fn same_config(&self, other: &Self) -> bool { self.dimension == other.dimension && self.range == other.range }
}
struct HoleCycle { request: HoleRequest, cells: Cells, found: Vec<([i32;3], u32)> }
#[derive(Default)]
pub(super) struct HoleScan {
    latest: Option<HoleRequest>, cycle: Option<HoleCycle>, published_for: Option<HoleRequest>, published_at: u64,
    pub found: Vec<([i32;3], u32)>,
}
impl HoleScan {
    pub fn clear(&mut self) { *self = Self::default(); }
    pub fn observe(&mut self, request: HoleRequest) {
        if !request.valid() { self.clear(); return; }
        if self.latest.as_ref().is_some_and(|last| !last.same_config(&request) || distance(last.center, request.center) >= i64::from(request.range)*2) { self.clear(); }
        self.latest = Some(request);
    }
    fn slice(&mut self, ticks: u64, lease: usize, read: &mut impl FnMut([i32;3]) -> Option<bool>, expired: &mut impl FnMut() -> bool) -> Step {
        if lease == 0 { return Step::default(); }
        if self.cycle.is_none() {
            let Some(request) = self.latest.as_ref() else { return Step::default(); };
            if self.published_for.is_some() && ticks.saturating_sub(self.published_at) < 20 { return Step::default(); }
            self.cycle = Some(HoleCycle { request: request.clone(), cells: request.cells().unwrap(), found: Vec::new() });
        }
        let mut step = Step::default();
        let cycle = self.cycle.as_mut().unwrap();
        while cycle.cells.next.is_some() && lease - step.reads >= 7 {
            if expired() { step.expired = true; break; }
            let at = cycle.cells.take().unwrap();
            let (used, colour) = hole_candidate(at, read);
            step.reads += used;
            if let Some(colour) = colour {
                cycle.found.push((at, colour));
                // The frozen old loop breaks only its Y column after >=400.
                if cycle.found.len() >= 400 { cycle.cells.skip_column(at); }
            }
        }
        step.progress = step.reads != 0;
        if cycle.cells.next.is_none() {
            let done = self.cycle.take().unwrap();
            self.found = done.found; self.published_for = Some(done.request); self.published_at = ticks;
            step.progress = true;
        }
        step
    }
}
fn hole_candidate([x,y,z]: [i32;3], read: &mut impl FnMut([i32;3]) -> Option<bool>) -> (usize, Option<u32>) {
    if read([x,y,z]).is_some() { return (1, None); }
    if read([x,y+1,z]).is_some() { return (2, None); }
    // Eager, ordered probes match the original array expression even on rejection.
    let around = [read([x,y-1,z]), read([x+1,y,z]), read([x-1,y,z]), read([x,y,z+1]), read([x,y,z-1])];
    let colour = if around.iter().any(Option::is_none) { None }
        else if around.iter().all(|value| *value == Some(true)) { Some(0xFF55_FF55) }
        else { Some(0xFFFF_FF55) };
    (7, colour)
}

// Both requests carry one world epoch; resetting cannot double the same tick's allowance.
pub(super) struct Requests { pub epoch: u64, pub block: Option<Request>, pub hole: Option<HoleRequest> }
#[derive(Default, Debug, PartialEq, Eq)]
pub(super) struct Reads { pub block: usize, pub hole: usize }
impl Reads { pub fn total(&self) -> usize { self.block + self.hole } }
#[derive(Default)]
pub(super) struct Shared {
    pub block: Scan, pub hole: HoleScan, epoch: u64, last_tick: Option<u64>, hole_first: bool,
}
impl Shared {
    pub fn epoch(&self) -> u64 { self.epoch }
    pub fn clear_jobs(&mut self) { self.block.clear(); self.hole.clear(); }
    pub fn reset_world(&mut self) {
        self.clear_jobs(); self.epoch = self.epoch.wrapping_add(1); self.hole_first = false;
        // State.ticks is monotonic across worlds. Preserve the guard through resets.
    }
    pub fn observe(&mut self, requests: Requests) {
        if requests.epoch != self.epoch { self.clear_jobs(); return; }
        match requests.block { Some(request) => self.block.observe(request), None => self.block.clear() }
        match requests.hole { Some(request) => self.hole.observe(request), None => self.hole.clear() }
    }
    pub fn tick(&mut self, requests: Requests, ticks: u64, mut block_read: impl FnMut([i32;3]) -> Option<u32>, mut hole_read: impl FnMut([i32;3]) -> Option<bool>, mut expired: impl FnMut() -> bool) -> Reads {
        self.observe(requests);
        if self.last_tick.is_some_and(|last| ticks <= last) { return Reads::default(); }
        self.last_tick = Some(ticks);
        let mut hole_turn = self.hole_first; self.hole_first = !self.hole_first;
        let mut reads = Reads::default(); let mut idle = 0;
        while reads.total() < CELLS_PER_TICK && idle < 2 {
            let lease = CLOCK_BATCH.min(CELLS_PER_TICK - reads.total());
            let step = if hole_turn { self.hole.slice(ticks, lease, &mut hole_read, &mut expired) }
                else { self.block.slice(ticks, lease, &mut block_read, &mut expired) };
            if hole_turn { reads.hole += step.reads; } else { reads.block += step.reads; }
            hole_turn = !hole_turn;
            idle = if step.progress { 0 } else { idle+1 };
            if step.expired { break; }
        }
        reads
    }
}

#[cfg(test)]
mod shared_tests {
    use super::*;
    use std::cell::{Cell, RefCell};
    fn block(range: i32) -> Request { Request { dimension: "overworld".into(), preset: 0, range, center: [0;3] } }
    fn hole(range: i32) -> HoleRequest { HoleRequest { dimension: "overworld".into(), range, center: [0;3] } }
    fn requests(shared: &Shared, b: bool, h: bool) -> Requests {
        Requests { epoch: shared.epoch(), block: b.then(|| block(48)), hole: h.then(|| hole(16)) }
    }
    // Frozen old inner loops from 5b3406e runtime.rs: scan_holes. This oracle
    // deliberately shares no cursor, admission, publication or scheduling code.
    fn old_holes([px,py,pz]: [i32;3], r: i32, mut hard: impl FnMut([i32;3]) -> Option<bool>) -> Vec<([i32;3],u32)> {
        let mut found = Vec::new();
        for x in px-r..=px+r { for z in pz-r..=pz+r { for y in (py-r).max(-63)..=py+r {
            if hard([x,y,z]).is_some() || hard([x,y+1,z]).is_some() { continue; }
            let around = [hard([x,y-1,z]), hard([x+1,y,z]), hard([x-1,y,z]), hard([x,y,z+1]), hard([x,y,z-1])];
            if around.iter().any(Option::is_none) { continue; }
            let safe = around.iter().all(|value| *value == Some(true));
            found.push(([x,y,z], if safe { 0xFF55_FF55 } else { 0xFFFF_FF55 }));
            if found.len() >= 400 { break; }
        } } }
        found
    }
    fn terrain([x,y,z]: [i32;3], variant: usize) -> Option<bool> {
        if variant == 3 && !(-64..320).contains(&y) { return None; }
        if x.rem_euclid(2) == 0 && z.rem_euclid(2) == 0 && y.rem_euclid(3) != 0 { return None; }
        if variant == 1 && x.rem_euclid(5) == 1 { return None; } // Missing enclosing sides/floors.
        Some(variant != 2 || (x+y+z).rem_euclid(5) != 0) // Non-air includes unsafe fluids/plants.
    }
    fn complete_hole(request: HoleRequest, mut read: impl FnMut([i32;3]) -> Option<bool>) -> HoleScan {
        let mut job = HoleScan::default(); job.observe(request);
        for tick in 1..1000 {
            assert!(job.slice(tick, CELLS_PER_TICK, &mut read, &mut || false).reads <= CELLS_PER_TICK);
            if job.published_for.is_some() { return job; }
        }
        panic!("hole cycle did not complete");
    }
    #[test]
    fn frozen_hole_oracle_all_ranges_negative_and_vertical_boundaries() {
        for r in 3..=16 { for y in [-80,-64,-63,0,319,320,400] { for variant in 0..4 {
            let center = [-7,y,-11];
            let mut old_reads = 0;
            let expected = old_holes(center,r,|at| { old_reads+=1; terrain(at,variant) });
            let mut new_reads = 0;
            let job = complete_hole(HoleRequest { center, ..hole(r) }, |at| { new_reads+=1; terrain(at,variant) });
            assert_eq!(job.found,expected,"range {r}, y {y}, fixture {variant}");
            assert_eq!(new_reads,old_reads,"read-order/early-rejection volume changed");
        } } }
    }
    #[test]
    fn dense_threshold_skips_only_current_y_column_and_exceeds_400() {
        let expected = old_holes([0;3],16,|at| terrain(at,0));
        assert!(expected.len() > 400);
        let job = complete_hole(hole(16),|at| terrain(at,0));
        assert_eq!(job.found,expected);
        assert!(job.found.len() <= 399+33*33);
        assert!(job.found[400..].windows(2).all(|pair| (pair[0].0[0],pair[0].0[2]) != (pair[1].0[0],pair[1].0[2])));
    }
    #[test]
    fn candidate_probes_are_eager_ordered_and_cost_exactly_one_two_or_seven() {
        let at = [10,20,30];
        let expected = vec![at,[10,21,30],[10,19,30],[11,20,30],[9,20,30],[10,20,31],[10,20,29]];
        for reject in [0,1,7] {
            let mut visits = Vec::new();
            let (used,colour) = hole_candidate(at,&mut |pos| { let i=visits.len(); visits.push(pos); if i<reject.min(2) { None } else { Some(true) } });
            let cost = match reject { 0=>1,1=>2,_=>7 };
            assert_eq!(used,cost); assert_eq!(visits,&expected[..cost]);
            assert_eq!(colour,(cost==7).then_some(0xFF55_FF55));
        }
        for missing in 2..7 {
            let mut i=0;
            let (used,colour)=hole_candidate(at,&mut |_| { let n=i; i+=1; if n<2 || n==missing { None } else { Some(true) } });
            assert_eq!((used,colour),(7,None));
        }
    }
    #[test]
    fn insufficient_hole_leases_never_consume_a_candidate_and_deadline_does_not_split_it() {
        let mut job = HoleScan::default(); job.observe(hole(3));
        for lease in 0..7 {
            let step=job.slice(1,lease,&mut |_| panic!("unadmitted read"),&mut || false);
            assert_eq!(step.reads,0);
            assert!(job.cycle.as_ref().is_none_or(|cycle| cycle.cells.next==Some([-3,-3,-3])));
        }
        let expired=Cell::new(false); let reads=Cell::new(0);
        let step=job.slice(2,64,&mut |_| { let i=reads.get(); reads.set(i+1); expired.set(true); if i<2 { None } else { Some(true) } },&mut || expired.get());
        assert_eq!(step.reads,7); assert_eq!(reads.get(),7);
        assert!(step.expired);
        assert_eq!(job.cycle.as_ref().unwrap().cells.next,Some([-3,-2,-3]));
        assert_eq!(job.cycle.as_ref().unwrap().found.len(),1);
        assert!(job.found.is_empty());
        assert_eq!(job.slice(3,64,&mut |_| panic!("deadline"),&mut || true).reads,0);
    }
    #[test]
    fn two_jobs_share_actual_read_cap_and_block_uses_remainder_below_seven() {
        let mut shared=Shared::default();
        for tick in 1..=8 {
            let count=Cell::new(0); let order=RefCell::new(Vec::new());
            let used=shared.tick(requests(&shared,true,true),tick,
                |_| { count.set(count.get()+1); order.borrow_mut().push('B'); None },
                |_| { count.set(count.get()+1); order.borrow_mut().push('H'); Some(false) }, || false);
            assert_eq!(used.total(),count.get()); assert_eq!(used.total(),CELLS_PER_TICK);
            assert!(used.block>0 && used.hole>0);
            assert_eq!(order.borrow()[0],if tick%2==1 {'B'} else {'H'});
        }
        // Admission of whole seven-read candidates leaves a short remainder for Block.
        let mut shared=Shared::default(); shared.hole_first=true;
        let used=shared.tick(requests(&shared,true,true),1,|_|None,|_|None,||false);
        assert_eq!(used.total(),CELLS_PER_TICK); assert_eq!(used.hole%7,0);
        assert!((1..7).contains(&(used.block%CLOCK_BATCH)));
    }
    #[test]
    fn absent_and_not_due_jobs_yield_their_allowance_without_stalling() {
        let mut shared=Shared::default(); shared.hole_first=true;
        let used=shared.tick(requests(&shared,true,false),1,|_|None,|_|panic!(),||false);
        assert_eq!(used,Reads{block:4096,hole:0});
        let mut shared=Shared::default();
        let used=shared.tick(requests(&shared,false,true),1,|_|panic!(),|_|None,||false);
        assert!(used.hole<=4096 && 4096-used.hole<7); assert_eq!(used.block,0);
        let mut shared=Shared::default();
        let req=Requests{epoch:0,block:Some(block(48)),hole:Some(hole(3))};
        shared.tick(req,1,|_|None,|_|Some(false),||false);
        assert!(shared.hole.published_for.is_some());
        let req=Requests{epoch:0,block:Some(block(48)),hole:Some(hole(3))};
        assert_eq!(shared.tick(req,2,|_|None,|_|panic!("not due"),||false),Reads{block:4096,hole:0});
    }
    #[test]
    fn deadline_fairness_expired_start_duplicate_stamps_and_world_resets() {
        let mut shared=Shared::default();
        for tick in 1..=6 {
            let checks=Cell::new(0);
            let used=shared.tick(requests(&shared,true,true),tick,|_|None,|_|Some(false),|| { let n=checks.get(); checks.set(n+1); n!=0 });
            if tick%2==1 { assert!(used.block>0 && used.hole==0); } else { assert!(used.hole>0 && used.block==0); }
            assert_eq!(shared.tick(requests(&shared,true,true),tick,|_|panic!(),|_|panic!(),||false).total(),0);
        }
        let old_epoch=shared.epoch(); shared.reset_world();
        assert_eq!(shared.tick(Requests{epoch:old_epoch,block:Some(block(48)),hole:Some(hole(16))},6,|_|panic!(),|_|panic!(),||false).total(),0);
        assert_eq!(shared.tick(requests(&shared,true,true),6,|_|panic!(),|_|panic!(),||false).total(),0);
        assert_eq!(shared.tick(requests(&shared,true,true),7,|_|panic!(),|_|panic!(),||true).total(),0);
        assert!(shared.tick(requests(&shared,true,true),8,|_|None,|_|Some(false),||false).total()>0);
    }
    #[test]
    fn request_invalidation_is_per_job_and_epoch_invalidates_both_without_reads() {
        let mut shared=Shared::default();
        shared.tick(requests(&shared,true,true),1,|_|Some(1),|_|Some(false),||false);
        shared.hole.found=vec![([1;3],2)];
        let mut b=block(48); b.preset=1;
        shared.observe(Requests{epoch:0,block:Some(b.clone()),hole:Some(hole(16))});
        assert!(shared.block.found.is_empty()); assert_eq!(shared.hole.found.len(),1);
        shared.block.found=vec![([2;3],3)];
        let mut h=hole(16); h.dimension="nether".into();
        shared.observe(Requests{epoch:0,block:Some(b),hole:Some(h)});
        assert!(shared.hole.found.is_empty()); assert_eq!(shared.block.found,vec![([2;3],3)]);
        shared.observe(Requests{epoch:0,block:None,hole:None});
        assert!(shared.block.found.is_empty() && shared.hole.found.is_empty());
        shared.reset_world(); assert_eq!(shared.epoch(),1);
    }
    #[test]
    fn hole_publication_is_atomic_and_twenty_tick_delay_counts_after_completion() {
        let mut job=complete_hole(hole(8),|at|terrain(at,0));
        let old=job.found.clone(); let published=job.published_at;
        assert_eq!(job.slice(published+19,64,&mut |_|panic!("early refresh"),&mut ||false).reads,0);
        assert!(job.slice(published+20,7,&mut |_|Some(false),&mut ||false).reads>0);
        assert_eq!(job.found,old);
        for tick in published+21..published+100 {
            job.slice(tick,4096,&mut |_|Some(false),&mut ||false);
            if job.published_at!=published { assert!(job.found.is_empty()); return; }
        }
        panic!("empty refresh not published");
    }
    #[test]
    fn walking_keeps_hole_cycle_center_until_completion_then_uses_latest() {
        let mut job=HoleScan::default(); let mut published=None;
        for tick in 0..1000 {
            job.observe(HoleRequest{center:[tick,0,0],..hole(8)});
            job.slice(tick as u64,64,&mut |_|Some(false),&mut ||false);
            if let Some(request)=&job.published_for { assert_eq!(request.center,[0;3]); published=Some(tick); break; }
        }
        let tick=published.expect("walking starved") as u64;
        for now in tick+1..tick+20 {
            job.observe(HoleRequest{center:[now as i32,0,0],..hole(8)});
            assert_eq!(job.slice(now,64,&mut |_|panic!("early walking refresh"),&mut ||false).reads,0);
        }
        job.observe(HoleRequest{center:[(tick+20) as i32,0,0],..hole(8)});
        let mut first=None;
        job.slice(tick+20,7,&mut |at| { first.get_or_insert(at); Some(false) },&mut ||false);
        assert_eq!(first,Some([(tick+20) as i32-8,-8,-8]));
        assert_eq!(job.published_for.as_ref().unwrap().center,[0;3]);
        job.observe(HoleRequest{center:[1000,0,0],..hole(8)}); // Explicit large jump invalidates old results.
        assert!(job.published_for.is_none());
        let mut first=None;
        job.slice(tick+21,7,&mut |at| { first.get_or_insert(at); Some(false) },&mut ||false);
        assert_eq!(first,Some([992,-8,-8]));
    }
    #[test]
    fn empty_idle_and_impossible_hole_bounds_terminate_without_reads() {
        let mut shared=Shared::default();
        assert_eq!(shared.tick(Requests{epoch:0,block:None,hole:Some(HoleRequest{center:[0,-100,0],..hole(3)})},1,|_|panic!(),|_|panic!(),||false).total(),0);
        assert!(shared.hole.published_for.is_some());
        assert_eq!(shared.tick(Requests{epoch:0,block:None,hole:None},2,|_|panic!(),|_|panic!(),||panic!()).total(),0);
        for center in [[i32::MIN,0,0],[i32::MAX,0,0],[0,i32::MAX,0],[0,0,i32::MIN]] {
            assert!(!HoleRequest{center,..hole(16)}.valid());
        }
        assert_eq!(hole_range(3.9),Some(3)); assert_eq!(hole_range(f64::NAN),None);
    }
}
