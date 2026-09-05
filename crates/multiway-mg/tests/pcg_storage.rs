//! Independent pre-storage PCG reference and caller-owned result contracts.

#[allow(dead_code)]
#[path = "../examples/support/issue3_recursive_fixtures.rs"]
mod fixtures;
#[allow(dead_code)]
#[path = "support/pre_workspace_pcg.rs"]
mod reference;

use multiway_mg::{
    CycleScreenedMapHierarchy, CycleScreenedMapHierarchyWorkspace, FactorAggregation,
    MultiwayError, PcgTraceOptions, PcgTraceResult, PcgTraceWorkspace, Preconditioner,
    ThreeWayProblem, solve_projected_pcg_traced,
    solve_projected_pcg_traced_with_hierarchy_workspace, solve_projected_pcg_traced_with_workspace,
    solve_projected_pcg_traced_with_workspaces,
};

fn old_options(options: PcgTraceOptions) -> reference::PcgTraceOptions {
    reference::PcgTraceOptions {
        relative_tolerance: options.relative_tolerance,
        absolute_tolerance: options.absolute_tolerance,
        max_iterations: options.max_iterations,
    }
}

fn assert_reference(actual: &PcgTraceResult, expected: &reference::PcgTraceResult) {
    assert_eq!(actual.iterations(), expected.iterations());
    assert_eq!(actual.converged(), expected.converged());
    assert_eq!(
        actual.gramian_applications(),
        expected.gramian_applications()
    );
    assert_eq!(
        actual.preconditioner_applications(),
        expected.preconditioner_applications()
    );
    assert_eq!(
        actual.rhs_projection_norm().to_bits(),
        expected.rhs_projection_norm().to_bits()
    );
    assert_eq!(
        actual.final_relative_residual().to_bits(),
        expected.final_relative_residual().to_bits()
    );
    assert_eq!(actual.solution().len(), expected.solution().len());
    for (a, b) in actual.solution().iter().zip(expected.solution()) {
        assert_eq!(a.to_bits(), b.to_bits());
    }
    assert_eq!(actual.samples().len(), expected.samples().len());
    for (a, b) in actual.samples().iter().zip(expected.samples()) {
        assert_eq!(a.iteration(), b.iteration());
        assert_eq!(a.residual_norm().to_bits(), b.residual_norm().to_bits());
        assert_eq!(
            a.relative_residual().to_bits(),
            b.relative_residual().to_bits()
        );
    }
}

fn rhs(problem: &ThreeWayProblem, scale: f64) -> Vec<f64> {
    let beta: Vec<_> = (0..problem.dimension())
        .map(|i| scale * ((i as f64 + 0.5) * 0.71).sin())
        .collect();
    let mut rhs = vec![0.0; beta.len()];
    problem.apply_gramian(&beta, &mut rhs).unwrap();
    rhs
}

fn problem(levels: usize, scale: f64) -> ThreeWayProblem {
    let mut tuples = Vec::new();
    let mut weights = Vec::new();
    for i in 0..levels {
        for j in 0..levels {
            for k in 0..levels {
                tuples.push([i as u32, j as u32, k as u32]);
                weights.push(scale * (1 + (7 * i + 11 * j + 13 * k + i * j) % 17) as f64);
            }
        }
    }
    ThreeWayProblem::from_observations([levels; 3], &tuples, &weights).unwrap()
}

fn hierarchy(problem: &ThreeWayProblem) -> CycleScreenedMapHierarchy {
    let mut counts = problem.topology().level_counts();
    let mut maps = Vec::new();
    while counts[0] > 1 {
        let map = FactorAggregation::consecutive_halving(counts).unwrap();
        counts = map.coarse_counts();
        maps.push(map);
    }
    CycleScreenedMapHierarchy::from_maps(problem.clone(), maps, 1.0e-12).unwrap()
}

fn certify(problem: &ThreeWayProblem, rhs: &[f64], actual: &PcgTraceResult) {
    if actual.converged() {
        let residual = problem.residual(rhs, actual.solution()).unwrap();
        let r = residual.iter().map(|x| x * x).sum::<f64>().sqrt();
        let b = rhs.iter().map(|x| x * x).sum::<f64>().sqrt();
        assert!(
            r <= 1.0e-8 * b.max(1.0),
            "independent original-operator residual {r} / {b}"
        );
    }
}

