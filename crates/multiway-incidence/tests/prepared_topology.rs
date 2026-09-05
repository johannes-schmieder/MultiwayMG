//! Independent symbolic references and source/owner invalidation.
use multiway_incidence::{
    IncidenceComponents, IncidenceError, PreparedThreeWayTopology, PreparedTopologySource,
    ThreeWayProblem, ThreeWayTopology,
};

#[test]
fn exact_original_row_groups_and_bit_preserving_scatter() {
    let rows = [
        [1, 0, 1],
        [0, 1, 0],
        [1, 0, 1],
        [0, 0, 0],
        [0, 1, 0],
        [1, 1, 1],
    ];
    let prepared = PreparedThreeWayTopology::try_from_observations([2; 3], &rows).unwrap();
    assert_eq!(
        prepared.topology().tuples(),
        &[[0, 0, 0], [0, 1, 0], [1, 0, 1], [1, 1, 1]]
    );
    let groups = prepared.observation_groups().unwrap();
    assert_eq!(groups.observation_to_tuple(), &[2, 1, 2, 0, 1, 3]);
    assert_eq!(groups.grouped_observations(), &[3, 1, 4, 0, 2, 5]);
    assert_eq!(groups.offsets(), &[0, 1, 3, 5, 6]);
    let values = [3.0, 5.0, f64::from_bits(0x7ff8_0000_0000_0123), -0.0];
    let mut output = [0.0; 6];
    prepared
        .scatter_tuple_values_into(prepared.binding(), &values, &mut output)
        .unwrap();
    for (row, &tuple) in groups.observation_to_tuple().iter().enumerate() {
        assert_eq!(output[row].to_bits(), values[tuple].to_bits());
    }
    for (tuple, range) in groups.offsets().windows(2).enumerate() {
        let expected: Vec<_> = rows
            .iter()
            .enumerate()
            .filter_map(|(row, key)| (*key == prepared.topology().tuples()[tuple]).then_some(row))
            .collect();
        assert_eq!(&groups.grouped_observations()[range[0]..range[1]], expected);
    }
}

#[test]
fn collapsed_layout_is_explicit_and_strict() {
    let rows = [[0, 0, 0], [1, 1, 1]];
    let prepared = PreparedThreeWayTopology::try_from_collapsed([2; 3], &rows).unwrap();
    assert_eq!(prepared.source(), PreparedTopologySource::Collapsed);
    assert_eq!(prepared.input_count(), 2);
    assert!(prepared.observation_groups().is_none());
    assert_eq!(prepared.component_labels(), &[0, 1, 0, 1, 0, 1]);
    assert_eq!(prepared.component_factor_sizes(), &[[1; 3], [1; 3]]);
    let mut output = [0; 2];
    prepared
        .scatter_tuple_values_into(prepared.binding(), &[31, 42], &mut output)
        .unwrap();
    assert_eq!(output, [31, 42]);
    for invalid in [[[1; 3], [0; 3]], [[0; 3], [0; 3]]] {
        assert!(matches!(
            PreparedThreeWayTopology::try_from_collapsed([2; 3], &invalid),
            Err(IncidenceError::NonCanonicalTuples { tuple_index: 1 })
        ));
    }
}

#[test]
fn exact_owner_and_layout_rejection_precedes_output_mutation() {
    let rows = [[0, 0, 0], [1, 1, 1], [0, 1, 1], [1, 0, 0]];
    let first = PreparedThreeWayTopology::try_from_observations([2; 3], &rows).unwrap();
    let second = PreparedThreeWayTopology::try_from_observations([2; 3], &rows).unwrap();
    assert_eq!(first.topology(), second.topology());
    assert_ne!(first.binding(), second.binding());
    let alias = &first;
    assert_eq!(first.binding(), alias.binding());
    let mut out = [23; 4];
    assert!(matches!(
        first.scatter_tuple_values_into(second.binding(), &[1, 2, 3, 4], &mut out),
        Err(IncidenceError::TopologyBindingMismatch)
    ));
    assert!(
        first
            .scatter_tuple_values_into(first.binding(), &[1, 2, 3], &mut out)
            .is_err()
    );
    assert!(
        first
            .scatter_tuple_values_into(first.binding(), &[1, 2, 3, 4], &mut out[..3])
            .is_err()
    );
    assert_eq!(out, [23; 4]);
    first.validate_input_layout([2; 3], &rows).unwrap();
    let mut reordered = rows;
    reordered.swap(0, 1);
    assert!(first.validate_input_layout([2; 3], &reordered).is_err());
    let mut changed = rows;
    changed[0] = [0, 0, 1];
    assert!(first.validate_input_layout([2; 3], &changed).is_err());
    assert!(first.validate_input_layout([2, 2, 3], &rows).is_err());
    assert!(first.validate_input_layout([2; 3], &rows[..3]).is_err());
    first
        .scatter_tuple_values_into(first.binding(), &[1, 2, 3, 4], &mut out)
        .unwrap();
    assert_eq!(out, [1, 4, 2, 3]);
}

