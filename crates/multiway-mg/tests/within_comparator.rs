//! Algebraic checks for the frozen public `within` comparator wrapper.
#![cfg(feature = "within-comparator")]

use multiway_mg::{
    Preconditioner, ThreeWayProblem, WithinApproxCholOptions, WithinApproxCholPreconditioner,
};
use within::{Effect, PreconditionerConfig, Solver};

fn problem() -> ThreeWayProblem {
    let tuples = vec![
        [0, 0, 0],
        [0, 1, 1],
        [1, 0, 1],
        [1, 1, 0],
        [2, 1, 1],
        [2, 2, 0],
        [3, 2, 1],
        [3, 0, 0],
    ];
    let weights = vec![1.0e-6, 0.03, 1.0, 7.0, 91.0, 2.0e3, 4.0e5, 8.0e7];
    ThreeWayProblem::from_observations([4, 3, 2], &tuples, &weights).unwrap()
}

fn rhs(problem: &ThreeWayProblem, phase: f64) -> Vec<f64> {
    let mut values: Vec<f64> = (0..problem.dimension())
        .map(|index| ((index as f64 + 1.0) * phase).sin())
        .collect();
    problem
        .components()
        .project_structural_range(&mut values)
        .unwrap();
    values
}

fn dot(left: &[f64], right: &[f64]) -> f64 {
    left.iter().zip(right).map(|(&x, &y)| x * y).sum()
}

fn max_abs(values: &[f64]) -> f64 {
    values.iter().copied().map(f64::abs).fold(0.0, f64::max)
}

#[test]
fn wrapper_matches_the_public_within_preconditioner() {
    let problem = problem();
    let options = WithinApproxCholOptions::default();
    let wrapped = WithinApproxCholPreconditioner::build(problem.clone(), options.clone()).unwrap();

    let mut levels: [Vec<u32>; 3] = std::array::from_fn(|_| Vec::new());
    for tuple in problem.topology().tuples() {
        for factor in 0..3 {
            levels[factor].push(tuple[factor]);
        }
    }
    let effects = levels
        .iter()
        .map(|codes| Effect::new(codes, true, std::iter::empty::<&[f64]>()).unwrap())
        .collect::<Vec<_>>();
    let solver = Solver::new(
        effects,
        Some(problem.weights().to_vec()),
        PreconditionerConfig::Additive {
            local_solver: options.local_solver,
            reduction: options.reduction,
        },
    )
    .unwrap();
    let direct = solver.preconditioner().unwrap();

    let x = rhs(&problem, 0.41);
    let mut expected = vec![0.0; problem.dimension()];
    direct.apply(&x, &mut expected).unwrap();
    problem
        .components()
        .project_structural_range(&mut expected)
        .unwrap();
    let mut actual = vec![0.0; problem.dimension()];
    wrapped.apply(&x, &mut actual).unwrap();

    let difference: Vec<f64> = actual.iter().zip(&expected).map(|(&a, &b)| a - b).collect();
    assert!(max_abs(&difference) < 2.0e-12 * max_abs(&expected).max(1.0));
    assert_eq!(
        wrapped.warnings(),
        solver
            .warnings()
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>()
    );
    assert_eq!(wrapped.projection_fallback_allocations(), 0);
}

#[test]
fn comparator_is_numerically_linear_symmetric_and_positive() {
    let problem = problem();
    let preconditioner =
        WithinApproxCholPreconditioner::build(problem.clone(), WithinApproxCholOptions::default())
            .unwrap();
    let x = rhs(&problem, 0.29);
    let y = rhs(&problem, 0.73);
    let a: f64 = -1.7;
    let b: f64 = 0.45;
    let combination: Vec<f64> = x
        .iter()
        .zip(&y)
        .map(|(&xi, &yi)| a.mul_add(xi, b * yi))
        .collect();
    let mut mx = vec![0.0; problem.dimension()];
    let mut my = vec![0.0; problem.dimension()];
    let mut mcombination = vec![0.0; problem.dimension()];
    preconditioner.apply(&x, &mut mx).unwrap();
    preconditioner.apply(&y, &mut my).unwrap();
    preconditioner
        .apply(&combination, &mut mcombination)
        .unwrap();

    let linearity = mcombination
        .iter()
        .zip(mx.iter().zip(&my))
        .map(|(&actual, (&mxi, &myi))| (actual - a.mul_add(mxi, b * myi)).abs())
        .fold(0.0, f64::max);
    let linearity_scale = max_abs(&mcombination)
        .max(max_abs(&mx))
        .max(max_abs(&my))
        .max(1.0);
    assert!(linearity / linearity_scale < 3.0e-13);

    let symmetry_defect = (dot(&x, &my) - dot(&mx, &y)).abs();
    let symmetry_scale = dot(&x, &my).abs().max(dot(&mx, &y).abs()).max(1.0);
    assert!(symmetry_defect / symmetry_scale < 3.0e-13);
    assert!(dot(&x, &mx) > 0.0);

    let memory = preconditioner.memory_report();
    assert!(memory.problem_state_bytes_estimate() > 0);
    assert!(memory.projection_workspace_bytes() > 0);
    assert_eq!(memory.within_retained_bytes(), None);
}
