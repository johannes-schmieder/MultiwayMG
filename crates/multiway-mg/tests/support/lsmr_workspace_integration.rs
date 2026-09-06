//! Compose the pinned reusable LSMR with actual frame actions and mutable MAP scratch.
use super::{GLOBAL, Result, no_events};
use multiway_incidence::{
    PreparedThreeWayTopology, ThreeWayOperatorView, ThreeWayWeightFrame, WeightFrameInput,
};
use multiway_mg::{
    Preconditioner, SymmetricMapPreconditioner, SymmetricMapWorkspace, ThreeWayProblem,
};
use schwarz_precond::{
    MlsmrWorkspace, MlsmrWorkspaceOptions, OperatorMut, SolveError, mlsmr_with_workspace,
};
use std::hint::black_box;

struct WeightedAction<'frame, 'topology>(ThreeWayOperatorView<'frame, 'topology>);
impl OperatorMut for WeightedAction<'_, '_> {
    fn nrows(&self) -> usize {
        self.0.tuple_count()
    }
    fn ncols(&self) -> usize {
        self.0.dimension()
    }
    fn apply(&mut self, x: &[f64], y: &mut [f64]) -> std::result::Result<(), SolveError> {
        self.0
            .apply_weighted_incidence(x, y)
            .map_err(|_| SolveError::Synchronization {
                context: "test incidence adapter",
            })
    }
    fn apply_adjoint(&mut self, x: &[f64], y: &mut [f64]) -> std::result::Result<(), SolveError> {
        self.0
            .apply_weighted_adjoint(x, y)
            .map_err(|_| SolveError::Synchronization {
                context: "test adjoint adapter",
            })
    }
}
struct MapAction<'a> {
    map: &'a SymmetricMapPreconditioner,
    scratch: SymmetricMapWorkspace,
}
impl OperatorMut for MapAction<'_> {
    fn nrows(&self) -> usize {
        self.map.dimension()
    }
    fn ncols(&self) -> usize {
        self.map.dimension()
    }
    fn apply(&mut self, x: &[f64], y: &mut [f64]) -> std::result::Result<(), SolveError> {
        self.map
            .apply_with_workspace(x, y, &mut self.scratch)
            .map_err(|_| SolveError::Synchronization {
                context: "test mutable MAP adapter",
            })
    }
    fn apply_adjoint(&mut self, x: &[f64], y: &mut [f64]) -> std::result::Result<(), SolveError> {
        self.apply(x, y)
    }
}
pub fn run() -> Result<()> {
    let tuples: Vec<_> = (0..3)
        .flat_map(|i| (0..4).flat_map(move |j| (0..2).map(move |k| [i, j, k])))
        .collect();
    let weights: Vec<_> = (0..tuples.len()).map(|i| 0.5 + (i % 7) as f64).collect();
    let topology = PreparedThreeWayTopology::try_from_collapsed([3, 4, 2], &tuples)?;
    let frame = ThreeWayWeightFrame::try_new(&topology, WeightFrameInput::Tuples(&weights))?;
    let view = frame.operator_view();
    let problem = ThreeWayProblem::from_observations([3, 4, 2], &tuples, &weights)?;
    let map = SymmetricMapPreconditioner::new(problem);
    let mut preconditioner = MapAction {
        map: &map,
        scratch: map.application_workspace()?,
    };
    let mut operator = WeightedAction(view);
    let mut workspace = MlsmrWorkspace::try_new(view.tuple_count(), view.dimension(), Some(8))?;
    let mut targets = vec![0.0; view.tuple_count()];
    let mut b = targets.clone();
    let mut residual = targets.clone();
    let mut rhs = vec![0.0; view.dimension()];
    let mut gradient = rhs.clone();
    let mut coefficients = rhs.clone();
    let before = GLOBAL.stats();
    for column in 0..32 {
        for (i, x) in coefficients.iter_mut().enumerate() {
            *x = ((i + column) as f64 * 0.31).sin();
        }
        view.apply_incidence(&coefficients, &mut targets)?;
        for ((out, &target), &root) in b.iter_mut().zip(&targets).zip(frame.square_root_weights()) {
            *out = target * root;
        }
        let result = mlsmr_with_workspace(
            &mut operator,
            black_box(&b),
            &mut preconditioner,
            1e-10,
            200,
            MlsmrWorkspaceOptions::default(),
            &mut workspace,
        )?;
        view.apply_incidence(result.x, &mut residual)?;
        for (r, &y) in residual.iter_mut().zip(&targets) {
            *r = y - *r;
        }
        view.rhs_from_targets_into(&residual, &mut gradient)?;
        view.rhs_from_targets_into(&targets, &mut rhs)?;
        let numerator = gradient.iter().map(|v| v * v).sum::<f64>().sqrt();
        let denominator = rhs.iter().map(|v| v * v).sum::<f64>().sqrt();
        let relative = numerator / denominator;
        assert!(relative.is_finite() && relative <= 1e-8);
    }
    no_events(GLOBAL.stats() - before);
    println!(
        "fork integration: frame incidence + mutable MAP + LSMR + original certificate; first/repeat32 allocations=0"
    );
    Ok(())
}
