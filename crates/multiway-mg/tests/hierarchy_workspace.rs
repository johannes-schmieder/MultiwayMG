//! Numerical and ownership contracts for hierarchy traversal workspaces.

#[allow(dead_code)]
#[path = "../examples/support/issue3_recursive_fixtures.rs"]
mod fixtures;

use multiway_mg::{
    CycleScreenedMapHierarchy, CycleScreenedMapHierarchyWorkspace, DensePseudoinverse,
    FactorAggregation, MultiwayError, Preconditioner, SymmetricMapPreconditioner, ThreeWayProblem,
};

// Test-only transcription of the allocating recurrence at b4f41fd. Keeping an
// independent reference avoids a tautological comparison of two public entry
// points that now intentionally share the same production implementation.
struct AllocatingReference {
    problems: Vec<ThreeWayProblem>,
    maps: Vec<FactorAggregation>,
    smoothers: Vec<SymmetricMapPreconditioner>,
    terminal: DensePseudoinverse,
}

impl AllocatingReference {
    fn new(problem: ThreeWayProblem, maps: Vec<FactorAggregation>) -> Result<Self, MultiwayError> {
        let mut problems = vec![problem];
        for map in &maps {
            problems.push(map.coarsen(problems.last().unwrap())?);
        }
        let smoothers = problems[..maps.len()]
            .iter()
            .cloned()
            .map(SymmetricMapPreconditioner::new)
            .collect();
        let terminal = DensePseudoinverse::from_problem(problems.last().unwrap(), 1.0e-12)?;
        Ok(Self { problems, maps, smoothers, terminal })
    }

    fn apply_level(&self, level: usize, rhs: &[f64]) -> Result<Vec<f64>, MultiwayError> {
        let problem = &self.problems[level];
        if level == self.maps.len() {
            let mut solution = vec![0.0; problem.dimension()];
            self.terminal.solve_into(rhs, &mut solution)?;
            problem.components().project_structural_range(&mut solution)?;
            return Ok(solution);
        }
        let mut compatible_rhs = rhs.to_vec();
        problem.components().project_structural_range(&mut compatible_rhs)?;
        let mut solution = vec![0.0; problem.dimension()];
        self.smoothers[level].apply(&compatible_rhs, &mut solution)?;
        let residual = problem.residual(&compatible_rhs, &solution)?;
        let coarse_problem = &self.problems[level + 1];
        let mut coarse_rhs = vec![0.0; coarse_problem.dimension()];
        self.maps[level].restrict(&residual, &mut coarse_rhs)?;
        coarse_problem.components().project_structural_range(&mut coarse_rhs)?;
        let coarse_solution = self.apply_level(level + 1, &coarse_rhs)?;
        let mut prolonged = vec![0.0; problem.dimension()];
        self.maps[level].prolong(&coarse_solution, &mut prolonged)?;
        add_assign(&mut solution, &prolonged);
        let post_residual = problem.residual(&compatible_rhs, &solution)?;
        let mut post = vec![0.0; problem.dimension()];
        self.smoothers[level].apply(&post_residual, &mut post)?;
        add_assign(&mut solution, &post);
        problem.components().project_structural_range(&mut solution)?;
        Ok(solution)
    }
}

fn add_assign(destination: &mut [f64], source: &[f64]) {
    for (left, &right) in destination.iter_mut().zip(source) {
        *left += right;
    }
}

fn rhs(dimension: usize, scale: f64) -> Vec<f64> {
    (0..dimension)
        .map(|i| scale * ((i as f64 + 0.25) * 0.73).sin())
        .collect()
}

fn assert_bits_equal(actual: &[f64], expected: &[f64]) {
    assert_eq!(actual.len(), expected.len());
    for (index, (&actual, &expected)) in actual.iter().zip(expected).enumerate() {
        assert_eq!(actual.to_bits(), expected.to_bits(), "coefficient {index}");
    }
}

#[test]
fn matches_prechange_recurrence_on_all_revealed_recursive_fixtures() -> Result<(), fixtures::DynError> {
    let fixtures = fixtures::recursive_holdout_fixtures()?;
    assert_eq!(fixtures.len(), 8);
    for fixture in fixtures {
        let reference = AllocatingReference::new(fixture.problem.clone(), fixture.oracle_maps.clone())?;
        let hierarchy = CycleScreenedMapHierarchy::from_maps(
            fixture.problem.clone(), fixture.oracle_maps.clone(), 1.0e-12,
        )?;
        let independent = CycleScreenedMapHierarchy::from_maps(
            fixture.problem, fixture.oracle_maps, 1.0e-12,
        )?;
        let mut workspace = hierarchy.application_workspace()?;
        let bytes = workspace.retained_bytes()?;
        let buffers = workspace.retained_buffer_count();
        for scale in [1.0, -2.0, 0.0, 0.5] {
            let rhs = rhs(hierarchy.dimension(), scale);
            let expected = reference.apply_level(0, &rhs)?;
            let mut ordinary = vec![f64::NAN; rhs.len()];
            hierarchy.apply(&rhs, &mut ordinary)?;
            assert_bits_equal(&ordinary, &expected);
            let mut reused = vec![f64::NAN; rhs.len()];
            hierarchy.apply_with_workspace(&rhs, &mut reused, &mut workspace)?;
            assert_bits_equal(&reused, &expected);
            independent.apply_with_workspace(&rhs, &mut reused, &mut workspace)?;
            assert_bits_equal(&reused, &expected);
            assert_eq!(workspace.retained_bytes()?, bytes);
            assert_eq!(workspace.retained_buffer_count(), buffers);
        }
    }
    Ok(())
}

