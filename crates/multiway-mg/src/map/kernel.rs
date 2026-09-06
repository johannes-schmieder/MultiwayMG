//! Shared ordered scalar triangular sweeps; projections are supplied by callers.
use multiway_incidence::ThreeWayTopology;
pub(super) struct MapData<'a> {
    pub(super) topology: &'a ThreeWayTopology,
    pub(super) weights: &'a [f64],
    pub(super) diagonal: &'a [f64],
}
pub(super) fn sweep(
    data: MapData<'_>,
    compatible_rhs: &[f64],
    forward: &mut [f64],
    middle: &mut [f64],
    solution: &mut [f64],
) {
    let MapData {
        topology,
        weights,
        diagonal,
    } = data;
    let offsets = topology.offsets();
    forward.fill(0.0);

    for factor in 0..3 {
        let start = offsets[factor];
        let end = offsets[factor + 1];
        forward[start..end].copy_from_slice(&compatible_rhs[start..end]);
        for (&tuple, &weight) in topology.tuples().iter().zip(weights) {
            let target = topology.global_index(factor, tuple[factor]);
            let mut coupling = 0.0;
            for previous in 0..factor {
                coupling = forward[topology.global_index(previous, tuple[previous])]
                    .mul_add(weight, coupling);
            }
            forward[target] -= coupling;
        }
        for index in start..end {
            forward[index] /= diagonal[index];
        }
    }

    for ((middle, &value), &degree) in middle.iter_mut().zip(forward.iter()).zip(diagonal) {
        *middle = value * degree;
    }
    solution.fill(0.0);
    for factor in (0..3).rev() {
        let start = offsets[factor];
        let end = offsets[factor + 1];
        solution[start..end].copy_from_slice(&middle[start..end]);
        for (&tuple, &weight) in topology.tuples().iter().zip(weights) {
            let target = topology.global_index(factor, tuple[factor]);
            let mut coupling = 0.0;
            for following in (factor + 1)..3 {
                coupling = solution[topology.global_index(following, tuple[following])]
                    .mul_add(weight, coupling);
            }
            solution[target] -= coupling;
        }
        for index in start..end {
            solution[index] /= diagonal[index];
        }
    }
}
