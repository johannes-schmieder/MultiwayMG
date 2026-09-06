//! Prepared/ordinary fixed V-cycle equivalence and bounded numerical ownership.
#[allow(dead_code)]
#[path = "../examples/support/issue3_recursive_fixtures.rs"]
mod fixtures;
use multiway_incidence::{
    FactorAggregation, HierarchyWeightFrames, PreparedHierarchyTopology, PreparedThreeWayTopology,
    ThreeWayWeightFrame, WeightFrameInput,
};
use multiway_mg::{
    CycleScreenedMapHierarchy, MultiwayError, Preconditioner, PreparedMapHierarchy, ThreeWayProblem,
};

fn bits(a: &[f64], b: &[f64]) {
    assert_eq!(
        a.iter().map(|x| x.to_bits()).collect::<Vec<_>>(),
        b.iter().map(|x| x.to_bits()).collect::<Vec<_>>()
    );
}
#[test]
fn recursive_cases_match_ordinary_bits_and_symmetry() -> Result<(), Box<dyn std::error::Error>> {
    for fixture in fixtures::recursive_holdout_fixtures()? {
        let topology = PreparedThreeWayTopology::try_from_collapsed(
            fixture.problem.topology().level_counts(),
            fixture.problem.topology().tuples(),
        )?;
        let structural =
            PreparedHierarchyTopology::try_new(&topology, fixture.oracle_maps.clone())?;
        for change in 0..2 {
            let weights: Vec<_> = fixture
                .problem
                .weights()
                .iter()
                .enumerate()
                .map(|(i, w)| w * (1.0 + change as f64 * (i % 7) as f64))
                .collect();
            let fine = ThreeWayWeightFrame::try_new(&topology, WeightFrameInput::Tuples(&weights))?;
            let frames = HierarchyWeightFrames::try_new(&structural, &fine)?;
            let hierarchy = PreparedMapHierarchy::try_new(&frames, 1e-12)?;
            let mut scratch = hierarchy.application_workspace()?;
            let problem = ThreeWayProblem::from_observations(
                topology.topology().level_counts(),
                topology.topology().tuples(),
                &weights,
            )?;
            let ordinary =
                CycleScreenedMapHierarchy::from_maps(problem, fixture.oracle_maps.clone(), 1e-12)?;
            let mut old_scratch = ordinary.application_workspace()?;
            let n = hierarchy.dimension();
            for column in 0..4 {
                let x: Vec<_> = (0..n).map(|i| ((i + column) as f64 * 0.31).sin()).collect();
                let y: Vec<_> = (0..n).map(|i| ((i + column) as f64 * 0.19).cos()).collect();
                let mut actual = vec![0.0; n];
                let mut expected = actual.clone();
                hierarchy.apply_with_workspace(&x, &mut actual, &mut scratch)?;
                ordinary.apply_with_workspace(&x, &mut expected, &mut old_scratch)?;
                bits(&actual, &expected);
                let mut my = actual.clone();
                hierarchy.apply_with_workspace(&y, &mut my, &mut scratch)?;
                let left: f64 = actual.iter().zip(&y).map(|(a, b)| a * b).sum();
                let right: f64 = my.iter().zip(&x).map(|(a, b)| a * b).sum();
                assert!(
                    (left - right).abs() <= 1e-10 * (1.0 + left.abs() + right.abs()),
                    "{}",
                    fixture.name
                );
                let positive: f64 = actual.iter().zip(&x).map(|(a, b)| a * b).sum();
                assert!(positive >= -1e-10);
                hierarchy.apply_with_workspace(&x, &mut my, &mut scratch)?;
                bits(&actual, &my);
            }
            let report = hierarchy.payload_report(&scratch, 123)?;
            assert_eq!(
                report.workspace_payload_bytes,
                hierarchy.workspace_required_bytes()?
            );
            let base = hierarchy.payload_report(&scratch, 0)?;
            assert_eq!(report.total_payload_bytes, base.total_payload_bytes + 123);
            assert!(hierarchy.payload_report(&scratch, usize::MAX).is_err());
        }
    }
    Ok(())
}

