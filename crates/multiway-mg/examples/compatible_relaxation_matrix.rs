//! Deterministic compatible-relaxation matrix for proposed hard coarse spaces.

use std::collections::BTreeMap;

use multiway_mg::{
    AffinityAggregationOptions, CompatibleRelaxationOptions, DiagonalPreconditioner,
    FactorAggregation, PairCmgOptions, PairCmgPreconditioner, PairNeighborhoodAggregationOptions,
    Preconditioner, SymmetricMapPreconditioner, ThreeWayProblem, analyze_compatible_relaxation,
    build_affinity_aggregation, build_pair_neighborhood_aggregation,
};

const TEST_VECTORS: usize = 16;
const SWEEPS: usize = 12;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!(
        "case\trefinement\tdimension\ttuples\tcomponents\tmap\toracle_match\tcoarse_dimension\tcoarse_tuples\tcompatible_dimension\tsmoother\tsweeps\tmaximum_diagonal_contraction\tgeometric_mean_diagonal_contraction\tmaximum_diagonal_factor_per_sweep\tgeometric_mean_diagonal_factor_per_sweep\tmaximum_energy_contraction\tgeometric_mean_energy_contraction\tmaximum_energy_factor_per_sweep\tgeometric_mean_energy_factor_per_sweep\tmaximum_final_coarse_defect\tmaximum_final_structural_defect\tstatus"
    );
    for case in cases()? {
        run_case(&case)?;
    }
    Ok(())
}

#[derive(Debug, Clone, Copy)]
enum RefinementPattern {
    Complete,
    ParitySparse,
}

impl RefinementPattern {
    const fn label(self) -> &'static str {
        match self {
            Self::Complete => "complete-child-tensor",
            Self::ParitySparse => "parity-sparse-child-tensor",
        }
    }
}

struct Case {
    name: &'static str,
    refinement: RefinementPattern,
    problem: ThreeWayProblem,
    oracle: FactorAggregation,
}

