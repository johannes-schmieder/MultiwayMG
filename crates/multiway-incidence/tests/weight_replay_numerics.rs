//! Numerical boundaries: explicit legacy rounding, original-operator algebra and overflow.
use multiway_incidence::{
    CoarseWeightReplay, FactorAggregation, FactorPair, IncidenceError, PairConductanceReplay,
    PreparedCoarseTupleMap, PreparedPairEdgeMap, PreparedThreeWayTopology, ThreeWayProblem,
    ThreeWayWeightFrame, WeightFrameInput,
};

#[test]
fn compensated_pair_protocol_differs_explicitly_from_legacy_ordinary_addition() {
    let tuples = [[0, 0, 0], [0, 0, 1], [0, 0, 2]];
    let topology = PreparedThreeWayTopology::try_from_collapsed([1, 1, 3], &tuples).unwrap();
    let weights = [1.0e16, 1.0, 1.0];
    let frame =
        ThreeWayWeightFrame::try_new(&topology, WeightFrameInput::Tuples(&weights)).unwrap();
    let map = PreparedPairEdgeMap::try_new(&topology, FactorPair::OneTwo).unwrap();
    let replay = PairConductanceReplay::try_new(&map, &frame).unwrap();
    // The existing research_pair.rs PairSystem::build uses this ordinary protocol.
    // This is an explicit arithmetic reference, not execution of its private builder.
    let legacy = weights.iter().fold(0.0, |sum, &value| sum + value);
    assert_eq!(legacy, 10_000_000_000_000_000.0);
    assert_eq!(replay.conductances(), &[10_000_000_000_000_002.0]);
    assert_ne!(legacy.to_bits(), replay.conductances()[0].to_bits());
}

#[test]
fn pair_degrees_are_rebuilt_from_represented_conductances_not_fine_degrees() {
    let tuples = [[0, 0, 0], [0, 0, 1], [0, 1, 0]];
    let topology = PreparedThreeWayTopology::try_from_collapsed([1, 2, 2], &tuples).unwrap();
    let frame = ThreeWayWeightFrame::try_new(
        &topology,
        WeightFrameInput::Tuples(&[9_007_199_254_740_990.0, 0.5, 0.53125]),
    )
    .unwrap();
    let map = PreparedPairEdgeMap::try_new(&topology, FactorPair::OneThree).unwrap();
    let replay = PairConductanceReplay::try_new(&map, &frame).unwrap();
    assert_eq!(frame.diagonal()[0], 9_007_199_254_740_991.0);
    assert_eq!(replay.conductances(), &[9_007_199_254_740_991.0, 0.5]);
    assert_eq!(replay.diagonal()[0], 9_007_199_254_740_992.0);
    assert_ne!(
        replay.diagonal()[0].to_bits(),
        frame.diagonal()[0].to_bits()
    );
}

#[test]
fn valid_fine_frames_can_reject_coarse_totals_coarse_degrees_or_pair_degrees() {
    let topology =
        PreparedThreeWayTopology::try_from_collapsed([2; 3], &[[0, 0, 0], [0, 1, 1], [1, 1, 1]])
            .unwrap();
    let frame = ThreeWayWeightFrame::try_new(
        &topology,
        WeightFrameInput::Tuples(&[f64::MAX * 0.75, 1.0, f64::MAX * 0.75]),
    )
    .unwrap();
    let collapse = FactorAggregation::consecutive_halving([2; 3]).unwrap();
    let total_map = PreparedCoarseTupleMap::try_new(&topology, &collapse).unwrap();
    assert!(matches!(
        CoarseWeightReplay::try_new(&total_map, &frame),
        Err(IncidenceError::InvalidWeightFrameDerivedValue {
            context: "coarse tuple total",
            index: 0,
            ..
        })
    ));
    let one_factor = FactorAggregation::new([2; 3], [vec![0, 0], vec![0, 1], vec![0, 1]]).unwrap();
    let degree_map = PreparedCoarseTupleMap::try_new(&topology, &one_factor).unwrap();
    assert!(matches!(
        CoarseWeightReplay::try_new(&degree_map, &frame),
        Err(IncidenceError::InvalidWeightedDegree {
            factor: 0,
            level: 0,
            ..
        })
    ));

    // Fine accumulation order is [a, c, b]; edge grouping first rounds a+b to
    // MAX, then adds c. Every fine degree is finite, but that pair degree overflows.
    let pair_topology =
        PreparedThreeWayTopology::try_from_collapsed([1, 2, 2], &[[0, 0, 0], [0, 0, 1], [0, 1, 0]])
            .unwrap();
    let ulp = 2.0_f64.powi(971);
    let pair_frame = ThreeWayWeightFrame::try_new(
        &pair_topology,
        WeightFrameInput::Tuples(&[
            f64::from_bits(f64::MAX.to_bits() - 1),
            ulp * 0.5,
            ulp * 0.53125,
        ]),
    )
    .unwrap();
    assert_eq!(pair_frame.diagonal()[0], f64::MAX);
    let pair_map = PreparedPairEdgeMap::try_new(&pair_topology, FactorPair::OneThree).unwrap();
    assert!(matches!(
        PairConductanceReplay::try_new(&pair_map, &pair_frame),
        Err(IncidenceError::InvalidWeightFrameDerivedValue {
            context: "pair weighted degree",
            index: 0,
            ..
        })
    ));
    let safe = ThreeWayWeightFrame::try_new(&pair_topology, WeightFrameInput::UnitTuples).unwrap();
    PairConductanceReplay::try_new(&pair_map, &safe).unwrap();
    assert_eq!(pair_frame.diagonal()[0], f64::MAX);
}

