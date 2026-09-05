//! Incidence-component metadata and structural-kernel projection.

use std::sync::Arc;

use crate::{IncidenceError, ThreeWayTopology};

#[derive(Debug, Clone, Copy, Default, PartialEq)]
struct StructuralProjectionScratch {
    sums: [f64; 3],
    corrections: [f64; 3],
    projection: [f64; 3],
}

/// Reusable scratch for structural-range projection and defect evaluation.
///
/// Construct this workspace from [`IncidenceComponents::projection_workspace`]
/// and reuse it only with that component decomposition or one of its ordinary
/// clones. Independently constructed decompositions are rejected, even if their
/// dimensions and component counts match. The private identity token contains
/// no numerical state. The workspace owns all mutable scratch needed by the
/// allocation-free projection and defect methods.
#[derive(Debug, Clone)]
pub struct StructuralProjectionWorkspace {
    dimension: usize,
    scratch: Vec<StructuralProjectionScratch>,
    binding: Arc<()>,
}

impl PartialEq for StructuralProjectionWorkspace {
    fn eq(&self, other: &Self) -> bool {
        self.dimension == other.dimension
            && Arc::ptr_eq(&self.binding, &other.binding)
            && self.scratch == other.scratch
    }
}

impl StructuralProjectionWorkspace {
    /// Coefficient dimension for which this workspace was prepared.
    #[must_use]
    pub const fn dimension(&self) -> usize {
        self.dimension
    }

    /// Number of incidence components for which this workspace was prepared.
    #[must_use]
    pub fn component_count(&self) -> usize {
        self.scratch.len()
    }

    /// Retained heap bytes in the exclusively owned component scratch array.
    ///
    /// This capacity-based payload count excludes the inline workspace object,
    /// allocator metadata, and the shared identity token's reference-counting
    /// metadata. Cloning the binding does not allocate another identity token.
    #[must_use]
    pub fn retained_bytes(&self) -> usize {
        self.scratch.capacity() * core::mem::size_of::<StructuralProjectionScratch>()
    }

    fn clear(&mut self) {
        self.scratch.fill(StructuralProjectionScratch::default());
    }
}

/// Connected components of the level--tuple incidence graph.
///
/// Every component carries the two structural shift directions
/// `(1, -1, 0)` and `(1, 0, -1)`. Extra rank deficiencies may exist and are
/// deliberately left to rank-revealing solvers and external certification.
/// Value equality compares component metadata, not workspace compatibility.
/// Ordinary clones share their private workspace binding; independent builds
/// do not, even when their component metadata is value-equal.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IncidenceComponents {
    labels: Vec<usize>,
    factor_sizes: Vec<[usize; 3]>,
    offsets: [usize; 4],
    // Arc<()> value equality preserves the existing metadata equality contract.
    // Workspace compatibility is deliberately checked with Arc::ptr_eq instead.
    binding: Arc<()>,
}

impl IncidenceComponents {
    /// Construct deterministic component labels in first-global-vertex order.
    #[must_use]
    pub fn from_topology(topology: &ThreeWayTopology) -> Self {
        let n = topology.total_levels();
        let mut parent: Vec<usize> = (0..n).collect();
        for tuple in topology.tuples() {
            let a = topology.global_index(0, tuple[0]);
            let b = topology.global_index(1, tuple[1]);
            let c = topology.global_index(2, tuple[2]);
            union_min_root(&mut parent, a, b);
            union_min_root(&mut parent, a, c);
        }
        for vertex in 0..n {
            parent[vertex] = find_root(&mut parent, vertex);
        }

        let mut root_to_label = vec![usize::MAX; n];
        let mut labels = vec![0; n];
        let mut factor_sizes: Vec<[usize; 3]> = Vec::new();
        let offsets = topology.offsets();
        for vertex in 0..n {
            let root = parent[vertex];
            let label = if root_to_label[root] == usize::MAX {
                let next = factor_sizes.len();
                root_to_label[root] = next;
                factor_sizes.push([0; 3]);
                next
            } else {
                root_to_label[root]
            };
            labels[vertex] = label;
            let factor = if vertex < offsets[1] {
                0
            } else if vertex < offsets[2] {
                1
            } else {
                2
            };
            factor_sizes[label][factor] += 1;
        }

        Self {
            labels,
            factor_sizes,
            offsets,
            binding: Arc::new(()),
        }
    }

