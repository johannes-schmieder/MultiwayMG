//! Fixed baseline algebra, scalable certified solves and shared scalar batching.
use multiway_incidence::{
    PreparedThreeWayTopology, PreparedTupleGrouping, ThreeWayWeightFrame, WeightFrameInput,
};
use multiway_mg::{
    GroupedGramianMode, Preconditioner, PreparedBaseline, PreparedBaselineKind, PreparedPcgOptions,
    PreparedPcgWorkspace, PreparedSolverAction, SymmetricMapPreconditioner, ThreeWayProblem,
    solve_prepared_pcg_batch_into, solve_prepared_pcg_least_squares,
};
use nalgebra::{DMatrix, linalg::SymmetricEigen};
type Result<T = ()> = std::result::Result<T, Box<dyn std::error::Error>>;
const KINDS: [PreparedBaselineKind; 3] = [
    PreparedBaselineKind::Identity,
    PreparedBaselineKind::InverseDiagonal,
    PreparedBaselineKind::SymmetricMap,
];
fn projected(t: &PreparedThreeWayTopology, x: &mut [f64]) {
    // Independent component/factor-sum formula for the orthogonal shift projector.
    let counts = t.topology().level_counts();
    let offsets = t.topology().offsets();
    for (component, ns) in t.component_factor_sizes().iter().enumerate() {
        let mut sums = [0.; 3];
        for q in 0..3 {
            for i in 0..counts[q] {
                let id = offsets[q] + i;
                if t.component_labels()[id] == component {
                    sums[q] += x[id];
                }
            }
        }
        let common = (0..3).map(|q| sums[q] / ns[q] as f64).sum::<f64>()
            / (0..3).map(|q| 1. / ns[q] as f64).sum::<f64>();
        for q in 0..3 {
            for i in 0..counts[q] {
                let id = offsets[q] + i;
                if t.component_labels()[id] == component {
                    x[id] -= (sums[q] - common) / ns[q] as f64;
                }
            }
        }
    }
}
fn check_algebra(problem: ThreeWayProblem) -> Result {
    let t = PreparedThreeWayTopology::try_from_collapsed(
        problem.topology().level_counts(),
        problem.topology().tuples(),
    )?;
    let grouping = PreparedTupleGrouping::try_new(&t)?;
    for generation in 0..2 {
        let weights: Vec<_> = problem
            .weights()
            .iter()
            .enumerate()
            .map(|(i, w)| w * (1. + generation as f64 * (i % 7) as f64 / 8.))
            .collect();
        let f = ThreeWayWeightFrame::try_new(&t, WeightFrameInput::Tuples(&weights))?;
        let local = ThreeWayProblem::from_observations(
            t.topology().level_counts(),
            t.topology().tuples(),
            &weights,
        )?;
        let ordinary = SymmetricMapPreconditioner::new(local.clone());
        let n = local.dimension();
        let dense = local.dense_gramian();
        let gramian = DMatrix::from_fn(n, n, |i, j| dense[i][j]);
        let spectrum = SymmetricEigen::new(gramian);
        let scale = spectrum.eigenvalues.max();
        let retained: Vec<_> = (0..n)
            .filter(|&i| spectrum.eigenvalues[i] > scale * 1e-10)
            .collect();
        let basis = DMatrix::from_fn(n, retained.len(), |i, j| {
            spectrum.eigenvectors[(i, retained[j])]
        });
        for kind in KINDS {
            for mode in [
                None,
                Some(GroupedGramianMode::RowGather),
                Some(GroupedGramianMode::TupleImage),
            ] {
                let mut owner = PreparedBaseline::new(&f, kind);
                if let Some(mode) = mode {
                    owner = owner.with_grouping(&grouping, mode)?;
                }
                let mut w = owner.application_workspace()?;
                let payload = owner.payload_report(&w, 19)?;
                assert_eq!(
                    payload.total_payload_bytes,
                    owner.workspace_setup_payload_bound(19)?
                );
                assert_eq!(
                    payload.coarse_topology_payload_bytes
                        + payload.coarse_frame_payload_bytes
                        + payload.terminal_payload_bytes,
                    0
                );
                let mut matrix = DMatrix::zeros(n, n);
                for column in 0..n {
                    let mut rhs = vec![0.; n];
                    rhs[column] = 1.;
                    let mut out = vec![f64::NAN; n];
                    owner.apply_with_workspace(&rhs, &mut out, &mut w)?;
                    let mut expected = rhs.clone();
                    if kind == PreparedBaselineKind::SymmetricMap {
                        ordinary.apply(&rhs, &mut expected)?;
                        for (&a, &b) in out.iter().zip(&expected) {
                            assert_eq!(a.to_bits(), b.to_bits());
                        }
                    } else {
                        if kind == PreparedBaselineKind::InverseDiagonal {
                            projected(&t, &mut expected);
                            for (x, d) in expected.iter_mut().zip(f.diagonal()) {
                                *x /= d;
                            }
                            projected(&t, &mut expected);
                        }
                        for (a, b) in out.iter().zip(&expected) {
                            assert!((a - b).abs() <= 1e-12 * (1. + b.abs()));
                        }
                    }
                    for row in 0..n {
                        matrix[(row, column)] = out[row];
                    }
                    let mut actual_g = vec![0.; n];
                    owner.fine_gramian_with_workspace(&rhs, &mut actual_g, &mut w)?;
                    for row in 0..n {
                        assert_eq!(actual_g[row].to_bits(), dense[row][column].to_bits());
                    }
                }
                assert!((&matrix - matrix.transpose()).amax() <= 1e-12 * (1. + matrix.amax()));
                let restricted = basis.transpose() * &matrix * &basis;
                assert!(SymmetricEigen::new(restricted).eigenvalues.min() > 0.);
                // Identity preserves shifts; projected diagonal/MAP actions annihilate them.
                for component in 0..t.component_factor_sizes().len() {
                    for other in [1, 2] {
                        let mut shift = vec![0.; n];
                        let offsets = t.topology().offsets();
                        let counts = t.topology().level_counts();
                        for q in [0, other] {
                            for i in 0..counts[q] {
                                let id = offsets[q] + i;
                                if t.component_labels()[id] == component {
                                    shift[id] = if q == 0 { 1. } else { -1. };
                                }
                            }
                        }
                        let mut out = vec![0.; n];
                        owner.apply_with_workspace(&shift, &mut out, &mut w)?;
                        if kind == PreparedBaselineKind::Identity {
                            assert_eq!(out, shift);
                        } else {
                            assert!(out.iter().all(|x| x.abs() <= 1e-12 * (1. + matrix.amax())));
                        }
                    }
                }
            }
        }
    }
    Ok(())
}
#[test]
fn baselines_match_independent_projectors_and_map_with_positive_range_spectrum() -> Result {
    let keys: Vec<_> = (0..3)
        .flat_map(|i| (0..4).flat_map(move |j| (0..2).map(move |k| [i, j, k])))
        .collect();
    check_algebra(ThreeWayProblem::from_observations(
        [3, 4, 2],
        &keys,
        &vec![1.; keys.len()],
    )?)?;
    let mut keys: Vec<_> = (0..4)
        .flat_map(|i| (0..3).map(move |j| [i, j, j]))
        .collect();
    keys.push([4, 3, 3]);
    check_algebra(ThreeWayProblem::from_observations(
        [5, 4, 4],
        &keys,
        &[1.; 13],
    )?)?;
    check_algebra(ThreeWayProblem::from_observations(
        [1; 3],
        &[[0; 3]],
        &[1.],
    )?)?;
    Ok(())
}
fn check_solves(problem: ThreeWayProblem) -> Result {
    let t = PreparedThreeWayTopology::try_from_collapsed(
        problem.topology().level_counts(),
        problem.topology().tuples(),
    )?;
    let grouping = PreparedTupleGrouping::try_new(&t)?;
    for generation in 0..2 {
        let weights: Vec<_> = problem
            .weights()
            .iter()
            .enumerate()
            .map(|(i, w)| w * (1. + generation as f64 * (i % 7) as f64 / 8.))
            .collect();
        let f = ThreeWayWeightFrame::try_new(&t, WeightFrameInput::Tuples(&weights))?;
        let n = f.diagonal().len();
        let e = f.weights().len();
        let offsets = t.topology().offsets();
        let exact: Vec<_> = (0..n).map(|i| (i as f64 * 0.71).cos()).collect();
        for kind in KINDS {
            let mut scalar_reports = Vec::new();
            let mut scalar_coefficients = Vec::new();
            #[cfg(feature = "lsmr")]
            let mut lsmr_references = Vec::new();
            for mode in [
                None,
                Some(GroupedGramianMode::RowGather),
                Some(GroupedGramianMode::TupleImage),
            ] {
                let mut owner = PreparedBaseline::new(&f, kind);
                if let Some(mode) = mode {
                    owner = owner.with_grouping(&grouping, mode)?;
                }
                let mut pcg = PreparedPcgWorkspace::try_new(&owner, PreparedPcgOptions::default())?;
                assert_eq!(pcg.payload_report(0)?.hierarchy.terminal_payload_bytes, 0);
                #[cfg(feature = "lsmr")]
                let mut lsmr =
                    multiway_mg::PreparedLsmrWorkspace::try_new(&owner, Default::default())?;
                for column in 0..3 {
                    let y: Vec<_> = t
                        .topology()
                        .tuples()
                        .iter()
                        .enumerate()
                        .map(|(i, k)| match column {
                            0 => (i as f64 * 0.31).sin(),
                            1 => {
                                exact[k[0] as usize]
                                    + exact[offsets[1] + k[1] as usize]
                                    + exact[offsets[2] + k[2] as usize]
                            }
                            _ => 0.,
                        })
                        .collect();
                    let result = solve_prepared_pcg_least_squares(&owner, &y, &mut pcg)?;
                    assert!(
                        result.report.accepted,
                        "n={n} e={e} generation={generation} kind={kind:?} mode={mode:?}: {:?}",
                        result.report
                    );
                    if mode.is_none() {
                        scalar_reports.push(result.report);
                        scalar_coefficients.push(
                            result
                                .coefficients
                                .iter()
                                .map(|x| x.to_bits())
                                .collect::<Vec<_>>(),
                        );
                    } else {
                        assert_eq!(result.report, scalar_reports[column]);
                        assert_eq!(
                            result
                                .coefficients
                                .iter()
                                .map(|x| x.to_bits())
                                .collect::<Vec<_>>(),
                            scalar_coefficients[column]
                        );
                    }
                    #[cfg(feature = "lsmr")]
                    {
                        let result =
                            multiway_mg::solve_prepared_least_squares_with_certificate_gate(
                                &owner, &y, &mut lsmr,
                            )?;
                        assert!(
                            result.report.solve.accepted,
                            "n={n} e={e} kind={kind:?} mode={mode:?}: {:?}",
                            result.report
                        );
                        let reference = (
                            result.report,
                            result
                                .coefficients
                                .iter()
                                .map(|x| x.to_bits())
                                .collect::<Vec<_>>(),
                        );
                        if mode.is_none() {
                            lsmr_references.push(reference);
                        } else {
                            assert_eq!(reference, lsmr_references[column]);
                        }
                    }
                }
            }
        }
    }
    Ok(())
}
#[test]
fn large_baselines_certify_without_dense_terminals_and_layouts_match_exactly() -> Result {
    let n = 128u32;
    let keys: Vec<_> = (0..n)
        .flat_map(|i| (0..n).map(move |j| [i, j, (i + j) % n]))
        .collect();
    check_solves(ThreeWayProblem::from_observations(
        [n as usize; 3],
        &keys,
        &vec![1.; keys.len()],
    )?)?;
    let keys: Vec<_> = (0..n)
        .flat_map(|i| [[i, i, i], [i, (i + 1) % n, (i + 1) % n]])
        .collect();
    let weights: Vec<_> = (0..keys.len())
        .map(|i| if i % 2 == 0 { 1. } else { 1. / 64. })
        .collect();
    check_solves(ThreeWayProblem::from_observations(
        [n as usize; 3],
        &keys,
        &weights,
    )?)?;
    Ok(())
}
#[test]
fn baseline_batches_keep_independent_lane_reports_and_reject_wrong_owners() -> Result {
    let t = PreparedThreeWayTopology::try_from_collapsed([1; 3], &[[0; 3]])?;
    let f = ThreeWayWeightFrame::try_new(&t, WeightFrameInput::UnitTuples)?;
    for kind in KINDS {
        let owner = PreparedBaseline::new(&f, kind);
        let other = PreparedBaseline::new(&f, kind);
        let mut pcg = PreparedPcgWorkspace::try_new(&owner, Default::default())?;
        #[cfg(feature = "lsmr")]
        let mut lsmr = multiway_mg::PreparedLsmrWorkspace::try_new(&owner, Default::default())?;
        for count in [1, 2, 4, 8, 16, 17, 32] {
            let targets: Vec<_> = (0..count)
                .map(|i| {
                    if i % 3 == 0 {
                        0.
                    } else {
                        (i as f64 * 0.37).cos()
                    }
                })
                .collect();
            let mut output = vec![f64::NAN; 3 * count];
            let mut reports = vec![None; count];
            solve_prepared_pcg_batch_into(
                &owner,
                &targets,
                count,
                &mut output,
                &mut reports,
                &mut pcg,
            )?;
            assert!(reports.iter().all(|x| x.unwrap().accepted));
            let saved = output.clone();
            let old_reports = reports.clone();
            let work = pcg.last_work();
            assert!(
                solve_prepared_pcg_batch_into(
                    &other,
                    &targets,
                    count,
                    &mut output,
                    &mut reports,
                    &mut pcg
                )
                .is_err()
            );
            assert_eq!(output, saved);
            assert_eq!(reports, old_reports);
            assert_eq!(pcg.last_work(), work);
            #[cfg(feature = "lsmr")]
            {
                let mut reports = vec![None; count];
                multiway_mg::solve_prepared_least_squares_with_certificate_gate_batch_into(
                    &owner,
                    &targets,
                    count,
                    &mut output,
                    &mut reports,
                    &mut lsmr,
                )?;
                assert!(reports.iter().all(|x| x.unwrap().solve.accepted));
                let saved = output.clone();
                let old_reports = reports.clone();
                let work = lsmr.last_work();
                let gate = lsmr.last_gate_work();
                assert!(
                    multiway_mg::solve_prepared_least_squares_with_certificate_gate_batch_into(
                        &other,
                        &targets,
                        count,
                        &mut output,
                        &mut reports,
                        &mut lsmr
                    )
                    .is_err()
                );
                assert_eq!(output, saved);
                assert_eq!(reports, old_reports);
                assert_eq!(lsmr.last_work(), work);
                assert_eq!(lsmr.last_gate_work(), gate);
            }
        }
    }
    Ok(())
}

