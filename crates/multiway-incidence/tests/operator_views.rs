//! Explicit dense incidence algebra, generation ownership and transactional dimensions.
use multiway_incidence::{
    CoarseWeightReplay, FactorAggregation, IncidenceError, PreparedCoarseTupleMap,
    PreparedThreeWayTopology, ThreeWayProblem, ThreeWayWeightFrame, WeightFrameInput,
};

fn dense_b(counts: [usize; 3], tuples: &[[u32; 3]]) -> Vec<Vec<f64>> {
    let offsets = [0, counts[0], counts[0] + counts[1]];
    tuples
        .iter()
        .map(|tuple| {
            let mut row = vec![0.0; counts.iter().sum()];
            for f in 0..3 {
                row[offsets[f] + tuple[f] as usize] = 1.0;
            }
            row
        })
        .collect()
}
fn dot(x: &[f64], y: &[f64]) -> f64 {
    x.iter().zip(y).map(|(a, b)| a * b).sum()
}
fn transpose(b: &[Vec<f64>], y: &[f64]) -> Vec<f64> {
    (0..b[0].len())
        .map(|j| b.iter().zip(y).map(|(r, v)| r[j] * v).sum())
        .collect()
}
fn bits(x: &[f64], y: &[f64]) {
    assert_eq!(
        x.iter().map(|v| v.to_bits()).collect::<Vec<_>>(),
        y.iter().map(|v| v.to_bits()).collect::<Vec<_>>()
    );
}

#[test]
fn every_small_valid_support_matches_dense_algebra_including_extra_nullity() {
    let universe: Vec<_> = (0..2)
        .flat_map(|i| (0..2).flat_map(move |j| (0..2).map(move |k| [i, j, k])))
        .collect();
    for mask in 1..256 {
        let tuples: Vec<_> = universe
            .iter()
            .enumerate()
            .filter_map(|(i, &t)| ((mask >> i) & 1 == 1).then_some(t))
            .collect();
        if (0..3).any(|f| (0..2).any(|l| !tuples.iter().any(|t| t[f] == l))) {
            continue;
        }
        let t = PreparedThreeWayTopology::try_from_collapsed([2; 3], &tuples).unwrap();
        let weights: Vec<_> = (0..tuples.len())
            .map(|i| [0.25, 1.0, 4.0, 16.0][i % 4])
            .collect();
        let frame = ThreeWayWeightFrame::try_new(&t, WeightFrameInput::Tuples(&weights)).unwrap();
        let view = frame.operator_view();
        let owned = ThreeWayProblem::from_observations([2; 3], &tuples, &weights).unwrap();
        let b = dense_b([2; 3], &tuples);
        let x = [-0.25, 0.5, -1.0, 2.0, 4.0, -8.0];
        let y: Vec<_> = (0..tuples.len()).map(|i| i as f64 * 0.125 - 0.5).collect();
        let bx: Vec<_> = b.iter().map(|r| dot(r, &x)).collect();
        let wx: Vec<_> = bx.iter().zip(&weights).map(|(v, w)| v * w).collect();
        let mut out = vec![0.0; tuples.len()];
        let mut co = [0.0; 6];
        view.apply_incidence(&x, &mut out).unwrap();
        bits(&out, &bx);
        view.apply_adjoint(&y, &mut co).unwrap();
        bits(&co, &transpose(&b, &y));
        assert_eq!(dot(&bx, &y), dot(&x, &co));
        view.apply_weighted_incidence(&x, &mut out).unwrap();
        let sbx: Vec<_> = bx.iter().zip(&weights).map(|(v, w)| v * w.sqrt()).collect();
        bits(&out, &sbx);
        view.apply_weighted_adjoint(&y, &mut co).unwrap();
        let sy: Vec<_> = y.iter().zip(&weights).map(|(v, w)| v * w.sqrt()).collect();
        bits(&co, &transpose(&b, &sy));
        assert_eq!(dot(&sbx, &y), dot(&x, &co));
        let gx = transpose(&b, &wx);
        view.apply_gramian(&x, &mut co).unwrap();
        bits(&co, &gx);
        let mut legacy = [0.0; 6];
        owned.apply_gramian(&x, &mut legacy).unwrap();
        bits(&co, &legacy);
        view.rhs_from_targets_into(&y, &mut co).unwrap();
        let wy: Vec<_> = y.iter().zip(&weights).map(|(v, w)| v * w).collect();
        bits(&co, &transpose(&b, &wy));
        let rhs = [1.0; 6];
        view.residual_into(&rhs, &x, &mut co).unwrap();
        bits(&co, &gx.iter().map(|g| 1.0 - g).collect::<Vec<_>>());
        assert_eq!(view.energy(&x).unwrap(), dot(&bx, &wx));
        assert_eq!(view.retained_payload_bytes(), 0);
        assert!(std::ptr::eq(view.frame(), &frame));
        assert!(std::ptr::eq(view.topology(), &t));
    }
}

