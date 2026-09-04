#![allow(dead_code)]

//! Frozen graph-cover fixtures for the issue #3 complete-cycle holdout.
//!
//! Every base factor level is lifted to four sheets. Each base tuple receives
//! seed-dependent factor permutations, producing a sparse hypergraph cover
//! rather than a complete child tensor. The exact fiber map is retained as the
//! oracle aggregation. Requested seeds may advance only to satisfy structural
//! validity: every fine level must be used and each oracle aggregate must stay
//! inside one exact incidence component.

use std::error::Error;

use multiway_mg::{FactorAggregation, ThreeWayProblem};

/// Shared dynamic error type for research executables.
pub type DynError = Box<dyn Error>;

/// One frozen unseen graph-cover problem with its exact fiber map.
pub struct CycleHoldoutFixture {
    /// Evidence-set label.
    pub set: &'static str,
    /// Stable case name.
    pub name: String,
    /// Structural family.
    pub family: &'static str,
    /// Seed declared before numerical evaluation.
    pub requested_seed: u64,
    /// First structurally valid seed at or after the requested seed.
    pub actual_seed: u64,
    /// Number of seeds skipped solely for structural invalidity.
    pub structural_skips: usize,
    /// Fine weighted three-way problem.
    pub problem: ThreeWayProblem,
    /// Exact factor-fiber aggregation.
    pub oracle: FactorAggregation,
}

/// Build the predeclared seeds 700--709 without conditioning on solver results.
pub fn cycle_holdout_fixtures() -> Result<Vec<CycleHoldoutFixture>, DynError> {
    let specifications = [
        ("cover-latin", 700_u64),
        ("cover-latin", 701),
        ("cover-weak-chain", 702),
        ("cover-weak-chain", 703),
        ("cover-nearly-nested", 704),
        ("cover-nearly-nested", 705),
        ("cover-dominant-pair", 706),
        ("cover-dominant-pair", 707),
        ("cover-communities", 708),
        ("cover-communities", 709),
    ];
    specifications
        .into_iter()
        .map(|(family, requested_seed)| build_fixture(family, requested_seed))
        .collect()
}

fn build_fixture(
    family: &'static str,
    requested_seed: u64,
) -> Result<CycleHoldoutFixture, DynError> {
    for structural_skips in 0..256_usize {
        let actual_seed = requested_seed.wrapping_add(structural_skips as u64);
        let base = base_problem(family)?;
        match lift_cover(&base, 4, actual_seed) {
            Ok((problem, oracle)) if oracle_preserves_components(&problem, &oracle) => {
                return Ok(CycleHoldoutFixture {
                    set: "cycle-holdout-v2",
                    name: format!("{family}-seed-{requested_seed}"),
                    family,
                    requested_seed,
                    actual_seed,
                    structural_skips,
                    problem,
                    oracle,
                });
            }
            Ok(_) | Err(_) => {}
        }
    }
    Err(format!("no structurally valid {family} cover found from seed {requested_seed}").into())
}

fn base_problem(family: &str) -> Result<ThreeWayProblem, DynError> {
    match family {
        "cover-latin" => latin_base(8),
        "cover-weak-chain" => weak_chain_base(10, 0.01),
        "cover-nearly-nested" => nearly_nested_base(8, 0.01),
        "cover-dominant-pair" => dominant_pair_base(8, 0.02),
        "cover-communities" => community_base(8, 0.01),
        _ => Err(format!("unknown cycle holdout family {family}").into()),
    }
}

fn lift_cover(
    coarse: &ThreeWayProblem,
    sheets: usize,
    seed: u64,
) -> Result<(ThreeWayProblem, FactorAggregation), DynError> {
    let coarse_counts = coarse.topology().level_counts();
    let fine_counts = coarse_counts.map(|count| count * sheets);
    let parents = core::array::from_fn(|factor| {
        (0..fine_counts[factor])
            .map(|level| (level / sheets) as u32)
            .collect()
    });
    let oracle = FactorAggregation::new(fine_counts, parents)?;
    let mut fine_tuples = Vec::with_capacity(coarse.tuple_count() * sheets);
    let mut fine_weights = Vec::with_capacity(fine_tuples.capacity());

    for (tuple_index, (&tuple, &coarse_weight)) in coarse
        .topology()
        .tuples()
        .iter()
        .zip(coarse.weights())
        .enumerate()
    {
        let tuple_seed = mix(seed
            ^ (tuple_index as u64).wrapping_mul(0x9e37_79b9_7f4a_7c15)
            ^ (tuple[0] as u64).wrapping_mul(0xbf58_476d_1ce4_e5b9)
            ^ (tuple[1] as u64).wrapping_mul(0x94d0_49bb_1331_11eb)
            ^ (tuple[2] as u64).wrapping_mul(0xd6e8_feb8_6659_fd93));
        let multiplier_second = if tuple_seed & 1 == 0 { 1 } else { 3 };
        let multiplier_third = if tuple_seed & 2 == 0 { 1 } else { 3 };
        let shift_second = ((tuple_seed >> 8) as usize) % sheets;
        let shift_third = ((tuple_seed >> 16) as usize) % sheets;
        let mut scores = Vec::with_capacity(sheets);
        let mut score_sum = 0.0;
        for sheet in 0..sheets {
            let score_seed = mix(tuple_seed ^ (sheet as u64).wrapping_mul(0xa076_1d64_78bd_642f));
            let score = 0.75 + (score_seed % 17) as f64 / 16.0;
            scores.push(score);
            score_sum += score;
        }
        for (sheet, &score) in scores.iter().enumerate() {
            let second_sheet = (multiplier_second * sheet + shift_second) % sheets;
            let third_sheet = (multiplier_third * sheet + shift_third) % sheets;
            fine_tuples.push([
                tuple[0] * sheets as u32 + sheet as u32,
                tuple[1] * sheets as u32 + second_sheet as u32,
                tuple[2] * sheets as u32 + third_sheet as u32,
            ]);
            fine_weights.push(coarse_weight * score / score_sum);
        }
    }

    let fine = ThreeWayProblem::from_observations(fine_counts, &fine_tuples, &fine_weights)?;
    let reconstructed = oracle.coarsen(&fine)?;
    verify_same_problem(coarse, &reconstructed)?;
    Ok((fine, oracle))
}

