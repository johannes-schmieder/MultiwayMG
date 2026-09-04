//! Frozen multi-level graph-cover fixtures for issue #3.

use std::error::Error;

use multiway_mg::{FactorAggregation, ThreeWayProblem};

/// Shared dynamic error type for research executables.
pub type DynError = Box<dyn Error>;

/// One exact multi-level graph-cover hierarchy.
pub struct RecursiveHoldoutFixture {
    /// Evidence-set label.
    pub set: &'static str,
    /// Stable case name.
    pub name: String,
    /// Structural family.
    pub family: &'static str,
    /// Requested unseen seed.
    pub requested_seed: u64,
    /// First structurally valid seed at or after the request.
    pub actual_seed: u64,
    /// Seeds skipped only because the exact cover violated structural validity.
    pub structural_skips: usize,
    /// Number of supplied oracle levels.
    pub depth: usize,
    /// Fine weighted problem.
    pub problem: ThreeWayProblem,
    /// Exact maps in finest-to-coarsest order.
    pub oracle_maps: Vec<FactorAggregation>,
    /// Coefficient dimension of the original base terminal.
    pub terminal_dimension: usize,
}

/// Build the predeclared seeds 800--807 without conditioning on numerical results.
pub fn recursive_holdout_fixtures() -> Result<Vec<RecursiveHoldoutFixture>, DynError> {
    let specifications = [
        ("recursive-latin", 800_u64, 2_usize),
        ("recursive-latin", 801, 3),
        ("recursive-weak-chain", 802, 2),
        ("recursive-weak-chain", 803, 3),
        ("recursive-nearly-nested", 804, 2),
        ("recursive-nearly-nested", 805, 3),
        ("recursive-dominant-pair", 806, 2),
        ("recursive-communities", 807, 3),
    ];
    specifications
        .into_iter()
        .map(|(family, requested_seed, depth)| build_fixture(family, requested_seed, depth))
        .collect()
}

fn build_fixture(
    family: &'static str,
    requested_seed: u64,
    depth: usize,
) -> Result<RecursiveHoldoutFixture, DynError> {
    let base = base_problem(family)?;
    let terminal_dimension = base.dimension();
    for structural_skips in 0..256_usize {
        let actual_seed = requested_seed.wrapping_add(structural_skips as u64);
        match recursively_lift(&base, depth, actual_seed) {
            Ok((problem, oracle_maps))
                if hierarchy_preserves_components(&problem, &oracle_maps) =>
            {
                return Ok(RecursiveHoldoutFixture {
                    set: "recursive-cycle-holdout-v1",
                    name: format!("{family}-depth-{depth}-seed-{requested_seed}"),
                    family,
                    requested_seed,
                    actual_seed,
                    structural_skips,
                    depth,
                    problem,
                    oracle_maps,
                    terminal_dimension,
                });
            }
            Ok(_) | Err(_) => {}
        }
    }
    Err(format!(
        "no structurally valid {family} depth-{depth} hierarchy found from seed {requested_seed}"
    )
    .into())
}

