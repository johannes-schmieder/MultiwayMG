//! Allocation and lifetime inventory of the complete prepared serial V-cycle.
use super::{GLOBAL, Result, no_events};
use multiway_incidence::{
    FactorAggregation, HierarchyWeightFrames, PreparedHierarchyTopology, PreparedThreeWayTopology,
    ThreeWayWeightFrame, WeightFrameInput,
};
use multiway_mg::PreparedMapHierarchy;
use std::hint::black_box;

pub fn run() -> Result<()> {
    let tuples: Vec<_> = (0..4)
        .flat_map(|i| (0..4).flat_map(move |j| (0..4).map(move |k| [i, j, k])))
        .collect();
    let topology = PreparedThreeWayTopology::try_from_collapsed([4; 3], &tuples)?;
    let maps = vec![
        FactorAggregation::consecutive_halving([4; 3])?,
        FactorAggregation::consecutive_halving([2; 3])?,
    ];
    let structural = PreparedHierarchyTopology::try_new(&topology, maps)?;
    let fine = ThreeWayWeightFrame::try_new(&topology, WeightFrameInput::UnitTuples)?;
    let frames = HierarchyWeightFrames::try_new(&structural, &fine)?;
    let before = GLOBAL.stats();
    let hierarchy = black_box(PreparedMapHierarchy::try_new(&frames, 1e-12)?);
    let setup = GLOBAL.stats() - before;
    let terminal = hierarchy.retained_payload_bytes()?;
    assert_eq!(setup.bytes_allocated - setup.bytes_deallocated, terminal);
    let other = PreparedMapHierarchy::try_new(&frames, 1e-12)?;
    let before = GLOBAL.stats();
    let mut scratch = hierarchy.application_workspace()?;
    let setup = GLOBAL.stats() - before;
    let retained = scratch.retained_payload_bytes()?;
    assert_eq!(setup.allocations, 31);
    assert_eq!(setup.reallocations, 0);
    assert_eq!(setup.deallocations, 0);
    assert_eq!(setup.bytes_allocated, retained);
    assert_eq!(retained, hierarchy.workspace_required_bytes()?);
    let rhs: Vec<_> = (0..12).map(|i| (i as f64 * 0.31).sin()).collect();
    let mut output = [0.0; 12];
    let report = hierarchy.payload_report(&scratch, 96)?;
    let before = GLOBAL.stats();
    for _ in 0..32 {
        hierarchy.apply_with_workspace(black_box(&rhs), &mut output, &mut scratch)?;
        assert_eq!(hierarchy.payload_report(&scratch, 96)?, report);
    }
    assert!(
        other
            .apply_with_workspace(&rhs, &mut output, &mut scratch)
            .is_err()
    );
    assert!(
        hierarchy
            .apply_with_workspace(&rhs[..11], &mut output, &mut scratch)
            .is_err()
    );
    assert!(
        hierarchy
            .apply_with_workspace(&[f64::NAN; 12], &mut output, &mut scratch)
            .is_err()
    );
    assert!(
        hierarchy
            .apply_with_workspace(&[f64::MAX; 12], &mut output, &mut scratch)
            .is_err()
    );
    hierarchy.apply_with_workspace(&rhs, &mut output, &mut scratch)?;
    no_events(GLOBAL.stats() - before);
    let before = GLOBAL.stats();
    drop(scratch);
    let released = GLOBAL.stats() - before;
    assert_eq!(released.bytes_deallocated, retained);
    assert_eq!(released.allocations, 0);
    let before = GLOBAL.stats();
    drop(hierarchy);
    let released = GLOBAL.stats() - before;
    assert_eq!(released.bytes_deallocated, terminal);
    assert_eq!(released.allocations, 0);
    println!(
        "complete prepared cycle: first/repeat32 and failure/recovery allocations=0; all 31 workspace arrays released exactly"
    );
    Ok(())
}
