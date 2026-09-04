//! Search sparse graph-cover lifts for informative bootstrap aggregation cases.

#[path = "support/issue2_fixtures.rs"]
mod fixtures;

use fixtures::{DynError, one_level_cases};
use multiway_mg::{
    BootstrapAggregationOptions, CompatibleRelaxationCriteria, CompatibleRelaxationOptions,
    DenseRangeDecomposition, DiagonalPreconditioner, FactorAggregation,
    PairNeighborhoodAggregationOptions, SpectralAnalysisOptions, SymmetricMapPreconditioner,
    SymmetricTwoGridPreconditioner, ThreeWayProblem, build_bootstrap_aggregation,
    build_pair_neighborhood_aggregation,
};

fn main() -> Result<(), DynError> {
    println!(
        "family\tseed\tfine_dimension\tfine_tuples\toracle_condition\tone_shot_available\tone_shot_condition\tbootstrap_condition\tbaseline_condition\tbootstrap_recovery_of_oracle\tone_shot_recovery_of_oracle\toracle_pair_recall\tone_shot_pair_recall\tbootstrap_pair_recall\tone_shot_coarse_dimension\tone_shot_coarse_tuples\tbootstrap_coarse_dimension\tbootstrap_coarse_tuples\tbootstrap_accepted\tbootstrap_rounds\tbootstrap_witnesses\tbootstrap_stop"
    );
    for source in one_level_cases()? {
        if !matches!(
            source.family,
            "planted-communities"
                | "dominant-pair-weak-third"
                | "weak-chain"
                | "nearly-nested"
                | "latin-square"
                | "hub-power-law"
                | "weight-dynamic-range"
        ) {
            continue;
        }
        let source_map = source.maps.first().ok_or("source oracle map missing")?;
        let base = source_map.coarsen(&source.problem)?;
        let mut retained = Vec::new();
        for seed in 0..512_u64 {
            let Some((problem, oracle)) = cover_lift(&base, seed)? else {
                continue;
            };
            let spectral_options = SpectralAnalysisOptions {
                maximum_dimension: 256,
                ..SpectralAnalysisOptions::default()
            };
            let range = DenseRangeDecomposition::from_problem(&problem, spectral_options)?;
            let baseline_preconditioner = SymmetricMapPreconditioner::new(problem.clone());
            let baseline_condition = range
                .analyze(&baseline_preconditioner, spectral_options)?
                .preconditioned_condition_number();
            let oracle_condition = condition(&problem, &oracle, &range, spectral_options)?;
            if oracle_condition > 3.0 || baseline_condition <= 1.05 * oracle_condition {
                continue;
            }

            let one_shot = build_pair_neighborhood_aggregation(
                &problem,
                PairNeighborhoodAggregationOptions::default(),
            )?;
            let one_shot_dimension: usize = one_shot.coarse_counts().iter().sum();
            let one_shot_available = one_shot_dimension < problem.dimension();
            let one_shot_condition = if one_shot_available {
                condition(&problem, &one_shot, &range, spectral_options)?
            } else {
                baseline_condition
            };
            let one_shot_recovery = recovery(
                baseline_condition,
                oracle_condition,
                one_shot_condition,
            );
            if one_shot_recovery >= 0.80 {
                continue;
            }

            let screen = DiagonalPreconditioner::new(&problem, 0.5)?;
            let bootstrap = build_bootstrap_aggregation(&problem, &screen, bootstrap_options())?;
            let bootstrap_condition = condition(
                &problem,
                bootstrap.final_aggregation(),
                &range,
                spectral_options,
            )?;
            let bootstrap_recovery = recovery(
                baseline_condition,
                oracle_condition,
                bootstrap_condition,
            );
            let one_shot_coarse = one_shot.coarsen(&problem)?;
            let bootstrap_coarse = bootstrap.final_aggregation().coarsen(&problem)?;
            let witnesses = bootstrap
                .rounds()
                .last()
                .map_or(0, |round| round.bootstrap_witnesses());
            let score = bootstrap_recovery - one_shot_recovery;
            retained.push((
                score,
                seed,
                problem.dimension(),
                problem.tuple_count(),
                oracle_condition,
                one_shot_available,
                one_shot_condition,
                bootstrap_condition,
                baseline_condition,
                bootstrap_recovery,
                one_shot_recovery,
                pair_recall(&oracle, &oracle),
                pair_recall(&oracle, &one_shot),
                pair_recall(&oracle, bootstrap.final_aggregation()),
                one_shot_dimension,
                one_shot_coarse.tuple_count(),
                bootstrap
                    .final_aggregation()
                    .coarse_counts()
                    .iter()
                    .sum::<usize>(),
                bootstrap_coarse.tuple_count(),
                bootstrap.accepted(),
                bootstrap.rounds().len(),
                witnesses,
                format!("{:?}", bootstrap.stop_reason()),
            ));
        }
        retained.sort_by(|left, right| {
            right
                .0
                .total_cmp(&left.0)
                .then_with(|| left.1.cmp(&right.1))
        });
        for candidate in retained.into_iter().take(8) {
            println!(
                "{}\t{}\t{}\t{}\t{:.9e}\t{}\t{:.9e}\t{:.9e}\t{:.9e}\t{:.9e}\t{:.9e}\t{:.9e}\t{:.9e}\t{:.9e}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}",
                source.family,
                candidate.1,
                candidate.2,
                candidate.3,
                candidate.4,
                candidate.5,
                candidate.6,
                candidate.7,
                candidate.8,
                candidate.9,
                candidate.10,
                candidate.11,
                candidate.12,
                candidate.13,
                candidate.14,
                candidate.15,
                candidate.16,
                candidate.17,
                candidate.18,
                candidate.19,
                candidate.20,
                candidate.21.replace(['\t', '\n'], " "),
            );
        }
    }
    Ok(())
}

