from pathlib import Path

def put(path, content):
    p = Path(path)
    assert not p.exists(), path
    p.parent.mkdir(parents=True, exist_ok=True)
    p.write_text(content, encoding='utf-8')
def replace(path, old, new):
    p = Path(path)
    text = p.read_text()
    assert text.count(old) == 1, (path, text.count(old))
    p.write_text(text.replace(old, new), encoding='utf-8')

put('crates/multiway-incidence/tests/symbolic_maps.rs', r'''//! Independent grouping, exact-component invariants and fresh Galerkin references.

use std::collections::BTreeMap;
use multiway_incidence::{
    FactorAggregation, FactorPair, IncidenceError, PreparedCoarseTupleMap,
    PreparedPairEdgeMap, PreparedThreeWayTopology, ThreeWayProblem, TupleMergeGroups,
};

fn full(counts: [usize; 3]) -> Vec<[u32; 3]> {
    (0..counts[0]).flat_map(|i| (0..counts[1]).flat_map(move |j| {
        (0..counts[2]).map(move |k| [i as u32, j as u32, k as u32])
    })).collect()
}

fn check_groups<K: Copy + Ord + std::fmt::Debug>(
    keys: &[K], groups: &TupleMergeGroups, count: usize, key_of: impl Fn(usize) -> K,
) {
    let mut reference: BTreeMap<K, Vec<usize>> = BTreeMap::new();
    for index in 0..count { reference.entry(key_of(index)).or_default().push(index); }
    assert_eq!(keys, reference.keys().copied().collect::<Vec<_>>());
    assert_eq!(groups.source_to_group().len(), count);
    assert_eq!(groups.grouped_sources().len(), count);
    assert_eq!(groups.offsets().len(), keys.len() + 1);
    assert_eq!(groups.offsets().first(), Some(&0));
    assert_eq!(groups.offsets().last(), Some(&count));
    for (group, expected) in reference.values().enumerate() {
        let begin = groups.offsets()[group];
        let end = groups.offsets()[group + 1];
        assert_eq!(&groups.grouped_sources()[begin..end], expected);
        for &source in expected { assert_eq!(groups.source_to_group()[source], group); }
    }
    let mut permutation = groups.grouped_sources().to_vec();
    permutation.sort_unstable();
    assert_eq!(permutation, (0..count).collect::<Vec<_>>());
}

fn check_pair(source: &PreparedThreeWayTopology, pair: FactorPair) {
    let map = PreparedPairEdgeMap::try_new(source, pair).unwrap();
    let (left, right) = pair.factors();
    check_groups(map.edges(), map.merge_groups(), source.topology().tuple_count(), |index| {
        let tuple = source.topology().tuples()[index];
        [tuple[left], tuple[right]]
    });
    assert!(std::ptr::eq(map.source(), source));
    assert_eq!(map.source_binding(), source.binding());
    assert_eq!(map.pair(), pair);
    let mut local = vec![[usize::MAX; 2]; map.edges().len()];
    map.write_local_endpoints_into(source, pair, &mut local).unwrap();
    let counts = source.topology().level_counts();
    assert_eq!(map.level_counts(), [counts[left], counts[right]]);
    assert_eq!(map.local_dimension(), counts[left] + counts[right]);
    for (&key, &edge) in map.edges().iter().zip(&local) {
        assert_eq!(edge, [key[0] as usize, counts[left] + key[1] as usize]);
        assert!(edge[0] < counts[left] && edge[1] < map.local_dimension());
    }
    let values: Vec<_> = (0..map.edges().len()).collect();
    let mut out = vec![usize::MAX; source.topology().tuple_count()];
    map.scatter_edge_values_into(source, pair, &values, &mut out).unwrap();
    assert_eq!(out, map.merge_groups().source_to_group());
}

fn check_coarse(source: &PreparedThreeWayTopology, aggregation: &FactorAggregation) {
    let map = PreparedCoarseTupleMap::try_new(source, aggregation).unwrap();
    let tuples = source.topology().tuples();
    check_groups(map.coarse().topology().tuples(), map.merge_groups(), tuples.len(), |index| {
        core::array::from_fn(|factor| aggregation.parents(factor)[tuples[index][factor] as usize])
    });
    assert!(std::ptr::eq(map.source(), source));
    assert!(std::ptr::eq(map.aggregation(), aggregation));
    assert_eq!(map.source_binding(), source.binding());
    assert!(map.coarse().observation_groups().is_none());
    for (fine, &coarse) in map.fine_to_coarse_components().iter().enumerate() {
        assert_eq!(map.coarse_to_fine_components()[coarse], fine);
    }
    // Independent test-only compensated reduction, then the existing fresh constructor.
    let weights: Vec<_> = (0..tuples.len()).map(|i| match i % 4 {
        0 => 1.0e16, 1 | 2 => 1.0, _ => 0.25,
    }).collect();
    let fine = ThreeWayProblem::from_observations(source.topology().level_counts(), tuples, &weights).unwrap();
    let fresh = aggregation.coarsen(&fine).unwrap();
    assert_eq!(map.coarse().topology(), fresh.topology());
    assert_eq!(map.coarse().component_labels(), fresh.components().labels());
    for (group, bounds) in map.merge_groups().offsets().windows(2).enumerate() {
        let mut sum: f64 = 0.0;
        let mut correction = 0.0;
        for &index in &map.merge_groups().grouped_sources()[bounds[0]..bounds[1]] {
            let value = fine.weights()[index];
            let updated = sum + value;
            correction += if sum.abs() >= value.abs() { (sum - updated) + value } else { (value - updated) + sum };
            sum = updated;
        }
        assert_eq!((sum + correction).to_bits(), fresh.weights()[group].to_bits());
    }
    // Pure factor-shift modes must pull back to the matching fine component.
    for component in 0..map.coarse_to_fine_components().len() {
        let mut coarse = vec![0.0; map.coarse().topology().total_levels()];
        for factor in 0..3 {
            for index in map.coarse().topology().factor_range(factor) {
                if map.coarse().component_labels()[index] == component {
                    coarse[index] = [1.0, -1.0, 0.0][factor];
                }
            }
        }
        let mut pulled = vec![0.0; source.topology().total_levels()];
        aggregation.prolong(&coarse, &mut pulled).unwrap();
        for factor in 0..3 {
            for index in source.topology().factor_range(factor) {
                let expected = if source.component_labels()[index] == map.coarse_to_fine_components()[component] {
                    [1.0, -1.0, 0.0][factor]
                } else { 0.0 };
                assert_eq!(pulled[index], expected);
            }
        }
    }
}

#[test]
fn exhaustive_small_supports_check_groups_and_cross_component_rejection() {
    let universe = full([2; 3]);
    let identity = FactorAggregation::identity([2; 3]).unwrap();
    let collapse = FactorAggregation::consecutive_halving([2; 3]).unwrap();
    let permutation = FactorAggregation::new([2; 3], [vec![1,0], vec![0,1], vec![1,0]]).unwrap();
    for mask in 1..256 {
        let tuples: Vec<_> = universe.iter().enumerate().filter_map(|(i, tuple)| {
            ((mask >> i) & 1 == 1).then_some(*tuple)
        }).collect();
        let source = match PreparedThreeWayTopology::try_from_collapsed([2; 3], &tuples) {
            Ok(source) => source,
            Err(IncidenceError::UnusedLevel { .. }) => continue,
            Err(error) => panic!("unexpected support error {error}"),
        };
        for pair in FactorPair::ALL { check_pair(&source, pair); }
        check_coarse(&source, &identity);
        check_coarse(&source, &permutation);
        if source.component_factor_sizes().len() == 1 {
            check_coarse(&source, &collapse);
        } else {
            assert!(matches!(PreparedCoarseTupleMap::try_new(&source, &collapse),
                Err(IncidenceError::CrossComponentAggregation { .. })));
        }
    }
}

#[test]
fn unequal_factors_nonmonotone_parents_and_raw_observation_order() {
    let tuples = full([3, 4, 2]);
    let mut rows = tuples.clone(); rows.extend(tuples.iter().rev()); rows.rotate_left(7);
    let source = PreparedThreeWayTopology::try_from_observations([3,4,2], &rows).unwrap();
    let aggregation = FactorAggregation::new([3,4,2], [vec![1,0,1], vec![1,0,1,0], vec![0,0]]).unwrap();
    check_coarse(&source, &aggregation);
    for pair in FactorPair::ALL { check_pair(&source, pair); }
    let coarse = PreparedCoarseTupleMap::try_new(&source, &aggregation).unwrap();
    assert_eq!(coarse.merge_groups().source_to_group().len(), 24);
    assert_eq!(source.input_count(), 48);
}

#[test]
fn component_renumbering_is_explicit_and_crossing_one_factor_rejects() {
    let source = PreparedThreeWayTopology::try_from_collapsed([2;3], &[[0;3],[1;3]]).unwrap();
    let rename = FactorAggregation::new([2;3], [vec![1,0],vec![1,0],vec![1,0]]).unwrap();
    let map = PreparedCoarseTupleMap::try_new(&source, &rename).unwrap();
    assert_eq!(map.coarse_to_fine_components(), &[1,0]);
    assert_eq!(map.fine_to_coarse_components(), &[1,0]);
    check_coarse(&source, &rename);
    for factor in 0..3 {
        let mut parents = [vec![0,1],vec![0,1],vec![0,1]];
        parents[factor] = vec![0,0];
        let crossing = FactorAggregation::new([2;3], parents).unwrap();
        assert!(matches!(PreparedCoarseTupleMap::try_new(&source, &crossing),
            Err(IncidenceError::CrossComponentAggregation { factor: f, parent: 0 }) if f == factor));
    }
}

#[test]
fn pair_components_need_not_equal_three_way_components() {
    let source = PreparedThreeWayTopology::try_from_collapsed([2,2,1], &[[0,0,0],[1,1,0]]).unwrap();
    assert_eq!(source.component_factor_sizes().len(), 1);
    let pair = PreparedPairEdgeMap::try_new(&source, FactorPair::OneTwo).unwrap();
    let mut endpoints = [[0;2];2];
    pair.write_local_endpoints_into(&source, FactorPair::OneTwo, &mut endpoints).unwrap();
    assert_eq!(endpoints, [[0,2],[1,3]]); // two disjoint pair components
}

#[test]
fn exact_source_map_pair_and_dimensions_reject_before_writes() {
    let tuples = full([2;3]);
    let source = PreparedThreeWayTopology::try_from_collapsed([2;3], &tuples).unwrap();
    let foreign = PreparedThreeWayTopology::try_from_collapsed([2;3], &tuples).unwrap();
    let aggregation = FactorAggregation::consecutive_halving([2;3]).unwrap();
    let equal_map = aggregation.clone();
    let coarse = PreparedCoarseTupleMap::try_new(&source, &aggregation).unwrap();
    let mut out = [23_u64;8];
    assert!(matches!(coarse.scatter_coarse_values_into(&foreign, &aggregation, &[11], &mut out),
        Err(IncidenceError::TopologyBindingMismatch)));
    assert!(matches!(coarse.scatter_coarse_values_into(&source, &equal_map, &[11], &mut out),
        Err(IncidenceError::AggregationBindingMismatch)));
    assert!(coarse.scatter_coarse_values_into(&source, &aggregation, &[], &mut out).is_err());
    assert!(coarse.scatter_coarse_values_into(&source, &aggregation, &[11], &mut out[..7]).is_err());
    assert_eq!(out, [23;8]);
    let pair = PreparedPairEdgeMap::try_new(&source, FactorPair::OneTwo).unwrap();
    assert!(matches!(pair.scatter_edge_values_into(&foreign, FactorPair::OneTwo, &[1,2,3,4], &mut out),
        Err(IncidenceError::TopologyBindingMismatch)));
    assert!(matches!(pair.scatter_edge_values_into(&source, FactorPair::TwoThree, &[1,2,3,4], &mut out),
        Err(IncidenceError::FactorPairMismatch)));
    assert!(pair.scatter_edge_values_into(&source, FactorPair::OneTwo, &[1,2,3], &mut out).is_err());
    assert!(pair.scatter_edge_values_into(&source, FactorPair::OneTwo, &[1,2,3,4], &mut out[..7]).is_err());
    assert_eq!(out, [23;8]);
    let mut endpoints = [[23;2];4];
    assert!(pair.write_local_endpoints_into(&source, FactorPair::OneThree, &mut endpoints).is_err());
    assert!(pair.write_local_endpoints_into(&foreign, FactorPair::OneTwo, &mut endpoints).is_err());
    assert!(pair.write_local_endpoints_into(&source, FactorPair::OneTwo, &mut endpoints[..3]).is_err());
    assert_eq!(endpoints, [[23;2];4]);
    let nan = f64::from_bits(0x7ff8_0000_0000_0345);
    let values = [nan, -0.0, f64::INFINITY, -3.0];
    let mut float_out = [0.0_f64;8];
    pair.scatter_edge_values_into(&source, FactorPair::OneTwo, &values, &mut float_out).unwrap();
    for (&actual, &group) in float_out.iter().zip(pair.merge_groups().source_to_group()) {
        assert_eq!(actual.to_bits(), values[group].to_bits());
    }
    coarse.scatter_coarse_values_into(&source, &aggregation, &[nan], &mut float_out).unwrap();
    assert!(float_out.iter().all(|v| v.to_bits() == nan.to_bits()));
}

#[test]
fn chained_transitions_and_concurrent_borrows_preserve_source_owners() {
    let source = PreparedThreeWayTopology::try_from_collapsed([4;3], &full([4;3])).unwrap();
    let map1 = FactorAggregation::consecutive_halving([4;3]).unwrap();
    let map2 = FactorAggregation::consecutive_halving([2;3]).unwrap();
    let first = PreparedCoarseTupleMap::try_new(&source, &map1).unwrap();
    let second = PreparedCoarseTupleMap::try_new(first.coarse(), &map2).unwrap();
    assert_eq!(second.coarse().topology().tuples(), &[[0;3]]);
    let pair = PreparedPairEdgeMap::try_new(first.coarse(), FactorPair::TwoThree).unwrap();
    assert!(std::ptr::eq(pair.source(), first.coarse()));
    std::thread::scope(|scope| {
        let a = scope.spawn(|| {
            let mut out = [0;8];
            second.scatter_coarse_values_into(first.coarse(), &map2, &[9], &mut out).unwrap();
            out
        });
        let b = scope.spawn(|| {
            let mut out = [0;8];
            pair.scatter_edge_values_into(first.coarse(), FactorPair::TwoThree, &[1,2,3,4], &mut out).unwrap();
            out
        });
        assert_eq!(a.join().unwrap(), [9;8]);
        assert_eq!(b.join().unwrap(), [1,2,3,4,1,2,3,4]);
    });
}
''')

