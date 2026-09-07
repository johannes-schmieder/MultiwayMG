//! Prepared and ordinary scalar equivalence and exact numerical ownership.
use multiway_incidence::{
    PreparedThreeWayTopology, PreparedTupleGrouping, ThreeWayWeightFrame, WeightFrameInput,
};
use multiway_mg::{PreparedSymmetricMap, SymmetricMapPreconditioner, ThreeWayProblem};

fn bits(a: &[f64], b: &[f64]) {
    assert_eq!(
        a.iter().map(|x| x.to_bits()).collect::<Vec<_>>(),
        b.iter().map(|x| x.to_bits()).collect::<Vec<_>>()
    );
}
fn cases() -> Vec<([usize; 3], Vec<[u32; 3]>)> {
    vec![
        (
            [2; 3],
            (0..2)
                .flat_map(|i| (0..2).flat_map(move |j| (0..2).map(move |k| [i, j, k])))
                .collect(),
        ),
        ([2; 3], vec![[0, 0, 0], [1, 1, 1]]),
        ([2, 3, 4], vec![[0, 0, 0], [0, 1, 1], [0, 2, 3], [1, 2, 2]]),
        ([2; 3], vec![[0, 0, 0], [0, 1, 1], [1, 0, 1], [1, 1, 0]]),
        (
            [3, 33, 8],
            (0..32)
                .map(|j| [0, j, j % 7])
                .chain((0..32).map(|j| [1, j, (j + 3) % 7]))
                .chain([[2, 32, 7]])
                .collect(),
        ),
    ]
}

#[test]
fn prepared_projection_and_map_match_ordinary_bits_and_are_symmetric() {
    for (counts, tuples) in cases() {
        let topology = PreparedThreeWayTopology::try_from_collapsed(counts, &tuples).unwrap();
        let grouping = PreparedTupleGrouping::try_new(&topology).unwrap();
        for change in 0..3 {
            let weights: Vec<_> = (0..tuples.len())
                .map(|i| 1.0 + (i * (change + 1)) as f64)
                .collect();
            let frame = ThreeWayWeightFrame::try_new(&topology, WeightFrameInput::Tuples(&weights))
                .unwrap();
            let problem = ThreeWayProblem::from_observations(counts, &tuples, &weights).unwrap();
            let n = problem.dimension();
            let ordinary = SymmetricMapPreconditioner::new(problem);
            let map = PreparedSymmetricMap::new(&frame);
            let mut old_workspace = ordinary.application_workspace().unwrap();
            let mut workspace = map.application_workspace().unwrap();
            let mut old_projection = ordinary
                .problem()
                .components()
                .try_projection_workspace()
                .unwrap();
            let mut projection = topology.try_projection_workspace().unwrap();
            for rhs_index in 0..8 {
                let x: Vec<_> = (0..n)
                    .map(|i| ((i + rhs_index) as f64 * 0.19).sin())
                    .collect();
                let y: Vec<_> = (0..n)
                    .map(|i| ((i + rhs_index) as f64 * 0.73).cos())
                    .collect();
                let mut actual = x.clone();
                let mut expected = x.clone();
                let a = topology
                    .project_structural_range_with_workspace(&mut actual, &mut projection)
                    .unwrap();
                let b = ordinary
                    .problem()
                    .components()
                    .project_structural_range_with_workspace(&mut expected, &mut old_projection)
                    .unwrap();
                assert_eq!(a.to_bits(), b.to_bits());
                bits(&actual, &expected);
                let a = topology
                    .maximum_structural_defect_with_workspace(&actual, &mut projection)
                    .unwrap();
                let b = ordinary
                    .problem()
                    .components()
                    .maximum_structural_defect_with_workspace(&expected, &mut old_projection)
                    .unwrap();
                assert_eq!(a.to_bits(), b.to_bits());
                map.apply_with_workspace(&x, &mut actual, &mut workspace)
                    .unwrap();
                ordinary
                    .apply_with_workspace(&x, &mut expected, &mut old_workspace)
                    .unwrap();
                bits(&actual, &expected);
                let mut grouped = vec![f64::NAN; n];
                map.with_grouping(&grouping)
                    .unwrap()
                    .apply_with_workspace(&x, &mut grouped, &mut workspace)
                    .unwrap();
                bits(&grouped, &expected);
                let mut my = vec![0.0; n];
                map.apply_with_workspace(&y, &mut my, &mut workspace)
                    .unwrap();
                let left: f64 = actual.iter().zip(&y).map(|(a, b)| a * b).sum();
                let right: f64 = x.iter().zip(&my).map(|(a, b)| a * b).sum();
                assert!((left - right).abs() < 1e-12 * (1.0 + left.abs() + right.abs()));
                let energy: f64 = x.iter().zip(&actual).map(|(a, b)| a * b).sum();
                assert!(energy >= -1e-13);
                // Old and prepared arbitrary scratch contents are overwritten on each call.
                map.apply_with_workspace(&x, &mut my, &mut workspace)
                    .unwrap();
                bits(&actual, &my);
            }
        }
    }
}

