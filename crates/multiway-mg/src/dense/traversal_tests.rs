//! Exact old-loop reference and separately invoked application microbenchmark.
use super::*;

// Copied from verified M6l main4fdce09, with caller-owned modal scratch.
// Keep independent of the production application; the allocating wrapper is not
// a fair performance control. Setup and factorization are deliberately excluded.
fn old_application(
    terminal: &DensePseudoinverse,
    rhs: &[f64],
    out: &mut [f64],
    modal: &mut [f64],
) -> Result<(), MultiwayError> {
    let dimension = terminal.dimension();
    if rhs.len() != dimension {
        return Err(crate::error::dimension(
            "DensePseudoinverse::solve_into rhs",
            dimension,
            rhs.len(),
        ));
    }
    if out.len() != dimension {
        return Err(crate::error::dimension(
            "DensePseudoinverse::solve_into output",
            dimension,
            out.len(),
        ));
    }
    if modal.len() != dimension {
        return Err(crate::error::dimension(
            "DensePseudoinverseWorkspace",
            dimension,
            modal.len(),
        ));
    }
    for (mode, modal_value) in modal.iter_mut().enumerate() {
        let mut sum = 0.0;
        for (row, &right) in rhs.iter().enumerate() {
            sum = terminal.eigenvectors[(row, mode)].mul_add(right, sum);
        }
        *modal_value = sum * terminal.inverse_eigenvalues[mode];
    }
    for (row, value) in out.iter_mut().enumerate() {
        let mut sum = 0.0;
        for (mode, &modal_value) in modal.iter().enumerate() {
            sum = terminal.eigenvectors[(row, mode)].mul_add(modal_value, sum);
        }
        *value = sum;
    }
    Ok(())
}

fn synthetic(n: usize) -> DensePseudoinverse {
    // Deterministic coefficient table for traversal tests, not a fitted factor
    // or a scientific solve. Public tests separately exercise real eigensystems.
    let inverse_eigenvalues: Vec<_> = (0..n)
        .map(|i| {
            if i % 5 == 0 {
                0.0
            } else {
                1.0 / (i + 1) as f64
            }
        })
        .collect();
    DensePseudoinverse {
        eigenvectors: DMatrix::from_fn(n, n, |row, mode| {
            ((row * 17 + mode * 31 + 7) % 101) as f64 / 53.0 - 0.75
        }),
        rank: inverse_eigenvalues.iter().filter(|&&v| v != 0.0).count(),
        inverse_eigenvalues,
        threshold: 1.0e-12,
    }
}

fn bits(actual: &[f64], expected: &[f64]) {
    assert_eq!(actual.len(), expected.len());
    for (i, (&a, &b)) in actual.iter().zip(expected).enumerate() {
        assert_eq!(a.to_bits(), b.to_bits(), "entry {i}");
    }
}

#[test]
fn exact_modal_and_output_bits_at_vector_and_terminal_boundaries() {
    for n in [
        0, 1, 2, 3, 6, 9, 15, 16, 17, 31, 32, 33, 63, 64, 65, 127, 128, 129, 255, 256, 257,
    ] {
        let terminal = synthetic(n);
        let mut expected = vec![23.0; n];
        let mut old_modal = vec![-17.0; n];
        let mut actual = vec![f64::NAN; n];
        let mut modal = vec![f64::NAN; n];
        for kind in 0..4 {
            let rhs: Vec<_> = (0..n)
                .map(|i| match kind {
                    0 => ((i * 11 % 17) as f64 - 8.0) / 7.0,
                    1 => {
                        if i % 2 == 0 {
                            0.0
                        } else {
                            -0.0
                        }
                    }
                    2 => f64::from_bits((i + 1) as u64),
                    _ => {
                        if i % 3 == 0 {
                            -0.0
                        } else {
                            (i + 1) as f64 * -0.125
                        }
                    }
                })
                .collect();
            old_application(&terminal, &rhs, &mut expected, &mut old_modal).unwrap();
            terminal
                .solve_with_modal(&rhs, &mut actual, &mut modal)
                .unwrap();
            bits(&modal, &old_modal);
            bits(&actual, &expected);
        }
    }
}

