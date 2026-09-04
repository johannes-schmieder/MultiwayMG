//! Frozen calibration and holdout graph-cover fixtures for issue #3.

use super::issue2_fixtures::{DynError, OracleCase, one_level_cases};
use multiway_mg::{FactorAggregation, ThreeWayProblem};

/// One predeclared sparse graph-cover fixture.
#[derive(Debug, Clone)]
pub struct Issue3Fixture {
    /// Calibration or holdout set.
    pub set: &'static str,
    /// Stable fixture name.
    pub name: String,
    /// Underlying issue #2 family.
    pub family: &'static str,
    /// Predeclared first seed.
    pub requested_seed: u64,
    /// First component-preserving seed at or after the requested seed.
    pub actual_seed: u64,
    /// Structurally invalid covers skipped before the retained seed.
    pub structural_skips: usize,
    /// Fine weighted three-way problem.
    pub problem: ThreeWayProblem,
    /// Exact cover projection back to the source coarse problem.
    pub oracle: FactorAggregation,
}

#[derive(Debug, Clone, Copy)]
struct FixtureSpec {
    set: &'static str,
    family: &'static str,
    requested_seed: u64,
}

/// Six searched calibration covers used to choose the first bootstrap policy.
pub fn calibration_fixtures() -> Result<Vec<Issue3Fixture>, DynError> {
    build_fixtures(&[
        FixtureSpec {
            set: "calibration",
            family: "dominant-pair-weak-third",
            requested_seed: 476,
        },
        FixtureSpec {
            set: "calibration",
            family: "dominant-pair-weak-third",
            requested_seed: 421,
        },
        FixtureSpec {
            set: "calibration",
            family: "nearly-nested",
            requested_seed: 175,
        },
        FixtureSpec {
            set: "calibration",
            family: "nearly-nested",
            requested_seed: 370,
        },
        FixtureSpec {
            set: "calibration",
            family: "nearly-nested",
            requested_seed: 118,
        },
        FixtureSpec {
            set: "calibration",
            family: "nearly-nested",
            requested_seed: 211,
        },
    ])
}

/// Fixed unseen-seed holdouts. Their seeds were declared after calibration
/// options and before their numerical results were evaluated.
pub fn holdout_fixtures() -> Result<Vec<Issue3Fixture>, DynError> {
    build_fixtures(&[
        FixtureSpec {
            set: "holdout",
            family: "dominant-pair-weak-third",
            requested_seed: 512,
        },
        FixtureSpec {
            set: "holdout",
            family: "dominant-pair-weak-third",
            requested_seed: 513,
        },
        FixtureSpec {
            set: "holdout",
            family: "nearly-nested",
            requested_seed: 514,
        },
        FixtureSpec {
            set: "holdout",
            family: "nearly-nested",
            requested_seed: 515,
        },
        FixtureSpec {
            set: "holdout",
            family: "weak-chain",
            requested_seed: 516,
        },
        FixtureSpec {
            set: "holdout",
            family: "planted-communities",
            requested_seed: 517,
        },
        FixtureSpec {
            set: "holdout",
            family: "hub-power-law",
            requested_seed: 518,
        },
        FixtureSpec {
            set: "holdout",
            family: "weight-dynamic-range",
            requested_seed: 519,
        },
        FixtureSpec {
            set: "holdout",
            family: "latin-square",
            requested_seed: 520,
        },
        FixtureSpec {
            set: "holdout",
            family: "tensor-grid",
            requested_seed: 521,
        },
    ])
}

fn build_fixtures(specifications: &[FixtureSpec]) -> Result<Vec<Issue3Fixture>, DynError> {
    let sources = one_level_cases()?;
    specifications
        .iter()
        .copied()
        .map(|specification| build_fixture(specification, &sources))
        .collect()
}

fn build_fixture(
    specification: FixtureSpec,
    sources: &[OracleCase],
) -> Result<Issue3Fixture, DynError> {
    let source = sources
        .iter()
        .find(|source| source.family == specification.family)
        .ok_or_else(|| format!("missing issue #2 family {}", specification.family))?;
    let source_map = source
        .maps
        .first()
        .ok_or_else(|| format!("family {} has no oracle map", specification.family))?;
    let base = source_map.coarsen(&source.problem)?;
    for actual_seed in specification.requested_seed..specification.requested_seed + 128 {
        let Some((problem, oracle)) = graph_cover_lift(&base, actual_seed)? else {
            continue;
        };
        return Ok(Issue3Fixture {
            set: specification.set,
            name: format!("{}-cover-{actual_seed}", specification.family),
            family: specification.family,
            requested_seed: specification.requested_seed,
            actual_seed,
            structural_skips: (actual_seed - specification.requested_seed) as usize,
            problem,
            oracle,
        });
    }
    Err(format!(
        "no component-preserving graph cover found for {} from seed {}",
        specification.family, specification.requested_seed
    )
    .into())
}

fn graph_cover_lift(
    base: &ThreeWayProblem,
    seed: u64,
) -> Result<Option<(ThreeWayProblem, FactorAggregation)>, DynError> {
    let base_counts = base.topology().level_counts();
    let fine_counts = base_counts.map(|count| count * 2);
    let parents = core::array::from_fn(|factor| {
        (0..fine_counts[factor])
            .map(|level| (level / 2) as u32)
            .collect()
    });
    let oracle = FactorAggregation::new(fine_counts, parents)?;
    let mut tuples = Vec::with_capacity(base.tuple_count() * 2);
    let mut weights = Vec::with_capacity(tuples.capacity());
    for (tuple_index, (&tuple, &weight)) in base
        .topology()
        .tuples()
        .iter()
        .zip(base.weights())
        .enumerate()
    {
        let mixed = mix(seed ^ (tuple_index as u64).wrapping_mul(0x9e37_79b9_7f4a_7c15));
        let second_shift = (mixed & 1) as u32;
        let third_shift = ((mixed >> 1) & 1) as u32;
        let scores = [
            0.75 + ((mixed >> 8) % 17) as f64 / 16.0,
            0.75 + ((mixed >> 16) % 17) as f64 / 16.0,
        ];
        let score_sum = scores[0] + scores[1];
        for child in 0..2_u32 {
            tuples.push([
                2 * tuple[0] + child,
                2 * tuple[1] + (child ^ second_shift),
                2 * tuple[2] + (child ^ third_shift),
            ]);
            weights.push(weight * scores[child as usize] / score_sum);
        }
    }
    let problem = ThreeWayProblem::from_observations(fine_counts, &tuples, &weights)?;
    if !oracle_respects_components(&problem, &oracle) {
        return Ok(None);
    }
    let reconstructed = oracle.coarsen(&problem)?;
    if reconstructed.topology().tuples() != base.topology().tuples()
        || reconstructed.topology().level_counts() != base.topology().level_counts()
    {
        return Ok(None);
    }
    for (&expected, &actual) in base.weights().iter().zip(reconstructed.weights()) {
        if (expected - actual).abs() > 1.0e-12 * expected.abs().max(actual.abs()).max(1.0) {
            return Ok(None);
        }
    }
    Ok(Some((problem, oracle)))
}

fn oracle_respects_components(
    problem: &ThreeWayProblem,
    oracle: &FactorAggregation,
) -> bool {
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

fn mix(mut value: u64) -> u64 {
    value = (value ^ (value >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    value = (value ^ (value >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
    value ^ (value >> 31)
}
