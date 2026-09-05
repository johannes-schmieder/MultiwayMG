//! Isolated-process allocation proof with live positive controls.

#[allow(dead_code)]
#[path = "../examples/support/issue3_recursive_fixtures.rs"]
mod fixtures;
#[path = "support/payload_allocations.rs"]
mod payload_allocations;
#[path = "support/pcg_allocations.rs"]
mod pcg_allocations;

use multiway_mg::{
    CycleScreenedMapHierarchy, CycleScreenedMapHierarchyWorkspace, DensePseudoinverse,
    FactorAggregation, MultiwayError, Preconditioner, SymmetricMapPreconditioner, ThreeWayProblem,
};
use stats_alloc::{INSTRUMENTED_SYSTEM, Stats, StatsAlloc};
use std::{alloc::System, hint::black_box};

#[global_allocator]
static GLOBAL: &StatsAlloc<System> = &INSTRUMENTED_SYSTEM;

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

fn measure(mut operation: impl FnMut() -> std::result::Result<(), MultiwayError>) -> Result<Stats> {
    let before = GLOBAL.stats();
    let result = operation();
    let change = GLOBAL.stats() - before;
    result?;
    Ok(change)
}

fn no_events(stats: Stats) {
    assert_eq!(
        stats,
        Stats::default(),
        "prepared operation touched the allocator"
    );
}

fn equal_bits(actual: &[f64], expected: &[f64]) {
    assert_eq!(actual.len(), expected.len());
    for (actual, expected) in actual.iter().zip(expected) {
        assert_eq!(actual.to_bits(), expected.to_bits());
    }
}

fn positive_controls() {
    let before = GLOBAL.stats();
    let vector = black_box(vec![7_u8; black_box(1024)]);
    let allocation = GLOBAL.stats() - before;
    assert!(allocation.allocations > 0 && allocation.bytes_allocated >= 1024);
    black_box(&vector);
    drop(vector);
    let mut vector = Vec::<u8>::with_capacity(1);
    vector.push(1);
    let before = GLOBAL.stats();
    vector.reserve_exact(black_box(4096));
    black_box(&vector);
    let growth = GLOBAL.stats() - before;
    assert!(
        growth.reallocations > 0,
        "reallocation instrument is inactive"
    );
    println!(
        "positive-controls allocations={} reallocations={}",
        allocation.allocations, growth.reallocations
    );
}

fn operator_checks() -> Result<()> {
    let tuples = [[0, 0, 0], [0, 1, 1], [1, 0, 1], [1, 1, 0]];
    let problem = ThreeWayProblem::from_observations([2; 3], &tuples, &[1.0, 2.0, 3.0, 4.0])?;
    let map = SymmetricMapPreconditioner::new(problem.clone());
    let terminal = DensePseudoinverse::from_problem(&problem, 1.0e-12)?;
    let mut map_workspace = map.application_workspace()?;
    let mut terminal_workspace = terminal.application_workspace()?;
    let rhs = [1.0, -2.0, 3.0, -4.0, 5.0, -6.0];
    let mut out = [0.0; 6];
    no_events(measure(|| {
        map.apply_with_workspace(black_box(&rhs), black_box(&mut out), &mut map_workspace)
    })?);
    no_events(measure(|| {
        terminal.solve_into_with_workspace(
            black_box(&rhs),
            black_box(&mut out),
            &mut terminal_workspace,
        )
    })?);
    no_events(measure(|| {
        for _ in 0..64 {
            map.apply_with_workspace(black_box(&rhs), black_box(&mut out), &mut map_workspace)?;
            terminal.solve_into_with_workspace(
                black_box(&rhs),
                black_box(&mut out),
                &mut terminal_workspace,
            )?;
        }
        Ok(())
    })?);
    println!("operators first-and-64-repeated allocations=0 reallocations=0 deallocations=0");
    Ok(())
}

