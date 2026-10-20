//! Algebraic checks for the frozen public `within` comparator wrapper.
#![cfg(feature = "within-comparator")]

use multiway_mg::{
    MultiwayError, Preconditioner, ThreeWayProblem, WithinApproxCholOptions,
    WithinApproxCholPreconditioner,
};
use within::{Design, Effect, PreconditionerConfig, SetupLimit, SetupReport, Solver};

fn problem() -> ThreeWayProblem {
    let tuples = vec![
        [0, 0, 0],
        [0, 1, 1],
        [1, 0, 1],
        [1, 1, 0],
        [2, 1, 1],
        [2, 2, 0],
        [3, 2, 1],
        [3, 0, 0],
    ];
    let weights = vec![1.0e-6, 0.03, 1.0, 7.0, 91.0, 2.0e3, 4.0e5, 8.0e7];
    ThreeWayProblem::from_observations([4, 3, 2], &tuples, &weights).unwrap()
}

fn rhs(problem: &ThreeWayProblem, phase: f64) -> Vec<f64> {
    let mut values: Vec<f64> = (0..problem.dimension())
        .map(|index| ((index as f64 + 1.0) * phase).sin())
        .collect();
    problem
        .components()
        .project_structural_range(&mut values)
        .unwrap();
    values
}

fn dot(left: &[f64], right: &[f64]) -> f64 {
    left.iter().zip(right).map(|(&x, &y)| x * y).sum()
}

fn max_abs(values: &[f64]) -> f64 {
    values.iter().copied().map(f64::abs).fold(0.0, f64::max)
}

#[test]
fn wrapper_matches_the_public_within_preconditioner() {
    let problem = problem();
    let options = WithinApproxCholOptions::default();
    let wrapped = WithinApproxCholPreconditioner::build(problem.clone(), options.clone()).unwrap();

    let mut levels: [Vec<u32>; 3] = std::array::from_fn(|_| Vec::new());
    for tuple in problem.topology().tuples() {
        for factor in 0..3 {
            levels[factor].push(tuple[factor]);
        }
    }
    let effects = levels
        .iter()
        .map(|codes| Effect::new(codes, true, std::iter::empty::<&[f64]>()).unwrap())
        .collect::<Vec<_>>();
    let solver = Solver::new(
        effects,
        Some(problem.weights().to_vec()),
        PreconditionerConfig::Additive {
            local_solver: options.local_solver,
            reduction: options.reduction,
        },
    )
    .unwrap();
    let direct = solver.preconditioner().unwrap();

    let x = rhs(&problem, 0.41);
    let mut expected = vec![0.0; problem.dimension()];
    direct.apply(&x, &mut expected).unwrap();
    problem
        .components()
        .project_structural_range(&mut expected)
        .unwrap();
    let mut actual = vec![0.0; problem.dimension()];
    wrapped.apply(&x, &mut actual).unwrap();

    let difference: Vec<f64> = actual.iter().zip(&expected).map(|(&a, &b)| a - b).collect();
    assert!(max_abs(&difference) < 2.0e-12 * max_abs(&expected).max(1.0));
    assert_eq!(
        wrapped.warnings(),
        solver
            .warnings()
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>()
    );
    assert_eq!(wrapped.projection_fallback_allocations(), 0);
}

