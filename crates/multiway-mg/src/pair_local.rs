//! Identical-domain pair-local solvers and dense diagnostics for issue #4.
//!
//! This module isolates one connected weighted bipartite factor-pair problem
//! from the surrounding three-way hierarchy.  The exact pseudoinverse, the
//! frozen `within` approximate-Cholesky path, and a fixed CMG action all consume
//! the same canonical [`PairDomain`].

use std::{
    collections::BTreeMap,
    sync::{
        Mutex,
        atomic::{AtomicUsize, Ordering},
    },
    time::{Duration, Instant},
};

use cmg::{CmgOptions, CmgPreconditioner, CmgWorkspace, Components, Laplacian};
use nalgebra::{DMatrix, linalg::SymmetricEigen};
use within::{Effect, PreconditionerConfig, Solver};

use crate::{MultiwayError, Preconditioner, WithinApproxCholOptions};

/// One canonical positive-weight edge in a bipartite pair domain.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PairEdge {
    left: u32,
    right: u32,
    weight: f64,
}

impl PairEdge {
    /// Zero-based endpoint in the first factor block.
    #[must_use]
    pub const fn left(self) -> u32 {
        self.left
    }

    /// Zero-based endpoint in the second factor block.
    #[must_use]
    pub const fn right(self) -> u32 {
        self.right
    }

    /// Strictly positive finite edge weight.
    #[must_use]
    pub const fn weight(self) -> f64 {
        self.weight
    }
}

/// One connected weighted bipartite pair problem in canonical edge order.
#[derive(Debug, Clone)]
pub struct PairDomain {
    left_count: usize,
    right_count: usize,
    edges: Vec<PairEdge>,
    graph: Laplacian,
    degrees: Vec<usize>,
    build_duration: Duration,
}

impl PairDomain {
    /// Validate, aggregate, and canonicalize a connected bipartite domain.
    ///
    /// Duplicate `(left, right)` entries are summed deterministically with
    /// compensated summation. Every declared vertex must be incident to a
    /// positive edge, and the resulting graph must contain exactly one
    /// connected component.
    pub fn from_edges<I>(
        left_count: usize,
        right_count: usize,
        edges: I,
    ) -> Result<Self, MultiwayError>
    where
        I: IntoIterator<Item = (u32, u32, f64)>,
    {
        let start = Instant::now();
        if left_count == 0 || right_count == 0 {
            return Err(pair_domain_error(format!(
                "both factor counts must be positive, got {left_count} and {right_count}"
            )));
        }
        if left_count > u32::MAX as usize || right_count > u32::MAX as usize {
            return Err(pair_domain_error(format!(
                "factor counts exceed the u32 code range: {left_count} and {right_count}"
            )));
        }

        let mut accumulated: BTreeMap<(u32, u32), (f64, f64)> = BTreeMap::new();
        for (position, (left, right, weight)) in edges.into_iter().enumerate() {
            if left as usize >= left_count {
                return Err(pair_domain_error(format!(
                    "edge {position} left endpoint {left} is outside 0..{left_count}"
                )));
            }
            if right as usize >= right_count {
                return Err(pair_domain_error(format!(
                    "edge {position} right endpoint {right} is outside 0..{right_count}"
                )));
            }
            if !weight.is_finite() || weight <= 0.0 {
                return Err(pair_domain_error(format!(
                    "edge {position} weight must be finite and positive, got {weight}"
                )));
            }
            let (sum, correction) = accumulated.entry((left, right)).or_insert((0.0, 0.0));
            neumaier_add(sum, correction, weight);
        }
        if accumulated.is_empty() {
            return Err(pair_domain_error(
                "at least one edge is required".to_owned(),
            ));
        }

        let mut canonical = Vec::with_capacity(accumulated.len());
        for ((left, right), (sum, correction)) in accumulated {
            let weight = sum + correction;
            if !weight.is_finite() || weight <= 0.0 {
                return Err(pair_domain_error(format!(
                    "aggregated edge ({left}, {right}) has invalid weight {weight}"
                )));
            }
            canonical.push(PairEdge {
                left,
                right,
                weight,
            });
        }

        let dimension = left_count + right_count;
        let graph = Laplacian::from_edges(
            dimension,
            canonical.iter().map(|edge| {
                (
                    edge.left as usize,
                    left_count + edge.right as usize,
                    edge.weight,
                )
            }),
        )
        .map_err(|error| pair_domain_error(error.to_string()))?;
        let components = Components::from_laplacian(&graph);
        if components.count() != 1 {
            return Err(pair_domain_error(format!(
                "domain must be connected and cover every declared vertex, found {} components",
                components.count()
            )));
        }

        let mut degrees = vec![0usize; dimension];
        for edge in &canonical {
            degrees[edge.left as usize] += 1;
            degrees[left_count + edge.right as usize] += 1;
        }
        if let Some(vertex) = degrees.iter().position(|&degree| degree == 0) {
            return Err(pair_domain_error(format!(
                "declared vertex {vertex} is not incident to a positive edge"
            )));
        }

        Ok(Self {
            left_count,
            right_count,
            edges: canonical,
            graph,
            degrees,
            build_duration: start.elapsed(),
        })
    }

    /// Number of vertices in the first factor block.
    #[must_use]
    pub const fn left_count(&self) -> usize {
        self.left_count
    }

    /// Number of vertices in the second factor block.
    #[must_use]
    pub const fn right_count(&self) -> usize {
        self.right_count
    }

    /// Total coefficient dimension.
    #[must_use]
    pub const fn dimension(&self) -> usize {
        self.left_count + self.right_count
    }

    /// Canonical unique weighted edges.
    #[must_use]
    pub fn edges(&self) -> &[PairEdge] {
        &self.edges
    }

    /// Number of canonical unique edges.
    #[must_use]
    pub fn edge_count(&self) -> usize {
        self.edges.len()
    }

    /// Graph cycle excess `edges - vertices + 1`.
    #[must_use]
    pub fn cycle_excess(&self) -> usize {
        self.edge_count() + 1 - self.dimension()
    }

    /// Minimum unique-edge degree over all declared vertices.
    #[must_use]
    pub fn minimum_degree(&self) -> usize {
        self.degrees.iter().copied().min().unwrap_or(0)
    }

    /// Maximum unique-edge degree over all declared vertices.
    #[must_use]
    pub fn maximum_degree(&self) -> usize {
        self.degrees.iter().copied().max().unwrap_or(0)
    }