#[test]
fn every_public_path_matches_independent_prechange_pcg_on_revealed_fixtures()
-> Result<(), fixtures::DynError> {
    let fixtures = fixtures::recursive_holdout_fixtures()?;
    assert_eq!(fixtures.len(), 8);
    let options = PcgTraceOptions::default();
    let mut storage = PcgTraceWorkspace::new();
    let mut hierarchy_storage = CycleScreenedMapHierarchyWorkspace::new();
    for fixture in fixtures {
        let problem = fixture.problem;
        let hierarchy =
            CycleScreenedMapHierarchy::from_maps(problem.clone(), fixture.oracle_maps, 1.0e-12)?;
        storage.try_prepare_for(&problem, options)?;
        hierarchy_storage.try_prepare_for(&hierarchy)?;
        let bytes = storage.retained_bytes()?;
        let trace_capacity = storage.trace_capacity();
        for scale in [1.0, -2.0, 0.0, 0.5] {
            let rhs = rhs(&problem, scale);
            let expected = reference::solve_projected_pcg_traced(
                &problem,
                &rhs,
                &hierarchy,
                old_options(options),
            )?;
            let owned = solve_projected_pcg_traced(&problem, &rhs, &hierarchy, options)?;
            assert_reference(&owned, &expected);
            let owned_hierarchy = solve_projected_pcg_traced_with_hierarchy_workspace(
                &problem,
                &rhs,
                &hierarchy,
                options,
                &mut hierarchy_storage,
            )?;
            assert_reference(&owned_hierarchy, &expected);
            let generic = solve_projected_pcg_traced_with_workspace(
                &problem,
                &rhs,
                &hierarchy,
                options,
                &mut storage,
            )?
            .to_owned();
            assert_reference(&generic, &expected);
            let prepared = solve_projected_pcg_traced_with_workspaces(
                &problem,
                &rhs,
                &hierarchy,
                options,
                &mut storage,
                &mut hierarchy_storage,
            )?
            .to_owned();
            assert_reference(&prepared, &expected);
            certify(&problem, &rhs, &prepared);
            assert_eq!(storage.retained_bytes()?, bytes);
            assert_eq!(storage.trace_capacity(), trace_capacity);
        }
    }
    Ok(())
}

#[test]
fn zero_limit_and_explicit_owned_copy_preserve_result_lifetimes() {
    let problem = problem(5, 1.0);
    let hierarchy = hierarchy(&problem);
    let options = PcgTraceOptions::default();
    let mut storage = PcgTraceWorkspace::try_new(&problem, options).unwrap();
    let mut hierarchy_storage = CycleScreenedMapHierarchyWorkspace::new();
    let zero = vec![0.0; problem.dimension()];
    let first = solve_projected_pcg_traced_with_workspaces(
        &problem,
        &zero,
        &hierarchy,
        options,
        &mut storage,
        &mut hierarchy_storage,
    )
    .unwrap()
    .to_owned();
    assert!(first.converged());
    assert_eq!(first.iterations(), 0);
    assert_eq!(first.gramian_applications(), 0);
    assert_eq!(first.preconditioner_applications(), 0);
    assert_eq!(first.samples().len(), 1);
    assert_eq!(hierarchy_storage.retained_bytes().unwrap(), 0);
    let input = rhs(&problem, 1.0);
    let limit = PcgTraceOptions {
        relative_tolerance: 1.0e-14,
        max_iterations: 1,
        ..options
    };
    let expected =
        reference::solve_projected_pcg_traced(&problem, &input, &hierarchy, old_options(limit))
            .unwrap();
    let actual = solve_projected_pcg_traced_with_workspaces(
        &problem,
        &input,
        &hierarchy,
        limit,
        &mut storage,
        &mut hierarchy_storage,
    )
    .unwrap()
    .to_owned();
    assert_reference(&actual, &expected);
    assert!(!actual.converged());
    assert_eq!(actual.gramian_applications(), 2);
    assert_eq!(actual.preconditioner_applications(), 2);
    assert!(first.solution().iter().all(|&x| x == 0.0));
    assert_eq!(first.samples().len(), 1);
    let converged = solve_projected_pcg_traced_with_workspaces(
        &problem,
        &input,
        &hierarchy,
        options,
        &mut storage,
        &mut hierarchy_storage,
    )
    .unwrap()
    .to_owned();
    assert!(converged.converged());
    certify(&problem, &input, &converged);
}

