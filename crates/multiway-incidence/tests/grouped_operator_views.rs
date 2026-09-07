//! Explicit dense incidence algebra, generation ownership and transactional dimensions.
use multiway_incidence::{
    CoarseWeightReplay, FactorAggregation, IncidenceError, PreparedCoarseTupleMap,
    PreparedThreeWayTopology, PreparedTupleGrouping, ThreeWayProblem, ThreeWayWeightFrame,
    WeightFrameInput,
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
fn grouped_rows_match_dense_algebra_for_every_small_valid_support() {
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
        let groups = PreparedTupleGrouping::try_new(&t).unwrap();
        let view = frame.operator_view().with_grouping(&groups).unwrap();
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
        let mut image = vec![f64::NAN; tuples.len()];
        view.apply_gramian_with_image(&x, &mut co, &mut image)
            .unwrap();
        bits(&co, &gx);
        bits(&image, &wx);
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
        assert!(std::ptr::eq(view.original().frame(), &frame));
        assert!(std::ptr::eq(view.original().topology(), &t));
    }
}

#[test]
fn stable_gathers_preserve_extreme_bits_and_reuse_groups_across_weight_frames() {
    let mut cases = vec![([2, 3, 4], vec![[0, 0, 0], [0, 1, 1], [0, 2, 3], [1, 2, 2]])];
    // A skewed row plus disconnected support exercises long stable permutations.
    let mut skew: Vec<_> = (0..32).map(|j| [0, j, j % 7]).collect();
    skew.extend((0..32).map(|j| [1, j, (j + 3) % 7]));
    skew.push([2, 32, 7]);
    cases.push(([3, 33, 8], skew));
    for (counts, tuples) in cases {
        let t = PreparedThreeWayTopology::try_from_collapsed(counts, &tuples).unwrap();
        let groups = PreparedTupleGrouping::try_new(&t).unwrap();
        for shift in [0, 7, 13] {
            let weights: Vec<_> = (0..tuples.len())
                .map(|i| 2.0_f64.powi(((i * 11 + shift) % 61) as i32 - 30))
                .collect();
            let frame =
                ThreeWayWeightFrame::try_new(&t, WeightFrameInput::Tuples(&weights)).unwrap();
            let scalar = frame.operator_view();
            let grouped = scalar.with_grouping(&groups).unwrap();
            let n = scalar.dimension();
            let e = scalar.tuple_count();
            for scale in [
                0.0,
                -0.0,
                f64::from_bits(1),
                -f64::from_bits(1),
                1.0,
                1e-100,
                1e100,
            ] {
                let x: Vec<_> = (0..n).map(|i| scale * (i as f64 - 8.5)).collect();
                let y: Vec<_> = (0..e).map(|i| scale * (i as f64 - 13.25)).collect();
                let mut a = vec![f64::NAN; n];
                let mut b = vec![f64::INFINITY; n];
                let mut image = vec![f64::NAN; e];
                scalar.apply_gramian(&x, &mut a).unwrap();
                grouped.apply_gramian(&x, &mut b).unwrap();
                bits(&a, &b);
                grouped
                    .apply_gramian_with_image(&x, &mut b, &mut image)
                    .unwrap();
                bits(&a, &b);
                scalar.apply_adjoint(&y, &mut a).unwrap();
                grouped.apply_adjoint(&y, &mut b).unwrap();
                bits(&a, &b);
                scalar.apply_weighted_adjoint(&y, &mut a).unwrap();
                grouped.apply_weighted_adjoint(&y, &mut b).unwrap();
                bits(&a, &b);
                scalar.rhs_from_targets_into(&y, &mut a).unwrap();
                grouped.rhs_from_targets_into(&y, &mut b).unwrap();
                bits(&a, &b);
            }
        }
    }
}
#[test]
fn grouped_owners_dimensions_and_image_fail_before_any_write() {
    let t = PreparedThreeWayTopology::try_from_collapsed([1; 3], &[[0; 3]]).unwrap();
    let other = PreparedThreeWayTopology::try_from_collapsed([1; 3], &[[0; 3]]).unwrap();
    let groups = PreparedTupleGrouping::try_new(&t).unwrap();
    let foreign = PreparedTupleGrouping::try_new(&other).unwrap();
    let f = ThreeWayWeightFrame::try_new(&t, WeightFrameInput::UnitTuples).unwrap();
    assert!(matches!(
        f.operator_view().with_grouping(&foreign),
        Err(IncidenceError::TopologyBindingMismatch)
    ));
    let g = f.operator_view().with_grouping(&groups).unwrap();
    let sentinel = f64::from_bits(0x7ff8000000005432);
    let mut out = [sentinel; 3];
    let mut image = [sentinel];
    assert!(
        g.apply_gramian_with_image(&[], &mut out, &mut image)
            .is_err()
    );
    bits(&out, &[sentinel; 3]);
    bits(&image, &[sentinel]);
    assert!(
        g.apply_gramian_with_image(&[1.0; 3], &mut out[..2], &mut image)
            .is_err()
    );
    bits(&out, &[sentinel; 3]);
    bits(&image, &[sentinel]);
    assert!(
        g.apply_gramian_with_image(&[1.0; 3], &mut out, &mut image[..0])
            .is_err()
    );
    bits(&out, &[sentinel; 3]);
    bits(&image, &[sentinel]);
    assert!(g.apply_gramian(&[], &mut out).is_err());
    assert!(g.apply_gramian(&[1.0; 3], &mut out[..2]).is_err());
    assert!(g.apply_adjoint(&[], &mut out).is_err());
    assert!(g.apply_adjoint(&[1.0], &mut out[..2]).is_err());
    assert!(g.apply_weighted_adjoint(&[], &mut out).is_err());
    assert!(g.apply_weighted_adjoint(&[1.0], &mut out[..2]).is_err());
    assert!(g.rhs_from_targets_into(&[], &mut out).is_err());
    assert!(g.rhs_from_targets_into(&[1.0], &mut out[..2]).is_err());
    assert!(g.residual_into(&[], &[1.0; 3], &mut out).is_err());
    assert!(g.residual_into(&[1.0; 3], &[], &mut out).is_err());
    assert!(
        g.residual_into(&[1.0; 3], &[1.0; 3], &mut out[..2])
            .is_err()
    );
    bits(&out, &[sentinel; 3]);
    g.apply_gramian_with_image(&[1.0; 3], &mut out, &mut image)
        .unwrap();
    bits(&out, &[3.0; 3]);
    let next = ThreeWayWeightFrame::try_new(&t, WeightFrameInput::Tuples(&[2.0])).unwrap();
    let new = next.operator_view().with_grouping(&groups).unwrap();
    std::thread::scope(|s| {
        let old = s.spawn(|| {
            let mut x = [0.0; 3];
            g.apply_gramian(&[1.0; 3], &mut x).unwrap();
            x
        });
        let fresh = s.spawn(|| {
            let mut x = [0.0; 3];
            new.apply_gramian(&[1.0; 3], &mut x).unwrap();
            x
        });
        bits(&old.join().unwrap(), &[3.0; 3]);
        bits(&fresh.join().unwrap(), &[6.0; 3]);
    });
}

#[test]
fn grouped_fine_and_replayed_coarse_operators_preserve_nonmonotone_galerkin() {
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
    let coarse_groups = PreparedTupleGrouping::try_new(replay.frame().topology()).unwrap();
    let coarse = replay
        .frame()
        .operator_view()
        .with_grouping(&coarse_groups)
        .unwrap();
    let fine_groups = PreparedTupleGrouping::try_new(&t).unwrap();
    let x = [0.25, -0.5, 1.0, -2.0, 4.0];
    let mut prolonged = [0.0; 9];
    aggregation.prolong(&x, &mut prolonged).unwrap();
    let mut applied = [0.0; 9];
    f.operator_view()
        .with_grouping(&fine_groups)
        .unwrap()
        .apply_gramian(&prolonged, &mut applied)
        .unwrap();
    let mut restricted = [0.0; 5];
    aggregation.restrict(&applied, &mut restricted).unwrap();
    let mut actual = [0.0; 5];
    coarse.apply_gramian(&x, &mut actual).unwrap();
    bits(&actual, &restricted);
}
