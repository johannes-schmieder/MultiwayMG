//! Deterministic reservation-boundary failure injection, not malloc-failure coverage.

use super::*;
use crate::{CycleScreenedMapHierarchy, solve_projected_pcg_traced_with_workspaces};

fn problem(levels: usize) -> ThreeWayProblem {
    let tuples: Vec<_> = (0..levels).map(|i| [i as u32; 3]).collect();
    ThreeWayProblem::from_observations([levels; 3], &tuples, &vec![1.0; levels]).unwrap()
}
fn injected() -> MultiwayError {
    let source = Vec::<u8>::new().try_reserve(usize::MAX).unwrap_err();
    MultiwayError::WorkspaceAllocation {
        context: "injected outer reservation boundary",
        source,
    }
}

#[test]
fn every_outer_growth_boundary_preserves_old_state_on_error_and_unwind() {
    let owner = problem(2);
    let larger = problem(16);
    let hierarchy = CycleScreenedMapHierarchy::from_maps(owner.clone(), vec![], 1.0e-12).unwrap();
    let options = PcgTraceOptions {
        max_iterations: 4,
        ..PcgTraceOptions::default()
    };
    let next_options = PcgTraceOptions {
        max_iterations: 64,
        ..options
    };
    let rhs = vec![1.0; owner.dimension()];
    // Six coefficient growth reservations, one trace reservation, one projection delegation.
    for unwind in [false, true] {
        for fail_at in 0..8 {
            let mut outer = PcgTraceWorkspace::try_new(&owner, options).unwrap();
            let mut inner = hierarchy.application_workspace().unwrap();
            let expected = solve_projected_pcg_traced_with_workspaces(
                &owner, &rhs, &hierarchy, options, &mut outer, &mut inner,
            )
            .unwrap()
            .to_owned();
            let before = format!("{outer:?}");
            let before_bytes = outer.retained_bytes().unwrap();
            let mut reached = 0;
            let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                outer.prepare_with(&larger, next_options, || {
                    let index = reached;
                    reached += 1;
                    if index == fail_at {
                        assert!(!unwind, "injected preparation unwind");
                        return Err(injected());
                    }
                    Ok(())
                })
            }));
            assert_eq!(reached, fail_at + 1);
            if unwind {
                assert!(outcome.is_err());
            } else {
                assert!(matches!(
                    outcome.unwrap(),
                    Err(MultiwayError::WorkspaceAllocation { .. })
                ));
            }
            assert_eq!(format!("{outer:?}"), before);
            assert!(outer.retained_bytes().unwrap() >= before_bytes);
            outer.validate(&owner, options).unwrap();
            assert!(outer.validate(&larger, next_options).is_err());
            let recovered = solve_projected_pcg_traced_with_workspaces(
                &owner, &rhs, &hierarchy, options, &mut outer, &mut inner,
            )
            .unwrap()
            .to_owned();
            assert_eq!(recovered, expected);
            outer.try_prepare_for(&larger, next_options).unwrap();
            outer.validate(&larger, next_options).unwrap();
        }
    }
}

#[test]
fn each_fresh_preparation_failure_can_be_retried() {
    let problem = problem(8);
    let options = PcgTraceOptions::default();
    for fail_at in 0..8 {
        let mut outer = PcgTraceWorkspace::new();
        let mut reached = 0;
        assert!(
            outer
                .prepare_with(&problem, options, || {
                    let index = reached;
                    reached += 1;
                    if index == fail_at {
                        Err(injected())
                    } else {
                        Ok(())
                    }
                })
                .is_err()
        );
        assert_eq!(reached, fail_at + 1);
        assert!(outer.projected_rhs.is_empty());
        assert!(outer.solution.is_empty());
        assert!(outer.residual.is_empty());
        assert!(outer.preconditioned.is_empty());
        assert!(outer.direction.is_empty());
        assert!(outer.applied.is_empty());
        assert!(outer.samples.is_empty());
        assert!(outer.projection.is_none());
        outer.try_prepare_for(&problem, options).unwrap();
        outer.validate(&problem, options).unwrap();
        outer
            .prepare_with(&problem, options, || {
                panic!("prepared storage must not reserve or rebind")
            })
            .unwrap();
    }
}
