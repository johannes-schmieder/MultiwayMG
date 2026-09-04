//! Production-shaped large-domain pair-local timing matrix for issue #4.
//!
//! The parent process launches one isolated child per case, method, thread
//! count, and setup repetition.  Each child reports its own setup/apply timing
//! and Linux RSS/HWM so opaque retained state in the frozen `within` comparator
//! is charged rather than guessed.

#[path = "support/issue4_pair_fixtures.rs"]
mod fixtures;

use std::{
    env,
    fs::{self, File},
    hint::black_box,
    io::{BufWriter, Write},
    path::PathBuf,
    process::Command,
    time::{Duration, Instant},
};

use fixtures::{
    DynError, PairCase, PairSuite, deterministic_range_rhs, large_case, large_case_names,
};
use multiway_mg::{
    PairLocalCmgOptions, PairLocalCmgPreconditioner, PairLocalWithinPreconditioner, Preconditioner,
    WithinApproxCholOptions,
};

const RHS_COUNTS: [usize; 4] = [1, 4, 16, 32];
const APPLY_REPETITIONS: usize = 3;
const SETUP_REPETITIONS: usize = 2;
const WARMUP_APPLICATIONS: usize = 3;
const THREAD_CANDIDATES: [usize; 6] = [1, 2, 4, 8, 16, 32];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Method {
    CmgOne,
    CmgTwo,
    Within,
}

impl Method {
    const ALL: [Self; 3] = [Self::CmgOne, Self::Within, Self::CmgTwo];

    const fn label(self) -> &'static str {
        match self {
            Self::CmgOne => "cmg-1-fixed",
            Self::CmgTwo => "cmg-2-fixed",
            Self::Within => "within-approx-cholesky",
        }
    }

    fn parse(value: &str) -> Result<Self, DynError> {
        Self::ALL
            .into_iter()
            .find(|method| method.label() == value)
            .ok_or_else(|| format!("unknown timing method {value:?}").into())
    }
}

struct BuildMetadata {
    total: Duration,
    input_setup: Duration,
    local_setup: Duration,
    preconditioner_setup: Duration,
    workspace_setup: Duration,
    known_retained_bytes: usize,
    opaque_retained_bytes: bool,
    workspace_bytes: usize,
    hierarchy_levels: usize,
    hierarchy_matrix_nonzeros: usize,
    warnings: usize,
}

enum BuiltMethod {
    Cmg(Box<PairLocalCmgPreconditioner>),
    Within(Box<PairLocalWithinPreconditioner>),
}

impl BuiltMethod {
    fn build(method: Method, case: &PairCase) -> Result<(Self, BuildMetadata), DynError> {
        match method {
            Method::CmgOne | Method::CmgTwo => {
                let fixed_cycles = usize::from(method == Method::CmgTwo) + 1;
                let preconditioner = PairLocalCmgPreconditioner::build(
                    case.domain.clone(),
                    PairLocalCmgOptions {
                        cmg: cmg::CmgOptions::default(),
                        fixed_cycles,
                    },
                )?;
                let timing = preconditioner.build_timing();
                let memory = preconditioner.memory_report();
                let metadata = BuildMetadata {
                    total: timing.total(),
                    input_setup: Duration::ZERO,
                    local_setup: timing.cmg_setup(),
                    preconditioner_setup: timing.cmg_setup(),
                    workspace_setup: timing.workspace_setup(),
                    known_retained_bytes: memory.total_retained_bytes_estimate(),
                    opaque_retained_bytes: false,
                    workspace_bytes: memory.workspace_pool_bytes(),
                    hierarchy_levels: preconditioner.hierarchy_levels(),
                    hierarchy_matrix_nonzeros: preconditioner.hierarchy_matrix_nonzeros(),
                    warnings: 0,
                };
                Ok((Self::Cmg(Box::new(preconditioner)), metadata))
            }
            Method::Within => {
                let mut options = WithinApproxCholOptions::default();
                options.local_solver.dense_threshold = 0;
                let preconditioner =
                    PairLocalWithinPreconditioner::build(case.domain.clone(), options)?;
                let timing = preconditioner.build_timing();
                let memory = preconditioner.memory_report();
                let metadata = BuildMetadata {
                    total: timing.total(),
                    input_setup: timing.design_input_setup(),
                    local_setup: timing.within_solver_setup(),
                    preconditioner_setup: timing.within_preconditioner_setup(),
                    workspace_setup: timing.workspace_setup(),
                    known_retained_bytes: memory.known_retained_bytes_estimate(),
                    opaque_retained_bytes: memory.within_retained_bytes().is_none(),
                    workspace_bytes: memory.range_workspace_bytes(),
                    hierarchy_levels: 0,
                    hierarchy_matrix_nonzeros: 0,
                    warnings: preconditioner.warnings().len(),
                };
                Ok((Self::Within(Box::new(preconditioner)), metadata))
            }
        }
    }

