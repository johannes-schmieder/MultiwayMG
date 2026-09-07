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
    solution: &mut [f64],
) {
    #[cfg(feature = "profiling")]
    let _profile_span =
        multiway_incidence::profiling::span(multiway_incidence::profiling::Phase::MapSweep);

    let MapData {
        topology,
        weights,
        diagonal,
    } = data;
    let offsets = topology.offsets();

    for factor in 0..3 {
        let start = offsets[factor];
        let end = offsets[factor + 1];
        forward[start..end].copy_from_slice(&compatible_rhs[start..end]);
        // Factor 0 has no previous factor: its coupling is identically zero.
        if factor > 0 {
            for (&tuple, &weight) in topology.tuples().iter().zip(weights) {
                let target = topology.global_index(factor, tuple[factor]);
                let mut coupling = 0.0;
                for previous in 0..factor {
                    coupling = forward[topology.global_index(previous, tuple[previous])]
                        .mul_add(weight, coupling);
                }
                forward[target] -= coupling;
            }
        }
        for index in start..end {
            forward[index] /= diagonal[index];
        }
    }

    // The forward solution dies here. Keep each product rounded/stored before
    // the reverse solve, preserving the original separate-middle arithmetic.
    for (value, &degree) in forward.iter_mut().zip(diagonal) {
        *value *= degree;
    }
    for factor in (0..3).rev() {
        let start = offsets[factor];
        let end = offsets[factor + 1];
        solution[start..end].copy_from_slice(&forward[start..end]);
        // Factor 2 has no following factor. Each factor is copied before read.
        if factor < 2 {
            for (&tuple, &weight) in topology.tuples().iter().zip(weights) {
                let target = topology.global_index(factor, tuple[factor]);
                let mut coupling = 0.0;
                for following in (factor + 1)..3 {
                    coupling = solution[topology.global_index(following, tuple[following])]
                        .mul_add(weight, coupling);
                }
                solution[target] -= coupling;
            }
        }
        for index in start..end {
            solution[index] /= diagonal[index];
        }
    }
}

// Exact per-row traversal: start from the RHS/middle and subtract each tuple
// coupling in original order. Summing couplings first would change rounding.
pub(super) fn sweep_grouped(
    data: MapData<'_>,
    grouping: &multiway_incidence::PreparedTupleGrouping<'_>,
    compatible_rhs: &[f64],
    forward: &mut [f64],
    solution: &mut [f64],
) {
    #[cfg(feature = "profiling")]
    let _profile_span =
        multiway_incidence::profiling::span(multiway_incidence::profiling::Phase::MapSweep);
    let MapData {
        topology,
        weights,
        diagonal,
    } = data;
    debug_assert!(core::ptr::eq(topology, grouping.topology().topology()));
    let offsets = topology.offsets();
    for factor in 0..3 {
        for level in 0..topology.level_counts()[factor] {
            let index = offsets[factor] + level;
            let mut value = compatible_rhs[index];
            if factor > 0 {
                grouping
                    .row(factor, level)
                    .expect("validated MAP row")
                    .for_each(|id| {
                        let tuple = topology.tuples()[id];
                        let weight = weights[id];
                        let mut coupling = 0.0;
                        for previous in 0..factor {
                            coupling = forward[offsets[previous] + tuple[previous] as usize]
                                .mul_add(weight, coupling);
                        }
                        value -= coupling;
                    });
            }
            forward[index] = value / diagonal[index];
        }
    }
    for (value, &degree) in forward.iter_mut().zip(diagonal) {
        *value *= degree;
    }
    for factor in (0..3).rev() {
        for level in 0..topology.level_counts()[factor] {
            let index = offsets[factor] + level;
            let mut value = forward[index];
            if factor < 2 {
                grouping
                    .row(factor, level)
                    .expect("validated MAP row")
                    .for_each(|id| {
                        let tuple = topology.tuples()[id];
                        let weight = weights[id];
                        let mut coupling = 0.0;
                        for following in (factor + 1)..3 {
                            coupling = solution[offsets[following] + tuple[following] as usize]
                                .mul_add(weight, coupling);
                        }
                        value -= coupling;
                    });
            }
            solution[index] = value / diagonal[index];
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use multiway_incidence::ThreeWayProblem;

    // Exact pre-M5 loop from 4fa6401, independently retained for pass/liveness changes.
    fn frozen_sweep(
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

    #[test]
    fn pass_removal_and_in_place_middle_preserve_bits_with_poisoned_scratch() {
        let cases = [
            ([2, 2, 2], vec![[0, 0, 0], [1, 1, 1]]),
            ([2, 3, 4], vec![[0, 0, 0], [0, 1, 1], [0, 2, 3], [1, 2, 2]]),
            ([2, 2, 2], vec![[0, 0, 0], [0, 1, 1], [1, 0, 1], [1, 1, 0]]),
        ];
        for (counts, tuples) in cases {
            for shift in [0, 3, 9] {
                let weights: Vec<_> = (0..tuples.len())
                    .map(|i| 2.0_f64.powi(((i * 7 + shift) % 21) as i32 - 10))
                    .collect();
                let problem =
                    ThreeWayProblem::from_observations(counts, &tuples, &weights).unwrap();
                let grouped_topology =
                    multiway_incidence::PreparedThreeWayTopology::try_from_collapsed(
                        counts,
                        problem.topology().tuples(),
                    )
                    .unwrap();
                let grouping =
                    multiway_incidence::PreparedTupleGrouping::try_new(&grouped_topology).unwrap();
                let n = problem.dimension();
                for scale in [
                    0.0,
                    -0.0,
                    f64::from_bits(1),
                    -f64::from_bits(1),
                    1.0,
                    -2.0,
                    1e100,
                    1e-100,
                ] {
                    let rhs: Vec<_> = (0..n).map(|i| scale * (i as f64 - 2.0)).collect();
                    let mut forward = vec![f64::NAN; n];
                    let mut actual = vec![f64::INFINITY; n];
                    let mut old_forward = vec![f64::NAN; n];
                    let mut old_middle = vec![f64::NAN; n];
                    let mut expected = vec![f64::NAN; n];
                    let data = || MapData {
                        topology: problem.topology(),
                        weights: problem.weights(),
                        diagonal: problem.diagonal(),
                    };
                    frozen_sweep(
                        data(),
                        &rhs,
                        &mut old_forward,
                        &mut old_middle,
                        &mut expected,
                    );
                    for _ in 0..2 {
                        sweep(data(), &rhs, &mut forward, &mut actual);
                        for i in 0..n {
                            assert_eq!(actual[i].to_bits(), expected[i].to_bits(), "output {i}");
                            assert_eq!(forward[i].to_bits(), old_middle[i].to_bits(), "middle {i}");
                        }
                        forward.fill(f64::NAN);
                        actual.fill(f64::INFINITY);
                        sweep_grouped(
                            MapData {
                                topology: grouped_topology.topology(),
                                weights: problem.weights(),
                                diagonal: problem.diagonal(),
                            },
                            &grouping,
                            &rhs,
                            &mut forward,
                            &mut actual,
                        );
                        for i in 0..n {
                            assert_eq!(
                                actual[i].to_bits(),
                                expected[i].to_bits(),
                                "grouped output {i}"
                            );
                            assert_eq!(
                                forward[i].to_bits(),
                                old_middle[i].to_bits(),
                                "grouped middle {i}"
                            );
                        }
                    }
                }
            }
        }
    }
}
