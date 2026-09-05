//! Frozen issue-4 size ladder around the whole-system work crossover.
//!
//! This is calibration, not a holdout. It keeps the issue-3 coarse hierarchy
//! disabled and compares finest-level diagonal, pair-CMG Schwarz, and pinned
//! `within` Schwarz on balanced planted/noisy/Latin systems of increasing size.

use std::{
    error::Error,
    fs::{self, File},
    io::{BufWriter, Write},
    path::PathBuf,
    time::Instant,
};

use cmg::{CmgOptions, TerminalReason};
use multiway_mg::{
    DiagonalPreconditioner, LeastSquaresOptions, PairCmgSchwarzOptions,
    PairCmgSchwarzPreconditioner, PcgTraceOptions, Preconditioner, ThreeWayProblem,
    WithinApproxCholOptions, WithinApproxCholPreconditioner, solve_projected_pcg_traced,
    solve_weighted_least_squares,
};

const PREFIXES: [usize; 4] = [1, 4, 16, 32];
const MAX_RHS: usize = 32;
const CERTIFICATE_TOLERANCE: f64 = 1.0e-8;
const DIRECT_THRESHOLD: usize = 8;
const LEVELS: [usize; 6] = [12, 18, 24, 36, 48, 72];

type Result<T> = std::result::Result<T, Box<dyn Error>>;

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

struct Case {
    family: &'static str,
    levels: usize,
    problem: ThreeWayProblem,
    targets: Vec<Vec<f64>>,
}

enum Action {
    Diagonal(DiagonalPreconditioner),
    PairCmg(PairCmgSchwarzPreconditioner),
    Within(WithinApproxCholPreconditioner),
}

impl Action {
    fn preconditioner(&self) -> &dyn Preconditioner {
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

#[derive(Default)]
struct TerminalSummary {
    pair_components: usize,
    max_pair_vertices: usize,
    max_pair_edges: usize,
    max_cycle_excess: usize,
    max_levels: usize,
    multilevel: usize,
    direct: usize,
    full_contraction: usize,
    stagnated_vertex: usize,
    stagnated_fill: usize,
    maximum_levels: usize,
    one_level_iterative: usize,
    direct_factors: usize,
}

struct Built {
    action: Action,
    constructor_seconds: f64,
    initialization_seconds: f64,
    known_retained_bytes: Option<usize>,
    warning_count: usize,
    terminal: TerminalSummary,
}

impl Built {
    fn setup_seconds(&self) -> f64 {
        self.constructor_seconds + self.initialization_seconds
    }
}

fn build(problem: &ThreeWayProblem, target: &[f64], method: Method) -> Result<Built> {
    let start = Instant::now();
    let (action, known_retained_bytes, warning_count, terminal) = match method {
        Method::Diagonal => (
            Action::Diagonal(DiagonalPreconditioner::new(problem, 0.5)?),
            None,
            0,
            TerminalSummary::default(),
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
            let mut terminal = TerminalSummary::default();
            for report in preconditioner.component_reports() {
                terminal.pair_components += 1;
                terminal.max_pair_vertices = terminal.max_pair_vertices.max(report.vertices());
                terminal.max_pair_edges = terminal.max_pair_edges.max(report.edges());
                terminal.max_cycle_excess = terminal.max_cycle_excess.max(report.cycle_excess());
                terminal.max_levels = terminal.max_levels.max(report.cmg_levels());
                terminal.multilevel += usize::from(report.cmg_levels() > 1);
                terminal.direct_factors += usize::from(report.cmg_direct_factor());
                terminal.one_level_iterative +=
                    usize::from(report.cmg_levels() == 1 && report.cmg_terminal().is_iterative());
                match report.cmg_terminal() {
                    TerminalReason::Direct => terminal.direct += 1,
                    TerminalReason::FullContraction => terminal.full_contraction += 1,
                    TerminalReason::StagnatedVertexReduction => terminal.stagnated_vertex += 1,
                    TerminalReason::StagnatedFill => terminal.stagnated_fill += 1,
                    TerminalReason::MaximumLevels => terminal.maximum_levels += 1,
                }
            }
            let bytes = preconditioner
                .memory_report()
                .total_retained_bytes_estimate();
            (Action::PairCmg(preconditioner), Some(bytes), 0, terminal)
        }
        Method::Within => {
            let preconditioner = WithinApproxCholPreconditioner::build(
                problem.clone(),
                WithinApproxCholOptions::default(),
            )?;
            let warnings = preconditioner.warnings().len();
            let bytes = preconditioner
                .memory_report()
                .known_retained_bytes_estimate();
            (
                Action::Within(preconditioner),
                Some(bytes),
                warnings,
                TerminalSummary::default(),
            )
        }
    };
    let constructor_seconds = start.elapsed().as_secs_f64();

    let rhs = problem.rhs_from_targets(target)?;
    let mut output = vec![0.0; problem.dimension()];
    let initialization_start = Instant::now();
    action.preconditioner().apply(&rhs, &mut output)?;
    let initialization_seconds = initialization_start.elapsed().as_secs_f64();
    if output.iter().any(|value| !value.is_finite()) {
        return Err("nonfinite preconditioner initialization output".into());
    }

    Ok(Built {
        action,
        constructor_seconds,
        initialization_seconds,
        known_retained_bytes,
        warning_count,
        terminal,
    })
}

#[derive(Default)]
struct OneSolve {
    seconds: f64,
    iterations: usize,
    outer_work: usize,
    preconditioner_applications: usize,
    certificate_work: usize,
    true_residual: f64,
    converged: bool,
    certified: bool,
    error: String,
}

fn run_lsmr(problem: &ThreeWayProblem, target: &[f64], built: &Built) -> OneSolve {
    let start = Instant::now();
    let result = solve_weighted_least_squares(
        problem,
        target,
        built.action.preconditioner(),
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
            OneSolve {
                seconds,
                iterations: result.iterations(),
                outer_work: result.work().solver_outer_operator_applications(),
                preconditioner_applications: result.work().preconditioner_applications(),
                certificate_work: result.work().certification_incidence_applications()
                    + result.work().certification_adjoint_applications(),
                true_residual: residual,
                converged: result.converged(),
                certified: residual.is_finite() && residual <= CERTIFICATE_TOLERANCE,
                error: String::new(),
            }
        }
        Err(error) => OneSolve {
            seconds,
            true_residual: f64::INFINITY,
            error: sanitize(&error.to_string()),
            ..OneSolve::default()
        },
    }
}

fn run_pcg(problem: &ThreeWayProblem, target: &[f64], built: &Built) -> OneSolve {
    let rhs = match problem.rhs_from_targets(target) {
        Ok(value) => value,
        Err(error) => {
            return OneSolve {
                true_residual: f64::INFINITY,
                error: sanitize(&error.to_string()),
                ..OneSolve::default()
            };
        }
    };
    let start = Instant::now();
    let result = solve_projected_pcg_traced(
        problem,
        &rhs,
        built.action.preconditioner(),
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
            OneSolve {
                seconds,
                iterations: result.iterations(),
                outer_work: result.gramian_applications(),
                preconditioner_applications: result.preconditioner_applications(),
                certificate_work: 0,
                true_residual: residual,
                converged: result.converged(),
                certified: residual.is_finite() && residual <= CERTIFICATE_TOLERANCE,
                error: String::new(),
            }
        }
        Err(error) => OneSolve {
            seconds,
            true_residual: f64::INFINITY,
            error: sanitize(&error.to_string()),
            ..OneSolve::default()
        },
    }
}

