//! Complete component scheduling, admission, fallback and original certificates.
#![cfg(feature = "lsmr")]
use multiway_incidence::{
    PreparedHierarchyBudget, PreparedThreeWayTopology, ThreeWayWeightFrame, WeightFrameInput,
};
use multiway_mg::*;
type Result<T = ()> = std::result::Result<T, Box<dyn std::error::Error>>;
const B: PreparedHierarchyBudget = PreparedHierarchyBudget::UNLIMITED;
fn options() -> PreparedAutomaticOptions {
    PreparedAutomaticOptions {
        hierarchy: Some(PreparedAutomaticHierarchyOptions {
            maximum_transitions: 8,
            maximum_tuple_multiplier: 4,
            maximum_coefficient_multiplier: 3,
            coverage: PreparedPairProposalCoverage::AdjacentPairs,
            candidate: PairNeighborhoodAggregationOptions {
                minimum_affinity: 0.02,
                maximum_neighbor_degree: 4,
            },
            screen: CycleQualityOptions {
                test_vectors: 2,
                power_iterations: 8,
                tail_iterations: 4,
                ..CycleQualityOptions::default()
            },
            criteria: CycleQualityCriteria {
                maximum_estimated_energy_factor: 0.8,
                maximum_observed_energy_factor: Some(1.05),
                maximum_structural_defect: 1e-10,
            },
        }),
        terminal_relative_tolerance: 1e-12,
        fallback: PreparedBaselineKind::SymmetricMap,
        lsmr: PreparedLsmrOptions::default(),
    }
}
fn targets(frame: &ThreeWayWeightFrame<'_>, k: usize) -> Vec<f64> {
    let e = frame.weights().len();
    let mut values = vec![0.; e * k];
    let x: Vec<_> = (0..frame.diagonal().len())
        .map(|i| (i as f64 * 0.23).cos())
        .collect();
    for j in 0..k {
        if j % 3 == 2 {
            continue;
        }
        if j % 3 == 1 {
            frame
                .operator_view()
                .apply_incidence(&x, &mut values[j * e..(j + 1) * e])
                .unwrap();
        } else {
            for i in 0..e {
                values[j * e + i] = ((i + 19 * j) as f64 * 0.17).sin();
            }
        }
    }
    values
}
fn execute(
    frame: &ThreeWayWeightFrame<'_>,
    y: &[f64],
    k: usize,
    o: PreparedAutomaticOptions,
    b: PreparedHierarchyBudget,
) -> Result<(
    Vec<f64>,
    Vec<Option<PreparedAutomaticColumnReport>>,
    PreparedAutomaticProgress,
)> {
    let mut x = vec![f64::from_bits(0x7ff8_0000_0000_0042); frame.diagonal().len() * k];
    let mut reports = vec![None; k];
    let mut p = PreparedAutomaticProgress::default();
    solve_prepared_automatic_batch_into(
        frame,
        PreparedAutomaticBatch {
            targets: y,
            columns: k,
            coefficients: &mut x,
            reports: &mut reports,
        },
        o,
        b,
        &mut p,
    )?;
    assert_eq!(p.stage, PreparedAutomaticStage::Complete);
    let mut certificate = PreparedCertificateWorkspace::try_new(frame)?;
    for j in 0..k {
        let c = certify_prepared_normal_equations(
            frame.operator_view(),
            &y[j * frame.weights().len()..(j + 1) * frame.weights().len()],
            &x[j * frame.diagonal().len()..(j + 1) * frame.diagonal().len()],
            &mut certificate,
        )?;
        let report = reports[j].unwrap();
        assert!(report.accepted, "{p:#?}\n{report:?}");
        assert_eq!(
            c.to_bits(),
            report.certified_normal_equation_residual.to_bits()
        );
    }
    Ok((x, reports, p))
}
fn bits(a: &[f64], b: &[f64]) {
    assert_eq!(a.len(), b.len());
    for (a, b) in a.iter().zip(b) {
        assert_eq!(a.to_bits(), b.to_bits());
    }
}

