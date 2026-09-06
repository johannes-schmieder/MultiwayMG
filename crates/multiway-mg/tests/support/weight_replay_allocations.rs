//! Measured replay construction/destruction, changed-frame cycles and static rejection.
use super::{GLOBAL, Result, equal_bits, no_events};
use multiway_incidence::{
    CoarseWeightReplay, FactorAggregation, FactorPair, PairConductanceReplay,
    PreparedCoarseTupleMap, PreparedPairEdgeMap, PreparedThreeWayTopology, ThreeWayWeightFrame,
    WeightFrameInput, WeightReplayPayloadBudget,
};
use std::hint::black_box;

fn coarse(map: &PreparedCoarseTupleMap<'_>, parent: &ThreeWayWeightFrame<'_>) -> Result<()> {
    let old = CoarseWeightReplay::try_new(map, parent)?;
    let old_bytes = old.retained_payload_bytes()?;
    let report = CoarseWeightReplay::setup_payload_report(map, parent, old_bytes)?;
    let budget = |maximum_payload_bytes| WeightReplayPayloadBudget { maximum_payload_bytes, additional_live_payload_bytes: old_bytes };
    let before = GLOBAL.stats();
    let error = CoarseWeightReplay::try_new_with_budget(map, parent, budget(report.total_payload_bytes - 1)).unwrap_err();
    no_events(GLOBAL.stats() - before);
    drop(error);
    let before = GLOBAL.stats();
    let replay = black_box(CoarseWeightReplay::try_new_with_budget(map, parent, budget(report.total_payload_bytes))?);
    let setup = GLOBAL.stats() - before;
    let retained = replay.retained_payload_bytes()?;
    assert_eq!(setup.allocations, 5);
    assert_eq!(setup.reallocations, 0);
    assert_eq!(setup.deallocations, 1);
    assert_eq!(setup.bytes_allocated, report.new_arrays_payload_bytes);
    assert_eq!(setup.bytes_deallocated, map.coarse().topology().total_levels() * 8);
    assert_eq!(setup.bytes_allocated - setup.bytes_deallocated, retained);
    equal_bits(replay.frame().weights(), old.frame().weights());
    let alternative = PreparedCoarseTupleMap::try_new(map.source(), map.aggregation())?;
    let equal = ThreeWayWeightFrame::try_new(map.source(), WeightFrameInput::Tuples(parent.weights()))?;
    let mut out = vec![0.0; replay.frame().weights().len()];
    let before = GLOBAL.stats();
    for _ in 0..64 {
        replay.validate_for(map, parent)?;
        replay.copy_weights_into(map, parent, black_box(&mut out))?;
        assert_eq!(CoarseWeightReplay::setup_payload_report(map, parent, old_bytes)?, report);
        assert_eq!(replay.retained_payload_bytes()?, retained);
        black_box(replay.frame().diagonal());
        black_box(replay.frame().square_root_weights());
        black_box(replay.frame().component_ranges());
    }
    no_events(GLOBAL.stats() - before);
    let saved = out.clone();
    let before = GLOBAL.stats();
    assert!(replay.copy_weights_into(&alternative, parent, &mut out).is_err());
    assert!(replay.copy_weights_into(map, &equal, &mut out).is_err());
    assert!(replay.copy_weights_into(map, parent, &mut out[..0]).is_err());
    no_events(GLOBAL.stats() - before);
    equal_bits(&out, &saved);
    let before = GLOBAL.stats();
    drop(replay);
    let released = GLOBAL.stats() - before;
    assert_eq!(released.allocations, 0);
    assert_eq!(released.reallocations, 0);
    assert_eq!(released.bytes_deallocated, retained);
    let mut weights = parent.weights().to_vec();
    for step in 0..16 {
        for (i, weight) in weights.iter_mut().enumerate() {
            *weight = parent.weights()[i] * (1.0 + step as f64 / 16.0) * if (i + step) % 5 == 0 { 1024.0 } else { 1.0 };
        }
        let before = GLOBAL.stats();
        let fresh = ThreeWayWeightFrame::try_new(map.source(), WeightFrameInput::Tuples(&weights))?;
        let result = CoarseWeightReplay::try_new(map, &fresh)?;
        assert!(old.validate_for(map, &fresh).is_err());
        assert_ne!(result.source_binding(), old.source_binding());
        drop(result);
        drop(fresh);
        let cycle = GLOBAL.stats() - before;
        assert_eq!(cycle.allocations, cycle.deallocations);
        assert_eq!(cycle.bytes_allocated, cycle.bytes_deallocated);
        assert_eq!(cycle.reallocations, 0);
    }
    old.copy_weights_into(map, parent, &mut out)?;
    equal_bits(&out, &saved);
    println!("coarse-replay fine_tuples={} coarse_tuples={} new_setup={} retained={retained} live_budget={} read/copy/reject_allocations=0 release=exact changed16=balanced",
        parent.weights().len(), out.len(), report.new_arrays_payload_bytes, report.total_payload_bytes);
    Ok(())
}