fn reference_labels(topology: &ThreeWayTopology) -> Vec<usize> {
    let mut labels = vec![usize::MAX; topology.total_levels()];
    let mut component = 0;
    for start in 0..labels.len() {
        if labels[start] != usize::MAX {
            continue;
        }
        labels[start] = component;
        let mut pending = vec![start];
        while let Some(vertex) = pending.pop() {
            for tuple in topology.tuples() {
                let adjacent = core::array::from_fn::<_, 3, _>(|factor| {
                    topology.global_index(factor, tuple[factor])
                });
                if adjacent.contains(&vertex) {
                    for neighbor in adjacent {
                        if labels[neighbor] == usize::MAX {
                            labels[neighbor] = component;
                            pending.push(neighbor);
                        }
                    }
                }
            }
        }
        component += 1;
    }
    labels
}

#[test]
fn exhaustive_small_supports_match_independent_graph_search() {
    let universe: Vec<_> = (0..2)
        .flat_map(|i| (0..2).flat_map(move |j| (0..2).map(move |k| [i, j, k])))
        .collect();
    for mask in 1..256 {
        let tuples: Vec<_> = universe
            .iter()
            .enumerate()
            .filter_map(|(i, tuple)| ((mask >> i) & 1 == 1).then_some(*tuple))
            .collect();
        let topology = ThreeWayTopology::new([2; 3], tuples.clone()).unwrap();
        let reference = reference_labels(&topology);
        let legacy = IncidenceComponents::from_topology(&topology);
        assert_eq!(legacy.labels(), reference);
        let unused = (0..3).any(|f| (0..2).any(|level| !tuples.iter().any(|t| t[f] == level)));
        let result = PreparedThreeWayTopology::try_from_collapsed([2; 3], &tuples);
        if unused {
            assert!(matches!(result, Err(IncidenceError::UnusedLevel { .. })));
        } else {
            let prepared = result.unwrap();
            assert_eq!(prepared.component_labels(), reference);
            assert_eq!(prepared.component_factor_sizes(), legacy.factor_sizes());
            let problem =
                ThreeWayProblem::from_observations([2; 3], &tuples, &vec![1.0; tuples.len()])
                    .unwrap();
            assert_eq!(prepared.topology(), problem.topology());
            let mut raw = tuples.clone();
            raw.extend(tuples.iter().rev());
            raw.reverse();
            let observed = PreparedThreeWayTopology::try_from_observations([2; 3], &raw).unwrap();
            assert_eq!(observed.topology(), prepared.topology());
            assert_eq!(observed.component_labels(), reference);
            observed.validate_input_layout([2; 3], &raw).unwrap();
        }
    }
    // Existing component-only construction still permits isolated/unused vertices.
    let empty = ThreeWayTopology::new([2; 3], vec![]).unwrap();
    assert_eq!(
        IncidenceComponents::from_topology(&empty).labels(),
        reference_labels(&empty)
    );
}

#[test]
fn invalid_inputs_and_checked_setup_budgets() {
    assert!(matches!(
        PreparedThreeWayTopology::try_from_observations([1; 3], &[]),
        Err(IncidenceError::EmptyProblem)
    ));
    assert!(matches!(
        PreparedThreeWayTopology::try_from_observations([0, 1, 1], &[[0; 3]]),
        Err(IncidenceError::EmptyFactor { factor: 0 })
    ));
    assert!(matches!(
        PreparedThreeWayTopology::try_from_observations([1; 3], &[[1, 0, 0]]),
        Err(IncidenceError::TupleOutOfBounds { .. })
    ));
    assert!(matches!(
        PreparedThreeWayTopology::try_from_observations([2, 1, 1], &[[0; 3]]),
        Err(IncidenceError::UnusedLevel {
            factor: 0,
            level: 1
        })
    ));
    assert!(
        PreparedThreeWayTopology::setup_payload_bound(
            [1; 3],
            usize::MAX,
            PreparedTopologySource::Observations
        )
        .is_err()
    );
    for source in [
        PreparedTopologySource::Observations,
        PreparedTopologySource::Collapsed,
    ] {
        let bound = PreparedThreeWayTopology::setup_payload_bound([1; 3], 1, source).unwrap();
        let build = |budget| match source {
            PreparedTopologySource::Observations => {
                PreparedThreeWayTopology::try_from_observations_with_budget(
                    [1; 3],
                    &[[0; 3]],
                    budget,
                )
            }
            PreparedTopologySource::Collapsed => {
                PreparedThreeWayTopology::try_from_collapsed_with_budget([1; 3], &[[0; 3]], budget)
            }
        };
        assert!(
            matches!(build(bound - 1), Err(IncidenceError::TopologySetupBudgetExceeded { required, budget }) if required == bound && budget == bound-1)
        );
        let prepared = build(bound).unwrap();
        assert!(prepared.retained_payload_bytes().unwrap() <= bound);
    }
}

#[test]
fn shared_borrows_work_across_threads_without_mutable_owner_state() {
    fn send_sync<T: Send + Sync>() {}
    send_sync::<PreparedThreeWayTopology>();
    let prepared =
        PreparedThreeWayTopology::try_from_observations([2; 3], &[[1; 3], [0; 3], [1; 3]]).unwrap();
    std::thread::scope(|scope| {
        let first = scope.spawn(|| {
            let mut out = [0; 3];
            prepared
                .scatter_tuple_values_into(prepared.binding(), &[10, 20], &mut out)
                .unwrap();
            out
        });
        let second = scope.spawn(|| {
            let mut out = [0; 3];
            prepared
                .scatter_tuple_values_into(prepared.binding(), &[30, 40], &mut out)
                .unwrap();
            out
        });
        assert_eq!(first.join().unwrap(), [20, 10, 20]);
        assert_eq!(second.join().unwrap(), [40, 30, 40]);
    });
}
