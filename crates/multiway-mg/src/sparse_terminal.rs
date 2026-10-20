//! Fixed generation-bound sparse terminal for a coarse level above the dense cap.
//!
//! The terminal is `P W P`: `W` is one serial application of `within`'s additive
//! pair Schwarz preconditioner, built for exactly this level's current tuples and
//! weights, and `P` is the certified orthogonal projector onto the complement of
//! the level's exact incidence kernel. `W` is symmetric positive semidefinite, so
//! `P W P` is symmetric, maps into the exact range and annihilates the exact kernel.
//! It has no RHS-dependent inner stopping. Positivity on the range is a property
//! of `W` that callers screen through the actual recursive cycle.
//!
//! Every construction array is charged against one declared limit before it is
//! allocated: the level-code columns, `within`'s checked serial setup, the exact
//! kernel certificate and its projector. Nothing survives a refused build.
use crate::{MultiwayError, certificate::ensure_finite};
use multiway_incidence::{
    IncidenceKernelCertificate, IncidenceKernelLimits, IncidenceRangeProjector,
    PreparedThreeWayTopology, ThreeWayWeightFrame,
};
use std::borrow::Cow;
use within::{
    BuildError, Design, LocalSolverConfig, ReductionStrategy, SerialPreconditionerWorkspace,
    SetupLimit,
};

/// Fixed upper bound on the heap bytes of the borrowed three-factor `within`
/// observation frame and design descriptors (column handles, term metadata and
/// loadings). Their size does not depend on the tuple count; an isolated
/// allocator test checks the bound.
pub const SPARSE_TERMINAL_DESIGN_DESCRIPTOR_BOUND: usize = 1024;

/// Explicit sparse-terminal construction policy; nothing is selected implicitly.
#[derive(Debug, Clone, PartialEq)]
pub struct PreparedSparseTerminalOptions {
    /// Inclusive coefficient-dimension cap for an admitted sparse terminal.
    pub maximum_dimension: usize,
    /// `within` local block-elimination and approximate-Cholesky settings.
    pub local_solver: LocalSolverConfig,
    /// Largest admissible orthogonality defect or relative incidence residual of
    /// the floating kernel basis built from the exact certificate.
    pub projector_tolerance: f64,
}

/// Construction accounting of one admitted sparse terminal.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct PreparedSparseTerminalReport {
    /// Coefficient dimension of the terminal level.
    pub dimension: usize,
    /// Unique tuples of the terminal level.
    pub tuples: usize,
    /// Exact incidence nullity of the terminal level.
    pub nullity: usize,
    /// Exact nullity beyond the two structural directions per component.
    pub additional_nullity: usize,
    /// Peak requested payload of the kernel certificate and projector build.
    pub kernel_peak_payload_bytes: usize,
    /// Retained projector payload, including its consumed certificate.
    pub projector_retained_payload_bytes: usize,
    /// Level-code columns plus the fixed design-descriptor bound.
    pub design_payload_bytes: usize,
    /// Peak payload of `within`'s checked serial setup.
    pub within_setup_peak_payload_bytes: usize,
    /// Retained payload of the `within` preconditioner.
    pub within_retained_payload_bytes: usize,
    /// Checked peak of the whole construction, an upper bound on live payload.
    pub peak_payload_bytes: usize,
    /// Retained payload of the terminal: projector plus `within` preconditioner.
    pub retained_payload_bytes: usize,
}

/// A fixed `P W P` action for one exact current numerical frame.
pub struct PreparedSparseTerminal<'state> {
    frame: &'state ThreeWayWeightFrame<'state>,
    preconditioner: within::Preconditioner,
    projector: IncidenceRangeProjector<'state>,
    report: PreparedSparseTerminalReport,
}

impl core::fmt::Debug for PreparedSparseTerminal<'_> {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        formatter
            .debug_struct("PreparedSparseTerminal")
            .field("report", &self.report)
            .finish_non_exhaustive()
    }
}

/// Caller-owned serial scratch for one sparse terminal.
pub struct SparseTerminalWorkspace<'owner> {
    serial: SerialPreconditionerWorkspace<'owner>,
    projected: Vec<f64>,
}

impl core::fmt::Debug for SparseTerminalWorkspace<'_> {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        formatter
            .debug_struct("SparseTerminalWorkspace")
            .field("dimension", &self.projected.len())
            .finish_non_exhaustive()
    }
}

