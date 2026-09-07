//! Actual recursive tails agree with independent suffixes and legacy probes.
#[allow(dead_code)]
#[path = "../examples/support/issue3_recursive_fixtures.rs"]
mod fixtures;
use multiway_incidence::{
    FactorAggregation, HierarchyWeightFrames, PreparedHierarchyBudget, PreparedHierarchyGrouping,
    PreparedHierarchyTopology, PreparedThreeWayTopology, ThreeWayWeightFrame, WeightFrameInput,
};
use multiway_mg::{
    CycleQualityCriteria, CycleQualityOptions, CycleScreenedMapHierarchy, GroupedGramianMode,
    Preconditioner, PreparedCycleScreenWorkspace, PreparedMapHierarchy, ThreeWayProblem,
    analyze_cycle_quality,
};
fn bits(a: f64, b: f64) {
    assert_eq!(a.to_bits(), b.to_bits(), "{a} versus {b}");
}
fn compare(
    problem: ThreeWayProblem,
    maps: Vec<FactorAggregation>,
) -> Result<(), Box<dyn std::error::Error>> {
    let t = PreparedThreeWayTopology::try_from_collapsed(
        problem.topology().level_counts(),
        problem.topology().tuples(),
    )?;
    let f = ThreeWayWeightFrame::try_new(&t, WeightFrameInput::Tuples(problem.weights()))?;
    let h = PreparedHierarchyTopology::try_new(&t, maps.clone())?;
    let frames = HierarchyWeightFrames::try_new(&h, &f)?;
    let options = CycleQualityOptions {
        test_vectors: 2,
        power_iterations: 4,
        tail_iterations: 2,
        ..Default::default()
    };
    let criteria = CycleQualityCriteria {
        maximum_estimated_energy_factor: 1e20,
        maximum_observed_energy_factor: Some(1e20),
        maximum_structural_defect: 1e20,
    };
    for (prefix, mode) in [
        (0, GroupedGramianMode::RowGather),
        (1.min(h.level_count() - 1), GroupedGramianMode::RowGather),
        (h.level_count() - 1, GroupedGramianMode::RowGather),
        (1.min(h.level_count() - 1), GroupedGramianMode::TupleImage),
        (h.level_count() - 1, GroupedGramianMode::TupleImage),
    ] {
        let groups = PreparedHierarchyGrouping::try_new(&h, prefix)?;
        let numerical = PreparedMapHierarchy::try_new_with_grouping(&frames, &groups, mode, 1e-12)?;
        let mut cycle = numerical.application_workspace()?;
        let mut screen = PreparedCycleScreenWorkspace::try_new(
            &numerical,
            &cycle,
            PreparedHierarchyBudget::UNLIMITED,
        )?;
        let result = screen.screen(options, criteria, &mut cycle)?;
        assert!(result.accepted);
        assert_eq!(result.levels.len(), h.level_count());
        let mut gramian = 0;
        let mut preconditioner = 0;
        let mut energy = 0;
        for (position, report) in result.levels.iter().enumerate() {
            let level = h.level_count() - 1 - position;
            assert_eq!(report.level, level);
            let frame = frames.frame(level).unwrap();
            let local = ThreeWayProblem::from_observations(
                frame.topology().topology().level_counts(),
                frame.topology().topology().tuples(),
                frame.weights(),
            )?;
            let ordinary =
                CycleScreenedMapHierarchy::from_maps(local.clone(), maps[level..].to_vec(), 1e-12)?;
            for kind in 0..3 {
                let rhs: Vec<_> = (0..report.dimension)
                    .map(|i| {
                        if kind == 2 {
                            0.
                        } else {
                            ((i + kind) as f64 * 0.37).cos()
                        }
                    })
                    .collect();
                let mut expected = vec![0.; rhs.len()];
                ordinary.apply(&rhs, &mut expected)?;
                let mut actual = vec![f64::NAN; rhs.len()];
                numerical.apply_tail_with_workspace(level, &rhs, &mut actual, &mut cycle)?;
                for (&a, &b) in actual.iter().zip(&expected) {
                    bits(a, b);
                }
            }
            let reference = analyze_cycle_quality(&local, &ordinary, options)?;
            bits(
                report.maximum_estimated_energy_factor,
                reference.maximum_estimated_energy_factor(),
            );
            bits(
                report.maximum_observed_energy_factor,
                reference.maximum_observed_energy_factor(),
            );
            bits(
                report.maximum_absolute_final_rayleigh,
                reference.maximum_absolute_final_rayleigh(),
            );
            bits(
                report.maximum_structural_defect,
                reference.maximum_structural_defect(),
            );
            assert_eq!(
                report.annihilated_starts,
                reference
                    .vectors()
                    .iter()
                    .filter(|v| v.annihilated())
                    .count()
            );
            assert_eq!(report.completed_starts, options.test_vectors);
            gramian += reference.gramian_applications();
            preconditioner += reference.preconditioner_applications();
            energy += reference.energy_evaluations();
        }
        assert_eq!(result.work.gramian_applications, gramian);
        assert_eq!(result.work.cycle_applications, preconditioner);
        assert_eq!(result.work.energy_evaluations, energy);
        let reports = result.levels.to_vec();
        let work = result.work;
        let again = screen.screen(options, criteria, &mut cycle)?;
        assert_eq!(again.levels, reports);
        assert_eq!(again.work, work);
        let rejected = screen.screen(
            CycleQualityOptions {
                correction_damping: 0.01,
                ..options
            },
            CycleQualityCriteria {
                maximum_estimated_energy_factor: 0.5,
                ..criteria
            },
            &mut cycle,
        )?;
        assert!(!rejected.accepted);
        assert_eq!(rejected.levels.len(), 1);
        assert_eq!(rejected.levels[0].level, h.level_count() - 1);
        let rejected_report = rejected.levels[0];
        assert_eq!(screen.completed_level_reports(), &[rejected_report]);
        let failure = screen
            .screen(
                CycleQualityOptions {
                    relative_zero_tolerance: f64::MAX,
                    ..options
                },
                criteria,
                &mut cycle,
            )
            .unwrap_err();
        assert_eq!(failure.level, Some((h.level_count() - 1) as u8));
        assert_eq!(failure.start, Some(0));
        assert_eq!(failure.completed_tail_levels, 0);
        assert_eq!(failure.work.gramian_applications, 16);
        assert_eq!(failure.work.energy_evaluations, 16);
        assert_eq!(failure.work.projections, 16);
        assert_eq!(failure.work.cycle_applications, 0);
        assert!(screen.completed_level_reports().is_empty());
        let recovery = screen.screen(options, criteria, &mut cycle)?;
        assert_eq!(recovery.levels, reports);
        assert_eq!(recovery.work, work);
    }
    Ok(())
}
#[test]
fn recursive_tails_and_summary_probes_match_independent_full_history()
-> Result<(), Box<dyn std::error::Error>> {
    for fixture in fixtures::recursive_holdout_fixtures()? {
        let changed: Vec<_> = fixture
            .problem
            .weights()
            .iter()
            .enumerate()
            .map(|(i, w)| w * (0.5 + (i % 7) as f64 / 8.))
            .collect();
        let reweighted = ThreeWayProblem::from_observations(
            fixture.problem.topology().level_counts(),
            fixture.problem.topology().tuples(),
            &changed,
        )?;
        compare(fixture.problem, fixture.oracle_maps.clone())?;
        compare(reweighted, fixture.oracle_maps)?;
    }
    let mut keys: Vec<_> = (0..4)
        .flat_map(|i| (0..3).map(move |j| [i, j, j]))
        .collect();
    keys.push([4, 3, 3]);
    let problem = ThreeWayProblem::from_observations([5, 4, 4], &keys, &[1.; 13])?;
    compare(
        problem,
        vec![
            FactorAggregation::new(
                [5, 4, 4],
                [vec![0, 0, 1, 1, 2], vec![0, 0, 1, 2], vec![0, 0, 1, 2]],
            )?,
            FactorAggregation::new([3; 3], [vec![0, 0, 1], vec![0, 0, 1], vec![0, 0, 1]])?,
        ],
    )?;
    compare(
        ThreeWayProblem::from_observations([1; 3], &[[0; 3]], &[1.])?,
        vec![],
    )?;
    Ok(())
}
