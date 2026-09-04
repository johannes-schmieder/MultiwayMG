//! Identical-domain pair-local economics; no three-way coarse correction.
//!
//! Run in release mode. This is a controlled research harness, not production
//! routing. See docs/ISSUE4_PAIR_LOCAL_PROTOCOL.md for measurement boundaries.

use std::{
    error::Error,
    fs::{self, File},
    hint::black_box,
    io::{BufWriter, Write},
    path::PathBuf,
    sync::{
        Mutex,
        atomic::{AtomicUsize, Ordering},
    },
    time::Instant,
};

use cmg::{CmgOptions, CmgPreconditioner, CmgWorkspace, Components, Laplacian};
use nalgebra::{DMatrix, DVector, linalg::SymmetricEigen};
use schwarz_precond::{MlsmrOptions, Operator, ReductionStrategy, SolveError, mlsmr};
use within::{Effect, LocalSolverConfig, PreconditionerConfig, Solver};

type Result<T> = std::result::Result<T, Box<dyn Error>>;
const RHS_COUNTS: [usize; 4] = [1, 4, 16, 32];
const CERTIFICATE_TOLERANCE: f64 = 1.0e-8;
const DIRECT_THRESHOLD: usize = 8;

fn invalid(message: impl ToString) -> SolveError {
    SolveError::InvalidInput {
        context: "issue4 pair-local",
        message: message.to_string(),
    }
}

fn norm(values: &[f64]) -> f64 {
    let scale = values.iter().copied().map(f64::abs).fold(0.0, f64::max);
    if scale == 0.0 {
        return 0.0;
    }
    scale
        * values
            .iter()
            .map(|x| (x / scale).powi(2))
            .sum::<f64>()
            .sqrt()
}

/// The null vector in signless pair coordinates is (+1, -1).
fn project(left: usize, values: &mut [f64]) {
    let scale = values.iter().copied().map(f64::abs).fold(0.0, f64::max);
    if scale == 0.0 {
        return;
    }
    let mut sum: f64 = 0.0;
    let mut correction: f64 = 0.0;
    for (i, &x) in values.iter().enumerate() {
        let value = if i < left { x / scale } else { -x / scale };
        let next = sum + value;
        correction += if sum.abs() >= value.abs() {
            (sum - next) + value
        } else {
            (value - next) + sum
        };
        sum = next;
    }
    let mean = ((sum + correction) / values.len() as f64) * scale;
    for (i, x) in values.iter_mut().enumerate() {
        *x -= if i < left { mean } else { -mean };
    }
}

struct Domain {
    left: usize,
    right: usize,
    graph: Laplacian,
    b_calls: AtomicUsize,
    bt_calls: AtomicUsize,
}

impl Domain {
    fn new(left: usize, right: usize, edges: &[(u32, u32, f64)]) -> Result<Self> {
        let n = left.checked_add(right).ok_or("pair dimension overflow")?;
        if left == 0 || right == 0 || n > u32::MAX as usize {
            return Err("invalid pair dimensions".into());
        }
        if edges
            .iter()
            .any(|&(a, b, _)| a as usize >= left || b as usize >= right)
        {
            return Err("pair endpoint outside its factor".into());
        }
        // CMG canonicalizes duplicate edges by sorted compensated summation.
        let graph = Laplacian::from_edges(
            n,
            edges
                .iter()
                .map(|&(a, b, w)| (a as usize, left + b as usize, w)),
        )?;
        if Components::from_laplacian(&graph).count() != 1 {
            return Err("pair domain must be connected and cover all declared levels".into());
        }
        Ok(Self {
            left,
            right,
            graph,
            b_calls: AtomicUsize::new(0),
            bt_calls: AtomicUsize::new(0),
        })
    }

