#![allow(dead_code)]

//! Frozen deterministic pair-domain fixtures for issue #4.

use multiway_mg::PairDomain;

pub type DynError = Box<dyn std::error::Error>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PairSuite {
    Calibration,
    Holdout,
}

impl PairSuite {
    pub const fn label(self) -> &'static str {
        match self {
            Self::Calibration => "calibration",
            Self::Holdout => "holdout",
        }
    }
}

#[derive(Debug, Clone)]
pub struct PairCase {
    pub name: &'static str,
    pub family: &'static str,
    pub interpretation: &'static str,
    pub domain: PairDomain,
}

pub fn small_cases(suite: PairSuite) -> Result<Vec<PairCase>, DynError> {
    let cases = match suite {
        PairSuite::Calibration => vec![
            PairCase {
                name: "cal-path-balanced",
                family: "path",
                interpretation: "balanced low-mobility chain",
                domain: path_domain(18, 0, 101)?,
            },
            PairCase {
                name: "cal-path-dynamic",
                family: "path-dynamic-range",
                interpretation: "low-mobility chain with eight decades of edge weights",
                domain: path_domain(18, 4, 103)?,
            },
            PairCase {
                name: "cal-tree-dominant-left",
                family: "tree-dominant-factor",
                interpretation: "worker-heavy tree with one dominant factor",
                domain: tree_domain(42, 9, 2, 107)?,
            },
            PairCase {
                name: "cal-hub-power-law",
                family: "hub-power-law",
                interpretation: "hub-heavy mobility graph with a long degree tail",
                domain: hub_domain(30, 14, 4, 2, 109)?,
            },
            PairCase {
                name: "cal-weak-communities",
                family: "weak-communities",
                interpretation: "three dense communities joined by weak bridges",
                domain: community_domain(24, 24, 3, 3, 1.0e-4, 1, 113)?,
            },
            PairCase {
                name: "cal-dense-balanced",
                family: "dense",
                interpretation: "balanced complete bipartite component",
                domain: dense_domain(15, 13, 0, 127)?,
            },
            PairCase {
                name: "cal-dense-dynamic",
                family: "dense-dynamic-range",
                interpretation: "dense component with eight decades of edge weights",
                domain: dense_domain(12, 12, 4, 131)?,
            },
            PairCase {
                name: "cal-nearly-nested",
                family: "nearly-nested",
                interpretation: "mostly nested worker groups with weak mobility bridges",
                domain: nearly_nested_domain(36, 12, 2.0e-4, 2, 137)?,
            },
        ],
        PairSuite::Holdout => vec![
            PairCase {
                name: "holdout-path-ragged-weights",
                family: "path-dynamic-range",
                interpretation: "fresh weighted path with unseen deterministic weight phase",
                domain: path_domain(21, 3, 907)?,
            },
            PairCase {
                name: "holdout-tree-dominant-right",
                family: "tree-dominant-factor",
                interpretation: "fresh tree with the second factor dominant",
                domain: tree_domain(10, 43, 2, 911)?,
            },
            PairCase {
                name: "holdout-hub-asymmetric",
                family: "hub-power-law",
                interpretation: "fresh asymmetric hub graph",
                domain: hub_domain(34, 11, 5, 2, 919)?,
            },
            PairCase {
                name: "holdout-four-communities",
                family: "weak-communities",
                interpretation: "four communities with a new bridge pattern",
                domain: community_domain(28, 28, 4, 3, 3.0e-5, 1, 929)?,
            },
            PairCase {
                name: "holdout-dense-asymmetric",
                family: "dense",
                interpretation: "fresh asymmetric complete bipartite component",
                domain: dense_domain(17, 11, 1, 937)?,
            },
            PairCase {
                name: "holdout-dense-high-dynamic",
                family: "dense-dynamic-range",
                interpretation: "fresh dense graph with ten decades of edge weights",
                domain: dense_domain(11, 13, 5, 941)?,
            },
            PairCase {
                name: "holdout-nearly-nested-ragged",
                family: "nearly-nested",
                interpretation: "fresh ragged near-nesting with weak bridges",
                domain: nearly_nested_domain(41, 13, 7.0e-5, 2, 947)?,
            },
        ],
    };
    Ok(cases)
}

