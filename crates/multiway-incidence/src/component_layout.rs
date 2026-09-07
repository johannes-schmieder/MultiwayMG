//! Bounded flat component permutations and separately lived tuple recoding.
use crate::{
    IncidenceError, PreparedHierarchyBudget, PreparedThreeWayTopology,
    construction::{array_bytes, reserve, sum_bytes},
    error::dimension,
};
use core::ops::Range;

/// Source IDs, ascending within the named component factor or tuple row.
#[derive(Debug, Clone)]
pub enum ComponentSourceIds<'a> {
    /// A single component uses the original contiguous range without storage.
    Contiguous(Range<usize>),
    /// Factor-local level IDs, or tuple IDs whose largest value fits u32.
    Compact(&'a [u32]),
    /// Native tuple IDs when the largest value requires usize.
    Wide(&'a [usize]),
}
impl ComponentSourceIds<'_> {
    /// Number of source IDs in this view.
    pub fn len(&self) -> usize {
        match self {
            Self::Contiguous(r) => r.len(),
            Self::Compact(r) => r.len(),
            Self::Wide(r) => r.len(),
        }
    }
    /// Whether this ID sequence is empty.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
    /// Dispatch once, then visit ascending source IDs without allocating.
    pub fn for_each(self, mut f: impl FnMut(usize)) {
        match self {
            Self::Contiguous(r) => r.for_each(f),
            Self::Compact(r) => r.iter().for_each(|&i| f(i as usize)),
            Self::Wide(r) => r.iter().for_each(|&i| f(i)),
        }
    }
}
#[derive(Debug)]
enum TupleIds {
    Compact(Vec<u32>),
    Wide(Vec<usize>),
}
impl TupleIds {
    fn retained(&self) -> Result<usize, IncidenceError> {
        match self {
            Self::Compact(x) => array_bytes::<u32>(x.capacity()),
            Self::Wide(x) => array_bytes::<usize>(x.capacity()),
        }
    }
}

