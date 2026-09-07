//! Flat proposals preserve finite legacy maps and are not numerical hierarchies.
use multiway_incidence::{
    HierarchyWeightFrames, PreparedHierarchyBudget, PreparedHierarchyTopology,
    PreparedThreeWayTopology, ThreeWayWeightFrame, WeightFrameInput,
};
use multiway_mg::{
    MultiwayError, PairNeighborhoodAggregationOptions, PreparedPairNeighborhoodCandidate,
    ThreeWayProblem, build_pair_neighborhood_aggregation,
};

fn compare(counts: [usize; 3], mut tuples: Vec<[u32; 3]>) {
    tuples.sort_unstable();
    tuples.dedup();
    let t = PreparedThreeWayTopology::try_from_collapsed(counts, &tuples).unwrap();
    for weights in [
        vec![1.0; tuples.len()],
        (0..tuples.len())
            .map(|i| 1.0 + (i % 11) as f64 / 10.0)
            .collect(),
        (0..tuples.len())
            .map(|i| 2.0f64.powi((i % 17) as i32 - 8))
            .collect(),
    ] {
        let f = ThreeWayWeightFrame::try_new(&t, WeightFrameInput::Tuples(&weights)).unwrap();
        let equal = ThreeWayWeightFrame::try_new(&t, WeightFrameInput::Tuples(&weights)).unwrap();
        let ordinary = ThreeWayProblem::from_observations(counts, &tuples, &weights).unwrap();
        for k in [2, 4, 16] {
            for threshold in [0.0, 0.02, 0.15, 1.0] {
                let options = PairNeighborhoodAggregationOptions {
                    minimum_affinity: threshold,
                    maximum_neighbor_degree: k,
                };
                let expected = build_pair_neighborhood_aggregation(&ordinary, options).unwrap();
                let budget = PreparedHierarchyBudget {
                    maximum_payload_bytes: usize::MAX,
                    additional_live_payload_bytes: 123,
                };
                let a = PreparedPairNeighborhoodCandidate::try_new(&f, options, budget).unwrap();
                let b = PreparedPairNeighborhoodCandidate::try_new(&f, options, budget).unwrap();
                assert_eq!(a.aggregation(), &expected);
                assert_eq!(a.aggregation(), b.aggregation());
                assert_eq!(a.work_report(), b.work_report());
                assert_eq!(a.work_report().tuple_visits, 6 * tuples.len());
                assert_eq!(
                    a.retained_payload_bytes().unwrap(),
                    4 * counts.iter().sum::<usize>()
                );
                assert!(a.validate_for(&f).is_ok());
                assert!(a.validate_for(&equal).is_err());
                let required = a.setup_report().total_payload_bound;
                let rejected = PreparedPairNeighborhoodCandidate::try_new(
                    &f,
                    options,
                    PreparedHierarchyBudget {
                        maximum_payload_bytes: required - 1,
                        ..budget
                    },
                )
                .unwrap_err();
                assert!(
                    matches!(rejected.source,MultiwayError::PayloadBudgetExceeded{required:r,budget:b} if r==required && b==required-1)
                );
                assert_eq!(rejected.work, Default::default());
                assert_eq!(
                    rejected.setup_payload_bound,
                    Some(a.setup_report().total_payload_bound)
                );
                assert!(
                    PreparedPairNeighborhoodCandidate::try_new(
                        &f,
                        options,
                        PreparedHierarchyBudget {
                            maximum_payload_bytes: required,
                            ..budget
                        }
                    )
                    .is_ok()
                );
                // All original tuples and positive weights remain represented by exact Galerkin coarsening.
                let h = PreparedHierarchyTopology::try_new(&t, vec![a.into_aggregation()]).unwrap();
                let frames = HierarchyWeightFrames::try_new(&h, &f).unwrap();
                let coarse = expected.coarsen(&ordinary).unwrap();
                assert_eq!(
                    h.level(1).unwrap().topology().tuples(),
                    coarse.topology().tuples()
                );
                assert_eq!(frames.frame(1).unwrap().weights(), coarse.weights());
                assert_eq!(f.weights(), weights);
            }
        }
    }
}
#[test]
fn finite_legacy_maps_match_on_ragged_disconnected_hubs_and_extra_nullity() {
    let mut disconnected = Vec::new();
    for i in 0..3 {
        for j in 0..2 {
            for k in 0..4 {
                disconnected.push([i, j, k]);
            }
        }
    }
    for i in 0..2 {
        for j in 0..3 {
            for k in 0..2 {
                disconnected.push([i + 3, j + 2, k + 4]);
            }
        }
    }
    compare([5, 5, 6], disconnected);
    compare(
        [12, 3, 2],
        (0..12)
            .flat_map(|i| (0..3).flat_map(move |j| (0..2).map(move |k| [i, j, k])))
            .collect(),
    );
    compare(
        [8; 3],
        (0..8)
            .flat_map(|i| (0..8).map(move |j| [i, j, (i + j) % 8]))
            .collect(),
    );
    compare(
        [4, 3, 3],
        (0..4)
            .flat_map(|i| (0..3).map(move |j| [i, j, j]))
            .collect(),
    );
    compare([1; 3], vec![[0; 3]]);
}
#[test]
fn invalid_options_and_unrepresentable_overlap_return_charged_failure() {
    let t = PreparedThreeWayTopology::try_from_collapsed(
        [2; 3],
        &[[0, 0, 0], [0, 1, 1], [1, 0, 1], [1, 1, 0]],
    )
    .unwrap();
    let weights = [f64::MAX * 0.375; 4];
    let f = ThreeWayWeightFrame::try_new(&t, WeightFrameInput::Tuples(&weights)).unwrap();
    let bad = PreparedPairNeighborhoodCandidate::try_new(
        &f,
        Default::default(),
        PreparedHierarchyBudget::UNLIMITED,
    )
    .unwrap_err();
    assert!(matches!(
        bad.source,
        MultiwayError::NumericalFailure {
            context: "candidate overlap"
        }
    ));
    assert_eq!(bad.work.tuple_visits, 8);
    assert_eq!(bad.work.proposals, 4);
    assert_eq!(bad.work.unique_candidates, 1);
    assert!(bad.setup_payload_bound.is_some());
    assert_eq!(f.weights(), weights);
    for (affinity, k) in [
        (f64::NAN, 4),
        (f64::INFINITY, 4),
        (-0.1, 4),
        (1.1, 4),
        (0.02, 1),
    ] {
        let bad = PreparedPairNeighborhoodCandidate::try_new(
            &f,
            PairNeighborhoodAggregationOptions {
                minimum_affinity: affinity,
                maximum_neighbor_degree: k,
            },
            PreparedHierarchyBudget::UNLIMITED,
        )
        .unwrap_err();
        assert!(matches!(bad.source, MultiwayError::InvalidOption { .. }));
        assert_eq!(bad.work, Default::default());
        assert!(bad.setup_payload_bound.is_none());
    }
    let ordinary = ThreeWayWeightFrame::try_new(&t, WeightFrameInput::UnitTuples).unwrap();
    assert!(
        PreparedPairNeighborhoodCandidate::try_new(
            &ordinary,
            Default::default(),
            PreparedHierarchyBudget::UNLIMITED
        )
        .is_ok()
    );
}

