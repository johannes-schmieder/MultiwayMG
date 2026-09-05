//! Retained ownership inventory for one fixed MAP hierarchy, not setup peak RSS.

use super::CycleScreenedMapHierarchy;
use crate::{FactorAggregation, MultiwayError, SymmetricMapPreconditioner, ThreeWayProblem};

/// Disjoint payload categories reachable from one built MAP hierarchy.
///
/// Shared level problems are counted once, not again through smoother clones.
/// Excludes the inline hierarchy root, Arc headers/padding, allocator overhead,
/// construction plans/transients and all caller/workspace storage. Reports for
/// two hierarchies cannot simply be summed when they share problem allocations.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MapHierarchyPayloadReport {
    /// Immutable fine/coarse problem payload, potentially shared with other owners.
    pub shared_problem_bytes: usize,
    /// Heap descriptor arrays for problems, aggregations and smoothers.
    pub descriptor_bytes: usize,
    /// Exclusively owned factor-parent array capacities.
    pub aggregation_bytes: usize,
    /// Exclusively owned terminal matrix and inverse-eigenvalue capacities.
    pub terminal_bytes: usize,
}

impl MapHierarchyPayloadReport {
    /// Disjoint exclusive payload, excluding shared problem state.
    pub fn exclusive_bytes(self) -> Result<usize, MultiwayError> {
        sum(&[
            self.descriptor_bytes,
            self.aggregation_bytes,
            self.terminal_bytes,
        ])
    }

    /// Total retained payload for this ownership boundary.
    pub fn total_bytes(self) -> Result<usize, MultiwayError> {
        sum(&[self.shared_problem_bytes, self.exclusive_bytes()?])
    }
}

impl CycleScreenedMapHierarchy {
    /// Inventory one built hierarchy without allocating, cloning or applying it.
    ///
    /// Each strictly smaller level owns independently constructed problem state.
    /// The smoother at that level must share all of its immutable backing state;
    /// fail closed if this private construction invariant is ever broken.
    pub fn retained_payload_report(&self) -> Result<MapHierarchyPayloadReport, MultiwayError> {
        if self.smoothers.len() != self.aggregations.len()
            || self.problems.len() != self.aggregations.len() + 1
            || self
                .smoothers
                .iter()
                .zip(&self.problems)
                .any(|(smoother, problem)| !smoother.problem().shares_storage_with(problem))
            || self
                .problems
                .windows(2)
                .any(|pair| pair[1].dimension() >= pair[0].dimension())
        {
            return Err(MultiwayError::PayloadInventoryMismatch);
        }
        let shared_problem_bytes = self.problems.iter().try_fold(0usize, |total, problem| {
            sum(&[total, problem.retained_payload_bytes()?])
        })?;
        let descriptor_bytes = sum(&[
            bytes::<ThreeWayProblem>(self.problems.capacity())?,
            bytes::<FactorAggregation>(self.aggregations.capacity())?,
            bytes::<SymmetricMapPreconditioner>(self.smoothers.capacity())?,
        ])?;
        let aggregation_bytes = self.aggregations.iter().try_fold(0usize, |total, map| {
            sum(&[total, map.retained_payload_bytes()?])
        })?;
        let report = MapHierarchyPayloadReport {
            shared_problem_bytes,
            descriptor_bytes,
            aggregation_bytes,
            terminal_bytes: self.terminal.retained_payload_bytes()?,
        };
        report.total_bytes()?;
        Ok(report)
    }
}

fn bytes<T>(count: usize) -> Result<usize, MultiwayError> {
    count
        .checked_mul(core::mem::size_of::<T>())
        .ok_or_else(overflow)
}
fn sum(parts: &[usize]) -> Result<usize, MultiwayError> {
    parts.iter().try_fold(0usize, |total, &part| {
        total.checked_add(part).ok_or_else(overflow)
    })
}
fn overflow() -> MultiwayError {
    MultiwayError::WorkspaceSizeOverflow {
        context: "MAP hierarchy payload",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn report_arithmetic_never_wraps() {
        assert!(bytes::<f64>(usize::MAX).is_err());
        assert!(sum(&[usize::MAX, 1]).is_err());
        let report = MapHierarchyPayloadReport {
            shared_problem_bytes: usize::MAX,
            descriptor_bytes: 1,
            aggregation_bytes: 0,
            terminal_bytes: 0,
        };
        assert!(report.total_bytes().is_err());
    }
}
