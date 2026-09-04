//! Identical-domain tests for the issue-4 pair-local solver duel.
#![cfg(all(feature = "cmg", feature = "within-comparator"))]

use multiway_mg::{
    PairDomain, PairExactOptions, PairExactPseudoinverse, PairLocalAnalysisOptions,
    PairLocalCmgOptions, PairLocalCmgPreconditioner, PairLocalWithinPreconditioner,
    WithinApproxCholOptions, analyze_pair_local,
};

fn path_domain(vertices_per_side: usize, dynamic_range: bool) -> PairDomain {
    let mut edges = Vec::new();
    for index in 0..vertices_per_side {
        let base = if dynamic_range {
            10.0_f64.powi((index as i32 % 9) - 4)
        } else {
            1.0
        };
        edges.push((index as u32, index as u32, base));
        if index + 1 < vertices_per_side {
            edges.push(((index + 1) as u32, index as u32, 0.7 * base));
        }
    }
    PairDomain::from_edges(vertices_per_side, vertices_per_side, edges).unwrap()
}

#[test]
fn domain_aggregates_duplicates_and_projects_the_pair_shift() {
    let domain =
        PairDomain::from_edges(2, 2, [(0, 0, 0.25), (0, 0, 0.75), (1, 0, 2.0), (1, 1, 3.0)])
            .unwrap();
    assert_eq!(domain.edge_count(), 3);
    assert_eq!(domain.cycle_excess(), 0);
    assert_eq!(domain.minimum_degree(), 1);
    assert_eq!(domain.maximum_degree(), 2);
    assert_eq!(domain.edges()[0].weight(), 1.0);

    let mut values = vec![2.0, 4.0, -3.0, 7.0];
    domain.project_range_in_place(&mut values).unwrap();
    let null_dot = values[..2].iter().sum::<f64>() - values[2..].iter().sum::<f64>();
    assert!(null_dot.abs() < 2.0e-15 * values.iter().map(|x| x.abs()).sum::<f64>());
}

#[test]
fn disconnected_or_uncovered_domains_fail_closed() {
    let error = PairDomain::from_edges(2, 2, [(0, 0, 1.0), (1, 1, 1.0)]).unwrap_err();
    assert!(error.to_string().contains("connected"));
    let error = PairDomain::from_edges(3, 2, [(0, 0, 1.0), (1, 0, 1.0), (1, 1, 1.0)]).unwrap_err();
    assert!(error.to_string().contains("cover"));
}

#[test]
fn exact_pair_pseudoinverse_has_unit_preconditioned_spectrum() {
    let domain = path_domain(5, true);
    let exact = PairExactPseudoinverse::build(domain.clone(), PairExactOptions::default()).unwrap();
    let report = analyze_pair_local(&domain, &exact, PairLocalAnalysisOptions::default()).unwrap();
    assert_eq!(report.numerical_nullity(), 1);
    assert_eq!(report.numerical_rank(), domain.dimension() - 1);
    assert!(report.relative_inverse_frobenius_error() < 2.0e-10);
    assert!(report.preconditioned_condition_number() - 1.0 < 2.0e-9);
    assert!(report.unit_inverse_energy_error() < 2.0e-9);
    assert!(report.numerically_linear());
    assert!(report.numerically_symmetric());
    assert!(report.preserves_range());
    assert!(report.positive_on_range());
}

#[test]
fn fixed_cmg_and_frozen_within_are_valid_range_actions() {
    let domain = path_domain(18, true);
    let cmg = PairLocalCmgPreconditioner::build(
        domain.clone(),
        PairLocalCmgOptions {
            cmg: cmg::CmgOptions {
                direct_threshold: 2,
                ..cmg::CmgOptions::default()
            },
            fixed_cycles: 1,
        },
    )
    .unwrap();
    let mut within_options = WithinApproxCholOptions::default();
    within_options.local_solver.dense_threshold = 0;
    let within = PairLocalWithinPreconditioner::build(domain.clone(), within_options).unwrap();
    let options = PairLocalAnalysisOptions {
        maximum_dimension: 128,
        ..PairLocalAnalysisOptions::default()
    };
    let cmg_report = analyze_pair_local(&domain, &cmg, options).unwrap();
    let within_report = analyze_pair_local(&domain, &within, options).unwrap();
    for report in [&cmg_report, &within_report] {
        assert!(report.linearity_defect() < 2.0e-12);
        assert!(report.full_symmetry_defect() < 2.0e-10);
        assert!(report.quotient_symmetry_defect() < 2.0e-10);
        assert!(report.range_leakage() < 2.0e-10);
        assert_eq!(report.positive_action_defect(), 0.0);
        assert!(report.minimum_preconditioned_eigenvalue() > 0.0);
        assert!(report.preconditioned_condition_number().is_finite());
    }
    assert_eq!(cmg.fallback_workspace_allocations(), 0);
    assert_eq!(within.fallback_workspace_allocations(), 0);
    assert!(cmg.memory_report().cmg_preconditioner_bytes() > 0);
    assert_eq!(within.memory_report().within_retained_bytes(), None);
}