#[test]
fn validation_is_transactional_and_preparation_is_explicit() {
    let owner = problem(3, 1.0);
    let other = problem(3, 2.0);
    let hierarchy = hierarchy(&owner);
    let options = PcgTraceOptions {
        max_iterations: 1,
        ..PcgTraceOptions::default()
    };
    let mut storage = PcgTraceWorkspace::try_new(&owner, options).unwrap();
    let input = rhs(&owner, 1.0);
    let before = format!("{storage:?}");
    assert!(
        solve_projected_pcg_traced_with_workspace(
            &other,
            &input,
            &hierarchy,
            options,
            &mut storage
        )
        .is_err()
    );
    assert_eq!(format!("{storage:?}"), before);
    let larger_budget = PcgTraceOptions {
        max_iterations: storage.trace_capacity(),
        ..options
    };
    assert!(
        solve_projected_pcg_traced_with_workspace(
            &owner,
            &input,
            &hierarchy,
            larger_budget,
            &mut storage
        )
        .is_err()
    );
    assert_eq!(format!("{storage:?}"), before);
    let impossible = PcgTraceOptions {
        max_iterations: usize::MAX,
        ..options
    };
    assert!(storage.try_prepare_for(&owner, impossible).is_err());
    assert_eq!(format!("{storage:?}"), before);
    for options in [
        PcgTraceOptions {
            relative_tolerance: f64::NAN,
            ..options
        },
        PcgTraceOptions {
            absolute_tolerance: -1.0,
            ..options
        },
        PcgTraceOptions {
            relative_tolerance: 0.0,
            absolute_tolerance: 0.0,
            ..options
        },
        PcgTraceOptions {
            max_iterations: 0,
            ..options
        },
    ] {
        let expected =
            reference::solve_projected_pcg_traced(&owner, &input, &hierarchy, old_options(options))
                .unwrap_err();
        let actual = solve_projected_pcg_traced_with_workspace(
            &owner,
            &input,
            &hierarchy,
            options,
            &mut storage,
        )
        .unwrap_err();
        assert_eq!(actual.to_string(), expected.to_string());
        assert_eq!(format!("{storage:?}"), before);
    }
    for length in [input.len() - 1, input.len() + 1] {
        let wrong = vec![1.0; length];
        let expected =
            reference::solve_projected_pcg_traced(&owner, &wrong, &hierarchy, old_options(options))
                .unwrap_err();
        let actual = solve_projected_pcg_traced_with_workspace(
            &owner,
            &wrong,
            &hierarchy,
            options,
            &mut storage,
        )
        .unwrap_err();
        assert_eq!(actual.to_string(), expected.to_string());
        assert_eq!(format!("{storage:?}"), before);
    }
    let wrong_hierarchy = self::hierarchy(&problem(2, 1.0));
    let expected = reference::solve_projected_pcg_traced(
        &owner,
        &input,
        &wrong_hierarchy,
        old_options(options),
    )
    .unwrap_err();
    let actual = solve_projected_pcg_traced_with_workspace(
        &owner,
        &input,
        &wrong_hierarchy,
        options,
        &mut storage,
    )
    .unwrap_err();
    assert_eq!(actual.to_string(), expected.to_string());
    assert_eq!(format!("{storage:?}"), before);
    let mut hierarchy_storage = CycleScreenedMapHierarchyWorkspace::new();
    for levels in [5, 1, 4, 2, 5] {
        let next = problem(levels, 0.75);
        let next_hierarchy = self::hierarchy(&next);
        let options = PcgTraceOptions::default();
        storage.try_prepare_for(&next, options).unwrap();
        let input = rhs(&next, -1.25);
        let expected = reference::solve_projected_pcg_traced(
            &next,
            &input,
            &next_hierarchy,
            old_options(options),
        )
        .unwrap();
        let actual = solve_projected_pcg_traced_with_workspaces(
            &next.clone(),
            &input,
            &next_hierarchy,
            options,
            &mut storage,
            &mut hierarchy_storage,
        )
        .unwrap()
        .to_owned();
        assert_reference(&actual, &expected);
        assert!(
            storage.retained_bytes().unwrap()
                >= PcgTraceWorkspace::required_bytes(&next, options).unwrap()
        );
    }
}