#[test]
fn single_extreme_weights_and_disconnected_pair_support_are_preserved() {
    let t = PreparedThreeWayTopology::try_from_collapsed([1; 3], &[[0; 3]]).unwrap();
    let a = FactorAggregation::identity([1; 3]).unwrap();
    let map = PreparedCoarseTupleMap::try_new(&t, &a).unwrap();
    for w in [f64::from_bits(1), f64::MIN_POSITIVE, 1.0, f64::MAX] {
        let f = ThreeWayWeightFrame::try_new(&t, WeightFrameInput::Tuples(&[w])).unwrap();
        let c = CoarseWeightReplay::try_new(&map, &f).unwrap();
        assert_eq!(c.frame().weights()[0].to_bits(), w.to_bits());
        for pair in FactorPair::ALL {
            let p = PreparedPairEdgeMap::try_new(&t, pair).unwrap();
            let replay = PairConductanceReplay::try_new(&p, &f).unwrap();
            assert_eq!(replay.conductances()[0].to_bits(), w.to_bits());
            assert!(replay.diagonal().iter().all(|d| d.to_bits() == w.to_bits()));
        }
    }
    let t =
        PreparedThreeWayTopology::try_from_collapsed([2, 2, 1], &[[0, 0, 0], [1, 1, 0]]).unwrap();
    assert_eq!(t.component_factor_sizes().len(), 1);
    let f = ThreeWayWeightFrame::try_new(&t, WeightFrameInput::Tuples(&[2.0, 3.0])).unwrap();
    let p = PreparedPairEdgeMap::try_new(&t, FactorPair::OneTwo).unwrap();
    let replay = PairConductanceReplay::try_new(&p, &f).unwrap();
    let mut edges = [(0, 0, 0.0); 2];
    replay.write_edges_into(&p, &f, &mut edges).unwrap();
    assert_eq!(edges, [(0, 2, 2.0), (1, 3, 3.0)]);
    assert_eq!(replay.diagonal(), &[2.0, 3.0, 2.0, 3.0]);
}

#[test]
fn replayed_pair_and_coarse_matrices_match_original_operator_algebra_on_dyadic_weights() {
    let tuples: Vec<_> = (0..3)
        .flat_map(|i| (0..4).flat_map(move |j| (0..2).map(move |k| [i, j, k])))
        .collect();
    let weights: Vec<_> = (0..tuples.len()).map(|i| 0.25 + (i % 7) as f64).collect();
    let t = PreparedThreeWayTopology::try_from_collapsed([3, 4, 2], &tuples).unwrap();
    let f = ThreeWayWeightFrame::try_new(&t, WeightFrameInput::Tuples(&weights)).unwrap();
    let fine = ThreeWayProblem::from_observations([3, 4, 2], &tuples, &weights).unwrap();
    let gramian = fine.dense_gramian();
    for pair in FactorPair::ALL {
        let map = PreparedPairEdgeMap::try_new(&t, pair).unwrap();
        let replay = PairConductanceReplay::try_new(&map, &f).unwrap();
        let (l, r) = pair.factors();
        let n = map.local_dimension();
        let indices: Vec<_> = t
            .topology()
            .factor_range(l)
            .chain(t.topology().factor_range(r))
            .collect();
        let mut matrix = vec![vec![0.0; n]; n];
        for (i, &d) in replay.diagonal().iter().enumerate() {
            matrix[i][i] = d;
        }
        for (&[i, j], &w) in map.edges().iter().zip(replay.conductances()) {
            let (i, j) = (i as usize, map.level_counts()[0] + j as usize);
            matrix[i][j] -= w;
            matrix[j][i] -= w;
        }
        for i in 0..n {
            for j in 0..n {
                let sign = if (i < map.level_counts()[0]) == (j < map.level_counts()[0]) {
                    1.0
                } else {
                    -1.0
                };
                assert_eq!(matrix[i][j], sign * gramian[indices[i]][indices[j]]);
            }
        }
    }
    let a =
        FactorAggregation::new([3, 4, 2], [vec![1, 0, 1], vec![1, 0, 1, 0], vec![0, 0]]).unwrap();
    let map = PreparedCoarseTupleMap::try_new(&t, &a).unwrap();
    let replay = CoarseWeightReplay::try_new(&map, &f).unwrap();
    let ct = map.coarse().topology();
    let coarse = ThreeWayProblem::from_observations(
        ct.level_counts(),
        ct.tuples(),
        replay.frame().weights(),
    )
    .unwrap()
    .dense_gramian();
    let parents: Vec<_> = (0..3)
        .flat_map(|factor| {
            a.parents(factor)
                .iter()
                .map(move |&p| ct.global_index(factor, p))
        })
        .collect();
    let mut galerkin = vec![vec![0.0; ct.total_levels()]; ct.total_levels()];
    for i in 0..fine.dimension() {
        for j in 0..fine.dimension() {
            galerkin[parents[i]][parents[j]] += gramian[i][j];
        }
    }
    assert_eq!(coarse, galerkin);
}
