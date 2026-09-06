//! Real prepared projection/MAP first/repeated calls and rejection accounting.
use super::{GLOBAL, Result, no_events};
use multiway_incidence::{PreparedThreeWayTopology, ThreeWayWeightFrame, WeightFrameInput};
use multiway_mg::PreparedSymmetricMap;
use std::hint::black_box;

pub fn run() -> Result<()> {
    let tuples: Vec<_> = (0..4)
        .flat_map(|i| (0..3).flat_map(move |j| (0..2).map(move |k| [i, j, k])))
        .collect();
    let topology = PreparedThreeWayTopology::try_from_collapsed([4, 3, 2], &tuples)?;
    let fine = ThreeWayWeightFrame::try_new(&topology, WeightFrameInput::UnitTuples)?;
    let equal = ThreeWayWeightFrame::try_new(&topology, WeightFrameInput::UnitTuples)?;
    let before = GLOBAL.stats();
    let map = black_box(PreparedSymmetricMap::new(&fine));
    no_events(GLOBAL.stats() - before);
    let before = GLOBAL.stats();
    let mut scratch = map.application_workspace()?;
    let setup = GLOBAL.stats() - before;
    let retained = scratch.retained_payload_bytes()?;
    assert_eq!(setup.allocations, 5);
    assert_eq!(setup.reallocations, 0);
    assert_eq!(setup.deallocations, 0);
    assert_eq!(setup.bytes_allocated, retained);
    assert_eq!(map.workspace_required_bytes()?, retained);
    let mut out = [0.0; 9];
    let rhs = [1.0; 9];
    let mut projection = topology.try_projection_workspace()?;
    let before = GLOBAL.stats();
    for _ in 0..32 {
        map.apply_with_workspace(black_box(&rhs), &mut out, &mut scratch)?;
        topology.project_structural_range_with_workspace(&mut out, &mut projection)?;
        topology.maximum_structural_defect_with_workspace(&out, &mut projection)?;
        assert!(out.iter().all(|x| x.is_finite()));
    }
    assert!(
        PreparedSymmetricMap::new(&equal)
            .apply_with_workspace(&rhs, &mut out, &mut scratch)
            .is_err()
    );
    assert!(
        map.apply_with_workspace(&rhs[..8], &mut out, &mut scratch)
            .is_err()
    );
    assert!(
        map.apply_with_workspace(&rhs, &mut out[..8], &mut scratch)
            .is_err()
    );
    assert!(
        map.apply_with_workspace(&[f64::NAN; 9], &mut out, &mut scratch)
            .is_err()
    );
    // Finite values can overflow the unscaled ordinary projection; rejection
    // must remain allocation-free and the same scratch must recover.
    assert!(
        map.apply_with_workspace(&[f64::MAX; 9], &mut out, &mut scratch)
            .is_err()
    );
    map.apply_with_workspace(&rhs, &mut out, &mut scratch)?;
    no_events(GLOBAL.stats() - before);
    let before = GLOBAL.stats();
    drop(scratch);
    let released = GLOBAL.stats() - before;
    assert_eq!(released.bytes_deallocated, retained);
    assert_eq!(released.allocations, 0);
    println!(
        "prepared MAP: first/repeat32 and static/numerical failures allocate zero; five setup arrays released exactly"
    );
    Ok(())
}
