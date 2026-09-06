//! Independent grouped arithmetic, fresh Galerkin references and provenance tests.
use std::collections::BTreeMap;
use multiway_incidence::{
    CoarseWeightReplay, FactorAggregation, FactorPair, IncidenceError, PairConductanceReplay,
    PreparedCoarseTupleMap, PreparedPairEdgeMap, PreparedThreeWayTopology, ThreeWayProblem,
    ThreeWayWeightFrame, WeightFrameInput, WeightReplayPayloadBudget,
};

fn bits(a: &[f64], b: &[f64]) {
    assert_eq!(a.len(), b.len());
    for (&a, &b) in a.iter().zip(b) { assert_eq!(a.to_bits(), b.to_bits()); }
}

// Test-only transcription, independent of the production accumulator/degree kernel.
fn add(state: &mut (f64, f64), value: f64) {
    let next = state.0 + value;
    state.1 += if state.0.abs() >= value.abs() {
        (state.0 - next) + value
    } else { (value - next) + state.0 };
    state.0 = next;
}
fn total(values: impl IntoIterator<Item = f64>) -> f64 {
    let mut state = (0.0, 0.0);
    for value in values { add(&mut state, value); }
    state.0 + state.1
}
fn full(counts: [u32; 3]) -> Vec<[u32; 3]> {
    (0..counts[0]).flat_map(|i| (0..counts[1]).flat_map(move |j|
        (0..counts[2]).map(move |k| [i, j, k]))).collect()
}

fn check_coarse(map: &PreparedCoarseTupleMap<'_>, parent: &ThreeWayWeightFrame<'_>) {
    let replay = CoarseWeightReplay::try_new(map, parent).unwrap();
    let source = map.source().topology();
    let fine = ThreeWayProblem::from_observations(source.level_counts(), source.tuples(), parent.weights()).unwrap();
    let fresh = map.aggregation().coarsen(&fine).unwrap();
    assert_eq!(replay.frame().topology().topology(), fresh.topology());
    assert_eq!(replay.frame().topology().component_labels(), fresh.components().labels());
    bits(replay.frame().weights(), fresh.weights());
    bits(replay.frame().square_root_weights(), fresh.square_root_weights());
    bits(replay.frame().diagonal(), fresh.diagonal());
    let mut groups: BTreeMap<[u32; 3], Vec<f64>> = BTreeMap::new();
    for (tuple, &weight) in source.tuples().iter().zip(parent.weights()) {
        let key = core::array::from_fn(|f| map.aggregation().parents(f)[tuple[f] as usize]);
        groups.entry(key).or_default().push(weight);
    }
    let expected: Vec<_> = groups.values().map(|v| total(v.iter().copied())).collect();
    bits(replay.frame().weights(), &expected);
    let coarse = map.coarse().topology();
    let mut degrees = vec![(0.0, 0.0); coarse.total_levels()];
    for (&tuple, &weight) in coarse.tuples().iter().zip(&expected) {
        for f in 0..3 { add(&mut degrees[coarse.global_index(f, tuple[f])], weight); }
    }
    let degrees: Vec<_> = degrees.iter().map(|s| s.0 + s.1).collect();
    bits(replay.frame().diagonal(), &degrees);
    assert!(std::ptr::eq(replay.map(), map));
    assert!(std::ptr::eq(replay.source_frame(), parent));
    assert_eq!(replay.source_binding(), parent.binding());
    let mut output = vec![0.0; expected.len()];
    replay.copy_weights_into(map, parent, &mut output).unwrap();
    bits(&output, &expected);
}