    fn dense_reference(&self) -> Result<(DMatrix<f64>, DMatrix<f64>, DVector<f64>)> {
        let n = self.ncols();
        if n > 256 {
            return Err("dense pair reference is limited to 256 vertices".into());
        }
        let mut gram = DMatrix::<f64>::zeros(n, n);
        for edge in self.graph.edges() {
            for (i, j) in [
                (edge.u(), edge.u()),
                (edge.v(), edge.v()),
                (edge.u(), edge.v()),
                (edge.v(), edge.u()),
            ] {
                gram[(i, j)] += edge.weight();
            }
        }
        let scale = gram.diagonal().amax();
        let decomposition = SymmetricEigen::new(gram / scale);
        let mut positive = Vec::new();
        for (i, &value) in decomposition.eigenvalues.iter().enumerate() {
            if !value.is_finite() || value < -1.0e-12 {
                return Err("invalid dense pair spectrum".into());
            }
            if value > 1.0e-12 {
                positive.push(i);
            }
        }
        // Never silently discard an unresolved positive mode of a connected graph.
        if positive.len() != n - 1 {
            return Err("dense reference cannot resolve the full pair range".into());
        }
        let basis = DMatrix::from_fn(n, n - 1, |i, j| {
            decomposition.eigenvectors[(i, positive[j])]
        });
        let eigenvalues = DVector::from_iterator(
            n - 1,
            positive
                .iter()
                .map(|&i| decomposition.eigenvalues[i] * scale),
        );
        let inverse =
            &basis * DMatrix::from_diagonal(&eigenvalues.map(|x| 1.0 / x)) * basis.transpose();
        Ok((inverse, basis, eigenvalues))
    }

    fn targets(&self, rhs: usize) -> Vec<f64> {
        let coefficients: Vec<_> = (0..self.ncols())
            .map(|i| ((i + 1) as f64 * (0.13 + rhs as f64 * 0.017)).sin())
            .collect();
        self.graph
            .edges()
            .iter()
            .enumerate()
            .map(|(i, e)| {
                e.weight().sqrt()
                    * (coefficients[e.u()]
                        + coefficients[e.v()]
                        + 0.03 * ((i + 3 * rhs) as f64 * 0.31).cos())
            })
            .collect()
    }
}

impl Operator for Domain {
    fn nrows(&self) -> usize {
        self.graph.edge_count()
    }
    fn ncols(&self) -> usize {
        self.left + self.right
    }
    fn apply(&self, x: &[f64], y: &mut [f64]) -> std::result::Result<(), SolveError> {
        y.fill(0.0);
        if x.len() != self.ncols() || y.len() != self.nrows() || x.iter().any(|v| !v.is_finite()) {
            return Err(invalid("invalid weighted incidence input"));
        }
        self.b_calls.fetch_add(1, Ordering::Relaxed);
        for (out, edge) in y.iter_mut().zip(self.graph.edges()) {
            *out = edge.weight().sqrt() * (x[edge.u()] + x[edge.v()]);
        }
        if y.iter().any(|v| !v.is_finite()) {
            y.fill(0.0);
            return Err(invalid("nonfinite weighted incidence output"));
        }
        Ok(())
    }
    fn apply_adjoint(&self, x: &[f64], y: &mut [f64]) -> std::result::Result<(), SolveError> {
        y.fill(0.0);
        if x.len() != self.nrows() || y.len() != self.ncols() || x.iter().any(|v| !v.is_finite()) {
            return Err(invalid("invalid weighted adjoint input"));
        }
        self.bt_calls.fetch_add(1, Ordering::Relaxed);
        for (&value, edge) in x.iter().zip(self.graph.edges()) {
            let contribution = edge.weight().sqrt() * value;
            y[edge.u()] += contribution;
            y[edge.v()] += contribution;
        }
        if y.iter().any(|v| !v.is_finite()) {
            y.fill(0.0);
            return Err(invalid("nonfinite weighted adjoint output"));
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug)]
enum Method {
    Jacobi,
    Exact,
    Cmg,
    Within,
}
impl Method {
    const ALL: [Self; 4] = [Self::Jacobi, Self::Exact, Self::Cmg, Self::Within];
    fn label(self) -> &'static str {
        match self {
            Self::Jacobi => "jacobi",
            Self::Exact => "exact",
            Self::Cmg => "cmg-fixed",
            Self::Within => "within-default",
        }
    }
}