    /// Number of connected incidence components.
    #[must_use]
    pub fn count(&self) -> usize {
        self.factor_sizes.len()
    }

    /// Component label for every global coefficient coordinate.
    #[must_use]
    pub fn labels(&self) -> &[usize] {
        &self.labels
    }

    /// Counts of factor-1, factor-2, and factor-3 levels in each component.
    #[must_use]
    pub fn factor_sizes(&self) -> &[[usize; 3]] {
        &self.factor_sizes
    }

    /// Component label for one factor-local level.
    #[must_use]
    pub fn component_of(&self, factor: usize, level: usize) -> usize {
        assert!(factor < 3, "factor index must be below three");
        self.labels[self.offsets[factor] + level]
    }

    /// Allocate reusable scratch bound to this component decomposition.
    ///
    /// Ordinary clones of this decomposition share the binding. Independently
    /// constructed decompositions require their own projection workspaces.
    #[must_use]
    pub fn projection_workspace(&self) -> StructuralProjectionWorkspace {
        StructuralProjectionWorkspace {
            dimension: self.labels.len(),
            scratch: vec![StructuralProjectionScratch::default(); self.count()],
            binding: Arc::clone(&self.binding),
        }
    }

    /// Orthogonally remove the two known factor-shift directions per component.
    ///
    /// The returned value is the Euclidean norm of the removed projection. This
    /// convenience method allocates one temporary workspace; repeated callers
    /// should use [`Self::project_structural_range_with_workspace`].
    pub fn project_structural_range(&self, values: &mut [f64]) -> Result<f64, IncidenceError> {
        let mut workspace = self.projection_workspace();
        self.project_structural_range_with_workspace(values, &mut workspace)
    }

    /// Orthogonally remove structural shift directions without allocating.
    ///
    /// `workspace` must have been prepared by this component decomposition or
    /// one of its ordinary clones. Dimensions and exact private binding are
    /// checked before either `values` or workspace scratch is modified.
    pub fn project_structural_range_with_workspace(
        &self,
        values: &mut [f64],
        workspace: &mut StructuralProjectionWorkspace,
    ) -> Result<f64, IncidenceError> {
        self.validate_values(
            "IncidenceComponents::project_structural_range_with_workspace values",
            values,
        )?;
        self.validate_workspace(
            "IncidenceComponents::project_structural_range_with_workspace",
            workspace,
        )?;
        self.accumulate_factor_sums(values, workspace);

        let mut removed_squared = 0.0;
        for (component, scratch) in workspace.scratch.iter_mut().enumerate() {
            let [n1, n2, n3] = self.factor_sizes[component];
            debug_assert!(n1 > 0 && n2 > 0 && n3 > 0);
            let [s1, s2, s3] = scratch.sums;
            let g1 = s1 - s2;
            let g2 = s1 - s3;
            let a11 = (n1 + n2) as f64;
            let a12 = n1 as f64;
            let a22 = (n1 + n3) as f64;
            let determinant = a11.mul_add(a22, -(a12 * a12));
            debug_assert!(determinant > 0.0);
            let alpha = (a22.mul_add(g1, -(a12 * g2))) / determinant;
            let beta = (a11.mul_add(g2, -(a12 * g1))) / determinant;
            let projection = [alpha + beta, -alpha, -beta];
            scratch.projection = projection;
            removed_squared += (n1 as f64).mul_add(
                projection[0] * projection[0],
                (n2 as f64).mul_add(
                    projection[1] * projection[1],
                    n3 as f64 * projection[2] * projection[2],
                ),
            );
        }

        for factor in 0..3 {
            for vertex in self.offsets[factor]..self.offsets[factor + 1] {
                values[vertex] -= workspace.scratch[self.labels[vertex]].projection[factor];
            }
        }
        Ok(removed_squared.sqrt())
    }

