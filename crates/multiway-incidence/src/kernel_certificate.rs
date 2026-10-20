//! Exact topology-only certificate of the complete incidence kernel.
//!
//! The kernel of the tuple-by-level incidence matrix `B` equals the kernel of
//! every Gramian `B^T W B` with positive weights, so it depends only on the
//! prepared topology. Each component always contains the two factor-shift
//! directions; supports with additional nullity contain further directions that
//! structural projection alone cannot remove.
//!
//! Certification is exact and deterministic. Gaussian elimination of the tuple
//! rows over the prime field `2^61 - 1` yields pivots whose minor is nonzero
//! modulo the prime and therefore nonzero over the integers: a lower bound on
//! the rational rank. When a component has fewer than `V_c - 2` pivots, back
//! substitution yields one modular kernel vector per free coordinate. Each is
//! rationally reconstructed, scaled to coprime integers and verified exactly
//! against every incidence row of its component. Their free-coordinate pattern
//! proves independence, so the verified vectors bound the rank from above and
//! the two bounds meet. Any failed reconstruction or verification refuses to
//! certify; it is never silently replaced by a floating-point rank decision.

mod modular;
mod projector;
#[cfg(test)]
mod tests;

use std::{cmp::Reverse, collections::BinaryHeap};

use crate::{
    IncidenceError, PreparedThreeWayTopology, PreparedTopologyBinding, construction::array_bytes,
};
use modular::{add, inverse, mul, neg, reconstruct, sub};
pub use projector::{IncidenceRangeProjector, IncidenceRangeProjectorReport};

/// Declared live requested-array payload for exact kernel certification.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IncidenceKernelLimits {
    /// Ceiling on simultaneously requested array payload, charging both old and
    /// new capacities during growth. Excludes the borrowed topology, inline
    /// objects and allocator excess; this is not a process-memory quota.
    pub maximum_payload_bytes: usize,
}

/// Exact kernel classification of one incidence component.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ComponentKernelKind {
    /// The kernel is exactly the span of the two factor-shift directions.
    Structural,
    /// The kernel is larger; an explicit verified integer basis is retained.
    Extended,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
struct ComponentSummary {
    rank: usize,
    nullity: usize,
    basis_offset: usize,
}

/// Work and payload counters for one certification.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct IncidenceKernelReport {
    /// Peak live requested-array payload, including reallocation overlap.
    pub peak_requested_payload_bytes: usize,
    /// Tuple rows reduced before every component was resolved.
    pub eliminated_tuples: usize,
    /// Stored modular echelon entries at the end of elimination.
    pub stored_pivot_entries: usize,
    /// Largest absolute entry of any retained integer kernel vector.
    pub maximum_basis_magnitude: u64,
}

/// Borrowed exact integer kernel basis for one extended component.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ComponentKernelBasis<'c> {
    coordinates: &'c [usize],
    vectors: &'c [i64],
}

impl<'c> ComponentKernelBasis<'c> {
    /// Global coefficient coordinates of the component in increasing order.
    #[must_use]
    pub const fn coordinates(&self) -> &'c [usize] {
        self.coordinates
    }

    /// Number of basis vectors, equal to the exact component nullity.
    #[must_use]
    pub fn nullity(&self) -> usize {
        self.vectors.len() / self.coordinates.len()
    }

    /// One coprime integer kernel vector over [`Self::coordinates`].
    #[must_use]
    pub fn vector(&self, index: usize) -> &'c [i64] {
        let width = self.coordinates.len();
        &self.vectors[index * width..(index + 1) * width]
    }
}

/// Exact kernel of a prepared topology's incidence matrix, per component.
///
/// Bound to one prepared owner, which it cannot outlive. Contains no weights:
/// every positive-weight frame on the same owner shares this kernel.
#[derive(Debug)]
pub struct IncidenceKernelCertificate<'a> {
    binding: PreparedTopologyBinding<'a>,
    component_offsets: Vec<usize>,
    coordinates: Vec<usize>,
    summaries: Vec<ComponentSummary>,
    basis: Vec<i64>,
    report: IncidenceKernelReport,
}