struct CmgState {
    inverse: CmgPreconditioner,
    workspace: CmgWorkspace,
}
enum Action {
    Jacobi(Vec<f64>),
    Exact(DMatrix<f64>),
    Cmg(Box<CmgState>),
    Within(within::Preconditioner),
}
struct LocalState {
    rhs: Vec<f64>,
    action: Action,
}
struct PairInverse {
    n: usize,
    left: usize,
    state: Mutex<LocalState>,
    calls: AtomicUsize,
    setup_seconds: f64,
    workspace_seconds: f64,
    principal_bytes: Option<usize>,
    workspace_bytes: usize,
    cmg_levels: usize,
    warnings: Vec<String>,
}

impl PairInverse {
    fn build(domain: &Domain, method: Method) -> Result<Self> {
        let start = Instant::now();
        let n = domain.ncols();
        let mut workspace_seconds = 0.0;
        let mut workspace_bytes = n * 8;
        let mut principal_bytes = None;
        let mut cmg_levels = 0;
        let mut warnings = Vec::new();
        let action = match method {
            Method::Jacobi => {
                principal_bytes = Some(n * 8);
                Action::Jacobi(domain.graph.diagonal().iter().map(|x| 1.0 / x).collect())
            }
            Method::Exact => {
                let (inverse, _, _) = domain.dense_reference()?;
                principal_bytes = Some(n * n * 8);
                Action::Exact(inverse)
            }
            Method::Cmg => {
                let inverse = CmgPreconditioner::build(
                    &domain.graph,
                    CmgOptions {
                        direct_threshold: DIRECT_THRESHOLD,
                        ..CmgOptions::default()
                    },
                )?;
                cmg_levels = inverse.hierarchy().levels().len();
                principal_bytes = Some(inverse.retained_bytes());
                let workspace_start = Instant::now();
                let workspace = inverse.try_workspace()?;
                workspace_seconds = workspace_start.elapsed().as_secs_f64();
                workspace_bytes += workspace.byte_len();
                Action::Cmg(Box::new(CmgState { inverse, workspace }))
            }
            Method::Within => {
                let a: Vec<_> = domain.graph.edges().iter().map(|e| e.u() as u32).collect();
                let b: Vec<_> = domain
                    .graph
                    .edges()
                    .iter()
                    .map(|e| (e.v() - domain.left) as u32)
                    .collect();
                let effects = [&a, &b]
                    .iter()
                    .map(|codes| Effect::new(codes, true, std::iter::empty::<&[f64]>()))
                    .collect::<std::result::Result<Vec<_>, _>>()?;
                let solver = Solver::new(
                    effects,
                    Some(domain.graph.edges().iter().map(|e| e.weight()).collect()),
                    PreconditionerConfig::Additive {
                        local_solver: LocalSolverConfig::default(),
                        reduction: ReductionStrategy::AtomicScatter,
                    },
                )?;
                warnings = solver.warnings().iter().map(ToString::to_string).collect();
                let inverse = solver
                    .preconditioner()
                    .cloned()
                    .ok_or("missing within preconditioner")?;
                if inverse.nrows() != n || inverse.ncols() != n {
                    return Err("within pair dimension mismatch".into());
                }
                Action::Within(inverse)
            }
        };
        let workspace_start = Instant::now();
        let rhs = vec![0.0; n];
        workspace_seconds += workspace_start.elapsed().as_secs_f64();
        Ok(Self {
            n,
            left: domain.left,
            state: Mutex::new(LocalState { rhs, action }),
            calls: AtomicUsize::new(0),
            setup_seconds: start.elapsed().as_secs_f64(),
            workspace_seconds,
            principal_bytes,
            workspace_bytes,
            cmg_levels,
            warnings,
        })
    }
}

