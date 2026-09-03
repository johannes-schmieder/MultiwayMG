//! Oracle-hierarchy quotient-space spectral matrix for issue #2.

use multiway_mg::{
    AggregationStrategy, DensePairOptions, DensePairSchwarzPreconditioner, DensePseudoinverse,
    DenseRangeDecomposition, DiagonalPreconditioner, FactorAggregation, HierarchyOptions,
    HybridPairVcycle, PairCmgOptions, PairCmgPreconditioner, PcgOptions, Preconditioner,
    SpectralAnalysisOptions, SymmetricMapPreconditioner, ThreeWayHierarchy, ThreeWayProblem,
    solve_projected_pcg,
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!(
        "case\trefinements\tdimension\ttuples\tcomponents\trank\tnullity\tgramian_kappa\tmethod\thierarchy_depth\ttuple_complexity\tfull_symmetry_defect\tquotient_symmetry_defect\trange_leakage\tmin_preconditioner_energy\tmax_preconditioner_energy\tnegative_energy_directions\tzero_energy_directions\tmin_preconditioned_eigenvalue\tmax_preconditioned_eigenvalue\tpreconditioned_kappa\tunit_step_energy_radius\toptimal_richardson_damping\toptimal_energy_radius\tpcg_iterations\tpcg_converged\tpcg_relative_residual"
    );
    for case in oracle_cases()? {
        run_case(&case)?;
    }
    Ok(())
}

struct OracleCase {
    name: &'static str,
    problem: ThreeWayProblem,
    aggregations: Vec<FactorAggregation>,
    terminal_dimension: usize,
}

