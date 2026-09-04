#![cfg(all(feature = "cmg", feature = "lsmr"))]
//! Integration tests for the production-shaped pair-CMG Schwarz adapter.

use multiway_mg::{
    FactorPair, PairCmgOptions, PairCmgPreconditioner, PairCmgSchwarzOptions,
    PairCmgSchwarzPreconditioner, Preconditioner, ThreeWayProblem,
};

fn connected_problem(size: usize, dynamic_range: bool) -> ThreeWayProblem {
    let counts = [size, size, size];
    let mut tuples = Vec::new();
    let mut weights = Vec::new();
    for index in 0..size {
        for step in 0..3 {
            let a = index;
            let b = (index + step) % size;
            let c = (2 * index + step) % size;
            tuples.push([a as u32, b as u32, c as u32]);
            let exponent = if dynamic_range {
                ((index * 7 + step * 11) % 17) as i32 - 8
            } else {
                0
            };
            weights.push(10.0_f64.powi(exponent));
        }
    }
    ThreeWayProblem::from_observations(counts, &tuples, &weights).expect("valid connected problem")
}

fn projected_vector(problem: &ThreeWayProblem, phase: f64) -> Vec<f64> {
    let mut values: Vec<f64> = (0..problem.dimension())
        .map(|index| (phase + index as f64 * 0.173).sin() + (index as f64 * 0.071).cos())
        .collect();
    problem
        .components()
        .project_structural_range(&mut values)
        .expect("structural projection");
    values
}

fn dot(left: &[f64], right: &[f64]) -> f64 {
    left.iter().zip(right).map(|(a, b)| a * b).sum()
}

fn relative_difference(left: &[f64], right: &[f64]) -> f64 {
    let numerator = left
        .iter()
        .zip(right)
        .map(|(a, b)| (a - b) * (a - b))
        .sum::<f64>()
        .sqrt();
    let denominator = left
        .iter()
        .chain(right)
        .map(|value| value * value)
        .sum::<f64>()
        .sqrt()
        .max(f64::MIN_POSITIVE);
    numerator / denominator
}

#[test]
fn component_adapter_matches_original_pair_cmg_reference() {
    let problem = connected_problem(18, false);
    let cmg = cmg::CmgOptions {
        direct_threshold: 4,
        ..cmg::CmgOptions::default()
    };
    let reference = PairCmgPreconditioner::build(
        problem.clone(),
        PairCmgOptions {
            cmg,
            ..PairCmgOptions::default()
        },
    )
    .expect("reference pair CMG");
    let adapter = PairCmgSchwarzPreconditioner::build_all(
        problem.clone(),
        PairCmgSchwarzOptions {
            cmg,
            reduction: schwarz_precond::ReductionStrategy::AtomicScatter,
            ..PairCmgSchwarzOptions::default()
        },
    )
    .expect("component pair CMG");

    let rhs = projected_vector(&problem, 0.3);
    let mut expected = vec![0.0; problem.dimension()];
    let mut actual = vec![0.0; problem.dimension()];
    reference
        .apply(&rhs, &mut expected)
        .expect("reference apply");
    adapter.apply(&rhs, &mut actual).expect("adapter apply");
    assert!(
        relative_difference(&expected, &actual) < 5.0e-12,
        "component adapter must preserve the original one-cycle pair action"
    );
    assert_eq!(adapter.fallback_workspace_allocations(), 0);
}

#[test]
fn fixed_two_cycle_adapter_is_linear_symmetric_positive_and_range_preserving() {
    let problem = connected_problem(40, true);
    let adapter = PairCmgSchwarzPreconditioner::build_all(
        problem.clone(),
        PairCmgSchwarzOptions {
            cmg: cmg::CmgOptions {
                direct_threshold: 4,
                ..cmg::CmgOptions::default()
            },
            fixed_cycles: 2,
            reduction: schwarz_precond::ReductionStrategy::AtomicScatter,
            ..PairCmgSchwarzOptions::default()
        },
    )
    .expect("two-cycle adapter");
    let x = projected_vector(&problem, 0.2);
    let y = projected_vector(&problem, 1.7);
    let a: f64 = -0.41;
    let b: f64 = 1.23;
    let combination: Vec<f64> = x
        .iter()
        .zip(&y)
        .map(|(&xi, &yi)| a.mul_add(xi, b * yi))
        .collect();
    let mut mx = vec![0.0; problem.dimension()];
    let mut my = vec![0.0; problem.dimension()];
    let mut mcombination = vec![0.0; problem.dimension()];
    adapter.apply(&x, &mut mx).expect("Mx");
    adapter.apply(&y, &mut my).expect("My");
    adapter
        .apply(&combination, &mut mcombination)
        .expect("M(ax+by)");
    let expected: Vec<f64> = mx
        .iter()
        .zip(&my)
        .map(|(&mxi, &myi)| a.mul_add(mxi, b * myi))
        .collect();
    assert!(relative_difference(&expected, &mcombination) < 2.0e-11);

    let xy = dot(&x, &my);
    let yx = dot(&mx, &y);
    let symmetry_scale = xy.abs().max(yx.abs()).max(1.0);
    assert!((xy - yx).abs() / symmetry_scale < 2.0e-10);
    assert!(dot(&x, &mx) > 0.0);
    assert!(
        problem
            .components()
            .maximum_structural_defect(&mx)
            .expect("range defect")
            < 2.0e-9
    );
    assert_eq!(adapter.fallback_workspace_allocations(), 0);
}

#[test]
fn component_splitting_and_selected_pair_are_reported_truthfully() {
    let counts = [6, 6, 4];
    let tuples = vec![
        [0, 0, 0],
        [1, 1, 0],
        [2, 2, 1],
        [3, 3, 1],
        [4, 4, 2],
        [5, 5, 3],
        [0, 1, 2],
        [2, 3, 3],
        [4, 5, 0],
    ];
    let weights = vec![1.0; tuples.len()];
    let problem = ThreeWayProblem::from_observations(counts, &tuples, &weights).expect("problem");
    let adapter = PairCmgSchwarzPreconditioner::build(
        problem,
        &[FactorPair::OneTwo],
        PairCmgSchwarzOptions::default(),
    )
    .expect("selected pair adapter");
    assert_eq!(adapter.selected_pairs(), &[FactorPair::OneTwo]);
    assert!(adapter.component_reports().len() > 1);
    assert!(
        adapter
            .component_reports()
            .iter()
            .all(|report| report.pair() == FactorPair::OneTwo)
    );
    assert!(adapter.memory_report().cmg_preconditioner_bytes() > 0);
    assert!(adapter.memory_report().cmg_workspace_pool_bytes() > 0);
}

#[test]
fn invalid_cycle_count_is_rejected_before_setup() {
    let problem = connected_problem(8, false);
    let error = PairCmgSchwarzPreconditioner::build_all(
        problem,
        PairCmgSchwarzOptions {
            fixed_cycles: 0,
            ..PairCmgSchwarzOptions::default()
        },
    )
    .expect_err("zero cycles must fail");
    assert!(error.to_string().contains("pair_cmg_fixed_cycles"));
}