impl Operator for PairInverse {
    fn nrows(&self) -> usize {
        self.n
    }
    fn ncols(&self) -> usize {
        self.n
    }
    fn apply(&self, x: &[f64], out: &mut [f64]) -> std::result::Result<(), SolveError> {
        out.fill(0.0);
        if x.len() != self.n || out.len() != self.n || x.iter().any(|v| !v.is_finite()) {
            return Err(invalid("invalid pair inverse input"));
        }
        self.calls.fetch_add(1, Ordering::Relaxed);
        let mut state = self
            .state
            .lock()
            .map_err(|_| invalid("pair workspace lock poisoned"))?;
        let LocalState { rhs, action } = &mut *state;
        rhs.copy_from_slice(x);
        project(self.left, rhs);
        let result = match action {
            Action::Jacobi(inverse) => {
                for ((y, &b), &d) in out.iter_mut().zip(rhs.iter()).zip(inverse.iter()) {
                    *y = b * d;
                }
                Ok(())
            }
            Action::Exact(inverse) => {
                for i in 0..self.n {
                    out[i] = (0..self.n).map(|j| inverse[(i, j)] * rhs[j]).sum();
                }
                Ok(())
            }
            Action::Within(inverse) => inverse.apply(rhs, out),
            Action::Cmg(state) => {
                for value in &mut rhs[self.left..] {
                    *value = -*value;
                }
                let result = state
                    .inverse
                    .apply_into(rhs, out, &mut state.workspace)
                    .map_err(invalid);
                for value in &mut out[self.left..] {
                    *value = -*value;
                }
                result
            }
        };
        if let Err(error) = result {
            out.fill(0.0);
            return Err(error);
        }
        project(self.left, out);
        if out.iter().any(|v| !v.is_finite()) {
            out.fill(0.0);
            return Err(invalid("nonfinite pair inverse output"));
        }
        Ok(())
    }
    fn apply_adjoint(&self, x: &[f64], y: &mut [f64]) -> std::result::Result<(), SolveError> {
        self.apply(x, y)
    }
}

#[derive(Debug)]
struct Quality {
    symmetry: f64,
    linearity: f64,
    minimum: f64,
    condition: f64,
    inverse_error: f64,
}
fn quality(domain: &Domain, inverse: &PairInverse) -> Result<Quality> {
    let (exact, basis, eigenvalues) = domain.dense_reference()?;
    let n = domain.ncols();
    let mut materialized = DMatrix::<f64>::zeros(n, n);
    let mut rhs = vec![0.0; n];
    let mut out = vec![0.0; n];
    for j in 0..n {
        rhs.fill(0.0);
        rhs[j] = 1.0;
        inverse.apply(&rhs, &mut out)?;
        for i in 0..n {
            materialized[(i, j)] = out[i];
        }
    }
    let symmetry = (&materialized - materialized.transpose()).norm() / materialized.norm();
    let inverse_error = (&materialized - &exact).norm() / exact.norm();
    let roots = DMatrix::from_diagonal(&eigenvalues.map(f64::sqrt));
    let energy = &roots * basis.transpose() * &materialized * basis * roots;
    let spectrum = SymmetricEigen::new((&energy + energy.transpose()) * 0.5).eigenvalues;
    let minimum = spectrum.min();
    let condition = spectrum.max() / minimum;
    let x: Vec<_> = (0..n).map(|i| (i as f64 * 0.31).sin()).collect();
    let y: Vec<_> = (0..n).map(|i| (i as f64 * 0.17).cos()).collect();
    let combination: Vec<_> = x
        .iter()
        .zip(&y)
        .map(|(&a, &b)| -0.41 * a + 1.23 * b)
        .collect();
    let mut mx = vec![0.0; n];
    let mut my = vec![0.0; n];
    inverse.apply(&x, &mut mx)?;
    inverse.apply(&y, &mut my)?;
    inverse.apply(&combination, &mut out)?;
    let expected: Vec<_> = mx
        .iter()
        .zip(&my)
        .map(|(&a, &b)| -0.41 * a + 1.23 * b)
        .collect();
    let difference: Vec<_> = out.iter().zip(&expected).map(|(&a, &b)| a - b).collect();
    let linearity = norm(&difference) / norm(&expected).max(f64::MIN_POSITIVE);
    Ok(Quality {
        symmetry,
        linearity,
        minimum,
        condition,
        inverse_error,
    })
}