#[derive(Debug)]
struct Interrupt {
    dimension: usize,
    calls: std::sync::atomic::AtomicUsize,
    at: usize,
    panic: bool,
}
impl Preconditioner for Interrupt {
    fn dimension(&self) -> usize {
        self.dimension
    }
    fn apply(&self, rhs: &[f64], out: &mut [f64]) -> Result<(), MultiwayError> {
        let calls = self
            .calls
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed)
            + 1;
        if calls == self.at {
            assert!(!self.panic, "injected preconditioner unwind");
            return Err(MultiwayError::PcgBreakdown {
                iteration: calls,
                message: "injected preconditioner error".to_owned(),
            });
        }
        out.copy_from_slice(rhs);
        Ok(())
    }
}

#[test]
fn numerical_errors_and_preconditioner_unwind_leave_reusable_scratch() {
    let problem = problem(5, 1.0);
    let hierarchy = hierarchy(&problem);
    let options = PcgTraceOptions::default();
    let mut storage = PcgTraceWorkspace::try_new(&problem, options).unwrap();
    let mut hierarchy_storage = CycleScreenedMapHierarchyWorkspace::new();
    let input = rhs(&problem, 1.0);
    let bytes = storage.retained_bytes().unwrap();
    for bad in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        for all in [false, true] {
            let mut wrong = input.clone();
            if all {
                wrong.fill(bad);
            } else {
                wrong[0] = bad;
            }
            let expected = reference::solve_projected_pcg_traced(
                &problem,
                &wrong,
                &hierarchy,
                old_options(options),
            )
            .unwrap_err();
            let actual = solve_projected_pcg_traced_with_workspaces(
                &problem,
                &wrong,
                &hierarchy,
                options,
                &mut storage,
                &mut hierarchy_storage,
            )
            .unwrap_err();
            assert_eq!(actual.to_string(), expected.to_string());
            assert_eq!(hierarchy_storage.retained_bytes().unwrap(), 0);
            assert_eq!(storage.retained_bytes().unwrap(), bytes);
        }
    }
    let huge: Vec<_> = input.iter().map(|x| x * 1.0e160).collect();
    let expected =
        reference::solve_projected_pcg_traced(&problem, &huge, &hierarchy, old_options(options))
            .unwrap_err();
    let actual = solve_projected_pcg_traced_with_workspaces(
        &problem,
        &huge,
        &hierarchy,
        options,
        &mut storage,
        &mut hierarchy_storage,
    )
    .unwrap_err();
    assert_eq!(actual.to_string(), expected.to_string());
    for panic in [false, true] {
        for at in [1, 2] {
            let mock = Interrupt {
                dimension: problem.dimension(),
                calls: std::sync::atomic::AtomicUsize::new(0),
                at,
                panic,
            };
            let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                assert!(
                    solve_projected_pcg_traced_with_workspace(
                        &problem,
                        &input,
                        &mock,
                        options,
                        &mut storage
                    )
                    .is_err()
                );
            }));
            assert_eq!(outcome.is_err(), panic);
            assert_eq!(mock.calls.load(std::sync::atomic::Ordering::Relaxed), at);
            let expected = reference::solve_projected_pcg_traced(
                &problem,
                &input,
                &hierarchy,
                old_options(options),
            )
            .unwrap();
            let actual = solve_projected_pcg_traced_with_workspaces(
                &problem,
                &input,
                &hierarchy,
                options,
                &mut storage,
                &mut hierarchy_storage,
            )
            .unwrap()
            .to_owned();
            assert_reference(&actual, &expected);
            assert_eq!(storage.retained_bytes().unwrap(), bytes);
        }
    }
}

