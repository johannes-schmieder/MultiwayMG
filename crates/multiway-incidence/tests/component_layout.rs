//! Component permutations preserve original coordinates, bits and local operators.
use multiway_incidence::{
    PreparedComponentLayout, PreparedComponentRecoding, PreparedComponentRoot,
    PreparedHierarchyBudget, PreparedThreeWayTopology, ThreeWayWeightFrame, WeightFrameInput,
};
const B: PreparedHierarchyBudget = PreparedHierarchyBudget::UNLIMITED;
fn bits(a: &[f64], b: &[f64]) {
    assert_eq!(a.len(), b.len());
    for (a, b) in a.iter().zip(b) {
        assert_eq!(a.to_bits(), b.to_bits());
    }
}
fn check(t: &PreparedThreeWayTopology) {
    let a = PreparedComponentLayout::try_new(t, B).unwrap();
    let r = PreparedComponentRecoding::try_new(&a, B).unwrap();
    assert!(a.validate_for(t).is_ok());
    let other = PreparedThreeWayTopology::try_from_collapsed(
        t.topology().level_counts(),
        t.topology().tuples(),
    )
    .unwrap();
    assert!(a.validate_for(&other).is_err());
    assert!(a.component(a.component_count()).is_none());
    assert!(a.component(usize::MAX).is_none());
    let v = t.topology().total_levels();
    let e = t.topology().tuple_count();
    let patterns = [
        0u64,
        1u64,
        (-0.0f64).to_bits(),
        0x7ff8_0000_0000_0042,
        0xfff0_0000_0000_0000,
        1.25f64.to_bits(),
    ];
    let x: Vec<_> = (0..v)
        .map(|i| f64::from_bits(patterns[i % patterns.len()]))
        .collect();
    let y: Vec<_> = (0..e)
        .map(|i| f64::from_bits(patterns[i % patterns.len()]))
        .collect();
    let mut sx = vec![7.; v];
    let mut sy = vec![9.; e];
    let mut seen_levels = vec![0; v];
    let mut seen_tuples = vec![0; e];
    let weights: Vec<_> = (0..e).map(|i| 2f64.powi((i % 11) as i32 - 5)).collect();
    let fine = ThreeWayWeightFrame::try_new(t, WeightFrameInput::Tuples(&weights)).unwrap();
    let target: Vec<_> = (0..e).map(|i| (i as f64 * 0.17).cos()).collect();
    let mut global_rhs = vec![0.; v];
    fine.operator_view()
        .rhs_from_targets_into(&target, &mut global_rhs)
        .unwrap();
    let wrong_frame = ThreeWayWeightFrame::try_new(&other, WeightFrameInput::UnitTuples).unwrap();
    for c in 0..a.component_count() {
        let view = a.component(c).unwrap();
        assert!(view.factor_levels(3).is_none());
        let mut global_ids = vec![];
        for q in 0..3 {
            let mut last = None;
            view.factor_levels(q).unwrap().for_each(|id| {
                assert!(last.is_none_or(|previous| previous < id));
                last = Some(id);
                let global = t.topology().offsets()[q] + id;
                assert_eq!(t.component_labels()[global], c);
                seen_levels[global] += 1;
                global_ids.push(global);
            });
        }
        let mut tuple_ids = vec![];
        view.tuple_ids().for_each(|id| {
            assert!(tuple_ids.last().is_none_or(|&last| last < id));
            seen_tuples[id] += 1;
            tuple_ids.push(id);
        });
        let mut lx = vec![1.; view.dimension()];
        let mut ly = vec![2.; view.tuple_count()];
        view.gather_coefficients(&x, &mut lx).unwrap();
        view.gather_tuple_values(&y, &mut ly).unwrap();
        view.scatter_coefficients(&lx, &mut sx).unwrap();
        view.scatter_tuple_values(&ly, &mut sy).unwrap();
        let mut keys = vec![[0; 3]; view.tuple_count()];
        r.write_keys_into(c, &mut keys).unwrap();
        let mut visited = vec![];
        r.for_each_key(c, |id, key| visited.push((id, key)))
            .unwrap();
        assert_eq!(
            visited,
            tuple_ids
                .iter()
                .copied()
                .zip(keys.iter().copied())
                .collect::<Vec<_>>()
        );
        let mut called = false;
        assert!(r.for_each_key(usize::MAX, |_, _| called = true).is_err());
        assert!(!called);
        assert!(keys.windows(2).all(|k| k[0] < k[1]));
        for (i, key) in keys.iter().enumerate() {
            for q in 0..3 {
                let local_offset = view.level_counts()[..q].iter().sum::<usize>();
                assert_eq!(
                    global_ids[local_offset + key[q] as usize],
                    t.topology()
                        .global_index(q, t.topology().tuples()[tuple_ids[i]][q])
                );
            }
        }
        let local =
            PreparedThreeWayTopology::try_from_collapsed(view.level_counts(), &keys).unwrap();
        let mut w = vec![0.; view.tuple_count()];
        view.gather_tuple_values(&weights, &mut w).unwrap();
        let frame = ThreeWayWeightFrame::try_new(&local, WeightFrameInput::Tuples(&w)).unwrap();
        let root = PreparedComponentRoot::try_new(&r, c, B).unwrap();
        assert_eq!(root.component().index(), c);
        assert_eq!(root.topology().topology(), local.topology());
        assert_eq!(root.topology().component_labels(), local.component_labels());
        assert_eq!(
            root.topology().component_factor_sizes(),
            local.component_factor_sizes()
        );
        let root_frame = root.try_weight_frame(&fine, B).unwrap();
        bits(root_frame.weights(), frame.weights());
        bits(
            root_frame.square_root_weights(),
            frame.square_root_weights(),
        );
        bits(root_frame.diagonal(), frame.diagonal());
        // Original-coordinate RHS touches only this component and is identical
        // to the matching restriction of the independently computed full RHS.
        let untouched = f64::from_bits(0x7ff8_0000_0000_0042);
        let mut partial = vec![untouched; v];
        view.rhs_from_targets_in_global(&fine, &target, &mut partial)
            .unwrap();
        for i in 0..v {
            assert_eq!(
                partial[i].to_bits(),
                if t.component_labels()[i] == c {
                    global_rhs[i].to_bits()
                } else {
                    untouched.to_bits()
                }
            );
        }
        let saved = partial.clone();
        assert!(
            view.rhs_from_targets_in_global(&wrong_frame, &target, &mut partial)
                .is_err()
        );
        bits(&partial, &saved);
        assert!(
            view.rhs_from_targets_in_global(&fine, &target[..e - 1], &mut partial)
                .is_err()
        );
        bits(&partial, &saved);
        let mut short_rhs = vec![untouched; v - 1];
        assert!(
            view.rhs_from_targets_in_global(&fine, &target, &mut short_rhs)
                .is_err()
        );
        assert!(short_rhs.iter().all(|x| x.to_bits() == untouched.to_bits()));

        let z: Vec<_> = (0..view.dimension())
            .map(|i| (i as f64 * 0.19).sin())
            .collect();
        let mut global = vec![0.; v];
        view.scatter_coefficients(&z, &mut global).unwrap();
        let mut image = vec![0.; v];
        fine.operator_view()
            .apply_gramian(&global, &mut image)
            .unwrap();
        let mut expected = vec![0.; view.dimension()];
        view.gather_coefficients(&image, &mut expected).unwrap();
        let mut actual = vec![0.; view.dimension()];
        frame
            .operator_view()
            .apply_gramian(&z, &mut actual)
            .unwrap();
        bits(&actual, &expected);
        root_frame
            .operator_view()
            .apply_gramian(&z, &mut actual)
            .unwrap();
        bits(&actual, &expected);

        // Both malformed lengths are checked before writes, independently.
        let before = lx.clone();
        assert!(view.gather_coefficients(&x[..v - 1], &mut lx).is_err());
        bits(&lx, &before);
        let mut short = vec![17.; view.dimension() - 1];
        assert!(view.gather_coefficients(&x, &mut short).is_err());
        assert!(short.iter().all(|&v| v == 17.));
        let before = sx.clone();
        assert!(
            view.scatter_coefficients(&lx[..lx.len() - 1], &mut sx)
                .is_err()
        );
        bits(&sx, &before);
        let mut short = vec![19.; v - 1];
        assert!(view.scatter_coefficients(&lx, &mut short).is_err());
        assert!(short.iter().all(|&v| v == 19.));
        let before = ly.clone();
        assert!(view.gather_tuple_values(&y[..e - 1], &mut ly).is_err());
        bits(&ly, &before);
        let mut short = vec![23.; ly.len() - 1];
        assert!(view.gather_tuple_values(&y, &mut short).is_err());
        assert!(short.iter().all(|&v| v == 23.));
        let before = sy.clone();
        assert!(
            view.scatter_tuple_values(&ly[..ly.len() - 1], &mut sy)
                .is_err()
        );
        bits(&sy, &before);
        let mut short = vec![29.; e - 1];
        assert!(view.scatter_tuple_values(&ly, &mut short).is_err());
        assert!(short.iter().all(|&v| v == 29.));
        let mut bad = vec![[99; 3]; view.tuple_count() + 1];
        assert!(r.write_keys_into(c, &mut bad).is_err());
        assert!(r.write_keys_into(usize::MAX, &mut bad).is_err());
        assert!(bad.iter().all(|key| *key == [99; 3]));
    }
    assert!(seen_levels.iter().all(|&n| n == 1));
    assert!(seen_tuples.iter().all(|&n| n == 1));
    bits(&x, &sx);
    bits(&y, &sy);
    drop(r);
    assert_eq!(a.component_count(), t.component_factor_sizes().len());
}
#[test]
fn identity_interleaving_ragged_and_observation_groups() {
    let keys: Vec<_> = (0..3)
        .flat_map(|a| (0..2).flat_map(move |b| (0..4).map(move |c| [a, b, c])))
        .collect();
    let t = PreparedThreeWayTopology::try_from_collapsed([3, 2, 4], &keys).unwrap();
    check(&t);
    let mut keys: Vec<_> = [0, 2]
        .into_iter()
        .flat_map(|a| {
            [1, 3]
                .into_iter()
                .flat_map(move |b| [0, 2].into_iter().map(move |c| [a, b, c]))
        })
        .collect();
    keys.extend([[1, 0, 1], [1, 2, 1]]);
    keys.sort_unstable();
    let t = PreparedThreeWayTopology::try_from_collapsed([3, 4, 3], &keys).unwrap();
    check(&t);
    let keys: Vec<_> = (0..17).map(|i| [i, i, 16 - i]).collect();
    let t = PreparedThreeWayTopology::try_from_collapsed([17; 3], &keys).unwrap();
    check(&t);
}
#[test]
fn duplicate_observations_and_disconnected_extra_nullity() {
    let rows = [[1, 0, 1], [0, 1, 0], [1, 0, 1], [0, 1, 0], [1, 0, 1]];
    let t = PreparedThreeWayTopology::try_from_observations([2; 3], &rows).unwrap();
    let before = t
        .observation_groups()
        .unwrap()
        .observation_to_tuple()
        .to_vec();
    check(&t);
    assert_eq!(
        t.observation_groups().unwrap().observation_to_tuple(),
        before
    );
    let mut keys: Vec<_> = (0..8)
        .flat_map(|a| (0..8).map(move |b| [2 * a, 2 * b, 2 * ((a + b) % 8)]))
        .collect();
    keys.extend((0..8).map(|i| [2 * i + 1, 2 * i + 1, 2 * i + 1]));
    keys.sort_unstable();
    let t = PreparedThreeWayTopology::try_from_collapsed([16; 3], &keys).unwrap();
    check(&t);
}

