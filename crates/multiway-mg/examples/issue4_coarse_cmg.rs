//! Issue-4 calibration: preserve the frozen issue-3 maps and fine `within`
//! smoother, then change only the non-finest hierarchy smoothers.
//!
//! This is development/calibration evidence, not a holdout. The recursive
//! issue-3 fixtures and their numerical outcomes are already known. Any policy
//! suggested by this experiment must be frozen before a fresh issue-4 holdout.

#[allow(dead_code)]
#[path = "support/issue3_recursive_fixtures.rs"]
mod issue3_recursive_fixtures;

use std::{
    error::Error,
    fs::{self, File},
    io::{BufWriter, Write},
    path::PathBuf,
    time::Instant,
};

use cmg::{CmgOptions, TerminalReason};
use issue3_recursive_fixtures::{RecursiveHoldoutFixture, recursive_holdout_fixtures};
use multiway_mg::{
    AggregationRepairOptions, BootstrapAggregationOptions, CompatibleRelaxationCriteria,
    CompatibleRelaxationOptions, CycleQualityCriteria, CycleQualityOptions,
    CycleScreenedHierarchyOptions, CycleScreenedHierarchyPlan, DensePseudoinverse,
    FactorAggregation, LeastSquaresOptions, MultiwayError, PairCmgSchwarzOptions,
    PairCmgSchwarzPreconditioner, PcgTraceOptions, Preconditioner, ThreeWayProblem,
    WithinApproxCholOptions, WithinApproxCholPreconditioner, solve_projected_pcg_traced,
    solve_weighted_least_squares,
};

const PREFIXES: [usize; 4] = [1, 4, 16, 32];
const MAX_RHS: usize = 32;
const CERTIFICATE_TOLERANCE: f64 = 1.0e-8;
const DIRECT_THRESHOLD: usize = 8;
const TERMINAL_TOLERANCE: f64 = 1.0e-12;

type Result<T> = std::result::Result<T, Box<dyn Error>>;

#[derive(Clone, Copy, Debug)]
enum Method {
    WithinAllLevels,
    WithinFineCmgCoarse,
}

impl Method {
    const ALL: [Self; 2] = [Self::WithinAllLevels, Self::WithinFineCmgCoarse];

    fn label(self) -> &'static str {
        match self {
            Self::WithinAllLevels => "within-all-levels",
            Self::WithinFineCmgCoarse => "within-fine-cmg-coarse",
        }
    }
}

enum Smoother {
    Within(WithinApproxCholPreconditioner),
    PairCmg(PairCmgSchwarzPreconditioner),
}

impl Smoother {
    fn action(&self) -> &dyn Preconditioner {
        match self {
            Self::Within(value) => value,
            Self::PairCmg(value) => value,
        }
    }

    fn fallback_allocations(&self) -> usize {
        match self {
            Self::Within(value) => value.projection_fallback_allocations(),
            Self::PairCmg(value) => value.fallback_workspace_allocations(),
        }
    }
}

#[derive(Default)]
struct CmgSummary {
    components: usize,
    max_vertices: usize,
    max_edges: usize,
    max_cycle_excess: usize,
    max_levels: usize,
    multilevel: usize,
    direct: usize,
    full_contraction: usize,
    stagnated_vertex: usize,
    stagnated_fill: usize,
    maximum_levels: usize,
    one_level_iterative: usize,
}

impl CmgSummary {
    fn add(&mut self, preconditioner: &PairCmgSchwarzPreconditioner) {
        for report in preconditioner.component_reports() {
            self.components += 1;
            self.max_vertices = self.max_vertices.max(report.vertices());
            self.max_edges = self.max_edges.max(report.edges());
            self.max_cycle_excess = self.max_cycle_excess.max(report.cycle_excess());
            self.max_levels = self.max_levels.max(report.cmg_levels());
            self.multilevel += usize::from(report.cmg_levels() > 1);
            self.one_level_iterative +=
                usize::from(report.cmg_levels() == 1 && report.cmg_terminal().is_iterative());
            match report.cmg_terminal() {
                TerminalReason::Direct => self.direct += 1,
                TerminalReason::FullContraction => self.full_contraction += 1,
                TerminalReason::StagnatedVertexReduction => self.stagnated_vertex += 1,
                TerminalReason::StagnatedFill => self.stagnated_fill += 1,
                TerminalReason::MaximumLevels => self.maximum_levels += 1,
            }
        }
    }
}

