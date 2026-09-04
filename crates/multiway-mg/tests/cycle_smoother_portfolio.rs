#![cfg(feature = "cmg")]

//! Regression tests for the selective complete-cycle smoother portfolio.

#[path = "../examples/support/issue3_cycle_fixtures.rs"]
mod issue3_cycle_fixtures;

use issue3_cycle_fixtures::{CycleHoldoutFixture, cycle_holdout_fixtures};
use multiway_mg::{
    AggregationRepairOptions, BootstrapAggregationOptions, CompatibleRelaxationCriteria,
    CompatibleRelaxationOptions, CycleQualityCriteria, CycleQualityOptions, CycleSmootherKind,
    CycleSmootherPortfolioOptions, CycleSmootherPortfolioStopReason, DiagonalPreconditioner,
    PairCmgOptions, Preconditioner, build_cycle_smoother_portfolio,
};

#[test]
fn portfolio_selects_map_then_pair_fallback_and_rejects_inadmissible_cycles() {
    let fixtures = cycle_holdout_fixtures().expect("frozen fixtures build");
    let latin = fixture(&fixtures, "cover-latin-seed-700");
    let nearly_nested = fixture(&fixtures, "cover-nearly-nested-seed-704");
    let weak_chain = fixture(&fixtures, "cover-weak-chain-seed-702");

    let latin_result = build(latin);
    assert!(latin_result.accepted());
    assert_eq!(
        latin_result.selected_smoother(),
        Some(CycleSmootherKind::SymmetricMap)
    );
    assert_eq!(
        latin_result.stop_reason(),
        CycleSmootherPortfolioStopReason::AcceptedSymmetricMap
    );
    assert!(latin_result.pair_pass().is_none());
    assert!(latin_result
        .build_selected_cycle(&latin.problem)
        .expect("selected MAP cycle builds")
        .is_some());

    let nested_result = build(nearly_nested);
    assert!(nested_result.accepted());
    assert!(!nested_result.map_pass().accepted());
    assert!(nested_result
        .pair_pass()
        .expect("pair fallback was evaluated")
        .accepted());
    assert_eq!(
        nested_result.selected_smoother(),
        Some(CycleSmootherKind::AllPairsCmg)
    );
    assert_eq!(
        nested_result.stop_reason(),
        CycleSmootherPortfolioStopReason::AcceptedAllPairsCmg
    );
    let nested_cycle = nested_result
        .build_selected_cycle(&nearly_nested.problem)
        .expect("selected pair-CMG cycle builds")
        .expect("accepted portfolio has a cycle");
    assert_eq!(nested_cycle.dimension(), nearly_nested.problem.dimension());

    let weak_result = build(weak_chain);
    assert!(!weak_result.accepted());
    assert!(!weak_result.map_pass().accepted());
    assert!(!weak_result
        .pair_pass()
        .expect("pair fallback was evaluated")
        .accepted());
    assert_eq!(weak_result.selected_smoother(), None);
    assert_eq!(
        weak_result.stop_reason(),
        CycleSmootherPortfolioStopReason::NoAcceptedCycle
    );
    assert!(weak_result
        .build_selected_cycle(&weak_chain.problem)
        .expect("rejected portfolio is a valid result")
        .is_none());
}

fn fixture<'a>(fixtures: &'a [CycleHoldoutFixture], name: &str) -> &'a CycleHoldoutFixture {
    fixtures
        .iter()
        .find(|fixture| fixture.name == name)
        .expect("requested frozen fixture exists")
}

fn build(
    fixture: &CycleHoldoutFixture,
) -> multiway_mg::CycleSmootherPortfolioResult {
    let primary_smoother = DiagonalPreconditioner::new(&fixture.problem, 0.5)
        .expect("primary Jacobi smoother builds");
    build_cycle_smoother_portfolio(
        &fixture.problem,
        &primary_smoother,
        portfolio_options(),
    )
    .expect("selective cycle portfolio builds")
    .0
}

fn portfolio_options() -> CycleSmootherPortfolioOptions {
    CycleSmootherPortfolioOptions {
        bootstrap: BootstrapAggregationOptions {
            setup_test_vectors: 5,
            setup_sweeps: 5,
            setup_jacobi_omega: 0.5,
            maximum_neighbor_degree: 12,
            signature_window: 3,
            maximum_candidate_degree: 12,
            minimum_combined_affinity: 0.40,
            algebraic_affinity_weight: 0.75,
            structural_affinity_weight: 0.05,
            degree_affinity_weight: 0.10,
            signature_hit_weight: 0.10,
            structural_baseline_required_factor_ratio: 0.90,
            structural_baseline_maximum_dimension_overhead_ratio: 0.05,
            structural_baseline_maximum_tuple_overhead_ratio: 0.05,
            compatible_relaxation: compatible_options(),
            compatible_criteria: compatible_criteria(),
            maximum_bootstrap_witnesses: 6,
            maximum_coarse_dimension_ratio: 0.80,
            minimum_tuple_reduction: 0.05,
            maximum_two_level_tuple_complexity: 1.95,
            split_repair: Some(AggregationRepairOptions {
                relaxation: compatible_options(),
                criteria: compatible_criteria(),
                maximum_rounds: 18,
                maximum_coarse_dimension_ratio: 0.80,
                minimum_tuple_reduction: 0.05,
                maximum_two_level_tuple_complexity: 1.95,
                minimum_split_score_fraction: 0.001,
            }),
            seed: 0x4d57_4d47_434f_5645,
        },
        probe: CycleQualityOptions {
            test_vectors: 12,
            power_iterations: 24,
            tail_iterations: 6,
            correction_damping: 1.0,
            seed: 0x4d57_4d47_4359_4331,
            relative_zero_tolerance: 1.0e-13,
        },
        criteria: CycleQualityCriteria {
            maximum_estimated_energy_factor: 0.50,
            maximum_observed_energy_factor: Some(1.05),
            maximum_structural_defect: 1.0e-10,
        },
        smoothing_steps: 1,
        smoother_damping: 1.0,
        terminal_relative_tolerance: 1.0e-12,
        pair_cmg: PairCmgOptions::default(),
    }
}

fn compatible_options() -> CompatibleRelaxationOptions {
    CompatibleRelaxationOptions {
        test_vectors: 16,
        sweeps: 12,
        relaxation_damping: 1.0,
        seed: 0x4d57_4d47_4352_3031,
        relative_zero_tolerance: 1.0e-13,
    }
}

fn compatible_criteria() -> CompatibleRelaxationCriteria {
    CompatibleRelaxationCriteria {
        maximum_diagonal_factor_per_sweep: 0.85,
        maximum_energy_factor_per_sweep: Some(0.85),
        maximum_final_coarse_defect: 1.0e-10,
        maximum_final_structural_defect: 1.0e-10,
    }
}