pub const LARGE_CALIBRATION_CASES: [&str; 6] = [
    "cal-large-path-dynamic",
    "cal-large-tree-dominant-left",
    "cal-large-hub-power-law",
    "cal-large-weak-communities",
    "cal-large-dense",
    "cal-large-nearly-nested",
];

pub const LARGE_HOLDOUT_CASES: [&str; 6] = [
    "holdout-large-path-ragged",
    "holdout-large-tree-dominant-right",
    "holdout-large-hub-asymmetric",
    "holdout-large-seven-communities",
    "holdout-large-dense-asymmetric",
    "holdout-large-nearly-nested-ragged",
];

pub fn large_case_names(suite: PairSuite) -> &'static [&'static str] {
    match suite {
        PairSuite::Calibration => &LARGE_CALIBRATION_CASES,
        PairSuite::Holdout => &LARGE_HOLDOUT_CASES,
    }
}

pub fn large_case(suite: PairSuite, name: &str) -> Result<PairCase, DynError> {
    let case = match (suite, name) {
        (PairSuite::Calibration, "cal-large-path-dynamic") => PairCase {
            name: "cal-large-path-dynamic",
            family: "path-dynamic-range",
            interpretation: "40k-vertex low-mobility path with eight decades of weights",
            domain: path_domain(20_000, 4, 20_003)?,
        },
        (PairSuite::Calibration, "cal-large-tree-dominant-left") => PairCase {
            name: "cal-large-tree-dominant-left",
            family: "tree-dominant-factor",
            interpretation: "48k-vertex worker-heavy pair tree",
            domain: tree_domain(40_000, 8_000, 3, 20_011)?,
        },
        (PairSuite::Calibration, "cal-large-hub-power-law") => PairCase {
            name: "cal-large-hub-power-law",
            family: "hub-power-law",
            interpretation: "28k-vertex hub graph with a long degree tail",
            domain: hub_domain(20_000, 8_000, 256, 3, 20_021)?,
        },
        (PairSuite::Calibration, "cal-large-weak-communities") => PairCase {
            name: "cal-large-weak-communities",
            family: "weak-communities",
            interpretation: "24k-vertex six-community graph with weak global bridges",
            domain: community_domain(12_000, 12_000, 6, 6, 1.0e-6, 2, 20_033)?,
        },
        (PairSuite::Calibration, "cal-large-dense") => PairCase {
            name: "cal-large-dense",
            family: "dense",
            interpretation: "420k-edge asymmetric dense component",
            domain: dense_domain(700, 600, 2, 20_047)?,
        },
        (PairSuite::Calibration, "cal-large-nearly-nested") => PairCase {
            name: "cal-large-nearly-nested",
            family: "nearly-nested",
            interpretation: "48k-vertex near-nesting with weak mobility bridges",
            domain: nearly_nested_domain(40_000, 8_000, 1.0e-6, 3, 20_059)?,
        },
        (PairSuite::Holdout, "holdout-large-path-ragged") => PairCase {
            name: "holdout-large-path-ragged",
            family: "path-dynamic-range",
            interpretation: "fresh 46k-vertex weighted path",
            domain: path_domain(23_000, 5, 90_007)?,
        },
        (PairSuite::Holdout, "holdout-large-tree-dominant-right") => PairCase {
            name: "holdout-large-tree-dominant-right",
            family: "tree-dominant-factor",
            interpretation: "fresh 51.5k-vertex tree with the second factor dominant",
            domain: tree_domain(8_500, 43_000, 3, 90_011)?,
        },
        (PairSuite::Holdout, "holdout-large-hub-asymmetric") => PairCase {
            name: "holdout-large-hub-asymmetric",
            family: "hub-power-law",
            interpretation: "fresh asymmetric 27k-vertex hub graph",
            domain: hub_domain(18_000, 9_000, 384, 3, 90_019)?,
        },
        (PairSuite::Holdout, "holdout-large-seven-communities") => PairCase {
            name: "holdout-large-seven-communities",
            family: "weak-communities",
            interpretation: "fresh seven-community graph with a new bridge scale",
            domain: community_domain(14_000, 14_000, 7, 7, 3.0e-7, 2, 90_029)?,
        },
        (PairSuite::Holdout, "holdout-large-dense-asymmetric") => PairCase {
            name: "holdout-large-dense-asymmetric",
            family: "dense",
            interpretation: "fresh 471k-edge asymmetric dense component",
            domain: dense_domain(620, 760, 3, 90_037)?,
        },
        (PairSuite::Holdout, "holdout-large-nearly-nested-ragged") => PairCase {
            name: "holdout-large-nearly-nested-ragged",
            family: "nearly-nested",
            interpretation: "fresh 51k-vertex ragged near-nesting",
            domain: nearly_nested_domain(42_000, 9_000, 3.0e-7, 3, 90_047)?,
        },
        _ => return Err(format!("unknown {} large pair case {name:?}", suite.label()).into()),
    };
    Ok(case)
}

