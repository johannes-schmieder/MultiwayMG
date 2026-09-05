//! One component-discovery implementation, independent of projection identities.

use super::{find_root, union_min_root};
use crate::{IncidenceError, ThreeWayTopology, construction::reserve};

#[derive(Debug)]
pub(crate) struct Partition {
    pub(crate) labels: Vec<usize>,
    pub(crate) factor_sizes: Vec<[usize; 3]>,
}

pub(crate) fn build<F>(
    topology: &ThreeWayTopology,
    before: &mut F,
) -> Result<Partition, IncidenceError>
where
    F: FnMut(&'static str) -> Result<(), IncidenceError>,
{
    let dimension = topology.total_levels();
    let mut labels = reserve(dimension, "component roots", before)?;
    labels.extend(0..dimension);
    for tuple in topology.tuples() {
        let a = topology.global_index(0, tuple[0]);
        let b = topology.global_index(1, tuple[1]);
        let c = topology.global_index(2, tuple[2]);
        union_min_root(&mut labels, a, b);
        union_min_root(&mut labels, a, c);
    }
    for vertex in 0..dimension {
        labels[vertex] = find_root(&mut labels, vertex);
    }
    let count = labels
        .iter()
        .enumerate()
        .filter(|&(vertex, root)| vertex == *root)
        .count();
    let mut root_to_label = reserve(dimension, "component root labels", before)?;
    root_to_label.resize(dimension, usize::MAX);
    let mut factor_sizes = reserve(count, "component factor sizes", before)?;
    factor_sizes.resize(count, [0; 3]);
    let offsets = topology.offsets();
    let mut next = 0;
    // All roots were compressed above. Relabel the same array without chasing
    // a parent after an earlier root coordinate has been overwritten.
    for (vertex, root) in labels.iter_mut().enumerate() {
        let label = if root_to_label[*root] == usize::MAX {
            let label = next;
            root_to_label[*root] = label;
            next += 1;
            label
        } else {
            root_to_label[*root]
        };
        *root = label;
        let factor = if vertex < offsets[1] {
            0
        } else if vertex < offsets[2] {
            1
        } else {
            2
        };
        factor_sizes[label][factor] += 1;
    }
    Ok(Partition {
        labels,
        factor_sizes,
    })
}
