//! Independent arithmetic references and numerical-generation invalidation.
use multiway_incidence::{
    IncidenceError, PreparedThreeWayTopology, ThreeWayProblem, ThreeWayWeightFrame,
    WeightFrameInput, WeightFrameInputKind, WeightFramePayloadBudget,
};
use std::collections::BTreeMap;

fn bits(actual: &[f64], expected: &[f64]) {
    assert_eq!(actual.len(), expected.len());
    for (&a, &b) in actual.iter().zip(expected) { assert_eq!(a.to_bits(), b.to_bits()); }
}

// Independent transcription of the pre-frame compensated arithmetic. Neither
// frame construction nor its newly shared degree kernel is the test oracle.
fn add(state: &mut (f64, f64), value: f64) {
    let next = state.0 + value;
    state.1 += if state.0.abs() >= value.abs() {
        (state.0 - next) + value
    } else { (value - next) + state.0 };
    state.0 = next;
}

fn check_observations(counts: [usize; 3], rows: &[[u32; 3]], weights: &[f64]) {
    let topology = PreparedThreeWayTopology::try_from_observations(counts, rows).unwrap();
    let frame = ThreeWayWeightFrame::try_new(&topology, WeightFrameInput::Observations(weights)).unwrap();
    let fresh = ThreeWayProblem::from_observations(counts, rows, weights).unwrap();
    let mut grouped = BTreeMap::<[u32; 3], (f64, f64)>::new();
    for (&row, &value) in rows.iter().zip(weights) { add(grouped.entry(row).or_default(), value); }
    let expected: Vec<_> = grouped.values().map(|&(sum, correction)| sum + correction).collect();
    bits(frame.weights(), &expected);
    bits(frame.weights(), fresh.weights());
    bits(frame.square_root_weights(), fresh.square_root_weights());
    let mut degree = vec![(0.0, 0.0); counts.iter().sum()];
    for (&tuple, &value) in topology.topology().tuples().iter().zip(&expected) {
        for factor in 0..3 {
            add(&mut degree[topology.topology().global_index(factor, tuple[factor])], value);
        }
    }
    let degree: Vec<_> = degree.into_iter().map(|(sum, correction)| sum + correction).collect();
    bits(frame.diagonal(), &degree);
    bits(frame.diagonal(), fresh.diagonal());
    let from_tuples = ThreeWayWeightFrame::try_new(&topology, WeightFrameInput::Tuples(&expected)).unwrap();
    bits(frame.weights(), from_tuples.weights());
    bits(frame.diagonal(), from_tuples.diagonal());
    assert_ne!(frame.binding(), from_tuples.binding());
    assert_eq!(frame.topology_binding(), from_tuples.topology_binding());
    assert!(std::ptr::eq(frame.topology(), &topology));
    assert_eq!(frame.validation_report().input_kind, WeightFrameInputKind::Observations);
    assert_eq!(frame.validation_report().input_count, rows.len());
    assert_eq!(frame.validation_report().tuple_count, expected.len());
    assert_eq!(frame.validation_report().dimension, degree.len());
    assert_eq!(frame.validation_report().component_count, topology.component_factor_sizes().len());
    for (component, range) in frame.component_ranges().iter().enumerate() {
        let component_weights: Vec<_> = topology.topology().tuples().iter().zip(&expected)
            .filter_map(|(tuple, &weight)| (topology.component_labels()[tuple[0] as usize] == component).then_some(weight)).collect();
        let component_degrees: Vec<_> = topology.component_labels().iter().zip(&degree)
            .filter_map(|(&label, &value)| (label == component).then_some(value)).collect();
        assert_eq!(range.minimum_tuple_weight, component_weights.iter().copied().fold(f64::INFINITY, f64::min));
        assert_eq!(range.maximum_tuple_weight, component_weights.iter().copied().fold(0.0, f64::max));
        assert_eq!(range.minimum_degree, component_degrees.iter().copied().fold(f64::INFINITY, f64::min));
        assert_eq!(range.maximum_degree, component_degrees.iter().copied().fold(0.0, f64::max));
    }
}

#[test]
fn duplicate_order_and_all_small_valid_supports_match_independent_reference() {
    let universe: Vec<_> = (0..2).flat_map(|i| (0..2).flat_map(move |j| (0..2).map(move |k| [i,j,k]))).collect();
    for mask in 1..256 {
        let rows: Vec<_> = universe.iter().enumerate().filter_map(|(i, &t)| ((mask >> i) & 1 == 1).then_some(t)).collect();
        if (0..3).any(|f| (0..2).any(|l| !rows.iter().any(|t| t[f] == l))) { continue; }
        let mut duplicated = rows.clone();
        duplicated.extend(rows.iter().rev());
        duplicated.extend_from_slice(&rows);
        duplicated.rotate_left(1);
        let weights: Vec<_> = (0..duplicated.len()).map(|i| [1.0e16, 1.0, 1.0, 0.25, 1.0e-200][i % 5]).collect();
        check_observations([2; 3], &duplicated, &weights);
    }
    check_observations([1; 3], &[[0; 3]; 4], &[1.0e16, 1.0, 1.0, 0.25]);
    check_observations([2, 3, 1], &[[0,0,0],[1,1,0],[0,2,0],[1,1,0]], &[0.5, 1.0e10, 7.25, 0.125]);
}