fn run_case(case: &OracleCase) -> Result<(), Box<dyn std::error::Error>> {
    let spectral_options = SpectralAnalysisOptions {
        maximum_dimension: 256,
        ..SpectralAnalysisOptions::default()
    };
    let range = DenseRangeDecomposition::from_problem(&case.problem, spectral_options)?;
    let rhs = deterministic_range_rhs(&case.problem)?;
    let hierarchy_options = oracle_hierarchy_options(case);

    let exact = DensePseudoinverse::from_problem(&case.problem, 1.0e-12)?;
    run_method(
        case,
        &range,
        spectral_options,
        &rhs,
        "exact-pseudoinverse",
        &exact,
        0,
        1.0,
    )?;

    for omega in [0.4_f64, 0.5, 0.6] {
        let diagonal = DiagonalPreconditioner::new(&case.problem, omega)?;
        run_method(
            case,
            &range,
            spectral_options,
            &rhs,
            match omega {
                value if (value - 0.4).abs() < f64::EPSILON => "jacobi-0.4",
                value if (value - 0.5).abs() < f64::EPSILON => "jacobi-0.5",
                _ => "jacobi-0.6",
            },
            &diagonal,
            0,
            0.0,
        )?;
    }

    let symmetric_map = SymmetricMapPreconditioner::new(case.problem.clone());
    run_method(
        case,
        &range,
        spectral_options,
        &rhs,
        "symmetric-map",
        &symmetric_map,
        0,
        0.0,
    )?;

    let dense_pair = DensePairSchwarzPreconditioner::build(
        case.problem.clone(),
        DensePairOptions::default(),
    )?;
    run_method(
        case,
        &range,
        spectral_options,
        &rhs,
        "exact-pair-schwarz",
        &dense_pair,
        0,
        0.0,
    )?;

    let pair_cmg = PairCmgPreconditioner::build(case.problem.clone(), PairCmgOptions::default())?;
    run_method(
        case,
        &range,
        spectral_options,
        &rhs,
        "pair-cmg",
        &pair_cmg,
        0,
        0.0,
    )?;

    let hierarchy = ThreeWayHierarchy::build(case.problem.clone(), hierarchy_options.clone())?;
    run_method(
        case,
        &range,
        spectral_options,
        &rhs,
        "oracle-vcycle-jacobi",
        &hierarchy,
        hierarchy.depth(),
        hierarchy.report().tuple_complexity(),
    )?;

    let hybrid = HybridPairVcycle::build(
        case.problem.clone(),
        hierarchy_options,
        PairCmgOptions::default(),
    )?;
    run_method(
        case,
        &range,
        spectral_options,
        &rhs,
        "oracle-vcycle-pair-cmg",
        &hybrid,
        hybrid.hierarchy().depth(),
        hybrid.hierarchy().report().tuple_complexity(),
    )?;
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn run_method(
    case: &OracleCase,
    range: &DenseRangeDecomposition,
    spectral_options: SpectralAnalysisOptions,
    rhs: &[f64],
    method: &str,
    preconditioner: &dyn Preconditioner,
    hierarchy_depth: usize,
    tuple_complexity: f64,
) -> Result<(), Box<dyn std::error::Error>> {
    let report = range.analyze(preconditioner, spectral_options)?;
    let pcg = solve_projected_pcg(
        &case.problem,
        rhs,
        preconditioner,
        PcgOptions {
            relative_tolerance: 1.0e-10,
            absolute_tolerance: 0.0,
            max_iterations: 1_000,
            residual_recompute_interval: 10,
        },
    )?;
    if !pcg.converged() {
        return Err(format!("{method} did not converge on {}", case.name).into());
    }
    println!(
        "{}\t{}\t{}\t{}\t{}\t{}\t{}\t{:.9e}\t{}\t{}\t{:.6}\t{:.9e}\t{:.9e}\t{:.9e}\t{:.9e}\t{:.9e}\t{}\t{}\t{:.9e}\t{:.9e}\t{:.9e}\t{:.9e}\t{:.9e}\t{:.9e}\t{}\t{}\t{:.9e}",
        case.name,
        case.aggregations.len(),
        report.dimension(),
        case.problem.tuple_count(),
        case.problem.components().count(),
        report.numerical_rank(),
        report.numerical_nullity(),
        report.gramian_condition_number(),
        method,
        hierarchy_depth,
        tuple_complexity,
        report.preconditioner_symmetry_defect(),
        report.quotient_symmetry_defect(),
        report.range_leakage(),
        report.minimum_preconditioner_energy(),
        report.maximum_preconditioner_energy(),
        report.negative_preconditioner_directions(),
        report.near_zero_preconditioner_directions(),
        report.minimum_preconditioned_eigenvalue(),
        report.maximum_preconditioned_eigenvalue(),
        report.preconditioned_condition_number(),
        report.unit_step_energy_spectral_radius(),
        report.optimal_richardson_damping(),
        report.optimal_energy_spectral_radius(),
        pcg.iterations(),
        pcg.converged(),
        pcg.relative_residual(),
    );
    Ok(())
}

fn oracle_hierarchy_options(case: &OracleCase) -> HierarchyOptions {
    HierarchyOptions {
        max_levels: case.aggregations.len(),
        terminal_dimension: case.terminal_dimension,
        minimum_dimension_reduction: 0.0,
        minimum_tuple_reduction: 0.0,
        terminal_relative_tolerance: 1.0e-12,
        jacobi_omega: 0.5,
        pre_sweeps: 1,
        post_sweeps: 1,
        aggregation: AggregationStrategy::Supplied(case.aggregations.clone()),
    }
}

fn deterministic_range_rhs(problem: &ThreeWayProblem) -> Result<Vec<f64>, Box<dyn std::error::Error>> {
    let mut coefficients: Vec<f64> = (0..problem.dimension())
        .map(|index| {
            let position = index as f64 + 1.0;
            (0.173 * position).sin() + 0.37 * (0.071 * position).cos()
        })
        .collect();
    problem
        .components()
        .project_structural_range(&mut coefficients)?;
    let mut rhs = vec![0.0; problem.dimension()];
    problem.apply_gramian(&coefficients, &mut rhs)?;
    Ok(rhs)
}

fn oracle_cases() -> Result<Vec<OracleCase>, Box<dyn std::error::Error>> {
    Ok(vec![
        refine_case("planted-communities", planted_communities(4, 0.02)?, 2, 2)?,
        refine_case("latin-square", latin_square(6)?, 2, 2)?,
        refine_case("weak-chain", weak_chain(6, 0.02)?, 3, 2)?,
        refine_case("nearly-nested", nearly_nested(6, 0.02)?, 2, 2)?,
        refine_case("disconnected-latin", disconnected_latin(3)?, 2, 2)?,
        refine_case("four-level-complete", complete_weighted(2)?, 4, 2)?,
    ])
}

fn refine_case(
    name: &'static str,
    base: ThreeWayProblem,
    refinement_levels: usize,
    clone_factor: usize,
) -> Result<OracleCase, Box<dyn std::error::Error>> {
    let terminal_dimension = base.dimension();
    let mut current = base;
    let mut coarse_to_fine_aggregations = Vec::with_capacity(refinement_levels);
    for refinement in 0..refinement_levels {
        let (fine, aggregation) = refine_once(&current, clone_factor, refinement)?;
        let reconstructed = aggregation.coarsen(&fine)?;
        verify_same_problem(&current, &reconstructed)?;
        coarse_to_fine_aggregations.push(aggregation);
        current = fine;
    }
    coarse_to_fine_aggregations.reverse();
    Ok(OracleCase {
        name,
        problem: current,
        aggregations: coarse_to_fine_aggregations,
        terminal_dimension,
    })
}

fn refine_once(
    coarse: &ThreeWayProblem,
    clone_factor: usize,
    refinement: usize,
) -> Result<(ThreeWayProblem, FactorAggregation), Box<dyn std::error::Error>> {
    if clone_factor < 2 {
        return Err("clone_factor must be at least two".into());
    }
    let coarse_counts = coarse.topology().level_counts();
    let fine_counts = coarse_counts.map(|count| count * clone_factor);
    let parents = core::array::from_fn(|factor| {
        (0..fine_counts[factor])
            .map(|level| (level / clone_factor) as u32)
            .collect()
    });
    let aggregation = FactorAggregation::new(fine_counts, parents)?;
    let children_per_tuple = clone_factor * clone_factor * clone_factor;
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
    let fine = ThreeWayProblem::from_observations(fine_counts, &tuples, &weights)?;
    Ok((fine, aggregation))
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
                weights.push(base * (1.0 + ((3 * first + 5 * second + 7 * third) % 11) as f64 / 20.0));
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
            tuples.push([
                first as u32,
                second as u32,
                ((first + 1) % levels) as u32,
            ]);
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

fn complete_weighted(levels: usize) -> Result<ThreeWayProblem, Box<dyn std::error::Error>> {
    let mut tuples = Vec::new();
    let mut weights = Vec::new();
    for first in 0..levels {
        for second in 0..levels {
            for third in 0..levels {
                tuples.push([first as u32, second as u32, third as u32]);
                weights.push(0.7 + ((5 * first + 7 * second + 11 * third) % 13) as f64 / 10.0);
            }
        }
    }
    Ok(ThreeWayProblem::from_observations(
        [levels; 3],
        &tuples,
        &weights,
    )?)
}