fn fixture(family: &str, size: usize) -> Vec<(u32, u32, f64)> {
    let mut edges = Vec::new();
    for i in 0..size {
        match family {
            "path" => {
                edges.push((i as u32, i as u32, 1.0));
                if i + 1 < size {
                    edges.push(((i + 1) as u32, i as u32, 1.0));
                }
            }
            "hubs" => {
                edges.push((i as u32, 0, 1.0));
                if i > 0 {
                    edges.push((0, i as u32, 1.0));
                }
            }
            "weak" => {
                let width = 8;
                let start = i / width * width;
                for j in start..(start + width).min(size) {
                    edges.push((i as u32, j as u32, 1.0));
                }
                if i > 0 && i % width == 0 {
                    edges.push((i as u32, (i - 1) as u32, 1.0e-3));
                }
            }
            "dense" => {
                for j in 0..size {
                    edges.push((
                        i as u32,
                        j as u32,
                        1.0 + ((i * 7 + j * 3) % 13) as f64 / 13.0,
                    ));
                }
            }
            "dynamic" => {
                for step in 0..3 {
                    edges.push((
                        i as u32,
                        ((i + step) % size) as u32,
                        10.0_f64.powi(((i * 7 + step * 11) % 7) as i32 - 3),
                    ));
                }
            }
            _ => unreachable!("frozen fixture family"),
        }
    }
    edges
}

#[derive(Default)]
struct Batch {
    seconds: f64,
    b: usize,
    bt: usize,
    preconditioner: usize,
    max_residual: f64,
    recurrence_converged: bool,
    certified: bool,
    error: String,
}
fn solve_one(domain: &Domain, inverse: &PairInverse, targets: &[f64]) -> Result<Batch> {
    let before_b = domain.b_calls.load(Ordering::Relaxed);
    let before_bt = domain.bt_calls.load(Ordering::Relaxed);
    let before_p = inverse.calls.load(Ordering::Relaxed);
    let start = Instant::now();
    let result = mlsmr(
        domain,
        targets,
        inverse,
        1.0e-10,
        2000,
        MlsmrOptions {
            warm_start: None,
            escalation: None,
            local_size: Some(8),
        },
    )?;
    let b = domain.b_calls.load(Ordering::Relaxed) - before_b;
    let bt = domain.bt_calls.load(Ordering::Relaxed) - before_bt;
    let preconditioner = inverse.calls.load(Ordering::Relaxed) - before_p;
    // Independent original-operator certificate; included in solve wall time.
    let mut residual = vec![0.0; domain.nrows()];
    domain.apply(&result.x, &mut residual)?;
    for (r, &y) in residual.iter_mut().zip(targets) {
        *r = y - *r;
    }
    let mut gradient = vec![0.0; domain.ncols()];
    let mut reference = vec![0.0; domain.ncols()];
    domain.apply_adjoint(&residual, &mut gradient)?;
    domain.apply_adjoint(targets, &mut reference)?;
    let denominator = norm(&reference);
    let max_residual = if denominator > 0.0 {
        norm(&gradient) / denominator
    } else if norm(&gradient) == 0.0 {
        0.0
    } else {
        f64::INFINITY
    };
    Ok(Batch {
        seconds: start.elapsed().as_secs_f64(),
        b,
        bt,
        preconditioner,
        max_residual,
        recurrence_converged: result.converged,
        certified: max_residual.is_finite() && max_residual <= CERTIFICATE_TOLERANCE,
        error: String::new(),
    })
}