struct ExperimentalHierarchy {
    problems: Vec<ThreeWayProblem>,
    maps: Vec<FactorAggregation>,
    smoothers: Vec<Smoother>,
    terminal: DensePseudoinverse,
    setup_seconds: f64,
    known_retained_bytes: usize,
    warning_count: usize,
    cmg: CmgSummary,
}

impl ExperimentalHierarchy {
    fn build(finest: ThreeWayProblem, maps: &[FactorAggregation], method: Method) -> Result<Self> {
        let start = Instant::now();
        let mut problems = Vec::with_capacity(maps.len() + 1);
        problems.push(finest);
        for map in maps {
            let coarse = map.coarsen(
                problems
                    .last()
                    .expect("experimental hierarchy retains its finest problem"),
            )?;
            problems.push(coarse);
        }

        let mut smoothers = Vec::with_capacity(maps.len());
        let mut known_retained_bytes = 0_usize;
        let mut warning_count = 0_usize;
        let mut cmg = CmgSummary::default();
        for (level, problem) in problems[..maps.len()].iter().enumerate() {
            let use_cmg = matches!(method, Method::WithinFineCmgCoarse) && level > 0;
            if use_cmg {
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
                known_retained_bytes = known_retained_bytes.saturating_add(
                    preconditioner
                        .memory_report()
                        .total_retained_bytes_estimate(),
                );
                cmg.add(&preconditioner);
                smoothers.push(Smoother::PairCmg(preconditioner));
            } else {
                let preconditioner = WithinApproxCholPreconditioner::build(
                    problem.clone(),
                    WithinApproxCholOptions::default(),
                )?;
                known_retained_bytes = known_retained_bytes.saturating_add(
                    preconditioner
                        .memory_report()
                        .known_retained_bytes_estimate(),
                );
                warning_count += preconditioner.warnings().len();
                smoothers.push(Smoother::Within(preconditioner));
            }
        }

        let terminal = DensePseudoinverse::from_problem(
            problems
                .last()
                .expect("experimental hierarchy retains a terminal"),
            TERMINAL_TOLERANCE,
        )?;
        Ok(Self {
            problems,
            maps: maps.to_vec(),
            smoothers,
            terminal,
            setup_seconds: start.elapsed().as_secs_f64(),
            known_retained_bytes,
            warning_count,
            cmg,
        })
    }

    fn fallback_allocations(&self) -> usize {
        self.smoothers
            .iter()
            .map(Smoother::fallback_allocations)
            .sum()
    }

    fn apply_level(
        &self,
        level: usize,
        rhs: &[f64],
    ) -> std::result::Result<Vec<f64>, MultiwayError> {
        let problem = &self.problems[level];
        if rhs.len() != problem.dimension() {
            return Err(MultiwayError::DimensionMismatch {
                context: "ExperimentalHierarchy::apply_level",
                expected: problem.dimension(),
                actual: rhs.len(),
            });
        }
        if level == self.maps.len() {
            let mut solution = vec![0.0; problem.dimension()];
            self.terminal.solve_into(rhs, &mut solution)?;
            problem
                .components()
                .project_structural_range(&mut solution)?;
            return Ok(solution);
        }

        let mut compatible_rhs = rhs.to_vec();
        problem
            .components()
            .project_structural_range(&mut compatible_rhs)?;
        let mut solution = vec![0.0; problem.dimension()];
        self.smoothers[level]
            .action()
            .apply(&compatible_rhs, &mut solution)?;

        let residual = problem.residual(&compatible_rhs, &solution)?;
        let coarse_problem = &self.problems[level + 1];
        let mut coarse_rhs = vec![0.0; coarse_problem.dimension()];
        self.maps[level].restrict(&residual, &mut coarse_rhs)?;
        coarse_problem
            .components()
            .project_structural_range(&mut coarse_rhs)?;
        let coarse_solution = self.apply_level(level + 1, &coarse_rhs)?;
        let mut prolonged = vec![0.0; problem.dimension()];
        self.maps[level].prolong(&coarse_solution, &mut prolonged)?;
        add_assign(&mut solution, &prolonged);

        let post_residual = problem.residual(&compatible_rhs, &solution)?;
        let mut post = vec![0.0; problem.dimension()];
        self.smoothers[level]
            .action()
            .apply(&post_residual, &mut post)?;
        add_assign(&mut solution, &post);
        problem
            .components()
            .project_structural_range(&mut solution)?;
        Ok(solution)
    }
}