fn check_pair(map: &PreparedPairEdgeMap<'_>, parent: &ThreeWayWeightFrame<'_>) {
    let replay = PairConductanceReplay::try_new(map, parent).unwrap();
    let (left, right) = map.pair().factors();
    let mut groups: BTreeMap<[u32; 2], Vec<f64>> = BTreeMap::new();
    for (tuple, &weight) in map.source().topology().tuples().iter().zip(parent.weights()) {
        groups.entry([tuple[left], tuple[right]]).or_default().push(weight);
    }
    assert_eq!(map.edges(), groups.keys().copied().collect::<Vec<_>>());
    let expected: Vec<_> = groups.values().map(|v| total(v.iter().copied())).collect();
    bits(replay.conductances(), &expected);
    let mut degree = vec![(0.0, 0.0); map.local_dimension()];
    let mut edges = vec![(usize::MAX, usize::MAX, -1.0); expected.len()];
    replay.write_edges_into(map, parent, &mut edges).unwrap();
    for ((&[i, j], &weight), &(l, r, w)) in groups.keys().zip(&expected).zip(&edges) {
        assert_eq!((l, r), (i as usize, map.level_counts()[0] + j as usize));
        assert_eq!(w.to_bits(), weight.to_bits());
        add(&mut degree[l], weight);
        add(&mut degree[r], weight);
    }
    let degree: Vec<_> = degree.iter().map(|s| s.0 + s.1).collect();
    bits(replay.diagonal(), &degree);
    let mut copied = vec![0.0; expected.len()];
    replay.copy_conductances_into(map, parent, &mut copied).unwrap();
    bits(&copied, &expected);
    assert_eq!(replay.source_binding(), parent.binding());
    assert!(std::ptr::eq(replay.map(), map));
    assert!(std::ptr::eq(replay.source_frame(), parent));
}

#[test]
fn all_valid_small_supports_and_nonmonotone_maps_match_independent_references() {
    let universe = full([2; 3]);
    let identity = FactorAggregation::identity([2; 3]).unwrap();
    let rename = FactorAggregation::new([2; 3], [vec![1, 0], vec![0, 1], vec![1, 0]]).unwrap();
    let collapse = FactorAggregation::consecutive_halving([2; 3]).unwrap();
    for mask in 1..256 {
        let tuples: Vec<_> = universe.iter().enumerate().filter_map(|(i, &t)|
            ((mask >> i) & 1 == 1).then_some(t)).collect();
        if (0..3).any(|f| (0..2).any(|l| !tuples.iter().any(|t| t[f] == l))) { continue; }
        let t = PreparedThreeWayTopology::try_from_collapsed([2; 3], &tuples).unwrap();
        let weights: Vec<_> = (0..tuples.len()).map(|i| [1.0e16, 1.0, 1.0, 0.25][i % 4]).collect();
        let f = ThreeWayWeightFrame::try_new(&t, WeightFrameInput::Tuples(&weights)).unwrap();
        for a in [&identity, &rename] { check_coarse(&PreparedCoarseTupleMap::try_new(&t, a).unwrap(), &f); }
        if t.component_factor_sizes().len() == 1 {
            check_coarse(&PreparedCoarseTupleMap::try_new(&t, &collapse).unwrap(), &f);
        }
        for p in FactorPair::ALL { check_pair(&PreparedPairEdgeMap::try_new(&t, p).unwrap(), &f); }
    }
}

