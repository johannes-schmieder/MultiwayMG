//! Whole-system issue-4 duel: pair-CMG Schwarz versus pinned `within` Schwarz.
//!
//! This deliberately omits the three-way coarse hierarchy. The purpose is to
//! isolate whether a stronger pair-local action reduces work on the complete
//! three-way operator before coarse-space benefits are added back.

use std::{
    error::Error,
    fs::{self, File},
    io::{BufWriter, Write},
    path::PathBuf,
    time::Instant,
};

use cmg::CmgOptions;
use multiway_mg::{
    DiagonalPreconditioner, LeastSquaresOptions, PairCmgSchwarzOptions,
    PairCmgSchwarzPreconditioner, PcgTraceOptions, Preconditioner, ThreeWayProblem,
    WithinApproxCholOptions, WithinApproxCholPreconditioner, solve_projected_pcg_traced,
    solve_weighted_least_squares,
};

const RHS_COUNT: usize = 4;
const CERTIFICATE_TOLERANCE: f64 = 1.0e-8;
const DIRECT_THRESHOLD: usize = 8;

type Result<T> = std::result::Result<T, Box<dyn Error>>;

struct Case {
    name: &'static str,
    problem: ThreeWayProblem,
    targets: Vec<Vec<f64>>,
}

#[derive(Clone, Copy, Debug)]
enum Method {
    Diagonal,
    PairCmg,
    Within,
}

impl Method {
    const ALL: [Self; 3] = [Self::Diagonal, Self::PairCmg, Self::Within];

    fn label(self) -> &'static str {
        match self {
            Self::Diagonal => "diagonal",
            Self::PairCmg => "pair-cmg-schwarz",
            Self::Within => "within-default",
        }
    }
}

enum Action {
    Diagonal(DiagonalPreconditioner),
    PairCmg(PairCmgSchwarzPreconditioner),
    Within(WithinApproxCholPreconditioner),
}

impl Action {
    fn as_preconditioner(&self) -> &dyn Preconditioner {
        match self {
            Self::Diagonal(value) => value,
            Self::PairCmg(value) => value,
            Self::Within(value) => value,
        }
    }

    fn fallback_allocations(&self) -> usize {
        match self {
            Self::Diagonal(_) => 0,
            Self::PairCmg(value) => value.fallback_workspace_allocations(),
            Self::Within(value) => value.projection_fallback_allocations(),
        }
    }
}

struct Built {
    action: Action,
    constructor_seconds: f64,
    initialization_seconds: f64,
    known_retained_bytes: Option<usize>,
    warning_count: usize,
    pair_components: usize,
    max_pair_vertices: usize,
    max_pair_edges: usize,
    max_pair_cycle_excess: usize,
    max_pair_levels: usize,
    multilevel_pair_components: usize,
}

impl Built {
    fn preconditioner(&self) -> &dyn Preconditioner {
        self.action.as_preconditioner()
    }

    fn setup_seconds(&self) -> f64 {
        self.constructor_seconds + self.initialization_seconds
    }
}