#[test]
fn expanded_sparse_candidate_matches_legacy_with_bounded_pre_dense_setup() {
    let n = 4096u32;
    let mut tuples: Vec<_> = (0..n)
        .flat_map(|i| (0..16).map(move |j| [i, (i + j) % n, (i + 17 * j) % n]))
        .collect();
    tuples.sort_unstable();
    tuples.dedup();
    assert_eq!(tuples.len(), 65536);
    let weights: Vec<_> = (0..tuples.len())
        .map(|i| 2.0f64.powi((i % 13) as i32 - 6))
        .collect();
    let t = PreparedThreeWayTopology::try_from_collapsed([n as usize; 3], &tuples).unwrap();
    let f = ThreeWayWeightFrame::try_new(&t, WeightFrameInput::Tuples(&weights)).unwrap();
    let options = PairNeighborhoodAggregationOptions {
        minimum_affinity: 0.02,
        maximum_neighbor_degree: 4,
    };
    let report = PreparedPairNeighborhoodCandidate::setup_payload_report(
        &f,
        options,
        PreparedHierarchyBudget::UNLIMITED,
    )
    .unwrap();
    let candidate = PreparedPairNeighborhoodCandidate::try_new(
        &f,
        options,
        PreparedHierarchyBudget {
            maximum_payload_bytes: report.total_payload_bound,
            additional_live_payload_bytes: 0,
        },
    )
    .unwrap();
    assert_eq!(report.maximum_proposals, 49152);
    assert_eq!(candidate.work_report().tuple_visits, 6 * 65536);
    assert!(candidate.work_report().proposals <= 3 * report.maximum_proposals);
    let ordinary = ThreeWayProblem::from_observations([n as usize; 3], &tuples, &weights).unwrap();
    assert_eq!(
        candidate.aggregation(),
        &build_pair_neighborhood_aggregation(&ordinary, options).unwrap()
    );
}
