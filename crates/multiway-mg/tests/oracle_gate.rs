#![cfg(feature = "cmg")]

//! Acceptance gate for a known good three-way coarse space.

use multiway_mg::{
    AggregationStrategy, FactorAggregation, HierarchyOptions, HybridPairVcycle, PairCmgOptions,
    PairCmgPreconditioner, SpectralAnalysisOptions, ThreeWayHierarchy, ThreeWayProblem,
    analyze_preconditioner,
};

#[test]
fn oracle_coarse_space_closes_the_residual_pairwise_spectral_gap() {
    let base = planted_communities(4, 0.02);
    let (level_one, coarse_map_one) = refine_once(&base, 2, 0);
    let (finest, coarse_map_zero) = refine_once(&level_one, 2, 1);
    let hierarchy_options = HierarchyOptions {
        max_levels: 2,
        terminal_dimension: base.dimension(),
        minimum_dimension_reduction: 0.0,
        minimum_tuple_reduction: 0.0,
        aggregation: AggregationStrategy::Supplied(vec![coarse_map_zero, coarse_map_one]),
        ..HierarchyOptions::default()
    };
    let spectral_options = SpectralAnalysisOptions {
        maximum_dimension: 64,
        ..SpectralAnalysisOptions::default()
    };

    let pair = PairCmgPreconditioner::build(finest.clone(), PairCmgOptions::default())
        .expect("pair-CMG build succeeds");
    let jacobi_hierarchy = ThreeWayHierarchy::build(finest.clone(), hierarchy_options.clone())
        .expect("oracle Jacobi hierarchy succeeds");
    let hybrid =
        HybridPairVcycle::build(finest.clone(), hierarchy_options, PairCmgOptions::default())
            .expect("oracle hybrid succeeds");

    let pair_report = analyze_preconditioner(&finest, &pair, spectral_options)
        .expect("pair spectral analysis succeeds");
    let jacobi_report = analyze_preconditioner(&finest, &jacobi_hierarchy, spectral_options)
        .expect("Jacobi hierarchy spectral analysis succeeds");
    let hybrid_report = analyze_preconditioner(&finest, &hybrid, spectral_options)
        .expect("hybrid spectral analysis succeeds");

    assert!(pair_report.positive_definite_on_range());
    assert!(jacobi_report.positive_definite_on_range());
    assert!(hybrid_report.positive_definite_on_range());
    assert!(pair_report.preconditioned_condition_number() > 1.5);
    assert!(pair_report.preconditioned_condition_number() < 3.0);
    assert!(jacobi_report.preconditioned_condition_number() < 1.5);
    assert!(
        jacobi_report.preconditioned_condition_number()
            < pair_report.preconditioned_condition_number()
    );
    assert!(hybrid_report.preconditioned_condition_number() < 1.01);
    assert!(hybrid_report.optimal_energy_spectral_radius() < 0.01);
    assert!(
        hybrid_report.preconditioned_condition_number()
            < jacobi_report.preconditioned_condition_number()
    );
    assert_eq!(hybrid_report.negative_preconditioner_directions(), 0);
    assert_eq!(hybrid_report.near_zero_preconditioner_directions(), 0);
}

fn planted_communities(levels: usize, bridge_weight: f64) -> ThreeWayProblem {
    let mut tuples = Vec::new();
    let mut weights = Vec::new();
    for first in 0..levels {
        for second in 0..levels {
            for third in 0..levels {
                tuples.push([first as u32, second as u32, third as u32]);
                let first_group = 2 * first / levels;
                let second_group = 2 * second / levels;
                let third_group = 2 * third / levels;
                let base = if first_group == second_group && second_group == third_group {
                    1.0
                } else {
                    bridge_weight
                };
                weights
                    .push(base * (1.0 + ((3 * first + 5 * second + 7 * third) % 11) as f64 / 20.0));
            }
        }
    }
    ThreeWayProblem::from_observations([levels; 3], &tuples, &weights)
        .expect("planted problem is valid")
}

fn refine_once(
    coarse: &ThreeWayProblem,
    clone_factor: usize,
    refinement: usize,
) -> (ThreeWayProblem, FactorAggregation) {
    let coarse_counts = coarse.topology().level_counts();
    let fine_counts = coarse_counts.map(|count| count * clone_factor);
    let parents = core::array::from_fn(|factor| {
        (0..fine_counts[factor])
            .map(|level| (level / clone_factor) as u32)
            .collect()
    });
    let aggregation =
        FactorAggregation::new(fine_counts, parents).expect("oracle aggregation is valid");
    let children_per_tuple = clone_factor.pow(3);
    let mut tuples = Vec::with_capacity(coarse.tuple_count() * children_per_tuple);
    let mut weights = Vec::with_capacity(tuples.capacity());

    for (tuple_index, (&tuple, &coarse_weight)) in coarse
        .topology()
        .tuples()
        .iter()
        .zip(coarse.weights())
        .enumerate()
    {
        let mut scores = Vec::with_capacity(children_per_tuple);
        let mut score_sum = 0.0;
        for first_child in 0..clone_factor {
            for second_child in 0..clone_factor {
                for third_child in 0..clone_factor {
                    let mixed = tuple_index
                        .wrapping_mul(131)
                        .wrapping_add(refinement.wrapping_mul(47))
                        .wrapping_add(first_child.wrapping_mul(17))
                        .wrapping_add(second_child.wrapping_mul(11))
                        .wrapping_add(third_child.wrapping_mul(5));
                    let score = 0.75 + (mixed % 17) as f64 / 16.0;
                    scores.push(score);
                    score_sum += score;
                }
            }
        }

        let mut child_index = 0;
        for first_child in 0..clone_factor {
            for second_child in 0..clone_factor {
                for third_child in 0..clone_factor {
                    tuples.push([
                        (tuple[0] as usize * clone_factor + first_child) as u32,
                        (tuple[1] as usize * clone_factor + second_child) as u32,
                        (tuple[2] as usize * clone_factor + third_child) as u32,
                    ]);
                    weights.push(coarse_weight * scores[child_index] / score_sum);
                    child_index += 1;
                }
            }
        }
    }
    let fine = ThreeWayProblem::from_observations(fine_counts, &tuples, &weights)
        .expect("refined problem is valid");
    let reconstructed = aggregation
        .coarsen(&fine)
        .expect("oracle coarsening succeeds");
    for (&expected, &actual) in coarse.weights().iter().zip(reconstructed.weights()) {
        assert!((expected - actual).abs() <= 1.0e-12 * expected.abs().max(1.0));
    }
    (fine, aggregation)
}
