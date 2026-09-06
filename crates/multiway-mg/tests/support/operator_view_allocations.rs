//! First/repeated operator actions and rejected calls must allocate nothing.
use super::{GLOBAL, Result, equal_bits, no_events};
use multiway_incidence::{PreparedThreeWayTopology, ThreeWayWeightFrame, WeightFrameInput};
use std::hint::black_box;

pub fn run() -> Result<()> {
    let rows = [[0, 0, 0], [0, 1, 1], [1, 0, 1], [1, 1, 0]];
    let topology = PreparedThreeWayTopology::try_from_collapsed([2; 3], &rows)?;
    let old = ThreeWayWeightFrame::try_new(&topology, WeightFrameInput::UnitTuples)?;
    let new =
        ThreeWayWeightFrame::try_new(&topology, WeightFrameInput::Tuples(&[0.25, 1.0, 4.0, 16.0]))?;
    let x = [1.0, -2.0, 3.0, -4.0, 5.0, -6.0];
    let y = [1.0, -2.0, 3.0, -4.0];
    let rhs = [1.0; 6];
    let mut row = [0.0; 4];
    let mut col = [0.0; 6];
    let before = GLOBAL.stats();
    let view = black_box(new.operator_view());
    for _ in 0..64 {
        view.apply_incidence(black_box(&x), &mut row)?;
        view.apply_adjoint(black_box(&y), &mut col)?;
        view.apply_weighted_incidence(black_box(&x), &mut row)?;
        view.apply_weighted_adjoint(black_box(&y), &mut col)?;
        view.apply_gramian(black_box(&x), &mut col)?;
        view.rhs_from_targets_into(black_box(&y), &mut col)?;
        view.residual_into(black_box(&rhs), &x, &mut col)?;
        black_box(view.energy(black_box(&x))?);
        view.validate_for(&new)?;
        black_box(view.binding());
        assert_eq!(view.retained_payload_bytes(), 0);
    }
    no_events(GLOBAL.stats() - before);
    let snapshot = col;
    let before = GLOBAL.stats();
    assert!(view.validate_for(&old).is_err());
    assert!(view.apply_incidence(&[], &mut row).is_err());
    assert!(view.apply_adjoint(&[], &mut col).is_err());
    assert!(view.apply_weighted_incidence(&[], &mut row).is_err());
    assert!(view.apply_weighted_adjoint(&[], &mut col).is_err());
    assert!(view.apply_gramian(&[], &mut col).is_err());
    assert!(view.rhs_from_targets_into(&[], &mut col).is_err());
    assert!(view.residual_into(&rhs, &[], &mut col).is_err());
    assert!(view.energy(&[]).is_err());
    no_events(GLOBAL.stats() - before);
    equal_bits(&col, &snapshot);
    println!(
        "frame-operator views: construction/first/repeat64/rejection allocations=0; exclusive heap=0"
    );
    Ok(())
}