    /// Ratio of largest to smallest positive edge weight.
    #[must_use]
    pub fn weight_dynamic_range(&self) -> f64 {
        let minimum = self
            .edges
            .iter()
            .map(|edge| edge.weight)
            .fold(f64::INFINITY, f64::min);
        let maximum = self
            .edges
            .iter()
            .map(|edge| edge.weight)
            .fold(0.0, f64::max);
        maximum / minimum
    }

    /// Wall time spent validating, aggregating, and constructing the graph.
    #[must_use]
    pub const fn build_duration(&self) -> Duration {
        self.build_duration
    }

    /// Principal retained bytes for canonical edges, graph arrays, and degrees.
    #[must_use]
    pub fn retained_bytes_estimate(&self) -> usize {
        self.edges
            .capacity()
            .saturating_mul(core::mem::size_of::<PairEdge>())
            .saturating_add(self.graph.retained_bytes())
            .saturating_add(
                self.degrees
                    .capacity()
                    .saturating_mul(core::mem::size_of::<usize>()),
            )
    }

    /// Materialize the plus-sign pair Gramian `B' W B`.
    #[must_use]
    pub fn dense_gramian(&self) -> DMatrix<f64> {
        let mut gramian = DMatrix::zeros(self.dimension(), self.dimension());
        for edge in &self.edges {
            let left = edge.left as usize;
            let right = self.left_count + edge.right as usize;
            gramian[(left, left)] += edge.weight;
            gramian[(right, right)] += edge.weight;
            gramian[(left, right)] += edge.weight;
            gramian[(right, left)] += edge.weight;
        }
        gramian
    }

    /// Apply the plus-sign pair Gramian without materializing it.
    pub fn apply_gramian(&self, input: &[f64], output: &mut [f64]) -> Result<(), MultiwayError> {
        validate_pair_vectors("PairDomain::apply_gramian", self.dimension(), input, output)?;
        output.fill(0.0);
        for edge in &self.edges {
            let left = edge.left as usize;
            let right = self.left_count + edge.right as usize;
            let value = edge.weight * (input[left] + input[right]);
            output[left] += value;
            output[right] += value;
        }
        Ok(())
    }

    /// Euclidean projection onto the numerical range of a connected pair Gramian.
    ///
    /// The plus-sign Gramian has structural null vector `(1_left, -1_right)`.
    pub fn project_range_in_place(&self, values: &mut [f64]) -> Result<(), MultiwayError> {
        if values.len() != self.dimension() {
            return Err(crate::error::dimension(
                "PairDomain::project_range_in_place",
                self.dimension(),
                values.len(),
            ));
        }
        let mut sum = 0.0;
        let mut correction = 0.0;
        for &value in &values[..self.left_count] {
            neumaier_add(&mut sum, &mut correction, value);
        }
        for &value in &values[self.left_count..] {
            neumaier_add(&mut sum, &mut correction, -value);
        }
        let shift = (sum + correction) / self.dimension() as f64;
        for value in &mut values[..self.left_count] {
            *value -= shift;
        }
        for value in &mut values[self.left_count..] {
            *value += shift;
        }
        Ok(())
    }

    fn graph(&self) -> &Laplacian {
        &self.graph
    }
}

/// Options for the small dense pair pseudoinverse.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PairExactOptions {
    /// Relative eigenvalue threshold for the numerical range.
    pub relative_rank_tolerance: f64,
    /// Maximum dimension admitted to dense factorization.
    pub maximum_dimension: usize,
}

impl Default for PairExactOptions {
    fn default() -> Self {
        Self {
            relative_rank_tolerance: 1.0e-12,
            maximum_dimension: 512,
        }
    }
}

/// Exact dense pseudoinverse of one connected pair Gramian.
#[derive(Debug, Clone)]
pub struct PairExactPseudoinverse {
    domain: PairDomain,
    inverse: DMatrix<f64>,
    rank: usize,
    threshold: f64,
    build_duration: Duration,
}

impl PairExactPseudoinverse {
    /// Build an exact spectral pseudoinverse for a small pair domain.
    pub fn build(domain: PairDomain, options: PairExactOptions) -> Result<Self, MultiwayError> {
        let start = Instant::now();
        validate_rank_options(
            options.relative_rank_tolerance,
            options.maximum_dimension,
            domain.dimension(),
        )?;
        let decomposition = SymmetricEigen::new(domain.dense_gramian());
        let scale = decomposition
            .eigenvalues
            .iter()
            .copied()
            .map(f64::abs)
            .fold(0.0, f64::max);
        if !scale.is_finite() || scale <= 0.0 {
            return Err(MultiwayError::SpectralAnalysis {
                message: format!("pair Gramian spectral scale is {scale}"),
            });
        }
        let threshold = options.relative_rank_tolerance * scale;
        let mut inverse_eigenvalues = Vec::with_capacity(domain.dimension());
        let mut rank = 0usize;
        for &eigenvalue in decomposition.eigenvalues.iter() {
            if eigenvalue < -threshold {
                return Err(MultiwayError::NegativeEigenvalue {
                    value: eigenvalue,
                    tolerance: threshold,
                });
            }
            if eigenvalue > threshold {
                inverse_eigenvalues.push(1.0 / eigenvalue);
                rank += 1;
            } else {
                inverse_eigenvalues.push(0.0);
            }
        }
        let dimension = domain.dimension();
        let mut inverse = DMatrix::zeros(dimension, dimension);
        for row in 0..dimension {
            for column in 0..dimension {
                let mut value = 0.0;
                for mode in 0..dimension {
                    value = decomposition.eigenvectors[(row, mode)].mul_add(
                        inverse_eigenvalues[mode] * decomposition.eigenvectors[(column, mode)],
                        value,
                    );
                }
                inverse[(row, column)] = value;
            }
        }
        Ok(Self {
            domain,
            inverse,
            rank,
            threshold,
            build_duration: start.elapsed(),
        })
    }

    /// Numerical rank retained by the pseudoinverse.
    #[must_use]
    pub const fn rank(&self) -> usize {
        self.rank
    }

    /// Absolute eigenvalue threshold used by the pseudoinverse.
    #[must_use]
    pub const fn threshold(&self) -> f64 {
        self.threshold
    }

    /// Dense setup wall time.
    #[must_use]
    pub const fn build_duration(&self) -> Duration {
        self.build_duration
    }

