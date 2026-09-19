//! Dependency-pin integration, not an admitted sparse-terminal route.
#![cfg(feature = "within-comparator")]
use multiway_mg::{DensePseudoinverse, Preconditioner, ThreeWayProblem};
use schwarz_precond::{
    MlsmrWorkspace, MlsmrWorkspaceOptions, OperatorMut, SolveError,
    mlsmr_with_workspace_and_candidate_gate,
};
use within::{Effect, PreconditionerConfig, Solver};

struct Rect<'a>(&'a ThreeWayProblem);
impl OperatorMut for Rect<'_> {
    fn nrows(&self) -> usize {
        self.0.tuple_count()
    }
    fn ncols(&self) -> usize {
        self.0.dimension()
    }
    fn apply(&mut self, x: &[f64], y: &mut [f64]) -> Result<(), SolveError> {
        self.0
            .apply_weighted_incidence(x, y)
            .expect("validated test incidence");
        Ok(())
    }
    fn apply_adjoint(&mut self, x: &[f64], y: &mut [f64]) -> Result<(), SolveError> {
        self.0
            .apply_weighted_adjoint(x, y)
            .expect("validated test adjoint");
        Ok(())
    }
}
fn rhs_and_gradient(p: &ThreeWayProblem, y: &[f64], x: &[f64]) -> (Vec<f64>, Vec<f64>) {
    let mut rhs = vec![0.; 12];
    let mut gradient = vec![0.; 12];
    for ((t, &w), &target) in p.topology().tuples().iter().zip(p.weights()).zip(y) {
        let ids = [t[0] as usize, 4 + t[1] as usize, 8 + t[2] as usize];
        let fitted = x[ids[0]] + x[ids[1]] + x[ids[2]];
        for id in ids {
            rhs[id] += w * target;
            gradient[id] += w * (target - fitted);
        }
    }
    (rhs, gradient)
}
fn ratio(p: &ThreeWayProblem, y: &[f64], x: &[f64]) -> f64 {
    let (rhs, gradient) = rhs_and_gradient(p, y, x);
    let norm = |v: &[f64]| v.iter().map(|a| a * a).sum::<f64>().sqrt();
    let denom = norm(&rhs);
    let numerator = norm(&gradient);
    if denom == 0. {
        if numerator == 0. { 0. } else { f64::INFINITY }
    } else {
        numerator / denom
    }
}
#[test]
fn owner_bound_serial_dependency_action_drives_certified_independent_rhs() {
    for extra_nullity in [false, true] {
        let tuples = if extra_nullity {
            vec![
                [0, 0, 0],
                [1, 0, 0],
                [1, 1, 1],
                [2, 1, 1],
                [2, 2, 2],
                [3, 2, 3],
                [3, 3, 3],
            ]
        } else {
            (0..4)
                .flat_map(|a| (0..4).flat_map(move |b| (0..4).map(move |c| [a, b, c])))
                .collect()
        };
        let weights: Vec<_> = (0..tuples.len())
            .map(|i| 2.0_f64.powi((i % 7) as i32 - 3))
            .collect();
        let p = ThreeWayProblem::from_observations([4; 3], &tuples, &weights).unwrap();
        let levels: [Vec<u32>; 3] =
            std::array::from_fn(|factor| p.topology().tuples().iter().map(|t| t[factor]).collect());
        let effects = levels
            .iter()
            .map(|v| Effect::new(v, true, std::iter::empty::<&[f64]>()).unwrap())
            .collect::<Vec<_>>();
        let solver = Solver::new(
            effects,
            Some(p.weights().to_vec()),
            PreconditionerConfig::default(),
        )
        .unwrap();
        let pre = solver.preconditioner().unwrap();
        let bytes = pre.serial_workspace_required_payload_bytes().unwrap();
        let mut action = pre.try_serial_workspace(bytes).unwrap();
        let mut recurrence =
            MlsmrWorkspace::try_new(p.tuple_count(), p.dimension(), Some(8)).unwrap();
        let mut op = Rect(&p);
        let dense = DensePseudoinverse::from_problem(&p, 1e-12).unwrap();
        for columns in [1, 2, 4, 8, 16, 17, 32] {
            for column in 0..columns {
                let y: Vec<_> = (0..p.tuple_count())
                    .map(|i| {
                        if column == 0 {
                            0.
                        } else {
                            ((i + 1 + column * 3) as f64 * 0.37).sin()
                        }
                    })
                    .collect();
                let weighted: Vec<_> = y
                    .iter()
                    .zip(p.weights())
                    .map(|(v, w)| v * w.sqrt())
                    .collect();
                let mut gate = |x: &[f64], warm: Option<&[f64]>| {
                    assert!(warm.is_none());
                    Ok(ratio(&p, &y, x) <= 1e-10)
                };
                let result = mlsmr_with_workspace_and_candidate_gate(
                    &mut op,
                    &weighted,
                    &mut action,
                    1e-10,
                    1000,
                    MlsmrWorkspaceOptions::default(),
                    &mut gate,
                    &mut recurrence,
                )
                .unwrap();
                assert!(
                    ratio(&p, &y, result.x) <= 1e-10,
                    "nullity={extra_nullity} K={columns} column={column}"
                );
                let (rhs, _) = rhs_and_gradient(&p, &y, &[0.; 12]);
                let mut exact = vec![0.; 12];
                dense.apply(&rhs, &mut exact).unwrap();
                // Additional nullity permits distinct coefficients; compare fitted values.
                let mut fit = vec![0.; p.tuple_count()];
                let mut ref_fit = fit.clone();
                p.apply_incidence(result.x, &mut fit).unwrap();
                p.apply_incidence(&exact, &mut ref_fit).unwrap();
                for (a, b) in fit.iter().zip(ref_fit) {
                    assert!((a - b).abs() <= 1e-8 * (1. + b.abs()));
                }
            }
        }
        assert_eq!(action.retained_payload_bytes(), bytes);
    }
}
