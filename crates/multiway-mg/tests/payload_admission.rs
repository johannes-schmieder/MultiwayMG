//! Prepared capacity and owner rejection, with no change to numerical policy.

use multiway_mg::{
    CycleScreenedMapHierarchy, CycleScreenedMapHierarchyWorkspace, FactorAggregation,
    MultiwayError, PcgPayloadBudget, PcgTraceOptions, PcgTraceWorkspace, ThreeWayProblem,
    prepared_map_pcg_payload_report, solve_projected_pcg_traced_with_payload_budget,
};

fn hierarchy(levels: usize) -> CycleScreenedMapHierarchy {
    let tuples: Vec<_> = (0..levels)
        .flat_map(|i| {
            (0..levels).flat_map(move |j| (0..levels).map(move |k| [i as u32, j as u32, k as u32]))
        })
        .collect();
    let problem =
        ThreeWayProblem::from_observations([levels; 3], &tuples, &vec![1.0; tuples.len()]).unwrap();
    let mut maps = Vec::new();
    let mut counts = [levels; 3];
    while counts[0] > 1 {
        let map = FactorAggregation::consecutive_halving(counts).unwrap();
        counts = map.coarse_counts();
        maps.push(map);
    }
    CycleScreenedMapHierarchy::from_maps(problem, maps, 1.0e-12).unwrap()
}

#[test]
fn strict_zero_rhs_rejects_empty_and_foreign_nested_storage_without_mutation() {
    let hierarchy = hierarchy(4);
    let other = self::hierarchy(4);
    let options = PcgTraceOptions::default();
    let rhs = vec![0.0; hierarchy.finest_problem().dimension()];
    let mut outer = PcgTraceWorkspace::try_new(hierarchy.finest_problem(), options).unwrap();
    let mut empty = CycleScreenedMapHierarchyWorkspace::new();
    let before = format!("{outer:?}");
    let budget = PcgPayloadBudget {
        maximum_bytes: usize::MAX,
        additional_live_bytes: 0,
    };
    assert!(matches!(
        solve_projected_pcg_traced_with_payload_budget(
            &hierarchy, &rhs, options, &mut outer, &mut empty, budget
        ),
        Err(MultiwayError::WorkspaceNotPrepared { .. })
    ));
    assert_eq!(empty.retained_bytes().unwrap(), 0);
    assert_eq!(format!("{outer:?}"), before);
    let mut foreign = other.application_workspace().unwrap();
    assert!(!foreign.is_prepared_for(&hierarchy));
    let foreign_before = format!("{foreign:?}");
    assert!(
        solve_projected_pcg_traced_with_payload_budget(
            &hierarchy,
            &rhs,
            options,
            &mut outer,
            &mut foreign,
            budget
        )
        .is_err()
    );
    assert_eq!(format!("{foreign:?}"), foreign_before);
    assert_eq!(format!("{outer:?}"), before);
    foreign.try_prepare_for(&hierarchy).unwrap();
    assert!(foreign.is_prepared_for(&hierarchy));
    let result = solve_projected_pcg_traced_with_payload_budget(
        &hierarchy,
        &rhs,
        options,
        &mut outer,
        &mut foreign,
        budget,
    )
    .unwrap();
    assert!(result.converged());
    assert_eq!(result.iterations(), 0);
}

#[test]
fn retained_inactive_capacity_not_minimum_sizes_controls_admission() {
    let large = hierarchy(8);
    let small = hierarchy(2);
    let large_options = PcgTraceOptions {
        max_iterations: 256,
        ..PcgTraceOptions::default()
    };
    let options = PcgTraceOptions {
        max_iterations: 8,
        ..large_options
    };
    let mut outer = PcgTraceWorkspace::try_new(large.finest_problem(), large_options).unwrap();
    let mut inner = large.application_workspace().unwrap();
    outer
        .try_prepare_for(small.finest_problem(), options)
        .unwrap();
    inner.try_prepare_for(&small).unwrap();
    assert!(inner.is_prepared_for(&small));
    let rhs = vec![1.0; small.finest_problem().dimension()];
    let report = prepared_map_pcg_payload_report(&small, &rhs, options, &outer, &inner, 0).unwrap();
    assert!(
        report.outer_workspace_bytes
            > PcgTraceWorkspace::required_bytes(small.finest_problem(), options).unwrap()
    );
    assert!(report.hierarchy_workspace_bytes > small.workspace_required_bytes().unwrap());
    let minimum = small
        .retained_payload_report()
        .unwrap()
        .total_bytes()
        .unwrap()
        + PcgTraceWorkspace::required_bytes(small.finest_problem(), options).unwrap()
        + small.workspace_required_bytes().unwrap()
        + core::mem::size_of_val(rhs.as_slice());
    let before = format!("{outer:?}{inner:?}");
    assert!(matches!(
        solve_projected_pcg_traced_with_payload_budget(
            &small,
            &rhs,
            options,
            &mut outer,
            &mut inner,
            PcgPayloadBudget {
                maximum_bytes: minimum,
                additional_live_bytes: 0
            }
        ),
        Err(MultiwayError::PayloadBudgetExceeded { .. })
    ));
    assert_eq!(format!("{outer:?}{inner:?}"), before);
    let result = solve_projected_pcg_traced_with_payload_budget(
        &small,
        &rhs,
        options,
        &mut outer,
        &mut inner,
        PcgPayloadBudget {
            maximum_bytes: report.total_bytes().unwrap(),
            additional_live_bytes: 0,
        },
    )
    .unwrap();
    assert!(result.converged());
}

#[test]
fn shared_identity_is_distinct_from_value_equality_and_owned_copy_payload() {
    let first = hierarchy(4);
    let clone = first.clone();
    let independent = hierarchy(4);
    assert!(
        first
            .finest_problem()
            .shares_storage_with(clone.finest_problem())
    );
    assert!(
        !first
            .finest_problem()
            .shares_storage_with(independent.finest_problem())
    );
    assert_eq!(first.finest_problem(), independent.finest_problem());
    let a = first.retained_payload_report().unwrap();
    let b = clone.retained_payload_report().unwrap();
    assert_eq!(a.shared_problem_bytes, b.shared_problem_bytes);
    assert_eq!(
        a.total_bytes().unwrap(),
        a.shared_problem_bytes + a.exclusive_bytes().unwrap()
    );
    assert!(a.shared_problem_bytes >= first.finest_problem().retained_payload_bytes().unwrap());
    assert!(a.terminal_bytes > 0);
    assert!(a.aggregation_bytes > 0);
}
