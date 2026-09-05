//! Independent pre-change arithmetic and explicit preparation contracts.

#[allow(dead_code)]
#[path = "support/pre_workspace_map.rs"]
mod old_map;
#[allow(dead_code)]
#[path = "support/pre_workspace_dense.rs"]
mod old_dense;
#[allow(dead_code)]
#[path = "../examples/support/issue3_recursive_fixtures.rs"]
mod fixtures;

use multiway_mg::{
    DensePseudoinverse, DensePseudoinverseWorkspace, Preconditioner,
    SymmetricMapPreconditioner, ThreeWayProblem,
};

fn bits(actual: &[f64], expected: &[f64]) {
    assert_eq!(actual.len(), expected.len());
    for (index, (actual, expected)) in actual.iter().zip(expected).enumerate() {
        assert_eq!(actual.to_bits(), expected.to_bits(), "entry {index}");
    }
}

fn rhs(dimension: usize, scale: f64) -> Vec<f64> {
    (0..dimension).map(|i| scale * ((i as f64 + 0.25) * 0.71).sin()).collect()
}

fn problem(levels: usize, swapped: bool, scale: f64) -> ThreeWayProblem {
    let tuples: Vec<_> = (0..levels).map(|i| [i as u32, ((i + usize::from(swapped)) % levels) as u32, i as u32]).collect();
    let weights: Vec<_> = (0..levels).map(|i| scale * (i + 1) as f64).collect();
    ThreeWayProblem::from_observations([levels; 3], &tuples, &weights).unwrap()
}

#[test]
fn map_matches_prechange_arithmetic_on_every_revealed_recursive_fixture() -> Result<(), fixtures::DynError> {
    let fixtures = fixtures::recursive_holdout_fixtures()?;
    assert_eq!(fixtures.len(), 8);
    for fixture in fixtures {
        let reference = old_map::AllocatingMapReference::new(fixture.problem.clone());
        let map = SymmetricMapPreconditioner::new(fixture.problem);
        let mut workspace = map.application_workspace()?;
        let retained = workspace.retained_bytes()?;
        assert!(retained >= map.workspace_required_bytes()?);
        for scale in [1.0, -2.0, 0.0, 0.5] {
            let rhs = rhs(map.dimension(), scale);
            let mut expected = vec![0.0; rhs.len()];
            reference.apply(&rhs, &mut expected)?;
            let mut actual = vec![f64::NAN; rhs.len()];
            map.apply_with_workspace(&rhs, &mut actual, &mut workspace)?;
            bits(&actual, &expected);
            map.apply(&rhs, &mut actual)?;
            bits(&actual, &expected);
            map.clone().apply_with_workspace(&rhs, &mut actual, &mut workspace)?;
            bits(&actual, &expected);
            assert_eq!(workspace.retained_bytes()?, retained);
        }
    }
    Ok(())
}

#[test]
fn map_requires_explicit_repreparation_for_independent_components() {
    let owner = SymmetricMapPreconditioner::new(problem(2, false, 1.0));
    let other = SymmetricMapPreconditioner::new(problem(2, true, 1.0));
    let mut workspace = owner.application_workspace().unwrap();
    let input = rhs(owner.dimension(), 1.0);
    let mut out = vec![23.0; input.len()];
    let before = out.clone();
    assert!(other.apply_with_workspace(&input, &mut out, &mut workspace).is_err());
    bits(&out, &before);
    let retained = workspace.retained_bytes().unwrap();
    workspace.try_prepare_for(&other).unwrap();
    let reference = old_map::AllocatingMapReference::new(other.problem().clone());
    let mut expected = vec![0.0; input.len()];
    reference.apply(&input, &mut expected).unwrap();
    other.apply_with_workspace(&input, &mut out, &mut workspace).unwrap();
    bits(&out, &expected);
    assert_eq!(workspace.retained_bytes().unwrap(), retained);
    assert!(owner.apply_with_workspace(&input, &mut out, &mut workspace).is_err());
    for length in [input.len() - 1, input.len() + 1] {
        let mut out = vec![23.0; length];
        let before = out.clone();
        assert!(other.apply_with_workspace(&input, &mut out, &mut workspace).is_err());
        bits(&out, &before);
        let mut out = vec![23.0; input.len()];
        let before = out.clone();
        assert!(other.apply_with_workspace(&vec![1.0; length], &mut out, &mut workspace).is_err());
        bits(&out, &before);
    }
    for levels in [4, 1, 3, 2, 4] {
        let next = SymmetricMapPreconditioner::new(problem(levels, true, 0.75));
        workspace.try_prepare_for(&next).unwrap();
        let input = rhs(next.dimension(), -1.25);
        let mut actual = vec![0.0; input.len()];
        let mut expected = actual.clone();
        old_map::AllocatingMapReference::new(next.problem().clone()).apply(&input, &mut expected).unwrap();
        next.apply_with_workspace(&input, &mut actual, &mut workspace).unwrap();
        bits(&actual, &expected);
    }
}