fn run_case(case: &Case) -> Result<(), Box<dyn std::error::Error>> {
    let exact_context = build_affinity_aggregation(
        &case.problem,
        AffinityAggregationOptions {
            minimum_affinity: 0.15,
            maximum_context_degree: 16,
        },
    )?;
    let pair_neighborhood = build_pair_neighborhood_aggregation(
        &case.problem,
        PairNeighborhoodAggregationOptions {
            minimum_affinity: 0.02,
            maximum_neighbor_degree: 16,
        },
    )?;
    let misaligned = misaligned_aggregation(&case.problem)?;
    let maps = [
        ("oracle", case.oracle.clone()),
        ("exact-context", exact_context),
        ("pair-neighborhood", pair_neighborhood),
        ("misaligned-control", misaligned),
    ];

    let diagonal = DiagonalPreconditioner::new(&case.problem, 0.5)?;
    let symmetric_map = SymmetricMapPreconditioner::new(case.problem.clone());
    let pair_cmg = PairCmgPreconditioner::build(case.problem.clone(), PairCmgOptions::default())?;
    let smoothers: [(&str, &dyn Preconditioner, f64); 3] = [
        ("weighted-jacobi", &diagonal, 1.0),
        ("symmetric-map", &symmetric_map, 1.0),
        ("pair-cmg", &pair_cmg, 1.0),
    ];

    for (map_name, aggregation) in maps {
        let coarse = aggregation.coarsen(&case.problem)?;
        let coarse_dimension = coarse.dimension();
        let coarse_tuples = coarse.tuple_count();
        let compatible_dimension = case.problem.dimension() - coarse_dimension;
        let oracle_match = aggregation == case.oracle;
        for (smoother_name, smoother, damping) in smoothers {
            if compatible_dimension == 0 {
                print_skipped(
                    case,
                    map_name,
                    oracle_match,
                    coarse_dimension,
                    coarse_tuples,
                    compatible_dimension,
                    smoother_name,
                    "no-compatible-complement",
                );
                continue;
            }
            let options = CompatibleRelaxationOptions {
                test_vectors: TEST_VECTORS,
                sweeps: SWEEPS,
                relaxation_damping: damping,
                seed: 0x4d57_4d47_4352_3031,
                relative_zero_tolerance: 1.0e-13,
            };
            let report =
                analyze_compatible_relaxation(&case.problem, &aggregation, smoother, options)?;
            let max_energy = report.maximum_energy_contraction();
            let mean_energy = report.geometric_mean_energy_contraction();
            println!(
                "{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{:.9e}\t{:.9e}\t{:.9e}\t{:.9e}\t{}\t{}\t{}\t{}\t{:.9e}\t{:.9e}\tok",
                case.name,
                case.refinement.label(),
                case.problem.dimension(),
                case.problem.tuple_count(),
                case.problem.components().count(),
                map_name,
                oracle_match,
                coarse_dimension,
                coarse_tuples,
                report.compatible_dimension(),
                smoother_name,
                report.sweeps(),
                report.maximum_diagonal_contraction(),
                report.geometric_mean_diagonal_contraction(),
                per_sweep(report.maximum_diagonal_contraction()),
                per_sweep(report.geometric_mean_diagonal_contraction()),
                optional(max_energy),
                optional(mean_energy),
                optional(max_energy.map(per_sweep)),
                optional(mean_energy.map(per_sweep)),
                report.maximum_final_coarse_defect(),
                report.maximum_final_structural_defect(),
            );
        }
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn print_skipped(
    case: &Case,
    map_name: &str,
    oracle_match: bool,
    coarse_dimension: usize,
    coarse_tuples: usize,
    compatible_dimension: usize,
    smoother_name: &str,
    status: &str,
) {
    println!(
        "{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\tNaN\tNaN\tNaN\tNaN\tNaN\tNaN\tNaN\tNaN\tNaN\tNaN\t{}",
        case.name,
        case.refinement.label(),
        case.problem.dimension(),
        case.problem.tuple_count(),
        case.problem.components().count(),
        map_name,
        oracle_match,
        coarse_dimension,
        coarse_tuples,
        compatible_dimension,
        smoother_name,
        SWEEPS,
        status,
    );
}

fn per_sweep(total_contraction: f64) -> f64 {
    total_contraction.powf(1.0 / SWEEPS as f64)
}

fn optional(value: Option<f64>) -> String {
    value.map_or_else(|| "NaN".to_owned(), |number| format!("{number:.9e}"))
}

fn cases() -> Result<Vec<Case>, Box<dyn std::error::Error>> {
    let specifications = [
        (
            "planted-communities-complete",
            planted_communities(4, 0.02)?,
            RefinementPattern::Complete,
        ),
        (
            "planted-communities-sparse",
            planted_communities(4, 0.02)?,
            RefinementPattern::ParitySparse,
        ),
        (
            "weak-chain-sparse",
            weak_chain(8, 0.01)?,
            RefinementPattern::ParitySparse,
        ),
        (
            "latin-square-sparse",
            latin_square(8)?,
            RefinementPattern::ParitySparse,
        ),
        (
            "nearly-nested-sparse",
            nearly_nested(8, 0.01)?,
            RefinementPattern::ParitySparse,
        ),
        (
            "disconnected-latin-sparse",
            disconnected_latin(4)?,
            RefinementPattern::ParitySparse,
        ),
    ];
    specifications
        .into_iter()
        .enumerate()
        .map(|(index, (name, base, pattern))| {
            let (problem, oracle) = refine_once(&base, 2, pattern, index as u64)?;
            Ok(Case {
                name,
                refinement: pattern,
                problem,
                oracle,
            })
        })
        .collect()
}

fn refine_once(
    coarse: &ThreeWayProblem,
    clone_factor: usize,
    pattern: RefinementPattern,
    seed: u64,
) -> Result<(ThreeWayProblem, FactorAggregation), Box<dyn std::error::Error>> {
    let coarse_counts = coarse.topology().level_counts();
    let fine_counts = coarse_counts.map(|count| count * clone_factor);
    let parents = core::array::from_fn(|factor| {
        (0..fine_counts[factor])
            .map(|level| (level / clone_factor) as u32)
            .collect()
    });
    let oracle = FactorAggregation::new(fine_counts, parents)?;
    let mut tuples = Vec::new();
    let mut weights = Vec::new();

    for (tuple_index, (&tuple, &coarse_weight)) in coarse
        .topology()
        .tuples()
        .iter()
        .zip(coarse.weights())
        .enumerate()
    {
        let parity = ((tuple_index as u64).wrapping_mul(17) ^ seed) as usize & 1;
        let mut children = Vec::new();
        let mut score_sum = 0.0;
        for first_child in 0..clone_factor {
            for second_child in 0..clone_factor {
                for third_child in 0..clone_factor {
                    if matches!(pattern, RefinementPattern::ParitySparse)
                        && (first_child + second_child + third_child) % 2 != parity
                    {
                        continue;
                    }
                    let mixed = tuple_index
                        .wrapping_mul(131)
                        .wrapping_add(first_child.wrapping_mul(17))
                        .wrapping_add(second_child.wrapping_mul(11))
                        .wrapping_add(third_child.wrapping_mul(5))
                        .wrapping_add(seed as usize);
                    let score = 0.75 + (mixed % 17) as f64 / 16.0;
                    children.push((first_child, second_child, third_child, score));
                    score_sum += score;
                }
            }
        }
        for (first_child, second_child, third_child, score) in children {
            tuples.push([
                (tuple[0] as usize * clone_factor + first_child) as u32,
                (tuple[1] as usize * clone_factor + second_child) as u32,
                (tuple[2] as usize * clone_factor + third_child) as u32,
            ]);
            weights.push(coarse_weight * score / score_sum);
        }
    }

    let fine = ThreeWayProblem::from_observations(fine_counts, &tuples, &weights)?;
    let reconstructed = oracle.coarsen(&fine)?;
    verify_same_problem(coarse, &reconstructed)?;
    Ok((fine, oracle))
}

fn misaligned_aggregation(
    problem: &ThreeWayProblem,
) -> Result<FactorAggregation, Box<dyn std::error::Error>> {
    let counts = problem.topology().level_counts();
    let mut parents: [Vec<u32>; 3] = core::array::from_fn(|factor| vec![u32::MAX; counts[factor]]);
    for factor in 0..3 {
        let mut by_component: BTreeMap<usize, Vec<usize>> = BTreeMap::new();
        for level in 0..counts[factor] {
            by_component
                .entry(problem.components().component_of(factor, level))
                .or_default()
                .push(level);
        }
        let mut next_parent = 0_u32;
        for levels in by_component.values() {
            for chunk in levels.chunks(4) {
                match chunk {
                    [a, b, c, d] => {
                        parents[factor][*a] = next_parent;
                        parents[factor][*c] = next_parent;
                        next_parent += 1;
                        parents[factor][*b] = next_parent;
                        parents[factor][*d] = next_parent;
                        next_parent += 1;
                    }
                    [a, b] => {
                        parents[factor][*a] = next_parent;
                        parents[factor][*b] = next_parent;
                        next_parent += 1;
                    }
                    [a] => {
                        parents[factor][*a] = next_parent;
                        next_parent += 1;
                    }
                    _ => return Err("unexpected component-local refinement width".into()),
                }
            }
        }
    }
    Ok(FactorAggregation::new(counts, parents)?)
}

fn verify_same_problem(
    expected: &ThreeWayProblem,
    actual: &ThreeWayProblem,
) -> Result<(), Box<dyn std::error::Error>> {
    if expected.topology().level_counts() != actual.topology().level_counts()
        || expected.topology().tuples() != actual.topology().tuples()
        || expected.weights().len() != actual.weights().len()
    {
        return Err("oracle refinement did not reconstruct the coarse topology".into());
    }
    for (&left, &right) in expected.weights().iter().zip(actual.weights()) {
        let scale = left.abs().max(right.abs()).max(1.0);
        if (left - right).abs() > 1.0e-12 * scale {
            return Err(format!(
                "oracle refinement weight mismatch: expected {left}, reconstructed {right}"
            )
            .into());
        }
    }
    Ok(())
}

fn planted_communities(
    levels: usize,
    bridge_weight: f64,
) -> Result<ThreeWayProblem, Box<dyn std::error::Error>> {
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
    Ok(ThreeWayProblem::from_observations(
        [levels; 3],
        &tuples,
        &weights,
    )?)
}

fn latin_square(levels: usize) -> Result<ThreeWayProblem, Box<dyn std::error::Error>> {
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
    Ok(ThreeWayProblem::from_observations(
        [levels; 3],
        &tuples,
        &weights,
    )?)
}

fn weak_chain(
    levels: usize,
    bridge_weight: f64,
) -> Result<ThreeWayProblem, Box<dyn std::error::Error>> {
    let mut tuples = Vec::new();
    let mut weights = Vec::new();
    for level in 0..levels {
        tuples.push([level as u32, level as u32, level as u32]);
        weights.push(1.0 + (level % 5) as f64 / 10.0);
        if level + 1 < levels {
            tuples.push([level as u32, (level + 1) as u32, (level + 1) as u32]);
            weights.push(bridge_weight);
            tuples.push([(level + 1) as u32, level as u32, (level + 1) as u32]);
            weights.push(bridge_weight * 1.1);
            tuples.push([(level + 1) as u32, (level + 1) as u32, level as u32]);
            weights.push(bridge_weight * 0.9);
        }
    }
    Ok(ThreeWayProblem::from_observations(
        [levels; 3],
        &tuples,
        &weights,
    )?)
}

fn nearly_nested(
    levels: usize,
    perturbation_weight: f64,
) -> Result<ThreeWayProblem, Box<dyn std::error::Error>> {
    let mut tuples = Vec::new();
    let mut weights = Vec::new();
    for first in 0..levels {
        for second in 0..levels {
            tuples.push([first as u32, second as u32, first as u32]);
            weights.push(1.0 + ((first + 2 * second) % 7) as f64 / 10.0);
            tuples.push([first as u32, second as u32, ((first + 1) % levels) as u32]);
            weights.push(perturbation_weight);
        }
    }
    Ok(ThreeWayProblem::from_observations(
        [levels; 3],
        &tuples,
        &weights,
    )?)
}

fn disconnected_latin(
    levels_per_component: usize,
) -> Result<ThreeWayProblem, Box<dyn std::error::Error>> {
    let mut tuples = Vec::new();
    let mut weights = Vec::new();
    for component in 0..2 {
        let offset = component * levels_per_component;
        for first in 0..levels_per_component {
            for second in 0..levels_per_component {
                tuples.push([
                    (offset + first) as u32,
                    (offset + second) as u32,
                    (offset + (first + second) % levels_per_component) as u32,
                ]);
                weights.push(0.9 + ((component + 3 * first + 5 * second) % 9) as f64 / 10.0);
            }
        }
    }
    Ok(ThreeWayProblem::from_observations(
        [2 * levels_per_component; 3],
        &tuples,
        &weights,
    )?)
}
