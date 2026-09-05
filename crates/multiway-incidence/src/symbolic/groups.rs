//! Common canonical grouping for coarse tuples and pair endpoints.

use crate::{
    IncidenceError,
    construction::{array_bytes, reserve, sum_bytes},
};

/// Deterministic partition of canonical fine-tuple IDs into mapped-key groups.
///
/// These are unique fine-tuple IDs, not original observation rows. Each group
/// contains increasing IDs, preserving the source order needed for a later
/// deterministic numerical reduction. This type neither sums nor retains weights.
/// A borrowed group view has no independent owner/generation authorization.
#[derive(Debug)]
pub struct TupleMergeGroups {
    source_to_group: Vec<usize>,
    grouped_sources: Vec<usize>,
    offsets: Vec<usize>,
}

impl TupleMergeGroups {
    /// Mapped-key ID for every canonical fine tuple.
    #[must_use]
    pub fn source_to_group(&self) -> &[usize] {
        &self.source_to_group
    }

    /// Canonical fine-tuple IDs grouped by mapped key and increasing within groups.
    #[must_use]
    pub fn grouped_sources(&self) -> &[usize] {
        &self.grouped_sources
    }

    /// Group boundaries; length is mapped-key count plus one.
    #[must_use]
    pub fn offsets(&self) -> &[usize] {
        &self.offsets
    }

    /// Exclusive array payload by actual capacity, excluding inline descriptors.
    pub fn retained_payload_bytes(&self) -> Result<usize, IncidenceError> {
        sum_bytes(&[
            array_bytes::<usize>(self.source_to_group.capacity())?,
            array_bytes::<usize>(self.grouped_sources.capacity())?,
            array_bytes::<usize>(self.offsets.capacity())?,
        ])
    }

    pub(super) fn scatter<T: Copy>(
        &self,
        values: &[T],
        out: &mut [T],
    ) -> Result<(), IncidenceError> {
        let expected = self.offsets.len() - 1;
        if values.len() != expected {
            return Err(crate::error::dimension(
                "symbolic scatter groups",
                expected,
                values.len(),
            ));
        }
        if out.len() != self.source_to_group.len() {
            return Err(crate::error::dimension(
                "symbolic scatter source",
                self.source_to_group.len(),
                out.len(),
            ));
        }
        for (output, &group) in out.iter_mut().zip(&self.source_to_group) {
            *output = values[group];
        }
        Ok(())
    }
}

pub(super) fn setup_bound<K>(count: usize) -> Result<usize, IncidenceError> {
    if count == 0 {
        return Err(IncidenceError::EmptyProblem);
    }
    let offsets = count
        .checked_add(1)
        .ok_or(IncidenceError::DimensionOverflow {
            context: "symbolic group offsets",
        })?;
    sum_bytes(&[
        array_bytes::<K>(count)?,
        array_bytes::<usize>(count)?,
        array_bytes::<usize>(count)?,
        array_bytes::<usize>(offsets)?,
    ])
}

pub(super) fn build<K, Key, F>(
    count: usize,
    key_of: Key,
    before: &mut F,
) -> Result<(Vec<K>, TupleMergeGroups), IncidenceError>
where
    K: Copy + Ord,
    Key: Fn(usize) -> K,
    F: FnMut(&'static str) -> Result<(), IncidenceError>,
{
    setup_bound::<K>(count)?;
    let mut order = reserve(count, "symbolic source order", before)?;
    order.extend(0..count);
    // Original canonical source ID breaks all ties; no stable-sort scratch is needed.
    order.sort_unstable_by_key(|&index| (key_of(index), index));
    let unique = 1 + order
        .windows(2)
        .filter(|pair| key_of(pair[0]) != key_of(pair[1]))
        .count();
    let mut keys = reserve(unique, "symbolic mapped keys", before)?;
    let mut source_to_group = reserve(count, "symbolic source map", before)?;
    source_to_group.resize(count, 0);
    let mut offsets = reserve(unique + 1, "symbolic group offsets", before)?;
    for (position, &index) in order.iter().enumerate() {
        let key = key_of(index);
        if keys.last() != Some(&key) {
            keys.push(key);
            offsets.push(position);
        }
        source_to_group[index] = keys.len() - 1;
    }
    offsets.push(count);
    Ok((
        keys,
        TupleMergeGroups {
            source_to_group,
            grouped_sources: order,
            offsets,
        },
    ))
}
