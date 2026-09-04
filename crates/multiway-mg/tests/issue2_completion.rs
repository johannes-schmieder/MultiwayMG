//! Acceptance tests for the completed issue #2 research instrumentation.

use multiway_mg::{
    DensePseudoinverse, DenseRangeDecomposition, DiagonalPreconditioner, FactorAggregation,
    PcgTraceOptions, SpectralAnalysisOptions, SymmetricTwoGridPreconditioner, ThreeWayProblem,
    analyze_stationary_error, solve_projected_pcg_traced,
};

#[cfg(feature = "cmg")]
use multiway_mg::{
    FactorPair, OracleLevelSmootherSpec, PairCmgOptions, PairSubsetCmgPreconditioner,
    ScheduledOracleHierarchy, ScheduledOracleHierarchyOptions, WeightedSumPreconditioner,
};

#[test]
fn exact_pseudoinverse_has_zero_stationary_error_on_the_numerical_range() {
    let problem = complete_problem(2);
    let exact =
        DensePseudoinverse::from_problem(&problem, 1.0e-12).expect("exact pseudoinverse succeeds");
    let options = SpectralAnalysisOptions::default();
    let range = DenseRangeDecomposition::from_problem(&problem, options)
        .expect("range decomposition succeeds");
    let report = analyze_stationary_error(&range, &exact, 1.0, 3, options)
        .expect("stationary analysis succeeds");

    assert!(report.full_preconditioner_symmetry_defect() < 1.0e-10);
    assert!(report.energy_error_symmetry_defect() < 1.0e-10);
    assert!(report.one_sweep_spectral_radius() < 1.0e-9);
    assert!(report.one_sweep_energy_operator_norm() < 1.0e-9);
    assert!(report.repeated_spectral_radius() < 1.0e-24);
}

#[test]
fn explicit_two_grid_cycle_is_symmetric_positive_and_improves_a_weak_chain() {
    let (problem, aggregation) = refined_weak_chain(6, 2, 0.02);
    let diagonal =
        DiagonalPreconditioner::new(&problem, 0.5).expect("diagonal preconditioner succeeds");
    let two_grid = SymmetricTwoGridPreconditioner::build(
        problem.clone(),
        aggregation,
        DiagonalPreconditioner::new(&problem, 0.5).expect("two-grid smoother succeeds"),
        1,
        1.0,
        1.0e-12,
    )
    .expect("two-grid construction succeeds");
    let options = SpectralAnalysisOptions {
        maximum_dimension: 128,
        ..SpectralAnalysisOptions::default()
    };
    let range = DenseRangeDecomposition::from_problem(&problem, options)
        .expect("range decomposition succeeds");
    let diagonal_report = range
        .analyze(&diagonal, options)
        .expect("diagonal spectral analysis succeeds");
    let two_grid_report = range
        .analyze(&two_grid, options)
        .expect("two-grid spectral analysis succeeds");

    assert!(two_grid_report.numerically_symmetric());
    assert!(two_grid_report.positive_definite_on_range());
    assert_eq!(two_grid_report.negative_preconditioner_directions(), 0);
    assert_eq!(two_grid_report.near_zero_preconditioner_directions(), 0);
    assert!(
        two_grid_report.preconditioned_condition_number()
            < diagonal_report.preconditioned_condition_number()
    );
    assert!(two_grid_report.optimal_energy_spectral_radius() < 0.5);
}

#[test]
fn traced_pcg_records_every_true_residual_and_operator_count() {
    let problem = weighted_latin_square(5);
    let targets: Vec<f64> = problem
        .topology()
        .tuples()
        .iter()
        .enumerate()
        .map(|(index, tuple)| {
            (0.13 * index as f64).sin() + 0.2 * f64::from(tuple[0]) - 0.1 * f64::from(tuple[2])
        })
        .collect();
    let rhs = problem
        .rhs_from_targets(&targets)
        .expect("normal right-hand side succeeds");
    let diagonal =
        DiagonalPreconditioner::new(&problem, 0.5).expect("diagonal preconditioner succeeds");
    let result = solve_projected_pcg_traced(
        &problem,
        &rhs,
        &diagonal,
        PcgTraceOptions {
            relative_tolerance: 1.0e-9,
            max_iterations: 500,
            ..PcgTraceOptions::default()
        },
    )
    .expect("traced PCG succeeds");

    assert!(result.converged());
    assert_eq!(result.samples().len(), result.iterations() + 1);
    assert_eq!(result.samples()[0].iteration(), 0);
    assert_eq!(
        result.samples().last().expect("final sample").iteration(),
        result.iterations()
    );
    assert!(result.final_relative_residual() <= 1.0e-9);
    assert_eq!(result.gramian_applications(), 2 * result.iterations());
    assert_eq!(result.preconditioner_applications(), result.iterations());
}