#[test]
fn comparator_is_numerically_linear_symmetric_and_positive() {
    let problem = problem();
    let preconditioner =
        WithinApproxCholPreconditioner::build(problem.clone(), WithinApproxCholOptions::default())
            .unwrap();
    let x = rhs(&problem, 0.29);
    let y = rhs(&problem, 0.73);
    let a: f64 = -1.7;
    let b: f64 = 0.45;
    let combination: Vec<f64> = x
        .iter()
        .zip(&y)
        .map(|(&xi, &yi)| a.mul_add(xi, b * yi))
        .collect();
    let mut mx = vec![0.0; problem.dimension()];
    let mut my = vec![0.0; problem.dimension()];
    let mut mcombination = vec![0.0; problem.dimension()];
    preconditioner.apply(&x, &mut mx).unwrap();
    preconditioner.apply(&y, &mut my).unwrap();
    preconditioner
        .apply(&combination, &mut mcombination)
        .unwrap();

    let linearity = mcombination
        .iter()
        .zip(mx.iter().zip(&my))
        .map(|(&actual, (&mxi, &myi))| (actual - a.mul_add(mxi, b * myi)).abs())
        .fold(0.0, f64::max);
    let linearity_scale = max_abs(&mcombination)
        .max(max_abs(&mx))
        .max(max_abs(&my))
        .max(1.0);
    assert!(linearity / linearity_scale < 3.0e-13);

    let symmetry_defect = (dot(&x, &my) - dot(&mx, &y)).abs();
    let symmetry_scale = dot(&x, &my).abs().max(dot(&mx, &y).abs()).max(1.0);
    assert!(symmetry_defect / symmetry_scale < 3.0e-13);
    assert!(dot(&x, &mx) > 0.0);

    let memory = preconditioner.memory_report();
    assert!(memory.problem_state_bytes_estimate() > 0);
    assert!(memory.projection_workspace_bytes() > 0);
    assert_eq!(memory.within_retained_bytes(), None);
    let factors = memory
        .within_local_factor_bytes()
        .expect("factor capacity fits usize");
    assert!(factors > 0, "every pair domain retains a factor");
    assert_eq!(
        memory.known_retained_bytes_estimate(),
        memory.problem_state_bytes_estimate() + memory.projection_workspace_bytes(),
        "the factor inventory is reported separately, not added to the estimate"
    );
}

/// Canonical unique tuples with positive weights spanning `2^±20`, one shared factor
/// level per block so the pair graphs split into several components.
fn blocked_problem() -> ThreeWayProblem {
    let mut tuples = Vec::new();
    let mut weights = Vec::new();
    let mut state = 0x6d77_u64;
    let mut next = |bound: u64| {
        state = state
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        ((state >> 33) % bound) as u32
    };
    for block in 0..3 {
        // Cover every level of the block, then add random tuples within it.
        for level in 0..12 {
            tuples.push([
                12 * block + level,
                8 * block + level % 8,
                5 * block + level % 5,
            ]);
        }
        for _ in 0..60 {
            tuples.push([
                12 * block + next(12),
                8 * block + next(8),
                5 * block + next(5),
            ]);
        }
    }
    tuples.sort_unstable();
    tuples.dedup();
    for _ in &tuples {
        weights.push(2.0_f64.powi(next(41) as i32 - 20));
    }
    ThreeWayProblem::from_observations([36, 24, 15], &tuples, &weights).unwrap()
}

/// The `within` preconditioner both ways from the comparator's input shape.
fn within_both_ways(
    problem: &ThreeWayProblem,
    options: &WithinApproxCholOptions,
) -> (within::Preconditioner, within::Preconditioner, SetupReport) {
    let mut levels: [Vec<u32>; 3] = std::array::from_fn(|_| Vec::new());
    for tuple in problem.topology().tuples() {
        for factor in 0..3 {
            levels[factor].push(tuple[factor]);
        }
    }
    let effects = || {
        levels
            .iter()
            .map(|codes| Effect::new(codes, true, std::iter::empty::<&[f64]>()).unwrap())
            .collect::<Vec<_>>()
    };
    let solver = Solver::new(
        effects(),
        Some(problem.weights().to_vec()),
        PreconditionerConfig::Additive {
            local_solver: options.local_solver.clone(),
            reduction: options.reduction,
        },
    )
    .unwrap();
    let design = Design::new(effects()).unwrap();
    let (checked, report) = within::Preconditioner::build_checked(
        &design,
        Some(problem.weights()),
        &options.local_solver,
        options.reduction,
        SetupLimit::UNLIMITED,
    )
    .unwrap();
    (
        solver.preconditioner().unwrap().clone(),
        checked.unwrap(),
        report,
    )
}