#[test]
fn unit_observations_and_unit_tuples_are_explicitly_different() {
    let rows = [[1; 3], [0; 3], [1; 3]];
    let topology = PreparedThreeWayTopology::try_from_observations([2; 3], &rows).unwrap();
    let unit_obs = ThreeWayWeightFrame::try_new(&topology, WeightFrameInput::UnitObservations).unwrap();
    let ones_obs = ThreeWayWeightFrame::try_new(&topology, WeightFrameInput::Observations(&[1.0; 3])).unwrap();
    let unit_tuples = ThreeWayWeightFrame::try_new(&topology, WeightFrameInput::UnitTuples).unwrap();
    let ones_tuples = ThreeWayWeightFrame::try_new(&topology, WeightFrameInput::Tuples(&[1.0; 2])).unwrap();
    bits(unit_obs.weights(), &[1.0, 2.0]);
    bits(unit_obs.diagonal(), &[1.0, 2.0, 1.0, 2.0, 1.0, 2.0]);
    bits(unit_obs.weights(), ones_obs.weights());
    bits(unit_obs.square_root_weights(), ones_obs.square_root_weights());
    bits(unit_obs.diagonal(), ones_obs.diagonal());
    bits(unit_tuples.weights(), &[1.0, 1.0]);
    bits(unit_tuples.diagonal(), ones_tuples.diagonal());
    assert_eq!(unit_obs.validation_report().input_kind, WeightFrameInputKind::UnitObservations);
    assert_eq!(unit_tuples.validation_report().input_kind, WeightFrameInputKind::UnitTuples);
    let collapsed = PreparedThreeWayTopology::try_from_collapsed([2; 3], &[[0; 3], [1; 3]]).unwrap();
    for input in [WeightFrameInput::UnitObservations, WeightFrameInput::Observations(&[1.0; 2])] {
        assert!(matches!(ThreeWayWeightFrame::try_new(&collapsed, input), Err(IncidenceError::WeightFrameObservationLayoutRequired)));
    }
    let tuples = ThreeWayWeightFrame::try_new(&collapsed, WeightFrameInput::UnitTuples).unwrap();
    bits(tuples.weights(), ones_tuples.weights());
}

#[test]
fn generation_checks_distinguish_equal_and_changed_frames_before_output_mutation() {
    let topology = PreparedThreeWayTopology::try_from_collapsed([1; 3], &[[0; 3]]).unwrap();
    let foreign = PreparedThreeWayTopology::try_from_collapsed([1; 3], &[[0; 3]]).unwrap();
    let first = ThreeWayWeightFrame::try_new(&topology, WeightFrameInput::Tuples(&[1.0])).unwrap();
    let equal = ThreeWayWeightFrame::try_new(&topology, WeightFrameInput::Tuples(&[1.0])).unwrap();
    let changed = ThreeWayWeightFrame::try_new(&topology, WeightFrameInput::Tuples(&[2.0])).unwrap();
    let token = first.binding();
    assert_eq!(token, first.binding());
    assert_ne!(token, equal.binding());
    assert_ne!(token, changed.binding());
    let mut output = [23.0];
    for target in [&equal, &changed] {
        assert!(matches!(target.copy_weights_into(&topology, token, &mut output), Err(IncidenceError::WeightFrameBindingMismatch)));
        assert_eq!(output, [23.0]);
    }
    assert!(matches!(first.copy_weights_into(&foreign, token, &mut output), Err(IncidenceError::TopologyBindingMismatch)));
    assert!(first.copy_weights_into(&topology, token, &mut output[..0]).is_err());
    assert_eq!(output, [23.0]);
    first.copy_weights_into(&topology, token, &mut output).unwrap();
    assert_eq!(output, [1.0]);
    bits(first.diagonal(), &[1.0; 3]);
    bits(changed.diagonal(), &[2.0; 3]);
}