fn pair(map: &PreparedPairEdgeMap<'_>, parent: &ThreeWayWeightFrame<'_>) -> Result<()> {
    let old = PairConductanceReplay::try_new(map, parent)?;
    let old_bytes = old.retained_payload_bytes()?;
    let report = PairConductanceReplay::setup_payload_report(map, parent, old_bytes)?;
    let budget = |maximum_payload_bytes| WeightReplayPayloadBudget { maximum_payload_bytes, additional_live_payload_bytes: old_bytes };
    let before = GLOBAL.stats();
    let error = PairConductanceReplay::try_new_with_budget(map, parent, budget(report.total_payload_bytes - 1)).unwrap_err();
    no_events(GLOBAL.stats() - before);
    drop(error);
    let before = GLOBAL.stats();
    let replay = black_box(PairConductanceReplay::try_new_with_budget(map, parent, budget(report.total_payload_bytes))?);
    let setup = GLOBAL.stats() - before;
    let retained = replay.retained_payload_bytes()?;
    assert_eq!(setup.allocations, 3);
    assert_eq!(setup.reallocations, 0);
    assert_eq!(setup.deallocations, 1);
    assert_eq!(setup.bytes_allocated, report.new_arrays_payload_bytes);
    assert_eq!(setup.bytes_deallocated, map.local_dimension() * 16);
    assert_eq!(setup.bytes_allocated - setup.bytes_deallocated, retained);
    let alternative = PreparedPairEdgeMap::try_new(map.source(), map.pair())?;
    let equal = ThreeWayWeightFrame::try_new(map.source(), WeightFrameInput::Tuples(parent.weights()))?;
    let mut out = vec![0.0; map.edges().len()];
    let mut edges = vec![(0, 0, 0.0); map.edges().len()];
    let before = GLOBAL.stats();
    for _ in 0..64 {
        replay.copy_conductances_into(map, parent, black_box(&mut out))?;
        replay.write_edges_into(map, parent, black_box(&mut edges))?;
        assert_eq!(PairConductanceReplay::setup_payload_report(map, parent, old_bytes)?, report);
        assert_eq!(replay.retained_payload_bytes()?, retained);
        black_box(replay.diagonal());
    }
    no_events(GLOBAL.stats() - before);
    let saved = out.clone();
    let saved_edges = edges.clone();
    let before = GLOBAL.stats();
    assert!(replay.copy_conductances_into(&alternative, parent, &mut out).is_err());
    assert!(replay.copy_conductances_into(map, &equal, &mut out).is_err());
    assert!(replay.copy_conductances_into(map, parent, &mut out[..0]).is_err());
    assert!(replay.write_edges_into(&alternative, parent, &mut edges).is_err());
    assert!(replay.write_edges_into(map, &equal, &mut edges).is_err());
    assert!(replay.write_edges_into(map, parent, &mut edges[..0]).is_err());
    no_events(GLOBAL.stats() - before);
    equal_bits(&out, &saved);
    assert_eq!(edges, saved_edges);
    let before = GLOBAL.stats();
    drop(replay);
    let released = GLOBAL.stats() - before;
    assert_eq!(released.allocations, 0);
    assert_eq!(released.reallocations, 0);
    assert_eq!(released.bytes_deallocated, retained);
    let mut weights = parent.weights().to_vec();
    for step in 0..16 {
        for (i, weight) in weights.iter_mut().enumerate() {
            *weight = parent.weights()[i] * (1.0 + step as f64 / 16.0) * if (i + step) % 5 == 0 { 1024.0 } else { 1.0 };
        }
        let before = GLOBAL.stats();
        let fresh = ThreeWayWeightFrame::try_new(map.source(), WeightFrameInput::Tuples(&weights))?;
        let result = PairConductanceReplay::try_new(map, &fresh)?;
        assert!(old.validate_for(map, &fresh).is_err());
        drop(result);
        drop(fresh);
        let cycle = GLOBAL.stats() - before;
        assert_eq!(cycle.allocations, cycle.deallocations);
        assert_eq!(cycle.bytes_allocated, cycle.bytes_deallocated);
        assert_eq!(cycle.reallocations, 0);
    }
    old.copy_conductances_into(map, parent, &mut out)?;
    equal_bits(&out, &saved);
    println!("pair-replay pair={:?} fine_tuples={} edges={} new_setup={} retained={retained} live_budget={} read/copy/write/reject_allocations=0 release=exact changed16=balanced",
        map.pair(), parent.weights().len(), out.len(), report.new_arrays_payload_bytes, report.total_payload_bytes);
    Ok(())
}