    /// Maximum absolute dot product with either known structural kernel vector.
    ///
    /// This convenience method allocates one temporary workspace; repeated
    /// callers should use [`Self::maximum_structural_defect_with_workspace`].
    pub fn maximum_structural_defect(&self, values: &[f64]) -> Result<f64, IncidenceError> {
        let mut workspace = self.projection_workspace();
        self.maximum_structural_defect_with_workspace(values, &mut workspace)
    }

    /// Evaluate the maximum structural defect without allocating.
    ///
    /// `workspace` must have been prepared by this component decomposition or
    /// one of its ordinary clones. Validation precedes scratch mutation.
    pub fn maximum_structural_defect_with_workspace(
        &self,
        values: &[f64],
        workspace: &mut StructuralProjectionWorkspace,
    ) -> Result<f64, IncidenceError> {
        self.validate_values(
            "IncidenceComponents::maximum_structural_defect_with_workspace values",
            values,
        )?;
        self.validate_workspace(
            "IncidenceComponents::maximum_structural_defect_with_workspace",
            workspace,
        )?;
        self.accumulate_factor_sums(values, workspace);

        let mut maximum: f64 = 0.0;
        for scratch in &workspace.scratch {
            let [a, b, c] = scratch.sums;
            maximum = maximum.max((a - b).abs()).max((a - c).abs());
        }
        Ok(maximum)
    }

    fn validate_values(&self, context: &'static str, values: &[f64]) -> Result<(), IncidenceError> {
        if values.len() != self.labels.len() {
            return Err(crate::error::dimension(
                context,
                self.labels.len(),
                values.len(),
            ));
        }
        Ok(())
    }

    fn validate_workspace(
        &self,
        context: &'static str,
        workspace: &StructuralProjectionWorkspace,
    ) -> Result<(), IncidenceError> {
        if workspace.dimension != self.labels.len() {
            return Err(crate::error::dimension(
                context,
                self.labels.len(),
                workspace.dimension,
            ));
        }
        if workspace.scratch.len() != self.count() {
            return Err(crate::error::dimension(
                context,
                self.count(),
                workspace.scratch.len(),
            ));
        }
        if !Arc::ptr_eq(&self.binding, &workspace.binding) {
            return Err(IncidenceError::WorkspaceBindingMismatch { context });
        }
        Ok(())
    }

    fn accumulate_factor_sums(
        &self,
        values: &[f64],
        workspace: &mut StructuralProjectionWorkspace,
    ) {
        workspace.clear();
        for factor in 0..3 {
            for vertex in self.offsets[factor]..self.offsets[factor + 1] {
                let component = self.labels[vertex];
                let StructuralProjectionScratch {
                    sums, corrections, ..
                } = &mut workspace.scratch[component];
                neumaier_add(&mut sums[factor], &mut corrections[factor], values[vertex]);
            }
        }
        for scratch in &mut workspace.scratch {
            for factor in 0..3 {
                scratch.sums[factor] += scratch.corrections[factor];
            }
        }
    }
}

fn find_root(parent: &mut [usize], mut vertex: usize) -> usize {
    while parent[vertex] != vertex {
        parent[vertex] = parent[parent[vertex]];
        vertex = parent[vertex];
    }
    vertex
}

fn union_min_root(parent: &mut [usize], left: usize, right: usize) {
    let left_root = find_root(parent, left);
    let right_root = find_root(parent, right);
    if left_root == right_root {
        return;
    }
    let (small, large) = if left_root < right_root {
        (left_root, right_root)
    } else {
        (right_root, left_root)
    };
    parent[large] = small;
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
