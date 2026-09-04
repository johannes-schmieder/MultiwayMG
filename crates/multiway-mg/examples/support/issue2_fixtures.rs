#![allow(dead_code)]

//! Deterministic oracle fixtures shared by the issue #2 research matrices.

use multiway_mg::{FactorAggregation, FactorPair, ThreeWayProblem};

pub type DynError = Box<dyn std::error::Error>;

#[derive(Debug, Clone, Copy)]
pub enum RefinementPattern {
    Complete,
    ParitySparse,
}

#[derive(Debug, Clone)]
pub struct OracleCase {
    pub name: String,
    pub problem: ThreeWayProblem,
    pub maps: Vec<FactorAggregation>,
    pub dominant_pair: FactorPair,
    pub family: &'static str,
    pub depth: usize,
}

pub fn one_level_cases() -> Result<Vec<OracleCase>, DynError> {
    let specifications = vec![
        (
            "planted-communities",
            planted_communities(4, 0.01)?,
            FactorPair::OneTwo,
            RefinementPattern::ParitySparse,
        ),
        (
            "dominant-pair-weak-third",
            dominant_pair_weak_third(6, 0.015)?,
            FactorPair::OneTwo,
            RefinementPattern::ParitySparse,
        ),
        (
            "weak-chain",
            weak_chain(6, 0.015)?,
            FactorPair::OneTwo,
            RefinementPattern::Complete,
        ),
        (
            "nearly-nested",
            nearly_nested(6, 0.015)?,
            FactorPair::OneTwo,
            RefinementPattern::ParitySparse,
        ),
        (
            "latin-square",
            latin_square(6, false)?,
            FactorPair::OneTwo,
            RefinementPattern::ParitySparse,
        ),
        (
            "tensor-grid",
            tensor_grid([3, 4, 5])?,
            FactorPair::OneThree,
            RefinementPattern::ParitySparse,
        ),
        (
            "hub-power-law",
            hub_power_law(6)?,
            FactorPair::OneTwo,
            RefinementPattern::ParitySparse,
        ),
        (
            "weight-dynamic-range",
            latin_square(6, true)?,
            FactorPair::OneTwo,
            RefinementPattern::ParitySparse,
        ),
    ];
    let mut cases = specifications
        .into_iter()
        .enumerate()
        .map(|(index, (name, base, dominant_pair, pattern))| {
            refine_case(name, base, 1, pattern, dominant_pair, index as u64)
        })
        .collect::<Result<Vec<_>, _>>()?;
    cases.push(ragged_disconnected_case()?);
    Ok(cases)
}

pub fn resolution_cases() -> Result<Vec<OracleCase>, DynError> {
    let mut cases = Vec::new();
    for depth in 2..=5 {
        cases.push(refine_case(
            &format!("weak-chain-depth-{depth}"),
            weak_chain(2, 0.01)?,
            depth,
            RefinementPattern::ParitySparse,
            FactorPair::OneTwo,
            100 + depth as u64,
        )?);
        cases.push(refine_case(
            &format!("community-depth-{depth}"),
            planted_communities(2, 0.01)?,
            depth,
            RefinementPattern::ParitySparse,
            FactorPair::OneTwo,
            200 + depth as u64,
        )?);
    }
    for depth in 2..=4 {
        cases.push(refine_case(
            &format!("latin-depth-{depth}"),
            latin_square(3, false)?,
            depth,
            RefinementPattern::ParitySparse,
            FactorPair::OneTwo,
            300 + depth as u64,
        )?);
    }
    Ok(cases)
}

pub fn deterministic_rhs(problem: &ThreeWayProblem) -> Result<Vec<f64>, DynError> {
    let mut coefficients: Vec<f64> = (0..problem.dimension())
        .map(|index| {
            let position = index as f64 + 1.0;
            (0.173 * position).sin()
                + 0.37 * (0.071 * position).cos()
                + 0.11 * (0.019 * position * position).sin()
        })
        .collect();
    problem
        .components()
        .project_structural_range(&mut coefficients)?;
    let mut rhs = vec![0.0; problem.dimension()];
    problem.apply_gramian(&coefficients, &mut rhs)?;
    Ok(rhs)
}

