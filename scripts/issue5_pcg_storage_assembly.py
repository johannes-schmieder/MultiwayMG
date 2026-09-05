from pathlib import Path
import re
import subprocess

BASE = '78f13b1c8b17e5ba4df90e406e375549b6169f25'
path = Path('crates/multiway-mg/src/pcg_trace.rs')
original = subprocess.check_output(['git', 'show', f'{BASE}:{path}'], text=True)
assert path.read_text() == original
assert subprocess.check_output(['git', 'hash-object', str(path)], text=True).strip() == '93597a013d9cdc855d1ed3091cff02910b26c469'

def once(text, old, new):
    assert text.count(old) == 1, (old, text.count(old))
    return text.replace(old, new)

# Preserve an independent pre-change solver, including its finite guards.
reference = original.replace('use crate::{', 'use multiway_mg::{')
reference = reference.replace('crate::error::dimension(', 'reference_dimension_error(')
reference = once(reference, 'mod finite;', '#[path = "pre_workspace_pcg_finite.rs"]\nmod finite;')
reference += '\nfn reference_dimension_error(context: &\'static str, expected: usize, actual: usize) -> MultiwayError {\n    MultiwayError::DimensionMismatch { context, expected, actual }\n}\n'
Path('crates/multiway-mg/tests/support/pre_workspace_pcg.rs').write_text('// Test-only allocating reference from 78f13b1; never a production solver.\n' + reference)
finite = subprocess.check_output(['git', 'show', f'{BASE}:crates/multiway-mg/src/pcg_trace/finite.rs'], text=True)
Path('crates/multiway-mg/tests/support/pre_workspace_pcg_finite.rs').write_text('// Independent pre-change finite guards from 78f13b1.\n' + finite.replace('use crate::MultiwayError;', 'use multiway_mg::MultiwayError;'))

start = original.index('fn solve_projected_pcg_traced_with_apply<P, F>(')
end = original.index('\nfn validate_options(', start)
old_core = original[start:end]
vstart = old_core.index('    validate_options(options)?;')
vend = old_core.index('    let mut projected_rhs = rhs.to_vec();')
validation = old_core[vstart:vend]
core = once(old_core, 'fn solve_projected_pcg_traced_with_apply<P, F>(', 'fn run<P, F>(')
core = once(core, '    mut apply_preconditioner: F,', '    workspace: &mut PcgTraceWorkspace,\n    mut apply_preconditioner: F,')
core = core.replace('PcgTraceResult', 'PcgTraceSummary')
core = once(core, validation, '    validate_inputs(problem, rhs, preconditioner, options)?;\n    workspace.validate(problem, options)?;\n')
core = once(core, '            solution: vec![0.0; dimension],\n', '')
core, n = re.subn(r'(?m)^\s*solution,\n', '', core)
assert n == 2
core, n = re.subn(r'(?m)^\s*samples,\n', '', core)
assert n == 3
core = once(core, '    let mut projected_rhs = rhs.to_vec();', '''    let PcgTraceWorkspace {
        projected_rhs, solution, residual, preconditioned, direction, applied,
        projection, samples,
    } = workspace;
    let projection = projection.as_mut().expect("validated projection workspace");
    projected_rhs.copy_from_slice(rhs);
    solution.fill(0.0);
    samples.clear();''')
core = once(core, '    let mut samples = vec![PcgTraceSample {', '    samples.push(PcgTraceSample {')
core = once(core, '    }];\n    if rhs_norm == 0.0 {', '    });\n    if rhs_norm == 0.0 {')
core = once(core, '    let mut solution = vec![0.0; dimension];\n', '')
core = once(core, '    let mut residual = projected_rhs.clone();', '    residual.copy_from_slice(projected_rhs);')
core = once(core, '    let mut preconditioned = vec![0.0; dimension];', '    preconditioned.fill(0.0);')
core = once(core, '    let mut direction = preconditioned.clone();', '    direction.copy_from_slice(preconditioned);')
core = once(core, '    let mut applied = vec![0.0; dimension];', '    applied.fill(0.0);')
core = once(core, '.zip(&preconditioned)', '.zip(preconditioned.iter())')
for name in ['projected_rhs', 'solution', 'residual', 'preconditioned', 'direction', 'applied']:
    core = core.replace('&mut ' + name, name).replace('&' + name, name)