#[test]
fn small_ragged_components_and_singletons_certify_every_batch_prefix() -> Result {
    let keys = [
        [0, 0, 1],
        [0, 2, 3],
        [1, 1, 0],
        [2, 0, 1],
        [2, 2, 3],
        [3, 3, 2],
    ];
    let t = PreparedThreeWayTopology::try_from_collapsed([4; 3], &keys)?;
    for generation in 0..2 {
        let weights: Vec<_> = (0..keys.len())
            .map(|i| 1. + generation as f64 * (i + 1) as f64 / 7.)
            .collect();
        let f = ThreeWayWeightFrame::try_new(&t, WeightFrameInput::Tuples(&weights))?;
        let y = targets(&f, 32);
        let (full, _, p) = execute(&f, &y, 32, options(), B)?;
        assert_eq!(
            (
                p.components,
                p.singleton_components,
                p.dense_components,
                p.large_components
            ),
            (3, 2, 1, 0)
        );
        assert_eq!(p.global_fallback_columns, 0);
        let ordinary = ThreeWayProblem::from_observations([4; 3], &keys, &weights)?;
        let dense = DensePseudoinverse::from_problem(&ordinary, 1e-12)?;
        for k in [1, 2, 4, 8, 16, 17, 32] {
            let (x, reports, p) = execute(&f, &y[..k * keys.len()], k, options(), B)?;
            bits(&x, &full[..x.len()]);
            for j in 0..k {
                let rhs = ordinary.rhs_from_targets(&y[j * keys.len()..(j + 1) * keys.len()])?;
                let mut reference = vec![0.; 12];
                dense.solve_into(&rhs, &mut reference)?;
                for (a, b) in x[j * 12..(j + 1) * 12].iter().zip(&reference) {
                    assert!((a - b).abs() < 1e-10 * (1. + b.abs()));
                }
            }
            let (again, rr, pp) = execute(&f, &y[..k * keys.len()], k, options(), B)?;
            bits(&x, &again);
            assert_eq!(reports, rr);
            assert_eq!(format!("{p:?}"), format!("{pp:?}"));
        }
    }
    let keys: Vec<_> = (0..300).map(|i| [i, 299 - i, (i + 117) % 300]).collect();
    let t = PreparedThreeWayTopology::try_from_collapsed([300; 3], &keys)?;
    let f = ThreeWayWeightFrame::try_new(&t, WeightFrameInput::UnitTuples)?;
    for k in [1, 32] {
        let y = targets(&f, k);
        let (_, _, p) = execute(&f, &y, k, options(), B)?;
        assert_eq!(p.singleton_components, 300);
        assert_eq!(p.dense_components + p.large_components, 0);
        let caller = 8 * (y.len() + 900 * k)
            + std::mem::size_of::<Option<PreparedAutomaticColumnReport>>() * k;
        let exact = caller
            + t.retained_payload_bytes()?
            + f.retained_payload_bytes()?
            + t.projection_workspace_required_bytes()?
            + PreparedCertificateWorkspace::required_payload_bytes(&f)?;
        assert_eq!(p.maximum_admitted_payload_bytes, exact);
    }
    Ok(())
}
fn large_problem(n: u32, latin: bool) -> Result<PreparedThreeWayTopology> {
    let keys: Vec<_> = if latin {
        (0..n)
            .flat_map(|i| (0..n).map(move |j| [i, j, (i + j) % n]))
            .collect()
    } else {
        (0..n)
            .flat_map(|i| [[i, i, i], [i, (i + 1) % n, (i + 1) % n]])
            .collect()
    };
    let mut keys = keys;
    keys.sort_unstable();
    Ok(PreparedThreeWayTopology::try_from_collapsed(
        [n as usize; 3],
        &keys,
    )?)
}
#[test]
fn large_hierarchy_and_charged_construction_screen_and_budget_fallbacks() -> Result {
    for latin in [true, false] {
        let t = large_problem(128, latin)?;
        let weights: Vec<_> = (0..t.topology().tuple_count())
            .map(|i| if latin || i % 2 == 0 { 1. } else { 1. / 64. })
            .collect();
        let f = ThreeWayWeightFrame::try_new(&t, WeightFrameInput::Tuples(&weights))?;
        let y = targets(&f, 4);
        let (_, _, p) = execute(&f, &y, 4, options(), B)?;
        assert_eq!(p.large_components, 1);
        assert_eq!(p.hierarchy_attempts, 1);
        if latin {
            assert_eq!(p.accepted_hierarchies, 1);
        }
        assert!(p.candidate_work.tuple_visits > 0);
        let mut o = options();
        o.hierarchy.as_mut().unwrap().maximum_transitions = 0;
        let (x, _, p) = execute(&f, &y, 4, o, B)?;
        assert_eq!(p.hierarchy_rejections, 1);
        assert_eq!(p.baseline_components, 1);
        assert!(p.last_rejection.is_some());
        o.hierarchy = None;
        let (baseline, _, direct) = execute(&f, &y, 4, o, B)?;
        bits(&x, &baseline);
        assert_eq!(direct.hierarchy_attempts, 0);
        assert_eq!(p.solve_work, direct.solve_work);
        assert_eq!(p.gate_work, direct.gate_work);
        // Admit the complete baseline exactly, forcing the larger automatic
        // setup to reject without keeping its arrays alive during fallback.
        let owner = PreparedBaseline::new(&f, PreparedBaselineKind::SymmetricMap);
        let caller = 8 * (y.len() + 4 * f.diagonal().len())
            + 4 * std::mem::size_of::<Option<PreparedAutomaticColumnReport>>();
        let cap = PreparedLsmrWorkspace::setup_payload_bound(&owner, o.lsmr, caller)?;
        let (_, _, p) = execute(
            &f,
            &y,
            4,
            options(),
            PreparedHierarchyBudget {
                maximum_payload_bytes: cap,
                ..B
            },
        )?;
        assert!(p.hierarchy_rejections > 0);
        assert_eq!(p.baseline_components, 1);
        assert!(p.maximum_requested_payload_bytes > cap);
        assert!(p.maximum_admitted_payload_bytes <= cap);
        if !latin {
            let mut o = options();
            let h = o.hierarchy.as_mut().unwrap();
            h.criteria.maximum_estimated_energy_factor = 1e-12;
            h.criteria.maximum_observed_energy_factor = Some(1e-12);
            let (_, _, p) = execute(&f, &y, 4, o, B)?;
            assert_eq!(p.quality_rejections, 1);
            assert_eq!(p.baseline_components, 1);
            assert!(p.screen_work.cycle_applications > 0);
            assert!(p.last_quality_rejection.is_some());
        }
    }
    Ok(())
}

