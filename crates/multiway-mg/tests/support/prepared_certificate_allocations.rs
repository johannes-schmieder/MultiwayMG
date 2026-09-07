//! Isolated original-operator certificate setup, failures and release.
use super::{GLOBAL, Result, no_events};
use multiway_incidence::{PreparedThreeWayTopology, ThreeWayWeightFrame, WeightFrameInput};
use multiway_mg::{PreparedCertificateWorkspace, certify_prepared_normal_equations};
pub fn run() -> Result<()> {
    let topology = PreparedThreeWayTopology::try_from_collapsed([1; 3], &[[0; 3]])?;
    let fine = ThreeWayWeightFrame::try_new(&topology, WeightFrameInput::UnitTuples)?;
    let before = GLOBAL.stats();
    let mut workspace = PreparedCertificateWorkspace::try_new(&fine)?;
    let setup = GLOBAL.stats() - before;
    let retained = workspace.retained_payload_bytes()?;
    assert_eq!(setup.allocations, 2);
    assert_eq!(setup.deallocations, 0);
    assert_eq!(setup.reallocations, 0);
    assert_eq!(setup.bytes_allocated, retained);
    assert_eq!(retained, 8 * (1 + 3));
    assert_eq!(
        retained,
        PreparedCertificateWorkspace::required_payload_bytes(&fine)?
    );
    let before = GLOBAL.stats();
    for _ in 0..32 {
        assert_eq!(
            certify_prepared_normal_equations(
                fine.operator_view(),
                &[1.0],
                &[0.0; 3],
                &mut workspace
            )?,
            1.0
        );
        assert!(
            certify_prepared_normal_equations(
                fine.operator_view(),
                &[0.0],
                &[1.0; 3],
                &mut workspace
            )
            .is_err()
        );
        assert!(
            certify_prepared_normal_equations(
                fine.operator_view(),
                &[f64::MAX],
                &[0.0; 3],
                &mut workspace
            )
            .is_err()
        );
        assert_eq!(
            certify_prepared_normal_equations(
                fine.operator_view(),
                &[0.0],
                &[0.0; 3],
                &mut workspace
            )?,
            0.0
        );
    }
    no_events(GLOBAL.stats() - before);
    let before = GLOBAL.stats();
    drop(workspace);
    let released = GLOBAL.stats() - before;
    assert_eq!(released.bytes_deallocated, retained);
    assert_eq!(released.allocations, 0);
    assert_eq!(released.deallocations, 2);
    println!(
        "prepared original-operator certificate: first/repeat32 failures and recovery allocations=0; exact two-array release"
    );
    Ok(())
}