    /// Principal retained bytes in the dense inverse and shared domain.
    #[must_use]
    pub fn retained_bytes_estimate(&self) -> usize {
        self.domain.retained_bytes_estimate().saturating_add(
            self.inverse
                .len()
                .saturating_mul(core::mem::size_of::<f64>()),
        )
    }
}

impl Preconditioner for PairExactPseudoinverse {
    fn dimension(&self) -> usize {
        self.domain.dimension()
    }

    fn apply(&self, rhs: &[f64], out: &mut [f64]) -> Result<(), MultiwayError> {
        validate_pair_vectors("PairExactPseudoinverse::apply", self.dimension(), rhs, out)?;
        // Apply the exact Euclidean pair-range projector on both sides.  The
        // right projection is evaluated without allocating a temporary vector;
        // this keeps the materialized full-space action symmetric even when the
        // spectral inverse is very ill-conditioned and its computed null action
        // is only approximately zero.
        let mut null_dot = 0.0;
        let mut correction = 0.0;
        for &value in &rhs[..self.domain.left_count()] {
            neumaier_add(&mut null_dot, &mut correction, value);
        }
        for &value in &rhs[self.domain.left_count()..] {
            neumaier_add(&mut null_dot, &mut correction, -value);
        }
        let shift = (null_dot + correction) / self.dimension() as f64;
        for row in 0..self.dimension() {
            let mut value = 0.0;
            for (column, &right) in rhs.iter().enumerate() {
                let projected = if column < self.domain.left_count() {
                    right - shift
                } else {
                    right + shift
                };
                value = self.inverse[(row, column)].mul_add(projected, value);
            }
            out[row] = value;
        }
        self.domain.project_range_in_place(out)
    }
}

/// Fixed-cycle CMG options for one pair-local domain.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PairLocalCmgOptions {
    /// Frozen CMG hierarchy options.
    pub cmg: CmgOptions,
    /// Fixed number of stationary residual-correction cycles.
    pub fixed_cycles: usize,
}

impl Default for PairLocalCmgOptions {
    fn default() -> Self {
        Self {
            cmg: CmgOptions::default(),
            fixed_cycles: 1,
        }
    }
}

/// Setup timing for one pair-local fixed CMG action.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PairLocalCmgBuildTiming {
    cmg_setup: Duration,
    workspace_setup: Duration,
    total: Duration,
}

impl PairLocalCmgBuildTiming {
    /// Immutable CMG hierarchy construction time.
    #[must_use]
    pub const fn cmg_setup(self) -> Duration {
        self.cmg_setup
    }

    /// Initial reusable workspace construction time.
    #[must_use]
    pub const fn workspace_setup(self) -> Duration {
        self.workspace_setup
    }

    /// Complete local-solver construction time.
    #[must_use]
    pub const fn total(self) -> Duration {
        self.total
    }
}

/// Retained and scratch memory for one pair-local CMG action.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PairLocalCmgMemoryReport {
    domain_retained_bytes_estimate: usize,
    cmg_preconditioner_bytes: usize,
    workspace_pool_bytes: usize,
    total_retained_bytes_estimate: usize,
}

impl PairLocalCmgMemoryReport {
    /// Canonical pair-domain retained bytes.
    #[must_use]
    pub const fn domain_retained_bytes_estimate(self) -> usize {
        self.domain_retained_bytes_estimate
    }

    /// Principal immutable bytes reported by CMG.
    #[must_use]
    pub const fn cmg_preconditioner_bytes(self) -> usize {
        self.cmg_preconditioner_bytes
    }

    /// One retained caller-owned CMG and residual workspace.
    #[must_use]
    pub const fn workspace_pool_bytes(self) -> usize {
        self.workspace_pool_bytes
    }

    /// Sum of known principal retained categories.
    #[must_use]
    pub const fn total_retained_bytes_estimate(self) -> usize {
        self.total_retained_bytes_estimate
    }
}

/// One fixed CMG pair-local approximate inverse with a reusable workspace pool.
pub struct PairLocalCmgPreconditioner {
    domain: PairDomain,
    inner: CmgPreconditioner,
    fixed_cycles: usize,
    workspace_pool: Mutex<Vec<PairLocalCmgWorkspace>>,
    fallback_workspace_allocations: AtomicUsize,
    build_timing: PairLocalCmgBuildTiming,
    memory_report: PairLocalCmgMemoryReport,
}

impl core::fmt::Debug for PairLocalCmgPreconditioner {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        formatter
            .debug_struct("PairLocalCmgPreconditioner")
            .field("fixed_cycles", &self.fixed_cycles)
            .field("build_timing", &self.build_timing)
            .field("memory_report", &self.memory_report)
            .finish_non_exhaustive()
    }
}

impl PairLocalCmgPreconditioner {
    /// Build a fixed CMG action over the submitted canonical pair graph.
    pub fn build(domain: PairDomain, options: PairLocalCmgOptions) -> Result<Self, MultiwayError> {
        let total_start = Instant::now();
        options
            .cmg
            .validate()
            .map_err(|error| MultiwayError::Cmg(error.to_string()))?;
        if options.fixed_cycles == 0 {
            return Err(MultiwayError::InvalidOption {
                name: "pair_local_cmg_fixed_cycles",
                message: "must be positive".to_owned(),
            });
        }
        let cmg_start = Instant::now();
        let inner = CmgPreconditioner::build(domain.graph(), options.cmg)
            .map_err(|error| MultiwayError::Cmg(error.to_string()))?;
        let cmg_setup = cmg_start.elapsed();
        let workspace_start = Instant::now();
        let workspace =
            PairLocalCmgWorkspace::new(&inner, domain.dimension(), options.fixed_cycles);
        let workspace_pool_bytes = workspace.byte_len();
        let workspace_setup = workspace_start.elapsed();
        let cmg_preconditioner_bytes = inner.retained_bytes();
        let domain_retained_bytes_estimate = domain.retained_bytes_estimate();
        let total_retained_bytes_estimate = domain_retained_bytes_estimate
            .saturating_add(cmg_preconditioner_bytes)
            .saturating_add(workspace_pool_bytes);
        Ok(Self {
            domain,
            inner,
            fixed_cycles: options.fixed_cycles,
            workspace_pool: Mutex::new(vec![workspace]),
            fallback_workspace_allocations: AtomicUsize::new(0),
            build_timing: PairLocalCmgBuildTiming {
                cmg_setup,
                workspace_setup,
                total: total_start.elapsed(),
            },
            memory_report: PairLocalCmgMemoryReport {
                domain_retained_bytes_estimate,
                cmg_preconditioner_bytes,
                workspace_pool_bytes,
                total_retained_bytes_estimate,
            },
        })
    }