core = once(core, '        residual = problem.residual(projected_rhs, solution)?;', '        problem.residual_into(projected_rhs, solution, residual)?;')
core, n = re.subn(r'\.project_structural_range\((\w+)\)\?', r'.project_structural_range_with_workspace(\1, projection)?', core)
assert n == 7
assert 'vec![' not in core and '.clone()' not in core and '.to_vec()' not in core

wrapper = '''fn solve_projected_pcg_traced_with_apply<P, F>(
    problem: &ThreeWayProblem,
    rhs: &[f64],
    preconditioner: &P,
    options: PcgTraceOptions,
    apply_preconditioner: F,
) -> Result<PcgTraceResult, MultiwayError>
where
    P: Preconditioner + ?Sized,
    F: FnMut(&[f64], &mut [f64]) -> Result<(), MultiwayError>,
{
    validate_inputs(problem, rhs, preconditioner, options)?;
    let mut workspace = PcgTraceWorkspace::try_new(problem, options)?;
    let summary = run(problem, rhs, preconditioner, options, &mut workspace, apply_preconditioner)?;
    Ok(workspace.into_result(summary))
}

'''
validator = '''
fn validate_inputs<P: Preconditioner + ?Sized>(
    problem: &ThreeWayProblem,
    rhs: &[f64],
    preconditioner: &P,
    options: PcgTraceOptions,
) -> Result<(), MultiwayError> {
''' + validation + '    Ok(())\n}\n'
text = original[:start] + wrapper + core + validator + original[end:]
text = once(text, 'mod finite;', '''mod finite;
mod result_ref;
mod workspace;

use result_ref::PcgTraceSummary;
pub use result_ref::PcgTraceResultRef;
pub use workspace::{
    PcgTraceWorkspace, solve_projected_pcg_traced_with_workspace,
    solve_projected_pcg_traced_with_workspaces,
};''')
text = once(text, '''/// its first application. Outer PCG vectors, trace storage, and the remaining
/// MAP/projection internals still allocate; this is not a full solver workspace.''', '''/// its first application. This owned-result convenience API creates local outer
/// storage. Use [`solve_projected_pcg_traced_with_workspaces`] to retain that
/// storage as well and return a borrowed result without output cloning.''')
path.write_text(text)
lib = Path('crates/multiway-mg/src/lib.rs')
text = lib.read_text()
text = once(text, '''    PcgTraceOptions, PcgTraceResult, PcgTraceSample, solve_projected_pcg_traced,
    solve_projected_pcg_traced_with_hierarchy_workspace,''', '''    PcgTraceOptions, PcgTraceResult, PcgTraceResultRef, PcgTraceSample, PcgTraceWorkspace,
    solve_projected_pcg_traced, solve_projected_pcg_traced_with_hierarchy_workspace,
    solve_projected_pcg_traced_with_workspace, solve_projected_pcg_traced_with_workspaces,''')
lib.write_text(text)
doc = Path('docs/ISSUE5_PCG_WORKSPACE.md')
text = doc.read_text()
lines = text.splitlines(keepends=True)
lines.insert(2, 'For reusable outer vectors, projection scratch and borrowed solution/trace storage,\nsee [ISSUE5_PCG_STORAGE.md](ISSUE5_PCG_STORAGE.md). The owned-return entry points\nbelow remain allocating convenience APIs.\n\n')
doc.write_text(''.join(lines))
change = Path('CHANGELOG.md')
text = change.read_text()
text = once(text, '### Added\n', '### Added\n\n- Caller-owned outer traced-PCG vectors, projection and bounded trace storage,\n  with borrowed results and one shared recurrence; see `docs/ISSUE5_PCG_STORAGE.md`.\n')
change.write_text(text)