#[test]
fn raw_duplicates_changing_frames_chaining_and_old_generation_retention() {
    let mut rows = full([3, 4, 2]);
    rows.extend(full([3, 4, 2]).into_iter().rev());
    rows.rotate_left(7);
    let t = PreparedThreeWayTopology::try_from_observations([3, 4, 2], &rows).unwrap();
    let a = FactorAggregation::new([3, 4, 2], [vec![1, 0, 1], vec![1, 0, 1, 0], vec![0, 0]]).unwrap();
    let map = PreparedCoarseTupleMap::try_new(&t, &a).unwrap();
    let a2 = FactorAggregation::consecutive_halving([2, 2, 1]).unwrap();
    let map2 = PreparedCoarseTupleMap::try_new(map.coarse(), &a2).unwrap();
    let pairs: Vec<_> = FactorPair::ALL.into_iter().map(|p| PreparedPairEdgeMap::try_new(&t, p).unwrap()).collect();
    let old = ThreeWayWeightFrame::try_new(&t, WeightFrameInput::UnitObservations).unwrap();
    let old_replay = CoarseWeightReplay::try_new(&map, &old).unwrap();
    let old_weights = old_replay.frame().weights().to_vec();
    let old_pair = PairConductanceReplay::try_new(&pairs[0], &old).unwrap();
    let old_edges = old_pair.conductances().to_vec();
    for step in 0..32 {
        let weights: Vec<_> = (0..rows.len()).map(|i| {
            let smooth = 0.25 + (i % 7) as f64 + step as f64 / 16.0;
            smooth * if i % 3 == 0 && step % 4 == 0 { 1.0e8 } else { 1.0 }
        }).collect();
        let f = ThreeWayWeightFrame::try_new(&t, WeightFrameInput::Observations(&weights)).unwrap();
        let first = CoarseWeightReplay::try_new(&map, &f).unwrap();
        check_coarse(&map, &f);
        check_coarse(&map2, first.frame());
        for pair in &pairs { check_pair(pair, &f); }
        let coarse_pair = PreparedPairEdgeMap::try_new(map.coarse(), FactorPair::OneThree).unwrap();
        check_pair(&coarse_pair, first.frame());
        assert!(matches!(old_replay.validate_for(&map, &f), Err(IncidenceError::WeightFrameBindingMismatch)));
        assert!(matches!(old_pair.validate_for(&pairs[0], &f), Err(IncidenceError::WeightFrameBindingMismatch)));
        bits(old_replay.frame().weights(), &old_weights);
        bits(old_pair.conductances(), &old_edges);
    }
    std::thread::scope(|s| {
        let c = s.spawn(|| CoarseWeightReplay::try_new(&map, &old).unwrap().frame().weights().to_vec());
        let p = s.spawn(|| PairConductanceReplay::try_new(&pairs[0], &old).unwrap().conductances().to_vec());
        bits(&c.join().unwrap(), &old_weights);
        bits(&p.join().unwrap(), &old_edges);
    });
}

#[test]
fn wrong_equal_maps_frames_and_dimensions_reject_before_any_output_writes() {
    let t = PreparedThreeWayTopology::try_from_collapsed([2; 3], &full([2; 3])).unwrap();
    let foreign = PreparedThreeWayTopology::try_from_collapsed([2; 3], &full([2; 3])).unwrap();
    let a = FactorAggregation::consecutive_halving([2; 3]).unwrap();
    let c1 = PreparedCoarseTupleMap::try_new(&t, &a).unwrap();
    let c2 = PreparedCoarseTupleMap::try_new(&t, &a).unwrap();
    let p1 = PreparedPairEdgeMap::try_new(&t, FactorPair::OneTwo).unwrap();
    let p2 = PreparedPairEdgeMap::try_new(&t, FactorPair::OneTwo).unwrap();
    let p3 = PreparedPairEdgeMap::try_new(&t, FactorPair::TwoThree).unwrap();
    let f = ThreeWayWeightFrame::try_new(&t, WeightFrameInput::UnitTuples).unwrap();
    let equal = ThreeWayWeightFrame::try_new(&t, WeightFrameInput::UnitTuples).unwrap();
    let changed = ThreeWayWeightFrame::try_new(&t, WeightFrameInput::Tuples(&[2.0; 8])).unwrap();
    let wrong = ThreeWayWeightFrame::try_new(&foreign, WeightFrameInput::UnitTuples).unwrap();
    assert!(matches!(CoarseWeightReplay::try_new(&c1, &wrong), Err(IncidenceError::TopologyBindingMismatch)));
    assert!(matches!(PairConductanceReplay::try_new(&p1, &wrong), Err(IncidenceError::TopologyBindingMismatch)));
    let coarse = CoarseWeightReplay::try_new(&c1, &f).unwrap();
    let pair = PairConductanceReplay::try_new(&p1, &f).unwrap();
    let mut out = [23.0; 4];
    let mut edges = [(23, 23, 23.0); 4];
    for parent in [&equal, &changed, &wrong] {
        assert!(coarse.copy_weights_into(&c1, parent, &mut out[..1]).is_err());
        assert!(pair.copy_conductances_into(&p1, parent, &mut out).is_err());
        assert!(pair.write_edges_into(&p1, parent, &mut edges).is_err());
    }
    assert!(matches!(coarse.copy_weights_into(&c2, &f, &mut out[..1]), Err(IncidenceError::WeightReplayMapMismatch)));
    for map in [&p2, &p3] {
        assert!(matches!(pair.copy_conductances_into(map, &f, &mut out), Err(IncidenceError::WeightReplayMapMismatch)));
        assert!(pair.write_edges_into(map, &f, &mut edges).is_err());
    }
    assert!(coarse.copy_weights_into(&c1, &f, &mut out[..0]).is_err());
    assert!(coarse.copy_weights_into(&c1, &f, &mut out).is_err());
    assert!(pair.copy_conductances_into(&p1, &f, &mut out[..3]).is_err());
    assert!(pair.write_edges_into(&p1, &f, &mut edges[..3]).is_err());
    assert_eq!(out, [23.0; 4]);
    assert_eq!(edges, [(23, 23, 23.0); 4]);
}