fn condition(
    problem: &ThreeWayProblem,
    aggregation: &FactorAggregation,
    range: &DenseRangeDecomposition,
    spectral_options: SpectralAnalysisOptions,
) -> Result<f64, DynError> {
    let cycle = SymmetricTwoGridPreconditioner::build(
        problem.clone(),
        aggregation.clone(),
        SymmetricMapPreconditioner::new(problem.clone()),
        1,
        1.0,
        1.0e-12,
    )?;
    Ok(range
        .analyze(&cycle, spectral_options)?
        .preconditioned_condition_number())
}

fn cover_lift(
    base: &ThreeWayProblem,
    seed: u64,
) -> Result<Option<(ThreeWayProblem, FactorAggregation)>, DynError> {
    let base_counts = base.topology().level_counts();
    let fine_counts = base_counts.map(|count| count * 2);
    let parents = core::array::from_fn(|factor| {
        (0..fine_counts[factor])
            .map(|level| (level / 2) as u32)
            .collect()
    });
    let oracle = FactorAggregation::new(fine_counts, parents)?;
    let mut tuples = Vec::with_capacity(base.tuple_count() * 2);
    let mut weights = Vec::with_capacity(tuples.capacity());
    for (tuple_index, (&tuple, &weight)) in base
        .topology()
        .tuples()
        .iter()
        .zip(base.weights())
        .enumerate()
    {
        let mixed = mix(seed ^ (tuple_index as u64).wrapping_mul(0x9e37_79b9_7f4a_7c15));
        let second_shift = (mixed & 1) as u32;
        let third_shift = ((mixed >> 1) & 1) as u32;
        let scores = [
            0.75 + ((mixed >> 8) % 17) as f64 / 16.0,
            0.75 + ((mixed >> 16) % 17) as f64 / 16.0,
        ];
        let score_sum = scores[0] + scores[1];
        for child in 0..2_u32 {
            tuples.push([
                2 * tuple[0] + child,
                2 * tuple[1] + (child ^ second_shift),
                2 * tuple[2] + (child ^ third_shift),
            ]);
            weights.push(weight * scores[child as usize] / score_sum);
        }
    }
    let problem = ThreeWayProblem::from_observations(fine_counts, &tuples, &weights)?;
    if !oracle_respects_components(&problem, &oracle) {
        return Ok(None);
    }
    let reconstructed = oracle.coarsen(&problem)?;
    if reconstructed.topology().tuples() != base.topology().tuples() {
        return Ok(None);
    }
    for (&expected, &actual) in base.weights().iter().zip(reconstructed.weights()) {
        if (expected - actual).abs() > 1.0e-12 * expected.abs().max(actual.abs()).max(1.0) {
            return Ok(None);
        }
    }
    Ok(Some((problem, oracle)))
}

