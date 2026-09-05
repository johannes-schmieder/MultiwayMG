//! Whole prepared-solve allocation regions; setup, copies and diagnostics are separate.

use super::{GLOBAL, Result, fixtures, no_events};
use multiway_mg::{
    CycleScreenedMapHierarchy, CycleScreenedMapHierarchyWorkspace, FactorAggregation,
    MultiwayError, PcgTraceOptions, PcgTraceResult, PcgTraceResultRef, PcgTraceSample,
    PcgTraceWorkspace, Preconditioner, ThreeWayProblem, solve_projected_pcg_traced,
    solve_projected_pcg_traced_with_workspace, solve_projected_pcg_traced_with_workspaces,
};
use std::hint::black_box;

fn input(problem: &ThreeWayProblem, scale: f64) -> Vec<f64> {
    let beta: Vec<_> = (0..problem.dimension())
        .map(|i| scale * ((i as f64 + 0.5) * 0.71).sin())
        .collect();
    let mut rhs = vec![0.0; beta.len()];
    problem.apply_gramian(&beta, &mut rhs).unwrap();
    rhs
}

fn equal(view: PcgTraceResultRef<'_>, expected: &PcgTraceResult) {
    assert_eq!(view.iterations(), expected.iterations());
    assert_eq!(view.converged(), expected.converged());
    assert_eq!(view.gramian_applications(), expected.gramian_applications());
    assert_eq!(
        view.preconditioner_applications(),
        expected.preconditioner_applications()
    );
    assert_eq!(
        view.rhs_projection_norm().to_bits(),
        expected.rhs_projection_norm().to_bits()
    );
    assert_eq!(
        view.final_relative_residual().to_bits(),
        expected.final_relative_residual().to_bits()
    );
    assert_eq!(view.solution().len(), expected.solution().len());
    for (a, b) in view.solution().iter().zip(expected.solution()) {
        assert_eq!(a.to_bits(), b.to_bits());
    }
    assert_eq!(view.samples().len(), expected.samples().len());
    for (a, b) in view.samples().iter().zip(expected.samples()) {
        assert_eq!(a.iteration(), b.iteration());
        assert_eq!(a.residual_norm().to_bits(), b.residual_norm().to_bits());
        assert_eq!(
            a.relative_residual().to_bits(),
            b.relative_residual().to_bits()
        );
    }
}

