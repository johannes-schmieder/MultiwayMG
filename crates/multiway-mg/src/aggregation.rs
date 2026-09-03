//! Deterministic factor-wise aggregation from sparse tuple neighborhoods.

use std::collections::BTreeMap;

use crate::{FactorAggregation, MultiwayError, ThreeWayProblem};

/// Controls the first-generation exact shared-context matcher.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AffinityAggregationOptions {
    /// Minimum normalized shared-context mass required to pair two levels.
    pub minimum_affinity: f64,
    /// Maximum number of levels retained from one context when generating
    /// pair candidates. The heaviest entries are retained deterministically.
    pub maximum_context_degree: usize,
}

impl Default for AffinityAggregationOptions {
    fn default() -> Self {
        Self {
            minimum_affinity: 0.15,
            maximum_context_degree: 16,
        }
    }
}

impl AffinityAggregationOptions {
    fn validate(self) -> Result<Self, MultiwayError> {
        validate_options(
            "minimum_affinity",
            self.minimum_affinity,
            "maximum_context_degree",
            self.maximum_context_degree,
        )?;
        Ok(self)
    }
}

/// Controls a broader matcher based on shared neighbors in either pair marginal.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PairNeighborhoodAggregationOptions {
    /// Minimum normalized pair-neighborhood overlap required to pair two levels.
    pub minimum_affinity: f64,
    /// Maximum number of incident levels retained at one pair-graph neighbor.
    /// This bounds candidate generation by a constant per neighbor.
    pub maximum_neighbor_degree: usize,
}

impl Default for PairNeighborhoodAggregationOptions {
    fn default() -> Self {
        Self {
            minimum_affinity: 0.02,
            maximum_neighbor_degree: 16,
        }
    }
}

impl PairNeighborhoodAggregationOptions {
    fn validate(self) -> Result<Self, MultiwayError> {
        validate_options(
            "minimum_pair_neighborhood_affinity",
            self.minimum_affinity,
            "maximum_neighbor_degree",
            self.maximum_neighbor_degree,
        )?;
        Ok(self)
    }
}

/// Build one deterministic exact-context matching per factor.
///
/// Two levels in factor `q` become candidates when they occur with the same
/// pair of levels in the other factors. Candidate strength is shared tuple
/// mass divided by the geometric mean of the two level masses. Greedy matching
/// uses descending affinity and canonical level IDs to make ties reproducible.
pub fn build_affinity_aggregation(
    problem: &ThreeWayProblem,
    options: AffinityAggregationOptions,
) -> Result<FactorAggregation, MultiwayError> {
    let options = options.validate()?;
    let counts = problem.topology().level_counts();
    let parents = core::array::from_fn(|factor| build_context_factor(problem, factor, options));
    FactorAggregation::new(counts, parents).map_err(Into::into)
}

/// Build one deterministic pair-neighborhood matching per factor.
///
/// For a level in factor `q`, each of the other two factor marginals supplies a
/// sparse neighbor list. Two levels become candidates when they share a
/// retained neighbor in either marginal. Candidate generation is bounded by
/// [`PairNeighborhoodAggregationOptions::maximum_neighbor_degree`] at every
/// neighbor and never joins different incidence components.
///
/// This is deliberately a structural fallback rather than the planned final
/// adaptive method. It broadens exact-context matching for Latin-square-like
/// patterns while retaining near-linear setup for bounded neighborhood degree.
pub fn build_pair_neighborhood_aggregation(
    problem: &ThreeWayProblem,
    options: PairNeighborhoodAggregationOptions,
) -> Result<FactorAggregation, MultiwayError> {
    let options = options.validate()?;
    let counts = problem.topology().level_counts();
    let parents =
        core::array::from_fn(|factor| build_neighborhood_factor(problem, factor, options));
    FactorAggregation::new(counts, parents).map_err(Into::into)
}

fn build_context_factor(
    problem: &ThreeWayProblem,
    factor: usize,
    options: AffinityAggregationOptions,
) -> Vec<u32> {
    let other = match factor {
        0 => [1, 2],
        1 => [0, 2],
        2 => [0, 1],
        _ => unreachable!("three-way factor index"),
    };
    let mut contexts: BTreeMap<(u32, u32), Vec<(u32, f64)>> = BTreeMap::new();
    for (&tuple, &weight) in problem.topology().tuples().iter().zip(problem.weights()) {
        contexts
            .entry((tuple[other[0]], tuple[other[1]]))
            .or_default()
            .push((tuple[factor], weight));
    }

    let mut overlaps = BTreeMap::new();
    for entries in contexts.values_mut() {
        accumulate_bounded_pairs(
            problem,
            factor,
            entries,
            options.maximum_context_degree,
            &mut overlaps,
        );
    }
    matching_from_overlaps(problem, factor, options.minimum_affinity, overlaps)
}