fn refine_case(
    name: &str,
    base: ThreeWayProblem,
    depth: usize,
    pattern: RefinementPattern,
    dominant_pair: FactorPair,
    seed: u64,
) -> Result<OracleCase, DynError> {
    let family = Box::leak(name.to_owned().into_boxed_str()) as &'static str;
    let mut current = base;
    let mut maps = Vec::with_capacity(depth);
    for level in 0..depth {
        let (fine, map) = refine_once(&current, pattern, seed + level as u64)?;
        let reconstructed = map.coarsen(&fine)?;
        verify_same_problem(&current, &reconstructed)?;
        maps.push(map);
        current = fine;
    }
    maps.reverse();
    Ok(OracleCase {
        name: name.to_owned(),
        problem: current,
        maps,
        dominant_pair,
        family,
        depth,
    })
}

fn refine_once(
    coarse: &ThreeWayProblem,
    pattern: RefinementPattern,
    seed: u64,
) -> Result<(ThreeWayProblem, FactorAggregation), DynError> {
    let coarse_counts = coarse.topology().level_counts();
    let fine_counts = coarse_counts.map(|count| count * 2);
    let parents = core::array::from_fn(|factor| {
        (0..fine_counts[factor])
            .map(|level| (level / 2) as u32)
            .collect()
    });
    let aggregation = FactorAggregation::new(fine_counts, parents)?;
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
        for first_child in 0..2 {
            for second_child in 0..2 {
                for third_child in 0..2 {
                    if matches!(pattern, RefinementPattern::ParitySparse)
                        && (first_child + second_child + third_child) % 2 != parity
                    {
                        continue;
                    }
                    let mixed = tuple_index
                        .wrapping_mul(131)
                        .wrapping_add(first_child * 17)
                        .wrapping_add(second_child * 11)
                        .wrapping_add(third_child * 5)
                        .wrapping_add(seed as usize);
                    let score = 0.75 + (mixed % 17) as f64 / 16.0;
                    children.push((first_child, second_child, third_child, score));
                    score_sum += score;
                }
            }
        }
        for (first_child, second_child, third_child, score) in children {
            tuples.push([
                tuple[0] * 2 + first_child as u32,
                tuple[1] * 2 + second_child as u32,
                tuple[2] * 2 + third_child as u32,
            ]);
            weights.push(coarse_weight * score / score_sum);
        }
    }
    let fine = ThreeWayProblem::from_observations(fine_counts, &tuples, &weights)?;
    Ok((fine, aggregation))
}

fn ragged_disconnected_case() -> Result<OracleCase, DynError> {
    let component_sizes = [2_usize, 3_usize];
    let component_clones = [4_usize, 2_usize];
    let fine_levels_per_factor =
        component_sizes[0] * component_clones[0] + component_sizes[1] * component_clones[1];
    let mut tuples = Vec::new();
    let mut weights = Vec::new();
    let mut fine_offset = 0_usize;
    for (component, (&levels, &clones)) in component_sizes.iter().zip(&component_clones).enumerate()
    {
        for first in 0..levels {
            for second in 0..levels {
                let third = (first + second) % levels;
                let base_weight =
                    0.8 + component as f64 * 0.4 + ((3 * first + 5 * second) % 7) as f64 / 10.0;
                let child_count = clones * clones * clones;
                for first_child in 0..clones {
                    for second_child in 0..clones {
                        for third_child in 0..clones {
                            tuples.push([
                                (fine_offset + first * clones + first_child) as u32,
                                (fine_offset + second * clones + second_child) as u32,
                                (fine_offset + third * clones + third_child) as u32,
                            ]);
                            weights.push(base_weight / child_count as f64);
                        }
                    }
                }
            }
        }
        fine_offset += levels * clones;
    }
    let problem =
        ThreeWayProblem::from_observations([fine_levels_per_factor; 3], &tuples, &weights)?;

    let first_map_parents = core::array::from_fn(|_| {
        let mut parents = Vec::with_capacity(fine_levels_per_factor);
        for level in 0..component_sizes[0] * component_clones[0] {
            parents.push((level / 2) as u32);
        }
        let first_component_mid = component_sizes[0] * 2;
        for level in 0..component_sizes[1] * component_clones[1] {
            parents.push((first_component_mid + level / 2) as u32);
        }
        parents
    });
    let first_map = FactorAggregation::new([fine_levels_per_factor; 3], first_map_parents)?;
    let mid = first_map.coarsen(&problem)?;
    let mid_counts = mid.topology().level_counts();
    let second_map_parents = core::array::from_fn(|_| {
        let mut parents = Vec::with_capacity(mid_counts[0]);
        for level in 0..component_sizes[0] * 2 {
            parents.push((level / 2) as u32);
        }
        for level in 0..component_sizes[1] {
            parents.push((component_sizes[0] + level) as u32);
        }
        parents
    });
    let second_map = FactorAggregation::new(mid_counts, second_map_parents)?;
    Ok(OracleCase {
        name: "disconnected-ragged-depth".to_owned(),
        problem,
        maps: vec![first_map, second_map],
        dominant_pair: FactorPair::OneTwo,
        family: "disconnected-ragged-depth",
        depth: 2,
    })
}