fn tensor_problem(levels: usize, weight_scale: f64) -> ThreeWayProblem {
    let mut tuples = Vec::new();
    let mut weights = Vec::new();
    for i in 0..levels {
        for j in 0..levels {
            for k in 0..levels {
                tuples.push([i as u32, j as u32, k as u32]);
                weights.push(weight_scale * (1 + (i + 3 * j + 5 * k) % 9) as f64);
            }
        }
    }
    ThreeWayProblem::from_observations([levels; 3], &tuples, &weights).unwrap()
}

fn maps_for(problem: &ThreeWayProblem) -> Vec<FactorAggregation> {
    let mut counts = problem.topology().level_counts();
    let mut maps = Vec::new();
    while counts[0] > 1 {
        let map = FactorAggregation::consecutive_halving(counts).unwrap();
        counts = map.coarse_counts();
        maps.push(map);
    }
    maps
}

#[test]
fn anonymous_workspace_reuses_across_sizes_depths_weights_and_terminal_only() {
    let mut workspace = CycleScreenedMapHierarchyWorkspace::new();
    assert_eq!(workspace.retained_bytes().unwrap(), 0);
    assert_eq!(workspace.retained_buffer_count(), 0);
    let mut portfolio_bytes = 0;
    let mut portfolio_buffers = 0;
    for pass in 0..2 {
        for levels in [1, 2, 4, 3, 6, 2] {
            let problem = tensor_problem(levels, 1.0 + pass as f64 * 0.5);
            let maps = maps_for(&problem);
            let reference = AllocatingReference::new(problem.clone(), maps.clone()).unwrap();
            let hierarchy = CycleScreenedMapHierarchy::from_maps(problem, maps, 1.0e-12).unwrap();
            let rhs = rhs(hierarchy.dimension(), -1.25);
            let expected = reference.apply_level(0, &rhs).unwrap();
            let mut out = vec![f64::NAN; rhs.len()];
            hierarchy.apply_with_workspace(&rhs, &mut out, &mut workspace).unwrap();
            assert_bits_equal(&out, &expected);
            let bytes = workspace.retained_bytes().unwrap();
            let buffers = workspace.retained_buffer_count();
            hierarchy.apply_with_workspace(&rhs, &mut out, &mut workspace).unwrap();
            assert_bits_equal(&out, &expected);
            assert_eq!(workspace.retained_bytes().unwrap(), bytes);
            assert_eq!(workspace.retained_buffer_count(), buffers);
        }
        if pass == 0 {
            portfolio_bytes = workspace.retained_bytes().unwrap();
            portfolio_buffers = workspace.retained_buffer_count();
        } else {
            assert_eq!(workspace.retained_bytes().unwrap(), portfolio_bytes);
            assert_eq!(workspace.retained_buffer_count(), portfolio_buffers);
        }
    }
}

#[test]
fn dimension_errors_preserve_output_and_workspace_then_allow_reuse() {
    let problem = tensor_problem(4, 1.0);
    let maps = maps_for(&problem);
    let hierarchy = CycleScreenedMapHierarchy::from_maps(problem, maps, 1.0e-12).unwrap();
    let rhs = rhs(hierarchy.dimension(), 1.0);
    let mut workspace = hierarchy.application_workspace().unwrap();
    let bytes = workspace.retained_bytes().unwrap();
    let buffers = workspace.retained_buffer_count();
    for length in [rhs.len() - 1, rhs.len() + 1] {
        let mut out = vec![23.0; length];
        let sentinel = out.clone();
        assert!(matches!(
            hierarchy.apply_with_workspace(&rhs, &mut out, &mut workspace),
            Err(MultiwayError::DimensionMismatch { .. })
        ));
        assert_bits_equal(&out, &sentinel);
        let bad_rhs = vec![1.0; length];
        let mut out = vec![23.0; rhs.len()];
        let sentinel = out.clone();
        assert!(matches!(
            hierarchy.apply_with_workspace(&bad_rhs, &mut out, &mut workspace),
            Err(MultiwayError::DimensionMismatch { .. })
        ));
        assert_bits_equal(&out, &sentinel);
        assert_eq!(workspace.retained_bytes().unwrap(), bytes);
        assert_eq!(workspace.retained_buffer_count(), buffers);
    }
    let mut expected = vec![0.0; rhs.len()];
    hierarchy.apply(&rhs, &mut expected).unwrap();
    let mut actual = vec![f64::NAN; rhs.len()];
    hierarchy.apply_with_workspace(&rhs, &mut actual, &mut workspace).unwrap();
    assert_bits_equal(&actual, &expected);
}

#[test]
fn independent_workspaces_can_use_one_immutable_hierarchy_concurrently() {
    fn assert_send_sync<T: Send + Sync>() {}
    assert_send_sync::<CycleScreenedMapHierarchyWorkspace>();
    assert_send_sync::<CycleScreenedMapHierarchy>();
    let problem = tensor_problem(4, 1.0);
    let maps = maps_for(&problem);
    let hierarchy = CycleScreenedMapHierarchy::from_maps(problem, maps, 1.0e-12).unwrap();
    std::thread::scope(|scope| {
        for scale in [1.0, -2.0, 0.5] {
            let hierarchy = &hierarchy;
            scope.spawn(move || {
                let rhs = rhs(hierarchy.dimension(), scale);
                let mut expected = vec![0.0; rhs.len()];
                hierarchy.apply(&rhs, &mut expected).unwrap();
                let mut workspace = hierarchy.application_workspace().unwrap();
                let mut actual = vec![0.0; rhs.len()];
                hierarchy.apply_with_workspace(&rhs, &mut actual, &mut workspace).unwrap();
                assert_bits_equal(&actual, &expected);
            });
        }
    });
}