#[test]
fn giant_plus_small_components_use_original_coordinates_and_changed_weights() -> Result {
    let n = 128u32;
    let mut keys: Vec<_> = (0..n)
        .flat_map(|i| (0..n).map(move |j| [i, j, (i + j) % n]))
        .collect();
    keys.extend(
        [
            [0, 0, 1],
            [0, 2, 3],
            [1, 1, 0],
            [2, 0, 1],
            [2, 2, 3],
            [3, 3, 2],
        ]
        .map(|k| k.map(|i| i + n)),
    );
    keys.extend((132..137).map(|i| [i; 3]));
    for key in &mut keys {
        for (q, id) in key.iter_mut().enumerate() {
            *id = (*id * 57 + q as u32 * 13) % 137;
        }
    }
    keys.sort_unstable();
    let t = PreparedThreeWayTopology::try_from_collapsed([137; 3], &keys)?;
    for generation in 0..2 {
        let weights: Vec<_> = (0..keys.len())
            .map(|i| 1. + generation as f64 * (i % 7) as f64 / 8.)
            .collect();
        let f = ThreeWayWeightFrame::try_new(&t, WeightFrameInput::Tuples(&weights))?;
        let y = targets(&f, 4);
        let mut o = options();
        o.hierarchy = None;
        let (_, _, p) = execute(&f, &y, 4, o, B)?;
        assert_eq!(
            (
                p.components,
                p.large_components,
                p.dense_components,
                p.singleton_components
            ),
            (9, 1, 1, 7)
        );
        assert_eq!(p.baseline_components, 1);
        assert_eq!(p.global_fallback_columns, 0);
        let (_, _, automatic) = execute(&f, &y, 4, options(), B)?;
        assert_eq!(automatic.hierarchy_attempts, 1);
        assert_eq!(automatic.global_fallback_columns, 0);
    }
    Ok(())
}