fn oracle_respects_components(
    problem: &ThreeWayProblem,
    oracle: &FactorAggregation,
) -> bool {
    let counts = problem.topology().level_counts();
    (0..3).all(|factor| {
        (0..counts[factor]).all(|level| {
            let sibling = level ^ 1;
            oracle.parents(factor)[level] == oracle.parents(factor)[sibling]
                && problem.components().component_of(factor, level)
                    == problem.components().component_of(factor, sibling)
        })
    })
}

fn bootstrap_options() -> BootstrapAggregationOptions {
    BootstrapAggregationOptions {
        setup_test_vectors: 4,
        setup_sweeps: 4,
        setup_jacobi_omega: 0.5,
        maximum_neighbor_degree: 12,
        signature_window: 2,
        maximum_candidate_degree: 12,
        minimum_combined_affinity: 0.40,
        algebraic_affinity_weight: 0.75,
        structural_affinity_weight: 0.05,
        degree_affinity_weight: 0.10,
        signature_hit_weight: 0.10,
        compatible_relaxation: CompatibleRelaxationOptions {
            test_vectors: 12,
            sweeps: 10,
            relaxation_damping: 1.0,
            seed: 0x4d57_4d47_4352_3031,
            relative_zero_tolerance: 1.0e-13,
        },
        compatible_criteria: CompatibleRelaxationCriteria {
            maximum_diagonal_factor_per_sweep: 0.85,
            maximum_energy_factor_per_sweep: Some(0.85),
            maximum_final_coarse_defect: 1.0e-10,
            maximum_final_structural_defect: 1.0e-10,
        },
        maximum_bootstrap_witnesses: 6,
        maximum_coarse_dimension_ratio: 0.80,
        minimum_tuple_reduction: 0.0,
        maximum_two_level_tuple_complexity: 2.0,
        split_repair: None,
        seed: 0x4d57_4d47_434f_5645,
    }
}

fn recovery(baseline: f64, oracle: f64, candidate: f64) -> f64 {
    let denominator = baseline - oracle;
    if denominator <= 1.0e-12 * baseline.abs().max(1.0) {
        0.0
    } else {
        (baseline - candidate) / denominator
    }
}

fn pair_recall(oracle: &FactorAggregation, candidate: &FactorAggregation) -> f64 {
    let counts = oracle.fine_counts();
    let mut oracle_pairs = 0_usize;
    let mut recovered = 0_usize;
    for factor in 0..3 {
        for left in 0..counts[factor] {
            for right in (left + 1)..counts[factor] {
                if oracle.parents(factor)[left] == oracle.parents(factor)[right] {
                    oracle_pairs += 1;
                    recovered += usize::from(
                        candidate.parents(factor)[left] == candidate.parents(factor)[right],
                    );
                }
            }
        }
    }
    if oracle_pairs == 0 {
        1.0
    } else {
        recovered as f64 / oracle_pairs as f64
    }
}

fn mix(mut value: u64) -> u64 {
    value = (value ^ (value >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    value = (value ^ (value >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
    value ^ (value >> 31)
}
