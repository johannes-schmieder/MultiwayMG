//! Numerical edge-case tests for the first MultiwayMG research release.

use multiway_mg::{
    AggregationStrategy, DensePseudoinverse, HierarchyOptions, MultiwayError, ThreeWayHierarchy,
    ThreeWayProblem,
};

#[cfg(feature = "lsmr")]
use multiway_mg::DiagonalPreconditioner;
#[cfg(feature = "cmg")]
use multiway_mg::Preconditioner;
#[cfg(feature = "lsmr")]
use multiway_mg::{LeastSquaresOptions, solve_weighted_least_squares};
#[cfg(feature = "cmg")]
use multiway_mg::{PairCmgOptions, PairCmgPreconditioner};

#[test]
fn dense_terminal_rank_is_invariant_to_global_weight_scale() {
    let tuples = complete_tuples(2, 0);
    let small = ThreeWayProblem::from_observations([2, 2, 2], &tuples, &[1.0e-20; 8])
        .expect("small-scale problem is valid");
    let large = ThreeWayProblem::from_observations([2, 2, 2], &tuples, &[1.0e20; 8])
        .expect("large-scale problem is valid");
    let small_terminal =
        DensePseudoinverse::from_problem(&small, 1.0e-12).expect("small-scale terminal succeeds");
    let large_terminal =
        DensePseudoinverse::from_problem(&large, 1.0e-12).expect("large-scale terminal succeeds");
    assert_eq!(small_terminal.rank(), 4);
    assert_eq!(large_terminal.rank(), 4);
}

#[test]
fn asymmetric_jacobi_sweep_counts_are_rejected() {
    let problem = ThreeWayProblem::from_observations([2, 2, 2], &complete_tuples(2, 0), &[1.0; 8])
        .expect("problem is valid");
    let error = ThreeWayHierarchy::build(
        problem,
        HierarchyOptions {
            pre_sweeps: 1,
            post_sweeps: 2,
            aggregation: AggregationStrategy::Consecutive,
            ..HierarchyOptions::default()
        },
    )
    .expect_err("asymmetric smoothing must be rejected");
    assert!(matches!(
        error,
        MultiwayError::InvalidOption {
            name: "smoothing_sweeps",
            ..
        }
    ));
}

#[test]
fn disconnected_terminal_retains_one_structural_kernel_pair_per_component() {
    let mut tuples = complete_tuples(2, 0);
    tuples.extend(complete_tuples(2, 2));
    let problem = ThreeWayProblem::from_observations([4, 4, 4], &tuples, &[1.0; 16])
        .expect("disconnected problem is valid");
    assert_eq!(problem.components().count(), 2);
    let terminal = DensePseudoinverse::from_problem(&problem, 1.0e-12)
        .expect("disconnected terminal succeeds");
    assert_eq!(terminal.rank(), 8);
}

#[cfg(feature = "cmg")]
#[test]
fn pair_cmg_is_symmetric_on_arbitrary_inputs() {
    let tuples = complete_tuples(3, 0);
    let weights: Vec<f64> = (0..tuples.len())
        .map(|index| 0.75 + (index % 7) as f64 / 5.0)
        .collect();
    let problem =
        ThreeWayProblem::from_observations([3, 3, 3], &tuples, &weights).expect("problem is valid");
    let pair = PairCmgPreconditioner::build(problem.clone(), PairCmgOptions::default())
        .expect("pair CMG construction succeeds");
    let left: Vec<f64> = (0..problem.dimension())
        .map(|index| (0.31 * index as f64).sin() + 0.2)
        .collect();
    let right: Vec<f64> = (0..problem.dimension())
        .map(|index| (0.17 * index as f64).cos() - 0.1)
        .collect();
    let mut applied_left = vec![0.0; problem.dimension()];
    let mut applied_right = vec![0.0; problem.dimension()];
    pair.apply(&left, &mut applied_left)
        .expect("left application succeeds");
    pair.apply(&right, &mut applied_right)
        .expect("right application succeeds");
    let forward = dot(&left, &applied_right);
    let reverse = dot(&applied_left, &right);
    let scale = forward.abs().max(reverse.abs()).max(1.0);
    assert!((forward - reverse).abs() / scale < 1.0e-10);
    assert!(
        problem
            .components()
            .maximum_structural_defect(&applied_left)
            .expect("defect calculation succeeds")
            < 1.0e-10
    );
}

#[cfg(feature = "lsmr")]
#[test]
fn rectangular_lsmr_handles_rank_deficiency_beyond_factor_shifts() {
    let mut tuples = Vec::new();
    for first in 0..4_u32 {
        for second in 0..4_u32 {
            tuples.push([first, second, first]);
        }
    }
    let weights: Vec<f64> = (0..tuples.len())
        .map(|index| 1.0 + (index % 5) as f64 / 4.0)
        .collect();
    let problem = ThreeWayProblem::from_observations([4, 4, 4], &tuples, &weights)
        .expect("nested problem is valid");
    let coefficients: Vec<f64> = (0..problem.dimension())
        .map(|index| (0.23 * index as f64).sin())
        .collect();
    let mut targets = vec![0.0; problem.tuple_count()];
    problem
        .apply_incidence(&coefficients, &mut targets)
        .expect("target construction succeeds");
    let diagonal =
        DiagonalPreconditioner::new(&problem, 0.5).expect("diagonal preconditioner succeeds");
    let result = solve_weighted_least_squares(
        &problem,
        &targets,
        &diagonal,
        LeastSquaresOptions {
            tolerance: 1.0e-9,
            max_iterations: 200,
            local_size: Some(8),
        },
    )
    .expect("rank-deficient modified LSMR succeeds");
    assert!(result.converged());
    assert!(result.certified_normal_equation_residual() < 1.0e-8);
}

fn complete_tuples(levels: u32, offset: u32) -> Vec<[u32; 3]> {
    let mut tuples = Vec::new();
    for first in 0..levels {
        for second in 0..levels {
            for third in 0..levels {
                tuples.push([first + offset, second + offset, third + offset]);
            }
        }
    }
    tuples
}

#[cfg(feature = "cmg")]
fn dot(left: &[f64], right: &[f64]) -> f64 {
    left.iter().zip(right).map(|(a, b)| a * b).sum()
}