#[test]
fn dimensions_reject_before_writing_for_every_action() {
    let t = PreparedThreeWayTopology::try_from_collapsed([1; 3], &[[0; 3]]).unwrap();
    let f = ThreeWayWeightFrame::try_new(&t, WeightFrameInput::UnitTuples).unwrap();
    let v = f.operator_view();
    let sentinel = f64::from_bits(0x7ff8000000000345);
    let mut row = [sentinel];
    let mut col = [sentinel; 3];
    for weighted in [false, true] {
        let result = if weighted {
            v.apply_weighted_incidence(&[], &mut row)
        } else {
            v.apply_incidence(&[], &mut row)
        };
        assert!(matches!(
            result,
            Err(IncidenceError::DimensionMismatch { .. })
        ));
        bits(&row, &[sentinel]);
        let result = if weighted {
            v.apply_weighted_incidence(&[1.0; 3], &mut col)
        } else {
            v.apply_incidence(&[1.0; 3], &mut col)
        };
        assert!(result.is_err());
        bits(&col, &[sentinel; 3]);
        let result = if weighted {
            v.apply_weighted_adjoint(&[], &mut col)
        } else {
            v.apply_adjoint(&[], &mut col)
        };
        assert!(result.is_err());
        bits(&col, &[sentinel; 3]);
        let result = if weighted {
            v.apply_weighted_adjoint(&[1.0], &mut row)
        } else {
            v.apply_adjoint(&[1.0], &mut row)
        };
        assert!(result.is_err());
        bits(&row, &[sentinel]);
    }
    assert!(v.apply_gramian(&[], &mut col).is_err());
    bits(&col, &[sentinel; 3]);
    assert!(v.apply_gramian(&[1.0; 3], &mut row).is_err());
    bits(&row, &[sentinel]);
    assert!(v.rhs_from_targets_into(&[], &mut col).is_err());
    bits(&col, &[sentinel; 3]);
    assert!(v.rhs_from_targets_into(&[1.0], &mut row).is_err());
    bits(&row, &[sentinel]);
    assert!(v.residual_into(&[], &[1.0; 3], &mut col).is_err());
    bits(&col, &[sentinel; 3]);
    assert!(v.residual_into(&[1.0; 3], &[], &mut col).is_err());
    bits(&col, &[sentinel; 3]);
    assert!(v.residual_into(&[1.0; 3], &[1.0; 3], &mut row).is_err());
    bits(&row, &[sentinel]);
    assert!(v.energy(&[]).is_err());
}

#[test]
fn equal_and_changed_frames_are_distinct_while_old_views_remain_valid() {
    let rows = [[1; 3], [0; 3], [1; 3]];
    let t = PreparedThreeWayTopology::try_from_observations([2; 3], &rows).unwrap();
    let foreign = PreparedThreeWayTopology::try_from_observations([2; 3], &rows).unwrap();
    let old = ThreeWayWeightFrame::try_new(&t, WeightFrameInput::UnitObservations).unwrap();
    let equal = ThreeWayWeightFrame::try_new(&t, WeightFrameInput::Tuples(&[1.0, 2.0])).unwrap();
    let new = ThreeWayWeightFrame::try_new(&t, WeightFrameInput::UnitTuples).unwrap();
    let other = ThreeWayWeightFrame::try_new(&foreign, WeightFrameInput::UnitObservations).unwrap();
    let view = old.operator_view();
    view.validate_for(&old).unwrap();
    for frame in [&equal, &new, &other] {
        assert!(matches!(
            view.validate_for(frame),
            Err(IncidenceError::WeightFrameBindingMismatch)
        ));
    }
    std::thread::scope(|scope| {
        let a = scope.spawn(|| {
            let mut out = [0.0; 6];
            view.apply_gramian(&[1.0; 6], &mut out).unwrap();
            out
        });
        let b = scope.spawn(|| {
            let mut out = [0.0; 6];
            new.operator_view()
                .apply_gramian(&[1.0; 6], &mut out)
                .unwrap();
            out
        });
        bits(&a.join().unwrap(), &[3.0, 6.0, 3.0, 6.0, 3.0, 6.0]);
        bits(&b.join().unwrap(), &[3.0; 6]);
    });
}

#[test]
fn replayed_coarse_view_satisfies_galerkin_for_nonmonotone_factor_maps() {
    let rows: Vec<_> = (0..3)
        .flat_map(|i| (0..4).flat_map(move |j| (0..2).map(move |k| [i, j, k])))
        .collect();
    let t = PreparedThreeWayTopology::try_from_collapsed([3, 4, 2], &rows).unwrap();
    let weights: Vec<_> = (0..rows.len()).map(|i| 0.25 + (i % 7) as f64).collect();
    let f = ThreeWayWeightFrame::try_new(&t, WeightFrameInput::Tuples(&weights)).unwrap();
    let aggregation =
        FactorAggregation::new([3, 4, 2], [vec![1, 0, 1], vec![1, 0, 1, 0], vec![0, 0]]).unwrap();
    let map = PreparedCoarseTupleMap::try_new(&t, &aggregation).unwrap();
    let replay = CoarseWeightReplay::try_new(&map, &f).unwrap();
    let coarse = replay.frame().operator_view();
    let x = [0.25, -0.5, 1.0, -2.0, 4.0];
    let mut prolonged = [0.0; 9];
    aggregation.prolong(&x, &mut prolonged).unwrap();
    let mut applied = [0.0; 9];
    f.operator_view()
        .apply_gramian(&prolonged, &mut applied)
        .unwrap();
    let mut restricted = [0.0; 5];
    aggregation.restrict(&applied, &mut restricted).unwrap();
    let mut actual = [0.0; 5];
    coarse.apply_gramian(&x, &mut actual).unwrap();
    bits(&actual, &restricted);
}
