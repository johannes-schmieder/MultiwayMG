//! Finite-value checks before traced-PCG convergence decisions.

use crate::MultiwayError;

pub(super) fn ensure_values(
    values: &[f64],
    context: &'static str,
    iteration: usize,
) -> Result<(), MultiwayError> {
    if let Some((index, value)) = values
        .iter()
        .copied()
        .enumerate()
        .find(|(_, value)| !value.is_finite())
    {
        return Err(MultiwayError::PcgBreakdown {
            iteration,
            message: format!("{context} entry {index} is non-finite: {value}"),
        });
    }
    Ok(())
}

pub(super) fn require_finite(
    value: f64,
    context: &'static str,
    iteration: usize,
) -> Result<f64, MultiwayError> {
    if !value.is_finite() {
        return Err(MultiwayError::PcgBreakdown {
            iteration,
            message: format!("{context} is non-finite: {value}"),
        });
    }
    Ok(value)
}

pub(super) fn checked_norm(
    values: &[f64],
    context: &'static str,
    iteration: usize,
) -> Result<f64, MultiwayError> {
    // Validate before the original max reduction: an all-NaN slice must not
    // reach its zero-scale shortcut. Keep all valid finite arithmetic unchanged.
    ensure_values(values, context, iteration)?;
    require_finite(super::norm(values), context, iteration)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn checked_norm_preserves_finite_bits_including_zero_and_subnormals() {
        let tiny = f64::from_bits(1);
        let cases: &[&[f64]] = &[
            &[],
            &[0.0, -0.0],
            &[3.0, -4.0],
            &[tiny, -tiny, tiny],
            &[1.0e-200, -2.0e-200, 0.0],
            &[1.0e200, -2.0e200, 0.0],
            &[f64::MAX / 4.0, -f64::MAX / 8.0],
        ];
        for values in cases {
            let expected = super::super::norm(values);
            assert!(expected.is_finite());
            let actual = checked_norm(values, "test norm", 7).unwrap();
            assert_eq!(actual.to_bits(), expected.to_bits());
        }
    }

    #[test]
    fn nonfinite_values_and_unrepresentable_norms_fail_with_iteration_context() {
        for values in [
            vec![f64::NAN; 3],
            vec![0.0, f64::NAN],
            vec![f64::INFINITY],
            vec![f64::NEG_INFINITY],
            vec![f64::MAX; 4],
        ] {
            let error = checked_norm(&values, "test residual", 7).unwrap_err();
            assert!(matches!(
                error,
                MultiwayError::PcgBreakdown { iteration: 7, .. }
            ));
        }
        for value in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
            let error = require_finite(value, "test scalar", 4).unwrap_err();
            assert!(matches!(
                error,
                MultiwayError::PcgBreakdown { iteration: 4, .. }
            ));
        }
    }
}
