//! Reusable allocation-free projection onto the three-way structural range.

use std::sync::{
    Mutex,
    atomic::{AtomicUsize, Ordering},
};

use crate::{MultiwayError, ThreeWayProblem};

/// Pool-backed projector for the factor-shift complement of a fixed problem.
#[derive(Debug)]
pub(crate) struct StructuralRangeProjector {
    component_count: usize,
    workspace_bytes: usize,
    pool: Mutex<Vec<StructuralProjectionWorkspace>>,
    fallback_allocations: AtomicUsize,
}

impl StructuralRangeProjector {
    pub(crate) fn new(problem: &ThreeWayProblem) -> Self {
        let workspace = StructuralProjectionWorkspace::new(problem.components().count());
        let workspace_bytes = workspace.byte_len();
        Self {
            component_count: problem.components().count(),
            workspace_bytes,
            pool: Mutex::new(vec![workspace]),
            fallback_allocations: AtomicUsize::new(0),
        }
    }

    pub(crate) const fn workspace_bytes(&self) -> usize {
        self.workspace_bytes
    }

    pub(crate) fn fallback_allocations(&self) -> usize {
        self.fallback_allocations.load(Ordering::Relaxed)
    }

    pub(crate) fn project(
        &self,
        problem: &ThreeWayProblem,
        values: &mut [f64],
    ) -> Result<(), MultiwayError> {
        if problem.components().count() != self.component_count {
            return Err(MultiwayError::Lsmr(
                "structural projector does not match the submitted problem".to_owned(),
            ));
        }
        let mut workspace = {
            let mut pool = self.pool.lock().map_err(|_| {
                MultiwayError::Lsmr("structural projection workspace lock was poisoned".to_owned())
            })?;
            pool.pop()
        }
        .unwrap_or_else(|| {
            self.fallback_allocations.fetch_add(1, Ordering::Relaxed);
            StructuralProjectionWorkspace::new(self.component_count)
        });

        let result = project_structural_range_in_place(problem, values, &mut workspace);
        if result.is_ok() {
            self.pool
                .lock()
                .map_err(|_| {
                    MultiwayError::Lsmr(
                        "structural projection workspace return lock was poisoned".to_owned(),
                    )
                })?
                .push(workspace);
        }
        result
    }
}

#[derive(Debug)]
struct StructuralProjectionWorkspace {
    sums: Vec<[f64; 3]>,
    corrections: Vec<[f64; 3]>,
    projections: Vec<[f64; 3]>,
}

impl StructuralProjectionWorkspace {
    fn new(component_count: usize) -> Self {
        Self {
            sums: vec![[0.0; 3]; component_count],
            corrections: vec![[0.0; 3]; component_count],
            projections: vec![[0.0; 3]; component_count],
        }
    }

    fn byte_len(&self) -> usize {
        self.sums
            .len()
            .saturating_add(self.corrections.len())
            .saturating_add(self.projections.len())
            .saturating_mul(core::mem::size_of::<[f64; 3]>())
    }
}

fn project_structural_range_in_place(
    problem: &ThreeWayProblem,
    values: &mut [f64],
    workspace: &mut StructuralProjectionWorkspace,
) -> Result<(), MultiwayError> {
    if values.len() != problem.dimension() {
        return Err(crate::error::dimension(
            "three-way structural projection",
            problem.dimension(),
            values.len(),
        ));
    }
    let components = problem.components();
    if workspace.sums.len() != components.count()
        || workspace.corrections.len() != components.count()
        || workspace.projections.len() != components.count()
    {
        return Err(MultiwayError::Lsmr(
            "structural projection workspace has the wrong component count".to_owned(),
        ));
    }
    workspace.sums.fill([0.0; 3]);
    workspace.corrections.fill([0.0; 3]);
    workspace.projections.fill([0.0; 3]);
    let offsets = problem.topology().offsets();
    for factor in 0..3 {
        for vertex in offsets[factor]..offsets[factor + 1] {
            let component = components.labels()[vertex];
            neumaier_add(
                &mut workspace.sums[component][factor],
                &mut workspace.corrections[component][factor],
                values[vertex],
            );
        }
    }
    for component in 0..components.count() {
        for factor in 0..3 {
            workspace.sums[component][factor] += workspace.corrections[component][factor];
        }
        let [n1, n2, n3] = components.factor_sizes()[component];
        let [s1, s2, s3] = workspace.sums[component];
        let g1 = s1 - s2;
        let g2 = s1 - s3;
        let a11 = (n1 + n2) as f64;
        let a12 = n1 as f64;
        let a22 = (n1 + n3) as f64;
        let determinant = a11.mul_add(a22, -(a12 * a12));
        if !(determinant.is_finite() && determinant > 0.0) {
            return Err(MultiwayError::Lsmr(format!(
                "invalid structural projection determinant {determinant} in component {component}"
            )));
        }
        let alpha = a22.mul_add(g1, -(a12 * g2)) / determinant;
        let beta = a11.mul_add(g2, -(a12 * g1)) / determinant;
        workspace.projections[component] = [alpha + beta, -alpha, -beta];
    }
    for factor in 0..3 {
        for vertex in offsets[factor]..offsets[factor + 1] {
            values[vertex] -= workspace.projections[components.labels()[vertex]][factor];
        }
    }
    Ok(())
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