#[test]
fn frame_binding_and_dimensions_reject_before_output_or_projection_mutation() {
    let topology = PreparedThreeWayTopology::try_from_collapsed([2; 3], &[[0; 3], [1; 3]]).unwrap();
    let other = PreparedThreeWayTopology::try_from_collapsed([2; 3], &[[0; 3], [1; 3]]).unwrap();
    let frame = ThreeWayWeightFrame::try_new(&topology, WeightFrameInput::UnitTuples).unwrap();
    let equal = ThreeWayWeightFrame::try_new(&topology, WeightFrameInput::UnitTuples).unwrap();
    let changed =
        ThreeWayWeightFrame::try_new(&topology, WeightFrameInput::Tuples(&[2.0; 2])).unwrap();
    let map = PreparedSymmetricMap::new(&frame);
    let grouping = PreparedTupleGrouping::try_new(&topology).unwrap();
    let grouped = map.with_grouping(&grouping).unwrap();
    let mut scratch = map.application_workspace().unwrap();
    let sentinel = f64::from_bits(0x7ff8000000001234);
    let mut output = [sentinel; 6];
    let saved = format!("{scratch:?}");
    for alternate in [&equal, &changed] {
        assert!(
            PreparedSymmetricMap::new(alternate)
                .apply_with_workspace(&[1.0; 6], &mut output, &mut scratch)
                .is_err()
        );
        assert!(
            PreparedSymmetricMap::new(alternate)
                .with_grouping(&grouping)
                .unwrap()
                .apply_with_workspace(&[1.0; 6], &mut output, &mut scratch)
                .is_err()
        );
        assert_eq!(format!("{scratch:?}"), saved);
        bits(&output, &[sentinel; 6]);
    }
    assert!(
        map.apply_with_workspace(&[1.0; 5], &mut output, &mut scratch)
            .is_err()
    );
    assert!(
        map.apply_with_workspace(&[1.0; 6], &mut output[..5], &mut scratch)
            .is_err()
    );
    assert_eq!(format!("{scratch:?}"), saved);
    bits(&output, &[sentinel; 6]);
    assert!(
        grouped
            .apply_with_workspace(&[1.0; 5], &mut output, &mut scratch)
            .is_err()
    );
    assert!(
        grouped
            .apply_with_workspace(&[1.0; 6], &mut output[..5], &mut scratch)
            .is_err()
    );
    assert_eq!(format!("{scratch:?}"), saved);
    bits(&output, &[sentinel; 6]);
    let foreign = PreparedTupleGrouping::try_new(&other).unwrap();
    assert!(map.with_grouping(&foreign).is_err());
    let mut projection = topology.try_projection_workspace().unwrap();
    let saved = format!("{projection:?}");
    assert!(
        other
            .project_structural_range_with_workspace(&mut output, &mut projection)
            .is_err()
    );
    assert!(
        topology
            .project_structural_range_with_workspace(&mut output[..5], &mut projection)
            .is_err()
    );
    assert_eq!(format!("{projection:?}"), saved);
    bits(&output, &[sentinel; 6]);
    map.apply_with_workspace(&[1.0; 6], &mut output, &mut scratch)
        .unwrap();
}

#[test]
fn nonfinite_and_overflow_fail_transactionally_then_storage_recovers() {
    let topology = PreparedThreeWayTopology::try_from_collapsed([2; 3], &[[0; 3], [1; 3]]).unwrap();
    let frame =
        ThreeWayWeightFrame::try_new(&topology, WeightFrameInput::Tuples(&[1e-308; 2])).unwrap();
    let map = PreparedSymmetricMap::new(&frame);
    let grouping = PreparedTupleGrouping::try_new(&topology).unwrap();
    let grouped = map.with_grouping(&grouping).unwrap();
    let mut scratch = map.application_workspace().unwrap();
    let mut output = [7.0; 6];
    for bad in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY, f64::MAX] {
        assert!(
            map.apply_with_workspace(&[bad; 6], &mut output, &mut scratch)
                .is_err()
        );
        bits(&output, &[7.0; 6]);
        assert!(
            grouped
                .apply_with_workspace(&[bad; 6], &mut output, &mut scratch)
                .is_err()
        );
        bits(&output, &[7.0; 6]);
        let mut valid = [0.0; 6];
        map.apply_with_workspace(&[1e-308; 6], &mut valid, &mut scratch)
            .unwrap();
        assert!(valid.iter().all(|x| (x - 1.0 / 3.0).abs() < 1e-14));
        grouped
            .apply_with_workspace(&[1e-308; 6], &mut output, &mut scratch)
            .unwrap();
        bits(&valid, &output);
        output.fill(7.0);
    }
}
