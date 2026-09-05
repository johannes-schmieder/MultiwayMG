//! Local construction-boundary failure tests, not OS allocator fault injection.
use super::*;

#[test]
fn every_preparation_boundary_recovers_after_error_and_unwind() {
    let rows = [[0, 0, 0], [1, 1, 1], [0, 1, 1], [1, 0, 0]];
    let old = PreparedThreeWayTopology::try_from_observations([2; 3], &rows).unwrap();
    let token = old.binding();
    let mut sorted = rows;
    sorted.sort_unstable();
    for source in [
        PreparedTopologySource::Observations,
        PreparedTopologySource::Collapsed,
    ] {
        let input = if source == PreparedTopologySource::Observations {
            &rows
        } else {
            &sorted
        };
        let count = if source == PreparedTopologySource::Observations {
            7
        } else {
            4
        };
        for unwind in [false, true] {
            for fail_at in 0..count {
                let mut reached = 0;
                let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    PreparedThreeWayTopology::build_with(
                        [2; 3],
                        input,
                        source,
                        usize::MAX,
                        &mut |context| {
                            let index = reached;
                            reached += 1;
                            if index == fail_at {
                                assert!(!unwind, "injected topology construction unwind");
                                return Err(IncidenceError::TopologyAllocation { context });
                            }
                            Ok(())
                        },
                    )
                }));
                assert_eq!(reached, fail_at + 1);
                if unwind {
                    assert!(outcome.is_err());
                } else {
                    assert!(matches!(
                        outcome.unwrap(),
                        Err(IncidenceError::TopologyAllocation { .. })
                    ));
                }
                old.validate_input_layout([2; 3], &rows).unwrap();
                assert!(token.is_bound_to(&old));
                let fresh = PreparedThreeWayTopology::build_with(
                    [2; 3],
                    input,
                    source,
                    usize::MAX,
                    &mut |_| Ok(()),
                )
                .unwrap();
                fresh.validate_input_layout([2; 3], input).unwrap();
                assert!(!token.is_bound_to(&fresh));
                let mut output = [99; 4];
                old.scatter_tuple_values_into(token, &[10, 20, 30, 40], &mut output)
                    .unwrap();
                assert_eq!(output, [10, 40, 20, 30]);
            }
        }
    }
}

#[test]
fn preflight_rejections_do_not_reach_array_reservations() {
    let valid = [[0, 0, 0], [1, 1, 1]];
    let source = PreparedTopologySource::Observations;
    let bound = PreparedThreeWayTopology::setup_payload_bound([2; 3], valid.len(), source).unwrap();
    for (counts, input, source, budget) in [
        ([2; 3], valid.as_slice(), source, bound - 1),
        ([2; 3], &[][..], source, usize::MAX),
        ([0, 2, 2], valid.as_slice(), source, usize::MAX),
        ([1; 3], valid.as_slice(), source, usize::MAX),
        (
            [2; 3],
            &[[1; 3], [0; 3]][..],
            PreparedTopologySource::Collapsed,
            usize::MAX,
        ),
    ] {
        assert!(
            PreparedThreeWayTopology::build_with(counts, input, source, budget, &mut |_| panic!(
                "preflight failed to reject"
            ))
            .is_err()
        );
    }
}