#[test]
fn projection_preparation_changes_binding_only_at_an_explicit_boundary() {
    let owner = problem(2, false, 1.0);
    let other = problem(2, true, 1.0);
    let mut workspace = owner.components().try_projection_workspace().unwrap();
    assert!(workspace.is_compatible_with(owner.components()));
    assert!(!workspace.is_compatible_with(other.components()));
    let bytes = workspace.retained_bytes();
    let before = workspace.clone();
    workspace.try_prepare_for(owner.components()).unwrap();
    assert_eq!(workspace, before);
    workspace.try_prepare_for(other.components()).unwrap();
    assert!(workspace.is_compatible_with(other.components()));
    assert!(!workspace.is_compatible_with(owner.components()));
    assert_eq!(workspace.retained_bytes(), bytes);
    for levels in [4, 1, 3, 2, 4] {
        let next = problem(levels, true, 2.0);
        workspace.try_prepare_for(next.components()).unwrap();
        let mut actual = rhs(next.dimension(), 1.0);
        let mut expected = actual.clone();
        let a = next.components().project_structural_range(&mut expected).unwrap();
        let b = next.components().project_structural_range_with_workspace(&mut actual, &mut workspace).unwrap();
        bits(&actual, &expected);
        assert_eq!(a.to_bits(), b.to_bits());
        assert!(workspace.retained_bytes() >= next.components().projection_workspace_required_bytes().unwrap());
    }
}

#[test]
fn terminal_matches_independent_prechange_reference_and_reuses_anonymous_scratch() {
    let mut workspace = DensePseudoinverseWorkspace::new();
    for levels in [1, 4, 2, 4, 3] {
        let problem = problem(levels, true, 0.75);
        let terminal = DensePseudoinverse::from_problem(&problem, 1.0e-12).unwrap();
        let reference = old_dense::AllocatingTerminalReference::from_problem(&problem, 1.0e-12).unwrap();
        workspace.try_prepare_for(&terminal).unwrap();
        let retained = workspace.retained_bytes().unwrap();
        assert!(retained >= terminal.workspace_required_bytes().unwrap());
        for scale in [1.0, -2.0, 0.0, 0.5] {
            let input = rhs(problem.dimension(), scale);
            let mut expected = vec![0.0; input.len()];
            reference.solve_into(&input, &mut expected).unwrap();
            let mut actual = vec![f64::NAN; input.len()];
            terminal.solve_into_with_workspace(&input, &mut actual, &mut workspace).unwrap();
            bits(&actual, &expected);
            terminal.solve_into(&input, &mut actual).unwrap();
            bits(&actual, &expected);
            assert_eq!(workspace.retained_bytes().unwrap(), retained);
        }
        let input = rhs(problem.dimension(), 1.0);
        let mut out = vec![23.0; input.len()];
        let before = out.clone();
        assert!(terminal.solve_into_with_workspace(&input, &mut out, &mut DensePseudoinverseWorkspace::new()).is_err());
        bits(&out, &before);
        let mut bad_out = vec![23.0; input.len() + 1];
        let before = bad_out.clone();
        assert!(terminal.solve_into_with_workspace(&input, &mut bad_out, &mut workspace).is_err());
        bits(&bad_out, &before);
    }
}