/// Exact-owner component permutation; no local tuple or numerical copies.
///
/// Connected input borrows implicit source ranges and allocates no arrays.
/// Otherwise three arrays retain combined component offsets, factor-local source
/// level IDs and compact/wide source tuple IDs. A temporary counting cursor is
/// released during construction. The inverse needed to recode local keys lives
/// separately in PreparedComponentRecoding and need not survive numerical setup.
///
/// A layout cannot outlive its exact source owner:
/// ```compile_fail
/// use multiway_incidence::{PreparedComponentLayout, PreparedThreeWayTopology,
///     PreparedHierarchyBudget};
/// let layout = {
///     let t = PreparedThreeWayTopology::try_from_collapsed([1;3], &[[0;3]]).unwrap();
///     PreparedComponentLayout::try_new(&t, PreparedHierarchyBudget::UNLIMITED).unwrap()
/// };
/// assert_eq!(layout.component_count(), 1);
/// ```
/// Component views cannot survive moving or dropping their layout:
/// ```compile_fail
/// use multiway_incidence::{PreparedComponentLayout, PreparedThreeWayTopology,
///     PreparedHierarchyBudget};
/// let t = PreparedThreeWayTopology::try_from_collapsed([1;3], &[[0;3]]).unwrap();
/// let layout = PreparedComponentLayout::try_new(&t, PreparedHierarchyBudget::UNLIMITED).unwrap();
/// let view = layout.component(0).unwrap();
/// drop(layout);
/// assert_eq!(view.dimension(), 3);
/// ```
#[derive(Debug)]
pub struct PreparedComponentLayout<'topology> {
    topology: &'topology PreparedThreeWayTopology,
    offsets: Vec<usize>,
    levels: Vec<u32>,
    tuples: TupleIds,
    setup_peak_payload_bound: usize,
}
impl<'topology> PreparedComponentLayout<'topology> {
    /// Prepare component rows after checked sizing and payload admission.
    /// The borrowed prepared owner already guarantees every level is used.
    pub fn try_new(
        topology: &'topology PreparedThreeWayTopology,
        budget: PreparedHierarchyBudget,
    ) -> Result<Self, IncidenceError> {
        Self::build_with(
            topology,
            budget,
            narrow(topology.topology().tuple_count()),
            &mut |_| Ok(()),
        )
    }
    /// Requested exclusive arrays; zero for a single supported component.
    pub fn required_payload_bytes(
        topology: &PreparedThreeWayTopology,
    ) -> Result<usize, IncidenceError> {
        partition_bytes(topology, narrow(topology.topology().tuple_count()))
    }
    /// Conservative simultaneous requested arrays including source and caller state.
    pub fn setup_payload_bound(
        topology: &PreparedThreeWayTopology,
        budget: PreparedHierarchyBudget,
    ) -> Result<usize, IncidenceError> {
        setup_bytes(topology, budget, narrow(topology.topology().tuple_count()))
    }
    fn build_with<F>(
        topology: &'topology PreparedThreeWayTopology,
        budget: PreparedHierarchyBudget,
        compact: bool,
        before: &mut F,
    ) -> Result<Self, IncidenceError>
    where
        F: FnMut(&'static str) -> Result<(), IncidenceError>,
    {
        let required = setup_bytes(topology, budget, compact)?;
        admit(required, budget)?;
        let c = topology.component_factor_sizes().len();
        if c == 1 {
            return Ok(Self {
                topology,
                offsets: Vec::new(),
                levels: Vec::new(),
                tuples: TupleIds::Compact(Vec::new()),
                setup_peak_payload_bound: required,
            });
        }
        let v = topology.topology().total_levels();
        let e = topology.topology().tuple_count();
        let base = add(c, 1)?;
        let mut offsets = reserve(add(base, base)?, "component offsets", before)?;
        offsets.resize(2 * base, 0usize);
        let mut levels = reserve(v, "component source levels", before)?;
        levels.resize(v, 0u32);
        let mut tuples = if compact {
            let mut ids = reserve(e, "compact component tuple IDs", before)?;
            ids.resize(e, 0u32);
            TupleIds::Compact(ids)
        } else {
            let mut ids = reserve(e, "wide component tuple IDs", before)?;
            ids.resize(e, 0usize);
            TupleIds::Wide(ids)
        };
        let mut cursor = reserve(c, "component placement cursor", before)?;
        cursor.resize(c, 0usize);
        for (i, sizes) in topology.component_factor_sizes().iter().enumerate() {
            offsets[i + 1] = add(offsets[i], sizes.iter().sum())?;
        }
        let source = topology.topology();
        let labels = topology.component_labels();
        for q in 0..3 {
            for i in 0..c {
                cursor[i] = offsets[i]
                    + topology.component_factor_sizes()[i][..q]
                        .iter()
                        .sum::<usize>();
            }
            for level in 0..source.level_counts()[q] {
                let component = labels[source.offsets()[q] + level];
                levels[cursor[component]] = level as u32;
                cursor[component] += 1;
            }
        }
        for key in source.tuples() {
            let component = labels[key[0] as usize];
            offsets[base + component + 1] += 1;
        }
        for i in 0..c {
            offsets[base + i + 1] = add(offsets[base + i + 1], offsets[base + i])?;
        }
        cursor.copy_from_slice(&offsets[base..base + c]);
        match &mut tuples {
            TupleIds::Compact(ids) => {
                for (i, key) in source.tuples().iter().enumerate() {
                    let component = labels[key[0] as usize];
                    ids[cursor[component]] = i as u32;
                    cursor[component] += 1;
                }
            }
            TupleIds::Wide(ids) => {
                for (i, key) in source.tuples().iter().enumerate() {
                    let component = labels[key[0] as usize];
                    ids[cursor[component]] = i;
                    cursor[component] += 1;
                }
            }
        }
        let mut result = Self {
            topology,
            offsets,
            levels,
            tuples,
            setup_peak_payload_bound: required,
        };
        let actual = sum_bytes(&[
            topology.retained_payload_bytes()?,
            result.retained_payload_bytes()?,
            array_bytes::<usize>(cursor.capacity())?,
            budget.additional_live_payload_bytes,
        ])?;
        admit(actual, budget)?;
        result.setup_peak_payload_bound = required.max(actual);
        Ok(result)
    }
    /// Exact original structural owner, including original observation groups.
    pub const fn topology(&self) -> &'topology PreparedThreeWayTopology {
        self.topology
    }
    /// Number of exact supported incidence components; not a numerical rank.
    pub fn component_count(&self) -> usize {
        self.topology.component_factor_sizes().len()
    }
    /// Whether all source mappings are implicit and no partition arrays exist.
    pub fn is_identity(&self) -> bool {
        self.component_count() == 1
    }
    /// Checked component view; invalid IDs expose no writable operation.
    pub fn component(&self, index: usize) -> Option<PreparedComponentView<'_, 'topology>> {
        (index < self.component_count()).then_some(PreparedComponentView {
            layout: self,
            index,
        })
    }
    /// Reject even a value-equal reconstruction with another structural owner.
    pub fn validate_for(&self, topology: &PreparedThreeWayTopology) -> Result<(), IncidenceError> {
        self.topology.binding().validate_for(topology)
    }
    /// Actual exclusive retained capacity, excluding source, inline root and stack.
    pub fn retained_payload_bytes(&self) -> Result<usize, IncidenceError> {
        sum_bytes(&[
            array_bytes::<usize>(self.offsets.capacity())?,
            array_bytes::<u32>(self.levels.capacity())?,
            self.tuples.retained()?,
        ])
    }
    /// Largest admitted requested or actual-capacity construction payload.
    pub const fn setup_peak_payload_bound(&self) -> usize {
        self.setup_peak_payload_bound
    }
    /// Stored bytes per tuple ID, or zero for the implicit connected path.
    pub fn tuple_index_bytes(&self) -> usize {
        if self.is_identity() {
            0
        } else {
            match self.tuples {
                TupleIds::Compact(_) => size_of::<u32>(),
                TupleIds::Wide(_) => size_of::<usize>(),
            }
        }
    }
}

