from pathlib import Path
import subprocess
BASE='39642744cc30eca24dc8d19fb58bc259c5e04505'
assert subprocess.check_output(['git','rev-parse','HEAD^'],text=True).strip()==BASE

def replace(path, old, new):
    p=Path(path); text=p.read_text()
    assert text.count(old)==1, (path,old,text.count(old))
    p.write_text(text.replace(old,new))

def append(path, text):
    p=Path(path); p.write_text(p.read_text()+'\n'+text)

p=Path('crates/multiway-mg/src/cycle_hierarchy/workspace.rs')
s=p.read_text()
marker='\nimpl CycleScreenedMapHierarchyWorkspace {\n    /// Read-only validation'
pos=s.index(marker)
block=s[pos:]
s=s[:pos]
assert s.count('#[cfg(test)]\nmod tests {')==1
s=s.replace('#[cfg(test)]\nmod tests {',block.strip()+'\n\n#[cfg(test)]\nmod tests {')
assert s.rstrip().endswith('}')
s=s.rstrip()[:-1]+r'''
    fn assert_unprepared(hierarchy: &CycleScreenedMapHierarchy, workspace: &CycleScreenedMapHierarchyWorkspace) {
        let before = format!("{workspace:?}");
        assert!(!workspace.is_prepared_for(hierarchy));
        let options = crate::PcgTraceOptions::default();
        let outer = crate::PcgTraceWorkspace::try_new(hierarchy.finest_problem(), options).unwrap();
        let rhs = vec![0.0; hierarchy.dimension()];
        assert!(matches!(crate::prepared_map_pcg_payload_report(
            hierarchy, &rhs, options, &outer, workspace, 0),
            Err(MultiwayError::WorkspaceNotPrepared { .. })));
        assert_eq!(format!("{workspace:?}"), before);
    }

    #[test]
    fn strict_preparation_inspects_every_active_recursive_buffer_and_binding() {
        let hierarchy = hierarchy();
        let other = self::hierarchy();
        let mut workspace = hierarchy.application_workspace().unwrap();
        assert!(workspace.is_prepared_for(&hierarchy));
        assert!(workspace.is_prepared_for(&hierarchy.clone()));
        for index in 0..workspace.buffers.len() {
            let value = workspace.buffers[index].pop().unwrap();
            assert_unprepared(&hierarchy, &workspace);
            workspace.buffers[index].push(value);
            assert!(workspace.is_prepared_for(&hierarchy));
        }
        let last = workspace.buffers.pop().unwrap();
        assert_unprepared(&hierarchy, &workspace);
        workspace.buffers.push(last);
        let last = workspace.operators.levels.pop().unwrap();
        assert_unprepared(&hierarchy, &workspace);
        workspace.operators.levels.push(last);
        for level in 0..hierarchy.problems.len() {
            workspace.operators.levels[level].projection.try_prepare_for(other.problems[level].components()).unwrap();
            assert_unprepared(&hierarchy, &workspace);
            workspace.operators.levels[level].projection.try_prepare_for(hierarchy.problems[level].components()).unwrap();
            assert!(workspace.is_prepared_for(&hierarchy));
        }
        for level in 0..hierarchy.smoothers.len() {
            let map = workspace.operators.levels[level].map.take();
            assert_unprepared(&hierarchy, &workspace);
            workspace.operators.levels[level].map = map;
            workspace.operators.levels[level].map.as_mut().unwrap().try_prepare_for(&other.smoothers[level]).unwrap();
            assert_unprepared(&hierarchy, &workspace);
            workspace.operators.levels[level].map.as_mut().unwrap().try_prepare_for(&hierarchy.smoothers[level]).unwrap();
            assert!(workspace.is_prepared_for(&hierarchy));
        }
        let wrong_terminal = crate::DensePseudoinverse::from_problem(hierarchy.finest_problem(), 1.0e-12).unwrap();
        workspace.operators.terminal.try_prepare_for(&wrong_terminal).unwrap();
        assert_unprepared(&hierarchy, &workspace);
        workspace.operators.terminal.try_prepare_for(&hierarchy.terminal).unwrap();
        assert!(workspace.is_prepared_for(&hierarchy));
    }
}
'''
p.write_text(s)
append('crates/multiway-incidence/src/topology.rs', r'''
#[cfg(test)]
mod payload_tests {
    use super::*;
    #[test]
    fn unused_tuple_capacity_is_charged() {
        let mut tuples = Vec::with_capacity(32);
        tuples.extend([[0, 0, 0], [1, 1, 1]]);
        let capacity = tuples.capacity();
        let topology = ThreeWayTopology::new([2; 3], tuples).unwrap();
        let bytes = topology.retained_payload_bytes().unwrap();
        assert_eq!(bytes, capacity * core::mem::size_of::<[u32; 3]>());
        assert!(bytes > core::mem::size_of_val(topology.tuples()));
    }
}
''')
append('crates/multiway-incidence/src/aggregation.rs', r'''
#[cfg(test)]
mod payload_tests {
    use super::*;
    #[test]
    fn unused_parent_capacity_is_charged() {
        let parents: [Vec<u32>; 3] = core::array::from_fn(|factor| {
            let mut values = Vec::with_capacity(8 + factor * 8);
            values.extend([0, 0]);
            values
        });
        let expected: usize = parents.iter().map(|p| p.capacity() * 4).sum();
        let map = FactorAggregation::new([2; 3], parents).unwrap();
        assert_eq!(map.retained_payload_bytes().unwrap(), expected);
        assert!(expected > 6 * core::mem::size_of::<u32>());
        assert_eq!(map.retained_payload_bytes().unwrap(), map.retained_bytes());
    }
}
''')
append('crates/multiway-incidence/src/components.rs', r'''
#[cfg(test)]
mod payload_tests {
    use super::*;
    #[test]
    fn component_payload_counts_spare_capacity_without_projection_scratch() {
        let topology = ThreeWayTopology::new([2; 3], vec![[0, 0, 0], [1, 1, 1]]).unwrap();
        let mut components = IncidenceComponents::from_topology(&topology);
        components.labels.reserve_exact(64);
        components.factor_sizes.reserve_exact(16);
        let expected = components.labels.capacity() * core::mem::size_of::<usize>()
            + components.factor_sizes.capacity() * core::mem::size_of::<[usize; 3]>();
        assert_eq!(components.retained_payload_bytes().unwrap(), expected);
        let scratch = components.try_projection_workspace().unwrap();
        assert!(scratch.retained_bytes() > 0);
        assert_eq!(components.retained_payload_bytes().unwrap(), expected);
    }
}
''')
# Test the invariant guard without altering a production hierarchy constructor.
p=Path('crates/multiway-mg/src/cycle_hierarchy/payload.rs'); s=p.read_text().rstrip(); assert s.endswith('}')
s=s[:-1]+r'''
    #[test]
    fn inventory_rejects_an_independently_rebuilt_smoother_problem() {
        let tuples = [[0, 0, 0], [0, 1, 1], [1, 0, 1], [1, 1, 0]];
        let problem = ThreeWayProblem::from_observations([2; 3], &tuples, &[1.0; 4]).unwrap();
        let independent = ThreeWayProblem::from_observations([2; 3], &tuples, &[1.0; 4]).unwrap();
        let mut hierarchy = CycleScreenedMapHierarchy::from_maps(problem,
            vec![FactorAggregation::consecutive_halving([2; 3]).unwrap()], 1.0e-12).unwrap();
        assert!(hierarchy.retained_payload_report().is_ok());
        hierarchy.smoothers[0] = SymmetricMapPreconditioner::new(independent);
        assert!(matches!(hierarchy.retained_payload_report(), Err(MultiwayError::PayloadInventoryMismatch)));
    }
}
'''; p.write_text(s)
# Clarify conservative charging when an immutable RHS aliases already counted data.
replace('crates/multiway-mg/src/pcg_trace/admission.rs', '    /// Submitted RHS slice payload; unused owner capacity is caller-declared extra.', '''    /// Submitted RHS slice payload; unused owner capacity is caller-declared extra.
    ///
    /// Always charged, even if it aliases immutable hierarchy data. Such aliasing
    /// conservatively overcounts; this API is not a pointer-deduplicating ledger.''')
