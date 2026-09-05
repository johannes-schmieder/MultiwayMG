//! Allocation/lifetime reconciliation for eight coarse and twenty-four pair records.
use super::{GLOBAL, Result, no_events};
use multiway_incidence::{
    FactorAggregation, FactorPair, IncidenceError, PreparedCoarseTupleMap, PreparedPairEdgeMap,
    PreparedThreeWayTopology,
};
use std::hint::black_box;

fn coarse(source: &PreparedThreeWayTopology, aggregation: &FactorAggregation) -> Result<()> {
    let bound = PreparedCoarseTupleMap::setup_payload_bound(source, aggregation)?;
    let before = GLOBAL.stats();
    let error =
        PreparedCoarseTupleMap::try_new_with_budget(source, aggregation, bound - 1).unwrap_err();
    no_events(GLOBAL.stats() - before);
    assert!(
        matches!(error, IncidenceError::SymbolicSetupBudgetExceeded { required, budget, .. }
        if required == bound && budget == bound - 1)
    );
    let before = GLOBAL.stats();
    let map = black_box(PreparedCoarseTupleMap::try_new_with_budget(
        source,
        aggregation,
        bound,
    )?);
    let setup = GLOBAL.stats() - before;
    let retained = map.retained_payload_bytes()?;
    assert_eq!(setup.allocations, 10);
    assert_eq!(setup.reallocations, 0);
    assert_eq!(setup.bytes_allocated - setup.bytes_deallocated, retained);
    assert!(setup.bytes_allocated <= bound);
    let count = source.topology().tuple_count();
    let values: Vec<_> = (0..map.coarse().topology().tuple_count()).collect();
    let mut output = vec![usize::MAX; count];
    let equal_aggregation = aggregation.clone();
    let foreign = PreparedThreeWayTopology::try_from_collapsed(
        source.topology().level_counts(),
        source.topology().tuples(),
    )?;
    let before = GLOBAL.stats();
    for _ in 0..64 {
        map.validate_for(source, aggregation)?;
        assert_eq!(map.retained_payload_bytes()?, retained);
        map.scatter_coarse_values_into(
            source,
            aggregation,
            black_box(&values),
            black_box(&mut output),
        )?;
        black_box(map.coarse_to_fine_components());
        black_box(map.fine_to_coarse_components());
    }
    no_events(GLOBAL.stats() - before);
    assert_eq!(output, map.merge_groups().source_to_group());
    let snapshot = output.clone();
    let before = GLOBAL.stats();
    let foreign_error = map
        .scatter_coarse_values_into(&foreign, aggregation, &values, &mut output)
        .unwrap_err();
    let map_error = map
        .scatter_coarse_values_into(source, &equal_aggregation, &values, &mut output)
        .unwrap_err();
    let size_error = map
        .scatter_coarse_values_into(
            source,
            aggregation,
            &values[..values.len() - 1],
            &mut output,
        )
        .unwrap_err();
    no_events(GLOBAL.stats() - before);
    assert!(matches!(
        foreign_error,
        IncidenceError::TopologyBindingMismatch
    ));
    assert!(matches!(
        map_error,
        IncidenceError::AggregationBindingMismatch
    ));
    assert!(matches!(
        size_error,
        IncidenceError::DimensionMismatch { .. }
    ));
    assert_eq!(output, snapshot);
    let before = GLOBAL.stats();
    drop(map);
    let released = GLOBAL.stats() - before;
    assert_eq!(released.allocations, 0);
    assert_eq!(released.reallocations, 0);
    assert_eq!(released.bytes_deallocated, retained);
    println!(
        "symbolic-coarse tuples={count} bound={bound} allocated={} scratch_released={} retained={retained} release=exact read/scatter/reject_allocations=0",
        setup.bytes_allocated, setup.bytes_deallocated
    );
    Ok(())
}