put('crates/multiway-mg/tests/support/symbolic_map_allocations.rs', r'''//! Allocation/lifetime reconciliation for eight coarse and twenty-four pair records.
use super::{GLOBAL, Result, no_events};
use multiway_incidence::{
    FactorAggregation, FactorPair, IncidenceError, PreparedCoarseTupleMap,
    PreparedPairEdgeMap, PreparedThreeWayTopology,
};
use std::hint::black_box;

fn coarse(source: &PreparedThreeWayTopology, aggregation: &FactorAggregation) -> Result<()> {
    let bound = PreparedCoarseTupleMap::setup_payload_bound(source, aggregation)?;
    let before = GLOBAL.stats();
    let error = PreparedCoarseTupleMap::try_new_with_budget(source, aggregation, bound - 1).unwrap_err();
    no_events(GLOBAL.stats() - before);
    assert!(matches!(error, IncidenceError::SymbolicSetupBudgetExceeded { required, budget, .. }
        if required == bound && budget == bound - 1));
    let before = GLOBAL.stats();
    let map = black_box(PreparedCoarseTupleMap::try_new_with_budget(source, aggregation, bound)?);
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
    let foreign = PreparedThreeWayTopology::try_from_collapsed(source.topology().level_counts(), source.topology().tuples())?;
    let before = GLOBAL.stats();
    for _ in 0..64 {
        map.validate_for(source, aggregation)?;
        assert_eq!(map.retained_payload_bytes()?, retained);
        map.scatter_coarse_values_into(source, aggregation, black_box(&values), black_box(&mut output))?;
        black_box(map.coarse_to_fine_components());
        black_box(map.fine_to_coarse_components());
    }
    no_events(GLOBAL.stats() - before);
    assert_eq!(output, map.merge_groups().source_to_group());
    let snapshot = output.clone();
    let before = GLOBAL.stats();
    let foreign_error = map.scatter_coarse_values_into(&foreign, aggregation, &values, &mut output).unwrap_err();
    let map_error = map.scatter_coarse_values_into(source, &equal_aggregation, &values, &mut output).unwrap_err();
    let size_error = map.scatter_coarse_values_into(source, aggregation, &values[..values.len()-1], &mut output).unwrap_err();
    no_events(GLOBAL.stats() - before);
    assert!(matches!(foreign_error, IncidenceError::TopologyBindingMismatch));
    assert!(matches!(map_error, IncidenceError::AggregationBindingMismatch));
    assert!(matches!(size_error, IncidenceError::DimensionMismatch { .. }));
    assert_eq!(output, snapshot);
    let before = GLOBAL.stats();
    drop(map);
    let released = GLOBAL.stats() - before;
    assert_eq!(released.allocations, 0);
    assert_eq!(released.reallocations, 0);
    assert_eq!(released.bytes_deallocated, retained);
    println!("symbolic-coarse tuples={count} bound={bound} allocated={} scratch_released={} retained={retained} release=exact read/scatter/reject_allocations=0", setup.bytes_allocated, setup.bytes_deallocated);
    Ok(())
}

fn pair(source: &PreparedThreeWayTopology, pair: FactorPair) -> Result<()> {
    let bound = PreparedPairEdgeMap::setup_payload_bound(source)?;
    let before = GLOBAL.stats();
    let error = PreparedPairEdgeMap::try_new_with_budget(source, pair, bound-1).unwrap_err();
    no_events(GLOBAL.stats() - before);
    assert!(matches!(error, IncidenceError::SymbolicSetupBudgetExceeded { .. }));
    let before = GLOBAL.stats();
    let map = black_box(PreparedPairEdgeMap::try_new_with_budget(source, pair, bound)?);
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
    let mut endpoints = vec![[usize::MAX;2]; map.edges().len()];
    let foreign = PreparedThreeWayTopology::try_from_collapsed(source.topology().level_counts(), source.topology().tuples())?;
    let wrong_pair = if pair == FactorPair::OneTwo { FactorPair::OneThree } else { FactorPair::OneTwo };
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
    let foreign_error = map.scatter_edge_values_into(&foreign, pair, &values, &mut output).unwrap_err();
    let pair_error = map.scatter_edge_values_into(source, wrong_pair, &values, &mut output).unwrap_err();
    let size_error = map.scatter_edge_values_into(source, pair, &values[..values.len()-1], &mut output).unwrap_err();
    let endpoint_error = map.write_local_endpoints_into(source, wrong_pair, &mut endpoints).unwrap_err();
    no_events(GLOBAL.stats() - before);
    assert!(matches!(foreign_error, IncidenceError::TopologyBindingMismatch));
    assert!(matches!(pair_error, IncidenceError::FactorPairMismatch));
    assert!(matches!(endpoint_error, IncidenceError::FactorPairMismatch));
    assert!(matches!(size_error, IncidenceError::DimensionMismatch { .. }));
    assert_eq!(output, snapshot);
    assert_eq!(endpoints, saved_endpoints);
    let before = GLOBAL.stats();
    drop(map);
    let released = GLOBAL.stats() - before;
    assert_eq!(released.allocations, 0);
    assert_eq!(released.reallocations, 0);
    assert_eq!(released.bytes_deallocated, retained);
    println!("symbolic-pair pair={pair:?} tuples={count} bound={bound} retained={retained} release=exact read/scatter/endpoints/reject_allocations=0");
    Ok(())
}

pub(super) fn run() -> Result<()> {
    let mut full: Vec<_> = (0..4).flat_map(|i| (0..4).flat_map(move |j| (0..4).map(move |k| [i,j,k]))).collect();
    full.extend(full.clone().into_iter().rev());
    let cases = [
        ([1;3], vec![[0;3];5]),
        ([2;3], vec![[1;3],[0;3],[1;3]]),
        ([2,3,4], (0..2).flat_map(|i| (0..3).flat_map(move |j| (0..4).map(move |k| [i,j,k]))).collect()),
        ([4;3], full),
    ];
    let mut checked = 0;
    for (counts, rows) in cases {
        let observed = PreparedThreeWayTopology::try_from_observations(counts, &rows)?;
        let collapsed = PreparedThreeWayTopology::try_from_collapsed(counts, observed.topology().tuples())?;
        for source in [&observed, &collapsed] {
            let aggregation = if source.component_factor_sizes().len() > 1 {
                FactorAggregation::identity(counts)?
            } else {
                FactorAggregation::consecutive_halving(counts)?
            };
            let before = source.retained_payload_bytes()?;
            coarse(source, &aggregation)?;
            for factor_pair in FactorPair::ALL { pair(source, factor_pair)?; }
            assert_eq!(source.retained_payload_bytes()?, before);
            checked += 1;
        }
    }
    assert_eq!(checked, 8);
    println!("PASS symbolic-maps coarse=8 pair=24 exact exclusive lifetimes and allocation-free symbolic operations");
    Ok(())
}
''')

replace('crates/multiway-mg/tests/workspace_allocations.rs',
    'mod prepared_topology_allocations;\n',
    'mod prepared_topology_allocations;\n#[path = "support/symbolic_map_allocations.rs"]\nmod symbolic_map_allocations;\n')
replace('crates/multiway-mg/tests/workspace_allocations.rs',
    '    prepared_topology_allocations::run()?;\n',
    '    prepared_topology_allocations::run()?;\n    symbolic_map_allocations::run()?;\n')