append('crates/multiway-mg/tests/payload_admission.rs', r'''
#[test]
fn immutable_rhs_alias_is_conservatively_charged_and_not_omitted() {
    let hierarchy = hierarchy(2);
    let options = PcgTraceOptions::default();
    let mut outer = PcgTraceWorkspace::try_new(hierarchy.finest_problem(), options).unwrap();
    let mut inner = hierarchy.application_workspace().unwrap();
    let rhs = hierarchy.finest_problem().diagonal();
    let report = prepared_map_pcg_payload_report(&hierarchy, rhs, options, &outer, &inner, 0).unwrap();
    assert_eq!(report.rhs_bytes, core::mem::size_of_val(rhs));
    assert_eq!(report.total_bytes().unwrap(), report.hierarchy.total_bytes().unwrap()
        + outer.retained_bytes().unwrap() + inner.retained_bytes().unwrap() + core::mem::size_of_val(rhs));
    let result = solve_projected_pcg_traced_with_payload_budget(&hierarchy, rhs, options,
        &mut outer, &mut inner, PcgPayloadBudget {
            maximum_bytes: report.total_bytes().unwrap(), additional_live_bytes: 0,
        }).unwrap();
    assert!(result.converged());
}
''')
replace('docs/ISSUE5_PAYLOAD_ADMISSION.md', '''Do not charge an alias of already-counted storage twice. The library cannot discover
or verify unreported external allocations.''', '''Do not add an alias of already-counted storage to the caller's extra-live charge.
The RHS slice itself is always charged, even if it aliases immutable hierarchy data;
in that unusual case the report conservatively overcounts. It is exact within its
payload exclusions for the normal disjoint-RHS case, not an arbitrary pointer-based
deduplicating ledger. The library cannot discover or verify external allocations.''')
p=Path('docs/ISSUE5_PAYLOAD_ADMISSION.md'); p.write_text(p.read_text().replace('\n\n\n\n###','\n\n###'))
append('docs/ISSUE5_PAYLOAD_ADMISSION.md', '''
Review regressions also remove each active traversal vector element in turn, swap
each projection/MAP binding, omit per-level storage and change terminal modal size.
The read-only preparation check rejects all of them before a strict solve, even for
zero RHS. Separate tests verify spare tuple/parent/component capacity and the
smoother-sharing invariant. Immutable-RHS aliasing is explicitly tested as
conservative overcharging rather than silently claiming general deduplication.
''')
print('Payload review repairs applied; no numerical recurrence changed.')
