//! Dense calibration tests for the matrix-free complete-cycle probe.

use multiway_mg::{
    CycleQualityOptions, DenseRangeDecomposition, FactorAggregation, SpectralAnalysisOptions,
    SymmetricMapPreconditioner, SymmetricTwoGridPreconditioner, ThreeWayProblem,
    analyze_cycle_quality,
};

#[test]
fn matrix_free_power_probe_tracks_the_dense_energy_spectral_radius() {
    let (problem, oracle) = refined_weak_chain(10, 2, 0.005);
    let overmerged = overmerged_pairs(&oracle);
    let spectral_options = SpectralAnalysisOptions {
        maximum_dimension: 128,
        ..SpectralAnalysisOptions::default()
    };
    let range = DenseRangeDecomposition::from_problem(&problem, spectral_options)
        .expect("dense range decomposition succeeds");
    let probe_options = CycleQualityOptions {
        test_vectors: 16,
        power_iterations: 32,
        tail_iterations: 8,
        ..CycleQualityOptions::default()
    };

    for aggregation in [&oracle, &overmerged] {
        let cycle = map_cycle(&problem, aggregation);
        let dense = range
            .analyze(&cycle, spectral_options)
            .expect("dense spectral analysis succeeds");
        let exact_radius = dense
            .preconditioned_eigenvalues()
            .iter()
            .map(|&eigenvalue| (1.0 - eigenvalue).abs())
            .fold(0.0, f64::max);
        let probe = analyze_cycle_quality(&problem, &cycle, probe_options)
            .expect("matrix-free probe succeeds");
        let estimated = probe.maximum_estimated_energy_factor();

        assert!(estimated <= exact_radius * (1.0 + 1.0e-8) + 1.0e-10);
        assert!(exact_radius - estimated < 0.03);
        assert!(
            (probe.maximum_absolute_final_rayleigh() - estimated).abs()
                < 0.03 + 0.05 * exact_radius
        );
    }
}

fn map_cycle(
    problem: &ThreeWayProblem,
    aggregation: &FactorAggregation,
) -> SymmetricTwoGridPreconditioner<SymmetricMapPreconditioner> {
    SymmetricTwoGridPreconditioner::build(
        problem.clone(),
        aggregation.clone(),
        SymmetricMapPreconditioner::new(problem.clone()),
        1,
        1.0,
        1.0e-12,
    )
    .expect("two-grid cycle succeeds")
}

fn overmerged_pairs(oracle: &FactorAggregation) -> FactorAggregation {
    let fine_counts = oracle.fine_counts();
    let parents = core::array::from_fn(|factor| {
        (0..fine_counts[factor])
            .map(|level| oracle.parents(factor)[level] / 2)
            .collect()
    });
    FactorAggregation::new(fine_counts, parents).expect("overmerged map is valid")
}

fn refined_weak_chain(
    levels: usize,
    clones: usize,
    bridge_weight: f64,
) -> (ThreeWayProblem, FactorAggregation) {
    let mut coarse_tuples = Vec::new();
    let mut coarse_weights = Vec::new();
    for level in 0..levels {
        coarse_tuples.push([level as u32, level as u32, level as u32]);
        coarse_weights.push(1.0 + (level % 5) as f64 / 10.0);
        if level + 1 < levels {
            coarse_tuples.push([level as u32, (level + 1) as u32, (level + 1) as u32]);
            coarse_weights.push(bridge_weight);
            coarse_tuples.push([(level + 1) as u32, level as u32, (level + 1) as u32]);
            coarse_weights.push(bridge_weight * 1.1);
            coarse_tuples.push([(level + 1) as u32, (level + 1) as u32, level as u32]);
            coarse_weights.push(bridge_weight * 0.9);
        }
    }
    let coarse = ThreeWayProblem::from_observations([levels; 3], &coarse_tuples, &coarse_weights)
        .expect("coarse weak chain is valid");
    let fine_counts = [levels * clones; 3];
    let parents = core::array::from_fn(|_| {
        (0..levels * clones)
            .map(|level| (level / clones) as u32)
            .collect()
    });
    let aggregation =
        FactorAggregation::new(fine_counts, parents).expect("oracle aggregation is valid");
    let mut fine_tuples = Vec::new();
    let mut fine_weights = Vec::new();
    for (&tuple, &weight) in coarse.topology().tuples().iter().zip(coarse.weights()) {
        let child_weight = weight / (clones * clones * clones) as f64;
        for first_child in 0..clones {
            for second_child in 0..clones {
                for third_child in 0..clones {
                    fine_tuples.push([
                        (tuple[0] as usize * clones + first_child) as u32,
                        (tuple[1] as usize * clones + second_child) as u32,
                        (tuple[2] as usize * clones + third_child) as u32,
                    ]);
                    fine_weights.push(child_weight);
                }
            }
        }
    }
    let fine = ThreeWayProblem::from_observations(fine_counts, &fine_tuples, &fine_weights)
        .expect("refined weak chain is valid");
    (fine, aggregation)
}
