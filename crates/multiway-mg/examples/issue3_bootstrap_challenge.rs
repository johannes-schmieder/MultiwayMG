//! Sparse voltage-lift challenges for automatic three-way aggregation.

use multiway_mg::{
    BootstrapAggregationOptions, CompatibleRelaxationCriteria, CompatibleRelaxationOptions,
    DenseRangeDecomposition, DiagonalPreconditioner, FactorAggregation,
    PairNeighborhoodAggregationOptions, SpectralAnalysisOptions, SymmetricMapPreconditioner,
    SymmetricTwoGridPreconditioner, ThreeWayProblem, analyze_compatible_relaxation,
    build_bootstrap_aggregation, build_pair_neighborhood_aggregation,
    evaluate_compatible_relaxation,
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!(
        "case\tmethod\taccepted\texact_oracle_partition\toracle_pair_recall\twrong_pairs\tcoarse_dimension\tcoarse_tuples\tcompatible_factor\ttwo_grid_condition\toracle_recovery\tbootstrap_rounds\tbootstrap_witnesses\tstop_reason"
    );
    for case in challenge_cases()? {
        run_case(&case)?;
    }
    Ok(())
}

struct ChallengeCase {
    name: &'static str,
    problem: ThreeWayProblem,
    oracle: FactorAggregation,
}