fn verify_same_problem(
    expected: &ThreeWayProblem,
    actual: &ThreeWayProblem,
) -> Result<(), DynError> {
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

fn planted_communities(levels: usize, bridge_weight: f64) -> Result<ThreeWayProblem, DynError> {
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

fn dominant_pair_weak_third(levels: usize, weak_weight: f64) -> Result<ThreeWayProblem, DynError> {
    let mut tuples = Vec::new();
    let mut weights = Vec::new();
    for first in 0..levels {
        for second in 0..levels {
            let third = (first + 2 * second) % levels;
            tuples.push([first as u32, second as u32, third as u32]);
            weights.push(1.0 + ((7 * first + 3 * second) % 13) as f64 / 10.0);
            tuples.push([first as u32, second as u32, ((third + 1) % levels) as u32]);
            weights.push(weak_weight);
        }
    }
    Ok(ThreeWayProblem::from_observations(
        [levels; 3],
        &tuples,
        &weights,
    )?)
}

fn weak_chain(levels: usize, bridge_weight: f64) -> Result<ThreeWayProblem, DynError> {
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

fn nearly_nested(levels: usize, perturbation_weight: f64) -> Result<ThreeWayProblem, DynError> {
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

fn latin_square(levels: usize, dynamic_range: bool) -> Result<ThreeWayProblem, DynError> {
    let mut tuples = Vec::new();
    let mut weights = Vec::new();
    for first in 0..levels {
        for second in 0..levels {
            tuples.push([
                first as u32,
                second as u32,
                ((first + second) % levels) as u32,
            ]);
            let weight = if dynamic_range {
                let exponent = ((7 * first + 11 * second) % 13) as i32 - 6;
                10.0_f64.powi(exponent)
            } else {
                0.8 + ((7 * first + 3 * second) % 13) as f64 / 10.0
            };
            weights.push(weight);
        }
    }
    Ok(ThreeWayProblem::from_observations(
        [levels; 3],
        &tuples,
        &weights,
    )?)
}

fn tensor_grid(counts: [usize; 3]) -> Result<ThreeWayProblem, DynError> {
    let mut tuples = Vec::new();
    let mut weights = Vec::new();
    for first in 0..counts[0] {
        for second in 0..counts[1] {
            for third in 0..counts[2] {
                tuples.push([first as u32, second as u32, third as u32]);
                weights.push(0.5 + ((5 * first + 7 * second + 11 * third) % 17) as f64 / 10.0);
            }
        }
    }
    Ok(ThreeWayProblem::from_observations(
        counts, &tuples, &weights,
    )?)
}

fn hub_power_law(levels: usize) -> Result<ThreeWayProblem, DynError> {
    let mut tuples = Vec::new();
    let mut weights = Vec::new();
    for first in 0..levels {
        for second in 0..levels {
            for third in 0..levels {
                if first != 0 && second != 0 && third != 0 && (first + second + third) % 5 != 0 {
                    continue;
                }
                tuples.push([first as u32, second as u32, third as u32]);
                let degree_scale = ((first + 1) * (second + 1) * (third + 1)) as f64;
                let hub_boost = if first == 0 || second == 0 || third == 0 {
                    8.0
                } else {
                    1.0
                };
                weights.push(hub_boost / degree_scale.sqrt());
            }
        }
    }
    Ok(ThreeWayProblem::from_observations(
        [levels; 3],
        &tuples,
        &weights,
    )?)
}
