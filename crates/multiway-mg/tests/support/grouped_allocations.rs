//! Exact optional-grouping storage, construction cursor and first application costs.
use super::{GLOBAL, Result, no_events};
use multiway_incidence::{
    PreparedThreeWayTopology, PreparedTupleGrouping, ThreeWayWeightFrame, WeightFrameInput,
};
use multiway_mg::PreparedSymmetricMap;
use std::hint::black_box;

pub fn run() -> Result<()> {
    let tuples: Vec<_> = (0..4)
        .flat_map(|i| (0..3).flat_map(move |j| (0..2).map(move |k| [i, j, k])))
        .collect();
    let t = PreparedThreeWayTopology::try_from_collapsed([4, 3, 2], &tuples)?;
    let foreign = PreparedThreeWayTopology::try_from_collapsed([4, 3, 2], &tuples)?;
    let f = ThreeWayWeightFrame::try_new(&t, WeightFrameInput::UnitTuples)?;
    let other = ThreeWayWeightFrame::try_new(&foreign, WeightFrameInput::UnitTuples)?;
    let requested = PreparedTupleGrouping::required_payload_bytes(&t)?;
    let bound = PreparedTupleGrouping::setup_payload_bound(&t, 17)?;
    assert_eq!(requested, 8 * (9 + 3) + 8 * 24);
    assert_eq!(bound, t.retained_payload_bytes()? + requested + 8 * 3 + 17);
    let before = GLOBAL.stats();
    assert!(PreparedTupleGrouping::try_new_with_budget(&t, bound - 1, 17).is_err());
    no_events(GLOBAL.stats() - before);
    let before = GLOBAL.stats();
    let groups = PreparedTupleGrouping::try_new_with_budget(&t, bound, 17)?;
    let built = GLOBAL.stats() - before;
    let retained = groups.retained_payload_bytes()?;
    assert_eq!(retained, requested);
    assert_eq!(built.allocations, 3);
    assert_eq!(built.reallocations, 0);
    assert_eq!(built.deallocations, 1);
    assert_eq!(built.bytes_allocated, retained + 8 * 3);
    assert_eq!(built.bytes_deallocated, 8 * 3);
    let before = GLOBAL.stats();
    let view = black_box(f.operator_view().with_grouping(&groups)?);
    let map = black_box(PreparedSymmetricMap::new(&f).with_grouping(&groups)?);
    assert!(other.operator_view().with_grouping(&groups).is_err());
    assert!(
        PreparedSymmetricMap::new(&other)
            .with_grouping(&groups)
            .is_err()
    );
    no_events(GLOBAL.stats() - before);
    let before = GLOBAL.stats();
    let mut scratch = map.application_workspace()?;
    let built = GLOBAL.stats() - before;
    let workspace_bytes = scratch.retained_payload_bytes()?;
    assert_eq!(built.allocations, 4);
    assert_eq!(built.reallocations, 0);
    assert_eq!(built.deallocations, 0);
    assert_eq!(built.bytes_allocated, workspace_bytes);
    assert_eq!(map.workspace_required_bytes()?, workspace_bytes);
    let x = [0.5, -1.0, 2.0, -4.0, 8.0, 0.25, -0.125, 1.5, -3.5];
    let y = [0.25; 24];
    let mut co = [0.0; 9];
    let mut image = [f64::NAN; 24];
    let before = GLOBAL.stats();
    for _ in 0..32 {
        view.apply_incidence(black_box(&x), &mut image)?;
        view.apply_weighted_incidence(&x, &mut image)?;
        view.apply_adjoint(&y, &mut co)?;
        view.apply_weighted_adjoint(&y, &mut co)?;
        view.rhs_from_targets_into(&y, &mut co)?;
        view.apply_gramian(&x, &mut co)?;
        let gramian = co;
        view.apply_gramian_with_image(&x, &mut co, &mut image)?;
        for i in 0..9 {
            assert_eq!(co[i].to_bits(), gramian[i].to_bits());
        }
        view.residual_into(&x, &x, &mut co)?;
        black_box(view.energy(&x)?);
        map.apply_with_workspace(&x, &mut co, &mut scratch)?;
        assert!(co.iter().all(|x| x.is_finite()));
    }
    assert!(
        view.apply_gramian_with_image(&x, &mut co, &mut image[..23])
            .is_err()
    );
    assert!(
        map.apply_with_workspace(&x[..8], &mut co, &mut scratch)
            .is_err()
    );
    assert!(
        map.apply_with_workspace(&[f64::NAN; 9], &mut co, &mut scratch)
            .is_err()
    );
    assert!(
        map.apply_with_workspace(&[f64::MAX; 9], &mut co, &mut scratch)
            .is_err()
    );
    map.apply_with_workspace(&x, &mut co, &mut scratch)?;
    no_events(GLOBAL.stats() - before);
    let before = GLOBAL.stats();
    drop(scratch);
    let released = GLOBAL.stats() - before;
    assert_eq!(released.allocations, 0);
    assert_eq!(released.deallocations, 4);
    assert_eq!(released.bytes_deallocated, workspace_bytes);
    let before = GLOBAL.stats();
    drop(groups);
    let released = GLOBAL.stats() - before;
    assert_eq!(released.allocations, 0);
    assert_eq!(released.deallocations, 2);
    assert_eq!(released.bytes_deallocated, retained);
    println!(
        "grouped tuple kernels/MAP: two retained grouping arrays plus one released cursor; first/repeat32/errors allocate zero, all payload released exactly"
    );
    Ok(())
}