#[test]
fn independent_workspaces_support_concurrent_borrowed_solves() {
    let problem = problem(4, 1.0);
    let hierarchy = hierarchy(&problem);
    let input = rhs(&problem, 1.0);
    let options = PcgTraceOptions::default();
    let expected = solve_projected_pcg_traced(&problem, &input, &hierarchy, options).unwrap();
    std::thread::scope(|scope| {
        let mut threads = Vec::new();
        for _ in 0..4 {
            threads.push(scope.spawn(|| {
                let mut storage = PcgTraceWorkspace::try_new(&problem, options).unwrap();
                let mut hierarchy_storage = hierarchy.application_workspace().unwrap();
                for _ in 0..4 {
                    let result = solve_projected_pcg_traced_with_workspaces(
                        &problem,
                        &input,
                        &hierarchy,
                        options,
                        &mut storage,
                        &mut hierarchy_storage,
                    )
                    .unwrap();
                    assert_eq!(result.to_owned(), expected);
                }
            }));
        }
        for thread in threads {
            thread.join().unwrap();
        }
    });
}

#[test]
fn failed_trace_reservation_does_not_publish_a_new_problem_binding() {
    let owner = problem(2, 1.0);
    let larger = problem(5, 1.0);
    let hierarchy = hierarchy(&owner);
    let options = PcgTraceOptions::default();
    let input = rhs(&owner, 1.0);
    let mut workspace = PcgTraceWorkspace::try_new(&owner, options).unwrap();
    let expected = solve_projected_pcg_traced_with_workspace(
        &owner,
        &input,
        &hierarchy,
        options,
        &mut workspace,
    )
    .unwrap()
    .to_owned();
    let before = format!("{workspace:?}");
    // The requested trace exceeds isize::MAX bytes, so Vec rejects capacity
    // before asking the allocator for a huge allocation. Earlier small vector
    // reservations may succeed; lengths, values and binding must not publish.
    let impossible = PcgTraceOptions {
        max_iterations: isize::MAX as usize / core::mem::size_of::<multiway_mg::PcgTraceSample>(),
        ..options
    };
    assert!(matches!(
        workspace.try_prepare_for(&larger, impossible),
        Err(MultiwayError::WorkspaceAllocation { .. })
    ));
    assert_eq!(format!("{workspace:?}"), before);
    let actual = solve_projected_pcg_traced_with_workspace(
        &owner,
        &input,
        &hierarchy,
        options,
        &mut workspace,
    )
    .unwrap()
    .to_owned();
    assert_eq!(actual, expected);
}

#[test]
fn same_sized_different_component_partitions_require_explicit_preparation() {
    let owner =
        ThreeWayProblem::from_observations([2; 3], &[[0, 0, 0], [1, 1, 1]], &[1.0, 2.0]).unwrap();
    let other =
        ThreeWayProblem::from_observations([2; 3], &[[0, 1, 0], [1, 0, 1]], &[1.0, 2.0]).unwrap();
    assert_eq!(owner.dimension(), other.dimension());
    assert_eq!(owner.components().count(), other.components().count());
    let hierarchy = CycleScreenedMapHierarchy::from_maps(other.clone(), vec![], 1.0e-12).unwrap();
    let options = PcgTraceOptions::default();
    let input = rhs(&other, 1.0);
    let mut workspace = PcgTraceWorkspace::try_new(&owner, options).unwrap();
    let before = format!("{workspace:?}");
    let error = solve_projected_pcg_traced_with_workspace(
        &other,
        &input,
        &hierarchy,
        options,
        &mut workspace,
    )
    .unwrap_err();
    assert!(matches!(
        error,
        MultiwayError::Incidence(
            multiway_incidence::IncidenceError::WorkspaceBindingMismatch { .. }
        )
    ));
    assert_eq!(format!("{workspace:?}"), before);
    workspace.try_prepare_for(&other, options).unwrap();
    let actual = solve_projected_pcg_traced_with_workspace(
        &other,
        &input,
        &hierarchy,
        options,
        &mut workspace,
    )
    .unwrap()
    .to_owned();
    let expected =
        reference::solve_projected_pcg_traced(&other, &input, &hierarchy, old_options(options))
            .unwrap();
    assert_reference(&actual, &expected);
}
