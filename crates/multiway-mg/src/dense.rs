//! Rank-revealing dense terminal pseudoinverse.

use nalgebra::{DMatrix, linalg::SymmetricEigen};

use crate::{MultiwayError, Preconditioner, ThreeWayProblem};

mod workspace;
pub use workspace::DensePseudoinverseWorkspace;

/// Dense spectral pseudoinverse used by small hierarchy terminals and references.
#[derive(Debug, Clone)]
pub struct DensePseudoinverse {
    eigenvectors: DMatrix<f64>,
    inverse_eigenvalues: Vec<f64>,
    rank: usize,
    threshold: f64,
}

impl DensePseudoinverse {
    /// Build a pseudoinverse with a relative eigenvalue threshold.
    pub fn from_problem(
        problem: &ThreeWayProblem,
        relative_tolerance: f64,
    ) -> Result<Self, MultiwayError> {
        if !relative_tolerance.is_finite() || relative_tolerance <= 0.0 {
            return Err(MultiwayError::InvalidOption {
                name: "terminal_relative_tolerance",
                message: format!("must be finite and positive, got {relative_tolerance}"),
            });
        }
        let dense = problem.dense_gramian();
        let dimension = problem.dimension();
        let flat: Vec<f64> = dense.into_iter().flatten().collect();
        let matrix = DMatrix::from_row_slice(dimension, dimension, &flat);
        Self::from_matrix(matrix, relative_tolerance)
    }

    pub(crate) fn from_frame(
        frame: &multiway_incidence::ThreeWayWeightFrame<'_>,
        relative_tolerance: f64,
    ) -> Result<Self, MultiwayError> {
        if !relative_tolerance.is_finite() || relative_tolerance <= 0.0 {
            return Err(MultiwayError::InvalidOption {
                name: "terminal_relative_tolerance",
                message: format!("must be finite and positive, got {relative_tolerance}"),
            });
        }
        let topology = frame.topology().topology();
        let n = topology.total_levels();
        let mut values = Self::matrix_values(n)?;
        // Native column-major assembly follows the ordinary canonical tuple and
        // row/column accumulation order, without row-vector/flattened copies.
        for (&tuple, &weight) in topology.tuples().iter().zip(frame.weights()) {
            let indices = [
                topology.global_index(0, tuple[0]),
                topology.global_index(1, tuple[1]),
                topology.global_index(2, tuple[2]),
            ];
            accumulate_tuple(&mut values, n, indices, weight);
        }
        Self::from_matrix(DMatrix::from_vec(n, n, values), relative_tolerance)
    }

    // Both native frame and component assembly use the same admitted matrix
    // allocation and canonical tuple/row/column accumulation arithmetic.
    fn matrix_values(n: usize) -> Result<Vec<f64>, MultiwayError> {
        let length = n
            .checked_mul(n)
            .filter(|&n| n <= isize::MAX as usize / 8)
            .ok_or(MultiwayError::WorkspaceSizeOverflow {
                context: "prepared dense terminal matrix",
            })?;
        let mut values = Vec::new();
        values
            .try_reserve_exact(length)
            .map_err(|source| MultiwayError::WorkspaceAllocation {
                context: "prepared dense terminal matrix",
                source,
            })?;
        values.resize(length, 0.0);
        Ok(values)
    }

    #[cfg(feature = "lsmr")]
    pub(crate) fn from_component(
        frame: &multiway_incidence::ThreeWayWeightFrame<'_>,
        recoding: &multiway_incidence::PreparedComponentRecoding<'_, '_>,
        component: usize,
        relative_tolerance: f64,
    ) -> Result<Self, MultiwayError> {
        frame.validate_for(recoding.layout().topology())?;
        let view = recoding.layout().component(component).ok_or(
            multiway_incidence::IncidenceError::ComponentIndexOutOfBounds {
                component,
                count: recoding.layout().component_count(),
            },
        )?;
        let n = view.dimension();
        if n > crate::PREPARED_DENSE_TERMINAL_LIMIT {
            return Err(MultiwayError::HierarchyStagnated {
                dimension: n,
                tuples: view.tuple_count(),
                limit: crate::PREPARED_DENSE_TERMINAL_LIMIT,
            });
        }
        if !relative_tolerance.is_finite() || relative_tolerance <= 0.0 {
            return Err(MultiwayError::InvalidOption {
                name: "terminal_relative_tolerance",
                message: format!("must be finite and positive, got {relative_tolerance}"),
            });
        }
        let counts = view.level_counts();
        let offsets = [0, counts[0], counts[0] + counts[1]];
        let mut values = Self::matrix_values(n)?;
        recoding.for_each_key(component, |id, key| {
            let indices = core::array::from_fn(|q| offsets[q] + key[q] as usize);
            accumulate_tuple(&mut values, n, indices, frame.weights()[id]);
        })?;
        Self::from_matrix(DMatrix::from_vec(n, n, values), relative_tolerance)
    }

