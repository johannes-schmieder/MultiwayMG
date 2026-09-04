//! Principal immutable-state estimates used by research diagnostics.

use crate::ThreeWayProblem;

/// Estimate principal heap bytes in one immutable weighted three-way problem.
///
/// This counts tuple keys, tuple weights, square-root weights, the coefficient
/// diagonal, component labels, and component factor-size metadata. It excludes
/// allocator metadata, `Arc` headers, stack fields, and sharing with other
/// owners, so callers must label the result as an estimate rather than an exact
/// process-memory measurement.
#[must_use]
pub(crate) fn estimate_three_way_problem_bytes(problem: &ThreeWayProblem) -> usize {
    let tuple_bytes = problem
        .tuple_count()
        .saturating_mul(core::mem::size_of::<[u32; 3]>() + 16);
    let level_bytes = problem.dimension().saturating_mul(16);
    let component_bytes = problem
        .dimension()
        .saturating_mul(core::mem::size_of::<usize>())
        .saturating_add(
            problem
                .components()
                .count()
                .saturating_mul(core::mem::size_of::<[usize; 3]>()),
        );
    tuple_bytes
        .saturating_add(level_bytes)
        .saturating_add(component_bytes)
}