    /// Number of retained hierarchy levels.
    #[must_use]
    pub fn hierarchy_levels(&self) -> usize {
        self.inner.hierarchy().levels().len()
    }

    /// Phase-separated construction timing.
    #[must_use]
    pub const fn build_timing(&self) -> PairLocalCmgBuildTiming {
        self.build_timing
    }

    /// Principal retained-state accounting.
    #[must_use]
    pub const fn memory_report(&self) -> PairLocalCmgMemoryReport {
        self.memory_report
    }

    /// Emergency workspace allocations after construction.
    #[must_use]
    pub fn fallback_workspace_allocations(&self) -> usize {
        self.fallback_workspace_allocations.load(Ordering::Relaxed)
    }

    fn take_workspace(&self) -> Result<PairLocalCmgWorkspace, MultiwayError> {
        let workspace = self
            .workspace_pool
            .lock()
            .map_err(|_| {
                MultiwayError::Cmg("pair-local workspace borrow lock poisoned".to_owned())
            })?
            .pop();
        match workspace {
            Some(workspace) => Ok(workspace),
            None => {
                self.fallback_workspace_allocations
                    .fetch_add(1, Ordering::Relaxed);
                Ok(PairLocalCmgWorkspace::try_new(
                    &self.inner,
                    self.domain.dimension(),
                    self.fixed_cycles,
                )?)
            }
        }
    }

    fn return_workspace(&self, workspace: PairLocalCmgWorkspace) -> Result<(), MultiwayError> {
        self.workspace_pool
            .lock()
            .map_err(|_| {
                MultiwayError::Cmg("pair-local workspace return lock poisoned".to_owned())
            })?
            .push(workspace);
        Ok(())
    }

    fn apply_fixed(
        &self,
        workspace: &mut PairLocalCmgWorkspace,
        out: &mut [f64],
    ) -> Result<(), MultiwayError> {
        let n = self.domain.dimension();
        out.fill(0.0);
        if self.fixed_cycles == 1 {
            self.inner
                .apply_compatible_into(&workspace.rhs, out, &mut workspace.cmg)
                .map_err(|error| MultiwayError::Cmg(error.to_string()))?;
            return Ok(());
        }
        workspace.original_rhs.copy_from_slice(&workspace.rhs);
        for cycle in 0..self.fixed_cycles {
            if cycle > 0 {
                self.domain
                    .graph()
                    .matvec_into(out, &mut workspace.correction)
                    .map_err(|error| MultiwayError::Cmg(error.to_string()))?;
                for index in 0..n {
                    workspace.rhs[index] =
                        workspace.original_rhs[index] - workspace.correction[index];
                }
            }
            workspace.correction.fill(0.0);
            self.inner
                .apply_compatible_into(
                    &workspace.rhs,
                    &mut workspace.correction,
                    &mut workspace.cmg,
                )
                .map_err(|error| MultiwayError::Cmg(error.to_string()))?;
            for (value, &correction) in out.iter_mut().zip(&workspace.correction) {
                *value += correction;
            }
        }
        Ok(())
    }
}

impl Preconditioner for PairLocalCmgPreconditioner {
    fn dimension(&self) -> usize {
        self.domain.dimension()
    }

    fn apply(&self, rhs: &[f64], out: &mut [f64]) -> Result<(), MultiwayError> {
        validate_pair_vectors(
            "PairLocalCmgPreconditioner::apply",
            self.dimension(),
            rhs,
            out,
        )?;
        out.fill(0.0);
        let mut workspace = self.take_workspace()?;
        workspace.rhs.copy_from_slice(rhs);
        self.domain.project_range_in_place(&mut workspace.rhs)?;
        for value in &mut workspace.rhs[self.domain.left_count..] {
            *value = -*value;
        }
        let result = self.apply_fixed(&mut workspace, out);
        if result.is_ok() {
            for value in &mut out[self.domain.left_count..] {
                *value = -*value;
            }
            self.domain.project_range_in_place(out)?;
            if let Err(error) = self.return_workspace(workspace) {
                out.fill(0.0);
                return Err(error);
            }
        } else {
            out.fill(0.0);
        }
        result
    }
}

struct PairLocalCmgWorkspace {
    rhs: Vec<f64>,
    original_rhs: Vec<f64>,
    correction: Vec<f64>,
    cmg: CmgWorkspace,
}

impl PairLocalCmgWorkspace {
    fn new(inner: &CmgPreconditioner, dimension: usize, cycles: usize) -> Self {
        Self {
            rhs: vec![0.0; dimension],
            original_rhs: if cycles > 1 {
                vec![0.0; dimension]
            } else {
                Vec::new()
            },
            correction: if cycles > 1 {
                vec![0.0; dimension]
            } else {
                Vec::new()
            },
            cmg: inner.workspace(),
        }
    }

    fn try_new(
        inner: &CmgPreconditioner,
        dimension: usize,
        cycles: usize,
    ) -> Result<Self, MultiwayError> {
        Ok(Self {
            rhs: try_zeroed(dimension, "pair-local CMG rhs")?,
            original_rhs: if cycles > 1 {
                try_zeroed(dimension, "pair-local CMG original rhs")?
            } else {
                Vec::new()
            },
            correction: if cycles > 1 {
                try_zeroed(dimension, "pair-local CMG correction")?
            } else {
                Vec::new()
            },
            cmg: inner
                .try_workspace()
                .map_err(|error| MultiwayError::Cmg(error.to_string()))?,
        })
    }

    fn byte_len(&self) -> usize {
        self.rhs
            .capacity()
            .saturating_add(self.original_rhs.capacity())
            .saturating_add(self.correction.capacity())
            .saturating_mul(core::mem::size_of::<f64>())
            .saturating_add(self.cmg.byte_len())
    }
}

/// Setup timing for the frozen pair-local `within` action.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PairLocalWithinBuildTiming {
    design_input_setup: Duration,
    within_solver_setup: Duration,
    within_preconditioner_setup: Duration,
    workspace_setup: Duration,
    total: Duration,
}

impl PairLocalWithinBuildTiming {
    /// Edge-to-observation input conversion time.
    #[must_use]
    pub const fn design_input_setup(self) -> Duration {
        self.design_input_setup
    }

