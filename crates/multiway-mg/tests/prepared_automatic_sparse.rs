//! Automatic execution under the explicit sparse-terminal policy.
#![cfg(feature = "sparse-terminal")]
use multiway_incidence::{
    PreparedHierarchyBudget, PreparedThreeWayTopology, ThreeWayWeightFrame, WeightFrameInput,
};
use multiway_mg::*;
type Result<T = ()> = std::result::Result<T, Box<dyn std::error::Error>>;
const B: PreparedHierarchyBudget = PreparedHierarchyBudget::UNLIMITED;

fn options(transitions: usize) -> PreparedAutomaticOptions {
    PreparedAutomaticOptions {
        hierarchy: Some(PreparedAutomaticHierarchyOptions {
            maximum_transitions: transitions,
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

fn sparse() -> PreparedSparseTerminalOptions {
    PreparedSparseTerminalOptions {
        maximum_dimension: 4096,
        local_solver: within::LocalSolverConfig::default(),
        projector_tolerance: 1e-12,
    }
}

fn targets(frame: &ThreeWayWeightFrame<'_>, k: usize) -> Vec<f64> {
    let e = frame.weights().len();
    (0..e * k)
        .map(|i| ((i % e + 19 * (i / e)) as f64 * 0.17).sin())
        .collect()
}

struct Outcome {
    coefficients: Vec<f64>,
    reports: Vec<Option<PreparedAutomaticColumnReport>>,
    progress: PreparedAutomaticProgress,
}

/// Run one policy and independently certify every column against the original.
fn execute(
    frame: &ThreeWayWeightFrame<'_>,
    y: &[f64],
    k: usize,
    o: PreparedAutomaticOptions,
    policy: Option<&PreparedSparseTerminalOptions>,
    budget: PreparedHierarchyBudget,
    layout: PreparedAutomaticLayout,
) -> Result<Outcome> {
    let mut x = vec![f64::from_bits(0x7ff8_0000_0000_0042); frame.diagonal().len() * k];
    let mut reports = vec![None; k];
    let mut progress = PreparedAutomaticProgress::default();
    let mut layout_progress = PreparedAutomaticLayoutProgress::default();
    let batch = PreparedAutomaticBatch {
        targets: y,
        columns: k,
        coefficients: &mut x,
        reports: &mut reports,
    };
    match policy {
        Some(policy) => solve_prepared_automatic_batch_into_with_sparse_terminal(
            frame,
            batch,
            o,
            policy,
            budget,
            layout,
            &mut progress,
            &mut layout_progress,
        )?,
        None => solve_prepared_automatic_batch_into_with_layout(
            frame,
            batch,
            o,
            budget,
            layout,
            &mut progress,
            &mut layout_progress,
        )?,
    }
    assert_eq!(progress.stage, PreparedAutomaticStage::Complete);
    let mut certificate = PreparedCertificateWorkspace::try_new(frame)?;
    let (e, v) = (frame.weights().len(), frame.diagonal().len());
    for j in 0..k {
        let residual = certify_prepared_normal_equations(
            frame.operator_view(),
            &y[j * e..(j + 1) * e],
            &x[j * v..(j + 1) * v],
            &mut certificate,
        )?;
        let report = reports[j].unwrap();
        assert!(report.accepted, "{progress:#?}\n{report:?}");
        assert_eq!(
            residual.to_bits(),
            report.certified_normal_equation_residual.to_bits()
        );
    }
    Ok(Outcome {
        coefficients: x,
        reports,
        progress,
    })
}

/// A connected support above the dense cap: a cyclic chain plus random tuples.
fn connected(counts: [u32; 3], extra: usize, seed: u64) -> Result<PreparedThreeWayTopology> {
    let mut state = seed;
    let mut next = |bound: u32| {
        state = state
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        ((state >> 33) % u64::from(bound)) as u32
    };
    let maximum = *counts.iter().max().unwrap();
    let mut keys: Vec<[u32; 3]> = (0..maximum)
        .map(|i| [i % counts[0], i % counts[1], i % counts[2]])
        .collect();
    for _ in 0..extra {
        keys.push(counts.map(&mut next));
    }
    keys.sort_unstable();
    keys.dedup();
    Ok(PreparedThreeWayTopology::try_from_collapsed(
        counts.map(|n| n as usize),
        &keys,
    )?)
}

#[test]
fn zero_transitions_put_the_sparse_terminal_on_each_large_component() -> Result {
    let t = connected([160, 120, 60], 1500, 0x5350_0001)?;
    for weighting in [false, true] {
        let w: Vec<f64> = (0..t.topology().tuple_count())
            .map(|i| {
                if weighting {
                    2.0_f64.powi((i % 9) as i32 - 4)
                } else {
                    1.0
                }
            })
            .collect();
        let f = ThreeWayWeightFrame::try_new(&t, WeightFrameInput::Tuples(&w))?;
        for k in [1, 3] {
            let y = targets(&f, k);
            let out = execute(
                &f,
                &y,
                k,
                options(0),
                Some(&sparse()),
                B,
                PreparedAutomaticLayout::Scalar,
            )?;
            let p = &out.progress;
            assert_eq!((p.large_components, p.hierarchy_attempts), (1, 1));
            assert_eq!(p.sparse_terminals, 1, "{p:#?}");
            assert!(p.maximum_sparse_terminal_peak_payload_bytes > 0);
            // Either the screened sparse cycle solved the component or its typed
            // quality rejection sent it to the fixed baseline; both certify.
            assert_eq!(p.accepted_hierarchies + p.quality_rejections, 1, "{p:#?}");
            // The frozen policy rejects the same capped attempt as stagnated.
            let frozen = execute(
                &f,
                &y,
                k,
                options(0),
                None,
                B,
                PreparedAutomaticLayout::Scalar,
            )?;
            assert_eq!(frozen.progress.sparse_terminals, 0);
            assert_eq!(frozen.progress.hierarchy_rejections, 1);
            assert!(matches!(
                frozen.progress.last_rejection.as_ref().map(|r| &r.source),
                Some(MultiwayError::HierarchyStagnated { .. })
            ));
        }
    }
    Ok(())
}

#[test]
fn a_capped_coarse_level_above_256_takes_a_sparse_terminal() -> Result {
    // One adjacent-pair transition of a 1,200-coordinate support stays above 256.
    let t = connected([500, 400, 300], 4000, 0x5350_0002)?;
    let f = ThreeWayWeightFrame::try_new(&t, WeightFrameInput::UnitTuples)?;
    let y = targets(&f, 2);
    let out = execute(
        &f,
        &y,
        2,
        options(1),
        Some(&sparse()),
        B,
        PreparedAutomaticLayout::Scalar,
    )?;
    let p = &out.progress;
    assert_eq!(p.structural_attempts, 1, "{p:#?}");
    assert_eq!(p.sparse_terminals, 1, "{p:#?}");
    // A sparse cap below the capped level keeps the frozen rejection.
    let small = PreparedSparseTerminalOptions {
        maximum_dimension: 300,
        ..sparse()
    };
    let capped = execute(
        &f,
        &y,
        2,
        options(1),
        Some(&small),
        B,
        PreparedAutomaticLayout::Scalar,
    )?;
    assert_eq!(capped.progress.sparse_terminals, 0);
    assert!(matches!(
        capped.progress.last_rejection.as_ref().map(|r| &r.source),
        Some(MultiwayError::HierarchyStagnated { .. })
    ));
    Ok(())
}

#[test]
fn the_policy_is_inert_when_coarsening_reaches_the_dense_cap() -> Result {
    let t = connected([160, 120, 60], 1500, 0x5350_0003)?;
    let w: Vec<f64> = (0..t.topology().tuple_count())
        .map(|i| 1.0 + (i % 5) as f64 / 4.0)
        .collect();
    let f = ThreeWayWeightFrame::try_new(&t, WeightFrameInput::Tuples(&w))?;
    let y = targets(&f, 4);
    for layout in [
        PreparedAutomaticLayout::Scalar,
        PreparedAutomaticLayout::AllImage,
    ] {
        let frozen = execute(&f, &y, 4, options(8), None, B, layout)?;
        let policy = execute(&f, &y, 4, options(8), Some(&sparse()), B, layout)?;
        assert_eq!(policy.progress.sparse_terminals, 0);
        assert_eq!(
            frozen
                .coefficients
                .iter()
                .map(|v| v.to_bits())
                .collect::<Vec<_>>(),
            policy
                .coefficients
                .iter()
                .map(|v| v.to_bits())
                .collect::<Vec<_>>()
        );
        assert_eq!(frozen.reports, policy.reports);
        assert_eq!(
            format!("{:?}", frozen.progress),
            format!("{:?}", policy.progress)
        );
    }
    Ok(())
}

#[test]
fn grouped_layouts_and_mixed_components_certify_with_sparse_terminals() -> Result {
    // Two large components and a small one, interleaved by shifted level ranges.
    let a = connected([160, 120, 60], 1500, 0x5350_0004)?;
    let mut keys: Vec<[u32; 3]> = a.topology().tuples().to_vec();
    let b = connected([150, 110, 70], 1300, 0x5350_0005)?;
    keys.extend(
        b.topology()
            .tuples()
            .iter()
            .map(|k| [k[0] + 160, k[1] + 120, k[2] + 60]),
    );
    keys.extend([[310, 230, 130], [311, 230, 131], [310, 231, 131]]);
    keys.sort_unstable();
    let t = PreparedThreeWayTopology::try_from_collapsed([312, 232, 132], &keys)?;
    let f = ThreeWayWeightFrame::try_new(&t, WeightFrameInput::UnitTuples)?;
    let y = targets(&f, 2);
    for layout in [
        PreparedAutomaticLayout::Scalar,
        PreparedAutomaticLayout::AllImage,
    ] {
        let out = execute(&f, &y, 2, options(0), Some(&sparse()), B, layout)?;
        let p = &out.progress;
        assert_eq!((p.components, p.large_components), (3, 2), "{p:#?}");
        assert_eq!(p.sparse_terminals, 2, "{p:#?}");
    }
    Ok(())
}

#[test]
fn a_budget_refusing_the_terminal_falls_back_with_a_typed_rejection() -> Result {
    let t = connected([160, 120, 60], 1500, 0x5350_0006)?;
    let f = ThreeWayWeightFrame::try_new(&t, WeightFrameInput::UnitTuples)?;
    let y = targets(&f, 1);
    let full = execute(
        &f,
        &y,
        1,
        options(0),
        Some(&sparse()),
        B,
        PreparedAutomaticLayout::Scalar,
    )?;
    let baseline = execute(
        &f,
        &y,
        1,
        options(0),
        None,
        B,
        PreparedAutomaticLayout::Scalar,
    )?;
    // A budget just below the sparse attempt's admitted high-water mark but above
    // everything the frozen route (stagnation, then baseline) needs.
    let needed = baseline.progress.maximum_admitted_payload_bytes;
    let sparse_high = full.progress.maximum_admitted_payload_bytes;
    assert!(sparse_high > needed, "{sparse_high} {needed}");
    let tight = PreparedHierarchyBudget {
        maximum_payload_bytes: sparse_high - 1,
        additional_live_payload_bytes: 0,
    };
    let refused = execute(
        &f,
        &y,
        1,
        options(0),
        Some(&sparse()),
        tight,
        PreparedAutomaticLayout::Scalar,
    )?;
    let p = &refused.progress;
    assert_eq!(p.hierarchy_rejections, 1, "{p:#?}");
    assert_eq!(p.baseline_components, 1);
    assert!(p.maximum_admitted_payload_bytes < sparse_high);
    assert!(p.maximum_requested_payload_bytes >= sparse_high, "{p:#?}");
    match p.last_rejection.as_ref().map(|r| &r.source) {
        Some(MultiwayError::WithinSetupBudgetExceeded { .. })
        | Some(MultiwayError::PayloadBudgetExceeded { .. })
        | Some(MultiwayError::Incidence(_)) => {}
        other => panic!("{other:?}"),
    }
    Ok(())
}

#[test]
fn invalid_policies_are_refused_before_any_mutation() -> Result {
    let t = connected([160, 120, 60], 1500, 0x5350_0007)?;
    let f = ThreeWayWeightFrame::try_new(&t, WeightFrameInput::UnitTuples)?;
    let y = targets(&f, 1);
    let mut no_hierarchy = options(0);
    no_hierarchy.hierarchy = None;
    let at_dense_cap = PreparedSparseTerminalOptions {
        maximum_dimension: PREPARED_DENSE_TERMINAL_LIMIT,
        ..sparse()
    };
    let bad_tolerance = PreparedSparseTerminalOptions {
        projector_tolerance: f64::NAN,
        ..sparse()
    };
    for (o, policy) in [
        (no_hierarchy, sparse()),
        (options(0), at_dense_cap),
        (options(0), bad_tolerance),
    ] {
        let mut x = vec![7.0; f.diagonal().len()];
        let mut reports = vec![None];
        let mut progress = PreparedAutomaticProgress {
            components: 99,
            ..PreparedAutomaticProgress::default()
        };
        let mut layout_progress = PreparedAutomaticLayoutProgress::default();
        let result = solve_prepared_automatic_batch_into_with_sparse_terminal(
            &f,
            PreparedAutomaticBatch {
                targets: &y,
                columns: 1,
                coefficients: &mut x,
                reports: &mut reports,
            },
            o,
            &policy,
            B,
            PreparedAutomaticLayout::Scalar,
            &mut progress,
            &mut layout_progress,
        );
        assert!(result.is_err());
        assert!(x.iter().all(|&v| v == 7.0));
        assert_eq!(reports, vec![None]);
        assert_eq!(progress.components, 99);
    }
    Ok(())
}