#[cfg(feature = "lsmr")]
#[allow(dead_code)]
#[path = "../examples/support/issue3_recursive_fixtures.rs"]
mod fixtures;
#[cfg(feature = "lsmr")]
#[test]
fn baseline_native_candidates_and_certificate_gate_remain_separate() -> Result {
    use multiway_mg::{
        PreparedLsmrWorkspace, solve_prepared_least_squares,
        solve_prepared_least_squares_with_certificate_gate,
    };
    let mut rejected_native = 0;
    let mut vetoes = 0;
    let mut solved = 0;
    for fixture in fixtures::recursive_holdout_fixtures()? {
        let t = PreparedThreeWayTopology::try_from_collapsed(
            fixture.problem.topology().level_counts(),
            fixture.problem.topology().tuples(),
        )?;
        for generation in 0..2 {
            let weights: Vec<_> = fixture
                .problem
                .weights()
                .iter()
                .enumerate()
                .map(|(i, w)| w * (1. + generation as f64 * (i % 7) as f64 / 8.))
                .collect();
            let f = ThreeWayWeightFrame::try_new(&t, WeightFrameInput::Tuples(&weights))?;
            let owner = PreparedBaseline::new(&f, PreparedBaselineKind::SymmetricMap);
            let mut native = PreparedLsmrWorkspace::try_new(&owner, Default::default())?;
            let mut gated = PreparedLsmrWorkspace::try_new(&owner, Default::default())?;
            for column in 0..3 {
                let targets: Vec<_> = (0..f.weights().len())
                    .map(|i| {
                        if column == 2 {
                            0.
                        } else {
                            ((i + column) as f64 * 0.31).sin()
                        }
                    })
                    .collect();
                let a = solve_prepared_least_squares(&owner, &targets, &mut native)?;
                let b = solve_prepared_least_squares_with_certificate_gate(
                    &owner, &targets, &mut gated,
                )?;
                assert!(
                    b.report.solve.accepted,
                    "{} generation={generation} column={column}: {:?}",
                    fixture.name, b.report
                );
                if a.report.accepted {
                    assert_eq!(
                        a.coefficients
                            .iter()
                            .map(|x| x.to_bits())
                            .collect::<Vec<_>>(),
                        b.coefficients
                            .iter()
                            .map(|x| x.to_bits())
                            .collect::<Vec<_>>()
                    );
                } else {
                    rejected_native += 1;
                    assert!(b.report.gate.candidate_vetoes > 0);
                }
                println!(
                    "baseline MAP native/gated {} generation={} column={} native_accepted={} native_certificate={:.17e} native_iterations={} gated_certificate={:.17e} gated_iterations={} vetoes={}",
                    fixture.name,
                    generation,
                    column,
                    a.report.accepted,
                    a.report.certified_normal_equation_residual,
                    a.report.iterations,
                    b.report.solve.certified_normal_equation_residual,
                    b.report.solve.iterations,
                    b.report.gate.candidate_vetoes
                );
                vetoes += b.report.gate.candidate_vetoes;
                solved += 1;
                assert_eq!(
                    native.retained_payload_bytes()?,
                    gated.retained_payload_bytes()?
                );
            }
        }
    }
    println!(
        "baseline MAP certificate-gate diagnostics: solved={solved} native_rejections={rejected_native} candidate_vetoes={vetoes}"
    );
    Ok(())
}
