//! Deterministic factor-wise aggregation from shared tuple contexts.

use std::cmp::Ordering;
use std::collections::BTreeMap;

use crate::{FactorAggregation, MultiwayError, ThreeWayProblem};

/// Controls the first-generation shared-context affinity matcher.
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
        if !self.minimum_affinity.is_finite() || !(0.0..=1.0).contains(&self.minimum_affinity) {
            return Err(MultiwayError::InvalidOption {
                name: "minimum_affinity",
                message: format!(
                    "must be finite and lie in [0, 1], got {}",
                    self.minimum_affinity
                ),
            });
        }
        if self.maximum_context_degree < 2 {
            return Err(MultiwayError::InvalidOption {
                name: "maximum_context_degree",
                message: format!("must be at least two, got {}", self.maximum_context_degree),
            });
        }
        Ok(self)
    }
}

/// Build one deterministic aggregate matching per factor.
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
    let parents = core::array::from_fn(|factor| build_factor(problem, factor, options));
    FactorAggregation::new(counts, parents).map_err(Into::into)
}

fn build_factor(
    problem: &ThreeWayProblem,
    factor: usize,
    options: AffinityAggregationOptions,
) -> Vec<u32> {
    let counts = problem.topology().level_counts();
    let offsets = problem.topology().offsets();
    let level_count = counts[factor];
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

    let masses = &problem.diagonal()[offsets[factor]..offsets[factor + 1]];
    let mut overlaps: BTreeMap<(u32, u32), f64> = BTreeMap::new();
    for entries in contexts.values_mut() {
        entries.sort_by(|left, right| {
            right
                .1
                .total_cmp(&left.1)
                .then_with(|| left.0.cmp(&right.0))
        });
        entries.truncate(options.maximum_context_degree);
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

    let mut candidates = Vec::with_capacity(overlaps.len());
    for ((left, right), overlap) in overlaps {
        let denominator = masses[left as usize].sqrt() * masses[right as usize].sqrt();
        let affinity = if denominator == 0.0 {
            0.0
        } else {
            (overlap / denominator).clamp(0.0, 1.0)
        };
        if affinity >= options.minimum_affinity {
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

    let mut mate = vec![None; level_count];
    for candidate in candidates {
        let left = candidate.left as usize;
        let right = candidate.right as usize;
        if mate[left].is_none() && mate[right].is_none() {
            mate[left] = Some(right);
            mate[right] = Some(left);
        }
    }

    let mut parents = vec![u32::MAX; level_count];
    let mut next_parent = 0_u32;
    for level in 0..level_count {
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

#[derive(Debug, Clone, Copy)]
struct Candidate {
    affinity: f64,
    left: u32,
    right: u32,
}

impl PartialEq for Candidate {
    fn eq(&self, other: &Self) -> bool {
        self.affinity.to_bits() == other.affinity.to_bits()
            && self.left == other.left
            && self.right == other.right
    }
}

impl Eq for Candidate {}

impl PartialOrd for Candidate {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Candidate {
    fn cmp(&self, other: &Self) -> Ordering {
        self.affinity
            .total_cmp(&other.affinity)
            .then_with(|| self.left.cmp(&other.left))
            .then_with(|| self.right.cmp(&other.right))
    }
}