impl<'state> PreparedSparseTerminal<'state> {
    /// Build the terminal for `frame` with live construction payload at most
    /// `maximum_payload_bytes`, refusing before any allocation that would exceed it.
    ///
    /// The exact kernel certificate and projector are built first, then `within`'s
    /// checked serial setup runs against the remaining limit while the borrowed
    /// level-code columns are live. A dimension above the options' cap, a refused
    /// limit or an uncertified kernel returns an error with nothing retained.
    pub fn try_new(
        frame: &'state ThreeWayWeightFrame<'state>,
        options: &PreparedSparseTerminalOptions,
        maximum_payload_bytes: usize,
    ) -> Result<Self, MultiwayError> {
        validate(options)?;
        let topology: &'state PreparedThreeWayTopology = frame.topology();
        let dimension = topology.topology().total_levels();
        if dimension > options.maximum_dimension {
            return Err(MultiwayError::InvalidOption {
                name: "sparse_terminal_dimension",
                message: format!(
                    "terminal dimension {dimension} exceeds the declared cap {}",
                    options.maximum_dimension
                ),
            });
        }
        let limits = IncidenceKernelLimits {
            maximum_payload_bytes,
        };
        let certificate = IncidenceKernelCertificate::try_certify(topology, limits)?;
        let kernel_report = certificate.report();
        let projector =
            certificate.try_into_projector(topology, limits, options.projector_tolerance)?;
        let projector_report = projector.report();
        let projector_retained = projector.retained_payload_bytes()?;
        let kernel_peak = kernel_report
            .peak_requested_payload_bytes
            .max(projector_report.peak_requested_payload_bytes);

        let tuples = topology.topology().tuples();
        let design_payload = tuples
            .len()
            .checked_mul(3 * core::mem::size_of::<u32>())
            .and_then(|columns| columns.checked_add(SPARSE_TERMINAL_DESIGN_DESCRIPTOR_BOUND))
            .ok_or_else(overflow)?;
        let before_within = projector_retained
            .checked_add(design_payload)
            .ok_or_else(overflow)?;
        if before_within > maximum_payload_bytes {
            return Err(refused(
                "sparse terminal design columns",
                before_within,
                maximum_payload_bytes,
            ));
        }
        let mut levels: [Vec<u32>; 3] = [Vec::new(), Vec::new(), Vec::new()];
        for column in &mut levels {
            column.try_reserve_exact(tuples.len()).map_err(|source| {
                MultiwayError::WorkspaceAllocation {
                    context: "sparse terminal design columns",
                    source,
                }
            })?;
        }
        for tuple in tuples {
            for (column, &level) in levels.iter_mut().zip(tuple) {
                column.push(level);
            }
        }
        let (preconditioner, setup) = {
            let frame_columns: Vec<Cow<'_, [u32]>> =
                levels.iter().map(|c| Cow::Borrowed(c.as_slice())).collect();
            let observations =
                within::observation::ObservationFrame::new(frame_columns, Vec::new())
                    .map_err(within_error)?;
            let design = Design::from_frame_unsorted(observations).map_err(within_error)?;
            if design.n_dofs() != dimension {
                return Err(MultiwayError::Within(format!(
                    "sparse terminal design has {} coefficients, level has {dimension}",
                    design.n_dofs()
                )));
            }
            let limit = SetupLimit {
                maximum_payload_bytes: maximum_payload_bytes - before_within,
            };
            match within::Preconditioner::build_checked(
                &design,
                Some(frame.weights()),
                &options.local_solver,
                ReductionStrategy::Auto,
                limit,
            ) {
                Ok((Some(preconditioner), report)) => (preconditioner, report),
                Ok((None, _)) => {
                    return Err(MultiwayError::Within(
                        "sparse terminal level has no pair subdomain".to_owned(),
                    ));
                }
                Err(BuildError::SetupBudgetExceeded {
                    context, required, ..
                }) => {
                    return Err(MultiwayError::WithinSetupBudgetExceeded {
                        context,
                        required: before_within.saturating_add(required),
                        limit: maximum_payload_bytes,
                    });
                }
                Err(error) => return Err(within_error(error)),
            }
        };
        drop(levels);
        if preconditioner.nrows() != dimension || preconditioner.ncols() != dimension {
            return Err(MultiwayError::Within(format!(
                "sparse terminal preconditioner shape {}x{} does not match level dimension {dimension}",
                preconditioner.nrows(),
                preconditioner.ncols()
            )));
        }
        let within_peak = before_within
            .checked_add(setup.peak_payload_bytes)
            .ok_or_else(overflow)?;
        let retained = projector_retained
            .checked_add(setup.retained_payload_bytes)
            .ok_or_else(overflow)?;
        let certificate = projector.certificate();
        let report = PreparedSparseTerminalReport {
            dimension,
            tuples: tuples.len(),
            nullity: certificate.nullity(),
            additional_nullity: certificate.additional_nullity(),
            kernel_peak_payload_bytes: kernel_peak,
            projector_retained_payload_bytes: projector_retained,
            design_payload_bytes: design_payload,
            within_setup_peak_payload_bytes: setup.peak_payload_bytes,
            within_retained_payload_bytes: setup.retained_payload_bytes,
            peak_payload_bytes: kernel_peak.max(within_peak),
            retained_payload_bytes: retained,
        };
        Ok(Self {
            frame,
            preconditioner,
            projector,
            report,
        })
    }

    /// Construction accounting.
    #[must_use]
    pub const fn report(&self) -> PreparedSparseTerminalReport {
        self.report
    }

    /// Coefficient dimension.
    #[must_use]
    pub const fn dimension(&self) -> usize {
        self.report.dimension
    }

