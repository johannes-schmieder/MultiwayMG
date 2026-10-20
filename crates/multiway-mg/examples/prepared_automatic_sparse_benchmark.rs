//! Complete cold-process cost of the sparse-terminal policy against its controls.
//!
//! Reads one bounded `MG3AUT1` development input from stdin and runs one named
//! route through the whole one-shot driver: the frozen automatic policy, the
//! sparse-terminal policy with zero or one coarsening transition, component MAP,
//! or global MAP. Every route shares the frozen construction, screen and LSMR
//! options; only the transition cap and the sparse-terminal policy differ.
//! Diagnostic evidence only; no routing or default.
use multiway_incidence::{
    PreparedHierarchyBudget, PreparedThreeWayTopology, ThreeWayWeightFrame, WeightFrameInput,
};
use multiway_mg::*;
use std::{
    error::Error,
    io::{self, BufReader, Read},
    time::Instant,
};
type Result<T = ()> = std::result::Result<T, Box<dyn Error>>;
const MAX_PAYLOAD: usize = 1 << 30;
const ROUTES: [&str; 5] = [
    "automatic",
    "automatic-sparse-t0",
    "automatic-sparse-t1",
    "components-map",
    "global-map",
];

struct Input {
    counts: [usize; 3],
    tuples: Vec<[u32; 3]>,
    weights: Vec<f64>,
    targets: Vec<f64>,
    rhs: usize,
}

fn bytes<const N: usize>(r: &mut impl Read) -> Result<[u8; N]> {
    let mut b = [0; N];
    r.read_exact(&mut b)?;
    Ok(b)
}

fn decode(r: &mut impl Read) -> Result<Input> {
    if bytes::<8>(r)? != *b"MG3AUT1\0" {
        return Err("invalid automatic input magic".into());
    }
    let mut counts = [0; 3];
    for n in &mut counts {
        *n = u32::from_le_bytes(bytes(r)?) as usize;
    }
    let e = usize::try_from(u64::from_le_bytes(bytes(r)?))?;
    let rhs = u32::from_le_bytes(bytes(r)?) as usize;
    if counts.iter().any(|&n| n == 0 || n > 4096)
        || !(1..=100_000).contains(&e)
        || !(1..=32).contains(&rhs)
    {
        return Err("unsupported bounded development dimensions".into());
    }
    let mut tuples = vec![[0_u32; 3]; e];
    for key in &mut tuples {
        for x in key {
            *x = u32::from_le_bytes(bytes(r)?);
        }
    }
    let mut weights = vec![0.0; e];
    for w in &mut weights {
        *w = f64::from_le_bytes(bytes(r)?);
        if !w.is_finite() || *w <= 0. {
            return Err("invalid positive weight".into());
        }
    }
    let mut targets = vec![0.0; e * rhs];
    for y in &mut targets {
        *y = f64::from_le_bytes(bytes(r)?);
        if !y.is_finite() {
            return Err("invalid finite target".into());
        }
    }
    if r.read(&mut [0u8])? != 0 {
        return Err("trailing input".into());
    }
    Ok(Input {
        counts,
        tuples,
        weights,
        targets,
        rhs,
    })
}

/// The frozen `prepared-automatic-v1` options; only the transition cap varies.
fn options(transitions: usize) -> PreparedAutomaticOptions {
    PreparedAutomaticOptions {
        hierarchy: Some(PreparedAutomaticHierarchyOptions {
            maximum_transitions: transitions,
            maximum_tuple_multiplier: 4,
            maximum_coefficient_multiplier: 3,
            coverage: PreparedPairProposalCoverage::AdjacentPairs,
            candidate: PairNeighborhoodAggregationOptions {
                minimum_affinity: 0.02,
                maximum_neighbor_degree: 4,
            },
            screen: CycleQualityOptions {
                test_vectors: 2,
                power_iterations: 8,
                tail_iterations: 4,
                correction_damping: 1.,
                seed: 0x4d57_4d47_4359_4331,
                relative_zero_tolerance: 1e-13,
            },
            criteria: CycleQualityCriteria {
                maximum_estimated_energy_factor: 0.8,
                maximum_observed_energy_factor: Some(1.05),
                maximum_structural_defect: 1e-10,
            },
        }),
        terminal_relative_tolerance: 1e-12,
        fallback: PreparedBaselineKind::SymmetricMap,
        lsmr: PreparedLsmrOptions {
            tolerance: 1e-8,
            certificate_tolerance: 1e-8,
            max_iterations: 1000,
            local_size: Some(8),
        },
    }
}

fn sparse_policy() -> PreparedSparseTerminalOptions {
    PreparedSparseTerminalOptions {
        maximum_dimension: 4096,
        local_solver: within::LocalSolverConfig::default(),
        projector_tolerance: 1e-12,
    }
}

fn fingerprint(values: &[f64]) -> u64 {
    values.iter().fold(0xcbf2_9ce4_8422_2325_u64, |mut h, x| {
        for b in x.to_bits().to_le_bytes() {
            h = (h ^ u64::from(b)).wrapping_mul(0x0100_0000_01b3);
        }
        h
    })
}