impl<'a> IncidenceKernelCertificate<'a> {
    /// Certify the exact kernel within a declared live array payload.
    ///
    /// Budget, overflow, allocation and reconstruction failures return errors;
    /// no partial certificate is published.
    pub fn try_certify(
        owner: &'a PreparedThreeWayTopology,
        limits: IncidenceKernelLimits,
    ) -> Result<Self, IncidenceError> {
        Self::certify_with(owner, limits, &mut |_| Ok(()))
    }

    pub(crate) fn certify_with<F>(
        owner: &'a PreparedThreeWayTopology,
        limits: IncidenceKernelLimits,
        before: &mut F,
    ) -> Result<Self, IncidenceError>
    where
        F: FnMut(&'static str) -> Result<(), IncidenceError>,
    {
        let topology = owner.topology();
        let labels = owner.component_labels();
        let factor_sizes = owner.component_factor_sizes();
        let dimension = labels.len();
        let count = factor_sizes.len();
        let mut ledger = Ledger::new(limits.maximum_payload_bytes, before);

        let offsets_len = count
            .checked_add(1)
            .ok_or(IncidenceError::DimensionOverflow {
                context: "kernel component offsets",
            })?;
        let mut component_offsets = ledger.vec::<usize>(offsets_len, "kernel component offsets")?;
        component_offsets.resize(offsets_len, 0);
        for &label in labels {
            component_offsets[label + 1] += 1;
        }
        for component in 0..count {
            component_offsets[component + 1] += component_offsets[component];
        }
        let mut coordinates = ledger.vec::<usize>(dimension, "kernel component coordinates")?;
        coordinates.resize(dimension, 0);
        {
            let mut cursor = ledger.vec::<usize>(count, "kernel coordinate cursors")?;
            cursor.extend_from_slice(&component_offsets[..count]);
            for (coordinate, &label) in labels.iter().enumerate() {
                coordinates[cursor[label]] = coordinate;
                cursor[label] += 1;
            }
            ledger.release::<usize>(count);
        }
        let mut summaries = ledger.vec::<ComponentSummary>(count, "kernel component summaries")?;
        summaries.resize(count, ComponentSummary::default());

        let mut pivot_of = ledger.vec::<usize>(dimension, "kernel pivot rows")?;
        pivot_of.resize(dimension, usize::MAX);
        let mut accumulator = ledger.vec::<u64>(dimension, "kernel row accumulator")?;
        accumulator.resize(dimension, 0);
        let mut queued = ledger.vec::<bool>(dimension, "kernel queued columns")?;
        queued.resize(dimension, false);
        let mut heap = BinaryHeap::from(ledger.vec::<Reverse<usize>>(dimension, "kernel heap")?);
        let mut remaining = ledger.vec::<usize>(count, "kernel remaining pivots")?;
        // Every prepared component has a level in each factor, so V_c >= 3.
        remaining.extend(
            factor_sizes
                .iter()
                .map(|sizes| sizes.iter().sum::<usize>() - 2),
        );
        let starts_len = dimension
            .checked_add(1)
            .ok_or(IncidenceError::DimensionOverflow {
                context: "kernel pivot starts",
            })?;
        let mut row_starts = ledger.vec::<usize>(starts_len, "kernel pivot starts")?;
        row_starts.push(0);
        let mut row_columns = Growable::<usize>::default();
        let mut row_values = Growable::<u64>::default();
        let mut report = IncidenceKernelReport::default();

        let mut open = count;
        for tuple in topology.tuples() {
            if open == 0 {
                break;
            }
            let row = [0, 1, 2].map(|factor| topology.global_index(factor, tuple[factor]));
            let component = labels[row[0]];
            if remaining[component] == 0 {
                continue;
            }
            report.eliminated_tuples += 1;
            for coordinate in row {
                accumulator[coordinate] = 1;
                queued[coordinate] = true;
                heap.push(Reverse(coordinate));
            }
            while let Some(Reverse(column)) = heap.pop() {
                queued[column] = false;
                let value = accumulator[column];
                if value == 0 {
                    continue;
                }
                accumulator[column] = 0;
                let pivot = pivot_of[column];
                if pivot != usize::MAX {
                    // Stored rows lead with their pivot; every later entry is larger.
                    for entry in row_starts[pivot] + 1..row_starts[pivot + 1] {
                        let target = row_columns.values[entry];
                        let updated =
                            sub(accumulator[target], mul(value, row_values.values[entry]));
                        accumulator[target] = updated;
                        if updated != 0 && !queued[target] {
                            queued[target] = true;
                            heap.push(Reverse(target));
                        }
                    }
                    continue;
                }
                // New pivot: the remaining queued columns complete its normalized row.
                let needed = row_columns.values.len().checked_add(heap.len() + 1).ok_or(
                    IncidenceError::DimensionOverflow {
                        context: "kernel pivot entries",
                    },
                )?;
                row_columns.ensure(&mut ledger, needed, "kernel pivot columns")?;
                row_values.ensure(&mut ledger, needed, "kernel pivot values")?;
                let scale = inverse(value);
                row_columns.values.push(column);
                row_values.values.push(1);
                while let Some(Reverse(target)) = heap.pop() {
                    queued[target] = false;
                    let residual = accumulator[target];
                    if residual != 0 {
                        row_columns.values.push(target);
                        row_values.values.push(mul(residual, scale));
                        accumulator[target] = 0;
                    }
                }
                pivot_of[column] = row_starts.len() - 1;
                row_starts.push(row_columns.values.len());
                summaries[component].rank += 1;
                remaining[component] -= 1;
                if remaining[component] == 0 {
                    open -= 1;
                }
                break;
            }
        }
        report.stored_pivot_entries = row_columns.values.len();
        drop(heap);
        drop(queued);
        ledger.release::<Reverse<usize>>(dimension);
        ledger.release::<bool>(dimension);

        let mut basis_len = 0usize;
        for (component, summary) in summaries.iter_mut().enumerate() {
            let width = component_offsets[component + 1] - component_offsets[component];
            summary.nullity = width - summary.rank;
            summary.basis_offset = basis_len;
            if remaining[component] != 0 {
                basis_len = summary
                    .nullity
                    .checked_mul(width)
                    .and_then(|size| basis_len.checked_add(size))
                    .ok_or(IncidenceError::DimensionOverflow {
                        context: "kernel basis entries",
                    })?;
            }
        }
        let mut basis = ledger.vec::<i64>(basis_len, "kernel integer basis")?;
        basis.resize(basis_len, 0);
        let scratch = &mut accumulator;
        for component in 0..count {
            if remaining[component] == 0 {
                continue;
            }
            let span = &coordinates[component_offsets[component]..component_offsets[component + 1]];
            let summary = summaries[component];
            let width = span.len();
            let mut vector_index = 0;
            for &free in span {
                if pivot_of[free] != usize::MAX {
                    continue;
                }
                for &coordinate in span.iter().rev() {
                    let pivot = pivot_of[coordinate];
                    scratch[coordinate] = if pivot == usize::MAX {
                        u64::from(coordinate == free)
                    } else {
                        let mut sum = 0;
                        for entry in row_starts[pivot] + 1..row_starts[pivot + 1] {
                            sum = add(
                                sum,
                                mul(row_values.values[entry], scratch[row_columns.values[entry]]),
                            );
                        }
                        neg(sum)
                    };
                }
                let start = summary.basis_offset + vector_index * width;
                let magnitude =
                    scale_to_integers(component, span, scratch, &mut basis[start..start + width])?;
                report.maximum_basis_magnitude = report.maximum_basis_magnitude.max(magnitude);
                // Independence: own free coordinate nonzero, every other one zero.
                for (local, &coordinate) in span.iter().enumerate() {
                    if pivot_of[coordinate] == usize::MAX
                        && (basis[start + local] != 0) != (coordinate == free)
                    {
                        return Err(IncidenceError::KernelUncertified {
                            component,
                            reason: "free-coordinate pattern does not prove independence",
                        });
                    }
                }
                vector_index += 1;
            }
            if vector_index != summary.nullity {
                return Err(IncidenceError::KernelUncertified {
                    component,
                    reason: "free coordinates do not match the modular rank",
                });
            }
        }
        drop(row_columns.values);
        drop(row_values.values);
        ledger.release::<usize>(row_columns.requested);
        ledger.release::<u64>(row_values.requested);
        drop(row_starts);
        ledger.release::<usize>(starts_len);

        // Independent exact check of every retained vector against every tuple row.
        if basis_len != 0 {
            let local_of = &mut pivot_of;
            for component in 0..count {
                let range = component_offsets[component]..component_offsets[component + 1];
                for (local, &coordinate) in coordinates[range].iter().enumerate() {
                    local_of[coordinate] = local;
                }
            }
            for tuple in topology.tuples() {
                let row = [0, 1, 2].map(|factor| topology.global_index(factor, tuple[factor]));
                let component = labels[row[0]];
                if remaining[component] == 0 {
                    continue;
                }
                let summary = summaries[component];
                let width = component_offsets[component + 1] - component_offsets[component];
                for vector in 0..summary.nullity {
                    let start = summary.basis_offset + vector * width;
                    let sum: i128 = row
                        .iter()
                        .map(|&coordinate| i128::from(basis[start + local_of[coordinate]]))
                        .sum();
                    if sum != 0 {
                        return Err(IncidenceError::KernelUncertified {
                            component,
                            reason: "reconstructed kernel vector fails an incidence row",
                        });
                    }
                }
            }
        }
        report.peak_requested_payload_bytes = ledger.peak;
        Ok(Self {
            binding: owner.binding(),
            component_offsets,
            coordinates,
            summaries,
            basis,
            report,
        })
    }

    /// Borrowed identity of the prepared owner this certificate describes.
    #[must_use]
    pub const fn binding(&self) -> PreparedTopologyBinding<'a> {
        self.binding
    }