fn serial_action(preconditioner: &within::Preconditioner, x: &[f64]) -> Vec<f64> {
    let bytes = preconditioner
        .serial_workspace_required_payload_bytes()
        .unwrap();
    let mut workspace = preconditioner.try_serial_workspace(bytes).unwrap();
    let mut y = vec![0.0; x.len()];
    workspace.apply(x, &mut y).unwrap();
    y
}

#[test]
fn checked_within_setup_reproduces_the_solver_route_bit_for_bit() {
    let schur_modes = [within::SchurMode::default(), within::SchurMode::Exact];
    for problem in [problem(), blocked_problem()] {
        for schur in schur_modes.clone() {
            let options = WithinApproxCholOptions {
                local_solver: within::LocalSolverConfig {
                    schur,
                    ..within::LocalSolverConfig::default()
                },
                ..WithinApproxCholOptions::default()
            };
            let (reference, checked, report) = within_both_ways(&problem, &options);
            assert!(report.peak_payload_bytes >= report.retained_payload_bytes);
            for phase in [0.41, 1.3, 2.9] {
                let x = rhs(&problem, phase);
                let expected = serial_action(&reference, &x);
                let actual = serial_action(&checked, &x);
                for (a, b) in actual.iter().zip(&expected) {
                    assert_eq!(a.to_bits(), b.to_bits());
                }
            }
        }
    }
}

#[test]
fn checked_comparator_reports_complete_within_payload_and_refuses_inclusively() {
    for problem in [problem(), blocked_problem()] {
        let options = WithinApproxCholOptions::default();
        let unchecked =
            WithinApproxCholPreconditioner::build(problem.clone(), options.clone()).unwrap();
        let checked = WithinApproxCholPreconditioner::build_checked(
            problem.clone(),
            options.clone(),
            SetupLimit::UNLIMITED,
        )
        .unwrap();
        let (_, _, report) = within_both_ways(&problem, &options);

        let memory = checked.memory_report();
        assert_eq!(
            memory.within_retained_bytes(),
            Some(report.retained_payload_bytes)
        );
        assert_eq!(
            memory.within_setup_peak_bytes(),
            Some(report.peak_payload_bytes)
        );
        // Capacities are not part of a factor's identity: the unaccounted builder
        // may keep amortized spare capacity that the checked one does not.
        let factors = memory.within_local_factor_bytes().unwrap();
        assert!(factors > 0);
        assert!(report.retained_payload_bytes > factors);
        assert_eq!(unchecked.memory_report().within_setup_peak_bytes(), None);
        assert_eq!(
            memory.known_retained_bytes_estimate(),
            unchecked.memory_report().known_retained_bytes_estimate()
        );
        assert!(checked.warnings().is_empty());
        assert_eq!(checked.warnings(), unchecked.warnings());

        for phase in [0.29, 0.73] {
            let x = rhs(&problem, phase);
            let mut a = vec![0.0; problem.dimension()];
            let mut b = vec![0.0; problem.dimension()];
            unchecked.apply(&x, &mut a).unwrap();
            checked.apply(&x, &mut b).unwrap();
            let difference: Vec<f64> = a.iter().zip(&b).map(|(&u, &v)| u - v).collect();
            assert!(max_abs(&difference) <= 1.0e-12 * max_abs(&a).max(1.0));
        }

        let peak = report.peak_payload_bytes;
        let at_peak = WithinApproxCholPreconditioner::build_checked(
            problem.clone(),
            options.clone(),
            SetupLimit {
                maximum_payload_bytes: peak,
            },
        )
        .unwrap();
        assert_eq!(
            at_peak.memory_report().within_setup_peak_bytes(),
            Some(peak)
        );
        match WithinApproxCholPreconditioner::build_checked(
            problem.clone(),
            options,
            SetupLimit {
                maximum_payload_bytes: peak - 1,
            },
        ) {
            Err(MultiwayError::WithinSetupBudgetExceeded {
                required, limit, ..
            }) => {
                assert_eq!(limit, peak - 1);
                assert!(required > limit && required <= peak);
            }
            other => panic!("one byte under the peak must be refused: {other:?}"),
        }
    }
}