    // Conservative simultaneous requested f64 arrays for pinned nalgebra 0.33.3:
    // original matrix, Q matrix, diagonal and two off-diagonal vectors. Its
    // temporary p dies before Q is assembled. Inline/stack/allocator excess are
    // excluded; internal nalgebra allocations are not fallible Rust reservations.
    // Callers enforce the separate hard 256-coordinate cap before assembly.
    #[cfg(feature = "lsmr")]
    pub(crate) fn factorization_payload_bound(n: usize) -> Result<usize, MultiwayError> {
        let overflow = || MultiwayError::WorkspaceSizeOverflow {
            context: "dense terminal factorization",
        };
        if n == 0 {
            return Err(overflow());
        }
        n.checked_mul(n)
            .and_then(|square| square.checked_mul(2))
            .and_then(|matrices| n.checked_mul(3).and_then(|v| matrices.checked_add(v)))
            .and_then(|values| values.checked_sub(2))
            .and_then(|values| values.checked_mul(8))
            .filter(|&bytes| bytes <= isize::MAX as usize)
            .ok_or_else(overflow)
    }

    fn from_matrix(matrix: DMatrix<f64>, relative_tolerance: f64) -> Result<Self, MultiwayError> {
        let dimension = matrix.nrows();
        if !matrix.iter().all(|value| value.is_finite()) {
            return Err(MultiwayError::NumericalFailure {
                context: "dense terminal matrix",
            });
        }
        // Bound failure on numerically unrepresentable inputs. This is a setup
        // guard, not an RHS-dependent inner solver or a change in rank policy.
        let decomposition = SymmetricEigen::try_new(matrix, f64::EPSILON, 10_000).ok_or(
            MultiwayError::NumericalFailure {
                context: "dense terminal eigendecomposition",
            },
        )?;
        if !decomposition
            .eigenvalues
            .iter()
            .all(|value| value.is_finite())
            || !decomposition
                .eigenvectors
                .iter()
                .all(|value| value.is_finite())
        {
            return Err(MultiwayError::NumericalFailure {
                context: "dense terminal eigendecomposition",
            });
        }
        let spectral_scale = decomposition
            .eigenvalues
            .iter()
            .copied()
            .map(f64::abs)
            .fold(0.0, f64::max);
        let threshold = relative_tolerance * spectral_scale;
        if !spectral_scale.is_finite()
            || spectral_scale <= 0.0
            || !threshold.is_finite()
            || threshold <= 0.0
        {
            return Err(MultiwayError::NumericalFailure {
                context: "dense terminal spectral threshold",
            });
        }
        let mut inverse_eigenvalues = Vec::with_capacity(dimension);
        let mut rank = 0;
        for &value in decomposition.eigenvalues.iter() {
            if value < -threshold {
                return Err(MultiwayError::NegativeEigenvalue {
                    value,
                    tolerance: threshold,
                });
            }
            if value > threshold {
                let inverse = 1.0 / value;
                if !inverse.is_finite() || inverse <= 0.0 {
                    return Err(MultiwayError::NumericalFailure {
                        context: "dense terminal inverse eigenvalue",
                    });
                }
                inverse_eigenvalues.push(inverse);
                rank += 1;
            } else {
                inverse_eigenvalues.push(0.0);
            }
        }
        Ok(Self {
            eigenvectors: decomposition.eigenvectors,
            inverse_eigenvalues,
            rank,
            threshold,
        })
    }

    /// Numerical rank retained by the pseudoinverse.
    #[must_use]
    pub const fn rank(&self) -> usize {
        self.rank
    }

    /// Absolute eigenvalue threshold used by the factorization.
    #[must_use]
    pub const fn threshold(&self) -> f64 {
        self.threshold
    }

    /// Matrix dimension.
    #[must_use]
    pub fn dimension(&self) -> usize {
        self.inverse_eigenvalues.len()
    }