#[cfg(feature = "cmg")]
#[test]
fn a_selected_pair_plus_diagonal_background_is_positive_on_the_full_range() {
    let problem = dominant_pair_problem(6);
    let selected = PairSubsetCmgPreconditioner::build(
        problem.clone(),
        &[FactorPair::OneTwo],
        PairCmgOptions::default(),
    )
    .expect("selected pair build succeeds");
    let combined = WeightedSumPreconditioner::new(
        DiagonalPreconditioner::new(&problem, 0.5).expect("diagonal succeeds"),
        0.25,
        selected,
        1.0,
    )
    .expect("weighted sum succeeds");
    let options = SpectralAnalysisOptions {
        maximum_dimension: 128,
        ..SpectralAnalysisOptions::default()
    };
    let report = DenseRangeDecomposition::from_problem(&problem, options)
        .expect("range decomposition succeeds")
        .analyze(&combined, options)
        .expect("combined spectral analysis succeeds");

    assert!(report.numerically_symmetric());
    assert!(report.positive_definite_on_range());
    assert_eq!(report.negative_preconditioner_directions(), 0);
    assert_eq!(report.near_zero_preconditioner_directions(), 0);
    assert!(combined.right().memory_report().cmg_preconditioner_bytes() > 0);
    assert!(combined.right().memory_report().pair_workspace_bytes() > 0);
}

#[cfg(feature = "cmg")]
#[test]
fn scheduled_oracle_hierarchy_supports_finest_two_and_all_pair_levels() {
    let (problem, maps) = refined_complete_problem(2, 3);
    let jacobi = build_schedule(
        problem.clone(),
        maps.clone(),
        vec![OracleLevelSmootherSpec::Jacobi { omega: 0.5 }; 3],
    );
    let finest_pair = build_schedule(
        problem.clone(),
        maps.clone(),
        vec![
            OracleLevelSmootherSpec::AllPairsCmg {
                options: PairCmgOptions::default(),
            },
            OracleLevelSmootherSpec::Jacobi { omega: 0.5 },
            OracleLevelSmootherSpec::Jacobi { omega: 0.5 },
        ],
    );
    let first_two_pair = build_schedule(
        problem.clone(),
        maps.clone(),
        vec![
            OracleLevelSmootherSpec::AllPairsCmg {
                options: PairCmgOptions::default(),
            },
            OracleLevelSmootherSpec::AllPairsCmg {
                options: PairCmgOptions::default(),
            },
            OracleLevelSmootherSpec::Jacobi { omega: 0.5 },
        ],
    );
    let all_pair = build_schedule(
        problem.clone(),
        maps,
        vec![
            OracleLevelSmootherSpec::AllPairsCmg {
                options: PairCmgOptions::default(),
            };
            3
        ],
    );
    let options = SpectralAnalysisOptions {
        maximum_dimension: 128,
        ..SpectralAnalysisOptions::default()
    };
    let range = DenseRangeDecomposition::from_problem(&problem, options)
        .expect("range decomposition succeeds");

    for hierarchy in [&jacobi, &finest_pair, &first_two_pair, &all_pair] {
        let report = range
            .analyze(hierarchy, options)
            .expect("scheduled spectral analysis succeeds");
        assert!(report.numerically_symmetric());
        assert!(report.positive_definite_on_range());
        assert_eq!(hierarchy.depth(), 3);
        assert!(hierarchy.tuple_complexity() < 2.0);
        assert!(hierarchy.memory_report().total_retained_bytes_estimate() > 0);
        assert!(
            hierarchy
                .memory_report()
                .maximum_apply_scratch_bytes_estimate()
                > 0
        );
    }
    assert_eq!(jacobi.memory_report().pair_cmg_preconditioner_bytes(), 0);
    assert!(
        finest_pair.memory_report().pair_cmg_preconditioner_bytes()
            < first_two_pair
                .memory_report()
                .pair_cmg_preconditioner_bytes()
    );
    assert!(
        first_two_pair
            .memory_report()
            .pair_cmg_preconditioner_bytes()
            < all_pair.memory_report().pair_cmg_preconditioner_bytes()
    );
}

#[cfg(feature = "cmg")]
fn build_schedule(
    problem: ThreeWayProblem,
    maps: Vec<FactorAggregation>,
    smoothers: Vec<OracleLevelSmootherSpec>,
) -> ScheduledOracleHierarchy {
    ScheduledOracleHierarchy::build(
        problem,
        ScheduledOracleHierarchyOptions {
            aggregations: maps,
            smoothers,
            sweeps: 1,
            terminal_relative_tolerance: 1.0e-12,
        },
    )
    .expect("scheduled oracle hierarchy succeeds")
}

fn complete_problem(levels: u32) -> ThreeWayProblem {
    let mut tuples = Vec::new();
    for first in 0..levels {
        for second in 0..levels {
            for third in 0..levels {
                tuples.push([first, second, third]);
            }
        }
    }
    ThreeWayProblem::from_observations([levels as usize; 3], &tuples, &vec![1.0; tuples.len()])
        .expect("complete problem is valid")
}

