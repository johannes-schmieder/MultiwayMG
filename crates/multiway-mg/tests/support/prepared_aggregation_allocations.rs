//! Bounded candidate setup, failure release and absence of hidden sort allocations.
use super::{GLOBAL, Result, no_events};
use multiway_incidence::{
    PreparedHierarchyBudget, PreparedThreeWayTopology, ThreeWayWeightFrame, WeightFrameInput,
};
use multiway_mg::{PairNeighborhoodAggregationOptions, PreparedPairNeighborhoodCandidate};
pub fn run() -> Result<()> {
    for adjacent in [false, true] {
        check(adjacent)?;
    }
    Ok(())
}
fn check(adjacent: bool) -> Result<()> {
    let tuples: Vec<_> = (0..4)
        .flat_map(|i| (0..3).flat_map(move |j| (0..2).map(move |k| [i, j, k])))
        .collect();
    let t = PreparedThreeWayTopology::try_from_collapsed([4, 3, 2], &tuples)?;
    let f = ThreeWayWeightFrame::try_new(&t, WeightFrameInput::UnitTuples)?;
    let options = PairNeighborhoodAggregationOptions::default();
    let report = if adjacent {
        PreparedPairNeighborhoodCandidate::adjacent_pairs_setup_payload_report(
            &f,
            options.minimum_affinity,
            PreparedHierarchyBudget::UNLIMITED,
        )?
    } else {
        PreparedPairNeighborhoodCandidate::setup_payload_report(
            &f,
            options,
            PreparedHierarchyBudget::UNLIMITED,
        )?
    };
    let construct = |frame, budget| {
        if adjacent {
            PreparedPairNeighborhoodCandidate::try_adjacent_pairs(
                frame,
                options.minimum_affinity,
                budget,
            )
        } else {
            PreparedPairNeighborhoodCandidate::try_new(frame, options, budget)
        }
    };
    let before = GLOBAL.stats();
    let bad = construct(
        &f,
        PreparedHierarchyBudget {
            maximum_payload_bytes: report.total_payload_bound - 1,
            additional_live_payload_bytes: 0,
        },
    )
    .unwrap_err();
    assert_eq!(bad.work, Default::default());
    drop(bad);
    no_events(GLOBAL.stats() - before);
    let before = GLOBAL.stats();
    let candidate = construct(&f, PreparedHierarchyBudget::UNLIMITED)?;
    let setup = GLOBAL.stats() - before;
    let retained = candidate.retained_payload_bytes()?;
    assert_eq!(setup.allocations, 10);
    assert_eq!(setup.deallocations, 7);
    assert_eq!(setup.reallocations, 0);
    assert_eq!(setup.bytes_allocated - setup.bytes_deallocated, retained);
    assert_eq!(retained, 4 * 9);
    let seen = candidate
        .aggregation()
        .coarse_counts()
        .iter()
        .sum::<usize>();
    assert_eq!(
        setup.bytes_allocated,
        report.parent_payload_bytes + report.scratch_payload_bound + seen
    );
    let before = GLOBAL.stats();
    drop(candidate);
    let released = GLOBAL.stats() - before;
    assert_eq!(released.deallocations, 3);
    assert_eq!(released.bytes_deallocated, retained);
    assert_eq!(released.allocations, 0);
    let t = PreparedThreeWayTopology::try_from_collapsed(
        [2; 3],
        &[[0, 0, 0], [0, 1, 1], [1, 0, 1], [1, 1, 0]],
    )?;
    let f = ThreeWayWeightFrame::try_new(&t, WeightFrameInput::Tuples(&[f64::MAX * 0.375; 4]))?;
    let before = GLOBAL.stats();
    let bad = construct(&f, PreparedHierarchyBudget::UNLIMITED).unwrap_err();
    assert_eq!(bad.work.tuple_visits, 8);
    drop(bad);
    let failed = GLOBAL.stats() - before;
    assert_eq!(failed.allocations, 7);
    assert_eq!(failed.deallocations, 7);
    assert_eq!(failed.reallocations, 0);
    assert_eq!(failed.bytes_allocated, failed.bytes_deallocated);
    println!(
        "bounded flat candidates (adjacent={adjacent}): 10 setup arrays, three retained parent arrays; no hidden sort/reallocation; budget rejection allocates zero and numerical failure releases all arrays"
    );
    Ok(())
}
