//! Quotient-space spectral diagnostic tests.

use multiway_mg::{
    DensePairOptions, DensePairSchwarzPreconditioner, DensePseudoinverse,
    DenseRangeDecomposition, DiagonalPreconditioner, Preconditioner, SpectralAnalysisOptions,
    SymmetricMapPreconditioner, ThreeWayProblem,
};

#[test]
fn exact_pseudoinverse_has_unit_preconditioned_spectrum() {
    let problem = complete_problem(2);
    let inverse = DensePseudoinverse::from_problem(&problem, 1.0e-12)
        .expect("dense pseudoinverse succeeds");
    let options = SpectralAnalysisOptions::default();
    let range = DenseRangeDecomposition::from_problem(&problem, options)
        .expect("range decomposition succeeds");
    let report = range
        .analyze(&inverse, options)
        .expect("spectral analysis succeeds");

    assert_eq!(report.numerical_rank(), 4);
    assert_eq!(report.numerical_nullity(), 2);
    assert!(report.numerically_symmetric());
    assert!(report.preserves_range());
    assert!(report.positive_definite_on_range());
    assert!(report.preconditioner_symmetry_defect() < 1.0e-11);
    assert!(report.range_leakage() < 1.0e-11);
    assert!((report.minimum_preconditioned_eigenvalue() - 1.0).abs() < 1.0e-10);
    assert!((report.maximum_preconditioned_eigenvalue() - 1.0).abs() < 1.0e-10);
    assert!(report.unit_step_energy_spectral_radius() < 1.0e-10);
}

#[test]
fn diagonal_map_and_exact_pair_actions_are_symmetric_positive_on_the_range() {
    let problem = weighted_latin_square(4);
    let options = SpectralAnalysisOptions::default();
    let range = DenseRangeDecomposition::from_problem(&problem, options)
        .expect("range decomposition succeeds");
    let diagonal = DiagonalPreconditioner::new(&problem, 0.5)
        .expect("diagonal preconditioner succeeds");
    let map = SymmetricMapPreconditioner::new(problem.clone());
    let dense_pair = DensePairSchwarzPreconditioner::build(
        problem.clone(),
        DensePairOptions::default(),
    )
    .expect("dense pair preconditioner succeeds");

    for preconditioner in [
        &diagonal as &dyn Preconditioner,
        &map as &dyn Preconditioner,
        &dense_pair as &dyn Preconditioner,
    ] {
        let report = range
            .analyze(preconditioner, options)
            .expect("spectral analysis succeeds");
        assert!(report.numerically_symmetric());
        assert!(report.preserves_range());
        assert!(report.positive_definite_on_range());
        assert_eq!(report.negative_preconditioner_directions(), 0);
        assert_eq!(report.near_zero_preconditioner_directions(), 0);
        assert!(report.minimum_preconditioned_eigenvalue() > 0.0);
        assert!(report.preconditioned_condition_number().is_finite());
    }
}

#[test]
fn range_decomposition_detects_additional_nested_nullity() {
    let mut tuples = Vec::new();
    for first in 0..4_u32 {
        for second in 0..4_u32 {
            tuples.push([first, second, first]);
        }
    }
    let problem = ThreeWayProblem::from_observations([4, 4, 4], &tuples, &[1.0; 16])
        .expect("nested problem is valid");
    let range = DenseRangeDecomposition::from_problem(
        &problem,
        SpectralAnalysisOptions::default(),
    )
    .expect("range decomposition succeeds");
    assert!(range.nullity() > 2);
    assert_eq!(range.rank() + range.nullity(), problem.dimension());
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
    ThreeWayProblem::from_observations(
        [levels as usize; 3],
        &tuples,
        &vec![1.0; tuples.len()],
    )
    .expect("complete problem is valid")
}

fn weighted_latin_square(levels: u32) -> ThreeWayProblem {
    let mut tuples = Vec::new();
    let mut weights = Vec::new();
    for first in 0..levels {
        for second in 0..levels {
            tuples.push([first, second, (first + second) % levels]);
            weights.push(0.75 + ((3 * first + 5 * second) % 11) as f64 / 10.0);
        }
    }
    ThreeWayProblem::from_observations([levels as usize; 3], &tuples, &weights)
        .expect("Latin-square problem is valid")
}
