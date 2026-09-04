#![cfg(all(feature = "cmg", feature = "within-comparator"))]
//! Failure boundaries of the paired public issue-4 adapters.

use multiway_mg::{
    PairCmgSchwarzOptions, PairCmgSchwarzPreconditioner, Preconditioner, ThreeWayProblem,
    WithinApproxCholOptions, WithinApproxCholPreconditioner,
};

fn check(preconditioner: &impl Preconditioner, problem: &ThreeWayProblem) {
    for rhs in [
        vec![f64::NAN; problem.dimension()],
        vec![f64::INFINITY; problem.dimension()],
        vec![1.0; problem.dimension() - 1],
    ] {
        let mut output = vec![7.0; problem.dimension()];
        assert!(preconditioner.apply(&rhs, &mut output).is_err());
        assert!(output.iter().all(|&value| value == 0.0));
    }
    let mut wrong_output = vec![7.0; problem.dimension() - 1];
    assert!(
        preconditioner
            .apply(&vec![1.0; problem.dimension()], &mut wrong_output)
            .is_err()
    );
    assert!(wrong_output.iter().all(|&value| value == 0.0));
    let mut rhs: Vec<_> = (0..problem.dimension())
        .map(|i| (i as f64 * 0.31).sin())
        .collect();
    problem
        .components()
        .project_structural_range(&mut rhs)
        .unwrap();
    let mut output = vec![0.0; problem.dimension()];
    preconditioner.apply(&rhs, &mut output).unwrap();
    assert!(output.iter().all(|value| value.is_finite()));
}

#[test]
fn invalid_calls_leave_zero_output_and_reusable_adapters() {
    let problem = ThreeWayProblem::from_observations(
        [2, 2, 2],
        &[[0, 0, 0], [0, 1, 1], [1, 0, 1], [1, 1, 0]],
        &[1.0; 4],
    )
    .unwrap();
    let cmg =
        PairCmgSchwarzPreconditioner::build_all(problem.clone(), PairCmgSchwarzOptions::default())
            .unwrap();
    let within =
        WithinApproxCholPreconditioner::build(problem.clone(), WithinApproxCholOptions::default())
            .unwrap();
    check(&cmg, &problem);
    check(&within, &problem);
    assert_eq!(cmg.fallback_workspace_allocations(), 0);
    assert_eq!(within.projection_fallback_allocations(), 0);
}
