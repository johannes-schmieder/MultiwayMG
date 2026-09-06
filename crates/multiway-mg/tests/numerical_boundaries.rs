//! Public regressions for finite-input overflow and solver acceptance boundaries.

use multiway_mg::{
    DensePseudoinverse, DiagonalPreconditioner, MultiwayError, Preconditioner, ThreeWayProblem,
};

fn one_tuple(weight: f64) -> ThreeWayProblem {
    ThreeWayProblem::from_observations([1; 3], &[[0; 3]], &[weight]).unwrap()
}

#[test]
fn jacobi_requires_strictly_positive_representable_corrections() {
    let ordinary = one_tuple(1.0);
    for omega in [0.0, -0.0, -1.0, 2.0 / 3.0, f64::NAN, f64::INFINITY] {
        assert!(matches!(
            DiagonalPreconditioner::new(&ordinary, omega),
            Err(MultiwayError::InvalidOption {
                name: "jacobi_omega",
                ..
            })
        ));
    }
    // The input weight is valid; its damped inverse cannot be represented.
    assert!(matches!(
        DiagonalPreconditioner::new(&one_tuple(f64::from_bits(1)), 0.5),
        Err(MultiwayError::NumericalFailure { .. })
    ));
    assert!(matches!(
        DiagonalPreconditioner::new(&one_tuple(f64::MAX), f64::from_bits(1)),
        Err(MultiwayError::NumericalFailure { .. })
    ));
    for weight in [1.0e-200, 1.0, 1.0e200, f64::MAX] {
        let diagonal = DiagonalPreconditioner::new(&one_tuple(weight), 0.5).unwrap();
        let mut out = [0.0; 3];
        diagonal.apply(&[weight; 3], &mut out).unwrap();
        assert!(out.iter().all(|v| (v - 0.5).abs() < 1.0e-15));
    }
}

#[test]
fn ordinary_problem_rejects_overflowing_degrees_like_prepared_frames() {
    let error =
        ThreeWayProblem::from_observations([1, 1, 2], &[[0, 0, 0], [0, 0, 1]], &[f64::MAX; 2])
            .unwrap_err();
    assert!(matches!(
        error,
        multiway_incidence::IncidenceError::InvalidWeightedDegree { .. }
    ));
}

#[test]
fn terminal_extremes_return_a_valid_factor_or_a_typed_error_without_panicking() {
    for weight in [f64::from_bits(1), f64::MIN_POSITIVE, f64::MAX] {
        match DensePseudoinverse::from_problem(&one_tuple(weight), 1.0e-12) {
            Ok(terminal) => {
                assert_eq!(terminal.rank(), 1);
                assert!(terminal.threshold().is_finite() && terminal.threshold() > 0.0);
                let mut out = [0.0; 3];
                terminal.apply(&[weight; 3], &mut out).unwrap();
                assert!(out.iter().all(|v| v.is_finite()));
            }
            Err(MultiwayError::NumericalFailure { .. }) => {}
            other => panic!("unexpected terminal result: {other:?}"),
        }
    }
    assert!(matches!(
        DensePseudoinverse::from_problem(&one_tuple(1.0), f64::MAX),
        Err(MultiwayError::NumericalFailure { .. })
    ));
}

#[cfg(feature = "lsmr")]
#[test]
fn lsmr_never_publishes_the_reproduced_nan_certificates() {
    use multiway_mg::{LeastSquaresOptions, solve_weighted_least_squares};
    for (weight, target) in [(1.0e155, 1.0e154), (1.0e200, 1.0e110)] {
        let problem = one_tuple(weight);
        let diagonal = DiagonalPreconditioner::new(&problem, 0.5).unwrap();
        let result = solve_weighted_least_squares(
            &problem,
            &[target],
            &diagonal,
            LeastSquaresOptions::default(),
        );
        assert!(
            matches!(result, Err(MultiwayError::NumericalFailure { .. })),
            "{result:?}"
        );
    }
}

#[cfg(feature = "lsmr")]
#[test]
fn lsmr_certifies_ordinary_zero_and_representable_extreme_scales() {
    use multiway_mg::{LeastSquaresOptions, solve_weighted_least_squares};
    for (weight, target) in [(1.0, 0.0), (1.0, 2.0), (1.0e-200, 1.0e110), (1.0e200, 1.0)] {
        let problem = one_tuple(weight);
        let diagonal = DiagonalPreconditioner::new(&problem, 0.5).unwrap();
        let result = solve_weighted_least_squares(
            &problem,
            &[target],
            &diagonal,
            LeastSquaresOptions::default(),
        )
        .unwrap();
        assert!(result.is_certified(1.0e-8), "{result:?}");
        for invalid in [0.0, -1.0, f64::NAN, f64::INFINITY] {
            assert!(!result.is_certified(invalid));
        }
    }
}

#[test]
fn ordinary_pcg_rejects_invalid_norms_and_preserves_the_zero_case() {
    use multiway_mg::{PcgOptions, solve_projected_pcg};
    let p = one_tuple(1.0);
    let diagonal = DiagonalPreconditioner::new(&p, 0.5).unwrap();
    for bad in [f64::NAN, f64::INFINITY, f64::MAX] {
        assert!(matches!(
            solve_projected_pcg(&p, &[bad; 3], &diagonal, PcgOptions::default()),
            Err(MultiwayError::PcgBreakdown { .. })
        ));
    }
    let zero = solve_projected_pcg(&p, &[0.0; 3], &diagonal, PcgOptions::default()).unwrap();
    assert!(zero.converged());
    assert_eq!(zero.relative_residual(), 0.0);
}
