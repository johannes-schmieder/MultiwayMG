//! Compact three-factor tuple topology.

use core::ops::Range;

use crate::IncidenceError;

/// Immutable topology for a three-way categorical incidence matrix.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ThreeWayTopology {
    level_counts: [usize; 3],
    offsets: [usize; 4],
    tuples: Vec<[u32; 3]>,
}

impl ThreeWayTopology {
    /// Validate and construct a topology.
    pub fn new(level_counts: [usize; 3], tuples: Vec<[u32; 3]>) -> Result<Self, IncidenceError> {
        for (factor, &count) in level_counts.iter().enumerate() {
            if count == 0 {
                return Err(IncidenceError::EmptyFactor { factor });
            }
            if count > u32::MAX as usize {
                return Err(IncidenceError::LevelCountTooWide { factor, count });
            }
        }

        let offset_1 = level_counts[0];
        let offset_2 =
            offset_1
                .checked_add(level_counts[1])
                .ok_or(IncidenceError::DimensionOverflow {
                    context: "topology factor offsets",
                })?;
        let total =
            offset_2
                .checked_add(level_counts[2])
                .ok_or(IncidenceError::DimensionOverflow {
                    context: "topology total levels",
                })?;
        let offsets = [0, offset_1, offset_2, total];

        for (tuple_index, tuple) in tuples.iter().enumerate() {
            for factor in 0..3 {
                if tuple[factor] as usize >= level_counts[factor] {
                    return Err(IncidenceError::TupleOutOfBounds {
                        tuple_index,
                        factor,
                        level: tuple[factor],
                        level_count: level_counts[factor],
                    });
                }
            }
        }

        Ok(Self {
            level_counts,
            offsets,
            tuples,
        })
    }

    /// Number of levels in each factor.
    #[must_use]
    pub const fn level_counts(&self) -> [usize; 3] {
        self.level_counts
    }

    /// Prefix offsets for the three factor blocks, including the final total.
    #[must_use]
    pub const fn offsets(&self) -> [usize; 4] {
        self.offsets
    }

    /// Total number of coefficient coordinates.
    #[must_use]
    pub const fn total_levels(&self) -> usize {
        self.offsets[3]
    }

    /// Number of unique tuples.
    #[must_use]
    pub fn tuple_count(&self) -> usize {
        self.tuples.len()
    }

    /// Canonically sorted unique tuples.
    #[must_use]
    pub fn tuples(&self) -> &[[u32; 3]] {
        &self.tuples
    }

    /// Global coefficient range occupied by one factor.
    #[must_use]
    pub fn factor_range(&self, factor: usize) -> Range<usize> {
        assert!(factor < 3, "factor index must be below three");
        self.offsets[factor]..self.offsets[factor + 1]
    }

    /// Convert a factor-local level to its global coefficient index.
    #[must_use]
    pub fn global_index(&self, factor: usize, level: u32) -> usize {
        debug_assert!(factor < 3);
        debug_assert!((level as usize) < self.level_counts[factor]);
        self.offsets[factor] + level as usize
    }
}

impl ThreeWayTopology {
    /// Retained tuple-array payload, using capacity rather than length.
    ///
    /// Excludes this inline object and allocator overhead.
    pub fn retained_payload_bytes(&self) -> Result<usize, IncidenceError> {
        self.tuples
            .capacity()
            .checked_mul(core::mem::size_of::<[u32; 3]>())
            .ok_or(IncidenceError::DimensionOverflow {
                context: "topology payload",
            })
    }
}

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