fn instance(
    name: &str,
    hierarchy: &CycleScreenedMapHierarchy,
    reuse: &mut PcgTraceWorkspace,
    inner_reuse: &mut CycleScreenedMapHierarchyWorkspace,
) -> Result<()> {
    let problem = hierarchy.finest_problem();
    let options = PcgTraceOptions::default();
    let rhs = input(problem, 1.0);
    let before = GLOBAL.stats();
    let expected = solve_projected_pcg_traced(problem, &rhs, hierarchy, options)?;
    let ordinary = GLOBAL.stats() - before;
    assert!(
        ordinary.allocations > 0,
        "owned convenience solve must allocate"
    );
    let mut inner = hierarchy.application_workspace()?;
    // No numerical solve has used this outer workspace yet.
    let before = GLOBAL.stats();
    let mut outer = PcgTraceWorkspace::try_new(problem, options)?;
    let setup = GLOBAL.stats() - before;
    let retained = outer.retained_bytes()?;
    assert_eq!(setup.reallocations, 0);
    assert_eq!(setup.deallocations, 0);
    assert_eq!(setup.bytes_allocated, retained);
    assert!(retained >= PcgTraceWorkspace::required_bytes(problem, options)?);
    assert!(outer.trace_capacity() >= options.max_iterations + 1);
    assert_eq!(
        outer.trace_retained_bytes()?,
        outer.trace_capacity() * core::mem::size_of::<PcgTraceSample>()
    );
    let before = GLOBAL.stats();
    let result = solve_projected_pcg_traced_with_workspaces(
        problem,
        black_box(&rhs),
        hierarchy,
        options,
        &mut outer,
        &mut inner,
    )?;
    let first = GLOBAL.stats() - before;
    no_events(first);
    equal(result, &expected);
    let solution_pointer = result.solution().as_ptr();
    let trace_pointer = result.samples().as_ptr();
    // A requested owned copy is explicitly measured, never hidden in the solve.
    let before = GLOBAL.stats();
    let owned = result.to_owned();
    black_box(&owned);
    let copy = GLOBAL.stats() - before;
    assert_eq!(copy.allocations, 2);
    assert_eq!(
        copy.bytes_allocated,
        rhs.len() * 8 + owned.samples().len() * core::mem::size_of::<PcgTraceSample>()
    );
    assert_eq!(owned, expected);
    drop(owned);
    let panel: Vec<_> = [1.0, -2.0, 0.0, 0.5]
        .into_iter()
        .map(|s| input(problem, s))
        .collect();
    let expected_panel: Vec<_> = panel
        .iter()
        .map(|right| solve_projected_pcg_traced(problem, right, hierarchy, options))
        .collect::<std::result::Result<_, _>>()?;
    for repeat in 0..8 {
        let index = repeat % panel.len();
        let before = GLOBAL.stats();
        let result = solve_projected_pcg_traced_with_workspaces(
            problem,
            black_box(&panel[index]),
            hierarchy,
            options,
            &mut outer,
            &mut inner,
        )?;
        let events = GLOBAL.stats() - before;
        no_events(events);
        equal(result, &expected_panel[index]);
        assert_eq!(result.solution().as_ptr(), solution_pointer);
        assert_eq!(result.samples().as_ptr(), trace_pointer);
        black_box(result.solution());
        black_box(result.samples());
    }
    assert_eq!(outer.retained_bytes()?, retained);
    let limit = PcgTraceOptions {
        relative_tolerance: 1.0e-14,
        max_iterations: 1,
        ..options
    };
    let limited_expected = solve_projected_pcg_traced(problem, &rhs, hierarchy, limit)?;
    let before = GLOBAL.stats();
    let limited = solve_projected_pcg_traced_with_workspaces(
        problem,
        black_box(&rhs),
        hierarchy,
        limit,
        &mut outer,
        &mut inner,
    )?;
    let events = GLOBAL.stats() - before;
    no_events(events);
    equal(limited, &limited_expected);
    assert_eq!(limited.gramian_applications(), 2 * limited.iterations());
    if !limited.converged() {
        assert_eq!(
            limited.preconditioner_applications(),
            limited.iterations() + 1
        );
    }
    // Zero RHS must not prepare an empty hierarchy workspace.
    let zero = vec![0.0; rhs.len()];
    let mut empty_inner = CycleScreenedMapHierarchyWorkspace::new();
    let before = GLOBAL.stats();
    let result = solve_projected_pcg_traced_with_workspaces(
        problem,
        &zero,
        hierarchy,
        options,
        &mut outer,
        &mut empty_inner,
    )?;
    let events = GLOBAL.stats() - before;
    no_events(events);
    assert!(result.converged());
    assert_eq!(result.preconditioner_applications(), 0);
    assert_eq!(empty_inner.retained_bytes()?, 0);
    // Invalid numerical diagnostics may allocate an error message, not scratch.
    let bad = vec![f64::NAN; rhs.len()];
    let before = GLOBAL.stats();
    let error = solve_projected_pcg_traced_with_workspaces(
        problem,
        &bad,
        hierarchy,
        options,
        &mut outer,
        &mut empty_inner,
    )
    .unwrap_err();
    let diagnostic = GLOBAL.stats() - before;
    assert!(matches!(error, MultiwayError::PcgBreakdown { .. }));
    drop(error);
    assert_eq!(empty_inner.retained_bytes()?, 0);
    assert_eq!(outer.retained_bytes()?, retained);
    let before = GLOBAL.stats();
    let recovered = solve_projected_pcg_traced_with_workspaces(
        problem,
        black_box(&rhs),
        hierarchy,
        options,
        &mut outer,
        &mut inner,
    )?;
    let events = GLOBAL.stats() - before;
    no_events(events);
    equal(recovered, &expected);
    // Dimension and trace-capacity rejection are allocation-free static errors.
    let wrong = vec![1.0; rhs.len() + 1];
    let before = GLOBAL.stats();
    let error = solve_projected_pcg_traced_with_workspaces(
        problem, &wrong, hierarchy, options, &mut outer, &mut inner,
    )
    .unwrap_err();
    let events = GLOBAL.stats() - before;
    no_events(events);
    assert!(matches!(error, MultiwayError::DimensionMismatch { .. }));
    let oversized = PcgTraceOptions {
        max_iterations: outer.trace_capacity(),
        ..options
    };
    let before = GLOBAL.stats();
    let error = solve_projected_pcg_traced_with_workspaces(
        problem, &rhs, hierarchy, oversized, &mut outer, &mut inner,
    )
    .unwrap_err();
    let events = GLOBAL.stats() - before;
    no_events(events);
    assert!(matches!(error, MultiwayError::DimensionMismatch { .. }));
    assert_eq!(outer.retained_bytes()?, retained);
    // Reprepare traffic is charged separately, including obsolete identity release.
    let before = GLOBAL.stats();
    reuse.try_prepare_for(problem, options)?;
    inner_reuse.try_prepare_for(hierarchy)?;
    let reprepare = GLOBAL.stats() - before;
    let before = GLOBAL.stats();
    let result = solve_projected_pcg_traced_with_workspaces(
        problem,
        black_box(&rhs),
        hierarchy,
        options,
        reuse,
        inner_reuse,
    )?;
    let events = GLOBAL.stats() - before;
    no_events(events);
    equal(result, &expected);
    // Independent submitted-operator check, explicitly outside the solve region.
    if result.converged() {
        let residual = problem.residual(&rhs, result.solution())?;
        let r = residual.iter().map(|x| x * x).sum::<f64>().sqrt();
        let b = rhs.iter().map(|x| x * x).sum::<f64>().sqrt();
        assert!(r <= 1.0e-8 * b.max(1.0));
    }
    let before = GLOBAL.stats();
    drop(outer);
    let released = GLOBAL.stats() - before;
    assert_eq!(released.allocations, 0);
    assert_eq!(released.reallocations, 0);
    assert_eq!(released.bytes_deallocated, retained);
    println!(
        "pcg {name}\tdimension={}\tsetup_allocations={}\touter_bytes={}\ttrace_bytes={}\towned_solve_allocations={}\tfirst_allocations=0\trepeat8_allocations=0\tcopy_allocations={}\tdiagnostic_allocations={}\treprepare_allocations={}\treprepare_reallocations={}",
        problem.dimension(),
        setup.allocations,
        retained,
        (options.max_iterations + 1) * core::mem::size_of::<PcgTraceSample>(),
        ordinary.allocations,
        copy.allocations,
        diagnostic.allocations,
        reprepare.allocations,
        reprepare.reallocations
    );
    Ok(())
}