#[derive(Default)]
struct Batch {
    solve_seconds: f64,
    iterations: usize,
    outer_work: usize,
    preconditioner_applications: usize,
    certificate_work: usize,
    max_true_residual: f64,
    converged: bool,
    certified: bool,
    error: String,
}

impl Batch {
    fn new() -> Self {
        Self {
            converged: true,
            certified: true,
            ..Self::default()
        }
    }

    fn add(&mut self, rhs: usize, solve: OneSolve) {
        self.solve_seconds += solve.seconds;
        self.iterations += solve.iterations;
        self.outer_work += solve.outer_work;
        self.preconditioner_applications += solve.preconditioner_applications;
        self.certificate_work += solve.certificate_work;
        self.max_true_residual = self.max_true_residual.max(solve.true_residual);
        self.converged &= solve.converged;
        self.certified &= solve.certified;
        if !solve.error.is_empty() {
            self.error
                .push_str(&format!("rhs {rhs}: {}; ", solve.error));
        }
    }
}

fn main() -> Result<()> {
    let mut args = std::env::args().skip(1);
    let output = PathBuf::from(
        args.next()
            .unwrap_or_else(|| "issue4-size-ladder".to_owned()),
    );
    if args.next().is_some() {
        return Err("usage: issue4_size_ladder [output-directory]".into());
    }
    fs::create_dir_all(&output)?;
    let mut writer = BufWriter::new(File::create(output.join("size-ladder.tsv"))?);
    writeln!(
        writer,
        "family\tlevels\trepeat\tmethod\tsolver\trhs_count\ttuples\tcomponents\tconstructor_seconds\tinitialization_seconds\tsetup_seconds\tcumulative_solve_seconds\tsetup_plus_solve_seconds\tcumulative_iterations\tcumulative_outer_work\twork_unit\tcumulative_preconditioner_applications\tcumulative_certificate_work\tmax_true_residual\tconverged\tcertified\tknown_retained_bytes\tpair_components\tmax_pair_vertices\tmax_pair_edges\tmax_pair_cycle_excess\tmax_pair_levels\tmultilevel_pair_components\tdirect_pair_components\tfull_contraction_components\tstagnated_vertex_components\tstagnated_fill_components\tmaximum_levels_components\tone_level_iterative_components\tdirect_factor_components\tfallback_allocations\twarning_count\terror"
    )?;

    let cases = cases()?;
    for (case_index, case) in cases.iter().enumerate() {
        for repeat in 0..2 {
            for method_index in 0..Method::ALL.len() {
                let method = Method::ALL[(method_index + repeat + case_index) % Method::ALL.len()];
                let built = build(&case.problem, &case.targets[0], method)?;
                let solvers = if repeat % 2 == 0 {
                    ["mlsmr", "pcg-traced"]
                } else {
                    ["pcg-traced", "mlsmr"]
                };
                for solver in solvers {
                    let mut batch = Batch::new();
                    for (rhs_index, target) in case.targets.iter().enumerate() {
                        let one = if solver == "mlsmr" {
                            run_lsmr(&case.problem, target, &built)
                        } else {
                            run_pcg(&case.problem, target, &built)
                        };
                        let count = rhs_index + 1;
                        batch.add(count, one);
                        if !PREFIXES.contains(&count) {
                            continue;
                        }
                        let terminal = &built.terminal;
                        let work_unit = if solver == "mlsmr" {
                            "rectangular-operator"
                        } else {
                            "gramian"
                        };
                        writeln!(
                            writer,
                            "{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{:.9e}\t{:.9e}\t{:.9e}\t{:.9e}\t{:.9e}\t{}\t{}\t{}\t{}\t{}\t{:.9e}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}",
                            case.family,
                            case.levels,
                            repeat,
                            method.label(),
                            solver,
                            count,
                            case.problem.tuple_count(),
                            case.problem.components().count(),
                            built.constructor_seconds,
                            built.initialization_seconds,
                            built.setup_seconds(),
                            batch.solve_seconds,
                            built.setup_seconds() + batch.solve_seconds,
                            batch.iterations,
                            batch.outer_work,
                            work_unit,
                            batch.preconditioner_applications,
                            batch.certificate_work,
                            batch.max_true_residual,
                            batch.converged,
                            batch.certified,
                            built
                                .known_retained_bytes
                                .map_or_else(|| "NA".to_owned(), |value| value.to_string()),
                            terminal.pair_components,
                            terminal.max_pair_vertices,
                            terminal.max_pair_edges,
                            terminal.max_cycle_excess,
                            terminal.max_levels,
                            terminal.multilevel,
                            terminal.direct,
                            terminal.full_contraction,
                            terminal.stagnated_vertex,
                            terminal.stagnated_fill,
                            terminal.maximum_levels,
                            terminal.one_level_iterative,
                            terminal.direct_factors,
                            built.action.fallback_allocations(),
                            built.warning_count,
                            batch.error,
                        )?;
                        writer.flush()?;
                    }
                }
            }
        }
    }
    Ok(())
}