fn build_neighborhood_factor(
    problem: &ThreeWayProblem,
    factor: usize,
    options: PairNeighborhoodAggregationOptions,
) -> Vec<u32> {
    let counts = problem.topology().level_counts();
    let mut overlaps = BTreeMap::new();
    for neighbor_factor in 0..3 {
        if neighbor_factor == factor {
            continue;
        }
        let mut neighborhoods: Vec<BTreeMap<u32, f64>> = (0..counts[neighbor_factor])
            .map(|_| BTreeMap::new())
            .collect();
        for (&tuple, &weight) in problem.topology().tuples().iter().zip(problem.weights()) {
            *neighborhoods[tuple[neighbor_factor] as usize]
                .entry(tuple[factor])
                .or_insert(0.0) += weight;
        }
        for neighborhood in neighborhoods {
            let mut entries: Vec<(u32, f64)> = neighborhood.into_iter().collect();
            accumulate_bounded_pairs(
                problem,
                factor,
                &mut entries,
                options.maximum_neighbor_degree,
                &mut overlaps,
            );
        }
    }
    matching_from_overlaps(problem, factor, options.minimum_affinity, overlaps)
}

fn accumulate_bounded_pairs(
    problem: &ThreeWayProblem,
    factor: usize,
    entries: &mut Vec<(u32, f64)>,
    maximum_degree: usize,
    overlaps: &mut BTreeMap<(u32, u32), f64>,
) {
    entries.sort_by(|left, right| {
        right
            .1
            .total_cmp(&left.1)
            .then_with(|| left.0.cmp(&right.0))
    });
    entries.truncate(maximum_degree);
    entries.sort_by_key(|entry| entry.0);
    for left in 0..entries.len() {
        for right in (left + 1)..entries.len() {
            let a = entries[left].0;
            let b = entries[right].0;
            if problem.components().component_of(factor, a as usize)
                != problem.components().component_of(factor, b as usize)
            {
                continue;
            }
            let key = (a.min(b), a.max(b));
            *overlaps.entry(key).or_insert(0.0) += entries[left].1.min(entries[right].1);
        }
    }
}

fn matching_from_overlaps(
    problem: &ThreeWayProblem,
    factor: usize,
    minimum_affinity: f64,
    overlaps: BTreeMap<(u32, u32), f64>,
) -> Vec<u32> {
    let counts = problem.topology().level_counts();
    let offsets = problem.topology().offsets();
    let masses = &problem.diagonal()[offsets[factor]..offsets[factor + 1]];
    let mut candidates = Vec::with_capacity(overlaps.len());
    for ((left, right), overlap) in overlaps {
        let denominator = masses[left as usize].sqrt() * masses[right as usize].sqrt();
        let affinity = if denominator == 0.0 {
            0.0
        } else {
            (overlap / denominator).clamp(0.0, 1.0)
        };
        if affinity >= minimum_affinity {
            candidates.push(Candidate {
                affinity,
                left,
                right,
            });
        }
    }
    candidates.sort_by(|left, right| {
        right
            .affinity
            .total_cmp(&left.affinity)
            .then_with(|| left.left.cmp(&right.left))
            .then_with(|| left.right.cmp(&right.right))
    });

    let mut mate = vec![None; counts[factor]];
    for candidate in candidates {
        let left = candidate.left as usize;
        let right = candidate.right as usize;
        if mate[left].is_none() && mate[right].is_none() {
            mate[left] = Some(right);
            mate[right] = Some(left);
        }
    }

    let mut parents = vec![u32::MAX; counts[factor]];
    let mut next_parent = 0_u32;
    for level in 0..counts[factor] {
        if parents[level] != u32::MAX {
            continue;
        }
        parents[level] = next_parent;
        if let Some(other_level) = mate[level] {
            parents[other_level] = next_parent;
        }
        next_parent += 1;
    }
    parents
}

fn validate_options(
    affinity_name: &'static str,
    minimum_affinity: f64,
    degree_name: &'static str,
    maximum_degree: usize,
) -> Result<(), MultiwayError> {
    if !minimum_affinity.is_finite() || !(0.0..=1.0).contains(&minimum_affinity) {
        return Err(MultiwayError::InvalidOption {
            name: affinity_name,
            message: format!("must be finite and lie in [0, 1], got {minimum_affinity}"),
        });
    }
    if maximum_degree < 2 {
        return Err(MultiwayError::InvalidOption {
            name: degree_name,
            message: format!("must be at least two, got {maximum_degree}"),
        });
    }
    Ok(())
}

#[derive(Debug, Clone, Copy)]
struct Candidate {
    affinity: f64,
    left: u32,
    right: u32,
}