#[derive(Debug)]
struct Identity(usize);
impl Preconditioner for Identity {
    fn dimension(&self) -> usize {
        self.0
    }
    fn apply(&self, rhs: &[f64], out: &mut [f64]) -> std::result::Result<(), MultiwayError> {
        out.copy_from_slice(rhs);
        Ok(())
    }
}

fn generic_and_binding() -> Result<()> {
    let tuples = [[0, 0, 0], [0, 1, 1], [1, 0, 1], [1, 1, 0]];
    let problem = ThreeWayProblem::from_observations([2; 3], &tuples, &[1.0, 2.0, 3.0, 4.0])?;
    let independent = ThreeWayProblem::from_observations([2; 3], &tuples, &[4.0, 3.0, 2.0, 1.0])?;
    let identity = Identity(problem.dimension());
    let options = PcgTraceOptions::default();
    let mut outer = PcgTraceWorkspace::try_new(&problem, options)?;
    let rhs = input(&problem, 1.0);
    let expected = solve_projected_pcg_traced(&problem, &rhs, &identity, options)?;
    for _ in 0..8 {
        let before = GLOBAL.stats();
        let result = solve_projected_pcg_traced_with_workspace(
            &problem,
            black_box(&rhs),
            &identity,
            options,
            &mut outer,
        )?;
        let events = GLOBAL.stats() - before;
        no_events(events);
        equal(result, &expected);
    }
    let before = GLOBAL.stats();
    let error = solve_projected_pcg_traced_with_workspace(
        &independent,
        &rhs,
        &identity,
        options,
        &mut outer,
    )
    .unwrap_err();
    let events = GLOBAL.stats() - before;
    no_events(events);
    assert!(matches!(error, MultiwayError::Incidence(_)));
    let before = GLOBAL.stats();
    let error = solve_projected_pcg_traced_with_workspace(
        &problem,
        &rhs,
        &Identity(rhs.len() + 1),
        options,
        &mut outer,
    )
    .unwrap_err();
    let events = GLOBAL.stats() - before;
    no_events(events);
    assert!(matches!(error, MultiwayError::DimensionMismatch { .. }));
    outer.try_prepare_for(&independent, options)?;
    let rhs = input(&independent, -2.0);
    let expected = solve_projected_pcg_traced(&independent, &rhs, &identity, options)?;
    let before = GLOBAL.stats();
    let result = solve_projected_pcg_traced_with_workspace(
        &independent,
        black_box(&rhs),
        &identity,
        options,
        &mut outer,
    )?;
    let events = GLOBAL.stats() - before;
    no_events(events);
    equal(result, &expected);
    println!(
        "pcg generic-first/repeated/rebound and static binding/dimension rejection allocations=0"
    );
    Ok(())
}

pub(super) fn run() -> Result<()> {
    generic_and_binding()?;
    let mut reuse = PcgTraceWorkspace::new();
    let mut inner_reuse = CycleScreenedMapHierarchyWorkspace::new();
    let fixtures = fixtures::recursive_holdout_fixtures()?;
    assert_eq!(fixtures.len(), 8);
    for fixture in fixtures {
        let hierarchy = CycleScreenedMapHierarchy::from_maps(
            fixture.problem.clone(),
            fixture.oracle_maps.clone(),
            1.0e-12,
        )?;
        instance(&fixture.name, &hierarchy, &mut reuse, &mut inner_reuse)?;
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
        instance(
            &format!("{}-fresh-weights", fixture.name),
            &changed,
            &mut reuse,
            &mut inner_reuse,
        )?;
    }
    let problem = ThreeWayProblem::from_observations([2; 3], &[[0, 0, 0], [1, 1, 1]], &[1.0, 2.0])?;
    let terminal =
        CycleScreenedMapHierarchy::from_maps(problem, Vec::<FactorAggregation>::new(), 1.0e-12)?;
    instance(
        "disconnected-terminal-only",
        &terminal,
        &mut reuse,
        &mut inner_reuse,
    )?;
    println!(
        "PASS prepared-pcg cases=17 plus generic control; first/repeated/zero/limit/reprepared/recovered solves allocate nothing; trace/solution storage charged"
    );
    Ok(())
}