pub fn path_domain(n: usize, exponent_span: i32, seed: u64) -> Result<PairDomain, DynError> {
    let mut edges = Vec::with_capacity(2 * n - 1);
    for index in 0..n {
        edges.push((
            index as u32,
            index as u32,
            edge_weight(seed, index, index, exponent_span),
        ));
        if index + 1 < n {
            edges.push((
                (index + 1) as u32,
                index as u32,
                0.7 * edge_weight(seed ^ 0x9e37, index + 1, index, exponent_span),
            ));
        }
    }
    Ok(PairDomain::from_edges(n, n, edges)?)
}

pub fn tree_domain(
    left_count: usize,
    right_count: usize,
    exponent_span: i32,
    seed: u64,
) -> Result<PairDomain, DynError> {
    let mut edges = Vec::with_capacity(left_count + right_count - 1);
    for left in 0..left_count {
        edges.push((left as u32, 0, edge_weight(seed, left, 0, exponent_span)));
    }
    for right in 1..right_count {
        let left = mixed_index(seed, right, left_count);
        edges.push((
            left as u32,
            right as u32,
            edge_weight(seed ^ 0x51ed, left, right, exponent_span),
        ));
    }
    Ok(PairDomain::from_edges(left_count, right_count, edges)?)
}

pub fn hub_domain(
    left_count: usize,
    right_count: usize,
    extra_scale: usize,
    exponent_span: i32,
    seed: u64,
) -> Result<PairDomain, DynError> {
    let mut edges = Vec::new();
    for left in 0..left_count {
        edges.push((left as u32, 0, edge_weight(seed, left, 0, exponent_span)));
    }
    for right in 1..right_count {
        edges.push((
            0,
            right as u32,
            edge_weight(seed ^ 0x1337, 0, right, exponent_span),
        ));
    }
    for left in 1..left_count {
        let degree = 1 + extra_scale / integer_sqrt(left + 1).max(1);
        for offset in 0..degree {
            let right = 1 + mixed_index(seed ^ left as u64, offset, right_count - 1);
            edges.push((
                left as u32,
                right as u32,
                0.6 * edge_weight(seed ^ 0xa5a5, left, right, exponent_span),
            ));
        }
    }
    Ok(PairDomain::from_edges(left_count, right_count, edges)?)
}

