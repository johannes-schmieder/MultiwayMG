//! Exact component-binding contracts for structural projection scratch.

use multiway_incidence::ThreeWayProblem;

fn problem(swapped: bool) -> ThreeWayProblem {
    let tuples = if swapped {
        [[0, 1, 0], [1, 0, 1]]
    } else {
        [[0, 0, 0], [1, 1, 1]]
    };
    ThreeWayProblem::from_observations([2; 3], &tuples, &[1.0, 2.0]).unwrap()
}

#[test]
fn different_partition_rejects_projection_before_mutating_values_or_scratch() {
    let owner = problem(false);
    let other = problem(true);
    assert_eq!(owner.dimension(), other.dimension());
    assert_eq!(owner.components().count(), other.components().count());
    assert_ne!(owner.components().labels(), other.components().labels());
    let mut workspace = owner.components().projection_workspace();
    let before = workspace.clone();
    let mut values = [2.0, -3.0, 5.0, 7.0, -11.0, 13.0];
    let original = values;
    let result = other
        .components()
        .project_structural_range_with_workspace(&mut values, &mut workspace);
    assert!(result.is_err(), "incompatible partition accepted");
    assert_eq!(values, original);
    assert_eq!(workspace, before);
    let mut expected = original;
    owner
        .components()
        .project_structural_range(&mut expected)
        .unwrap();
    owner
        .components()
        .project_structural_range_with_workspace(&mut values, &mut workspace)
        .unwrap();
    assert_eq!(values, expected);
}

#[test]
fn different_partition_rejects_defect_before_mutating_scratch() {
    let owner = problem(false);
    let other = problem(true);
    let mut workspace = owner.components().projection_workspace();
    let before = workspace.clone();
    let values = [2.0, -3.0, 5.0, 7.0, -11.0, 13.0];
    let result = other
        .components()
        .maximum_structural_defect_with_workspace(&values, &mut workspace);
    assert!(result.is_err(), "incompatible partition accepted");
    assert_eq!(workspace, before);
    let expected = owner
        .components()
        .maximum_structural_defect(&values)
        .unwrap();
    let actual = owner
        .components()
        .maximum_structural_defect_with_workspace(&values, &mut workspace)
        .unwrap();
    assert_eq!(actual.to_bits(), expected.to_bits());
}

#[test]
fn cloned_components_and_workspace_remain_compatible_after_owner_drop() {
    let owner = problem(false);
    let components = owner.components().clone();
    let workspace = owner.components().projection_workspace();
    let mut cloned_workspace = workspace.clone();
    let bytes = workspace.retained_bytes();
    drop(owner);
    drop(workspace);
    for scale in [1.0, -2.0, 0.5] {
        let mut values = [2.0, -3.0, 5.0, 7.0, -11.0, 13.0].map(|v| scale * v);
        let mut expected = values;
        let expected_norm = components.project_structural_range(&mut expected).unwrap();
        let actual_norm = components
            .project_structural_range_with_workspace(&mut values, &mut cloned_workspace)
            .unwrap();
        assert_eq!(actual_norm.to_bits(), expected_norm.to_bits());
        assert_eq!(values, expected);
        assert_eq!(cloned_workspace.retained_bytes(), bytes);
    }
}

#[test]
fn equal_metadata_is_not_a_shared_binding_even_after_the_owner_is_dropped() {
    let owner = problem(false);
    let other = problem(false);
    assert_eq!(owner.components(), other.components());
    let mut workspace = owner.components().projection_workspace();
    let other_workspace = other.components().projection_workspace();
    assert_ne!(workspace, other_workspace);
    let before = workspace.clone();
    drop(owner);
    let mut values = [2.0, -3.0, 5.0, 7.0, -11.0, 13.0];
    let original = values;
    let result = other
        .components()
        .project_structural_range_with_workspace(&mut values, &mut workspace);
    assert!(matches!(
        result,
        Err(multiway_incidence::IncidenceError::WorkspaceBindingMismatch { .. })
    ));
    assert_eq!(values, original);
    assert_eq!(workspace, before);
}