    /// Complete public `within::Solver::new` time.
    #[must_use]
    pub const fn within_solver_setup(self) -> Duration {
        self.within_solver_setup
    }

    /// Subset reported by the retained public preconditioner.
    #[must_use]
    pub const fn within_preconditioner_setup(self) -> Duration {
        self.within_preconditioner_setup
    }

    /// Pair-range RHS workspace construction time.
    #[must_use]
    pub const fn workspace_setup(self) -> Duration {
        self.workspace_setup
    }

    /// Complete wrapper construction time.
    #[must_use]
    pub const fn total(self) -> Duration {
        self.total
    }
}

/// Known retained-state accounting for the pair-local `within` action.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PairLocalWithinMemoryReport {
    domain_retained_bytes_estimate: usize,
    range_workspace_bytes: usize,
    within_retained_bytes: Option<usize>,
}

impl PairLocalWithinMemoryReport {
    /// Canonical pair-domain retained bytes.
    #[must_use]
    pub const fn domain_retained_bytes_estimate(self) -> usize {
        self.domain_retained_bytes_estimate
    }

    /// Retained projected-RHS workspace bytes.
    #[must_use]
    pub const fn range_workspace_bytes(self) -> usize {
        self.range_workspace_bytes
    }

    /// Upstream retained bytes, unavailable at the pinned public revision.
    #[must_use]
    pub const fn within_retained_bytes(self) -> Option<usize> {
        self.within_retained_bytes
    }

    /// Sum of retained categories known to this wrapper.
    #[must_use]
    pub const fn known_retained_bytes_estimate(self) -> usize {
        self.domain_retained_bytes_estimate
            .saturating_add(self.range_workspace_bytes)
    }
}

/// Frozen `within` pair-local approximate-Cholesky action on one identical domain.
pub struct PairLocalWithinPreconditioner {
    domain: PairDomain,
    inner: within::Preconditioner,
    warnings: Vec<String>,
    rhs_pool: Mutex<Vec<Vec<f64>>>,
    fallback_workspace_allocations: AtomicUsize,
    build_timing: PairLocalWithinBuildTiming,
    memory_report: PairLocalWithinMemoryReport,
}

impl core::fmt::Debug for PairLocalWithinPreconditioner {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        formatter
            .debug_struct("PairLocalWithinPreconditioner")
            .field("warnings", &self.warnings)
            .field("build_timing", &self.build_timing)
            .field("memory_report", &self.memory_report)
            .finish_non_exhaustive()
    }
}

impl PairLocalWithinPreconditioner {
    /// Build the public production `within` preconditioner on the exact edge list.
    pub fn build(
        domain: PairDomain,
        options: WithinApproxCholOptions,
    ) -> Result<Self, MultiwayError> {
        let total_start = Instant::now();
        let input_start = Instant::now();
        let left_codes: Vec<u32> = domain.edges.iter().map(|edge| edge.left).collect();
        let right_codes: Vec<u32> = domain.edges.iter().map(|edge| edge.right).collect();
        let weights: Vec<f64> = domain.edges.iter().map(|edge| edge.weight).collect();
        let effects = vec![
            Effect::new(&left_codes, true, std::iter::empty::<&[f64]>())
                .map_err(|error| MultiwayError::Within(error.to_string()))?,
            Effect::new(&right_codes, true, std::iter::empty::<&[f64]>())
                .map_err(|error| MultiwayError::Within(error.to_string()))?,
        ];
        let design_input_setup = input_start.elapsed();

        let within_start = Instant::now();
        let solver = Solver::new(
            effects,
            Some(weights),
            PreconditionerConfig::Additive {
                local_solver: options.local_solver,
                reduction: options.reduction,
            },
        )
        .map_err(|error| MultiwayError::Within(error.to_string()))?;
        let within_solver_setup = within_start.elapsed();
        let inner = solver.preconditioner().cloned().ok_or_else(|| {
            MultiwayError::Within("within returned no pair preconditioner".to_owned())
        })?;
        if inner.nrows() != domain.dimension() || inner.ncols() != domain.dimension() {
            return Err(MultiwayError::Within(format!(
                "within pair preconditioner shape {}x{} does not match domain dimension {}",
                inner.nrows(),
                inner.ncols(),
                domain.dimension()
            )));
        }
        let within_preconditioner_setup = inner.build_duration();
        let warnings = solver.warnings().iter().map(ToString::to_string).collect();

        let workspace_start = Instant::now();
        let rhs = vec![0.0; domain.dimension()];
        let range_workspace_bytes = rhs.capacity().saturating_mul(core::mem::size_of::<f64>());
        let workspace_setup = workspace_start.elapsed();
        let memory_report = PairLocalWithinMemoryReport {
            domain_retained_bytes_estimate: domain.retained_bytes_estimate(),
            range_workspace_bytes,
            within_retained_bytes: None,
        };
        Ok(Self {
            domain,
            inner,
            warnings,
            rhs_pool: Mutex::new(vec![rhs]),
            fallback_workspace_allocations: AtomicUsize::new(0),
            build_timing: PairLocalWithinBuildTiming {
                design_input_setup,
                within_solver_setup,
                within_preconditioner_setup,
                workspace_setup,
                total: total_start.elapsed(),
            },
            memory_report,
        })
    }

    /// Non-fatal warnings emitted by the pinned comparator.
    #[must_use]
    pub fn warnings(&self) -> &[String] {
        &self.warnings
    }

    /// Phase-separated construction timing.
    #[must_use]
    pub const fn build_timing(&self) -> PairLocalWithinBuildTiming {
        self.build_timing
    }

    /// Retained-state accounting exposed by the wrapper.
    #[must_use]
    pub const fn memory_report(&self) -> PairLocalWithinMemoryReport {
        self.memory_report
    }

    /// Emergency projected-RHS allocations after construction.
    #[must_use]
    pub fn fallback_workspace_allocations(&self) -> usize {
        self.fallback_workspace_allocations.load(Ordering::Relaxed)
    }

    fn take_rhs(&self) -> Result<Vec<f64>, MultiwayError> {
        let rhs = self
            .rhs_pool
            .lock()
            .map_err(|_| MultiwayError::Within("pair RHS pool lock was poisoned".to_owned()))?
            .pop();
        Ok(rhs.unwrap_or_else(|| {
            self.fallback_workspace_allocations
                .fetch_add(1, Ordering::Relaxed);
            vec![0.0; self.domain.dimension()]
        }))
    }