#[test]
fn payload_budget_charges_direct_owners_and_declared_old_new_overlap() {
    let t = PreparedThreeWayTopology::try_from_collapsed([2; 3], &full([2; 3])).unwrap();
    let a = FactorAggregation::consecutive_halving([2; 3]).unwrap();
    let c = PreparedCoarseTupleMap::try_new(&t, &a).unwrap();
    let p = PreparedPairEdgeMap::try_new(&t, FactorPair::OneThree).unwrap();
    let f = ThreeWayWeightFrame::try_new(&t, WeightFrameInput::UnitTuples).unwrap();
    let old_c = CoarseWeightReplay::try_new(&c, &f).unwrap();
    let old_p = PairConductanceReplay::try_new(&p, &f).unwrap();
    let extra = old_c.retained_payload_bytes().unwrap() + old_p.retained_payload_bytes().unwrap();
    let cr = CoarseWeightReplay::setup_payload_report(&c, &f, extra).unwrap();
    let pr = PairConductanceReplay::setup_payload_report(&p, &f, extra).unwrap();
    for report in [cr, pr] {
        assert_eq!(report.source_topology_payload_bytes, t.retained_payload_bytes().unwrap());
        assert_eq!(report.source_frame_payload_bytes, f.retained_payload_bytes().unwrap());
        assert_eq!(report.additional_live_payload_bytes, extra);
        assert_eq!(report.total_payload_bytes, report.source_topology_payload_bytes + report.symbolic_map_payload_bytes
            + report.aggregation_payload_bytes + report.source_frame_payload_bytes + report.new_arrays_payload_bytes + extra);
    }
    assert_eq!(cr.aggregation_payload_bytes, a.retained_payload_bytes().unwrap());
    assert_eq!(pr.aggregation_payload_bytes, 0);
    let budget = |n| WeightReplayPayloadBudget { maximum_payload_bytes: n, additional_live_payload_bytes: extra };
    assert!(CoarseWeightReplay::try_new_with_budget(&c, &f, budget(cr.total_payload_bytes - 1)).is_err());
    CoarseWeightReplay::try_new_with_budget(&c, &f, budget(cr.total_payload_bytes)).unwrap();
    assert!(PairConductanceReplay::try_new_with_budget(&p, &f, budget(pr.total_payload_bytes - 1)).is_err());
    PairConductanceReplay::try_new_with_budget(&p, &f, budget(pr.total_payload_bytes)).unwrap();
    assert!(CoarseWeightReplay::setup_payload_report(&c, &f, usize::MAX).is_err());
    assert!(PairConductanceReplay::setup_payload_report(&p, &f, usize::MAX).is_err());
}