fn recursively_lift(
    base: &ThreeWayProblem,
    depth: usize,
    seed: u64,
) -> Result<(ThreeWayProblem, Vec<FactorAggregation>), DynError> {
    let mut current = base.clone();
    let mut coarse_to_fine = Vec::with_capacity(depth);
    for level in 0..depth {
        let level_seed = mix(
            seed ^ (level as u64).wrapping_mul(0xd6e8_feb8_6659_fd93)
                ^ (current.tuple_count() as u64).wrapping_mul(0xa076_1d64_78bd_642f),
        );
        let (fine, map) = lift_cover(&current, 2, level_seed)?;
        if !map_preserves_components(&fine, &map) {
            return Err("oracle map crosses a fine incidence component".into());
        }
        coarse_to_fine.push(map);
        current = fine;
    }
    coarse_to_fine.reverse();
    verify_hierarchy(base, &current, &coarse_to_fine)?;
    Ok((current, coarse_to_fine))
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
    let map = FactorAggregation::new(fine_counts, parents)?;
    let mut tuples = Vec::with_capacity(coarse.tuple_count() * sheets);
    let mut weights = Vec::with_capacity(tuples.capacity());

    for (tuple_index, (&tuple, &coarse_weight)) in coarse
        .topology()
        .tuples()
        .iter()
        .zip(coarse.weights())
        .enumerate()
    {
        let tuple_seed = mix(
            seed ^ (tuple_index as u64).wrapping_mul(0x9e37_79b9_7f4a_7c15)
                ^ (tuple[0] as u64).wrapping_mul(0xbf58_476d_1ce4_e5b9)
                ^ (tuple[1] as u64).wrapping_mul(0x94d0_49bb_1331_11eb)
                ^ (tuple[2] as u64).wrapping_mul(0xd6e8_feb8_6659_fd93),
        );
        let shift_second = ((tuple_seed >> 8) as usize) % sheets;
        let shift_third = ((tuple_seed >> 16) as usize) % sheets;
        let mut scores = Vec::with_capacity(sheets);
        let mut total_score = 0.0;
        for sheet in 0..sheets {
            let score_seed = mix(tuple_seed ^ (sheet as u64).wrapping_mul(0xa076_1d64_78bd_642f));
            let score = 0.75 + (score_seed % 17) as f64 / 16.0;
            scores.push(score);
            total_score += score;
        }
        for (sheet, &score) in scores.iter().enumerate() {
            tuples.push([
                tuple[0] * sheets as u32 + sheet as u32,
                tuple[1] * sheets as u32 + ((sheet + shift_second) % sheets) as u32,
                tuple[2] * sheets as u32 + ((sheet + shift_third) % sheets) as u32,
            ]);
            weights.push(coarse_weight * score / total_score);
        }
    }

    let fine = ThreeWayProblem::from_observations(fine_counts, &tuples, &weights)?;
    let reconstructed = map.coarsen(&fine)?;
    verify_same_problem(coarse, &reconstructed)?;
    Ok((fine, map))
}

fn verify_hierarchy(
    base: &ThreeWayProblem,
    fine: &ThreeWayProblem,
    maps: &[FactorAggregation],
) -> Result<(), DynError> {
    let mut current = fine.clone();
    for map in maps {
        current = map.coarsen(&current)?;
    }
    verify_same_problem(base, &current)
}

fn hierarchy_preserves_components(
    fine: &ThreeWayProblem,
    maps: &[FactorAggregation],
) -> bool {
    let mut current = fine.clone();
    for map in maps {
        if !map_preserves_components(&current, map) {
            return false;
        }
        let Ok(coarse) = map.coarsen(&current) else {
            return false;
        };
        current = coarse;
    }
    true
}

fn map_preserves_components(
    problem: &ThreeWayProblem,
    map: &FactorAggregation,
) -> bool {
    let counts = problem.topology().level_counts();
    for factor in 0..3 {
        let mut parent_components = vec![None; map.coarse_counts()[factor]];
        for level in 0..counts[factor] {
            let parent = map.parents(factor)[level] as usize;
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
        return Err("recursive cover did not reconstruct the base topology".into());
    }
    for (&left, &right) in expected.weights().iter().zip(actual.weights()) {
        let scale = left.abs().max(right.abs()).max(1.0);
        if (left - right).abs() > 4.0e-12 * scale {
            return Err(format!(
                "recursive cover weight mismatch: expected {left}, reconstructed {right}"
            )
            .into());
        }
    }
    Ok(())
}

fn base_problem(family: &str) -> Result<ThreeWayProblem, DynError> {
    match family {
        "recursive-latin" => latin_base(8),
        "recursive-weak-chain" => weak_chain_base(8, 0.01),
        "recursive-nearly-nested" => nearly_nested_base(8, 0.01),
        "recursive-dominant-pair" => dominant_pair_base(8, 0.02),
        "recursive-communities" => community_base(8, 0.01),
        _ => Err(format!("unknown recursive family {family}").into()),
    }
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
            tuples.push([
                first as u32,
                second as u32,
                ((first + 1) % levels) as u32,
            ]);
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
            tuples.push([
                first as u32,
                second as u32,
                ((third + 1) % levels) as u32,
            ]);
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
