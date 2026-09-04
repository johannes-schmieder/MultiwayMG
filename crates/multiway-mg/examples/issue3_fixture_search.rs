//! Search deterministic sparse refinements for informative issue #3 fixtures.

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
        "family\tseed\tfine_dimension\tfine_tuples\toracle_condition\tone_shot_condition\tbootstrap_condition\tone_shot_coarse_dimension\tone_shot_coarse_tuples\tbootstrap_coarse_dimension\tbootstrap_coarse_tuples\tbootstrap_accepted\tbootstrap_rounds\tbootstrap_witnesses\tbootstrap_stop"
    );
    for source in one_level_cases()? {
        if !matches!(
            source.family,
            "planted-communities"
                | "dominant-pair-weak-third"
                | "weak-chain"
                | "nearly-nested"
                | "hub-power-law"
                | "weight-dynamic-range"
        ) {
            continue;
        }
        let oracle_source = source.maps.first().ok_or("source oracle map missing")?;
        let base = oracle_source.coarsen(&source.problem)?;
        let mut retained = Vec::new();
        for seed in 0..384_u64 {
            let Some((problem, oracle)) = random_sparse_refinement(&base, seed)? else {
                continue;
            };
            let spectral_options = SpectralAnalysisOptions {
                maximum_dimension: 256,
                ..SpectralAnalysisOptions::default()
            };
            let range = DenseRangeDecomposition::from_problem(&problem, spectral_options)?;
            let oracle_condition = condition(&problem, &oracle, &range, spectral_options)?;
            if oracle_condition > 5.0 {
                continue;
            }
            let one_shot = build_pair_neighborhood_aggregation(
                &problem,
                PairNeighborhoodAggregationOptions::default(),
            )?;
            let one_shot_dimension: usize = one_shot.coarse_counts().iter().sum();
            if one_shot_dimension >= problem.dimension() {
                continue;
            }
            let one_shot_condition = condition(&problem, &one_shot, &range, spectral_options)?;
            if one_shot_condition < 1.10 * oracle_condition {
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
            let one_shot_coarse = one_shot.coarsen(&problem)?;
            let bootstrap_coarse = bootstrap.final_aggregation().coarsen(&problem)?;
            let witnesses = bootstrap
                .rounds()
                .last()
                .map_or(0, |round| round.bootstrap_witnesses());
            let score = one_shot_condition / bootstrap_condition;
            retained.push((
                score,
                seed,
                problem.dimension(),
                problem.tuple_count(),
                oracle_condition,
                one_shot_condition,
                bootstrap_condition,
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
        for candidate in retained.into_iter().take(5) {
            println!(
                "{}\t{}\t{}\t{}\t{:.9e}\t{:.9e}\t{:.9e}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}",
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
                candidate.14.replace(['\t', '\n'], " "),
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

fn random_sparse_refinement(
    base: &ThreeWayProblem,
    seed: u64,
) -> Result<Option<(ThreeWayProblem, FactorAggregation)>, DynError> {
    let coarse_counts = base.topology().level_counts();
    let fine_counts = coarse_counts.map(|count| count * 2);
    let parents = core::array::from_fn(|factor| {
        (0..fine_counts[factor])
            .map(|level| (level / 2) as u32)
            .collect()
    });
    let oracle = FactorAggregation::new(fine_counts, parents)?;
    let patterns = balanced_patterns();
    let mut tuples = Vec::new();
    let mut weights = Vec::new();
    for (tuple_index, (&tuple, &weight)) in base
        .topology()
        .tuples()
        .iter()
        .zip(base.weights())
        .enumerate()
    {
        let mixed = mix(seed ^ (tuple_index as u64).wrapping_mul(0x9e37_79b9_7f4a_7c15));
        let width = 4 + ((mixed >> 8) as usize % 3);
        let start = mixed as usize % patterns.len();
        let mut selected = Vec::new();
        let mut used = [false; 8];
        for offset in 0..patterns.len() {
            let pattern = &patterns[(start + offset) % patterns.len()];
            for &child in pattern {
                used[child] = true;
            }
            if used.iter().filter(|&&value| value).count() >= width {
                break;
            }
        }
        let mut score_sum = 0.0;
        for (child, &is_used) in used.iter().enumerate() {
            if is_used {
                let score = 0.6
                    + (mix(mixed ^ (child as u64).wrapping_mul(0xbf58_476d_1ce4_e5b9)) % 19) as f64
                        / 20.0;
                selected.push((child, score));
                score_sum += score;
            }
        }
        for (child, score) in selected {
            let first = ((child >> 2) & 1) as u32;
            let second = ((child >> 1) & 1) as u32;
            let third = (child & 1) as u32;
            tuples.push([
                2 * tuple[0] + first,
                2 * tuple[1] + second,
                2 * tuple[2] + third,
            ]);
            weights.push(weight * score / score_sum);
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

fn balanced_patterns() -> Vec<Vec<usize>> {
    let mut patterns = Vec::new();
    for mask in 0_u16..256 {
        let selected: Vec<usize> = (0..8).filter(|child| mask & (1 << child) != 0).collect();
        if !(4..=6).contains(&selected.len()) {
            continue;
        }
        let covers = (0..3).all(|factor| {
            (0..2).all(|bit| {
                selected.iter().any(|&child| {
                    let shift = 2 - factor;
                    ((child >> shift) & 1) == bit
                })
            })
        });
        if covers {
            patterns.push(selected);
        }
    }
    patterns
}

fn oracle_respects_components(problem: &ThreeWayProblem, oracle: &FactorAggregation) -> bool {
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
        setup_test_vectors: 3,
        setup_sweeps: 3,
        setup_jacobi_omega: 0.5,
        maximum_neighbor_degree: 16,
        signature_window: 2,
        maximum_candidate_degree: 12,
        minimum_combined_affinity: 0.45,
        algebraic_affinity_weight: 0.70,
        structural_affinity_weight: 0.10,
        degree_affinity_weight: 0.10,
        signature_hit_weight: 0.10,
        structural_baseline_required_factor_ratio: 0.97,
        structural_baseline_maximum_dimension_overhead_ratio: 0.05,
        structural_baseline_maximum_tuple_overhead_ratio: 0.10,
        compatible_relaxation: CompatibleRelaxationOptions {
            test_vectors: 12,
            sweeps: 10,
            relaxation_damping: 1.0,
            seed: 0x4d57_4d47_4352_3031,
            relative_zero_tolerance: 1.0e-13,
        },
        compatible_criteria: CompatibleRelaxationCriteria {
            maximum_diagonal_factor_per_sweep: 0.80,
            maximum_energy_factor_per_sweep: Some(0.80),
            maximum_final_coarse_defect: 1.0e-10,
            maximum_final_structural_defect: 1.0e-10,
        },
        maximum_bootstrap_witnesses: 5,
        maximum_coarse_dimension_ratio: 0.80,
        minimum_tuple_reduction: 0.02,
        maximum_two_level_tuple_complexity: 1.98,
        split_repair: None,
        seed: 0x4d57_4d47_5345_4152,
    }
}

fn mix(mut value: u64) -> u64 {
    value = (value ^ (value >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    value = (value ^ (value >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
    value ^ (value >> 31)
}