    fn return_rhs(&self, rhs: Vec<f64>) -> Result<(), MultiwayError> {
        self.rhs_pool
            .lock()
            .map_err(|_| MultiwayError::Within("pair RHS return lock was poisoned".to_owned()))?
            .push(rhs);
        Ok(())
    }
}

impl Preconditioner for PairLocalWithinPreconditioner {
    fn dimension(&self) -> usize {
        self.domain.dimension()
    }

    fn apply(&self, rhs: &[f64], out: &mut [f64]) -> Result<(), MultiwayError> {
        validate_pair_vectors(
            "PairLocalWithinPreconditioner::apply",
            self.dimension(),
            rhs,
            out,
        )?;
        out.fill(0.0);
        let mut projected_rhs = self.take_rhs()?;
        projected_rhs.copy_from_slice(rhs);
        self.domain.project_range_in_place(&mut projected_rhs)?;
        if let Err(error) = self.inner.apply(&projected_rhs, out) {
            out.fill(0.0);
            return Err(MultiwayError::Within(error.to_string()));
        }
        if let Err(error) = self.domain.project_range_in_place(out) {
            out.fill(0.0);
            return Err(error);
        }
        if let Err(error) = self.return_rhs(projected_rhs) {
            out.fill(0.0);
            return Err(error);
        }
        Ok(())
    }
}

/// Dense pair-local diagnostic options.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PairLocalAnalysisOptions {
    /// Relative Gramian rank tolerance.
    pub relative_rank_tolerance: f64,
    /// Relative threshold for symmetry, linearity, and range defects.
    pub relative_structure_tolerance: f64,
    /// Maximum dimension admitted to dense materialization.
    pub maximum_dimension: usize,
}

impl Default for PairLocalAnalysisOptions {
    fn default() -> Self {
        Self {
            relative_rank_tolerance: 1.0e-11,
            relative_structure_tolerance: 1.0e-10,
            maximum_dimension: 512,
        }
    }
}

/// Dense numerical-range quality report for one fixed pair-local action.
#[derive(Debug, Clone, PartialEq)]
pub struct PairLocalAnalysisReport {
    dimension: usize,
    numerical_rank: usize,
    numerical_nullity: usize,
    gramian_condition_number: f64,
    linearity_defect: f64,
    full_symmetry_defect: f64,
    quotient_symmetry_defect: f64,
    range_leakage: f64,
    relative_inverse_frobenius_error: f64,
    minimum_action_eigenvalue: f64,
    maximum_action_eigenvalue: f64,
    positive_action_defect: f64,
    minimum_preconditioned_eigenvalue: f64,
    maximum_preconditioned_eigenvalue: f64,
    preconditioned_condition_number: f64,
    unit_inverse_energy_error: f64,
    numerically_linear: bool,
    numerically_symmetric: bool,
    preserves_range: bool,
    positive_on_range: bool,
    preconditioned_eigenvalues: Vec<f64>,
}

impl PairLocalAnalysisReport {
    /// Pair coefficient dimension.
    #[must_use]
    pub const fn dimension(&self) -> usize {
        self.dimension
    }

    /// Numerical Gramian rank.
    #[must_use]
    pub const fn numerical_rank(&self) -> usize {
        self.numerical_rank
    }

    /// Numerical Gramian nullity.
    #[must_use]
    pub const fn numerical_nullity(&self) -> usize {
        self.numerical_nullity
    }

    /// Unpreconditioned Gramian condition number on its numerical range.
    #[must_use]
    pub const fn gramian_condition_number(&self) -> f64 {
        self.gramian_condition_number
    }

    /// Scale-normalized superposition defect from independent repeated applies.
    #[must_use]
    pub const fn linearity_defect(&self) -> f64 {
        self.linearity_defect
    }

    /// Relative Frobenius defect of the materialized full action.
    #[must_use]
    pub const fn full_symmetry_defect(&self) -> f64 {
        self.full_symmetry_defect
    }

    /// Relative Frobenius symmetry defect after range restriction.
    #[must_use]
    pub const fn quotient_symmetry_defect(&self) -> f64 {
        self.quotient_symmetry_defect
    }

    /// Relative action norm leaking from the range into the null space.
    #[must_use]
    pub const fn range_leakage(&self) -> f64 {
        self.range_leakage
    }

    /// Relative Frobenius distance from the exact range pseudoinverse.
    #[must_use]
    pub const fn relative_inverse_frobenius_error(&self) -> f64 {
        self.relative_inverse_frobenius_error
    }

    /// Smallest eigenvalue of the symmetric action on the range.
    #[must_use]
    pub const fn minimum_action_eigenvalue(&self) -> f64 {
        self.minimum_action_eigenvalue
    }

    /// Largest eigenvalue of the symmetric action on the range.
    #[must_use]
    pub const fn maximum_action_eigenvalue(&self) -> f64 {
        self.maximum_action_eigenvalue
    }

    /// Scale-normalized magnitude of any materially negative action.
    #[must_use]
    pub const fn positive_action_defect(&self) -> f64 {
        self.positive_action_defect
    }

    /// Smallest eigenvalue of `G^(1/2) M G^(1/2)` on the range.
    #[must_use]
    pub const fn minimum_preconditioned_eigenvalue(&self) -> f64 {
        self.minimum_preconditioned_eigenvalue
    }

    /// Largest eigenvalue of `G^(1/2) M G^(1/2)` on the range.
    #[must_use]
    pub const fn maximum_preconditioned_eigenvalue(&self) -> f64 {
        self.maximum_preconditioned_eigenvalue
    }

    /// Condition number of the energy-preconditioned pair operator.
    #[must_use]
    pub const fn preconditioned_condition_number(&self) -> f64 {
        self.preconditioned_condition_number
    }

    /// Maximum absolute deviation of preconditioned eigenvalues from one.
    #[must_use]
    pub const fn unit_inverse_energy_error(&self) -> f64 {
        self.unit_inverse_energy_error
    }

    /// Whether the explicit fixed-action linearity gate passes.
    #[must_use]
    pub const fn numerically_linear(&self) -> bool {
        self.numerically_linear
    }

    /// Whether full and quotient symmetry gates pass.
    #[must_use]
    pub const fn numerically_symmetric(&self) -> bool {
        self.numerically_symmetric
    }

    /// Whether the action preserves the numerical range.
    #[must_use]
    pub const fn preserves_range(&self) -> bool {
        self.preserves_range
    }