fn cases() -> Result<Vec<Case>> {
    let mut result = Vec::new();
    for &levels in &LEVELS {
        let planted = planted_clones(levels / 2, 2)?;
        result.push(make_case("planted-clones", levels, planted)?);
        let noisy = noisy_clones(levels / 3, 3)?;
        result.push(make_case("noisy-clones", levels, noisy)?);
        let latin = latin_square(levels)?;
        result.push(make_case("latin-square", levels, latin)?);
    }
    Ok(result)
}

fn make_case(family: &'static str, levels: usize, problem: ThreeWayProblem) -> Result<Case> {
    let targets = (0..MAX_RHS)
        .map(|rhs| exact_targets(&problem, rhs))
        .collect::<Result<Vec<_>>>()?;
    Ok(Case {
        family,
        levels,
        problem,
        targets,
    })
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

fn latin_square(levels: usize) -> Result<ThreeWayProblem> {
    let mut tuples = Vec::with_capacity(levels * levels);
    for first in 0..levels {
        for second in 0..levels {
            tuples.push([
                first as u32,
                second as u32,
                ((first + second) % levels) as u32,
            ]);
        }
    }
    let weights: Vec<_> = (0..tuples.len())
        .map(|index| 0.8 + (index % 11) as f64 / 10.0)
        .collect();
    Ok(ThreeWayProblem::from_observations(
        [levels; 3],
        &tuples,
        &weights,
    )?)
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

fn sanitize(value: &str) -> String {
    value.replace(['\t', '\n', '\r'], " ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ladder_is_frozen_and_aligned_on_factor_levels() {
        let cases = cases().unwrap();
        assert_eq!(cases.len(), 3 * LEVELS.len());
        for &levels in &LEVELS {
            let selected: Vec<_> = cases.iter().filter(|case| case.levels == levels).collect();
            assert_eq!(selected.len(), 3);
            for case in selected {
                assert_eq!(case.problem.topology().level_counts(), [levels; 3]);
                assert_eq!(case.targets.len(), MAX_RHS);
            }
        }
    }

    #[test]
    fn terminal_accounting_covers_every_pair_component() {
        let problem = latin_square(24).unwrap();
        let target = exact_targets(&problem, 0).unwrap();
        let built = build(&problem, &target, Method::PairCmg).unwrap();
        let terminal = &built.terminal;
        assert!(terminal.pair_components > 0);
        assert_eq!(
            terminal.direct
                + terminal.full_contraction
                + terminal.stagnated_vertex
                + terminal.stagnated_fill
                + terminal.maximum_levels,
            terminal.pair_components
        );
        assert_eq!(terminal.direct, terminal.direct_factors);
    }
}