#[test]
fn original_certificate_recovers_rank_truncation_and_failure_semantics() -> Result {
    let keys: Vec<_> = (0..3)
        .flat_map(|i| (0..4).flat_map(move |j| (0..2).map(move |k| [i, j, k])))
        .collect();
    let t = PreparedThreeWayTopology::try_from_collapsed([3, 4, 2], &keys)?;
    let weights: Vec<_> = (0..keys.len())
        .map(|i| 2f64.powi((i % 7) as i32 - 3))
        .collect();
    let f = ThreeWayWeightFrame::try_new(&t, WeightFrameInput::Tuples(&weights))?;
    let y = targets(&f, 3);
    let mut o = options();
    o.terminal_relative_tolerance = 0.9;
    let (mut x, mut reports, mut p) = execute(&f, &y, 3, o, B)?;
    assert!(p.global_fallback_columns > 0);
    assert!(p.global_fallback_columns < 3);
    assert!(
        reports
            .iter()
            .any(|r| r.unwrap().initial_certificate.unwrap() > o.lsmr.certificate_tolerance)
    );
    assert!(!reports[2].unwrap().global_fallback);
    assert_eq!(reports[2].unwrap().certified_normal_equation_residual, 0.);
    let before = x.clone();
    let rr = reports.clone();
    let pp = format!("{p:?}");
    let mut bad = y.clone();
    bad[0] = f64::NAN;
    assert!(
        solve_prepared_automatic_batch_into(
            &f,
            PreparedAutomaticBatch {
                targets: &bad,
                columns: 3,
                coefficients: &mut x,
                reports: &mut reports
            },
            o,
            B,
            &mut p
        )
        .is_err()
    );
    bits(&x, &before);
    assert_eq!(reports, rr);
    assert_eq!(format!("{p:?}"), pp);
    assert!(
        solve_prepared_automatic_batch_into(
            &f,
            PreparedAutomaticBatch {
                targets: &y,
                columns: 0,
                coefficients: &mut x,
                reports: &mut reports
            },
            o,
            B,
            &mut p
        )
        .is_err()
    );
    bits(&x, &before);
    assert_eq!(reports, rr);
    assert_eq!(format!("{p:?}"), pp);
    let huge =
        ThreeWayWeightFrame::try_new(&t, WeightFrameInput::Tuples(&vec![1e300; keys.len()]))?;
    let huge_y = vec![1e20; keys.len()];
    let mut output = vec![17.; 9];
    let mut records = [None];
    let mut failure = PreparedAutomaticProgress::default();
    assert!(
        solve_prepared_automatic_batch_into(
            &huge,
            PreparedAutomaticBatch {
                targets: &huge_y,
                columns: 1,
                coefficients: &mut output,
                reports: &mut records
            },
            options(),
            B,
            &mut failure
        )
        .is_err()
    );
    assert_eq!(failure.stage, PreparedAutomaticStage::GlobalFallback);
    assert!(failure.last_rejection.is_some());
    assert_eq!(records, [None]);
    let zeros = vec![0.; keys.len()];
    execute(&huge, &zeros, 1, options(), B)?;
    Ok(())
}

#[test]
fn unbalanced_large_factors_reach_multiple_current_weight_transitions() -> Result {
    for n in [512u32, 1024] {
        let keys: Vec<_> = (0..n)
            .flat_map(|i| (0..2).flat_map(move |j| (0..2).map(move |k| [i, j, k])))
            .collect();
        let t = PreparedThreeWayTopology::try_from_collapsed([n as usize, 2, 2], &keys)?;
        for generation in 0..2 {
            let weights: Vec<_> = (0..keys.len())
                .map(|i| 1. + generation as f64 * (i % 7) as f64 / 8.)
                .collect();
            let f = ThreeWayWeightFrame::try_new(&t, WeightFrameInput::Tuples(&weights))?;
            let y = targets(&f, 3);
            let (_, _, p) = execute(&f, &y, 3, options(), B)?;
            assert_eq!(p.accepted_hierarchies, 1);
            assert_eq!(p.baseline_components, 0);
            assert_eq!(p.structural_attempts, if n == 512 { 2 } else { 3 });
            assert!(p.provisional_input_tuples > 0);
            assert!(p.screen_work.cycle_applications > 0);
        }
    }
    Ok(())
}

#[test]
fn payload_boundaries_and_declared_live_state_never_publish_uncertified_outputs() -> Result {
    let keys = [
        [0, 0, 1],
        [0, 2, 3],
        [1, 1, 0],
        [2, 0, 1],
        [2, 2, 3],
        [3, 3, 2],
    ];
    let t = PreparedThreeWayTopology::try_from_collapsed([4; 3], &keys)?;
    let f = ThreeWayWeightFrame::try_new(&t, WeightFrameInput::UnitTuples)?;
    let y = targets(&f, 3);
    let (reference, reference_reports, progress) = execute(&f, &y, 3, options(), B)?;
    let high = progress.maximum_admitted_payload_bytes;
    let (extra, extra_reports, p) = execute(
        &f,
        &y,
        3,
        options(),
        PreparedHierarchyBudget {
            additional_live_payload_bytes: 73,
            ..B
        },
    )?;
    bits(&reference, &extra);
    assert_eq!(reference_reports, extra_reports);
    assert_eq!(p.maximum_admitted_payload_bytes, high + 73);
    for cap in [0, high / 2, high - 1, high, high + 1] {
        let mut x = vec![17.; 36];
        let mut reports = vec![None; 3];
        let mut p = PreparedAutomaticProgress::default();
        let result = solve_prepared_automatic_batch_into(
            &f,
            PreparedAutomaticBatch {
                targets: &y,
                columns: 3,
                coefficients: &mut x,
                reports: &mut reports,
            },
            options(),
            PreparedHierarchyBudget {
                maximum_payload_bytes: cap,
                ..B
            },
            &mut p,
        );
        assert!(p.maximum_admitted_payload_bytes <= cap);
        if cap >= high {
            result?;
            bits(&x, &reference);
            assert_eq!(reports, reference_reports);
        } else {
            assert!(p.maximum_requested_payload_bytes > cap);
            if result.is_err() {
                assert_ne!(p.stage, PreparedAutomaticStage::Complete);
            }
        }
        let mut c = PreparedCertificateWorkspace::try_new(&f)?;
        for (j, report) in reports.iter().enumerate() {
            if let Some(report) = report {
                let measured = certify_prepared_normal_equations(
                    f.operator_view(),
                    &y[j * 6..(j + 1) * 6],
                    &x[j * 12..(j + 1) * 12],
                    &mut c,
                )?;
                assert_eq!(
                    measured.to_bits(),
                    report.certified_normal_equation_residual.to_bits()
                );
                assert_eq!(
                    report.accepted,
                    measured <= options().lsmr.certificate_tolerance
                );
            }
        }
    }
    Ok(())
}