fn oracle_preserves_components(problem: &ThreeWayProblem, oracle: &FactorAggregation) -> bool {
    let counts = problem.topology().level_counts();
    for factor in 0..3 {
        let mut parent_components = vec![None; oracle.coarse_counts()[factor]];
        for level in 0..counts[factor] {
            let parent = oracle.parents(factor)[level] as usize;
            let component = problem.components().component_of(factor, level);
            match parent_components[parent] {
                None => parent_components[parent] = Some(component),
                Some(existing) if existing == component => {}
                Some(_) => return false,
            }
        }
    }
    true
}

fn verify_same_problem(
    expected: &ThreeWayProblem,
    actual: &ThreeWayProblem,
) -> Result<(), DynError> {
    if expected.topology().level_counts() != actual.topology().level_counts()
        || expected.topology().tuples() != actual.topology().tuples()
        || expected.weights().len() != actual.weights().len()
    {
        return Err("graph-cover oracle did not reconstruct the coarse topology".into());
    }
    for (&left, &right) in expected.weights().iter().zip(actual.weights()) {
        let scale = left.abs().max(right.abs()).max(1.0);
        if (left - right).abs() > 2.0e-12 * scale {
            return Err(format!(
                "graph-cover weight mismatch: expected {left}, reconstructed {right}"
            )
            .into());
        }
    }
    Ok(())
}

fn latin_base(levels: usize) -> Result<ThreeWayProblem, DynError> {
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

fn weak_chain_base(levels: usize, bridge: f64) -> Result<ThreeWayProblem, DynError> {
    let mut tuples = Vec::new();
    let mut weights = Vec::new();
    for level in 0..levels {
        tuples.push([level as u32, level as u32, level as u32]);
        weights.push(1.0 + (level % 5) as f64 / 10.0);
        if level + 1 < levels {
            tuples.push([level as u32, (level + 1) as u32, (level + 1) as u32]);
            weights.push(bridge);
            tuples.push([(level + 1) as u32, level as u32, (level + 1) as u32]);
            weights.push(1.1 * bridge);
            tuples.push([(level + 1) as u32, (level + 1) as u32, level as u32]);
            weights.push(0.9 * bridge);
        }
    }
    Ok(ThreeWayProblem::from_observations(
        [levels; 3],
        &tuples,
        &weights,
    )?)
}

fn nearly_nested_base(levels: usize, perturbation: f64) -> Result<ThreeWayProblem, DynError> {
    let mut tuples = Vec::new();
    let mut weights = Vec::new();
    for first in 0..levels {
        for second in 0..levels {
            tuples.push([first as u32, second as u32, first as u32]);
            weights.push(1.0 + ((first + 2 * second) % 7) as f64 / 10.0);
            tuples.push([first as u32, second as u32, ((first + 1) % levels) as u32]);
            weights.push(perturbation);
        }
    }
    Ok(ThreeWayProblem::from_observations(
        [levels; 3],
        &tuples,
        &weights,
    )?)
}

fn dominant_pair_base(levels: usize, weak: f64) -> Result<ThreeWayProblem, DynError> {
    let mut tuples = Vec::new();
    let mut weights = Vec::new();
    for first in 0..levels {
        for second in 0..levels {
            let third = (first + 3 * second) % levels;
            tuples.push([first as u32, second as u32, third as u32]);
            weights.push(1.0 + ((5 * first + 7 * second) % 11) as f64 / 20.0);
            tuples.push([first as u32, second as u32, ((third + 1) % levels) as u32]);
            weights.push(weak);
        }
    }
    Ok(ThreeWayProblem::from_observations(
        [levels; 3],
        &tuples,
        &weights,
    )?)
}

fn community_base(levels: usize, bridge: f64) -> Result<ThreeWayProblem, DynError> {
    let half = levels / 2;
    let mut tuples = Vec::new();
    let mut weights = Vec::new();
    for first in 0..levels {
        for second in 0..levels {
            let same_half = (first < half) == (second < half);
            let offset = if first < half { 0 } else { half };
            let third = if same_half {
                offset + (first + second) % half
            } else {
                (first + second) % levels
            };
            tuples.push([first as u32, second as u32, third as u32]);
            weights.push(if same_half {
                1.0 + ((3 * first + 5 * second) % 7) as f64 / 10.0
            } else {
                bridge
            });
        }
    }
    Ok(ThreeWayProblem::from_observations(
        [levels; 3],
        &tuples,
        &weights,
    )?)
}

fn mix(mut value: u64) -> u64 {
    value = value.wrapping_add(0x9e37_79b9_7f4a_7c15);
    value = (value ^ (value >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    value = (value ^ (value >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
    value ^ (value >> 31)
}