    /// Whether the symmetric range action is positive definite.
    #[must_use]
    pub const fn positive_on_range(&self) -> bool {
        self.positive_on_range
    }

    /// Energy-preconditioned eigenvalues in ascending order.
    #[must_use]
    pub fn preconditioned_eigenvalues(&self) -> &[f64] {
        &self.preconditioned_eigenvalues
    }
}

/// Materialize and analyze one fixed local action on an identical pair domain.
pub fn analyze_pair_local<P: Preconditioner + ?Sized>(
    domain: &PairDomain,
    preconditioner: &P,
    options: PairLocalAnalysisOptions,
) -> Result<PairLocalAnalysisReport, MultiwayError> {
    validate_rank_options(
        options.relative_rank_tolerance,
        options.maximum_dimension,
        domain.dimension(),
    )?;
    if !options.relative_structure_tolerance.is_finite()
        || options.relative_structure_tolerance <= 0.0
    {
        return Err(MultiwayError::InvalidOption {
            name: "pair_local_relative_structure_tolerance",
            message: format!(
                "must be finite and positive, got {}",
                options.relative_structure_tolerance
            ),
        });
    }
    if preconditioner.dimension() != domain.dimension() {
        return Err(crate::error::dimension(
            "analyze_pair_local preconditioner",
            domain.dimension(),
            preconditioner.dimension(),
        ));
    }

    let gramian_decomposition = SymmetricEigen::new(domain.dense_gramian());
    let gramian_scale = gramian_decomposition
        .eigenvalues
        .iter()
        .copied()
        .map(f64::abs)
        .fold(0.0, f64::max);
    if !gramian_scale.is_finite() || gramian_scale <= 0.0 {
        return Err(MultiwayError::SpectralAnalysis {
            message: format!("pair Gramian spectral scale is {gramian_scale}"),
        });
    }
    let threshold = options.relative_rank_tolerance * gramian_scale;
    let mut positive_modes = Vec::new();
    for (mode, &eigenvalue) in gramian_decomposition.eigenvalues.iter().enumerate() {
        if eigenvalue < -threshold {
            return Err(MultiwayError::NegativeEigenvalue {
                value: eigenvalue,
                tolerance: threshold,
            });
        }
        if eigenvalue > threshold {
            positive_modes.push((eigenvalue, mode));
        }
    }
    positive_modes.sort_by(|left, right| left.0.total_cmp(&right.0));
    if positive_modes.is_empty() {
        return Err(MultiwayError::SpectralAnalysis {
            message: "pair Gramian has no positive numerical range".to_owned(),
        });
    }
    let rank = positive_modes.len();
    let mut basis = DMatrix::zeros(domain.dimension(), rank);
    let mut eigenvalues = Vec::with_capacity(rank);
    for (column, &(eigenvalue, source)) in positive_modes.iter().enumerate() {
        eigenvalues.push(eigenvalue);
        for row in 0..domain.dimension() {
            basis[(row, column)] = gramian_decomposition.eigenvectors[(row, source)];
        }
    }

    let action = materialize_pair_action(preconditioner)?;
    let full_symmetry_defect = symmetry_defect(&action);
    let applied_range = &action * &basis;
    let quotient = basis.transpose() * &applied_range;
    let quotient_symmetry_defect = symmetry_defect(&quotient);
    let quotient_symmetric = (&quotient + quotient.transpose()) * 0.5;
    let range_leakage = structural_range_leakage(domain, &applied_range);

    let mut exact_inverse = DMatrix::zeros(rank, rank);
    for index in 0..rank {
        exact_inverse[(index, index)] = 1.0 / eigenvalues[index];
    }
    let inverse_difference = &quotient_symmetric - exact_inverse.clone();
    let relative_inverse_frobenius_error =
        frobenius_norm(&inverse_difference) / frobenius_norm(&exact_inverse);

    let action_decomposition = SymmetricEigen::new(quotient_symmetric.clone());
    let mut action_eigenvalues: Vec<f64> =
        action_decomposition.eigenvalues.iter().copied().collect();
    action_eigenvalues.sort_by(f64::total_cmp);
    let minimum_action_eigenvalue = action_eigenvalues[0];
    let maximum_action_eigenvalue = action_eigenvalues[action_eigenvalues.len() - 1];
    let action_scale = minimum_action_eigenvalue
        .abs()
        .max(maximum_action_eigenvalue.abs())
        .max(f64::MIN_POSITIVE);
    let positive_action_defect = (-minimum_action_eigenvalue).max(0.0) / action_scale;

    let mut energy_preconditioned = DMatrix::zeros(rank, rank);
    for row in 0..rank {
        for column in 0..rank {
            energy_preconditioned[(row, column)] = eigenvalues[row].sqrt()
                * quotient_symmetric[(row, column)]
                * eigenvalues[column].sqrt();
        }
    }
    let energy_decomposition = SymmetricEigen::new(energy_preconditioned);
    let mut preconditioned_eigenvalues: Vec<f64> =
        energy_decomposition.eigenvalues.iter().copied().collect();
    preconditioned_eigenvalues.sort_by(f64::total_cmp);
    let minimum_preconditioned_eigenvalue = preconditioned_eigenvalues[0];
    let maximum_preconditioned_eigenvalue =
        preconditioned_eigenvalues[preconditioned_eigenvalues.len() - 1];
    let preconditioned_condition_number = if minimum_preconditioned_eigenvalue > 0.0 {
        maximum_preconditioned_eigenvalue / minimum_preconditioned_eigenvalue
    } else {
        f64::INFINITY
    };
    let unit_inverse_energy_error = preconditioned_eigenvalues
        .iter()
        .map(|&value| (1.0 - value).abs())
        .fold(0.0, f64::max);
    let linearity_defect = linearity_defect(domain, preconditioner)?;
    let structure_tolerance = options.relative_structure_tolerance;
    let positive_tolerance = options.relative_rank_tolerance * action_scale;

    Ok(PairLocalAnalysisReport {
        dimension: domain.dimension(),
        numerical_rank: rank,
        numerical_nullity: domain.dimension() - rank,
        gramian_condition_number: eigenvalues[rank - 1] / eigenvalues[0],
        linearity_defect,
        full_symmetry_defect,
        quotient_symmetry_defect,
        range_leakage,
        relative_inverse_frobenius_error,
        minimum_action_eigenvalue,
        maximum_action_eigenvalue,
        positive_action_defect,
        minimum_preconditioned_eigenvalue,
        maximum_preconditioned_eigenvalue,
        preconditioned_condition_number,
        unit_inverse_energy_error,
        numerically_linear: linearity_defect <= structure_tolerance,
        numerically_symmetric: full_symmetry_defect <= structure_tolerance
            && quotient_symmetry_defect <= structure_tolerance,
        preserves_range: range_leakage <= structure_tolerance,
        positive_on_range: minimum_action_eigenvalue > positive_tolerance,
        preconditioned_eigenvalues,
    })
}