#[test]
fn completed_fallback_keeps_nonconvergence_separate_from_acceptance() -> Result {
    let t = large_problem(128, false)?;
    let f = ThreeWayWeightFrame::try_new(&t, WeightFrameInput::UnitTuples)?;
    let e = f.weights().len();
    let v = f.diagonal().len();
    let mut y = targets(&f, 2);
    y[e..].fill(0.);
    let mut o = options();
    o.lsmr.max_iterations = 1;
    let mut x = vec![17.; 2 * v];
    let mut reports = [None; 2];
    let mut p = PreparedAutomaticProgress::default();
    solve_prepared_automatic_batch_into(
        &f,
        PreparedAutomaticBatch {
            targets: &y,
            columns: 2,
            coefficients: &mut x,
            reports: &mut reports,
        },
        o,
        B,
        &mut p,
    )?;
    assert_eq!(p.stage, PreparedAutomaticStage::Complete);
    assert_eq!(p.global_fallback_columns, 2);
    assert!(!reports[0].unwrap().accepted);
    assert!(reports[1].unwrap().accepted);
    assert!(p.last_rejection.is_some());
    assert!(p.rejections >= 2);
    let mut c = PreparedCertificateWorkspace::try_new(&f)?;
    for (j, report) in reports.iter().enumerate() {
        let measured = certify_prepared_normal_equations(
            f.operator_view(),
            &y[j * e..(j + 1) * e],
            &x[j * v..(j + 1) * v],
            &mut c,
        )?;
        assert_eq!(
            measured.to_bits(),
            report.unwrap().certified_normal_equation_residual.to_bits()
        );
    }
    // A new adequately provisioned call recovers after the rejected attempt.
    execute(&f, &y, 2, options(), B)?;
    Ok(())
}

type LayoutResult = (
    Vec<f64>,
    Vec<Option<PreparedAutomaticColumnReport>>,
    PreparedAutomaticProgress,
    PreparedAutomaticLayoutProgress,
);
fn execute_layout(
    frame: &ThreeWayWeightFrame<'_>,
    y: &[f64],
    k: usize,
    o: PreparedAutomaticOptions,
    b: PreparedHierarchyBudget,
    layout: PreparedAutomaticLayout,
) -> Result<LayoutResult> {
    let mut x = vec![f64::NAN; frame.diagonal().len() * k];
    let mut reports = vec![None; k];
    let mut progress = PreparedAutomaticProgress::default();
    let mut layouts = PreparedAutomaticLayoutProgress::default();
    solve_prepared_automatic_batch_into_with_layout(
        frame,
        PreparedAutomaticBatch {
            targets: y,
            columns: k,
            coefficients: &mut x,
            reports: &mut reports,
        },
        o,
        b,
        layout,
        &mut progress,
        &mut layouts,
    )?;
    let mut certificate = PreparedCertificateWorkspace::try_new(frame)?;
    for j in 0..k {
        let value = certify_prepared_normal_equations(
            frame.operator_view(),
            &y[j * frame.weights().len()..(j + 1) * frame.weights().len()],
            &x[j * frame.diagonal().len()..(j + 1) * frame.diagonal().len()],
            &mut certificate,
        )?;
        assert!(reports[j].unwrap().accepted);
        assert_eq!(
            value.to_bits(),
            reports[j]
                .unwrap()
                .certified_normal_equation_residual
                .to_bits()
        );
    }
    Ok((x, reports, progress, layouts))
}
const GROUPED: [PreparedAutomaticLayout; 4] = [
    PreparedAutomaticLayout::FineRow,
    PreparedAutomaticLayout::AllRow,
    PreparedAutomaticLayout::FineImage,
    PreparedAutomaticLayout::AllImage,
];
fn same_work(a: &PreparedAutomaticProgress, b: &PreparedAutomaticProgress) {
    assert_eq!(a.candidate_work, b.candidate_work);
    assert_eq!(a.structural_attempts, b.structural_attempts);
    assert_eq!(a.provisional_input_tuples, b.provisional_input_tuples);
    assert_eq!(a.screen_work, b.screen_work);
    assert_eq!(a.last_quality_rejection, b.last_quality_rejection);
    assert_eq!(a.solve_work, b.solve_work);
    assert_eq!(a.gate_work, b.gate_work);
    assert_eq!(a.global_certificate_work, b.global_certificate_work);
    assert_eq!(a.accepted_hierarchies, b.accepted_hierarchies);
    assert_eq!(a.hierarchy_rejections, b.hierarchy_rejections);
    assert_eq!(a.baseline_components, b.baseline_components);
    assert_eq!(a.global_fallback_columns, b.global_fallback_columns);
    assert_eq!(a.rejections, b.rejections);
}