    /// Coefficient dimension of the certified topology.
    #[must_use]
    pub fn dimension(&self) -> usize {
        self.coordinates.len()
    }

    /// Number of incidence components.
    #[must_use]
    pub fn component_count(&self) -> usize {
        self.summaries.len()
    }

    /// Exact rank of the incidence matrix.
    #[must_use]
    pub fn rank(&self) -> usize {
        self.summaries.iter().map(|summary| summary.rank).sum()
    }

    /// Exact nullity of the incidence matrix and every positive-weight Gramian.
    #[must_use]
    pub fn nullity(&self) -> usize {
        self.dimension() - self.rank()
    }

    /// Exact nullity beyond the two structural directions per component.
    #[must_use]
    pub fn additional_nullity(&self) -> usize {
        self.nullity() - 2 * self.component_count()
    }

    /// Exact rank of one component's incidence block.
    #[must_use]
    pub fn component_rank(&self, component: usize) -> usize {
        self.summaries[component].rank
    }

    /// Exact nullity of one component's incidence block.
    #[must_use]
    pub fn component_nullity(&self, component: usize) -> usize {
        self.summaries[component].nullity
    }

    /// Whether one component's kernel is purely structural.
    #[must_use]
    pub fn component_kind(&self, component: usize) -> ComponentKernelKind {
        if self.summaries[component].nullity == 2 {
            ComponentKernelKind::Structural
        } else {
            ComponentKernelKind::Extended
        }
    }