fn main() -> Result {
    let route = std::env::args().nth(1).unwrap_or_default();
    if !ROUTES.contains(&route.as_str()) || std::env::args().count() != 2 {
        return Err("exactly one known route required".into());
    }
    let total = Instant::now();
    let started = Instant::now();
    let input = decode(&mut BufReader::with_capacity(65536, io::stdin().lock()))?;
    let decode_ns = started.elapsed().as_nanos();
    let started = Instant::now();
    let fine = PreparedThreeWayTopology::try_from_collapsed(input.counts, &input.tuples)?;
    let frame = ThreeWayWeightFrame::try_new(&fine, WeightFrameInput::Tuples(&input.weights))?;
    let prepare_ns = started.elapsed().as_nanos();
    let v: usize = input.counts.iter().sum();
    let k = input.rhs;
    let mut x = vec![0.0; v * k];
    let mut reports = vec![None; k];
    let mut progress = PreparedAutomaticProgress::default();
    let mut layout = PreparedAutomaticLayoutProgress::default();
    let budget = PreparedHierarchyBudget {
        maximum_payload_bytes: MAX_PAYLOAD,
        additional_live_payload_bytes: 12 * input.tuples.capacity() + 8 * input.weights.capacity(),
    };
    let started = Instant::now();
    let mut global_work = 0usize;
    match route.as_str() {
        "global-map" => {
            let owner = PreparedBaseline::new(&frame, PreparedBaselineKind::SymmetricMap);
            let lsmr = options(0).lsmr;
            let mut workspace = PreparedLsmrWorkspace::try_new_with_payload_budget(
                &owner,
                lsmr,
                MAX_PAYLOAD,
                budget.additional_live_payload_bytes,
            )?;
            let e = input.tuples.len();
            for j in 0..k {
                let solved = solve_prepared_least_squares_with_certificate_gate(
                    &owner,
                    &input.targets[j * e..(j + 1) * e],
                    &mut workspace,
                )?;
                global_work += solved.report.solve.work.weighted_incidence_applications;
                x[j * v..(j + 1) * v].copy_from_slice(solved.coefficients);
                reports[j] = Some(PreparedAutomaticColumnReport {
                    initial_certificate: None,
                    certified_normal_equation_residual: solved
                        .report
                        .solve
                        .certified_normal_equation_residual,
                    accepted: solved.report.solve.accepted,
                    global_fallback: false,
                });
            }
        }
        name => {
            let batch = PreparedAutomaticBatch {
                targets: &input.targets,
                columns: k,
                coefficients: &mut x,
                reports: &mut reports,
            };
            match name {
                "automatic" => solve_prepared_automatic_batch_into(
                    &frame,
                    batch,
                    options(8),
                    budget,
                    &mut progress,
                )?,
                "components-map" => {
                    let mut o = options(8);
                    o.hierarchy = None;
                    solve_prepared_automatic_batch_into(&frame, batch, o, budget, &mut progress)?;
                }
                sparse => solve_prepared_automatic_batch_into_with_sparse_terminal(
                    &frame,
                    batch,
                    options(usize::from(sparse == "automatic-sparse-t1")),
                    &sparse_policy(),
                    budget,
                    PreparedAutomaticLayout::Scalar,
                    &mut progress,
                    &mut layout,
                )?,
            }
        }
    }
    let driver_ns = started.elapsed().as_nanos();
    let total_ns = total.elapsed().as_nanos();
    let accepted = reports
        .iter()
        .filter(|r| r.is_some_and(|r| r.accepted))
        .count();
    let worst = reports
        .iter()
        .flatten()
        .map(|r| r.certified_normal_equation_residual)
        .fold(0.0_f64, f64::max);
    println!("route\t{route}");
    println!(
        "dimensions\t{}\t{}\t{}\t{}\t{k}",
        input.counts[0],
        input.counts[1],
        input.counts[2],
        input.tuples.len()
    );
    for (name, value) in [
        ("decode_ns", decode_ns),
        ("prepare_ns", prepare_ns),
        ("driver_ns", driver_ns),
        ("total_ns", total_ns),
    ] {
        println!("time\t{name}\t{value}");
    }
    let p = &progress;
    for (name, value) in [
        ("components", p.components),
        ("large_components", p.large_components),
        ("hierarchy_attempts", p.hierarchy_attempts),
        ("accepted_hierarchies", p.accepted_hierarchies),
        ("hierarchy_rejections", p.hierarchy_rejections),
        ("quality_rejections", p.quality_rejections),
        ("baseline_components", p.baseline_components),
        ("global_fallback_columns", p.global_fallback_columns),
        ("sparse_terminals", p.sparse_terminals),
        (
            "sparse_terminal_peak_bytes",
            p.maximum_sparse_terminal_peak_payload_bytes,
        ),
        ("maximum_admitted_bytes", p.maximum_admitted_payload_bytes),
        (
            "local_incidence_applications",
            p.solve_work.weighted_incidence_applications,
        ),
        ("global_incidence_applications", global_work),
        ("accepted_columns", accepted),
    ] {
        println!("count\t{name}\t{value}");
    }
    println!("certificate\t{worst:.17e}");
    println!("fingerprint\t{:016x}", fingerprint(&x));
    Ok(())
}