#[test]
fn grouped_recursive_layouts_preserve_current_weight_numerics_at_every_prefix() -> Result {
    for n in [512u32, 1024] {
        let keys: Vec<_> = (0..n)
            .flat_map(|i| (0..2).flat_map(move |j| (0..2).map(move |k| [i, j, k])))
            .collect();
        let t = PreparedThreeWayTopology::try_from_collapsed([n as usize, 2, 2], &keys)?;
        for generation in 0..2 {
            let weights: Vec<_> = (0..keys.len())
                .map(|i| 1. + generation as f64 * (i % 7) as f64 / 8.)
                .collect();
            let f = ThreeWayWeightFrame::try_new(&t, WeightFrameInput::Tuples(&weights))?;
            for k in if n == 512 {
                &[1, 2, 4, 8, 16, 17, 32][..]
            } else {
                &[3][..]
            } {
                let y = targets(&f, *k);
                let (reference, rr, rp) = execute(&f, &y, *k, options(), B)?;
                let (x, r, p, g) =
                    execute_layout(&f, &y, *k, options(), B, PreparedAutomaticLayout::Scalar)?;
                bits(&x, &reference);
                assert_eq!(r, rr);
                assert_eq!(format!("{p:?}"), format!("{rp:?}"));
                assert_eq!(g, PreparedAutomaticLayoutProgress::default());
                for layout in GROUPED {
                    let (x, r, p, g) = execute_layout(&f, &y, *k, options(), B, layout)?;
                    bits(&x, &reference);
                    assert_eq!(r, rr);
                    same_work(&p, &rp);
                    assert_eq!(g.grouping_attempts, 1);
                    assert_eq!(g.completed_groupings, 1);
                    assert_eq!(g.rejected_groupings, 0);
                    assert_eq!(
                        g.grouped_levels_built,
                        if matches!(
                            layout,
                            PreparedAutomaticLayout::FineRow | PreparedAutomaticLayout::FineImage
                        ) {
                            1
                        } else {
                            rp.structural_attempts
                        }
                    );
                    assert_eq!(
                        g.maximum_tuple_image_len,
                        if matches!(
                            layout,
                            PreparedAutomaticLayout::FineImage | PreparedAutomaticLayout::AllImage
                        ) {
                            keys.len()
                        } else {
                            0
                        }
                    );
                    assert!(g.maximum_grouping_payload_bytes > 0);
                    assert!(p.maximum_admitted_payload_bytes > rp.maximum_admitted_payload_bytes);
                }
            }
        }
    }
    Ok(())
}