impl Preconditioner for ExperimentalHierarchy {
    fn dimension(&self) -> usize {
        self.problems[0].dimension()
    }

    fn apply(&self, rhs: &[f64], out: &mut [f64]) -> std::result::Result<(), MultiwayError> {
        if rhs.len() != self.dimension() || out.len() != self.dimension() {
            return Err(MultiwayError::DimensionMismatch {
                context: "ExperimentalHierarchy::apply",
                expected: self.dimension(),
                actual: rhs.len().min(out.len()),
            });
        }
        let solution = self.apply_level(0, rhs)?;
        out.copy_from_slice(&solution);
        Ok(())
    }
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
            .unwrap_or_else(|| "issue4-coarse-cmg-output".to_owned()),
    );
    if args.next().is_some() {
        return Err("usage: issue4_coarse_cmg [output-directory]".into());
    }
    fs::create_dir_all(&output)?;
    let mut writer = BufWriter::new(File::create(output.join("coarse-cmg.tsv"))?);
    writeln!(
        writer,
        "case\tfamily\trequested_depth\tplan_accepted\tplan_depth\tplan_seconds\tdimension_complexity\ttuple_complexity\tmethod\trepeat\tsolver\trhs_count\tfine_dimension\tfine_tuples\tlevel_dimensions\tlevel_tuples\tnumerical_setup_seconds\tinitialization_seconds\tsetup_plus_solve_seconds\tcumulative_solve_seconds\tcumulative_iterations\tcumulative_outer_work\twork_unit\tcumulative_preconditioner_applications\tcumulative_certificate_work\tmax_true_residual\tconverged\tcertified\tknown_retained_bytes\tcmg_components\tcmg_max_vertices\tcmg_max_edges\tcmg_max_cycle_excess\tcmg_max_levels\tcmg_multilevel_components\tcmg_direct_components\tcmg_full_contraction_components\tcmg_stagnated_vertex_components\tcmg_stagnated_fill_components\tcmg_maximum_levels_components\tcmg_one_level_iterative_components\tfallback_allocations\twarning_count\terror"
    )?;

    for (case_index, fixture) in recursive_holdout_fixtures()?.iter().enumerate() {
        run_fixture(case_index, fixture, &mut writer)?;
    }
    Ok(())
}

