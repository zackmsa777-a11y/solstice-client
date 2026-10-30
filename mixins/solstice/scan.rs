// Bounded, host-free Block ESP scan model. MIT; AI-assisted implementation by Codex.
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
    fn take(&mut self) -> Option<[i32; 3]> {
        let at = self.next?;
        let mut next = at;
        if at[1] < self.hi[1] {
            next[1] += 1;
        } else if at[2] < self.hi[2] {
            next[1] = self.lo[1];
            next[2] += 1;
        } else if at[0] < self.hi[0] {
            next[1] = self.lo[1];
            next[2] = self.lo[2];
            next[0] += 1;
        } else {
            self.next = None;
            return Some(at);
        }
        self.next = Some(next);
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
    pub fn tick(
        &mut self,
        request: Request,
        ticks: u64,
        mut read: impl FnMut([i32; 3]) -> Option<u32>,
        mut expired: impl FnMut() -> bool,
    ) -> usize {
        self.observe(request.clone());
        if self.cycle.is_none() {
            let due = self.published_for.as_ref().is_none_or(|last| {
                distance(last.center, request.center) >= 6
                    || ticks.saturating_sub(self.published_at) >= 100
            });
            if !due {
                return 0;
            }
            self.cycle = Some(Cycle {
                cells: Cells::new(&request),
                request,
                found: Vec::new(),
            });
        }
        let mut used = 0;
        let cycle = self.cycle.as_mut().unwrap();
        // Fixed center until complete: ordinary walking updates latest, never resets progress.
        while used < CELLS_PER_TICK {
            if used % CLOCK_BATCH == 0 && expired() {
                break;
            }
            let Some(at) = cycle.cells.take() else {
                break;
            };
            used += 1;
            if let Some(colour) = read(at) {
                cycle.found.push((at, colour));
            }
            if cycle.found.len() == MAX_RESULTS {
                break;
            }
        }
        if cycle.cells.next.is_none() || cycle.found.len() == MAX_RESULTS {
            let done = self.cycle.take().unwrap();
            self.found = done.found;
            self.published_for = Some(done.request);
            self.published_at = ticks;
        }
        used
    }
}