fn main() -> Result<()> {
    let mut args = std::env::args().skip(1);
    let output = PathBuf::from(
        args.next()
            .unwrap_or_else(|| "issue4-pair-local".to_owned()),
    );
    let profile = args.next().unwrap_or_else(|| "smoke".to_owned());
    let selected = args.next().unwrap_or_else(|| "all".to_owned());
    if args.next().is_some()
        || !["smoke", "calibration"].contains(&profile.as_str())
        || (selected != "all" && !Method::ALL.iter().any(|m| m.label() == selected))
    {
        return Err("usage: issue4_pair_local [output-directory] [smoke|calibration] [all|jacobi|exact|cmg-fixed|within-default]".into());
    }
    fs::create_dir_all(&output)?;
    let mut writer = BufWriter::new(File::create(output.join("pair-local.tsv"))?);
    writeln!(
        writer,
        "profile\tfixture\trepeat\tmethod\tvertices\tedges\trhs_count\tdomain_seconds\tsetup_seconds\tworkspace_seconds\tapply_seconds\tsolve_seconds\ttotal_seconds\tsolver_b\tsolver_bt\tpreconditioner_calls\tcertificate_b\tcertificate_bt\tmax_true_residual\trecurrence_converged\tcertified\tprincipal_solver_bytes\tknown_workspace_bytes\tcommon_graph_bytes\tcmg_levels\tsymmetry_defect\tlinearity_defect\tminimum_energy_eigenvalue\trange_condition\trelative_inverse_error\twarning_count\terror"
    )?;
    let sizes: &[usize] = if profile == "smoke" {
        &[32]
    } else {
        &[64, 256]
    };
    for (fixture_id, (&size, family)) in sizes
        .iter()
        .flat_map(|n| ["path", "hubs", "weak", "dense", "dynamic"].map(|family| (n, family)))
        .enumerate()
    {
        let edges = fixture(family, size);
        let domain_start = Instant::now();
        let domain = Domain::new(size, size, &edges)?;
        let domain_seconds = domain_start.elapsed().as_secs_f64();
        let targets: Vec<_> = (0..32).map(|rhs| domain.targets(rhs)).collect();
        let methods: Vec<_> = Method::ALL
            .into_iter()
            .filter(|m| {
                (selected == "all" || m.label() == selected)
                    && (!matches!(m, Method::Exact) || domain.ncols() <= 256)
            })
            .collect();
        // Dedicated discarded warm-up builds; no hidden warm-up setup reuse.
        for &method in &methods {
            let warm = PairInverse::build(&domain, method)?;
            let _ = solve_one(&domain, &warm, &targets[0])?;
        }
        for repeat in 0..3 {
            for index in 0..methods.len() {
                let method = methods[(index + repeat + fixture_id) % methods.len()];
                let inverse = PairInverse::build(&domain, method)?;
                let diagnostics = if domain.ncols() <= 256 {
                    Some(quality(&domain, &inverse)?)
                } else {
                    None
                };
                let mut rhs = vec![0.0; domain.ncols()];
                domain.apply_adjoint(&targets[0], &mut rhs)?;
                let mut applied = vec![0.0; domain.ncols()];
                for _ in 0..3 {
                    inverse.apply(&rhs, &mut applied)?;
                }
                let apply_start = Instant::now();
                for _ in 0..25 {
                    inverse.apply(black_box(&rhs), black_box(&mut applied))?;
                }
                let apply_seconds = apply_start.elapsed().as_secs_f64() / 25.0;
                let mut batch = Batch {
                    recurrence_converged: true,
                    certified: true,
                    ..Batch::default()
                };
                for (rhs_index, target) in targets.iter().enumerate() {
                    match solve_one(&domain, &inverse, target) {
                        Ok(one) => {
                            batch.seconds += one.seconds;
                            batch.b += one.b;
                            batch.bt += one.bt;
                            batch.preconditioner += one.preconditioner;
                            batch.max_residual = batch.max_residual.max(one.max_residual);
                            batch.recurrence_converged &= one.recurrence_converged;
                            batch.certified &= one.certified;
                        }
                        Err(error) => {
                            batch.certified = false;
                            batch.recurrence_converged = false;
                            batch.error = error.to_string().replace(['\t', '\n', '\r'], " ");
                        }
                    }
                    let count = rhs_index + 1;
                    if !RHS_COUNTS.contains(&count) {
                        continue;
                    }
                    let quality_values = diagnostics.as_ref().map_or_else(
                        || "NA\tNA\tNA\tNA\tNA".to_owned(),
                        |q| {
                            format!(
                                "{:.9e}\t{:.9e}\t{:.9e}\t{:.9e}\t{:.9e}",
                                q.symmetry, q.linearity, q.minimum, q.condition, q.inverse_error
                            )
                        },
                    );
                    writeln!(
                        writer,
                        "{}\t{}-{}\t{}\t{}\t{}\t{}\t{}\t{:.9e}\t{:.9e}\t{:.9e}\t{:.9e}\t{:.9e}\t{:.9e}\t{}\t{}\t{}\t{}\t{}\t{:.9e}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}",
                        profile,
                        family,
                        size,
                        repeat,
                        method.label(),
                        domain.ncols(),
                        domain.nrows(),
                        count,
                        domain_seconds,
                        inverse.setup_seconds,
                        inverse.workspace_seconds,
                        apply_seconds,
                        batch.seconds,
                        domain_seconds + inverse.setup_seconds + batch.seconds,
                        batch.b,
                        batch.bt,
                        batch.preconditioner,
                        count,
                        2 * count,
                        batch.max_residual,
                        batch.recurrence_converged,
                        batch.certified,
                        inverse
                            .principal_bytes
                            .map_or_else(|| "NA".to_owned(), |x| x.to_string()),
                        inverse.workspace_bytes,
                        domain.graph.retained_bytes(),
                        inverse.cmg_levels,
                        quality_values,
                        inverse.warnings.len(),
                        batch.error
                    )?;
                    writer.flush()?;
                }
                for warning in &inverse.warnings {
                    eprintln!("{} {}: {}", family, method.label(), warning);
                }
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn invalid_domains_fail_closed() {
        for edges in [
            vec![(0, 0, 1.0), (1, 1, 1.0)],
            vec![(0, 2, 1.0)],
            vec![(0, 0, f64::NAN)],
            vec![(0, 0, 0.0)],
        ] {
            assert!(Domain::new(2, 2, &edges).is_err());
        }
        assert!(Domain::new(0, 1, &[]).is_err());
    }
    #[test]
    fn duplicate_collapse_is_order_invariant_and_keeps_small_positive_edges() {
        let edges = vec![
            (0, 0, 1.0e16),
            (0, 0, 1.0),
            (0, 0, 1.0),
            (1, 0, 1.0e-12),
            (1, 1, 1.0),
        ];
        let a = Domain::new(2, 2, &edges).unwrap();
        let mut reversed = edges;
        reversed.reverse();
        let b = Domain::new(2, 2, &reversed).unwrap();
        assert_eq!(a.graph, b.graph);
        assert_eq!(a.nrows(), 3);
    }
    #[test]
    fn pair_actions_are_linear_symmetric_positive_and_project_arbitrary_rhs() {
        for family in ["path", "weak", "dynamic"] {
            let domain = Domain::new(16, 16, &fixture(family, 16)).unwrap();
            for method in Method::ALL {
                let inverse = PairInverse::build(&domain, method).unwrap();
                let q = quality(&domain, &inverse).unwrap();
                assert!(
                    q.symmetry < 1.0e-9
                        && q.linearity < 1.0e-9
                        && q.minimum > 0.0
                        && q.condition.is_finite(),
                    "{family} {method:?}: {q:?}"
                );
                if matches!(method, Method::Exact) {
                    assert!(q.inverse_error < 1.0e-9 && (q.condition - 1.0).abs() < 1.0e-7);
                }
                let mut kernel = vec![-1.0; 32];
                kernel[..16].fill(1.0);
                let mut output = vec![7.0; 32];
                inverse.apply(&kernel, &mut output).unwrap();
                assert!(norm(&output) < 1.0e-12);
                let batch = solve_one(&domain, &inverse, &domain.targets(3)).unwrap();
                assert!(
                    batch.certified,
                    "{family} {method:?}: {}",
                    batch.max_residual
                );
            }
        }
    }
    #[test]
    fn nonfinite_and_bad_dimensions_do_not_leave_partial_output() {
        let domain = Domain::new(4, 4, &fixture("path", 4)).unwrap();
        for method in Method::ALL {
            let inverse = PairInverse::build(&domain, method).unwrap();
            let mut output = vec![7.0; 8];
            assert!(inverse.apply(&[f64::NAN; 8], &mut output).is_err());
            assert!(output.iter().all(|&x| x == 0.0));
            output.fill(7.0);
            assert!(inverse.apply(&[1.0; 7], &mut output).is_err());
            assert!(output.iter().all(|&x| x == 0.0));
            // Failure must not poison the reusable workspace.
            inverse.apply(&[1.0; 8], &mut output).unwrap();
            assert!(output.iter().all(|x| x.is_finite()));
        }
    }
}