fn check_hierarchy(
    name: &str,
    hierarchy: &CycleScreenedMapHierarchy,
    reuse: &mut CycleScreenedMapHierarchyWorkspace,
) -> Result<()> {
    let rhs: Vec<_> = (0..hierarchy.dimension())
        .map(|i| ((i as f64 + 0.25) * 0.73).sin())
        .collect();
    let mut reference = vec![0.0; rhs.len()];
    let ordinary = measure(|| hierarchy.apply(black_box(&rhs), black_box(&mut reference)))?;
    assert!(
        ordinary.allocations > 0,
        "ordinary allocating control did not allocate"
    );
    // No hierarchy application has used this fresh workspace yet.
    let before = GLOBAL.stats();
    let mut workspace = hierarchy.application_workspace()?;
    let setup = GLOBAL.stats() - before;
    let retained = workspace.retained_bytes()?;
    assert!(retained >= hierarchy.workspace_required_bytes()?);
    assert_eq!(setup.reallocations, 0);
    assert_eq!(setup.deallocations, 0);
    assert_eq!(
        setup.bytes_allocated, retained,
        "fresh exclusive payload accounting disagrees with allocator"
    );
    assert_eq!(
        retained,
        workspace.traversal_retained_bytes()? + workspace.operator_retained_bytes()?
    );
    let mut out = vec![f64::NAN; rhs.len()];
    let first = measure(|| {
        hierarchy.apply_with_workspace(black_box(&rhs), black_box(&mut out), &mut workspace)
    })?;
    no_events(first);
    equal_bits(&out, &reference);
    no_events(measure(|| {
        for _ in 0..64 {
            hierarchy.apply_with_workspace(black_box(&rhs), black_box(&mut out), &mut workspace)?;
            black_box(&out);
        }
        Ok(())
    })?);
    equal_bits(&out, &reference);
    assert_eq!(workspace.retained_bytes()?, retained);
    let mut varied_rhs = rhs.clone();
    for scale in [0.0, -2.0, 0.5] {
        for (value, original) in varied_rhs.iter_mut().zip(&rhs) {
            *value = scale * original;
        }
        hierarchy.apply(&varied_rhs, &mut reference)?;
        no_events(measure(|| {
            hierarchy.apply_with_workspace(
                black_box(&varied_rhs),
                black_box(&mut out),
                &mut workspace,
            )
        })?);
        equal_bits(&out, &reference);
    }
    hierarchy.apply(&rhs, &mut reference)?;
    no_events(measure(|| workspace.try_prepare_for(hierarchy))?);
    // Explicit reprepare for a different hierarchy may grow; charge it outside apply.
    let reprepare = measure(|| reuse.try_prepare_for(hierarchy))?;
    no_events(measure(|| {
        hierarchy.apply_with_workspace(black_box(&rhs), black_box(&mut out), reuse)
    })?);
    equal_bits(&out, &reference);
    let bytes = reuse.retained_bytes()?;
    let mut bad = vec![23.0; rhs.len() + 1];
    let before = GLOBAL.stats();
    assert!(
        hierarchy
            .apply_with_workspace(&rhs, &mut bad, reuse)
            .is_err()
    );
    let invalid = GLOBAL.stats() - before;
    no_events(invalid);
    assert!(bad.iter().all(|&x| x == 23.0));
    assert_eq!(reuse.retained_bytes()?, bytes);
    // Fresh workspace owns no unique identity token; dropping it must release
    // precisely its exclusive payload while the immutable hierarchy lives.
    let before = GLOBAL.stats();
    drop(workspace);
    let released = GLOBAL.stats() - before;
    assert_eq!(released.allocations, 0);
    assert_eq!(released.reallocations, 0);
    assert_eq!(released.bytes_deallocated, retained);
    println!(
        "reprepare {name} allocations={} reallocations={} bytes_allocated={} bytes_deallocated={}",
        reprepare.allocations,
        reprepare.reallocations,
        reprepare.bytes_allocated,
        reprepare.bytes_deallocated
    );
    println!(
        "{name}\tdimension={}\tdepth={}\tsetup_allocations={}\tretained_bytes={}\tordinary_allocations={}\tfirst_allocations=0\trepeat64_allocations=0\treallocations=0\tdeallocations=0",
        hierarchy.dimension(),
        hierarchy.depth(),
        setup.allocations,
        retained,
        ordinary.allocations
    );
    Ok(())
}

fn main() -> Result<()> {
    println!("issue5-complete-map-cycle-allocation-v1");
    positive_controls();
    operator_checks()?;
    pcg_allocations::run()?;
    let fixtures = fixtures::recursive_holdout_fixtures()?;
    assert_eq!(fixtures.len(), 8);
    let mut reuse = CycleScreenedMapHierarchyWorkspace::new();
    for fixture in fixtures {
        let hierarchy = CycleScreenedMapHierarchy::from_maps(
            fixture.problem.clone(),
            fixture.oracle_maps.clone(),
            1.0e-12,
        )?;
        check_hierarchy(&fixture.name, &hierarchy, &mut reuse)?;
        let weights: Vec<_> = fixture
            .problem
            .weights()
            .iter()
            .enumerate()
            .map(|(i, w)| *w * (0.5 + (i % 7) as f64 * 0.5))
            .collect();
        let problem = ThreeWayProblem::from_observations(
            fixture.problem.topology().level_counts(),
            fixture.problem.topology().tuples(),
            &weights,
        )?;
        let changed = CycleScreenedMapHierarchy::from_maps(problem, fixture.oracle_maps, 1.0e-12)?;
        check_hierarchy(
            &format!("{}-fresh-weights", fixture.name),
            &changed,
            &mut reuse,
        )?;
    }
    let problem = ThreeWayProblem::from_observations([2; 3], &[[0, 0, 0], [1, 1, 1]], &[1.0, 2.0])?;
    let terminal_only =
        CycleScreenedMapHierarchy::from_maps(problem, Vec::<FactorAggregation>::new(), 1.0e-12)?;
    check_hierarchy("disconnected-terminal-only", &terminal_only, &mut reuse)?;
    println!(
        "PASS cases=17 first/repeated/reprepared complete applies and exact fresh exclusive payload"
    );
    Ok(())
}
