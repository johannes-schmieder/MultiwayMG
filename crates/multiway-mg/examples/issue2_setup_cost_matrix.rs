//! Phase-separated setup, memory, and apply diagnostics for issue #2.

#[path = "support/issue2_fixtures.rs"]
mod fixtures;

use std::{
    env,
    fs::{self, File},
    hint::black_box,
    io::{BufWriter, Write},
    path::{Path, PathBuf},
    time::{Duration, Instant},
};

use fixtures::{DynError, OracleCase, deterministic_rhs, one_level_cases, resolution_cases};
use multiway_mg::{
    DiagonalPreconditioner, ExactCoarseCorrection, OracleLevelSmootherSpec, PairCmgOptions,
    PairSubsetCmgPreconditioner, Preconditioner, ScheduledOracleHierarchy,
    ScheduledOracleHierarchyOptions, SymmetricMapPreconditioner, estimate_problem_bytes,
};

const RECORDED_APPLIES: usize = 31;

fn main() -> Result<(), DynError> {
    let output_directory = env::args_os()
        .nth(1)
        .map_or_else(|| PathBuf::from("."), PathBuf::from);
    fs::create_dir_all(&output_directory)?;
    let mut writer = writer(&output_directory.join("issue2-setup-cost-matrix.tsv"))?;
    writeln!(
        writer,
        "case\tfamily\tdepth\tdimension\ttuples\tmethod\tpair_levels\tcoarsening_setup_ns\tsmoother_setup_ns\tpair_graph_setup_ns\tcmg_setup_ns\tpair_workspace_setup_ns\tterminal_setup_ns\ttotal_setup_ns\tmedian_apply_ns\tretained_bytes_estimate\tapply_scratch_bytes_estimate"
    )?;

    let mut cases = one_level_cases()?;
    cases.extend(
        resolution_cases()?
            .into_iter()
            .filter(|case| case.depth >= 4),
    );
    for case in cases {
        run_case(&case, &mut writer)?;
    }
    println!(
        "wrote {}",
        output_directory
            .join("issue2-setup-cost-matrix.tsv")
            .display()
    );
    Ok(())
}

fn run_case(case: &OracleCase, writer: &mut BufWriter<File>) -> Result<(), DynError> {
    let rhs = deterministic_rhs(&case.problem)?;
    let problem_bytes = estimate_problem_bytes(&case.problem);

    let start = Instant::now();
    let diagonal = DiagonalPreconditioner::new(&case.problem, 0.5)?;
    record(
        case,
        "diagonal",
        0,
        SetupPhases {
            smoother: start.elapsed(),
            total: start.elapsed(),
            ..SetupPhases::default()
        },
        &diagonal,
        problem_bytes.saturating_add(case.problem.dimension().saturating_mul(8)),
        0,
        &rhs,
        writer,
    )?;

    let start = Instant::now();
    let map = SymmetricMapPreconditioner::new(case.problem.clone());
    record(
        case,
        "symmetric-map",
        0,
        SetupPhases {
            smoother: start.elapsed(),
            total: start.elapsed(),
            ..SetupPhases::default()
        },
        &map,
        problem_bytes,
        0,
        &rhs,
        writer,
    )?;

    let pair_all =
        PairSubsetCmgPreconditioner::build_all(case.problem.clone(), PairCmgOptions::default())?;
    record_pair(case, "pair-cmg-all", 1, &pair_all, &rhs, writer)?;

    let pair_selected = PairSubsetCmgPreconditioner::build(
        case.problem.clone(),
        &[case.dominant_pair],
        PairCmgOptions::default(),
    )?;
    record_pair(
        case,
        &format!("pair-cmg-selected-{}", case.dominant_pair.label()),
        1,
        &pair_selected,
        &rhs,
        writer,
    )?;

    if let Some(first_map) = case.maps.first() {
        let coarse =
            ExactCoarseCorrection::build(case.problem.clone(), first_map.clone(), 1.0e-12)?;
        let timing = coarse.build_timing();
        record(
            case,
            "exact-first-coarse",
            0,
            SetupPhases {
                coarsening: timing.coarsening_setup(),
                terminal: timing.terminal_setup(),
                total: timing.total(),
                ..SetupPhases::default()
            },
            &coarse,
            coarse.retained_bytes_estimate(),
            case.problem.dimension().saturating_mul(4).saturating_mul(8),
            &rhs,
            writer,
        )?;
    }

    for schedule in [
        Schedule::Jacobi,
        Schedule::MapAll,
        Schedule::PairFinest,
        Schedule::PairFirstTwo,
        Schedule::PairAll,
    ] {
        let hierarchy = build_schedule(case, schedule)?;
        let timing = hierarchy.build_timing();
        let memory = hierarchy.memory_report();
        record(
            case,
            schedule.label(),
            schedule.pair_levels(case.depth),
            SetupPhases {
                coarsening: timing.coarsening_setup(),
                smoother: timing.smoother_setup(),
                pair_graph: timing.pair_graph_setup(),
                cmg: timing.cmg_setup(),
                pair_workspace: timing.pair_workspace_setup(),
                terminal: timing.terminal_setup(),
                total: timing.total(),
            },
            &hierarchy,
            memory.total_retained_bytes_estimate(),
            memory.maximum_apply_scratch_bytes_estimate(),
            &rhs,
            writer,
        )?;
    }
    Ok(())
}