fn build(problem: &ThreeWayProblem, targets: &[Vec<f64>], method: Method) -> Result<Built> {
    let start = Instant::now();
    let (action, known_retained_bytes, warning_count, pair_metadata) = match method {
        Method::Diagonal => (
            Action::Diagonal(DiagonalPreconditioner::new(problem, 0.5)?),
            None,
            0,
            (0, 0, 0, 0, 0, 0),
        ),
        Method::PairCmg => {
            let preconditioner = PairCmgSchwarzPreconditioner::build_all(
                problem.clone(),
                PairCmgSchwarzOptions {
                    cmg: CmgOptions {
                        direct_threshold: DIRECT_THRESHOLD,
                        ..CmgOptions::default()
                    },
                    ..PairCmgSchwarzOptions::default()
                },
            )?;
            let reports = preconditioner.component_reports();
            let metadata = (
                reports.len(),
                reports.iter().map(|r| r.vertices()).max().unwrap_or(0),
                reports.iter().map(|r| r.edges()).max().unwrap_or(0),
                reports.iter().map(|r| r.cycle_excess()).max().unwrap_or(0),
                reports.iter().map(|r| r.cmg_levels()).max().unwrap_or(0),
                reports.iter().filter(|r| r.cmg_levels() > 1).count(),
            );
            let bytes = preconditioner
                .memory_report()
                .total_retained_bytes_estimate();
            (Action::PairCmg(preconditioner), Some(bytes), 0, metadata)
        }
        Method::Within => {
            let preconditioner = WithinApproxCholPreconditioner::build(
                problem.clone(),
                WithinApproxCholOptions::default(),
            )?;
            let warning_count = preconditioner.warnings().len();
            let bytes = preconditioner
                .memory_report()
                .known_retained_bytes_estimate();
            (
                Action::Within(preconditioner),
                Some(bytes),
                warning_count,
                (0, 0, 0, 0, 0, 0),
            )
        }
    };
    let constructor_seconds = start.elapsed().as_secs_f64();

    // Charge one first application as workspace/lazy initialization for every
    // route. This is intentionally conservative and keeps later solves hot.
    let rhs = problem.rhs_from_targets(&targets[0])?;
    let mut output = vec![0.0; problem.dimension()];
    let initialization_start = Instant::now();
    action.as_preconditioner().apply(&rhs, &mut output)?;
    let initialization_seconds = initialization_start.elapsed().as_secs_f64();
    if output.iter().any(|x| !x.is_finite()) {
        return Err("nonfinite preconditioner initialization output".into());
    }

    Ok(Built {
        action,
        constructor_seconds,
        initialization_seconds,
        known_retained_bytes,
        warning_count,
        pair_components: pair_metadata.0,
        max_pair_vertices: pair_metadata.1,
        max_pair_edges: pair_metadata.2,
        max_pair_cycle_excess: pair_metadata.3,
        max_pair_levels: pair_metadata.4,
        multilevel_pair_components: pair_metadata.5,
    })
}

#[derive(Default)]
struct SolveRecord {
    solve_seconds: f64,
    iterations: usize,
    converged: bool,
    certified: bool,
    true_residual: f64,
    outer_work: usize,
    preconditioner_applications: usize,
    certificate_work: usize,
    stop_reason: String,
    error: String,
}

fn run_lsmr(case: &Case, target: &[f64], built: &Built) -> SolveRecord {
    let start = Instant::now();
    let result = solve_weighted_least_squares(
        &case.problem,
        target,
        built.preconditioner(),
        LeastSquaresOptions {
            tolerance: 1.0e-10,
            max_iterations: 2_000,
            local_size: Some(8),
        },
    );
    let seconds = start.elapsed().as_secs_f64();
    match result {
        Ok(result) => {
            let residual = result.certified_normal_equation_residual();
            SolveRecord {
                solve_seconds: seconds,
                iterations: result.iterations(),
                converged: result.converged(),
                certified: residual.is_finite() && residual <= CERTIFICATE_TOLERANCE,
                true_residual: residual,
                outer_work: result.work().solver_outer_operator_applications(),
                preconditioner_applications: result.work().preconditioner_applications(),
                certificate_work: result.work().certification_incidence_applications()
                    + result.work().certification_adjoint_applications(),
                stop_reason: format!("{:?}", result.stop_reason()),
                error: String::new(),
            }
        }
        Err(error) => SolveRecord {
            solve_seconds: seconds,
            true_residual: f64::INFINITY,
            stop_reason: "Error".to_owned(),
            error: sanitize(&error.to_string()),
            ..SolveRecord::default()
        },
    }
}

fn run_pcg(case: &Case, target: &[f64], built: &Built) -> SolveRecord {
    let rhs = match case.problem.rhs_from_targets(target) {
        Ok(value) => value,
        Err(error) => {
            return SolveRecord {
                true_residual: f64::INFINITY,
                stop_reason: "Error".to_owned(),
                error: sanitize(&error.to_string()),
                ..SolveRecord::default()
            };
        }
    };
    let start = Instant::now();
    let result = solve_projected_pcg_traced(
        &case.problem,
        &rhs,
        built.preconditioner(),
        PcgTraceOptions {
            relative_tolerance: 1.0e-10,
            absolute_tolerance: 0.0,
            max_iterations: 2_000,
        },
    );
    let seconds = start.elapsed().as_secs_f64();
    match result {
        Ok(result) => {
            let residual = result.final_relative_residual();
            SolveRecord {
                solve_seconds: seconds,
                iterations: result.iterations(),
                converged: result.converged(),
                certified: residual.is_finite() && residual <= CERTIFICATE_TOLERANCE,
                true_residual: residual,
                outer_work: result.gramian_applications(),
                preconditioner_applications: result.preconditioner_applications(),
                certificate_work: 0,
                stop_reason: if result.converged() {
                    "Converged".to_owned()
                } else {
                    "MaximumIterations".to_owned()
                },
                error: String::new(),
            }
        }
        Err(error) => SolveRecord {
            solve_seconds: seconds,
            true_residual: f64::INFINITY,
            stop_reason: "Error".to_owned(),
            error: sanitize(&error.to_string()),
            ..SolveRecord::default()
        },
    }
}

