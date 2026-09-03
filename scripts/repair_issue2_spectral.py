"""Repair eigenpair ordering and sharpen range-leakage tests."""

from pathlib import Path


SPECTRAL = Path("crates/multiway-mg/src/spectral.rs")
TESTS = Path("crates/multiway-mg/tests/spectral.rs")


def repair_spectral() -> None:
    text = SPECTRAL.read_text(encoding="utf-8")
    old = """        let mut positive_indices = Vec::new();
        for (index, &eigenvalue) in decomposition.eigenvalues.iter().enumerate() {
            if eigenvalue < -threshold {
                return Err(MultiwayError::NegativeEigenvalue {
                    value: eigenvalue,
                    tolerance: threshold,
                });
            }
            if eigenvalue > threshold {
                positive_indices.push(index);
            }
        }
        if positive_indices.is_empty() {
            return Err(MultiwayError::SpectralAnalysis {
                message: \"Gramian has no positive numerical range\".to_owned(),
            });
        }

        let rank = positive_indices.len();
        let mut basis = DMatrix::zeros(dimension, rank);
        let mut positive_eigenvalues = Vec::with_capacity(rank);
        for (column, &source_column) in positive_indices.iter().enumerate() {
            positive_eigenvalues.push(decomposition.eigenvalues[source_column]);
            for row in 0..dimension {
                basis[(row, column)] = decomposition.eigenvectors[(row, source_column)];
            }
        }
        positive_eigenvalues.sort_by(f64::total_cmp);
"""
    new = """        let mut positive_modes = Vec::new();
        for (index, &eigenvalue) in decomposition.eigenvalues.iter().enumerate() {
            if eigenvalue < -threshold {
                return Err(MultiwayError::NegativeEigenvalue {
                    value: eigenvalue,
                    tolerance: threshold,
                });
            }
            if eigenvalue > threshold {
                positive_modes.push((eigenvalue, index));
            }
        }
        if positive_modes.is_empty() {
            return Err(MultiwayError::SpectralAnalysis {
                message: \"Gramian has no positive numerical range\".to_owned(),
            });
        }
        positive_modes.sort_by(|left, right| left.0.total_cmp(&right.0));

        let rank = positive_modes.len();
        let mut basis = DMatrix::zeros(dimension, rank);
        let mut positive_eigenvalues = Vec::with_capacity(rank);
        for (column, &(eigenvalue, source_column)) in positive_modes.iter().enumerate() {
            positive_eigenvalues.push(eigenvalue);
            for row in 0..dimension {
                basis[(row, column)] = decomposition.eigenvectors[(row, source_column)];
            }
        }
"""
    if old not in text:
        raise RuntimeError("spectral eigenpair block was not found")
    SPECTRAL.write_text(text.replace(old, new), encoding="utf-8")


def repair_tests() -> None:
    text = TESTS.read_text(encoding="utf-8")
    old = """    for preconditioner in [
        &diagonal as &dyn Preconditioner,
        &map as &dyn Preconditioner,
        &dense_pair as &dyn Preconditioner,
    ] {
        let report = range
            .analyze(preconditioner, options)
            .expect(\"spectral analysis succeeds\");
        assert!(report.numerically_symmetric());
        assert!(report.preserves_range());
        assert!(report.positive_definite_on_range());
        assert_eq!(report.negative_preconditioner_directions(), 0);
        assert_eq!(report.near_zero_preconditioner_directions(), 0);
        assert!(report.minimum_preconditioned_eigenvalue() > 0.0);
        assert!(report.preconditioned_condition_number().is_finite());
    }
"""
    new = """    let diagonal_report = range
        .analyze(&diagonal, options)
        .expect(\"diagonal spectral analysis succeeds\");
    assert!(diagonal_report.numerically_symmetric());
    assert!(diagonal_report.positive_definite_on_range());
    assert_eq!(diagonal_report.negative_preconditioner_directions(), 0);
    assert_eq!(diagonal_report.near_zero_preconditioner_directions(), 0);
    assert!(diagonal_report.minimum_preconditioned_eigenvalue() > 0.0);
    assert!(diagonal_report.preconditioned_condition_number().is_finite());

    for preconditioner in [
        &map as &dyn Preconditioner,
        &dense_pair as &dyn Preconditioner,
    ] {
        let report = range
            .analyze(preconditioner, options)
            .expect(\"spectral analysis succeeds\");
        assert!(report.numerically_symmetric());
        assert!(report.preserves_range());
        assert!(report.positive_definite_on_range());
        assert_eq!(report.negative_preconditioner_directions(), 0);
        assert_eq!(report.near_zero_preconditioner_directions(), 0);
        assert!(report.minimum_preconditioned_eigenvalue() > 0.0);
        assert!(report.preconditioned_condition_number().is_finite());
    }
"""
    if old not in text:
        raise RuntimeError("spectral test loop was not found")
    TESTS.write_text(text.replace(old, new), encoding="utf-8")


def main() -> None:
    repair_spectral()
    repair_tests()


if __name__ == "__main__":
    main()
