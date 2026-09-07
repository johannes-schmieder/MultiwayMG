use super::*;
use multiway_incidence::{
    FactorAggregation, HierarchyWeightFrames, PreparedHierarchyTopology, PreparedThreeWayTopology,
    ThreeWayWeightFrame, WeightFrameInput,
};
use std::panic::{AssertUnwindSafe, catch_unwind};
const B: PreparedHierarchyBudget = PreparedHierarchyBudget::UNLIMITED;
macro_rules! fixture {
    ($t:ident,$f:ident,$h:ident,$frames:ident,$owner:ident,$cycle:ident) => {
        let keys: Vec<_> = (0..2)
            .flat_map(|a| (0..2).flat_map(move |b| (0..2).map(move |c| [a, b, c])))
            .collect();
        let $t = PreparedThreeWayTopology::try_from_collapsed([2; 3], &keys).unwrap();
        let $f = ThreeWayWeightFrame::try_new(&$t, WeightFrameInput::UnitTuples).unwrap();
        let $h = PreparedHierarchyTopology::try_new(
            &$t,
            vec![FactorAggregation::consecutive_halving([2; 3]).unwrap()],
        )
        .unwrap();
        let $frames = HierarchyWeightFrames::try_new(&$h, &$f).unwrap();
        let $owner = PreparedMapHierarchy::try_new(&$frames, 1e-12).unwrap();
        let mut $cycle = $owner.application_workspace().unwrap();
    };
}
fn options() -> CycleQualityOptions {
    CycleQualityOptions {
        test_vectors: 2,
        power_iterations: 4,
        tail_iterations: 2,
        ..Default::default()
    }
}
fn criteria() -> CycleQualityCriteria {
    CycleQualityCriteria {
        maximum_estimated_energy_factor: 1e10,
        maximum_observed_energy_factor: Some(1e10),
        maximum_structural_defect: 1e10,
    }
}
#[test]
fn two_reservations_fail_or_unwind_and_keep_original_owners_usable() {
    fixture!(t, f, h, frames, owner, cycle);
    let mut count = 0;
    PreparedCycleScreenWorkspace::build_with(&owner, &cycle, B, &mut |_| {
        count += 1;
        Ok(())
    })
    .unwrap();
    assert_eq!(count, 2);
    for failing in 0..2 {
        for unwind in [false, true] {
            let mut count = 0;
            let result = catch_unwind(AssertUnwindSafe(|| {
                PreparedCycleScreenWorkspace::build_with(&owner, &cycle, B, &mut |_| {
                    let index = count;
                    count += 1;
                    if index == failing {
                        assert!(!unwind, "injected screen unwind");
                        Err(numerical("injected reservation error"))
                    } else {
                        Ok(())
                    }
                })
            }));
            if unwind {
                assert!(result.is_err());
            } else {
                assert!(result.unwrap().is_err());
            }
            assert_eq!(count, failing + 1);
            assert_eq!(f.weights(), &[1.; 8]);
            let mut retry = PreparedCycleScreenWorkspace::try_new(&owner, &cycle, B).unwrap();
            assert!(
                retry
                    .screen(options(), criteria(), &mut cycle)
                    .unwrap()
                    .accepted
            );
        }
    }
    assert!(size_of::<PreparedCycleScreenFailure>() < 128);
}
#[test]
fn exact_budgets_checked_overflow_and_static_failures_precede_mutation() {
    fixture!(t, f, h, frames, owner, cycle);
    let budget = PreparedHierarchyBudget {
        additional_live_payload_bytes: 123,
        ..B
    };
    let setup = PreparedCycleScreenWorkspace::setup_payload_report(&owner, &cycle, budget).unwrap();
    assert_eq!(
        setup.existing_live_payload_bytes,
        owner
            .payload_report(&cycle, 123)
            .unwrap()
            .total_payload_bytes
    );
    assert_eq!(
        setup.new_arrays_payload_bytes,
        24 * owner.dimension() + 2 * size_of::<PreparedLevelCycleQuality>()
    );
    for maximum in [0, setup.total_payload_bound - 1] {
        assert!(
            PreparedCycleScreenWorkspace::build_with(
                &owner,
                &cycle,
                PreparedHierarchyBudget {
                    maximum_payload_bytes: maximum,
                    ..budget
                },
                &mut |_| panic!("not admitted")
            )
            .is_err()
        );
    }
    assert!(
        PreparedCycleScreenWorkspace::build_with(
            &owner,
            &cycle,
            PreparedHierarchyBudget {
                additional_live_payload_bytes: usize::MAX,
                ..B
            },
            &mut |_| panic!("overflow")
        )
        .is_err()
    );
    let mut screen = PreparedCycleScreenWorkspace::try_new(
        &owner,
        &cycle,
        PreparedHierarchyBudget {
            maximum_payload_bytes: setup.total_payload_bound,
            ..budget
        },
    )
    .unwrap();
    screen.screen(options(), criteria(), &mut cycle).unwrap();
    let reports = screen.reports.clone();
    let arena = screen.arena.clone();
    let cycle_bytes = cycle.retained_payload_bytes().unwrap();
    let o = options();
    let c = criteria();
    for bad in [
        CycleQualityOptions {
            test_vectors: 0,
            ..o
        },
        CycleQualityOptions {
            test_vectors: 17,
            ..o
        },
        CycleQualityOptions {
            power_iterations: 0,
            ..o
        },
        CycleQualityOptions {
            power_iterations: 65,
            ..o
        },
        CycleQualityOptions {
            tail_iterations: 0,
            ..o
        },
        CycleQualityOptions {
            tail_iterations: 5,
            ..o
        },
        CycleQualityOptions {
            correction_damping: 0.,
            ..o
        },
        CycleQualityOptions {
            correction_damping: f64::NAN,
            ..o
        },
        CycleQualityOptions {
            relative_zero_tolerance: f64::INFINITY,
            ..o
        },
        CycleQualityOptions {
            relative_zero_tolerance: -1.,
            ..o
        },
    ] {
        let e = screen.screen(bad, c, &mut cycle).unwrap_err();
        assert_eq!(e.work, Default::default());
        assert_eq!(e.level, None);
        assert_eq!(screen.reports, reports);
        assert_eq!(screen.completed_level_reports(), reports);
        assert_eq!(screen.arena, arena);
    }
    for bad in [
        CycleQualityCriteria {
            maximum_estimated_energy_factor: 0.,
            ..c
        },
        CycleQualityCriteria {
            maximum_estimated_energy_factor: f64::NAN,
            ..c
        },
        CycleQualityCriteria {
            maximum_observed_energy_factor: Some(f64::INFINITY),
            ..c
        },
        CycleQualityCriteria {
            maximum_observed_energy_factor: Some(0.),
            ..c
        },
        CycleQualityCriteria {
            maximum_structural_defect: -1.,
            ..c
        },
        CycleQualityCriteria {
            maximum_structural_defect: f64::NAN,
            ..c
        },
    ] {
        let e = screen.screen(o, bad, &mut cycle).unwrap_err();
        assert_eq!(e.work, Default::default());
        assert_eq!(screen.reports, reports);
        assert_eq!(screen.completed_level_reports(), reports);
        assert_eq!(screen.arena, arena);
    }
    let other = PreparedMapHierarchy::try_new(&frames, 1e-12).unwrap();
    let mut wrong = other.application_workspace().unwrap();
    assert!(
        PreparedCycleScreenWorkspace::build_with(&owner, &wrong, B, &mut |_| panic!("wrong owner"))
            .is_err()
    );
    let e = screen.screen(o, c, &mut wrong).unwrap_err();
    assert_eq!(e.work, Default::default());
    assert_eq!(screen.reports, reports);
    assert_eq!(screen.arena, arena);
    let failed = screen
        .screen(
            CycleQualityOptions {
                correction_damping: f64::MAX,
                ..o
            },
            c,
            &mut cycle,
        )
        .unwrap_err();
    assert!(failed.work.cycle_applications > 0);
    assert!(failed.work.energy_evaluations > 0);
    assert!(matches!(
        failed.source,
        MultiwayError::NumericalFailure { .. }
    ));
    let recovered = screen.screen(o, c, &mut cycle).unwrap();
    assert_eq!(recovered.levels, reports);
    assert_eq!(cycle.retained_payload_bytes().unwrap(), cycle_bytes);
    // Exercise every factor-history slot at the inclusive count caps.
    let full = screen
        .screen(
            CycleQualityOptions {
                test_vectors: 16,
                power_iterations: 64,
                tail_iterations: 64,
                correction_damping: 0.01,
                ..o
            },
            c,
            &mut cycle,
        )
        .unwrap();
    assert!(full.accepted);
    assert_eq!(full.work.cycle_applications, 16 * 64 * frames.level_count());
}