fn sanitize(value: &str) -> String {
    value.replace(['\t', '\n', '\r'], " ")
}

fn main() -> Result<()> {
    let mut args = std::env::args().skip(1);
    let output = PathBuf::from(
        args.next()
            .unwrap_or_else(|| "issue4-whole-system".to_owned()),
    );
    let profile = args.next().unwrap_or_else(|| "smoke".to_owned());
    if args.next().is_some() || !["smoke", "calibration"].contains(&profile.as_str()) {
        return Err("usage: issue4_whole_system [output-directory] [smoke|calibration]".into());
    }
    fs::create_dir_all(&output)?;
    let mut writer = BufWriter::new(File::create(output.join("whole-system.tsv"))?);
    writeln!(
        writer,
        "profile\tcase\trepeat\tmethod\tsolver\trhs\tfactor1\tfactor2\tfactor3\ttuples\tcomponents\tconstructor_seconds\tinitialization_seconds\tsetup_seconds\tsolve_seconds\titerations\tconverged\tcertified\ttrue_residual\touter_work\twork_unit\tpreconditioner_applications\tcertificate_work\tknown_retained_bytes\tpair_components\tmax_pair_vertices\tmax_pair_edges\tmax_pair_cycle_excess\tmax_pair_levels\tmultilevel_pair_components\tfallback_allocations\twarning_count\tstop_reason\terror"
    )?;

    let cases = cases(&profile)?;
    for (case_index, case) in cases.iter().enumerate() {
        let counts = case.problem.topology().level_counts();
        for repeat in 0..2 {
            for method_index in 0..Method::ALL.len() {
                let method = Method::ALL[(method_index + repeat + case_index) % Method::ALL.len()];
                let built = build(&case.problem, &case.targets, method)?;
                for (rhs_index, target) in case.targets.iter().enumerate() {
                    for solver in ["mlsmr", "pcg-traced"] {
                        let record = if solver == "mlsmr" {
                            run_lsmr(case, target, &built)
                        } else {
                            run_pcg(case, target, &built)
                        };
                        let work_unit = if solver == "mlsmr" {
                            "rectangular-operator"
                        } else {
                            "gramian"
                        };
                        writeln!(
                            writer,
                            "{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{:.9e}\t{:.9e}\t{:.9e}\t{:.9e}\t{}\t{}\t{}\t{:.9e}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}",
                            profile,
                            case.name,
                            repeat,
                            method.label(),
                            solver,
                            rhs_index + 1,
                            counts[0],
                            counts[1],
                            counts[2],
                            case.problem.tuple_count(),
                            case.problem.components().count(),
                            built.constructor_seconds,
                            built.initialization_seconds,
                            built.setup_seconds(),
                            record.solve_seconds,
                            record.iterations,
                            record.converged,
                            record.certified,
                            record.true_residual,
                            record.outer_work,
                            work_unit,
                            record.preconditioner_applications,
                            record.certificate_work,
                            built
                                .known_retained_bytes
                                .map_or_else(|| "NA".to_owned(), |value| value.to_string()),
                            built.pair_components,
                            built.max_pair_vertices,
                            built.max_pair_edges,
                            built.max_pair_cycle_excess,
                            built.max_pair_levels,
                            built.multilevel_pair_components,
                            built.action.fallback_allocations(),
                            built.warning_count,
                            record.stop_reason,
                            record.error,
                        )?;
                        writer.flush()?;
                    }
                }
            }
        }
    }
    Ok(())
}

