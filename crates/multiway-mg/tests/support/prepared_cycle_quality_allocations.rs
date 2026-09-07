use super::{GLOBAL, Result, no_events};
use multiway_incidence::{
    FactorAggregation, HierarchyWeightFrames, PreparedHierarchyBudget, PreparedHierarchyTopology,
    PreparedThreeWayTopology, ThreeWayWeightFrame, WeightFrameInput,
};
use multiway_mg::{
    CycleQualityCriteria, CycleQualityOptions, PreparedCycleScreenWorkspace,
    PreparedLevelCycleQuality, PreparedMapHierarchy,
};
use std::hint::black_box;
const B: PreparedHierarchyBudget = PreparedHierarchyBudget::UNLIMITED;
pub fn run() -> Result<()> {
    let keys: Vec<_> = (0..2)
        .flat_map(|a| (0..2).flat_map(move |b| (0..2).map(move |c| [a, b, c])))
        .collect();
    let t = PreparedThreeWayTopology::try_from_collapsed([2; 3], &keys)?;
    let f = ThreeWayWeightFrame::try_new(&t, WeightFrameInput::UnitTuples)?;
    let h = PreparedHierarchyTopology::try_new(
        &t,
        vec![FactorAggregation::consecutive_halving([2; 3])?],
    )?;
    let frames = HierarchyWeightFrames::try_new(&h, &f)?;
    let owner = PreparedMapHierarchy::try_new(&frames, 1e-12)?;
    let mut cycle = owner.application_workspace()?;
    let original_cycle_bytes = cycle.retained_payload_bytes()?;
    let bound =
        PreparedCycleScreenWorkspace::setup_payload_report(&owner, &cycle, B)?.total_payload_bound;
    let before = GLOBAL.stats();
    assert!(
        PreparedCycleScreenWorkspace::try_new(
            &owner,
            &cycle,
            PreparedHierarchyBudget {
                maximum_payload_bytes: bound - 1,
                ..B
            }
        )
        .is_err()
    );
    no_events(GLOBAL.stats() - before);
    let before = GLOBAL.stats();
    let mut screen = black_box(PreparedCycleScreenWorkspace::try_new(&owner, &cycle, B)?);
    let delta = GLOBAL.stats() - before;
    let retained = screen.retained_payload_bytes()?;
    assert_eq!(delta.allocations, 2);
    assert_eq!(delta.reallocations, 0);
    assert_eq!(delta.deallocations, 0);
    assert_eq!(delta.bytes_allocated, retained);
    assert_eq!(
        retained,
        24 * owner.dimension() + frames.level_count() * size_of::<PreparedLevelCycleQuality>()
    );
    let o = CycleQualityOptions {
        test_vectors: 2,
        power_iterations: 4,
        tail_iterations: 2,
        ..Default::default()
    };
    let c = CycleQualityCriteria {
        maximum_estimated_energy_factor: 1e10,
        maximum_observed_energy_factor: Some(1e10),
        maximum_structural_defect: 1e10,
    };
    let before = GLOBAL.stats();
    assert!(screen.screen(o, c, &mut cycle)?.accepted);
    no_events(GLOBAL.stats() - before);
    let reference = screen.screen(o, c, &mut cycle)?;
    let reports = reference.levels.to_vec();
    let work = reference.work;
    let before = GLOBAL.stats();
    for _ in 0..32 {
        let r = screen.screen(o, c, &mut cycle)?;
        assert_eq!(r.levels, reports);
        assert_eq!(r.work, work);
        assert!(
            screen
                .screen(
                    CycleQualityOptions {
                        test_vectors: 0,
                        ..o
                    },
                    c,
                    &mut cycle
                )
                .is_err()
        );
        let r = screen.screen(
            CycleQualityOptions {
                correction_damping: 0.01,
                ..o
            },
            CycleQualityCriteria {
                maximum_estimated_energy_factor: 0.5,
                ..c
            },
            &mut cycle,
        )?;
        assert!(!r.accepted);
        assert_eq!(r.levels.len(), 1);
        let e = screen
            .screen(
                CycleQualityOptions {
                    relative_zero_tolerance: f64::MAX,
                    ..o
                },
                c,
                &mut cycle,
            )
            .unwrap_err();
        assert_eq!(e.work.gramian_applications, 16);
        assert_eq!(e.work.cycle_applications, 0);
        let e = screen
            .screen(
                CycleQualityOptions {
                    correction_damping: f64::MAX,
                    ..o
                },
                c,
                &mut cycle,
            )
            .unwrap_err();
        assert!(e.work.cycle_applications > 0);
    }
    no_events(GLOBAL.stats() - before);
    let other = PreparedMapHierarchy::try_new(&frames, 1e-12)?;
    let mut wrong = other.application_workspace()?;
    let before = GLOBAL.stats();
    assert!(screen.screen(o, c, &mut wrong).is_err());
    no_events(GLOBAL.stats() - before);
    let rhs = [0.25; 3];
    let mut out = [0.; 3];
    let before = GLOBAL.stats();
    owner.apply_tail_with_workspace(1, &rhs, &mut out, &mut cycle)?;
    no_events(GLOBAL.stats() - before);
    let expected = out;
    let before = GLOBAL.stats();
    for _ in 0..32 {
        owner.apply_tail_with_workspace(1, &rhs, &mut out, &mut cycle)?;
        assert_eq!(out, expected);
        let mut invalid = [17.; 3];
        assert!(
            owner
                .apply_tail_with_workspace(2, &rhs, &mut invalid, &mut cycle)
                .is_err()
        );
        assert!(
            owner
                .apply_tail_with_workspace(usize::MAX, &rhs, &mut invalid, &mut cycle)
                .is_err()
        );
        assert!(
            owner
                .apply_tail_with_workspace(1, &rhs[..2], &mut invalid, &mut cycle)
                .is_err()
        );
        assert!(
            owner
                .apply_tail_with_workspace(1, &[f64::NAN; 3], &mut invalid, &mut cycle)
                .is_err()
        );
        assert!(
            owner
                .apply_tail_with_workspace(1, &rhs, &mut invalid, &mut wrong)
                .is_err()
        );
        assert_eq!(invalid, [17.; 3]);
        let mut short = [19.; 2];
        assert!(
            owner
                .apply_tail_with_workspace(1, &rhs, &mut short, &mut cycle)
                .is_err()
        );
        assert_eq!(short, [19.; 2]);
    }
    no_events(GLOBAL.stats() - before);
    assert_eq!(screen.retained_payload_bytes()?, retained);
    assert_eq!(cycle.retained_payload_bytes()?, original_cycle_bytes);
    let before = GLOBAL.stats();
    drop(screen);
    let delta = GLOBAL.stats() - before;
    assert_eq!(delta.allocations, 0);
    assert_eq!(delta.reallocations, 0);
    assert_eq!(delta.deallocations, 2);
    assert_eq!(delta.bytes_deallocated, retained);
    owner.apply_tail_with_workspace(1, &rhs, &mut out, &mut cycle)?;
    assert_eq!(out, expected);
    println!(
        "actual recursive screen: two arrays, three coefficient vectors; first/repeat32 screens, rejection, numerical failure, recovery and tail applications allocate zero; unchanged cycle payload and exact release"
    );
    Ok(())
}