fn run_fixture(
    case_index: usize,
    fixture: &RecursiveHoldoutFixture,
    writer: &mut BufWriter<File>,
) -> Result<()> {
    let plan_start = Instant::now();
    let plan = CycleScreenedHierarchyPlan::build(
        fixture.problem.clone(),
        hierarchy_options(fixture.terminal_dimension),
    )?;
    let plan_seconds = plan_start.elapsed().as_secs_f64();
    if !plan.accepted() || plan.depth() != fixture.depth {
        writeln!(
            writer,
            "{}\t{}\t{}\t{}\t{}\t{:.9e}\t{:.9e}\t{:.9e}\tNA\t0\tNA\t0\t{}\t{}\tNA\tNA\t0\t0\t0\t0\t0\t0\tNA\t0\t0\tinf\tfalse\tfalse\t0\t0\t0\t0\t0\t0\t0\t0\t0\t0\t0\t0\t0\t0\t0\tplan rejected: {:?}",
            fixture.name,
            fixture.family,
            fixture.depth,
            plan.accepted(),
            plan.depth(),
            plan_seconds,
            plan.dimension_complexity(),
            plan.tuple_complexity(),
            fixture.problem.dimension(),
            fixture.problem.tuple_count(),
            plan.stop_reason(),
        )?;
        return Ok(());
    }

    let maps = plan.aggregations();
    let targets = (0..MAX_RHS)
        .map(|rhs| exact_targets(&fixture.problem, rhs))
        .collect::<Result<Vec<_>>>()?;
    for repeat in 0..2 {
        for method_index in 0..Method::ALL.len() {
            let method = Method::ALL[(method_index + repeat + case_index) % Method::ALL.len()];
            let hierarchy = ExperimentalHierarchy::build(fixture.problem.clone(), maps, method)?;
            let rhs = fixture.problem.rhs_from_targets(&targets[0])?;
            let mut initialized = vec![0.0; fixture.problem.dimension()];
            let init_start = Instant::now();
            hierarchy.apply(&rhs, &mut initialized)?;
            let initialization_seconds = init_start.elapsed().as_secs_f64();
            if initialized.iter().any(|value| !value.is_finite()) {
                return Err("nonfinite hierarchy initialization output".into());
            }
            let solvers = if repeat % 2 == 0 {
                ["mlsmr", "pcg-traced"]
            } else {
                ["pcg-traced", "mlsmr"]
            };
            for solver in solvers {
                let mut batch = Batch::new();
                for (rhs_index, target) in targets.iter().enumerate() {
                    let one = if solver == "mlsmr" {
                        run_lsmr(&fixture.problem, target, &hierarchy)
                    } else {
                        run_pcg(&fixture.problem, target, &hierarchy)
                    };
                    let count = rhs_index + 1;
                    batch.add(count, one);
                    if !PREFIXES.contains(&count) {
                        continue;
                    }
                    let setup = plan_seconds + hierarchy.setup_seconds + initialization_seconds;
                    let work_unit = if solver == "mlsmr" {
                        "rectangular-operator"
                    } else {
                        "gramian"
                    };
                    writeln!(
                        writer,
                        "{}\t{}\t{}\ttrue\t{}\t{:.9e}\t{:.9e}\t{:.9e}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{:.9e}\t{:.9e}\t{:.9e}\t{:.9e}\t{}\t{}\t{}\t{}\t{}\t{:.9e}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}",
                        fixture.name,
                        fixture.family,
                        fixture.depth,
                        plan.depth(),
                        plan_seconds,
                        plan.dimension_complexity(),
                        plan.tuple_complexity(),
                        method.label(),
                        repeat,
                        solver,
                        count,
                        fixture.problem.dimension(),
                        fixture.problem.tuple_count(),
                        join_usize(hierarchy.problems.iter().map(ThreeWayProblem::dimension)),
                        join_usize(hierarchy.problems.iter().map(ThreeWayProblem::tuple_count)),
                        hierarchy.setup_seconds,
                        initialization_seconds,
                        setup + batch.solve_seconds,
                        batch.solve_seconds,
                        batch.iterations,
                        batch.outer_work,
                        work_unit,
                        batch.preconditioner_applications,
                        batch.certificate_work,
                        batch.max_true_residual,
                        batch.converged,
                        batch.certified,
                        hierarchy.known_retained_bytes,
                        hierarchy.cmg.components,
                        hierarchy.cmg.max_vertices,
                        hierarchy.cmg.max_edges,
                        hierarchy.cmg.max_cycle_excess,
                        hierarchy.cmg.max_levels,
                        hierarchy.cmg.multilevel,
                        hierarchy.cmg.direct,
                        hierarchy.cmg.full_contraction,
                        hierarchy.cmg.stagnated_vertex,
                        hierarchy.cmg.stagnated_fill,
                        hierarchy.cmg.maximum_levels,
                        hierarchy.cmg.one_level_iterative,
                        hierarchy.fallback_allocations(),
                        hierarchy.warning_count,
                        batch.error,
                    )?;
                    writer.flush()?;
                }
            }
        }
    }
    Ok(())
}