    fn apply(&self, rhs: &[f64], out: &mut [f64]) -> Result<(), DynError> {
        match self {
            Self::Cmg(preconditioner) => preconditioner.apply(rhs, out)?,
            Self::Within(preconditioner) => preconditioner.apply(rhs, out)?,
        }
        Ok(())
    }

    fn fallback_allocations(&self) -> usize {
        match self {
            Self::Cmg(preconditioner) => preconditioner.fallback_workspace_allocations(),
            Self::Within(preconditioner) => preconditioner.fallback_workspace_allocations(),
        }
    }
}

#[derive(Debug, Clone, Copy, Default)]
struct ProcessMemory {
    rss_kib: usize,
    hwm_kib: usize,
}

fn main() -> Result<(), DynError> {
    let arguments: Vec<String> = env::args().skip(1).collect();
    if arguments.first().map(String::as_str) == Some("--child") {
        return run_child(&arguments[1..]);
    }
    run_parent(&arguments)
}

fn run_parent(arguments: &[String]) -> Result<(), DynError> {
    let output_directory = arguments
        .first()
        .map_or_else(|| PathBuf::from("."), PathBuf::from);
    let suite = parse_suite(arguments.get(1).map(String::as_str))?;
    if arguments.len() > 2 {
        return Err(
            "usage: issue4_pair_local_timing [output-directory] [calibration|holdout]".into(),
        );
    }
    fs::create_dir_all(&output_directory)?;
    let output_path =
        output_directory.join(format!("issue4-pair-local-{}-timing.tsv", suite.label()));
    let mut writer = BufWriter::new(File::create(output_path)?);
    write_header(&mut writer)?;

    let available = std::thread::available_parallelism()?.get();
    let threads: Vec<usize> = THREAD_CANDIDATES
        .into_iter()
        .filter(|&count| count <= available)
        .collect();
    let executable = env::current_exe()?;
    for (case_index, &case_name) in large_case_names(suite).iter().enumerate() {
        for &thread_count in &threads {
            for setup_repetition in 0..SETUP_REPETITIONS {
                let rotation = (case_index + setup_repetition + thread_count) % Method::ALL.len();
                for method_offset in 0..Method::ALL.len() {
                    let method = Method::ALL[(rotation + method_offset) % Method::ALL.len()];
                    eprintln!(
                        "timing {} {} threads={} setup_rep={} method={}",
                        suite.label(),
                        case_name,
                        thread_count,
                        setup_repetition,
                        method.label()
                    );
                    let output = Command::new(&executable)
                        .args([
                            "--child",
                            suite.label(),
                            case_name,
                            method.label(),
                            &thread_count.to_string(),
                            &setup_repetition.to_string(),
                        ])
                        .env("RAYON_NUM_THREADS", thread_count.to_string())
                        .output()?;
                    if !output.status.success() {
                        return Err(format!(
                            "child failed for {case_name} {} threads={thread_count}: {}",
                            method.label(),
                            String::from_utf8_lossy(&output.stderr)
                        )
                        .into());
                    }
                    writer.write_all(&output.stdout)?;
                    writer.flush()?;
                }
            }
        }
    }
    Ok(())
}