    /// Global coordinates of one component in increasing order.
    #[must_use]
    pub fn component_coordinates(&self, component: usize) -> &[usize] {
        &self.coordinates[self.component_offsets[component]..self.component_offsets[component + 1]]
    }

    /// Verified integer basis of an extended component; `None` when structural.
    #[must_use]
    pub fn component_basis(&self, component: usize) -> Option<ComponentKernelBasis<'_>> {
        if self.component_kind(component) == ComponentKernelKind::Structural {
            return None;
        }
        let coordinates = self.component_coordinates(component);
        let summary = self.summaries[component];
        let end = summary.basis_offset + summary.nullity * coordinates.len();
        Some(ComponentKernelBasis {
            coordinates,
            vectors: &self.basis[summary.basis_offset..end],
        })
    }

    /// Certification work and peak requested payload.
    #[must_use]
    pub const fn report(&self) -> IncidenceKernelReport {
        self.report
    }

    /// Actual retained array capacity of this certificate.
    ///
    /// Excludes the inline object, the borrowed topology and allocator overhead.
    pub fn retained_payload_bytes(&self) -> Result<usize, IncidenceError> {
        crate::construction::sum_bytes(&[
            array_bytes::<usize>(self.component_offsets.capacity())?,
            array_bytes::<usize>(self.coordinates.capacity())?,
            array_bytes::<ComponentSummary>(self.summaries.capacity())?,
            array_bytes::<i64>(self.basis.capacity())?,
        ])
    }
}

