// MIT; AI-assisted regression tests by Codex. Run with rustc --test tools/block_scan_tests.rs.
#[path = "../mixins/solstice/scan.rs"]
mod scan;
use scan::{CELLS_PER_TICK, CLOCK_BATCH, MAX_RESULTS, Request, Scan, scan_center, scan_range};
use std::collections::HashSet;
fn request(center: [i32; 3], range: i32) -> Request {
    Request {
        dimension: "overworld".into(),
        preset: 0,
        center,
        range,
    }
}
fn complete(scan: &mut Scan, r: Request, start: u64) -> Vec<[i32; 3]> {
    let mut visits = Vec::new();
    // All matches let publication be observed even on an empty old cache.
    for tick in start..start + 1000 {
        let used = scan.tick(
            r.clone(),
            tick,
            |at| {
                visits.push(at);
                Some(42)
            },
            || false,
        );
        assert!(used <= CELLS_PER_TICK);
        if !scan.found.is_empty() {
            return visits;
        }
    }
    panic!("did not finish");
}
#[test]
fn exact_old_enumeration_and_no_duplicates_for_all_ranges_and_y_clamps() {
    for range in [8, 32, 48] {
        for y in [-65, -64, 0, 319, 320, 400] {
            let r = request([-7, y, 11], range);
            let mut scan = Scan::default();
            let mut visits = Vec::new();
            let total_y = ((y + range).min(320) - (y - range).max(-64) + 1).max(0) as usize;
            let total = (2 * range + 1).pow(2) as usize * total_y;
            for tick in 0..total / CELLS_PER_TICK + 1 {
                assert!(
                    scan.tick(
                        r.clone(),
                        tick as u64,
                        |at| {
                            visits.push(at);
                            None
                        },
                        || false
                    ) <= CELLS_PER_TICK
                );
            }
            let mut expected = Vec::new();
            for x in r.center[0] - range..=r.center[0] + range {
                for z in r.center[2] - range..=r.center[2] + range {
                    for yy in (y - range).max(-64)..=(y + range).min(320) {
                        expected.push([x, yy, z]);
                    }
                }
            }
            assert_eq!(visits, expected);
            assert_eq!(visits.len(), visits.iter().collect::<HashSet<_>>().len());
        }
    }
}
#[test]
fn hard_cell_budget_and_batched_deadline() {
    let mut scan = Scan::default();
    let r = request([0, 0, 0], 48);
    assert_eq!(scan.tick(r.clone(), 1, |_| None, || false), CELLS_PER_TICK);
    let mut checks = 0;
    assert_eq!(
        scan.tick(
            r.clone(),
            2,
            |_| None,
            || {
                checks += 1;
                checks > 1
            }
        ),
        CLOCK_BATCH
    );
    assert_eq!(checks, 2);
    assert_eq!(
        scan.tick(r, 3, |_| panic!("expired before first read"), || true),
        0
    );
}
#[test]
fn result_cap_publishes_original_prefix() {
    let mut scan = Scan::default();
    let visited = complete(&mut scan, request([0, 0, 0], 48), 1);
    assert_eq!(visited.len(), MAX_RESULTS);
    assert_eq!(scan.found.iter().map(|f| f.0).collect::<Vec<_>>(), visited);
}
#[test]
fn cancel_config_dimension_and_teleport_clear_partial_and_published_results() {
    for change in 0..4 {
        let mut scan = Scan::default();
        let r = request([0, 0, 0], 8);
        complete(&mut scan, r.clone(), 1);
        // Start refresh, leaving the old publication intact and one new match pending.
        let mut calls = 0;
        scan.tick(
            r.clone(),
            101,
            |_| Some(99),
            || {
                calls += 1;
                calls > 1
            },
        );
        assert!(scan.found.iter().all(|f| f.1 == 42));
        let mut next = r.clone();
        match change {
            0 => next.preset = 1,
            1 => next.dimension = "nether".into(),
            2 => next.center[0] = 16,
            _ => next.range = 12,
        }
        scan.observe(next.clone());
        assert!(scan.found.is_empty());
        let visited = complete(&mut scan, next.clone(), 102);
        assert_eq!(
            visited[0],
            [next.center[0] - next.range, -next.range, -next.range]
        );
        assert!(scan.found.iter().all(|f| f.1 == 42));
        scan.clear();
        assert!(scan.found.is_empty());
        assert!(scan.tick(next, 103, |_| None, || false) > 0);
    }
}
#[test]
fn ordinary_walking_completes_fixed_center_and_queues_latest_pose() {
    let mut scan = Scan::default();
    let mut first = None;
    let mut reads = 0;
    for tick in 0..100 {
        let r = request([tick, 0, 0], 8);
        let mut checks = 0;
        scan.tick(
            r,
            tick as u64,
            |at| {
                first.get_or_insert(at);
                reads += 1;
                (reads == 4913).then_some(7)
            },
            || {
                checks += 1;
                checks > 1
            },
        );
        if !scan.found.is_empty() {
            assert_eq!(reads, 4913);
            assert_eq!(first, Some([-8, -8, -8]));
            let mut next_first = None;
            scan.tick(
                request([tick + 1, 0, 0], 8),
                tick as u64 + 1,
                |at| {
                    next_first.get_or_insert(at);
                    None
                },
                || false,
            );
            assert_eq!(next_first.unwrap()[0], tick + 1 - 8);
            return;
        }
        assert!(scan.found.is_empty(), "partial results leaked");
    }
    panic!("ordinary walking starved the scan");
}
#[test]
fn completed_results_stay_coherent_while_refreshing() {
    let mut scan = Scan::default();
    let r = request([0, 0, 0], 8);
    complete(&mut scan, r.clone(), 1);
    let old = scan.found.clone();
    let mut checks = 0;
    scan.tick(
        r.clone(),
        101,
        |_| Some(99),
        || {
            checks += 1;
            checks > 1
        },
    );
    assert_eq!(scan.found, old);
    scan.tick(r, 102, |_| Some(99), || false);
    assert!(scan.found.iter().all(|f| f.1 == 99));
}
#[test]
fn stable_center_waits_100_ticks_after_publication_or_six_blocks() {
    let mut scan = Scan::default();
    let r = request([0, 0, 0], 8);
    complete(&mut scan, r.clone(), 1);
    assert_eq!(
        scan.tick(r.clone(), 100, |_| panic!("too early"), || false),
        0
    );
    assert_eq!(
        scan.tick(request([5, 0, 0], 8), 100, |_| panic!("too near"), || false),
        0
    );
    assert!(scan.tick(request([6, 0, 0], 8), 100, |_| None, || false) > 0);
}
#[test]
fn finite_and_integer_extremes_are_safe() {
    for v in [
        f64::NAN,
        f64::INFINITY,
        f64::NEG_INFINITY,
        7.9,
        48.1,
        f64::MAX,
    ] {
        assert_eq!(scan_range(v), None);
    }
    assert_eq!(scan_range(8.9), Some(8));
    assert_eq!(scan_range(48.0), Some(48));
    assert_eq!(scan_center([-0.1, 1.9, 0.0]), Some([-1, 1, 0]));
    for v in [
        f64::NAN,
        f64::INFINITY,
        f64::NEG_INFINITY,
        f64::MAX,
        2147483648.0,
    ] {
        assert_eq!(scan_center([v, 0.0, 0.0]), None);
    }
    for center in [
        [i32::MIN, 0, i32::MIN],
        [i32::MAX, 0, i32::MAX],
        [0, i32::MIN, 0],
        [0, i32::MAX, 0],
    ] {
        let mut scan = Scan::default();
        assert!(scan.tick(request(center, 48), 1, |_| None, || false) <= CELLS_PER_TICK);
        scan.observe(request([0, 0, 0], 8)); // i32 distance subtraction cannot overflow.
    }
}
