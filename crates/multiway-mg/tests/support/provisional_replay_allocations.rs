use super::{GLOBAL, Result, no_events};
use multiway_incidence::{
    FactorAggregation, PreparedHierarchyBudget, PreparedHierarchyBuilder, PreparedHierarchyLimits,
    PreparedProvisionalFrame, PreparedThreeWayTopology, ProvisionalFrameStage,
    ProvisionalWeightInput, ThreeWayWeightFrame, WeightFrameInput,
};
use std::hint::black_box;
const B: PreparedHierarchyBudget = PreparedHierarchyBudget::UNLIMITED;
fn builder(
    t: &PreparedThreeWayTopology,
    map: FactorAggregation,
) -> Result<PreparedHierarchyBuilder<'_>> {
    let mut b = PreparedHierarchyBuilder::try_new(
        t,
        PreparedHierarchyLimits {
            maximum_transitions: 2,
            maximum_total_tuples: usize::MAX,
            maximum_total_coefficients: usize::MAX,
            require_strict_dimension_reduction: true,
        },
        B,
    )?;
    b.try_append(map, B)?;
    Ok(b)
}
pub fn run() -> Result<()> {
    let keys: Vec<_> = (0..4)
        .flat_map(|a| (0..4).flat_map(move |b| (0..4).map(move |c| [a, b, c])))
        .collect();
    let t = PreparedThreeWayTopology::try_from_collapsed([4; 3], &keys)?;
    let b = builder(&t, FactorAggregation::consecutive_halving([4; 3])?)?;
    let f = ThreeWayWeightFrame::try_new(&t, WeightFrameInput::UnitTuples)?;
    for owned in [false, true] {
        let input = if owned {
            ProvisionalWeightInput::Owned(f.weights().to_vec())
        } else {
            ProvisionalWeightInput::Frame(&f)
        };
        let input_bytes = if owned { 64 * 8 } else { 0 };
        let before = GLOBAL.stats();
        let p = black_box(PreparedProvisionalFrame::try_replay_last(&b, input, B)?);
        let delta = GLOBAL.stats() - before;
        let retained = p.retained_payload_bytes()?;
        assert_eq!(delta.allocations, 5);
        assert_eq!(delta.deallocations, 1 + usize::from(owned));
        assert_eq!(delta.reallocations, 0);
        assert_eq!(
            delta.bytes_allocated + input_bytes - delta.bytes_deallocated,
            retained
        );
        let ptr = p.frame().weights().as_ptr();
        let before = GLOBAL.stats();
        let weights = black_box(p.into_tuple_weights());
        let delta = GLOBAL.stats() - before;
        assert_eq!(delta.allocations, 0);
        assert_eq!(delta.reallocations, 0);
        assert_eq!(delta.deallocations, 3);
        assert_eq!(delta.bytes_deallocated, retained - weights.capacity() * 8);
        assert_eq!(weights.as_ptr(), ptr);
        assert_eq!(weights, [8.; 8]);
        let bytes = weights.capacity() * 8;
        let before = GLOBAL.stats();
        drop(weights);
        let delta = GLOBAL.stats() - before;
        assert_eq!(delta.allocations, 0);
        assert_eq!(delta.deallocations, 1);
        assert_eq!(delta.bytes_deallocated, bytes);
        let input = if owned {
            ProvisionalWeightInput::Owned(f.weights().to_vec())
        } else {
            ProvisionalWeightInput::Frame(&f)
        };
        let setup = PreparedProvisionalFrame::setup_payload_report(&b, &input, B)?;
        let before = GLOBAL.stats();
        let e = PreparedProvisionalFrame::try_replay_last(
            &b,
            input,
            PreparedHierarchyBudget {
                maximum_payload_bytes: setup.total_payload_bound - 1,
                ..B
            },
        )
        .unwrap_err();
        let delta = GLOBAL.stats() - before;
        assert_eq!(e.stage, ProvisionalFrameStage::Admission);
        assert_eq!(delta.allocations, 0);
        assert_eq!(delta.reallocations, 0);
        assert_eq!(delta.deallocations, usize::from(owned));
        assert_eq!(delta.bytes_deallocated, input_bytes);
        if !owned {
            no_events(delta);
        }
    }
    let mut weights = vec![1.; 64];
    weights[9] = f64::NAN;
    let before = GLOBAL.stats();
    let e =
        PreparedProvisionalFrame::try_replay_last(&b, ProvisionalWeightInput::Owned(weights), B)
            .unwrap_err();
    let delta = GLOBAL.stats() - before;
    assert_eq!(e.stage, ProvisionalFrameStage::InputValidation);
    assert_eq!(delta.allocations, 0);
    assert_eq!(delta.deallocations, 1);
    assert_eq!(delta.bytes_deallocated, 512);
    let t = PreparedThreeWayTopology::try_from_collapsed(
        [2; 3],
        &[[0, 0, 0], [0, 1, 1], [1, 0, 1], [1, 1, 0]],
    )?;
    let f = ThreeWayWeightFrame::try_new(&t, WeightFrameInput::Tuples(&[f64::MAX * 0.3; 4]))?;
    for (map, stage, allocations) in [
        (
            FactorAggregation::consecutive_halving([2; 3])?,
            ProvisionalFrameStage::Reduction,
            1,
        ),
        (
            FactorAggregation::new([2; 3], [vec![0, 0], vec![0, 1], vec![0, 1]])?,
            ProvisionalFrameStage::Finishing,
            4,
        ),
    ] {
        let b = builder(&t, map)?;
        for owned in [false, true] {
            let input = if owned {
                ProvisionalWeightInput::Owned(f.weights().to_vec())
            } else {
                ProvisionalWeightInput::Frame(&f)
            };
            let before = GLOBAL.stats();
            let e = PreparedProvisionalFrame::try_replay_last(&b, input, B).unwrap_err();
            let delta = GLOBAL.stats() - before;
            assert_eq!(e.stage, stage);
            assert_eq!(delta.allocations, allocations);
            assert_eq!(delta.deallocations, allocations + usize::from(owned));
            assert_eq!(delta.reallocations, 0);
            assert_eq!(
                delta.bytes_deallocated,
                delta.bytes_allocated + if owned { 32 } else { 0 }
            );
        }
    }
    println!(
        "provisional replay: five allocations, predecessor freed before finishing; consume reuses weights and releases three arrays; admission/invalid input allocate zero; reduction/degree overflow release all temporary payload"
    );
    Ok(())
}