fn pair(source: &PreparedThreeWayTopology, pair: FactorPair) -> Result<()> {
    let bound = PreparedPairEdgeMap::setup_payload_bound(source)?;
    let before = GLOBAL.stats();
    let error = PreparedPairEdgeMap::try_new_with_budget(source, pair, bound - 1).unwrap_err();
    no_events(GLOBAL.stats() - before);
    assert!(matches!(
        error,
        IncidenceError::SymbolicSetupBudgetExceeded { .. }
    ));
    let before = GLOBAL.stats();
    let map = black_box(PreparedPairEdgeMap::try_new_with_budget(
        source, pair, bound,
    )?);
    let setup = GLOBAL.stats() - before;
    let retained = map.retained_payload_bytes()?;
    assert_eq!(setup.allocations, 4);
    assert_eq!(setup.reallocations, 0);
    assert_eq!(setup.deallocations, 0);
    assert_eq!(setup.bytes_allocated, retained);
    assert!(retained <= bound);
    let count = source.topology().tuple_count();
    let values: Vec<_> = (0..map.edges().len()).collect();
    let mut output = vec![usize::MAX; count];
    let mut endpoints = vec![[usize::MAX; 2]; map.edges().len()];
    let foreign = PreparedThreeWayTopology::try_from_collapsed(
        source.topology().level_counts(),
        source.topology().tuples(),
    )?;
    let wrong_pair = if pair == FactorPair::OneTwo {
        FactorPair::OneThree
    } else {
        FactorPair::OneTwo
    };
    let before = GLOBAL.stats();
    for _ in 0..64 {
        map.validate_for(source, pair)?;
        assert_eq!(map.retained_payload_bytes()?, retained);
        map.scatter_edge_values_into(source, pair, black_box(&values), black_box(&mut output))?;
        map.write_local_endpoints_into(source, pair, black_box(&mut endpoints))?;
    }
    no_events(GLOBAL.stats() - before);
    assert_eq!(output, map.merge_groups().source_to_group());
    let snapshot = output.clone();
    let saved_endpoints = endpoints.clone();
    let before = GLOBAL.stats();
    let foreign_error = map
        .scatter_edge_values_into(&foreign, pair, &values, &mut output)
        .unwrap_err();
    let pair_error = map
        .scatter_edge_values_into(source, wrong_pair, &values, &mut output)
        .unwrap_err();
    let size_error = map
        .scatter_edge_values_into(source, pair, &values[..values.len() - 1], &mut output)
        .unwrap_err();
    let endpoint_error = map
        .write_local_endpoints_into(source, wrong_pair, &mut endpoints)
        .unwrap_err();
    no_events(GLOBAL.stats() - before);
    assert!(matches!(
        foreign_error,
        IncidenceError::TopologyBindingMismatch
    ));
    assert!(matches!(pair_error, IncidenceError::FactorPairMismatch));
    assert!(matches!(endpoint_error, IncidenceError::FactorPairMismatch));
    assert!(matches!(
        size_error,
        IncidenceError::DimensionMismatch { .. }
    ));
    assert_eq!(output, snapshot);
    assert_eq!(endpoints, saved_endpoints);
    let before = GLOBAL.stats();
    drop(map);
    let released = GLOBAL.stats() - before;
    assert_eq!(released.allocations, 0);
    assert_eq!(released.reallocations, 0);
    assert_eq!(released.bytes_deallocated, retained);
    println!(
        "symbolic-pair pair={pair:?} tuples={count} bound={bound} retained={retained} release=exact read/scatter/endpoints/reject_allocations=0"
    );
    Ok(())
}

pub(super) fn run() -> Result<()> {
    let mut full: Vec<_> = (0..4)
        .flat_map(|i| (0..4).flat_map(move |j| (0..4).map(move |k| [i, j, k])))
        .collect();
    full.extend(full.clone().into_iter().rev());
    let cases = [
        ([1; 3], vec![[0; 3]; 5]),
        ([2; 3], vec![[1; 3], [0; 3], [1; 3]]),
        (
            [2, 3, 4],
            (0..2)
                .flat_map(|i| (0..3).flat_map(move |j| (0..4).map(move |k| [i, j, k])))
                .collect(),
        ),
        ([4; 3], full),
    ];
    let mut checked = 0;
    for (counts, rows) in cases {
        let observed = PreparedThreeWayTopology::try_from_observations(counts, &rows)?;
        let collapsed =
            PreparedThreeWayTopology::try_from_collapsed(counts, observed.topology().tuples())?;
        for source in [&observed, &collapsed] {
            let aggregation = if source.component_factor_sizes().len() > 1 {
                FactorAggregation::identity(counts)?
            } else {
                FactorAggregation::consecutive_halving(counts)?
            };
            let before = source.retained_payload_bytes()?;
            coarse(source, &aggregation)?;
            for factor_pair in FactorPair::ALL {
                pair(source, factor_pair)?;
            }
            assert_eq!(source.retained_payload_bytes()?, before);
            checked += 1;
        }
    }
    assert_eq!(checked, 8);
    println!(
        "PASS symbolic-maps coarse=8 pair=24 exact exclusive lifetimes and allocation-free symbolic operations"
    );
    Ok(())
}
