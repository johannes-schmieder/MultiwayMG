//! Exact array lifetime and allocation-free symbolic operations in the existing process.
use super::{GLOBAL, Result, no_events};
use multiway_incidence::{
    IncidenceError, PreparedThreeWayTopology, PreparedTopologySource, ThreeWayProblem,
};
use std::hint::black_box;

pub(super) fn run() -> Result<()> {
    let cases = [
        ([1, 1, 1], vec![[0, 0, 0]; 5]),
        ([2, 2, 2], vec![[1, 1, 1], [0, 0, 0], [1, 1, 1]]),
        (
            [2, 3, 4],
            (0..2)
                .flat_map(|i| (0..3).flat_map(move |j| (0..4).map(move |k| [i, j, k])))
                .collect(),
        ),
        (
            [3, 3, 3],
            (0..108)
                .map(|i| [(i % 3) as u32, ((i / 3) % 3) as u32, ((i / 9) % 3) as u32])
                .collect(),
        ),
    ];
    let mut tested = 0;
    for (counts, rows) in cases {
        let original = ThreeWayProblem::from_observations(counts, &rows, &vec![1.0; rows.len()])?;
        for source in [
            PreparedTopologySource::Observations,
            PreparedTopologySource::Collapsed,
        ] {
            let input = if source == PreparedTopologySource::Observations {
                rows.as_slice()
            } else {
                original.topology().tuples()
            };
            let bound = PreparedThreeWayTopology::setup_payload_bound(counts, input.len(), source)?;
            let build = |budget| match source {
                PreparedTopologySource::Observations => {
                    PreparedThreeWayTopology::try_from_observations_with_budget(
                        counts,
                        black_box(input),
                        budget,
                    )
                }
                PreparedTopologySource::Collapsed => {
                    PreparedThreeWayTopology::try_from_collapsed_with_budget(
                        counts,
                        black_box(input),
                        budget,
                    )
                }
            };
            let before = GLOBAL.stats();
            let error = build(bound - 1).unwrap_err();
            no_events(GLOBAL.stats() - before);
            assert!(matches!(
                error,
                IncidenceError::TopologySetupBudgetExceeded { .. }
            ));
            let before = GLOBAL.stats();
            let prepared = black_box(build(bound)?);
            let setup = GLOBAL.stats() - before;
            let retained = prepared.retained_payload_bytes()?;
            assert_eq!(setup.reallocations, 0);
            assert_eq!(setup.bytes_allocated - setup.bytes_deallocated, retained);
            assert!(setup.bytes_allocated <= bound);
            assert_eq!(
                setup.allocations,
                if source == PreparedTopologySource::Observations {
                    7
                } else {
                    4
                }
            );
            assert_eq!(prepared.topology(), original.topology());
            assert_eq!(prepared.component_labels(), original.components().labels());
            let foreign = build(bound)?;
            let values: Vec<_> = (0..prepared.topology().tuple_count()).collect();
            let mut output = vec![usize::MAX; input.len()];
            let before = GLOBAL.stats();
            let token = prepared.binding();
            for _ in 0..64 {
                prepared.validate_input_layout(counts, black_box(input))?;
                token.validate_for(&prepared)?;
                assert_eq!(prepared.retained_payload_bytes()?, retained);
                prepared.scatter_tuple_values_into(
                    token,
                    black_box(&values),
                    black_box(&mut output),
                )?;
            }
            no_events(GLOBAL.stats() - before);
            for (row, &tuple) in output.iter().enumerate() {
                assert_eq!(prepared.topology().tuples()[tuple], input[row]);
            }
            let snapshot = output.clone();
            let before = GLOBAL.stats();
            let wrong_owner = prepared
                .scatter_tuple_values_into(foreign.binding(), &values, &mut output)
                .unwrap_err();
            let wrong_dimension = prepared
                .scatter_tuple_values_into(token, &values[..values.len() - 1], &mut output)
                .unwrap_err();
            no_events(GLOBAL.stats() - before);
            assert!(matches!(
                wrong_owner,
                IncidenceError::TopologyBindingMismatch
            ));
            assert!(matches!(
                wrong_dimension,
                IncidenceError::DimensionMismatch { .. }
            ));
            assert_eq!(output, snapshot);
            let before = GLOBAL.stats();
            drop(prepared);
            let released = GLOBAL.stats() - before;
            assert_eq!(released.allocations, 0);
            assert_eq!(released.reallocations, 0);
            assert_eq!(released.bytes_deallocated, retained);
            println!(
                "prepared-topology source={source:?} rows={} setup_bound={bound} allocated={} freed_setup={} retained={retained} release=exact read/scatter/reject_allocations=0",
                input.len(),
                setup.bytes_allocated,
                setup.bytes_deallocated
            );
            tested += 1;
        }
    }
    assert_eq!(tested, 8);
    println!("PASS prepared-topology cases=8 borrowed-owner checks and array lifetime accounting");
    Ok(())
}