/// Borrowed component with checked identity and factor-major local coordinates.
#[derive(Debug, Clone, Copy)]
pub struct PreparedComponentView<'layout, 'topology> {
    layout: &'layout PreparedComponentLayout<'topology>,
    index: usize,
}
impl<'layout> PreparedComponentView<'layout, '_> {
    /// Exact original component label.
    pub const fn index(&self) -> usize {
        self.index
    }
    /// Local coefficient counts, independently within each factor.
    pub fn level_counts(&self) -> [usize; 3] {
        self.layout.topology.component_factor_sizes()[self.index]
    }
    /// Local coefficient dimension.
    pub fn dimension(&self) -> usize {
        self.level_counts().iter().sum()
    }
    /// Ascending original factor-local level IDs, or None for an invalid factor.
    pub fn factor_levels(&self, factor: usize) -> Option<ComponentSourceIds<'layout>> {
        if factor >= 3 {
            return None;
        }
        let counts = self.level_counts();
        if self.layout.is_identity() {
            Some(ComponentSourceIds::Contiguous(0..counts[factor]))
        } else {
            let start = self.layout.offsets[self.index] + counts[..factor].iter().sum::<usize>();
            Some(ComponentSourceIds::Compact(
                &self.layout.levels[start..start + counts[factor]],
            ))
        }
    }
    /// Ascending original canonical tuple IDs; no weight copies.
    pub fn tuple_ids(&self) -> ComponentSourceIds<'layout> {
        if self.layout.is_identity() {
            return ComponentSourceIds::Contiguous(
                0..self.layout.topology.topology().tuple_count(),
            );
        }
        let base = self.layout.component_count() + 1;
        let range =
            self.layout.offsets[base + self.index]..self.layout.offsets[base + self.index + 1];
        match &self.layout.tuples {
            TupleIds::Compact(ids) => ComponentSourceIds::Compact(&ids[range]),
            TupleIds::Wide(ids) => ComponentSourceIds::Wide(&ids[range]),
        }
    }
    /// Number of canonical tuples in this component.
    pub fn tuple_count(&self) -> usize {
        self.tuple_ids().len()
    }
    /// Gather factor-major coefficients; validate both lengths before writes.
    pub fn gather_coefficients<T: Copy>(
        &self,
        source: &[T],
        local: &mut [T],
    ) -> Result<(), IncidenceError> {
        self.coefficient_lengths(source.len(), local.len())?;
        if self.layout.is_identity() {
            local.copy_from_slice(source);
            return Ok(());
        }
        let mut i = 0;
        for q in 0..3 {
            let offset = self.layout.topology.topology().offsets()[q];
            self.factor_levels(q).unwrap().for_each(|id| {
                local[i] = source[offset + id];
                i += 1;
            });
        }
        Ok(())
    }
    /// Scatter only this component; other global coefficients remain untouched.
    pub fn scatter_coefficients<T: Copy>(
        &self,
        local: &[T],
        destination: &mut [T],
    ) -> Result<(), IncidenceError> {
        self.coefficient_lengths(destination.len(), local.len())?;
        if self.layout.is_identity() {
            destination.copy_from_slice(local);
            return Ok(());
        }
        let mut i = 0;
        for q in 0..3 {
            let offset = self.layout.topology.topology().offsets()[q];
            self.factor_levels(q).unwrap().for_each(|id| {
                destination[offset + id] = local[i];
                i += 1;
            });
        }
        Ok(())
    }
    /// Gather arbitrary tuple values in original canonical order, preserving bits.
    pub fn gather_tuple_values<T: Copy>(
        &self,
        source: &[T],
        local: &mut [T],
    ) -> Result<(), IncidenceError> {
        self.tuple_lengths(source.len(), local.len())?;
        if self.layout.is_identity() {
            local.copy_from_slice(source);
            return Ok(());
        }
        let mut i = 0;
        self.tuple_ids().for_each(|id| {
            local[i] = source[id];
            i += 1;
        });
        Ok(())
    }
    /// Scatter this component's tuple values; no numerical validation is implied.
    pub fn scatter_tuple_values<T: Copy>(
        &self,
        local: &[T],
        destination: &mut [T],
    ) -> Result<(), IncidenceError> {
        self.tuple_lengths(destination.len(), local.len())?;
        if self.layout.is_identity() {
            destination.copy_from_slice(local);
            return Ok(());
        }
        let mut i = 0;
        self.tuple_ids().for_each(|id| {
            destination[id] = local[i];
            i += 1;
        });
        Ok(())
    }
    fn coefficient_lengths(&self, global: usize, local: usize) -> Result<(), IncidenceError> {
        length(
            "component global coefficients",
            self.layout.topology.topology().total_levels(),
            global,
        )?;
        length("component local coefficients", self.dimension(), local)
    }
    fn tuple_lengths(&self, global: usize, local: usize) -> Result<(), IncidenceError> {
        length(
            "component global tuple values",
            self.layout.topology.topology().tuple_count(),
            global,
        )?;
        length("component local tuple values", self.tuple_count(), local)
    }
}