#[test]
fn invalid_lengths_preserve_both_buffers_and_error_precedence() {
    let terminal = synthetic(6);
    for (nr, no, nm, context) in [
        (5, 7, 0, "DensePseudoinverse::solve_into rhs"),
        (7, 6, 6, "DensePseudoinverse::solve_into rhs"),
        (6, 5, 0, "DensePseudoinverse::solve_into output"),
        (6, 7, 6, "DensePseudoinverse::solve_into output"),
        (6, 6, 5, "DensePseudoinverseWorkspace"),
        (6, 6, 7, "DensePseudoinverseWorkspace"),
    ] {
        let rhs = vec![1.0; nr];
        let mut out = vec![-0.0; no];
        let mut modal = vec![f64::from_bits(1); nm];
        let before_out = out.clone();
        let before_modal = modal.clone();
        let error = terminal
            .solve_with_modal(&rhs, &mut out, &mut modal)
            .unwrap_err();
        assert!(
            matches!(error, MultiwayError::DimensionMismatch { context: c, .. } if c == context)
        );
        bits(&out, &before_out);
        bits(&modal, &before_modal);
    }
}

#[test]
fn nonfinite_classification_is_preserved_without_skipping_zero_modes() {
    let terminal = synthetic(9);
    for value in [f64::INFINITY, f64::NEG_INFINITY, f64::NAN] {
        let mut rhs = vec![1.0; 9];
        rhs[4] = value;
        let mut expected = vec![0.0; 9];
        let mut old_modal = expected.clone();
        let mut actual = expected.clone();
        let mut modal = expected.clone();
        old_application(&terminal, &rhs, &mut expected, &mut old_modal).unwrap();
        terminal
            .solve_with_modal(&rhs, &mut actual, &mut modal)
            .unwrap();
        for (a, b) in actual
            .iter()
            .chain(&modal)
            .zip(expected.iter().chain(&old_modal))
        {
            assert!((a.is_nan() && b.is_nan()) || a.to_bits() == b.to_bits());
        }
    }
}

#[cfg(not(feature = "profiling"))]
fn fingerprint(values: &[f64]) -> u64 {
    values.iter().fold(0xcbf2_9ce4_8422_2325, |h, v| {
        (h ^ v.to_bits()).wrapping_mul(0x100_0000_01b3)
    })
}

#[test]
#[ignore = "fixed uninstrumented application microbenchmark; invoke through collector"]
#[cfg(not(feature = "profiling"))]
fn benchmark_dense_traversal() {
    use std::{hint::black_box, time::Instant};
    type Apply =
        fn(&DensePseudoinverse, &[f64], &mut [f64], &mut [f64]) -> Result<(), MultiwayError>;
    let arms: [(&str, Apply); 2] = [
        ("old", old_application),
        ("stream", DensePseudoinverse::solve_with_modal),
    ];
    // Policy dense-reconstruction-v1: fixed work, one warmup, five measured
    // samples, rotating arms. No minimum-time loop or best-sample selection.
    for (case, n) in [
        3, 6, 9, 16, 31, 32, 33, 63, 64, 65, 127, 128, 129, 255, 256, 257,
    ]
    .into_iter()
    .enumerate()
    {
        let terminal = synthetic(n);
        let rhs: Vec<_> = (0..n).map(|i| ((i * 11 % 17) as f64 - 8.0) / 7.0).collect();
        let mut expected = vec![0.0; n];
        let mut expected_modal = expected.clone();
        old_application(&terminal, &rhs, &mut expected, &mut expected_modal).unwrap();
        let iterations = (1 << 24) / (2 * n * n);
        let mut out = vec![0.0; n];
        let mut modal = vec![0.0; n];
        for round in 0..6 {
            for position in 0..2 {
                let (name, apply) = arms[(case + round + position) % 2];
                let apply = black_box(apply);
                let start = Instant::now();
                for _ in 0..iterations {
                    apply(
                        black_box(&terminal),
                        black_box(&rhs),
                        black_box(&mut out),
                        black_box(&mut modal),
                    )
                    .unwrap();
                }
                let elapsed = start.elapsed().as_nanos();
                bits(&out, &expected);
                bits(&modal, &expected_modal);
                let kind = if round == 0 { "warmup" } else { "measure" };
                let repeat = round.saturating_sub(1);
                println!(
                    "\ndense_reconstruction\t1\t{n}\t{name}\t{repeat}\t{kind}\t{iterations}\t{elapsed}\t{:016x}\t{:016x}",
                    fingerprint(&out),
                    fingerprint(&modal)
                );
            }
        }
    }
}