fn record_pair(
    case: &OracleCase,
    method: &str,
    pair_levels: usize,
    pair: &PairSubsetCmgPreconditioner,
    rhs: &[f64],
    writer: &mut BufWriter<File>,
) -> Result<(), DynError> {
    let timing = pair.build_timing();
    let memory = pair.memory_report();
    record(
        case,
        method,
        pair_levels,
        SetupPhases {
            smoother: timing.total(),
            pair_graph: timing.pair_graph_setup(),
            cmg: timing.cmg_setup(),
            pair_workspace: timing.workspace_setup(),
            total: timing.total(),
            ..SetupPhases::default()
        },
        pair,
        memory.total_retained_bytes_estimate(),
        memory.pair_workspace_bytes(),
        rhs,
        writer,
    )
}

#[derive(Debug, Clone, Copy, Default)]
struct SetupPhases {
    coarsening: Duration,
    smoother: Duration,
    pair_graph: Duration,
    cmg: Duration,
    pair_workspace: Duration,
    terminal: Duration,
    total: Duration,
}

#[allow(clippy::too_many_arguments)]
fn record(
    case: &OracleCase,
    method: &str,
    pair_levels: usize,
    phases: SetupPhases,
    preconditioner: &dyn Preconditioner,
    retained_bytes_estimate: usize,
    apply_scratch_bytes_estimate: usize,
    rhs: &[f64],
    writer: &mut BufWriter<File>,
) -> Result<(), DynError> {
    let median_apply_ns = median_apply_ns(preconditioner, rhs)?;
    writeln!(
        writer,
        "{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}",
        case.name,
        case.family,
        case.depth,
        case.problem.dimension(),
        case.problem.tuple_count(),
        method,
        pair_levels,
        phases.coarsening.as_nanos(),
        phases.smoother.as_nanos(),
        phases.pair_graph.as_nanos(),
        phases.cmg.as_nanos(),
        phases.pair_workspace.as_nanos(),
        phases.terminal.as_nanos(),
        phases.total.as_nanos(),
        median_apply_ns,
        retained_bytes_estimate,
        apply_scratch_bytes_estimate,
    )?;
    Ok(())
}

fn median_apply_ns(preconditioner: &dyn Preconditioner, rhs: &[f64]) -> Result<u128, DynError> {
    let mut output = vec![0.0; preconditioner.dimension()];
    preconditioner.apply(rhs, &mut output)?;
    black_box(output.iter().sum::<f64>());
    let mut samples = Vec::with_capacity(RECORDED_APPLIES);
    for _ in 0..RECORDED_APPLIES {
        let start = Instant::now();
        preconditioner.apply(rhs, &mut output)?;
        samples.push(start.elapsed().as_nanos());
        black_box(output.iter().sum::<f64>());
    }
    samples.sort_unstable();
    Ok(samples[samples.len() / 2])
}

#[derive(Debug, Clone, Copy)]
enum Schedule {
    Jacobi,
    MapAll,
    PairFinest,
    PairFirstTwo,
    PairAll,
}

impl Schedule {
    const fn label(self) -> &'static str {
        match self {
            Self::Jacobi => "oracle-jacobi",
            Self::MapAll => "oracle-map-all-levels",
            Self::PairFinest => "oracle-pair-finest",
            Self::PairFirstTwo => "oracle-pair-first-two",
            Self::PairAll => "oracle-pair-all-levels",
        }
    }

    const fn pair_levels(self, depth: usize) -> usize {
        match self {
            Self::Jacobi | Self::MapAll => 0,
            Self::PairFinest => 1,
            Self::PairFirstTwo => {
                if depth < 2 {
                    depth
                } else {
                    2
                }
            }
            Self::PairAll => depth,
        }
    }
}

fn build_schedule(
    case: &OracleCase,
    schedule: Schedule,
) -> Result<ScheduledOracleHierarchy, DynError> {
    let smoothers = (0..case.depth)
        .map(|level| match schedule {
            Schedule::Jacobi => OracleLevelSmootherSpec::Jacobi { omega: 0.5 },
            Schedule::MapAll => OracleLevelSmootherSpec::SymmetricMap,
            Schedule::PairFinest if level == 0 => OracleLevelSmootherSpec::AllPairsCmg {
                options: PairCmgOptions::default(),
            },
            Schedule::PairFirstTwo if level < 2 => OracleLevelSmootherSpec::AllPairsCmg {
                options: PairCmgOptions::default(),
            },
            Schedule::PairAll => OracleLevelSmootherSpec::AllPairsCmg {
                options: PairCmgOptions::default(),
            },
            Schedule::PairFinest | Schedule::PairFirstTwo => {
                OracleLevelSmootherSpec::Jacobi { omega: 0.5 }
            }
        })
        .collect();
    Ok(ScheduledOracleHierarchy::build(
        case.problem.clone(),
        ScheduledOracleHierarchyOptions {
            aggregations: case.maps.clone(),
            smoothers,
            sweeps: 1,
            terminal_relative_tolerance: 1.0e-12,
        },
    )?)
}

fn writer(path: &Path) -> Result<BufWriter<File>, DynError> {
    Ok(BufWriter::new(File::create(path)?))
}