/// Temporary inverse codes for constant-time recoding of every local tuple.
///
/// Drop after materializing local roots; coefficient/weight permutation only
/// needs the borrowed layout. The single-component path has no inverse array.
///
/// The inverse cannot outlive its exact partition owner:
/// ```compile_fail
/// use multiway_incidence::{PreparedComponentLayout, PreparedComponentRecoding,
///     PreparedThreeWayTopology, PreparedHierarchyBudget};
/// let t = PreparedThreeWayTopology::try_from_collapsed([1;3], &[[0;3]]).unwrap();
/// let inverse = {
///     let layout = PreparedComponentLayout::try_new(&t, PreparedHierarchyBudget::UNLIMITED).unwrap();
///     PreparedComponentRecoding::try_new(&layout, PreparedHierarchyBudget::UNLIMITED).unwrap()
/// };
/// assert_eq!(inverse.retained_payload_bytes().unwrap(), 0);
/// ```
#[derive(Debug)]
pub struct PreparedComponentRecoding<'layout, 'topology> {
    layout: &'layout PreparedComponentLayout<'topology>,
    inverse: Vec<u32>,
    setup_peak_payload_bound: usize,
}
impl<'layout, 'topology> PreparedComponentRecoding<'layout, 'topology> {
    /// Admit the source, partition, temporary inverse and other live state.
    pub fn try_new(
        layout: &'layout PreparedComponentLayout<'topology>,
        budget: PreparedHierarchyBudget,
    ) -> Result<Self, IncidenceError> {
        Self::build_with(layout, budget, &mut |_| Ok(()))
    }
    /// Requested exclusive inverse array, released before later numerical setup.
    pub fn required_payload_bytes(
        layout: &PreparedComponentLayout<'_>,
    ) -> Result<usize, IncidenceError> {
        array_bytes::<u32>(if layout.is_identity() {
            0
        } else {
            layout.topology.topology().total_levels()
        })
    }
    /// Source and partition are counted once, alongside other accumulating owners.
    pub fn setup_payload_bound(
        layout: &PreparedComponentLayout<'_>,
        budget: PreparedHierarchyBudget,
    ) -> Result<usize, IncidenceError> {
        sum_bytes(&[
            layout.topology.retained_payload_bytes()?,
            layout.retained_payload_bytes()?,
            Self::required_payload_bytes(layout)?,
            budget.additional_live_payload_bytes,
        ])
    }
    fn build_with<F>(
        layout: &'layout PreparedComponentLayout<'topology>,
        budget: PreparedHierarchyBudget,
        before: &mut F,
    ) -> Result<Self, IncidenceError>
    where
        F: FnMut(&'static str) -> Result<(), IncidenceError>,
    {
        let required = Self::setup_payload_bound(layout, budget)?;
        admit(required, budget)?;
        let count = if layout.is_identity() {
            0
        } else {
            layout.topology.topology().total_levels()
        };
        let mut inverse = reserve(count, "component inverse recoding", before)?;
        inverse.resize(count, 0u32);
        if !layout.is_identity() {
            for c in 0..layout.component_count() {
                let view = layout.component(c).unwrap();
                for q in 0..3 {
                    let offset = layout.topology.topology().offsets()[q];
                    let mut i = 0;
                    view.factor_levels(q).unwrap().for_each(|id| {
                        inverse[offset + id] = i;
                        i += 1;
                    });
                }
            }
        }
        let mut result = Self {
            layout,
            inverse,
            setup_peak_payload_bound: required,
        };
        let actual = sum_bytes(&[
            layout.topology.retained_payload_bytes()?,
            layout.retained_payload_bytes()?,
            result.retained_payload_bytes()?,
            budget.additional_live_payload_bytes,
        ])?;
        admit(actual, budget)?;
        result.setup_peak_payload_bound = required.max(actual);
        Ok(result)
    }
    /// Exact borrowed partition owner.
    pub const fn layout(&self) -> &'layout PreparedComponentLayout<'topology> {
        self.layout
    }
    /// Actual exclusive temporary inverse capacity.
    pub fn retained_payload_bytes(&self) -> Result<usize, IncidenceError> {
        array_bytes::<u32>(self.inverse.capacity())
    }
    /// Largest admitted requested or actual-capacity setup payload.
    pub const fn setup_peak_payload_bound(&self) -> usize {
        self.setup_peak_payload_bound
    }
    /// Recode local keys after validating component ID and complete output length.
    ///
    /// Monotone factor-local recoding preserves sorted unique canonical keys.
    /// The caller owns the output and must charge it as additional live payload
    /// when admitting simultaneously live setup stages.
    pub fn write_keys_into(
        &self,
        component: usize,
        output: &mut [[u32; 3]],
    ) -> Result<(), IncidenceError> {
        let view =
            self.layout
                .component(component)
                .ok_or(IncidenceError::ComponentIndexOutOfBounds {
                    component,
                    count: self.layout.component_count(),
                })?;
        length("component tuple keys", view.tuple_count(), output.len())?;
        if self.layout.is_identity() {
            output.copy_from_slice(self.layout.topology.topology().tuples());
            return Ok(());
        }
        let source = self.layout.topology.topology();
        let mut i = 0;
        view.tuple_ids().for_each(|id| {
            let key = source.tuples()[id];
            output[i] =
                core::array::from_fn(|q| self.inverse[source.offsets()[q] + key[q] as usize]);
            i += 1;
        });
        Ok(())
    }
}
fn narrow(count: usize) -> bool {
    count
        .checked_sub(1)
        .is_none_or(|id| u32::try_from(id).is_ok())
}
fn add(a: usize, b: usize) -> Result<usize, IncidenceError> {
    a.checked_add(b).ok_or(IncidenceError::DimensionOverflow {
        context: "component layout",
    })
}
fn length(context: &'static str, expected: usize, actual: usize) -> Result<(), IncidenceError> {
    if expected == actual {
        Ok(())
    } else {
        Err(dimension(context, expected, actual))
    }
}
fn admit(required: usize, budget: PreparedHierarchyBudget) -> Result<(), IncidenceError> {
    if required > budget.maximum_payload_bytes {
        Err(IncidenceError::HierarchyBudgetExceeded {
            required,
            budget: budget.maximum_payload_bytes,
        })
    } else {
        Ok(())
    }
}
fn partition_bytes(t: &PreparedThreeWayTopology, compact: bool) -> Result<usize, IncidenceError> {
    let c = t.component_factor_sizes().len();
    if c == 1 {
        return Ok(0);
    }
    let base = add(c, 1)?;
    let e = t.topology().tuple_count();
    sum_bytes(&[
        array_bytes::<usize>(add(base, base)?)?,
        array_bytes::<u32>(t.topology().total_levels())?,
        if compact {
            array_bytes::<u32>(e)?
        } else {
            array_bytes::<usize>(e)?
        },
    ])
}
fn setup_bytes(
    t: &PreparedThreeWayTopology,
    budget: PreparedHierarchyBudget,
    compact: bool,
) -> Result<usize, IncidenceError> {
    sum_bytes(&[
        t.retained_payload_bytes()?,
        partition_bytes(t, compact)?,
        array_bytes::<usize>(if t.component_factor_sizes().len() == 1 {
            0
        } else {
            t.component_factor_sizes().len()
        })?,
        budget.additional_live_payload_bytes,
    ])
}

#[cfg(test)]
#[path = "component_layout_tests.rs"]
mod tests;
