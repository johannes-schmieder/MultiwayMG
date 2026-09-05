from pathlib import Path

def once(text, old, new):
    assert text.count(old) == 1, (old, text.count(old))
    return text.replace(old, new)

p = Path('crates/multiway-mg/tests/pcg_storage.rs')
s = p.read_text()
s = s.replace('std::cell::Cell<usize>', 'std::sync::atomic::AtomicUsize')
s = s.replace('std::cell::Cell::new(0)', 'std::sync::atomic::AtomicUsize::new(0)')
s = once(s, '        let calls = self.calls.get() + 1;\n        self.calls.set(calls);', '        let calls = self.calls.fetch_add(1, std::sync::atomic::Ordering::Relaxed) + 1;')
s = once(s, 'mock.calls.get()', 'mock.calls.load(std::sync::atomic::Ordering::Relaxed)')
s += '''
#[test]
fn failed_trace_reservation_does_not_publish_a_new_problem_binding() {
    let owner = problem(2, 1.0);
    let larger = problem(5, 1.0);
    let hierarchy = hierarchy(&owner);
    let options = PcgTraceOptions::default();
    let input = rhs(&owner, 1.0);
    let mut workspace = PcgTraceWorkspace::try_new(&owner, options).unwrap();
    let expected = solve_projected_pcg_traced_with_workspace(&owner, &input, &hierarchy, options, &mut workspace).unwrap().to_owned();
    let before = format!("{workspace:?}");
    // The requested trace exceeds isize::MAX bytes, so Vec rejects capacity
    // before asking the allocator for a huge allocation. Earlier small vector
    // reservations may succeed; lengths, values and binding must not publish.
    let impossible = PcgTraceOptions {
        max_iterations: isize::MAX as usize / core::mem::size_of::<multiway_mg::PcgTraceSample>(),
        ..options
    };
    assert!(matches!(workspace.try_prepare_for(&larger, impossible), Err(MultiwayError::WorkspaceAllocation { .. })));
    assert_eq!(format!("{workspace:?}"), before);
    let actual = solve_projected_pcg_traced_with_workspace(&owner, &input, &hierarchy, options, &mut workspace).unwrap().to_owned();
    assert_eq!(actual, expected);
}

#[test]
fn same_sized_different_component_partitions_require_explicit_preparation() {
    let owner = ThreeWayProblem::from_observations([2; 3], &[[0,0,0], [1,1,1]], &[1.0, 2.0]).unwrap();
    let other = ThreeWayProblem::from_observations([2; 3], &[[0,1,0], [1,0,1]], &[1.0, 2.0]).unwrap();
    assert_eq!(owner.dimension(), other.dimension());
    assert_eq!(owner.components().count(), other.components().count());
    let hierarchy = CycleScreenedMapHierarchy::from_maps(other.clone(), vec![], 1.0e-12).unwrap();
    let options = PcgTraceOptions::default();
    let input = rhs(&other, 1.0);
    let mut workspace = PcgTraceWorkspace::try_new(&owner, options).unwrap();
    let before = format!("{workspace:?}");
    let error = solve_projected_pcg_traced_with_workspace(&other, &input, &hierarchy, options, &mut workspace).unwrap_err();
    assert!(matches!(error, MultiwayError::Incidence(multiway_incidence::IncidenceError::WorkspaceBindingMismatch { .. })));
    assert_eq!(format!("{workspace:?}"), before);
    workspace.try_prepare_for(&other, options).unwrap();
    let actual = solve_projected_pcg_traced_with_workspace(&other, &input, &hierarchy, options, &mut workspace).unwrap().to_owned();
    let expected = reference::solve_projected_pcg_traced(&other, &input, &hierarchy, old_options(options)).unwrap();
    assert_reference(&actual, &expected);
}
'''
p.write_text(s)
p = Path('crates/multiway-mg/src/pcg_trace/workspace.rs')
s = p.read_text()
needle = '    #[test]\n    fn counter_and_capacity_overflow_preserve_existing_storage()'
addition = '''    #[test]
    fn poisoned_vectors_and_trace_are_overwritten_before_use() {
        let problem = ThreeWayProblem::from_observations([2; 3], &[[0,0,0], [1,1,1]], &[1.0, 2.0]).unwrap();
        let hierarchy = CycleScreenedMapHierarchy::from_maps(problem.clone(), vec![], 1.0e-12).unwrap();
        let options = PcgTraceOptions::default();
        let mut input = vec![0.0; problem.dimension()];
        problem.apply_gramian(&[1.0, -2.0, 3.0, -4.0, 5.0, -6.0], &mut input).unwrap();
        let expected = crate::solve_projected_pcg_traced(&problem, &input, &hierarchy, options).unwrap();
        let mut workspace = PcgTraceWorkspace::try_new(&problem, options).unwrap();
        let bytes = workspace.retained_bytes().unwrap();
        for vector in [&mut workspace.projected_rhs, &mut workspace.solution, &mut workspace.residual, &mut workspace.preconditioned, &mut workspace.direction, &mut workspace.applied] {
            vector.fill(f64::NAN);
        }
        workspace.samples.push(PcgTraceSample { iteration: 999, residual_norm: f64::NAN, relative_residual: f64::NAN });
        let actual = solve_projected_pcg_traced_with_workspace(&problem, &input, &hierarchy, options, &mut workspace).unwrap().to_owned();
        assert_eq!(actual, expected);
        assert_eq!(workspace.retained_bytes().unwrap(), bytes);
    }

'''
s = once(s, needle, addition + needle)
p.write_text(s)
p = Path('crates/multiway-mg/tests/workspace_allocations.rs')
s = p.read_text()
s = once(s, 'mod fixtures;', 'mod fixtures;\n#[path = "support/pcg_allocations.rs"]\nmod pcg_allocations;')
s = once(s, '    operator_checks()?;', '    operator_checks()?;\n    pcg_allocations::run()?;')
p.write_text(s)
p = Path('docs/ISSUE5_PCG_STORAGE.md')
s = p.read_text()
old = '''The complete prepared-solve allocator gate must additionally run in the existing
isolated executable and three-platform debug/release, minimal/all-feature Actions
matrix before this increment is qualified. Full Rust 1.85/scientific Actions and
an exact-diff review are required.'''
new = '''The existing isolated allocation executable now measures complete borrowed solves
in `tests/support/pcg_allocations.rs`: 17 hierarchy cases plus an allocation-free
generic-preconditioner control. It asserts zero allocations, reallocations and
deallocations on the first prepared solve, eight repeated signed/zero/scaled RHS
solves, the iteration-limit path, explicit repreparation and recovery after an
invalid numerical state. Solution and trace pointers remain stable. Static
binding/dimension/budget rejection allocates nothing. Error-message construction
for numerical failures is separately measured and is not claimed allocation-free.
Fresh outer storage allocation and release must equal its exclusive byte report;
an explicit `to_owned` copy must make the separately charged two array allocations.
Input construction, setup, reference solves, logging and external independent
certification are outside the measured solve region. The pre-existing complete
MAP-cycle allocation tests remain intact.

The unchanged three-platform debug/release, minimal/all-feature Actions workflow
runs this expanded executable. Full Rust 1.85/scientific Actions and an exact-diff
review are required.'''
s = once(s, old, new)
p.write_text(s)
