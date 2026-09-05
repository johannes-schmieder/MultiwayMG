//! Measured numerical-frame lifetimes, overlap admission and allocation-free reads.
use super::{GLOBAL, Result, equal_bits, no_events};
use multiway_incidence::{
    IncidenceError, PreparedThreeWayTopology, ThreeWayWeightFrame, WeightFrameInput,
    WeightFramePayloadBudget,
};
use std::hint::black_box;

fn check(topology: &PreparedThreeWayTopology, input: WeightFrameInput<'_>) -> Result<()> {
    let old = ThreeWayWeightFrame::try_new(topology, input)?;
    let old_bytes = old.retained_payload_bytes()?;
    let report = ThreeWayWeightFrame::setup_payload_report(topology, input, old_bytes)?;
    let budget = |maximum_payload_bytes| WeightFramePayloadBudget {
        maximum_payload_bytes,
        additional_live_payload_bytes: old_bytes,
    };
    let before = GLOBAL.stats();
    let error = ThreeWayWeightFrame::try_new_with_budget(topology, input, budget(report.total_payload_bytes - 1)).unwrap_err();
    no_events(GLOBAL.stats() - before);
    assert!(matches!(error, IncidenceError::WeightFrameBudgetExceeded { required, .. } if required == report.total_payload_bytes));
    let before = GLOBAL.stats();
    let frame = black_box(ThreeWayWeightFrame::try_new_with_budget(topology, input, budget(report.total_payload_bytes))?);
    let setup = GLOBAL.stats() - before;
    let retained = frame.retained_payload_bytes()?;
    assert_eq!(setup.allocations, 5);
    assert_eq!(setup.reallocations, 0);
    assert_eq!(setup.deallocations, 1);
    assert_eq!(setup.bytes_allocated, report.new_arrays_payload_bytes);
    assert_eq!(setup.bytes_allocated - setup.bytes_deallocated, retained);
    assert_eq!(setup.bytes_deallocated, topology.topology().total_levels() * 8);
    assert_eq!(old.retained_payload_bytes()?, old_bytes);
    assert_ne!(frame.binding(), old.binding());
    equal_bits(frame.weights(), old.weights());
    let mut output = vec![f64::NAN; frame.weights().len()];
    let before = GLOBAL.stats();
    let binding = frame.binding();
    for _ in 0..64 {
        frame.copy_weights_into(topology, binding, black_box(&mut output))?;
        binding.validate_for(&frame)?;
        assert_eq!(frame.retained_payload_bytes()?, retained);
        assert_eq!(ThreeWayWeightFrame::setup_payload_report(topology, input, old_bytes)?, report);
        black_box(frame.square_root_weights());
        black_box(frame.diagonal());
        black_box(frame.component_ranges());
        black_box(frame.validation_report());
    }
    no_events(GLOBAL.stats() - before);
    equal_bits(&output, frame.weights());
    let foreign = PreparedThreeWayTopology::try_from_collapsed(topology.topology().level_counts(), topology.topology().tuples())?;
    let snapshot = output.clone();
    let before = GLOBAL.stats();
    let wrong_frame = frame.copy_weights_into(topology, old.binding(), &mut output).unwrap_err();
    let wrong_topology = frame.copy_weights_into(&foreign, binding, &mut output).unwrap_err();
    let wrong_size = frame.copy_weights_into(topology, binding, &mut output[..0]).unwrap_err();
    no_events(GLOBAL.stats() - before);
    assert!(matches!(wrong_frame, IncidenceError::WeightFrameBindingMismatch));
    assert!(matches!(wrong_topology, IncidenceError::TopologyBindingMismatch));
    assert!(matches!(wrong_size, IncidenceError::DimensionMismatch { .. }));
    equal_bits(&output, &snapshot);
    let before = GLOBAL.stats();
    drop(frame);
    let released = GLOBAL.stats() - before;
    assert_eq!(released.allocations, 0);
    assert_eq!(released.reallocations, 0);
    assert_eq!(released.bytes_deallocated, retained);
    // Rebuild/drop repeatedly while an older generation is still live. Every
    // iteration balances its own allocation ledger, not just its final capacities.
    for _ in 0..16 {
        let before = GLOBAL.stats();
        let fresh = ThreeWayWeightFrame::try_new(topology, input)?;
        equal_bits(fresh.weights(), old.weights());
        drop(fresh);
        let cycle = GLOBAL.stats() - before;
        assert_eq!(cycle.allocations, cycle.deallocations);
        assert_eq!(cycle.bytes_allocated, cycle.bytes_deallocated);
        assert_eq!(cycle.reallocations, 0);
    }
    old.copy_weights_into(topology, old.binding(), &mut output)?;
    println!("weight-frame input={:?} tuples={} new_setup={} retained={retained} old_live={old_bytes} live_budget={} first/repeat/read/copy/reject_allocations=0 release=exact rebuild16=balanced", input.kind(), topology.topology().tuple_count(), report.new_arrays_payload_bytes, report.total_payload_bytes);
    Ok(())
}