fn structural_range_leakage(domain: &PairDomain, applied_range: &DMatrix<f64>) -> f64 {
    let mut squared_null_action = 0.0;
    for column in 0..applied_range.ncols() {
        let mut sum = 0.0;
        let mut correction = 0.0;
        for row in 0..domain.left_count() {
            neumaier_add(&mut sum, &mut correction, applied_range[(row, column)]);
        }
        for row in domain.left_count()..domain.dimension() {
            neumaier_add(&mut sum, &mut correction, -applied_range[(row, column)]);
        }
        let null_action = sum + correction;
        squared_null_action = null_action.mul_add(null_action, squared_null_action);
    }
    squared_null_action.sqrt()
        / ((domain.dimension() as f64).sqrt()
            * frobenius_norm(applied_range).max(f64::MIN_POSITIVE))
}

fn materialize_pair_action<P: Preconditioner + ?Sized>(
    preconditioner: &P,
) -> Result<DMatrix<f64>, MultiwayError> {
    let dimension = preconditioner.dimension();
    let mut matrix = DMatrix::zeros(dimension, dimension);
    let mut basis_vector = vec![0.0; dimension];
    let mut output = vec![0.0; dimension];
    for column in 0..dimension {
        basis_vector.fill(0.0);
        basis_vector[column] = 1.0;
        preconditioner.apply(&basis_vector, &mut output)?;
        for row in 0..dimension {
            matrix[(row, column)] = output[row];
        }
    }
    Ok(matrix)
}

fn linearity_defect<P: Preconditioner + ?Sized>(
    domain: &PairDomain,
    preconditioner: &P,
) -> Result<f64, MultiwayError> {
    let mut x: Vec<f64> = (0..domain.dimension())
        .map(|index| ((index + 1) as f64 * 0.37).sin())
        .collect();
    let mut y: Vec<f64> = (0..domain.dimension())
        .map(|index| ((index + 1) as f64 * 0.61).cos())
        .collect();
    domain.project_range_in_place(&mut x)?;
    domain.project_range_in_place(&mut y)?;
    let a: f64 = -1.7;
    let b: f64 = 0.45;
    let combination: Vec<f64> = x
        .iter()
        .zip(&y)
        .map(|(&left, &right)| a.mul_add(left, b * right))
        .collect();
    let mut mx = vec![0.0; domain.dimension()];
    let mut my = vec![0.0; domain.dimension()];
    let mut mcombination = vec![0.0; domain.dimension()];
    preconditioner.apply(&x, &mut mx)?;
    preconditioner.apply(&y, &mut my)?;
    preconditioner.apply(&combination, &mut mcombination)?;
    let difference: Vec<f64> = mcombination
        .iter()
        .zip(mx.iter().zip(&my))
        .map(|(&actual, (&left, &right))| actual - a.mul_add(left, b * right))
        .collect();
    let scale = euclidean_norm(&mcombination)
        .max(euclidean_norm(&mx))
        .max(euclidean_norm(&my))
        .max(f64::MIN_POSITIVE);
    Ok(euclidean_norm(&difference) / scale)
}

fn symmetry_defect(matrix: &DMatrix<f64>) -> f64 {
    let difference = matrix - matrix.transpose();
    frobenius_norm(&difference) / frobenius_norm(matrix).max(f64::MIN_POSITIVE)
}

fn frobenius_norm(matrix: &DMatrix<f64>) -> f64 {
    matrix.iter().map(|value| value * value).sum::<f64>().sqrt()
}

fn euclidean_norm(values: &[f64]) -> f64 {
    let scale = values.iter().copied().map(f64::abs).fold(0.0, f64::max);
    if scale == 0.0 {
        return 0.0;
    }
    scale
        * values
            .iter()
            .map(|value| (value / scale) * (value / scale))
            .sum::<f64>()
            .sqrt()
}

fn validate_rank_options(
    relative_tolerance: f64,
    maximum_dimension: usize,
    dimension: usize,
) -> Result<(), MultiwayError> {
    if !relative_tolerance.is_finite() || relative_tolerance <= 0.0 {
        return Err(MultiwayError::InvalidOption {
            name: "pair_local_relative_rank_tolerance",
            message: format!("must be finite and positive, got {relative_tolerance}"),
        });
    }
    if maximum_dimension == 0 {
        return Err(MultiwayError::InvalidOption {
            name: "pair_local_maximum_dimension",
            message: "must be positive".to_owned(),
        });
    }
    if dimension > maximum_dimension {
        return Err(MultiwayError::SpectralAnalysis {
            message: format!(
                "pair dimension {dimension} exceeds dense-analysis limit {maximum_dimension}"
            ),
        });
    }
    Ok(())
}

fn validate_pair_vectors(
    context: &'static str,
    dimension: usize,
    input: &[f64],
    output: &[f64],
) -> Result<(), MultiwayError> {
    if input.len() != dimension {
        return Err(crate::error::dimension(context, dimension, input.len()));
    }
    if output.len() != dimension {
        return Err(crate::error::dimension(context, dimension, output.len()));
    }
    Ok(())
}

fn pair_domain_error(message: String) -> MultiwayError {
    MultiwayError::InvalidOption {
        name: "pair_domain",
        message,
    }
}

fn try_zeroed(length: usize, context: &'static str) -> Result<Vec<f64>, MultiwayError> {
    let mut values = Vec::new();
    values
        .try_reserve_exact(length)
        .map_err(|_| MultiwayError::Cmg(format!("allocation failed for {context}")))?;
    values.resize(length, 0.0);
    Ok(values)
}

fn neumaier_add(sum: &mut f64, correction: &mut f64, value: f64) {
    let updated = *sum + value;
    if sum.abs() >= value.abs() {
        *correction += (*sum - updated) + value;
    } else {
        *correction += (value - updated) + *sum;
    }
    *sum = updated;
}