/// Scale one modular kernel vector to coprime integers, returning its largest magnitude.
fn scale_to_integers(
    component: usize,
    span: &[usize],
    residues: &[u64],
    output: &mut [i64],
) -> Result<u64, IncidenceError> {
    let failed = |reason| IncidenceError::KernelUncertified { component, reason };
    let mut common: i128 = 1;
    for &coordinate in span {
        let (_, denominator) = reconstruct(residues[coordinate])
            .ok_or_else(|| failed("rational reconstruction exceeded its bound"))?;
        let denominator = i128::from(denominator);
        let factor = denominator / modular::gcd(common as u128, denominator as u128) as i128;
        common = common
            .checked_mul(factor)
            .filter(|&value| value <= i128::from(i64::MAX))
            .ok_or_else(|| failed("common denominator exceeds 64-bit integers"))?;
    }
    let mut divisor: u128 = 0;
    for (slot, &coordinate) in output.iter_mut().zip(span) {
        let (numerator, denominator) =
            reconstruct(residues[coordinate]).expect("reconstruction succeeded above");
        let value = i128::from(numerator) * (common / i128::from(denominator));
        *slot = i64::try_from(value)
            .ok()
            .filter(|&value| value != i64::MIN)
            .ok_or_else(|| failed("scaled kernel entry exceeds 64-bit integers"))?;
        divisor = modular::gcd(divisor, value.unsigned_abs());
    }
    if divisor == 0 {
        return Err(failed("reconstructed kernel vector is zero"));
    }
    let mut maximum = 0;
    for slot in output.iter_mut() {
        *slot /= divisor as i64;
        maximum = maximum.max(slot.unsigned_abs());
    }
    Ok(maximum)
}

/// Live requested-array payload with checked growth and a reservation hook.
struct Ledger<'f, F> {
    live: usize,
    peak: usize,
    budget: usize,
    before: &'f mut F,
}

impl<'f, F> Ledger<'f, F>
where
    F: FnMut(&'static str) -> Result<(), IncidenceError>,
{
    fn new(budget: usize, before: &'f mut F) -> Self {
        Self {
            live: 0,
            peak: 0,
            budget,
            before,
        }
    }

    fn charge(&mut self, context: &'static str, bytes: usize) -> Result<(), IncidenceError> {
        let required = self
            .live
            .checked_add(bytes)
            .ok_or(IncidenceError::DimensionOverflow { context })?;
        if required > self.budget {
            return Err(IncidenceError::KernelBudgetExceeded {
                context,
                required,
                budget: self.budget,
            });
        }
        self.live = required;
        self.peak = self.peak.max(required);
        Ok(())
    }

    fn release<T>(&mut self, count: usize) {
        self.live -= count * core::mem::size_of::<T>();
    }

    fn vec<T>(&mut self, count: usize, context: &'static str) -> Result<Vec<T>, IncidenceError> {
        let bytes = array_bytes::<T>(count)?;
        self.charge(context, bytes)?;
        let mut values = Vec::new();
        if count != 0 {
            (self.before)(context)?;
            values
                .try_reserve_exact(count)
                .map_err(|_| IncidenceError::KernelAllocation { context })?;
        }
        Ok(values)
    }
}

/// Geometrically grown array whose requested capacity is charged with overlap.
#[derive(Default)]
struct Growable<T> {
    values: Vec<T>,
    requested: usize,
}

impl<T> Growable<T> {
    fn ensure<F>(
        &mut self,
        ledger: &mut Ledger<'_, F>,
        needed: usize,
        context: &'static str,
    ) -> Result<(), IncidenceError>
    where
        F: FnMut(&'static str) -> Result<(), IncidenceError>,
    {
        if needed <= self.requested {
            return Ok(());
        }
        // A budget-independent growth policy keeps the peak a property of the input.
        let target = needed.max(self.requested.saturating_mul(2)).max(64);
        // Old and new capacities coexist while the array is reallocated.
        ledger.charge(context, array_bytes::<T>(target)?)?;
        (ledger.before)(context)?;
        self.values
            .try_reserve_exact(target - self.values.len())
            .map_err(|_| IncidenceError::KernelAllocation { context })?;
        ledger.release::<T>(self.requested);
        self.requested = target;
        Ok(())
    }
}