fn run_case(case: &ChallengeCase) -> Result<(), Box<dyn std::error::Error>> {
    let problem = &case.problem;
    let screen = DiagonalPreconditioner::new(problem, 0.5)?;
    let map = SymmetricMapPreconditioner::new(problem.clone());
    let spectral_options = SpectralAnalysisOptions {
        maximum_dimension: 256,
        ..SpectralAnalysisOptions::default()
    };
    let range = DenseRangeDecomposition::from_problem(problem, spectral_options)?;
    let baseline = range
        .analyze(&map, spectral_options)?
        .preconditioned_condition_number();
    let oracle_cycle = two_grid(problem, &case.oracle)?;
    let oracle_condition = range
        .analyze(&oracle_cycle, spectral_options)?
        .preconditioned_condition_number();

    record(
        case,
        "oracle",
        &case.oracle,
        true,
        0,
        0,
        "oracle",
        &screen,
        &range,
        spectral_options,
        baseline,
        oracle_condition,
    )?;

    let one_shot = build_pair_neighborhood_aggregation(
        problem,
        PairNeighborhoodAggregationOptions::default(),
    )?;
    record(
        case,
        "one-shot-pair-neighborhood",
        &one_shot,
        accepts(problem, &one_shot, &screen)?,
        0,
        0,
        "one-shot",
        &screen,
        &range,
        spectral_options,
        baseline,
        oracle_condition,
    )?;

    let bootstrap = build_bootstrap_aggregation(problem, &screen, bootstrap_options())?;
    let witnesses = bootstrap
        .rounds()
        .last()
        .map_or(0, |round| round.bootstrap_witnesses());
    record(
        case,
        "bootstrap",
        bootstrap.final_aggregation(),
        bootstrap.accepted(),
        bootstrap.rounds().len(),
        witnesses,
        &format!("{:?}", bootstrap.stop_reason()),
        &screen,
        &range,
        spectral_options,
        baseline,
        oracle_condition,
    )?;
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn record(
    case: &ChallengeCase,
    method: &str,
    aggregation: &FactorAggregation,
    accepted: bool,
    rounds: usize,
    witnesses: usize,
    stop_reason: &str,
    screen: &DiagonalPreconditioner,
    range: &DenseRangeDecomposition,
    spectral_options: SpectralAnalysisOptions,
    baseline_condition: f64,
    oracle_condition: f64,
) -> Result<(), Box<dyn std::error::Error>> {
    let problem = &case.problem;
    let coarse = aggregation.coarsen(problem)?;
    let compatible_factor = compatible_factor(problem, aggregation, screen)?;
    let cycle = two_grid(problem, aggregation)?;
    let condition = range
        .analyze(&cycle, spectral_options)?
        .preconditioned_condition_number();
    let partition = partition_metrics(&case.oracle, aggregation);
    let recovery = (baseline_condition - condition) / (baseline_condition - oracle_condition);
    println!(
        "{}\t{}\t{}\t{}\t{:.6}\t{}\t{}\t{}\t{}\t{:.9e}\t{:.9e}\t{}\t{}\t{}",
        case.name,
        method,
        accepted,
        partition.exact,
        partition.recall,
        partition.wrong_pairs,
        aggregation.coarse_counts().iter().sum::<usize>(),
        coarse.tuple_count(),
        optional(compatible_factor),
        condition,
        recovery,
        rounds,
        witnesses,
        stop_reason.replace(['\t', '\n'], " "),
    );
    Ok(())
}

fn two_grid(
    problem: &ThreeWayProblem,
    aggregation: &FactorAggregation,
) -> Result<SymmetricTwoGridPreconditioner<SymmetricMapPreconditioner>, Box<dyn std::error::Error>>
{
    Ok(SymmetricTwoGridPreconditioner::build(
        problem.clone(),
        aggregation.clone(),
        SymmetricMapPreconditioner::new(problem.clone()),
        1,
        1.0,
        1.0e-12,
    )?)
}

fn accepts(
    problem: &ThreeWayProblem,
    aggregation: &FactorAggregation,
    smoother: &DiagonalPreconditioner,
) -> Result<bool, Box<dyn std::error::Error>> {
    if aggregation.coarse_counts().iter().sum::<usize>() >= problem.dimension() {
        return Ok(false);
    }
    let report = analyze_compatible_relaxation(
        problem,
        aggregation,
        smoother,
        compatible_options(),
    )?;
    Ok(evaluate_compatible_relaxation(&report, compatible_criteria())?.accepted())
}

fn compatible_factor(
    problem: &ThreeWayProblem,
    aggregation: &FactorAggregation,
    smoother: &DiagonalPreconditioner,
) -> Result<Option<f64>, Box<dyn std::error::Error>> {
    if aggregation.coarse_counts().iter().sum::<usize>() >= problem.dimension() {
        return Ok(None);
    }
    let report = analyze_compatible_relaxation(
        problem,
        aggregation,
        smoother,
        compatible_options(),
    )?;
    Ok(Some(
        report
            .maximum_diagonal_contraction()
            .powf(1.0 / report.sweeps() as f64),
    ))
}

fn bootstrap_options() -> BootstrapAggregationOptions {
    BootstrapAggregationOptions {
        setup_test_vectors: 3,
        setup_sweeps: 3,
        setup_jacobi_omega: 0.5,
        maximum_neighbor_degree: 12,
        signature_window: 2,
        maximum_candidate_degree: 10,
        minimum_combined_affinity: 0.45,
        algebraic_affinity_weight: 0.70,
        structural_affinity_weight: 0.10,
        degree_affinity_weight: 0.10,
        signature_hit_weight: 0.10,
        compatible_relaxation: compatible_options(),
        compatible_criteria: compatible_criteria(),
        maximum_bootstrap_witnesses: 5,
        maximum_coarse_dimension_ratio: 0.80,
        minimum_tuple_reduction: 0.05,
        maximum_two_level_tuple_complexity: 1.95,
        split_repair: None,
        seed: 0x4d57_4d47_4348_414c,
    }
}

fn compatible_options() -> CompatibleRelaxationOptions {
    CompatibleRelaxationOptions {
        test_vectors: 12,
        sweeps: 10,
        relaxation_damping: 1.0,
        seed: 0x4d57_4d47_4352_3031,
        relative_zero_tolerance: 1.0e-13,
    }
}

fn compatible_criteria() -> CompatibleRelaxationCriteria {
    CompatibleRelaxationCriteria {
        maximum_diagonal_factor_per_sweep: 0.72,
        maximum_energy_factor_per_sweep: Some(0.72),
        maximum_final_coarse_defect: 1.0e-10,
        maximum_final_structural_defect: 1.0e-10,
    }
}

fn challenge_cases() -> Result<Vec<ChallengeCase>, Box<dyn std::error::Error>> {
    let bases = [
        ("voltage-latin", latin_base(8)?),
        ("voltage-weak-cycle", weak_cycle_base(8, 0.01)?),
        ("voltage-nearly-nested", nearly_nested_base(7, 0.015)?),
        ("voltage-communities", community_base(8, 0.01)?),
    ];
    bases
        .into_iter()
        .enumerate()
        .map(|(index, (name, base))| voltage_lift(name, &base, 100 + index as u64))
        .collect()
}

fn voltage_lift(
    name: &'static str,
    base: &ThreeWayProblem,
    starting_seed: u64,
) -> Result<ChallengeCase, Box<dyn std::error::Error>> {
    for seed in starting_seed..starting_seed + 256 {
        let counts = base.topology().level_counts().map(|count| count * 2);
        let parents = core::array::from_fn(|_| {
            (0..counts[0])
                .map(|level| (level / 2) as u32)
                .collect()
        });
        let oracle = FactorAggregation::new(counts, parents)?;
        let mut tuples = Vec::with_capacity(base.tuple_count() * 2);
        let mut weights = Vec::with_capacity(tuples.capacity());
        for (index, (&tuple, &weight)) in base
            .topology()
            .tuples()
            .iter()
            .zip(base.weights())
            .enumerate()
        {
            let first_shift = hash_bit(seed, index as u64, 0);
            let second_shift = hash_bit(seed, index as u64, 1);
            let child_scores = [
                0.8 + ((index + seed as usize) % 7) as f64 / 10.0,
                0.8 + ((3 * index + seed as usize + 1) % 7) as f64 / 10.0,
            ];
            let score_sum = child_scores[0] + child_scores[1];
            for child in 0..2_u32 {
                tuples.push([
                    2 * tuple[0] + child,
                    2 * tuple[1] + (child ^ first_shift),
                    2 * tuple[2] + (child ^ second_shift),
                ]);
                weights.push(weight * child_scores[child as usize] / score_sum);
            }
        }
        let problem = ThreeWayProblem::from_observations(counts, &tuples, &weights)?;
        if oracle_respects_components(&problem, &oracle) {
            let reconstructed = oracle.coarsen(&problem)?;
            verify_same_problem(base, &reconstructed)?;
            return Ok(ChallengeCase {
                name,
                problem,
                oracle,
            });
        }
    }
    Err(format!("could not find a component-preserving voltage lift for {name}").into())
}

fn oracle_respects_components(
    problem: &ThreeWayProblem,
    oracle: &FactorAggregation,
) -> bool {
    let counts = problem.topology().level_counts();
    (0..3).all(|factor| {
        (0..counts[factor]).all(|level| {
            let sibling = if level % 2 == 0 { level + 1 } else { level - 1 };
            oracle.parents(factor)[level] == oracle.parents(factor)[sibling]
                && problem.components().component_of(factor, level)
                    == problem.components().component_of(factor, sibling)
        })
    })
}

fn verify_same_problem(
    expected: &ThreeWayProblem,
    actual: &ThreeWayProblem,
) -> Result<(), Box<dyn std::error::Error>> {
    if expected.topology().level_counts() != actual.topology().level_counts()
        || expected.topology().tuples() != actual.topology().tuples()
    {
        return Err("voltage lift did not reconstruct the base topology".into());
    }
    for (&left, &right) in expected.weights().iter().zip(actual.weights()) {
        if (left - right).abs() > 1.0e-12 * left.abs().max(right.abs()).max(1.0) {
            return Err(format!("voltage lift weight mismatch {left} versus {right}").into());
        }
    }
    Ok(())
}

fn hash_bit(seed: u64, index: u64, stream: u64) -> u32 {
    let mut value = seed
        ^ index.wrapping_mul(0x9e37_79b9_7f4a_7c15)
        ^ stream.wrapping_mul(0xbf58_476d_1ce4_e5b9);
    value = (value ^ (value >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    value = (value ^ (value >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
    ((value ^ (value >> 31)) & 1) as u32
}

fn latin_base(levels: usize) -> Result<ThreeWayProblem, Box<dyn std::error::Error>> {
    let mut tuples = Vec::new();
    let mut weights = Vec::new();
    for first in 0..levels {
        for second in 0..levels {
            tuples.push([
                first as u32,
                second as u32,
                ((first + second) % levels) as u32,
            ]);
            weights.push(0.8 + ((7 * first + 3 * second) % 13) as f64 / 10.0);
        }
    }
    Ok(ThreeWayProblem::from_observations([levels; 3], &tuples, &weights)?)
}

fn weak_cycle_base(
    levels: usize,
    bridge_weight: f64,
) -> Result<ThreeWayProblem, Box<dyn std::error::Error>> {
    let mut tuples = Vec::new();
    let mut weights = Vec::new();
    for level in 0..levels {
        tuples.push([level as u32, level as u32, level as u32]);
        weights.push(1.0 + (level % 5) as f64 / 10.0);
        tuples.push([
            level as u32,
            ((level + 1) % levels) as u32,
            ((level + 2) % levels) as u32,
        ]);
        weights.push(bridge_weight);
        tuples.push([
            ((level + 2) % levels) as u32,
            level as u32,
            ((level + 1) % levels) as u32,
        ]);
        weights.push(bridge_weight * 1.1);
    }
    Ok(ThreeWayProblem::from_observations([levels; 3], &tuples, &weights)?)
}

fn nearly_nested_base(
    levels: usize,
    perturbation_weight: f64,
) -> Result<ThreeWayProblem, Box<dyn std::error::Error>> {
    let mut tuples = Vec::new();
    let mut weights = Vec::new();
    for first in 0..levels {
        for second in 0..levels {
            tuples.push([first as u32, second as u32, first as u32]);
            weights.push(1.0 + ((first + 2 * second) % 7) as f64 / 10.0);
            tuples.push([
                first as u32,
                second as u32,
                ((first + 1) % levels) as u32,
            ]);
            weights.push(perturbation_weight);
        }
    }
    Ok(ThreeWayProblem::from_observations([levels; 3], &tuples, &weights)?)
}

fn community_base(
    levels: usize,
    bridge_weight: f64,
) -> Result<ThreeWayProblem, Box<dyn std::error::Error>> {
    let mut tuples = Vec::new();
    let mut weights = Vec::new();
    let half = levels / 2;
    for first in 0..levels {
        for second in 0..levels {
            let same = (first < half) == (second < half);
            let third = if same {
                (first + second) % half + if first < half { 0 } else { half }
            } else {
                (first + second) % levels
            };
            tuples.push([first as u32, second as u32, third as u32]);
            weights.push(if same { 1.0 } else { bridge_weight });
        }
    }
    Ok(ThreeWayProblem::from_observations([levels; 3], &tuples, &weights)?)
}

struct PartitionMetrics {
    exact: bool,
    recall: f64,
    wrong_pairs: usize,
}

fn partition_metrics(
    oracle: &FactorAggregation,
    candidate: &FactorAggregation,
) -> PartitionMetrics {
    let counts = oracle.fine_counts();
    let mut oracle_pairs = 0;
    let mut recovered = 0;
    let mut wrong = 0;
    for factor in 0..3 {
        for left in 0..counts[factor] {
            for right in (left + 1)..counts[factor] {
                let oracle_same = oracle.parents(factor)[left] == oracle.parents(factor)[right];
                let candidate_same =
                    candidate.parents(factor)[left] == candidate.parents(factor)[right];
                if oracle_same {
                    oracle_pairs += 1;
                    recovered += usize::from(candidate_same);
                } else {
                    wrong += usize::from(candidate_same);
                }
            }
        }
    }
    PartitionMetrics {
        exact: recovered == oracle_pairs && wrong == 0,
        recall: recovered as f64 / oracle_pairs as f64,
        wrong_pairs: wrong,
    }
}

fn optional(value: Option<f64>) -> String {
    value.map_or_else(|| "NA".to_owned(), |number| format!("{number:.9e}"))
}