fn run_lsmr(
    problem: &ThreeWayProblem,
    target: &[f64],
    hierarchy: &ExperimentalHierarchy,
) -> OneSolve {
    let start = Instant::now();
    let result = solve_weighted_least_squares(
        problem,
        target,
        hierarchy,
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

fn run_pcg(
    problem: &ThreeWayProblem,
    target: &[f64],
    hierarchy: &ExperimentalHierarchy,
) -> OneSolve {
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
        hierarchy,
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

fn hierarchy_options(terminal_dimension: usize) -> CycleScreenedHierarchyOptions {
    CycleScreenedHierarchyOptions {
        maximum_levels: 4,
        terminal_dimension,
        maximum_dimension_complexity: 2.25,
        maximum_tuple_complexity: 2.25,
        bootstrap: bootstrap_options(),
        cycle_probe: CycleQualityOptions {
            test_vectors: 12,
            power_iterations: 24,
            tail_iterations: 6,
            correction_damping: 1.0,
            seed: 0x4d57_4d47_4359_4331,
            relative_zero_tolerance: 1.0e-13,
        },
        cycle_criteria: CycleQualityCriteria {
            maximum_estimated_energy_factor: 0.50,
            maximum_observed_energy_factor: Some(1.05),
            maximum_structural_defect: 1.0e-10,
        },
        terminal_relative_tolerance: TERMINAL_TOLERANCE,
    }
}

fn bootstrap_options() -> BootstrapAggregationOptions {
    BootstrapAggregationOptions {
        setup_test_vectors: 5,
        setup_sweeps: 5,
        setup_jacobi_omega: 0.5,
        maximum_neighbor_degree: 12,
        signature_window: 3,
        maximum_candidate_degree: 12,
        minimum_combined_affinity: 0.40,
        algebraic_affinity_weight: 0.75,
        structural_affinity_weight: 0.05,
        degree_affinity_weight: 0.10,
        signature_hit_weight: 0.10,
        structural_baseline_required_factor_ratio: 0.90,
        structural_baseline_maximum_dimension_overhead_ratio: 0.05,
        structural_baseline_maximum_tuple_overhead_ratio: 0.05,
        compatible_relaxation: CompatibleRelaxationOptions {
            test_vectors: 16,
            sweeps: 12,
            relaxation_damping: 1.0,
            seed: 0x4d57_4d47_4352_3031,
            relative_zero_tolerance: 1.0e-13,
        },
        compatible_criteria: CompatibleRelaxationCriteria {
            maximum_diagonal_factor_per_sweep: 0.85,
            maximum_energy_factor_per_sweep: Some(0.85),
            maximum_final_coarse_defect: 1.0e-10,
            maximum_final_structural_defect: 1.0e-10,
        },
        maximum_bootstrap_witnesses: 6,
        maximum_coarse_dimension_ratio: 0.80,
        minimum_tuple_reduction: 0.05,
        maximum_two_level_tuple_complexity: 1.95,
        split_repair: Some(AggregationRepairOptions {
            relaxation: CompatibleRelaxationOptions {
                test_vectors: 16,
                sweeps: 12,
                relaxation_damping: 1.0,
                seed: 0x4d57_4d47_4352_3031,
                relative_zero_tolerance: 1.0e-13,
            },
            criteria: CompatibleRelaxationCriteria {
                maximum_diagonal_factor_per_sweep: 0.85,
                maximum_energy_factor_per_sweep: Some(0.85),
                maximum_final_coarse_defect: 1.0e-10,
                maximum_final_structural_defect: 1.0e-10,
            },
            maximum_rounds: 18,
            maximum_coarse_dimension_ratio: 0.80,
            minimum_tuple_reduction: 0.05,
            maximum_two_level_tuple_complexity: 1.95,
            minimum_split_score_fraction: 0.001,
        }),
        seed: 0x4d57_4d47_434f_5645,
    }
}

fn join_usize(values: impl Iterator<Item = usize>) -> String {
    values
        .map(|value| value.to_string())
        .collect::<Vec<_>>()
        .join(";")
}

fn add_assign(destination: &mut [f64], source: &[f64]) {
    for (left, &right) in destination.iter_mut().zip(source) {
        *left += right;
    }
}

fn sanitize(value: &str) -> String {
    value.replace(['\t', '\n', '\r'], " ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn coarse_variant_preserves_fine_within_and_level_sequence() {
        let mut fixtures = recursive_holdout_fixtures().unwrap();
        let fixture = fixtures.remove(0);
        let plan = CycleScreenedHierarchyPlan::build(
            fixture.problem.clone(),
            hierarchy_options(fixture.terminal_dimension),
        )
        .unwrap();
        assert!(!plan.aggregations().is_empty());
        let within = ExperimentalHierarchy::build(
            fixture.problem.clone(),
            plan.aggregations(),
            Method::WithinAllLevels,
        )
        .unwrap();
        let hybrid = ExperimentalHierarchy::build(
            fixture.problem.clone(),
            plan.aggregations(),
            Method::WithinFineCmgCoarse,
        )
        .unwrap();
        assert_eq!(within.maps.len(), hybrid.maps.len());
        assert_eq!(within.problems.len(), hybrid.problems.len());
        for (left, right) in within.problems.iter().zip(&hybrid.problems) {
            assert_eq!(left.dimension(), right.dimension());
            assert_eq!(left.tuple_count(), right.tuple_count());
            assert_eq!(
                left.topology().level_counts(),
                right.topology().level_counts()
            );
        }
        assert!(matches!(within.smoothers[0], Smoother::Within(_)));
        assert!(matches!(hybrid.smoothers[0], Smoother::Within(_)));
        assert!(
            hybrid.smoothers[1..]
                .iter()
                .all(|smoother| matches!(smoother, Smoother::PairCmg(_)))
        );
    }

    #[test]
    fn hybrid_action_is_numerically_linear_and_symmetric() {
        let mut fixtures = recursive_holdout_fixtures().unwrap();
        let fixture = fixtures.remove(0);
        let plan = CycleScreenedHierarchyPlan::build(
            fixture.problem.clone(),
            hierarchy_options(fixture.terminal_dimension),
        )
        .unwrap();
        let hybrid = ExperimentalHierarchy::build(
            fixture.problem.clone(),
            plan.aggregations(),
            Method::WithinFineCmgCoarse,
        )
        .unwrap();
        let n = hybrid.dimension();
        let x: Vec<_> = (0..n).map(|i| ((i + 1) as f64 * 0.17).sin()).collect();
        let y: Vec<_> = (0..n).map(|i| ((i + 3) as f64 * 0.11).cos()).collect();
        let mut axby: Vec<_> = x
            .iter()
            .zip(&y)
            .map(|(&left, &right)| 0.7 * left - 1.3 * right)
            .collect();
        fixture
            .problem
            .components()
            .project_structural_range(&mut axby)
            .unwrap();
        let mut px = vec![0.0; n];
        let mut py = vec![0.0; n];
        let mut pcombo = vec![0.0; n];
        hybrid.apply(&x, &mut px).unwrap();
        hybrid.apply(&y, &mut py).unwrap();
        hybrid.apply(&axby, &mut pcombo).unwrap();
        let linear_defect = pcombo
            .iter()
            .zip(px.iter().zip(&py))
            .map(|(&actual, (&left, &right))| (actual - (0.7 * left - 1.3 * right)).abs())
            .fold(0.0, f64::max);
        assert!(linear_defect <= 1.0e-9, "linearity defect {linear_defect}");
        let xpy: f64 = x.iter().zip(&py).map(|(&a, &b)| a * b).sum();
        let pxy: f64 = px.iter().zip(&y).map(|(&a, &b)| a * b).sum();
        let symmetry_scale = xpy.abs().max(pxy.abs()).max(1.0);
        assert!((xpy - pxy).abs() <= 1.0e-9 * symmetry_scale);
    }
}