    /// Exact rank of the terminal level's incidence.
    #[must_use]
    pub const fn rank(&self) -> usize {
        self.report.dimension - self.report.nullity
    }

    /// Borrowed `within` preconditioner, for diagnostics.
    #[must_use]
    pub const fn preconditioner(&self) -> &within::Preconditioner {
        &self.preconditioner
    }

    /// Retained payload: projector, consumed certificate and `within` preconditioner.
    pub fn retained_payload_bytes(&self) -> Result<usize, MultiwayError> {
        Ok(self.report.retained_payload_bytes)
    }

    /// Requested bytes of one application workspace.
    pub fn workspace_required_bytes(&self) -> Result<usize, MultiwayError> {
        let serial = self
            .preconditioner
            .serial_workspace_required_payload_bytes()
            .map_err(|error| MultiwayError::Within(error.to_string()))?;
        self.dimension()
            .checked_mul(core::mem::size_of::<f64>())
            .and_then(|projected| projected.checked_add(serial))
            .ok_or_else(overflow)
    }

    /// Reserve the serial `within` scratch and one projected-input vector.
    pub fn application_workspace(&self) -> Result<SparseTerminalWorkspace<'_>, MultiwayError> {
        let serial_bytes = self
            .preconditioner
            .serial_workspace_required_payload_bytes()
            .map_err(|error| MultiwayError::Within(error.to_string()))?;
        let serial = self
            .preconditioner
            .try_serial_workspace(serial_bytes)
            .map_err(|error| MultiwayError::Within(error.to_string()))?;
        let mut projected = Vec::new();
        projected
            .try_reserve_exact(self.dimension())
            .map_err(|source| MultiwayError::WorkspaceAllocation {
                context: "sparse terminal projected input",
                source,
            })?;
        projected.resize(self.dimension(), 0.0);
        Ok(SparseTerminalWorkspace { serial, projected })
    }

    /// Whether this terminal was built for exactly `frame`.
    pub fn validate_for(&self, frame: &ThreeWayWeightFrame<'_>) -> Result<(), MultiwayError> {
        if !core::ptr::eq(
            core::ptr::from_ref(self.frame).cast::<u8>(),
            core::ptr::from_ref(frame).cast::<u8>(),
        ) {
            return Err(MultiwayError::WorkspaceNotPrepared {
                context: "sparse terminal numerical frame",
            });
        }
        Ok(())
    }

    /// Apply `out = P W P rhs` without allocation; output is transactional.
    pub fn apply_with_workspace(
        &self,
        rhs: &[f64],
        out: &mut [f64],
        workspace: &mut SparseTerminalWorkspace<'_>,
    ) -> Result<(), MultiwayError> {
        let n = self.dimension();
        if rhs.len() != n {
            return Err(crate::error::dimension("sparse terminal rhs", n, rhs.len()));
        }
        if out.len() != n {
            return Err(crate::error::dimension(
                "sparse terminal output",
                n,
                out.len(),
            ));
        }
        if workspace.projected.len() != n || workspace.serial.dimension() != n {
            return Err(MultiwayError::WorkspaceNotPrepared {
                context: "sparse terminal application",
            });
        }
        workspace.projected.copy_from_slice(rhs);
        self.projector.project(&mut workspace.projected)?;
        workspace
            .serial
            .apply(&workspace.projected, out)
            .map_err(|error| MultiwayError::Within(error.to_string()))?;
        self.projector.project(out)?;
        ensure_finite(out, "sparse terminal output")
    }
}

impl SparseTerminalWorkspace<'_> {
    /// Retained scratch capacity: the serial `within` array and the projected input.
    pub fn retained_payload_bytes(&self) -> Result<usize, MultiwayError> {
        self.projected
            .capacity()
            .checked_mul(core::mem::size_of::<f64>())
            .and_then(|projected| projected.checked_add(self.serial.retained_payload_bytes()))
            .ok_or_else(overflow)
    }
}

fn validate(options: &PreparedSparseTerminalOptions) -> Result<(), MultiwayError> {
    if !options.projector_tolerance.is_finite() || options.projector_tolerance <= 0.0 {
        return Err(MultiwayError::InvalidOption {
            name: "sparse_terminal_projector_tolerance",
            message: "must be finite and positive".to_owned(),
        });
    }
    if options.maximum_dimension == 0 {
        return Err(MultiwayError::InvalidOption {
            name: "sparse_terminal_maximum_dimension",
            message: "must be positive".to_owned(),
        });
    }
    Ok(())
}

fn within_error(error: BuildError) -> MultiwayError {
    MultiwayError::Within(error.to_string())
}

fn refused(context: &'static str, required: usize, limit: usize) -> MultiwayError {
    MultiwayError::WithinSetupBudgetExceeded {
        context,
        required,
        limit,
    }
}

fn overflow() -> MultiwayError {
    MultiwayError::WorkspaceSizeOverflow {
        context: "sparse terminal payload",
    }
}