fn cases(profile: &str) -> Result<Vec<Case>> {
    let scale = if profile == "smoke" { 1 } else { 2 };
    let problems = [
        ("planted-clones", planted_clones(12 * scale, 2)?),
        ("noisy-clones", noisy_clones(8 * scale, 3)?),
        ("latin-square", latin_square(24 * scale, 0)?),
        ("weak-chain", weak_chain(16 * scale, 2)?),
        ("disconnected-latin", disconnected_latin(12 * scale)?),
        (
            "unbalanced-cycle",
            unbalanced_cycle(96 * scale, 48 * scale, 12 * scale)?,
        ),
    ];
    problems
        .into_iter()
        .map(|(name, problem)| {
            let targets = (0..RHS_COUNT)
                .map(|rhs| exact_targets(&problem, rhs))
                .collect::<Result<Vec<_>>>()?;
            Ok(Case {
                name,
                problem,
                targets,
            })
        })
        .collect()
}

fn planted_clones(groups: usize, clones: usize) -> Result<ThreeWayProblem> {
    let mut tuples = Vec::new();
    let mut weights = Vec::new();
    for first in 0..groups {
        for second in 0..groups {
            let third = (first + second) % groups;
            for first_clone in 0..clones {
                for second_clone in 0..clones {
                    for third_clone in 0..clones {
                        tuples.push([
                            (first * clones + first_clone) as u32,
                            (second * clones + second_clone) as u32,
                            (third * clones + third_clone) as u32,
                        ]);
                        weights.push(
                            1.0 + ((first + 2 * second + first_clone + second_clone + third_clone)
                                % 7) as f64
                                / 10.0,
                        );
                    }
                }
            }
        }
    }
    Ok(ThreeWayProblem::from_observations(
        [groups * clones; 3],
        &tuples,
        &weights,
    )?)
}

fn noisy_clones(groups: usize, clones: usize) -> Result<ThreeWayProblem> {
    let mut tuples = Vec::new();
    let mut weights = Vec::new();
    for first in 0..groups {
        for second in 0..groups {
            let third = (first + second) % groups;
            for first_clone in 0..clones {
                for second_clone in 0..clones {
                    for third_clone in 0..clones {
                        if (first_clone + 2 * second_clone + 3 * third_clone + first + 2 * second)
                            % 4
                            == 0
                        {
                            continue;
                        }
                        tuples.push([
                            (first * clones + first_clone) as u32,
                            (second * clones + second_clone) as u32,
                            (third * clones + third_clone) as u32,
                        ]);
                        weights.push(
                            0.5 + ((11 * first
                                + 7 * second
                                + 5 * first_clone
                                + 3 * second_clone
                                + third_clone)
                                % 13) as f64
                                / 10.0,
                        );
                    }
                }
            }
        }
    }
    Ok(ThreeWayProblem::from_observations(
        [groups * clones; 3],
        &tuples,
        &weights,
    )?)
}

fn latin_square(levels: usize, offset: u32) -> Result<ThreeWayProblem> {
    let tuples = latin_square_tuples(levels as u32, offset);
    let weights: Vec<_> = (0..tuples.len())
        .map(|index| 0.8 + (index % 11) as f64 / 10.0)
        .collect();
    Ok(ThreeWayProblem::from_observations(
        [levels; 3],
        &tuples,
        &weights,
    )?)
}

fn weak_chain(groups: usize, clones: usize) -> Result<ThreeWayProblem> {
    let mut tuples = Vec::new();
    let mut weights = Vec::new();
    for group in 0..groups {
        for first_clone in 0..clones {
            for second_clone in 0..clones {
                for third_clone in 0..clones {
                    tuples.push([
                        (group * clones + first_clone) as u32,
                        (group * clones + second_clone) as u32,
                        (group * clones + third_clone) as u32,
                    ]);
                    weights.push(
                        1.0 + ((group + first_clone + 2 * second_clone + third_clone) % 7) as f64
                            / 10.0,
                    );
                }
            }
        }
        if group + 1 < groups {
            for clone in 0..clones {
                tuples.push([
                    (group * clones + clone) as u32,
                    ((group + 1) * clones + clone) as u32,
                    ((group + 1) * clones + clone) as u32,
                ]);
                weights.push(0.05);
                tuples.push([
                    ((group + 1) * clones + clone) as u32,
                    (group * clones + clone) as u32,
                    ((group + 1) * clones + (clone + 1) % clones) as u32,
                ]);
                weights.push(0.05);
            }
        }
    }
    Ok(ThreeWayProblem::from_observations(
        [groups * clones; 3],
        &tuples,
        &weights,
    )?)
}