#[test]
fn grouped_component_baselines_and_failed_screens_keep_exact_references() -> Result {
    let n = 256u32;
    let mut keys: Vec<_> = (0..2)
        .flat_map(|c| {
            (0..n).flat_map(move |i| {
                (0..2).flat_map(move |j| (0..2).map(move |k| [c * n + i, c * 2 + j, c * 2 + k]))
            })
        })
        .collect();
    keys.extend(
        (0..2)
            .flat_map(|i| (0..2).flat_map(move |j| (0..2).map(move |k| [2 * n + i, 4 + j, 4 + k]))),
    );
    keys.push([2 * n + 2, 6, 6]);
    keys.sort_unstable();
    let t = PreparedThreeWayTopology::try_from_collapsed([(2 * n + 3) as usize, 7, 7], &keys)?;
    for generation in 0..2 {
        let weights: Vec<_> = (0..keys.len())
            .map(|i| 1. + generation as f64 * (i % 7) as f64 / 8.)
            .collect();
        let f = ThreeWayWeightFrame::try_new(&t, WeightFrameInput::Tuples(&weights))?;
        let y = targets(&f, 4);
        for route in 0..3 {
            let mut o = options();
            if route == 0 {
                o.hierarchy = None;
            }
            if route == 1 {
                o.hierarchy
                    .as_mut()
                    .unwrap()
                    .criteria
                    .maximum_estimated_energy_factor = 1e-12;
                o.hierarchy
                    .as_mut()
                    .unwrap()
                    .criteria
                    .maximum_observed_energy_factor = Some(1e-12);
            }
            let (reference, rr, rp) = execute(&f, &y, 4, o, B)?;
            assert_eq!(
                (
                    rp.large_components,
                    rp.dense_components,
                    rp.singleton_components
                ),
                (2, 1, 1)
            );
            for layout in GROUPED {
                let (x, r, p, g) = execute_layout(&f, &y, 4, o, B, layout)?;
                bits(&x, &reference);
                assert_eq!(r, rr);
                same_work(&p, &rp);
                assert_eq!(g.grouping_attempts, 2 + rp.hierarchy_rejections);
                assert_eq!(g.completed_groupings, g.grouping_attempts);
                assert_eq!(g.rejected_groupings, 0);
                if matches!(
                    layout,
                    PreparedAutomaticLayout::FineImage | PreparedAutomaticLayout::AllImage
                ) {
                    assert_eq!(
                        g.maximum_tuple_image_len,
                        if route == 0 { 0 } else { 4 * n as usize }
                    );
                    assert!(g.maximum_tuple_image_len < f.weights().len());
                }
            }
        }
    }
    // Additional exact kernel modes: tied second/third factors, weak links.
    let t = large_problem(128, false)?;
    let w: Vec<_> = (0..t.topology().tuple_count())
        .map(|i| if i % 2 == 0 { 1. } else { 1. / 64. })
        .collect();
    let f = ThreeWayWeightFrame::try_new(&t, WeightFrameInput::Tuples(&w))?;
    let y = targets(&f, 3);
    let mut rejected_options = options();
    rejected_options
        .hierarchy
        .as_mut()
        .unwrap()
        .criteria
        .maximum_estimated_energy_factor = 1e-12;
    rejected_options
        .hierarchy
        .as_mut()
        .unwrap()
        .criteria
        .maximum_observed_energy_factor = Some(1e-12);
    let (reference, rr, rp) = execute(&f, &y, 3, rejected_options, B)?;
    assert_eq!(rp.quality_rejections, 1);
    for layout in GROUPED {
        let (x, r, p, g) = execute_layout(&f, &y, 3, rejected_options, B, layout)?;
        assert_eq!(g.completed_groupings, 2);
        assert_eq!(g.rejected_groupings, 0);
        bits(&x, &reference);
        assert_eq!(r, rr);
        same_work(&p, &rp);
    }
    Ok(())
}

#[test]
fn grouped_global_recovery_and_validation_keep_report_contract() -> Result {
    let keys: Vec<_> = (0..3)
        .flat_map(|i| (0..4).flat_map(move |j| (0..2).map(move |k| [i, j, k])))
        .collect();
    let t = PreparedThreeWayTopology::try_from_collapsed([3, 4, 2], &keys)?;
    let w: Vec<_> = (0..keys.len())
        .map(|i| 2f64.powi((i % 7) as i32 - 3))
        .collect();
    let f = ThreeWayWeightFrame::try_new(&t, WeightFrameInput::Tuples(&w))?;
    let y = targets(&f, 3);
    let mut o = options();
    o.terminal_relative_tolerance = 0.9;
    let (reference, rr, rp) = execute(&f, &y, 3, o, B)?;
    assert!(rp.global_fallback_columns > 0);
    for layout in GROUPED {
        let (mut x, mut r, mut p, mut g) = execute_layout(&f, &y, 3, o, B, layout)?;
        bits(&x, &reference);
        assert_eq!(r, rr);
        same_work(&p, &rp);
        assert_eq!(g.grouping_attempts, 1);
        assert_eq!(g.grouped_levels_built, 1);
        let gg = g;
        let pp = format!("{p:?}");
        let mut invalid = y.clone();
        invalid[0] = f64::NAN;
        assert!(
            solve_prepared_automatic_batch_into_with_layout(
                &f,
                PreparedAutomaticBatch {
                    targets: &invalid,
                    columns: 3,
                    coefficients: &mut x,
                    reports: &mut r
                },
                o,
                B,
                layout,
                &mut p,
                &mut g
            )
            .is_err()
        );
        bits(&x, &reference);
        assert_eq!(r, rr);
        assert_eq!(g, gg);
        assert_eq!(format!("{p:?}"), pp);
    }
    Ok(())
}