#[allow(clippy::too_many_arguments)]
pub fn community_domain(
    left_count: usize,
    right_count: usize,
    communities: usize,
    fanout: usize,
    bridge_weight: f64,
    exponent_span: i32,
    seed: u64,
) -> Result<PairDomain, DynError> {
    assert!(communities > 1);
    assert!(left_count >= communities && right_count >= communities);
    let mut edges = Vec::new();
    for left in 0..left_count {
        let community = left * communities / left_count;
        let right_start = community * right_count / communities;
        let right_end = (community + 1) * right_count / communities;
        let width = right_end - right_start;
        for offset in 0..fanout.min(width) {
            let right = right_start + (mixed_index(seed ^ left as u64, offset, width));
            edges.push((
                left as u32,
                right as u32,
                edge_weight(seed ^ 0xcafe, left, right, exponent_span),
            ));
        }
    }
    // Ensure every right vertex is covered inside its own community.
    for right in 0..right_count {
        let community = right * communities / right_count;
        let left_start = community * left_count / communities;
        edges.push((
            left_start as u32,
            right as u32,
            0.8 * edge_weight(seed ^ 0xdead, left_start, right, exponent_span),
        ));
    }
    // A chain of weak cross-community bridges makes the graph connected.
    for community in 0..communities - 1 {
        let left = ((community + 1) * left_count / communities) - 1;
        let right = (community + 1) * right_count / communities;
        edges.push((left as u32, right as u32, bridge_weight));
    }
    Ok(PairDomain::from_edges(left_count, right_count, edges)?)
}

pub fn dense_domain(
    left_count: usize,
    right_count: usize,
    exponent_span: i32,
    seed: u64,
) -> Result<PairDomain, DynError> {
    let mut edges = Vec::with_capacity(left_count * right_count);
    for left in 0..left_count {
        for right in 0..right_count {
            edges.push((
                left as u32,
                right as u32,
                edge_weight(seed, left, right, exponent_span),
            ));
        }
    }
    Ok(PairDomain::from_edges(left_count, right_count, edges)?)
}

pub fn nearly_nested_domain(
    left_count: usize,
    right_count: usize,
    bridge_weight: f64,
    exponent_span: i32,
    seed: u64,
) -> Result<PairDomain, DynError> {
    assert!(left_count >= right_count);
    let mut edges = Vec::with_capacity(left_count + right_count - 1);
    for left in 0..left_count {
        let right = left * right_count / left_count;
        edges.push((
            left as u32,
            right as u32,
            edge_weight(seed, left, right, exponent_span),
        ));
    }
    for right in 0..right_count - 1 {
        let left = right * left_count / right_count;
        edges.push((left as u32, (right + 1) as u32, bridge_weight));
    }
    Ok(PairDomain::from_edges(left_count, right_count, edges)?)
}

pub fn deterministic_range_rhs(domain: &PairDomain, phase: f64) -> Result<Vec<f64>, DynError> {
    let mut coefficients: Vec<f64> = (0..domain.dimension())
        .map(|index| {
            let position = index as f64 + 1.0;
            (phase * position).sin()
                + 0.37 * (0.071 * position).cos()
                + 0.11 * (0.019 * position * position).sin()
        })
        .collect();
    domain.project_range_in_place(&mut coefficients)?;
    let mut rhs = vec![0.0; domain.dimension()];
    domain.apply_gramian(&coefficients, &mut rhs)?;
    Ok(rhs)
}

fn edge_weight(seed: u64, left: usize, right: usize, exponent_span: i32) -> f64 {
    let mixed = mix64(
        seed ^ (left as u64).wrapping_mul(0x9e37_79b9_7f4a_7c15)
            ^ (right as u64).wrapping_mul(0xbf58_476d_1ce4_e5b9),
    );
    let mantissa = 0.75 + ((mixed >> 16) & 0xffff) as f64 / 65_535.0;
    if exponent_span == 0 {
        return mantissa;
    }
    let width = (2 * exponent_span + 1) as u64;
    let exponent = (mixed % width) as i32 - exponent_span;
    mantissa * 10.0_f64.powi(exponent)
}

fn mixed_index(seed: u64, index: usize, modulus: usize) -> usize {
    assert!(modulus > 0);
    (mix64(seed ^ index as u64) as usize) % modulus
}

fn integer_sqrt(value: usize) -> usize {
    (value as f64).sqrt() as usize
}

fn mix64(mut value: u64) -> u64 {
    value ^= value >> 30;
    value = value.wrapping_mul(0xbf58_476d_1ce4_e5b9);
    value ^= value >> 27;
    value = value.wrapping_mul(0x94d0_49bb_1331_11eb);
    value ^ (value >> 31)
}