#[test]
fn extra_nullity_and_terminal_only_match_rank_revealing_reference() {
    // Four tuples in six coordinates: rank may be below the structural maximum.
    for tuples in [
        vec![[0, 0, 0], [1, 1, 1]],
        vec![[0, 0, 0], [0, 1, 1], [1, 0, 1]],
        vec![[0, 0, 0], [0, 1, 1], [1, 0, 1], [1, 1, 0]],
    ] {
        let topology = PreparedThreeWayTopology::try_from_collapsed([2; 3], &tuples).unwrap();
        let structural = PreparedHierarchyTopology::try_new(&topology, vec![]).unwrap();
        let fine = ThreeWayWeightFrame::try_new(&topology, WeightFrameInput::UnitTuples).unwrap();
        let frames = HierarchyWeightFrames::try_new(&structural, &fine).unwrap();
        let hierarchy = PreparedMapHierarchy::try_new(&frames, 1e-12).unwrap();
        let mut scratch = hierarchy.application_workspace().unwrap();
        let problem =
            ThreeWayProblem::from_observations([2; 3], &tuples, &vec![1.0; tuples.len()]).unwrap();
        let old = CycleScreenedMapHierarchy::from_maps(problem.clone(), vec![], 1e-12).unwrap();
        let x: Vec<_> = (0..6).map(|i| (i as f64 * 0.31).sin()).collect();
        let mut rhs = vec![0.0; 6];
        problem.apply_gramian(&x, &mut rhs).unwrap();
        let mut actual = [0.0; 6];
        let mut expected = actual;
        hierarchy
            .apply_with_workspace(&rhs, &mut actual, &mut scratch)
            .unwrap();
        old.apply(&rhs, &mut expected).unwrap();
        bits(&actual, &expected);
        let mut image = [0.0; 6];
        fine.operator_view()
            .apply_gramian(&actual, &mut image)
            .unwrap();
        for (&a, &b) in image.iter().zip(&rhs) {
            assert!((a - b).abs() < 1e-12);
        }
        assert!(hierarchy.terminal_rank() <= tuples.len());
    }
}

#[test]
fn owner_errors_and_numerical_failure_preserve_output_and_allow_recovery() {
    let topology = PreparedThreeWayTopology::try_from_collapsed([2; 3], &[[0; 3], [1; 3]]).unwrap();
    let structural = PreparedHierarchyTopology::try_new(
        &topology,
        vec![FactorAggregation::identity([2; 3]).unwrap()],
    )
    .unwrap();
    let fine = ThreeWayWeightFrame::try_new(&topology, WeightFrameInput::UnitTuples).unwrap();
    let equal = ThreeWayWeightFrame::try_new(&topology, WeightFrameInput::UnitTuples).unwrap();
    let frames = HierarchyWeightFrames::try_new(&structural, &fine).unwrap();
    let equal_frames = HierarchyWeightFrames::try_new(&structural, &equal).unwrap();
    let hierarchy = PreparedMapHierarchy::try_new(&frames, 1e-12).unwrap();
    let other = PreparedMapHierarchy::try_new(&frames, 1e-12).unwrap();
    let mut scratch = hierarchy.application_workspace().unwrap();
    let mut out = [7.0; 6];
    let before = format!("{scratch:?}");
    assert!(
        other
            .apply_with_workspace(&[1.0; 6], &mut out, &mut scratch)
            .is_err()
    );
    assert!(
        hierarchy
            .apply_with_workspace(&[1.0; 5], &mut out, &mut scratch)
            .is_err()
    );
    assert!(
        hierarchy
            .apply_with_workspace(&[1.0; 6], &mut out[..5], &mut scratch)
            .is_err()
    );
    assert!(hierarchy.validate_for(&equal_frames).is_err());
    assert_eq!(format!("{scratch:?}"), before);
    for bad in [f64::NAN, f64::INFINITY, f64::MAX] {
        assert!(
            hierarchy
                .apply_with_workspace(&[bad; 6], &mut out, &mut scratch)
                .is_err()
        );
        bits(&out, &[7.0; 6]);
        let mut valid = [0.0; 6];
        hierarchy
            .apply_with_workspace(&[1.0; 6], &mut valid, &mut scratch)
            .unwrap();
        assert!(valid.iter().all(|x| (x - 1.0 / 3.0).abs() < 1e-12));
    }
}

#[test]
fn large_terminal_and_excessive_depth_reject_before_factorization() {
    let rows: Vec<_> = (0..86).map(|i| [i; 3]).collect();
    let topology = PreparedThreeWayTopology::try_from_collapsed([86; 3], &rows).unwrap();
    let structural = PreparedHierarchyTopology::try_new(&topology, vec![]).unwrap();
    let fine = ThreeWayWeightFrame::try_new(&topology, WeightFrameInput::UnitTuples).unwrap();
    let frames = HierarchyWeightFrames::try_new(&structural, &fine).unwrap();
    assert!(matches!(
        PreparedMapHierarchy::try_new(&frames, 1e-12),
        Err(MultiwayError::HierarchyStagnated {
            dimension: 258,
            limit: 256,
            ..
        })
    ));
    let small = PreparedThreeWayTopology::try_from_collapsed([1; 3], &[[0; 3]]).unwrap();
    let maps = (0..64)
        .map(|_| FactorAggregation::identity([1; 3]).unwrap())
        .collect();
    let structural = PreparedHierarchyTopology::try_new(&small, maps).unwrap();
    let fine = ThreeWayWeightFrame::try_new(&small, WeightFrameInput::UnitTuples).unwrap();
    let frames = HierarchyWeightFrames::try_new(&structural, &fine).unwrap();
    assert!(matches!(
        PreparedMapHierarchy::try_new(&frames, 1e-12),
        Err(MultiwayError::InvalidOption {
            name: "prepared_hierarchy_levels",
            ..
        })
    ));
}
