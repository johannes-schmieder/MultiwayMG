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
        let result = self.project_with_workspace(problem, values);
        if result.is_err() {
            values.fill(0.0);
        }
        result
    }

    fn project_with_workspace(
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
        // Every array is reset on the next use, including after an input error.
        // Return the workspace without masking the original numerical failure.
        match self.pool.lock() {
            Ok(mut pool) => pool.push(workspace),
            Err(_) if result.is_ok() => {
                return Err(MultiwayError::Lsmr(
                    "structural projection workspace return lock was poisoned".to_owned(),
                ));
            }
            Err(_) => {}
        }
        result
    }
}

#[derive(Debug)]
struct StructuralProjectionWorkspace {
    sums: Vec<[f64; 3]>,
    corrections: Vec<[f64; 3]>,
    projections: Vec<[f64; 3]>,
    scales: Vec<f64>,
}

impl StructuralProjectionWorkspace {
    fn new(component_count: usize) -> Self {
        Self {
            sums: vec![[0.0; 3]; component_count],
            corrections: vec![[0.0; 3]; component_count],
            projections: vec![[0.0; 3]; component_count],
            scales: vec![0.0; component_count],
        }
    }

    fn byte_len(&self) -> usize {
        self.sums
            .len()
            .saturating_add(self.corrections.len())
            .saturating_add(self.projections.len())
            .saturating_mul(core::mem::size_of::<[f64; 3]>())
            .saturating_add(self.scales.len().saturating_mul(core::mem::size_of::<f64>()))
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
        || workspace.scales.len() != components.count()
    {
        return Err(MultiwayError::Lsmr(
            "structural projection workspace has the wrong component count".to_owned(),
        ));
    }
    workspace.sums.fill([0.0; 3]);
    workspace.corrections.fill([0.0; 3]);
    workspace.projections.fill([0.0; 3]);
    workspace.scales.fill(0.0);
    for (vertex, value) in values.iter().enumerate() {
        if !value.is_finite() {
            return Err(MultiwayError::Lsmr(
                "structural projection received a nonfinite value".to_owned(),
            ));
        }
        let scale = &mut workspace.scales[components.labels()[vertex]];
        *scale = scale.max(value.abs());
    }
    let offsets = problem.topology().offsets();
    for factor in 0..3 {
        for vertex in offsets[factor]..offsets[factor + 1] {
            let component = components.labels()[vertex];
            let scale = workspace.scales[component];
            if scale > 0.0 {
                neumaier_add(
                    &mut workspace.sums[component][factor],
                    &mut workspace.corrections[component][factor],
                    values[vertex] / scale,
                );
            }
        }
    }
    for component in 0..components.count() {
        let sizes = components.factor_sizes()[component];
        if sizes.contains(&0) {
            return Err(MultiwayError::Lsmr(format!(
                "structural projection component {component} does not cover every factor"
            )));
        }
        let mut means = [0.0; 3];
        let mut inverse_sizes = [0.0; 3];
        for factor in 0..3 {
            let sum = workspace.sums[component][factor] + workspace.corrections[component][factor];
            inverse_sizes[factor] = 1.0 / sizes[factor] as f64;
            means[factor] = sum * inverse_sizes[factor];
        }
        // Projected factor sums are equal to t. The corrections are constant
        // within factors and sum to zero: t = sum(s_f/n_f) / sum(1/n_f).
        // This avoids both overflowing raw sums and subtractive determinants.
        let t = means.iter().sum::<f64>() / inverse_sizes.iter().sum::<f64>();
        for factor in 0..3 {
            workspace.projections[component][factor] = means[factor] - t * inverse_sizes[factor];
        }
    }
    for factor in 0..3 {
        for vertex in offsets[factor]..offsets[factor + 1] {
            let component = components.labels()[vertex];
            let scale = workspace.scales[component];
            if scale > 0.0 {
                values[vertex] =
                    (values[vertex] / scale - workspace.projections[component][factor]) * scale;
                if !values[vertex].is_finite() {
                    return Err(MultiwayError::Lsmr(
                        "structural projection output is not representable as finite f64"
                            .to_owned(),
                    ));
                }
            }
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

#[cfg(test)]
mod tests {
    use super::*;

    fn disconnected() -> ThreeWayProblem {
        ThreeWayProblem::from_observations([2, 2, 2], &[[0, 0, 0], [1, 1, 1]], &[1.0, 1.0]).unwrap()
    }

    #[test]
    fn scales_are_component_local_and_overflowing_intermediates_are_avoided() {
        let problem = disconnected();
        let projector = StructuralRangeProjector::new(&problem);
        let mut values = vec![1e308, -1e-308, 1e308, 1e-308, -1e308, 3e-308];
        projector.project(&problem, &mut values).unwrap();
        for (i, value) in values.iter().enumerate() {
            let expected = if i % 2 == 0 { 1e308 / 3.0 } else { 1e-308 };
            assert!((value / expected - 1.0).abs() < 2e-14);
        }
        assert_eq!(projector.fallback_allocations(), 0);
    }

    #[test]
    fn nonfinite_inputs_fail_closed_without_losing_workspace() {
        let problem = disconnected();
        let projector = StructuralRangeProjector::new(&problem);
        for bad in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
            let mut values = vec![bad; problem.dimension()];
            assert!(projector.project(&problem, &mut values).is_err());
            assert!(values.iter().all(|&value| value == 0.0));
        }
        let mut valid = vec![1.0; problem.dimension()];
        projector.project(&problem, &mut valid).unwrap();
        assert_eq!(projector.fallback_allocations(), 0);
    }

    #[test]
    fn genuinely_unrepresentable_projection_fails_closed() {
        let problem =
            ThreeWayProblem::from_observations([1, 2, 2], &[[0, 0, 0], [0, 1, 1]], &[1.0, 1.0])
                .unwrap();
        let mut values = vec![f64::MAX; problem.dimension()];
        let projector = StructuralRangeProjector::new(&problem);
        assert!(projector.project(&problem, &mut values).is_err());
        assert!(values.iter().all(|&value| value == 0.0));
    }

    #[test]
    fn agrees_with_reference_for_unequal_factor_sizes_and_is_idempotent() {
        let problem = ThreeWayProblem::from_observations(
            [2, 3, 4],
            &[[0, 0, 0], [0, 1, 1], [1, 2, 2], [0, 2, 3]],
            &[1.0; 4],
        )
        .unwrap();
        let original: Vec<_> = (0..problem.dimension())
            .map(|i| (i as f64 * 0.37).sin())
            .collect();
        let mut expected = original.clone();
        problem
            .components()
            .project_structural_range(&mut expected)
            .unwrap();
        let projector = StructuralRangeProjector::new(&problem);
        let mut actual = original;
        for _ in 0..2 {
            projector.project(&problem, &mut actual).unwrap();
            assert!(
                actual
                    .iter()
                    .zip(&expected)
                    .all(|(a, b)| (a - b).abs() < 2e-14)
            );
        }
    }
}
