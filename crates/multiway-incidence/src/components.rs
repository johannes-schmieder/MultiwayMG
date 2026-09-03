//! Incidence-component metadata and structural-kernel projection.

use crate::{IncidenceError, ThreeWayTopology};

/// Connected components of the level--tuple incidence graph.
///
/// Every component carries the two structural shift directions
/// `(1, -1, 0)` and `(1, 0, -1)`. Extra rank deficiencies may exist and are
/// deliberately left to rank-revealing solvers and external certification.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IncidenceComponents {
    labels: Vec<usize>,
    factor_sizes: Vec<[usize; 3]>,
    offsets: [usize; 4],
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

    /// Orthogonally remove the two known factor-shift directions per component.
    ///
    /// The returned value is the Euclidean norm of the removed projection.
    pub fn project_structural_range(&self, values: &mut [f64]) -> Result<f64, IncidenceError> {
        if values.len() != self.labels.len() {
            return Err(crate::error::dimension(
                "IncidenceComponents::project_structural_range",
                self.labels.len(),
                values.len(),
            ));
        }

        let mut sums = vec![[0.0; 3]; self.count()];
        let mut corrections = vec![[0.0; 3]; self.count()];
        for factor in 0..3 {
            for vertex in self.offsets[factor]..self.offsets[factor + 1] {
                let component = self.labels[vertex];
                neumaier_add(
                    &mut sums[component][factor],
                    &mut corrections[component][factor],
                    values[vertex],
                );
            }
        }
        for component in 0..self.count() {
            for factor in 0..3 {
                sums[component][factor] += corrections[component][factor];
            }
        }

        let mut projections = vec![[0.0; 3]; self.count()];
        let mut removed_squared = 0.0;
        for component in 0..self.count() {
            let [n1, n2, n3] = self.factor_sizes[component];
            debug_assert!(n1 > 0 && n2 > 0 && n3 > 0);
            let [s1, s2, s3] = sums[component];
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
            projections[component] = projection;
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
                values[vertex] -= projections[self.labels[vertex]][factor];
            }
        }
        Ok(removed_squared.sqrt())
    }

    /// Maximum absolute dot product with either known structural kernel vector.
    pub fn maximum_structural_defect(&self, values: &[f64]) -> Result<f64, IncidenceError> {
        if values.len() != self.labels.len() {
            return Err(crate::error::dimension(
                "IncidenceComponents::maximum_structural_defect",
                self.labels.len(),
                values.len(),
            ));
        }
        let mut sums = vec![[0.0; 3]; self.count()];
        for factor in 0..3 {
            for vertex in self.offsets[factor]..self.offsets[factor + 1] {
                sums[self.labels[vertex]][factor] += values[vertex];
            }
        }
        Ok(sums
            .into_iter()
            .flat_map(|[a, b, c]| [(a - b).abs(), (a - c).abs()])
            .fold(0.0, f64::max))
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