#[test]
fn nested_factor_has_explicit_nonstructural_null_vectors() {
    let mut keys: Vec<_> = (0..4)
        .flat_map(|i| (0..3).map(move |j| [i, j, j]))
        .collect();
    keys.push([4, 3, 3]);
    let t = PreparedThreeWayTopology::try_from_collapsed([5, 4, 4], &keys).unwrap();
    check(&t);
    assert_eq!(t.component_factor_sizes().len(), 2);
    let f = ThreeWayWeightFrame::try_new(&t, WeightFrameInput::UnitTuples).unwrap();
    // Each vector sums to zero within its factor and is therefore independent
    // of the constant factor-shift modes. The two vectors are independent.
    for second in [1, 2] {
        let mut x = [0.; 13];
        x[5] = 1.;
        x[5 + second] = -1.;
        x[9] = -1.;
        x[9 + second] = 1.;
        let mut y = [7.; 13];
        f.operator_view().apply_incidence(&x, &mut y).unwrap();
        assert_eq!(y, [0.; 13]);
    }
}

#[test]
fn local_root_replays_current_weights_after_inverse_is_dropped() {
    let keys = [
        [0, 0, 1],
        [0, 2, 3],
        [1, 1, 0],
        [2, 0, 1],
        [2, 2, 3],
        [3, 3, 2],
    ];
    let t = PreparedThreeWayTopology::try_from_collapsed([4; 3], &keys).unwrap();
    let layout = PreparedComponentLayout::try_new(&t, B).unwrap();
    let inverse = PreparedComponentRecoding::try_new(&layout, B).unwrap();
    let roots: Vec<_> = (0..layout.component_count())
        .map(|c| PreparedComponentRoot::try_new(&inverse, c, B).unwrap())
        .collect();
    drop(inverse);
    for generation in 0..16 {
        let weights: Vec<_> = (0..keys.len())
            .map(|i| {
                if generation % 2 == 0 {
                    2f64.powi(((i + generation) % 13) as i32 - 6)
                } else {
                    (i + generation + 1) as f64 / 7.0
                }
            })
            .collect();
        let source = ThreeWayWeightFrame::try_new(&t, WeightFrameInput::Tuples(&weights)).unwrap();
        for root in &roots {
            let frame = root.try_weight_frame(&source, B).unwrap();
            let mut local_weights = vec![0.; root.component().tuple_count()];
            root.component()
                .gather_tuple_values(&weights, &mut local_weights)
                .unwrap();
            let reference = ThreeWayWeightFrame::try_new(
                root.topology(),
                WeightFrameInput::Tuples(&local_weights),
            )
            .unwrap();
            bits(frame.weights(), reference.weights());
            bits(frame.square_root_weights(), reference.square_root_weights());
            bits(frame.diagonal(), reference.diagonal());
            let x: Vec<_> = (0..frame.diagonal().len())
                .map(|i| (i as f64 * 0.37).sin())
                .collect();
            let mut a = vec![0.; x.len()];
            let mut b = a.clone();
            frame.operator_view().apply_gramian(&x, &mut a).unwrap();
            reference.operator_view().apply_gramian(&x, &mut b).unwrap();
            bits(&a, &b);
        }
    }
}