fn weighted_latin_square(levels: u32) -> ThreeWayProblem {
    let mut tuples = Vec::new();
    let mut weights = Vec::new();
    for first in 0..levels {
        for second in 0..levels {
            tuples.push([first, second, (first + second) % levels]);
            weights.push(0.75 + f64::from((3 * first + 5 * second) % 11) / 10.0);
        }
    }
    ThreeWayProblem::from_observations([levels as usize; 3], &tuples, &weights)
        .expect("Latin-square problem is valid")
}

fn dominant_pair_problem(levels: u32) -> ThreeWayProblem {
    let mut tuples = Vec::new();
    let mut weights = Vec::new();
    for first in 0..levels {
        for second in 0..levels {
            let third = (first + 2 * second) % levels;
            tuples.push([first, second, third]);
            weights.push(1.0 + f64::from((7 * first + 3 * second) % 13) / 10.0);
            tuples.push([first, second, (third + 1) % levels]);
            weights.push(0.02);
        }
    }
    ThreeWayProblem::from_observations([levels as usize; 3], &tuples, &weights)
        .expect("dominant pair problem is valid")
}

fn refined_weak_chain(
    levels: usize,
    clones: usize,
    bridge_weight: f64,
) -> (ThreeWayProblem, FactorAggregation) {
    let mut coarse_tuples = Vec::new();
    let mut coarse_weights = Vec::new();
    for level in 0..levels {
        coarse_tuples.push([level as u32, level as u32, level as u32]);
        coarse_weights.push(1.0 + (level % 5) as f64 / 10.0);
        if level + 1 < levels {
            coarse_tuples.push([level as u32, (level + 1) as u32, (level + 1) as u32]);
            coarse_weights.push(bridge_weight);
            coarse_tuples.push([(level + 1) as u32, level as u32, (level + 1) as u32]);
            coarse_weights.push(bridge_weight * 1.1);
            coarse_tuples.push([(level + 1) as u32, (level + 1) as u32, level as u32]);
            coarse_weights.push(bridge_weight * 0.9);
        }
    }
    let coarse = ThreeWayProblem::from_observations([levels; 3], &coarse_tuples, &coarse_weights)
        .expect("coarse weak chain is valid");
    refine_once(&coarse, clones, 0)
}

#[cfg(feature = "cmg")]
fn refined_complete_problem(
    base_levels: usize,
    depth: usize,
) -> (ThreeWayProblem, Vec<FactorAggregation>) {
    let mut current = complete_problem(base_levels as u32);
    let mut maps = Vec::with_capacity(depth);
    for refinement in 0..depth {
        let (fine, map) = refine_once(&current, 2, refinement);
        maps.push(map);
        current = fine;
    }
    maps.reverse();
    (current, maps)
}

fn refine_once(
    coarse: &ThreeWayProblem,
    clones: usize,
    refinement: usize,
) -> (ThreeWayProblem, FactorAggregation) {
    let coarse_counts = coarse.topology().level_counts();
    let fine_counts = coarse_counts.map(|count| count * clones);
    let parents = core::array::from_fn(|factor| {
        (0..fine_counts[factor])
            .map(|level| (level / clones) as u32)
            .collect()
    });
    let aggregation =
        FactorAggregation::new(fine_counts, parents).expect("oracle aggregation is valid");
    let children = clones * clones * clones;
    let mut tuples = Vec::with_capacity(coarse.tuple_count() * children);
    let mut weights = Vec::with_capacity(tuples.capacity());
    for (tuple_index, (&tuple, &weight)) in coarse
        .topology()
        .tuples()
        .iter()
        .zip(coarse.weights())
        .enumerate()
    {
        let mut scores = Vec::with_capacity(children);
        let mut score_sum = 0.0;
        for first_child in 0..clones {
            for second_child in 0..clones {
                for third_child in 0..clones {
                    let score = 0.75
                        + ((tuple_index * 31
                            + refinement * 17
                            + first_child * 7
                            + second_child * 5
                            + third_child * 3)
                            % 13) as f64
                            / 10.0;
                    scores.push(score);
                    score_sum += score;
                }
            }
        }
        let mut child_index = 0;
        for first_child in 0..clones {
            for second_child in 0..clones {
                for third_child in 0..clones {
                    tuples.push([
                        (tuple[0] as usize * clones + first_child) as u32,
                        (tuple[1] as usize * clones + second_child) as u32,
                        (tuple[2] as usize * clones + third_child) as u32,
                    ]);
                    weights.push(weight * scores[child_index] / score_sum);
                    child_index += 1;
                }
            }
        }
    }
    let fine = ThreeWayProblem::from_observations(fine_counts, &tuples, &weights)
        .expect("refined problem is valid");
    (fine, aggregation)
}