#[test]
fn grouped_budget_accounts_one_owner_and_denied_setup_recovers_on_reuse() -> Result {
    use multiway_incidence::PreparedTupleGrouping;
    let keys: Vec<_> = (0..512)
        .flat_map(|i| (0..2).flat_map(move |j| (0..2).map(move |k| [i, j, k])))
        .collect();
    let t = PreparedThreeWayTopology::try_from_collapsed([512, 2, 2], &keys)?;
    let f = ThreeWayWeightFrame::try_new(&t, WeightFrameInput::UnitTuples)?;
    let y = targets(&f, 3);
    let mut o = options();
    o.hierarchy = None;
    let caller = 8 * (y.len() + 3 * f.diagonal().len())
        + 3 * std::mem::size_of::<Option<PreparedAutomaticColumnReport>>();
    let setup =
        PreparedTupleGrouping::setup_payload_bound(&t, caller + f.retained_payload_bytes()?)?;
    for layout in GROUPED {
        let (x, r, p, g) = execute_layout(&f, &y, 3, o, B, layout)?;
        let groups = PreparedTupleGrouping::try_new(&t)?;
        assert_eq!(
            g.maximum_grouping_payload_bytes,
            groups.retained_payload_bytes()?
        );
        // Even image layouts use the grouped baseline's LSMR-only row path.
        let mode = GroupedGramianMode::RowGather;
        assert_eq!(g.maximum_tuple_image_len, 0);
        let owner = PreparedBaseline::new(&f, o.fallback).with_grouping(&groups, mode)?;
        let solve = PreparedLsmrWorkspace::setup_payload_bound(&owner, o.lsmr, caller)?;
        let certificate = caller
            + t.retained_payload_bytes()?
            + f.retained_payload_bytes()?
            + t.projection_workspace_required_bytes()?
            + PreparedCertificateWorkspace::required_payload_bytes(&f)?;
        assert_eq!(
            p.maximum_admitted_payload_bytes,
            setup.max(solve).max(certificate)
        );
        let (exact, er, ep, eg) = execute_layout(
            &f,
            &y,
            3,
            o,
            PreparedHierarchyBudget {
                maximum_payload_bytes: p.maximum_admitted_payload_bytes,
                ..B
            },
            layout,
        )?;
        bits(&x, &exact);
        assert_eq!(r, er);
        same_work(&p, &ep);
        assert_eq!(g, eg);
        let (shifted, sr, sp, sg) = execute_layout(
            &f,
            &y,
            3,
            o,
            PreparedHierarchyBudget {
                additional_live_payload_bytes: 1234,
                ..B
            },
            layout,
        )?;
        bits(&x, &shifted);
        assert_eq!(r, sr);
        same_work(&p, &sp);
        assert_eq!(g, sg);
        assert_eq!(
            sp.maximum_admitted_payload_bytes,
            p.maximum_admitted_payload_bytes + 1234
        );
        assert_eq!(
            sp.maximum_requested_payload_bytes,
            p.maximum_requested_payload_bytes + 1234
        );
        let mut output = vec![17.; x.len()];
        let mut reports = vec![None; 3];
        let mut progress = PreparedAutomaticProgress::default();
        let mut layouts = PreparedAutomaticLayoutProgress::default();
        assert!(
            solve_prepared_automatic_batch_into_with_layout(
                &f,
                PreparedAutomaticBatch {
                    targets: &y,
                    columns: 3,
                    coefficients: &mut output,
                    reports: &mut reports
                },
                o,
                PreparedHierarchyBudget {
                    maximum_payload_bytes: setup - 1,
                    ..B
                },
                layout,
                &mut progress,
                &mut layouts
            )
            .is_err()
        );
        assert_eq!(reports, vec![None; 3]);
        assert_eq!(layouts.grouping_attempts, 2);
        assert_eq!(layouts.rejected_groupings, 2);
        assert_eq!(layouts.completed_groupings, 0);
        assert_eq!(layouts.maximum_tuple_image_len, 0);
        assert_eq!(
            layouts.last_rejected_grouping.unwrap().scope,
            PreparedAutomaticGroupingScope::GlobalBaseline
        );
        assert!(progress.maximum_requested_payload_bytes >= setup);
        assert!(progress.maximum_admitted_payload_bytes < setup);
        solve_prepared_automatic_batch_into_with_layout(
            &f,
            PreparedAutomaticBatch {
                targets: &y,
                columns: 3,
                coefficients: &mut output,
                reports: &mut reports,
            },
            o,
            B,
            layout,
            &mut progress,
            &mut layouts,
        )?;
        bits(&output, &x);
        assert_eq!(reports, r);
        assert_eq!(layouts, g);
    }
    Ok(())
}