fn disconnected_latin(levels: usize) -> Result<ThreeWayProblem> {
    let mut tuples = latin_square_tuples(levels as u32, 0);
    tuples.extend(latin_square_tuples(levels as u32, levels as u32));
    let weights: Vec<_> = (0..tuples.len())
        .map(|index| 0.9 + (index % 9) as f64 / 10.0)
        .collect();
    Ok(ThreeWayProblem::from_observations(
        [2 * levels; 3],
        &tuples,
        &weights,
    )?)
}

fn unbalanced_cycle(first: usize, second: usize, third: usize) -> Result<ThreeWayProblem> {
    let mut tuples = Vec::with_capacity(first * 8);
    let mut weights = Vec::with_capacity(first * 8);
    for a in 0..first {
        for step in 0..8 {
            tuples.push([
                a as u32,
                ((7 * a + 11 * step) % second) as u32,
                ((a + 3 * step) % third) as u32,
            ]);
            weights.push(0.5 + ((13 * a + 5 * step) % 19) as f64 / 10.0);
        }
    }
    Ok(ThreeWayProblem::from_observations(
        [first, second, third],
        &tuples,
        &weights,
    )?)
}

fn latin_square_tuples(levels: u32, offset: u32) -> Vec<[u32; 3]> {
    let mut tuples = Vec::with_capacity((levels * levels) as usize);
    for first in 0..levels {
        for second in 0..levels {
            tuples.push([
                first + offset,
                second + offset,
                (first + second) % levels + offset,
            ]);
        }
    }
    tuples
}

fn exact_targets(problem: &ThreeWayProblem, rhs: usize) -> Result<Vec<f64>> {
    let counts = problem.topology().level_counts();
    let mut coefficients = Vec::with_capacity(problem.dimension());
    for (factor, &count) in counts.iter().enumerate() {
        for level in 0..count {
            let phase = (rhs + 1) as f64 * 0.071;
            coefficients.push(
                ((factor + 1) as f64 * 0.37 + level as f64 * (0.11 + phase)).sin()
                    + (level as f64 * (0.07 + phase * 0.5)).cos(),
            );
        }
    }
    problem
        .components()
        .project_structural_range(&mut coefficients)?;
    let mut targets = vec![0.0; problem.tuple_count()];
    problem.apply_incidence(&coefficients, &mut targets)?;
    Ok(targets)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn smoke_matrix_contains_balanced_unbalanced_and_disconnected_cases() {
        let cases = cases("smoke").unwrap();
        assert_eq!(cases.len(), 6);
        let disconnected = cases
            .iter()
            .find(|case| case.name == "disconnected-latin")
            .unwrap();
        assert_eq!(disconnected.problem.components().count(), 2);
        let unbalanced = cases
            .iter()
            .find(|case| case.name == "unbalanced-cycle")
            .unwrap();
        let counts = unbalanced.problem.topology().level_counts();
        assert!(counts[0] > counts[1] && counts[1] > counts[2]);
        for case in cases {
            assert_eq!(case.targets.len(), RHS_COUNT);
            assert!(
                case.targets
                    .iter()
                    .all(|target| target.len() == case.problem.tuple_count())
            );
        }
    }

    #[test]
    fn all_routes_certify_a_small_connected_system_under_both_outer_solvers() {
        let problem = latin_square(8, 0).unwrap();
        let targets = vec![exact_targets(&problem, 0).unwrap()];
        let case = Case {
            name: "test",
            problem,
            targets,
        };
        for method in Method::ALL {
            let built = build(&case.problem, &case.targets, method).unwrap();
            let lsmr = run_lsmr(&case, &case.targets[0], &built);
            assert!(lsmr.certified, "{method:?} LSMR: {}", lsmr.error);
            let pcg = run_pcg(&case, &case.targets[0], &built);
            assert!(pcg.certified, "{method:?} PCG: {}", pcg.error);
            assert_eq!(built.action.fallback_allocations(), 0);
        }
    }
}