#[test]
fn finite_positive_extremes_are_preserved_but_duplicate_and_degree_overflow_reject() {
    let topology = PreparedThreeWayTopology::try_from_observations([1; 3], &[[0; 3]; 2]).unwrap();
    for weight in [f64::from_bits(1), f64::MIN_POSITIVE, 1.0e-200, 1.0e200, f64::MAX] {
        let frame = ThreeWayWeightFrame::try_new(&topology, WeightFrameInput::Tuples(&[weight])).unwrap();
        bits(frame.weights(), &[weight]);
        bits(frame.diagonal(), &[weight; 3]);
        assert!(frame.square_root_weights()[0].is_finite() && frame.square_root_weights()[0] > 0.0);
    }
    for invalid in [0.0, -0.0, -1.0, f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        assert!(matches!(ThreeWayWeightFrame::try_new(&topology, WeightFrameInput::Observations(&[1.0, invalid])), Err(IncidenceError::InvalidWeight { tuple_index: 1, .. })));
        assert!(matches!(ThreeWayWeightFrame::try_new(&topology, WeightFrameInput::Tuples(&[invalid])), Err(IncidenceError::InvalidWeight { tuple_index: 0, .. })));
    }
    assert!(matches!(ThreeWayWeightFrame::try_new(&topology, WeightFrameInput::Observations(&[f64::MAX; 2])), Err(IncidenceError::InvalidCollapsedWeight { .. })));
    // Distinct finite tuple totals can still overflow a shared factor's degree.
    let shared = PreparedThreeWayTopology::try_from_collapsed([1, 2, 2], &[[0,0,0], [0,1,1]]).unwrap();
    assert!(matches!(ThreeWayWeightFrame::try_new(&shared, WeightFrameInput::Tuples(&[f64::MAX; 2])), Err(IncidenceError::InvalidWeightedDegree { factor: 0, level: 0, .. })));
    // No thresholding or overflow-prone max/min ratio is needed for diagnostics.
    let disconnected = PreparedThreeWayTopology::try_from_collapsed([2; 3], &[[0; 3], [1; 3]]).unwrap();
    let frame = ThreeWayWeightFrame::try_new(&disconnected, WeightFrameInput::Tuples(&[f64::from_bits(1), f64::MAX])).unwrap();
    bits(frame.weights(), &[f64::from_bits(1), f64::MAX]);
}

#[test]
fn checked_budget_counts_old_new_overlap_and_input_storage() {
    let topology = PreparedThreeWayTopology::try_from_collapsed([1; 3], &[[0; 3]]).unwrap();
    let old = ThreeWayWeightFrame::try_new(&topology, WeightFrameInput::UnitTuples).unwrap();
    let retained = old.retained_payload_bytes().unwrap();
    let input = WeightFrameInput::Tuples(&[3.0]);
    let report = ThreeWayWeightFrame::setup_payload_report(&topology, input, retained).unwrap();
    assert_eq!(report.input_payload_bytes, 8);
    assert_eq!(report.topology_payload_bytes, topology.retained_payload_bytes().unwrap());
    assert_eq!(report.additional_live_payload_bytes, retained);
    assert_eq!(report.new_arrays_payload_bytes, retained + 3 * 8);
    assert_eq!(report.total_payload_bytes, report.topology_payload_bytes + 8 + report.new_arrays_payload_bytes + retained);
    let build = |limit| ThreeWayWeightFrame::try_new_with_budget(&topology, input, WeightFramePayloadBudget { maximum_payload_bytes: limit, additional_live_payload_bytes: retained });
    assert!(matches!(build(report.total_payload_bytes - 1), Err(IncidenceError::WeightFrameBudgetExceeded { .. })));
    let new = build(report.total_payload_bytes).unwrap();
    bits(old.weights(), &[1.0]);
    bits(new.weights(), &[3.0]);
    let alias = ThreeWayWeightFrame::setup_payload_report(&topology, WeightFrameInput::Tuples(old.weights()), retained).unwrap();
    assert_eq!(alias.total_payload_bytes, report.total_payload_bytes); // deliberately conservative alias counting
    assert!(ThreeWayWeightFrame::setup_payload_report(&topology, input, usize::MAX).is_err());
    assert!(ThreeWayWeightFrame::try_new(&topology, WeightFrameInput::Tuples(&[])).is_err());
    assert!(ThreeWayWeightFrame::try_new(&topology, WeightFrameInput::Tuples(&[1.0, 2.0])).is_err());
}

#[test]
fn topology_borrows_and_numerical_frames_can_be_shared_across_threads() {
    fn send_sync<T: Send + Sync>() {}
    send_sync::<ThreeWayWeightFrame<'_>>();
    let topology = PreparedThreeWayTopology::try_from_collapsed([1; 3], &[[0; 3]]).unwrap();
    let first = ThreeWayWeightFrame::try_new(&topology, WeightFrameInput::UnitTuples).unwrap();
    std::thread::scope(|scope| {
        let a = scope.spawn(|| {
            let second = ThreeWayWeightFrame::try_new(&topology, WeightFrameInput::Tuples(&[2.0])).unwrap();
            assert_ne!(first.binding(), second.binding());
            second.diagonal()[0]
        });
        let b = scope.spawn(|| {
            let mut output = [0.0];
            first.copy_weights_into(&topology, first.binding(), &mut output).unwrap();
            output[0]
        });
        assert_eq!(a.join().unwrap(), 2.0);
        assert_eq!(b.join().unwrap(), 1.0);
    });
}