fn failed_construction_releases_arrays() -> Result<()> {
    let observed = PreparedThreeWayTopology::try_from_observations([1; 3], &[[0; 3]; 2])?;
    let degree = PreparedThreeWayTopology::try_from_collapsed([1,2,2], &[[0,0,0], [0,1,1]])?;
    for (topology, input, expected_allocations) in [
        (&observed, WeightFrameInput::Observations(&[1.0, f64::NAN]), 0),
        (&observed, WeightFrameInput::Observations(&[f64::MAX; 2]), 1),
        (&degree, WeightFrameInput::Tuples(&[f64::MAX; 2]), 4),
    ] {
        let before = GLOBAL.stats();
        let result = ThreeWayWeightFrame::try_new(topology, input);
        let stats = GLOBAL.stats() - before;
        assert!(result.is_err());
        assert_eq!(stats.allocations, expected_allocations);
        assert_eq!(stats.reallocations, 0);
        assert_eq!(stats.allocations, stats.deallocations);
        assert_eq!(stats.bytes_allocated, stats.bytes_deallocated);
        drop(result);
    }
    println!("weight-frame failed input/duplicate/degree construction allocations=0/1/4 all releases balanced");
    Ok(())
}

pub(super) fn run() -> Result<()> {
    let cases = [
        ([1; 3], vec![[0; 3]; 5]),
        ([2; 3], vec![[1; 3], [0; 3], [1; 3]]),
        ([2,3,4], (0..2).flat_map(|i| (0..3).flat_map(move |j| (0..4).map(move |k| [i,j,k]))).collect()),
        ([3; 3], (0..108).map(|i| [(i%3) as u32, ((i/3)%3) as u32, ((i/9)%3) as u32]).collect()),
    ];
    let mut checked = 0;
    for (counts, rows) in cases {
        let observed = PreparedThreeWayTopology::try_from_observations(counts, &rows)?;
        let collapsed = PreparedThreeWayTopology::try_from_collapsed(counts, observed.topology().tuples())?;
        let observations: Vec<_> = (0..rows.len()).map(|i| 0.25 + (i%11) as f64).collect();
        let tuples: Vec<_> = (0..observed.topology().tuple_count()).map(|i| 0.5 + (i%7) as f64).collect();
        for input in [WeightFrameInput::Observations(&observations), WeightFrameInput::UnitObservations] {
            check(&observed, input)?;
            checked += 1;
        }
        for topology in [&observed, &collapsed] {
            for input in [WeightFrameInput::Tuples(&tuples), WeightFrameInput::UnitTuples] {
                check(topology, input)?;
                checked += 1;
            }
        }
    }
    assert_eq!(checked, 24);
    failed_construction_releases_arrays()?;
    println!("PASS weight-frames cases=24 exact setup/retained/destruction/overlap; allocation-free reads and static rejection");
    Ok(())
}