    /// Apply the spectral pseudoinverse.
    pub fn solve_into(&self, rhs: &[f64], out: &mut [f64]) -> Result<(), MultiwayError> {
        if rhs.len() != self.dimension() {
            return Err(crate::error::dimension(
                "DensePseudoinverse::solve_into rhs",
                self.dimension(),
                rhs.len(),
            ));
        }
        if out.len() != self.dimension() {
            return Err(crate::error::dimension(
                "DensePseudoinverse::solve_into output",
                self.dimension(),
                out.len(),
            ));
        }
        let mut workspace = self.application_workspace()?;
        self.solve_into_with_workspace(rhs, out, &mut workspace)
    }

    /// Apply the spectral pseudoinverse without allocating prepared scratch.
    ///
    /// All vector lengths are validated before output or scratch is changed.
    pub fn solve_into_with_workspace(
        &self,
        rhs: &[f64],
        out: &mut [f64],
        workspace: &mut DensePseudoinverseWorkspace,
    ) -> Result<(), MultiwayError> {
        self.solve_with_modal(rhs, out, &mut workspace.modal)
    }

    // Anonymous exact-length modal scratch also supports the one-shot driver's
    // bounded stack arrays, with identical arithmetic and public validation.
    pub(crate) fn solve_with_modal(
        &self,
        rhs: &[f64],
        out: &mut [f64],
        modal: &mut [f64],
    ) -> Result<(), MultiwayError> {
        #[cfg(feature = "profiling")]
        let _profile_span = multiway_incidence::profiling::span(
            multiway_incidence::profiling::Phase::DenseTerminal,
        );

        let dimension = self.dimension();
        if rhs.len() != dimension {
            return Err(crate::error::dimension(
                "DensePseudoinverse::solve_into rhs",
                dimension,
                rhs.len(),
            ));
        }
        if out.len() != dimension {
            return Err(crate::error::dimension(
                "DensePseudoinverse::solve_into output",
                dimension,
                out.len(),
            ));
        }
        if modal.len() != dimension {
            return Err(crate::error::dimension(
                "DensePseudoinverseWorkspace",
                dimension,
                modal.len(),
            ));
        }
        for (mode, modal_value) in modal.iter_mut().enumerate() {
            let mut sum = 0.0;
            for (row, &right) in rhs.iter().enumerate() {
                sum = self.eigenvectors[(row, mode)].mul_add(right, sum);
            }
            *modal_value = sum * self.inverse_eigenvalues[mode];
        }
        // Q is column-major. Stream each column and update independent output
        // entries, retaining each entry's ascending-mode FMA order exactly.
        out.fill(0.0);
        for (mode, &modal_value) in modal.iter().enumerate() {
            let column = self.eigenvectors.column(mode);
            for (value, &coefficient) in out.iter_mut().zip(column.iter()) {
                *value = coefficient.mul_add(modal_value, *value);
            }
        }
        Ok(())
    }
}

impl Preconditioner for DensePseudoinverse {
    fn dimension(&self) -> usize {
        self.inverse_eigenvalues.len()
    }

    fn apply(&self, rhs: &[f64], out: &mut [f64]) -> Result<(), MultiwayError> {
        self.solve_into(rhs, out)
    }
}

impl DensePseudoinverse {
    /// Exclusive retained matrix and inverse-eigenvalue payload, by capacity.
    ///
    /// Excludes the inline descriptor, temporary factorization storage and
    /// allocator overhead. This does not estimate factorization peak memory.
    pub fn retained_payload_bytes(&self) -> Result<usize, MultiwayError> {
        self.eigenvectors
            .data
            .as_vec()
            .capacity()
            .checked_add(self.inverse_eigenvalues.capacity())
            .and_then(|values| values.checked_mul(core::mem::size_of::<f64>()))
            .ok_or(MultiwayError::WorkspaceSizeOverflow {
                context: "dense terminal payload",
            })
    }

    pub(crate) fn workspace_is_prepared(&self, workspace: &DensePseudoinverseWorkspace) -> bool {
        workspace.modal.len() == self.dimension()
    }
}

#[inline]
fn accumulate_tuple(values: &mut [f64], n: usize, indices: [usize; 3], weight: f64) {
    for &row in &indices {
        for &column in &indices {
            values[column * n + row] += weight;
        }
    }
}

#[cfg(test)]
mod traversal_tests;