fn run_child(arguments: &[String]) -> Result<(), DynError> {
    if arguments.len() != 5 {
        return Err("child usage: --child SUITE CASE METHOD THREADS SETUP_REPETITION".into());
    }
    let suite = parse_suite(Some(&arguments[0]))?;
    let case_name = &arguments[1];
    let method = Method::parse(&arguments[2])?;
    let threads: usize = arguments[3].parse()?;
    let setup_repetition: usize = arguments[4].parse()?;
    if threads == 0 {
        return Err("thread count must be positive".into());
    }
    if env::var("RAYON_NUM_THREADS").ok().as_deref() != Some(arguments[3].as_str()) {
        return Err("child RAYON_NUM_THREADS does not match requested width".into());
    }

    let memory_start = process_memory()?;
    let case_start = Instant::now();
    let case = large_case(suite, case_name)?;
    let case_generation = case_start.elapsed();
    let memory_after_domain = process_memory()?;

    let rhs_start = Instant::now();
    let right_hand_sides: Vec<Vec<f64>> = (0..*RHS_COUNTS.last().expect("nonempty RHS counts"))
        .map(|index| deterministic_range_rhs(&case.domain, 0.17 + index as f64 * 0.031))
        .collect::<Result<_, _>>()?;
    let rhs_generation = rhs_start.elapsed();
    let memory_after_rhs = process_memory()?;

    let (preconditioner, build) = BuiltMethod::build(method, &case)?;
    let memory_after_setup = process_memory()?;
    let mut output = vec![0.0; case.domain.dimension()];
    for index in 0..WARMUP_APPLICATIONS {
        preconditioner.apply(&right_hand_sides[index], &mut output)?;
        black_box(output.first().copied().unwrap_or(0.0));
    }

    let mut count_order = RHS_COUNTS;
    let count_len = count_order.len();
    count_order.rotate_left((setup_repetition + threads + method as usize) % count_len);
    let mut rows = Vec::with_capacity(APPLY_REPETITIONS * RHS_COUNTS.len());
    for &rhs_count in &count_order {
        for repetition in 0..APPLY_REPETITIONS {
            let start_index = (repetition * 7 + setup_repetition * 3) % right_hand_sides.len();
            let started = Instant::now();
            let mut checksum = 0.0;
            for offset in 0..rhs_count {
                let rhs = &right_hand_sides[(start_index + offset) % right_hand_sides.len()];
                preconditioner.apply(rhs, &mut output)?;
                checksum += output[(offset * 104_729 + 17) % output.len()];
            }
            let elapsed = started.elapsed();
            black_box(checksum);
            rows.push((rhs_count, repetition, elapsed, checksum));
        }
    }
    let memory_final = process_memory()?;
    let fallback_allocations = preconditioner.fallback_allocations();
    if fallback_allocations != 0 {
        return Err(format!(
            "{} {} allocated {fallback_allocations} fallback workspaces",
            case.name,
            method.label()
        )
        .into());
    }

    let mut stdout = BufWriter::new(std::io::stdout().lock());
    for (rhs_count, repetition, apply_duration, checksum) in rows {
        writeln!(
            stdout,
            "{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{:.17e}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{:.17e}",
            suite.label(),
            case.name,
            case.family,
            method.label(),
            threads,
            std::thread::available_parallelism()?.get(),
            setup_repetition,
            repetition,
            rhs_count,
            case.domain.left_count(),
            case.domain.right_count(),
            case.domain.dimension(),
            case.domain.weight_dynamic_range(),
            case.domain.edge_count(),
            case.domain.cycle_excess(),
            case.domain.minimum_degree(),
            case.domain.maximum_degree(),
            duration_ns(case_generation),
            duration_ns(case.domain.build_duration()),
            duration_ns(rhs_generation),
            duration_ns(build.total),
            duration_ns(build.input_setup),
            duration_ns(build.local_setup),
            duration_ns(build.preconditioner_setup),
            duration_ns(build.workspace_setup),
            duration_ns(apply_duration),
            duration_ns(apply_duration) / rhs_count as u128,
            case.domain.retained_bytes_estimate(),
            build.known_retained_bytes,
            build.opaque_retained_bytes,
            build.workspace_bytes,
            build.hierarchy_levels,
            build.hierarchy_matrix_nonzeros,
            build.warnings,
            fallback_allocations,
            memory_start.rss_kib,
            memory_after_domain.rss_kib,
            memory_after_rhs.rss_kib,
            memory_after_setup.rss_kib,
            memory_final.rss_kib,
            memory_final.hwm_kib,
            memory_after_setup
                .rss_kib
                .saturating_sub(memory_after_rhs.rss_kib),
            checksum,
        )?;
    }
    Ok(())
}

fn write_header(writer: &mut BufWriter<File>) -> Result<(), DynError> {
    writeln!(
        writer,
        "suite\tcase\tfamily\tmethod\tthreads\thost_parallelism\tsetup_repetition\tapply_repetition\trhs_count\tleft_count\tright_count\tdimension\tweight_dynamic_range\tedges\tcycle_excess\tminimum_degree\tmaximum_degree\tcase_generation_ns\tdomain_build_ns\trhs_generation_ns\tsolver_setup_total_ns\tinput_setup_ns\tlocal_setup_ns\tpreconditioner_setup_ns\tworkspace_setup_ns\tapply_batch_ns\tapply_per_rhs_ns\tdomain_retained_bytes\tknown_solver_retained_bytes\topaque_solver_retained_bytes\tworkspace_bytes\thierarchy_levels\thierarchy_matrix_nonzeros\twarnings\tfallback_allocations\trss_start_kib\trss_after_domain_kib\trss_after_rhs_kib\trss_after_setup_kib\trss_final_kib\thwm_final_kib\tsetup_rss_delta_kib\tchecksum"
    )?;
    Ok(())
}

fn parse_suite(value: Option<&str>) -> Result<PairSuite, DynError> {
    match value {
        None | Some("calibration") => Ok(PairSuite::Calibration),
        Some("holdout") => Ok(PairSuite::Holdout),
        Some(value) => Err(format!("unknown suite {value:?}").into()),
    }
}

fn duration_ns(duration: Duration) -> u128 {
    duration.as_nanos()
}

fn process_memory() -> Result<ProcessMemory, DynError> {
    #[cfg(target_os = "linux")]
    {
        let status = fs::read_to_string("/proc/self/status")?;
        let mut memory = ProcessMemory::default();
        for line in status.lines() {
            if let Some(value) = parse_status_kib(line, "VmRSS:") {
                memory.rss_kib = value;
            } else if let Some(value) = parse_status_kib(line, "VmHWM:") {
                memory.hwm_kib = value;
            }
        }
        Ok(memory)
    }
    #[cfg(not(target_os = "linux"))]
    {
        Ok(ProcessMemory::default())
    }
}

#[cfg(target_os = "linux")]
fn parse_status_kib(line: &str, key: &str) -> Option<usize> {
    line.strip_prefix(key)?
        .split_whitespace()
        .next()?
        .parse()
        .ok()
}
