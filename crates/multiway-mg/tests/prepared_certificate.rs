//! Independent original-operator certificate storage and numerical rejection.
use multiway_incidence::{PreparedThreeWayTopology, ThreeWayWeightFrame, WeightFrameInput};
use multiway_mg::{PreparedCertificateWorkspace, certify_prepared_normal_equations};
#[test]
fn independent_certificate_rejects_nonfinite_overflow_underflow_and_foreign_frames() {
    let topology = PreparedThreeWayTopology::try_from_collapsed([1; 3], &[[0; 3]]).unwrap();
    for weight in [1.0, 1e-200, 1e200] {
        let fine =
            ThreeWayWeightFrame::try_new(&topology, WeightFrameInput::Tuples(&[weight])).unwrap();
        let equal =
            ThreeWayWeightFrame::try_new(&topology, WeightFrameInput::Tuples(&[weight])).unwrap();
        let mut workspace = PreparedCertificateWorkspace::try_new(&fine).unwrap();
        assert_eq!(
            certify_prepared_normal_equations(
                fine.operator_view(),
                &[0.0],
                &[0.0; 3],
                &mut workspace
            )
            .unwrap(),
            0.0
        );
        let before = format!("{workspace:?}");
        assert!(
            certify_prepared_normal_equations(
                equal.operator_view(),
                &[0.0],
                &[0.0; 3],
                &mut workspace
            )
            .is_err()
        );
        for bad in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
            assert!(
                certify_prepared_normal_equations(
                    fine.operator_view(),
                    &[bad],
                    &[0.0; 3],
                    &mut workspace
                )
                .is_err()
            );
            assert!(
                certify_prepared_normal_equations(
                    fine.operator_view(),
                    &[0.0],
                    &[bad; 3],
                    &mut workspace
                )
                .is_err()
            );
        }
        assert_eq!(format!("{workspace:?}"), before);
        assert!(
            certify_prepared_normal_equations(
                fine.operator_view(),
                &[0.0],
                &[1.0; 3],
                &mut workspace
            )
            .is_err()
        );
        let bad_target = if weight < 1.0 { 1e-200 } else { f64::MAX };
        assert!(
            certify_prepared_normal_equations(
                fine.operator_view(),
                &[bad_target],
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
            )
            .unwrap(),
            0.0
        );
        assert_eq!(workspace.last_work().incidence_applications, 1);
        assert_eq!(workspace.last_work().adjoint_applications, 2);
    }
}
