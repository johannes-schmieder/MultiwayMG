//! Complete hierarchy ownership, numerical replay and release accounting.
use super::{GLOBAL, Result, no_events};
use multiway_incidence::{
    FactorAggregation, HierarchyWeightFrames, PreparedHierarchyBudget, PreparedHierarchyTopology,
    PreparedThreeWayTopology, ThreeWayWeightFrame, WeightFrameInput,
};
use std::hint::black_box;

pub fn run() -> Result<()> {
    let tuples: Vec<_> = (0..4)
        .flat_map(|a| (0..4).flat_map(move |b| (0..4).map(move |c| [a, b, c])))
        .collect();
    let topology = PreparedThreeWayTopology::try_from_collapsed([4; 3], &tuples)?;
    let mut maps = Vec::with_capacity(4);
    maps.push(FactorAggregation::consecutive_halving([4; 3])?);
    maps.push(FactorAggregation::consecutive_halving([2; 3])?);
    let input_bytes = maps.capacity() * std::mem::size_of::<FactorAggregation>()
        + maps
            .iter()
            .map(|a| a.retained_payload_bytes().unwrap())
            .sum::<usize>();
    let before = GLOBAL.stats();
    let hierarchy = black_box(PreparedHierarchyTopology::try_new(&topology, maps)?);
    let setup = GLOBAL.stats() - before;
    assert_eq!(setup.allocations, 21);
    assert_eq!(setup.reallocations, 0);
    let structural = hierarchy.retained_payload_bytes()?;
    assert_eq!(
        setup.bytes_allocated - setup.bytes_deallocated + input_bytes,
        structural
    );
    assert!(
        hierarchy.setup_peak_payload_bound() >= topology.retained_payload_bytes()? + structural
    );
    let fine = ThreeWayWeightFrame::try_new(&topology, WeightFrameInput::UnitTuples)?;
    let old = HierarchyWeightFrames::try_new(&hierarchy, &fine)?;
    let bound = HierarchyWeightFrames::setup_payload_bound(
        &hierarchy,
        &fine,
        old.retained_payload_bytes()?,
    )?;
    let budget = |maximum_payload_bytes| PreparedHierarchyBudget {
        maximum_payload_bytes,
        additional_live_payload_bytes: old.retained_payload_bytes().unwrap(),
    };
    let before = GLOBAL.stats();
    assert!(
        HierarchyWeightFrames::try_new_with_budget(&hierarchy, &fine, budget(bound - 1)).is_err()
    );
    no_events(GLOBAL.stats() - before);
    let before = GLOBAL.stats();
    let replay = black_box(HierarchyWeightFrames::try_new_with_budget(
        &hierarchy,
        &fine,
        budget(bound),
    )?);
    let setup = GLOBAL.stats() - before;
    let numerical = replay.retained_payload_bytes()?;
    assert_eq!(setup.allocations, 11);
    assert_eq!(setup.deallocations, 2);
    assert_eq!(setup.reallocations, 0);
    assert_eq!(setup.bytes_allocated - setup.bytes_deallocated, numerical);
    let mut weights = vec![0.0; 64];
    let mut output = [0.0; 12];
    let values = [1.0; 12];
    let before = GLOBAL.stats();
    for _ in 0..32 {
        replay.validate_for(&hierarchy, &fine)?;
        assert_eq!(replay.retained_payload_bytes()?, numerical);
        assert_eq!(hierarchy.retained_payload_bytes()?, structural);
        for i in 0..replay.level_count() {
            let view = replay.frame(i).unwrap().operator_view();
            view.apply_gramian(&values[..view.dimension()], &mut output[..view.dimension()])?;
        }
    }
    no_events(GLOBAL.stats() - before);
    for step in 0..16 {
        for (i, w) in weights.iter_mut().enumerate() {
            *w = 1.0 + (i + step) as f64;
        }
        let before = GLOBAL.stats();
        let new_fine = ThreeWayWeightFrame::try_new(&topology, WeightFrameInput::Tuples(&weights))?;
        let new = HierarchyWeightFrames::try_new(&hierarchy, &new_fine)?;
        assert!(old.validate_for(&hierarchy, &new_fine).is_err());
        drop(new);
        drop(new_fine);
        let cycle = GLOBAL.stats() - before;
        assert_eq!(cycle.bytes_allocated, cycle.bytes_deallocated);
        assert_eq!(cycle.reallocations, 0);
    }
    let before = GLOBAL.stats();
    drop(replay);
    let released = GLOBAL.stats() - before;
    assert_eq!(released.bytes_deallocated, numerical);
    assert_eq!(released.allocations, 0);
    drop(old);
    let before = GLOBAL.stats();
    drop(hierarchy);
    let released = GLOBAL.stats() - before;
    assert_eq!(released.bytes_deallocated, structural);
    assert_eq!(released.allocations, 0);
    println!(
        "complete hierarchy: exact structural/numerical release, changed-frame16 balanced, first/repeat32 actions allocation-free"
    );
    Ok(())
}