fn numerical_failure_cleanup() -> Result<()> {
    let t = PreparedThreeWayTopology::try_from_collapsed([2; 3], &[[0, 0, 0], [0, 1, 1], [1, 1, 1]])?;
    let frame = ThreeWayWeightFrame::try_new(&t, WeightFrameInput::Tuples(&[f64::MAX * 0.75, 1.0, f64::MAX * 0.75]))?;
    let all = FactorAggregation::consecutive_halving([2; 3])?;
    let one = FactorAggregation::new([2; 3], [vec![0, 0], vec![0, 1], vec![0, 1]])?;
    for (a, expected) in [(&all, 1), (&one, 4)] {
        let map = PreparedCoarseTupleMap::try_new(&t, a)?;
        let before = GLOBAL.stats();
        let result = CoarseWeightReplay::try_new(&map, &frame);
        let stats = GLOBAL.stats() - before;
        assert!(result.is_err());
        assert_eq!(stats.allocations, expected);
        assert_eq!(stats.reallocations, 0);
        assert_eq!(stats.allocations, stats.deallocations);
        assert_eq!(stats.bytes_allocated, stats.bytes_deallocated);
    }
    let t = PreparedThreeWayTopology::try_from_collapsed([1, 2, 2], &[[0, 0, 0], [0, 0, 1], [0, 1, 0]])?;
    let ulp = 2.0_f64.powi(971);
    let frame = ThreeWayWeightFrame::try_new(&t, WeightFrameInput::Tuples(&[
        f64::from_bits(f64::MAX.to_bits() - 1), ulp * 0.5, ulp * 0.53125,
    ]))?;
    let map = PreparedPairEdgeMap::try_new(&t, FactorPair::OneThree)?;
    let before = GLOBAL.stats();
    let result = PairConductanceReplay::try_new(&map, &frame);
    let stats = GLOBAL.stats() - before;
    assert!(result.is_err());
    assert_eq!(stats.allocations, 2);
    assert_eq!(stats.reallocations, 0);
    assert_eq!(stats.allocations, stats.deallocations);
    assert_eq!(stats.bytes_allocated, stats.bytes_deallocated);
    println!("weight-replay numerical failure allocations=1/4/2 all partial arrays released");
    Ok(())
}

pub(super) fn run() -> Result<()> {
    let cases = [
        ([1; 3], vec![[0; 3]; 5]),
        ([2; 3], vec![[1; 3], [0; 3], [1; 3]]),
        ([2, 3, 4], (0..2).flat_map(|i| (0..3).flat_map(move |j| (0..4).map(move |k| [i, j, k]))).collect()),
        ([4; 3], (0..128).map(|i| [(i % 4) as u32, ((i / 4) % 4) as u32, ((i / 16) % 4) as u32]).collect()),
    ];
    let mut checked = 0;
    for (counts, rows) in cases {
        let observed = PreparedThreeWayTopology::try_from_observations(counts, &rows)?;
        let collapsed = PreparedThreeWayTopology::try_from_collapsed(counts, observed.topology().tuples())?;
        for (t, input) in [(&observed, WeightFrameInput::UnitObservations), (&collapsed, WeightFrameInput::UnitTuples)] {
            let frame = ThreeWayWeightFrame::try_new(t, input)?;
            let a = if t.component_factor_sizes().len() > 1 { FactorAggregation::identity(counts)? }
                else { FactorAggregation::consecutive_halving(counts)? };
            coarse(&PreparedCoarseTupleMap::try_new(t, &a)?, &frame)?;
            for p in FactorPair::ALL { pair(&PreparedPairEdgeMap::try_new(t, p)?, &frame)?; }
            checked += 1;
        }
    }
    assert_eq!(checked, 8);
    numerical_failure_cleanup()?;
    println!("PASS weight-replay coarse=8 pair=24 exact lifetimes; changed-frame cycles balanced; reads/static rejection allocate nothing");
    Ok(())
}
