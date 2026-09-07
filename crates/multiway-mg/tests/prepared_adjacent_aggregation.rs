//! Explicit full-row policy has an independent ordered-map reference.
use multiway_incidence::{
    FactorAggregation, PreparedHierarchyBudget, PreparedThreeWayTopology, ThreeWayWeightFrame,
    WeightFrameInput,
};
use multiway_mg::{
    PairNeighborhoodAggregationOptions, PreparedPairNeighborhoodCandidate,
    PreparedPairProposalCoverage,
};
use std::collections::BTreeMap;
fn reference(frame: &ThreeWayWeightFrame<'_>, threshold: f64) -> FactorAggregation {
    let counts = frame.topology().topology().level_counts();
    let offsets = frame.topology().topology().offsets();
    let mut parents = [Vec::new(), Vec::new(), Vec::new()];
    for q in 0..3 {
        let mut overlaps = BTreeMap::<(u32, u32), f64>::new();
        for neighbor in 0..3 {
            if neighbor != q {
                let mut rows = BTreeMap::<u32, BTreeMap<u32, f64>>::new();
                for (key, &weight) in frame
                    .topology()
                    .topology()
                    .tuples()
                    .iter()
                    .zip(frame.weights())
                {
                    *rows
                        .entry(key[neighbor])
                        .or_default()
                        .entry(key[q])
                        .or_default() += weight;
                }
                for row in rows.values() {
                    let mut entries: Vec<_> = row.iter().map(|(&id, &w)| (id, w)).collect();
                    entries.sort_by(|a, b| b.1.total_cmp(&a.1).then(a.0.cmp(&b.0)));
                    for pair in entries.chunks_exact(2) {
                        let edge = (pair[0].0.min(pair[1].0), pair[0].0.max(pair[1].0));
                        *overlaps.entry(edge).or_default() += pair[0].1.min(pair[1].1);
                    }
                }
            }
        }
        let mut edges: Vec<_> = overlaps
            .into_iter()
            .map(|((a, b), mass)| {
                let denom = frame.diagonal()[offsets[q] + a as usize].sqrt()
                    * frame.diagonal()[offsets[q] + b as usize].sqrt();
                (a, b, (mass / denom).clamp(0., 1.))
            })
            .filter(|e| e.2 >= threshold)
            .collect();
        edges.sort_by(|a, b| b.2.total_cmp(&a.2).then(a.0.cmp(&b.0)).then(a.1.cmp(&b.1)));
        let mut mates = vec![None; counts[q]];
        for (a, b, _) in edges {
            if mates[a as usize].is_none() && mates[b as usize].is_none() {
                mates[a as usize] = Some(b);
                mates[b as usize] = Some(a);
            }
        }
        parents[q] = vec![u32::MAX; counts[q]];
        let mut next = 0;
        for id in 0..counts[q] {
            if parents[q][id] == u32::MAX {
                parents[q][id] = next;
                if let Some(other) = mates[id] {
                    parents[q][other as usize] = next;
                }
                next += 1;
            }
        }
    }
    FactorAggregation::new(counts, parents).unwrap()
}
fn compare(counts: [usize; 3], mut keys: Vec<[u32; 3]>) {
    keys.sort_unstable();
    keys.dedup();
    let t = PreparedThreeWayTopology::try_from_collapsed(counts, &keys).unwrap();
    for weights in [
        vec![1.; keys.len()],
        (0..keys.len())
            .map(|i| 0.5 + (i % 11) as f64 / 10.)
            .collect(),
        (0..keys.len())
            .map(|i| 2f64.powi((i % 15) as i32 - 7))
            .collect(),
    ] {
        let f = ThreeWayWeightFrame::try_new(&t, WeightFrameInput::Tuples(&weights)).unwrap();
        for threshold in [0., 0.02, 0.2, 1.] {
            let expected = reference(&f, threshold);
            let candidate = PreparedPairNeighborhoodCandidate::try_adjacent_pairs(
                &f,
                threshold,
                PreparedHierarchyBudget::UNLIMITED,
            )
            .unwrap();
            assert_eq!(candidate.aggregation(), &expected);
            assert_eq!(
                candidate.setup_report().coverage,
                PreparedPairProposalCoverage::AdjacentPairs
            );
            assert!(candidate.setup_report().maximum_proposals <= keys.len());
            assert!(
                candidate.work_report().proposals <= 3 * candidate.setup_report().maximum_proposals
            );
            assert_eq!(candidate.work_report().truncated_neighbors, 0);
            assert_eq!(candidate.work_report().tuple_visits, 6 * keys.len());
        }
    }
}
#[test]
fn full_row_maps_match_independent_reference() {
    compare(
        [3, 4, 5],
        (0..3)
            .flat_map(|a| (0..4).flat_map(move |b| (0..5).map(move |c| [a, b, c])))
            .collect(),
    );
    compare(
        [4, 3, 3],
        (0..4)
            .flat_map(|a| (0..3).map(move |b| [a, b, b]))
            .collect(),
    );
    let mut keys: Vec<_> = (0..4)
        .flat_map(|a| (0..3).map(move |b| [a, b, b]))
        .collect();
    keys.push([4, 3, 3]);
    compare([5, 4, 4], keys);
    compare(
        [12, 3, 2],
        (0..12)
            .flat_map(|a| (0..3).flat_map(move |b| (0..2).map(move |c| [a, b, c])))
            .collect(),
    );
    compare([1; 3], vec![[0; 3]]);
}
#[test]
fn tied_latin_coverage_has_explicit_work_and_memory_tradeoff() {
    let n = 128u32;
    let keys: Vec<_> = (0..n)
        .flat_map(|a| (0..n).map(move |b| [a, b, (a + b) % n]))
        .collect();
    let t = PreparedThreeWayTopology::try_from_collapsed([n as usize; 3], &keys).unwrap();
    let f = ThreeWayWeightFrame::try_new(&t, WeightFrameInput::UnitTuples).unwrap();
    let legacy = PreparedPairNeighborhoodCandidate::try_new(
        &f,
        PairNeighborhoodAggregationOptions {
            minimum_affinity: 0.02,
            maximum_neighbor_degree: 4,
        },
        PreparedHierarchyBudget::UNLIMITED,
    )
    .unwrap();
    let adjacent = PreparedPairNeighborhoodCandidate::try_adjacent_pairs(
        &f,
        0.02,
        PreparedHierarchyBudget::UNLIMITED,
    )
    .unwrap();
    assert_eq!(legacy.aggregation().coarse_counts(), [126; 3]);
    assert_eq!(adjacent.aggregation().coarse_counts(), [64; 3]);
    assert_eq!(legacy.work_report().accepted_pairs, 6);
    assert_eq!(adjacent.work_report().accepted_pairs, 192);
    assert_eq!(legacy.setup_report().maximum_proposals, 1536);
    assert_eq!(adjacent.setup_report().maximum_proposals, 16384);
    assert_eq!(legacy.work_report().proposals, 4608);
    assert_eq!(adjacent.work_report().proposals, 49152);
    assert_eq!(
        adjacent.setup_report().total_payload_bound - legacy.setup_report().total_payload_bound,
        356352
    );
    assert_eq!(adjacent.aggregation(), &reference(&f, 0.02));
}

#[allow(dead_code)]
#[path = "../examples/support/issue3_recursive_fixtures.rs"]
mod fixtures;

fn qualify_cycles(
    problem: multiway_mg::ThreeWayProblem,
    label: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    use multiway_incidence::{
        HierarchyWeightFrames, PreparedHierarchyBuilder, PreparedHierarchyLimits,
        PreparedProvisionalFrame, ProvisionalWeightInput,
    };
    use multiway_mg::{
        CycleQualityCriteria, CycleQualityOptions, CycleScreenedMapHierarchy,
        PreparedCycleScreenWorkspace, PreparedMapHierarchy, PreparedPcgOptions,
        PreparedPcgWorkspace, ThreeWayProblem, analyze_cycle_quality,
        solve_prepared_pcg_least_squares,
    };
    const B: PreparedHierarchyBudget = PreparedHierarchyBudget::UNLIMITED;
    let t = PreparedThreeWayTopology::try_from_collapsed(
        problem.topology().level_counts(),
        problem.topology().tuples(),
    )?;
    for generation in 0..2 {
        let weights: Vec<_> = problem
            .weights()
            .iter()
            .enumerate()
            .map(|(i, w)| {
                if generation == 0 {
                    *w
                } else {
                    w * (0.5 + (i % 7) as f64 / 8.)
                }
            })
            .collect();
        let fine = ThreeWayWeightFrame::try_new(&t, WeightFrameInput::Tuples(&weights))?;
        let mut b = PreparedHierarchyBuilder::try_new(
            &t,
            PreparedHierarchyLimits {
                maximum_transitions: 2,
                maximum_total_tuples: 3 * weights.len(),
                maximum_total_coefficients: 3 * fine.diagonal().len(),
                require_strict_dimension_reduction: true,
            },
            B,
        )?;
        let mut map = PreparedPairNeighborhoodCandidate::try_adjacent_pairs(&fine, 0.02, B)?
            .into_aggregation();
        let mut previous = None;
        // Force at most two proposals for qualification, even for an already-small
        // problem. This is not the future terminal-first automatic route.
        for _ in 0..2 {
            if map.coarse_counts().iter().sum::<usize>()
                == b.current_level().topology().total_levels()
            {
                break;
            }
            b.try_append(map, B)?;
            let input = previous
                .take()
                .map(ProvisionalWeightInput::Owned)
                .unwrap_or(ProvisionalWeightInput::Frame(&fine));
            let p = PreparedProvisionalFrame::try_replay_last(&b, input, B)?;
            let next = PreparedPairNeighborhoodCandidate::try_adjacent_pairs(p.frame(), 0.02, B)?;
            assert_eq!(next.aggregation(), &reference(p.frame(), 0.02));
            map = next.into_aggregation();
            previous = Some(p.into_tuple_weights());
        }
        let h = b.finish();
        let frames = HierarchyWeightFrames::try_new(&h, &fine)?;
        if let Some(w) = previous {
            assert_eq!(frames.frame(frames.level_count() - 1).unwrap().weights(), w);
        }
        let numerical = PreparedMapHierarchy::try_new(&frames, 1e-12)?;
        let options = CycleQualityOptions {
            test_vectors: 2,
            power_iterations: 8,
            tail_iterations: 4,
            ..Default::default()
        };
        // Explicit development diagnostic, selected before these results.
        let criteria = CycleQualityCriteria {
            maximum_estimated_energy_factor: 0.8,
            maximum_observed_energy_factor: Some(1.05),
            maximum_structural_defect: 1e-10,
        };
        let mut cycle = numerical.application_workspace()?;
        let mut screen = PreparedCycleScreenWorkspace::try_new(&numerical, &cycle, B)?;
        let screened = screen.screen(options, criteria, &mut cycle)?;
        let mut expected_acceptance = true;
        for level in (0..h.level_count()).rev() {
            let frame = frames.frame(level).unwrap();
            let local = ThreeWayProblem::from_observations(
                frame.topology().topology().level_counts(),
                frame.topology().topology().tuples(),
                frame.weights(),
            )?;
            let maps = (level..h.level_count() - 1)
                .map(|i| h.aggregation(i).unwrap().clone())
                .collect();
            let ordinary = CycleScreenedMapHierarchy::from_maps(local.clone(), maps, 1e-12)?;
            let expected = analyze_cycle_quality(&local, &ordinary, options)?;
            let accepted = expected.maximum_estimated_energy_factor()
                <= criteria.maximum_estimated_energy_factor
                && expected.maximum_observed_energy_factor()
                    <= criteria.maximum_observed_energy_factor.unwrap()
                && expected.maximum_structural_defect() <= criteria.maximum_structural_defect;
            if expected_acceptance {
                let actual = &screened.levels[h.level_count() - 1 - level];
                assert_eq!(actual.level, level);
                assert_eq!(actual.accepted, accepted);
                assert_eq!(
                    actual.maximum_estimated_energy_factor.to_bits(),
                    expected.maximum_estimated_energy_factor().to_bits()
                );
                assert_eq!(
                    actual.maximum_observed_energy_factor.to_bits(),
                    expected.maximum_observed_energy_factor().to_bits()
                );
                assert_eq!(
                    actual.maximum_structural_defect.to_bits(),
                    expected.maximum_structural_defect().to_bits()
                );
            }
            expected_acceptance &= accepted;
            println!(
                "adjacent qualification {label} generation={generation} level={level} dimension={} estimated={:.17e} observed={:.17e} defect={:.17e} accepted={accepted}",
                local.dimension(),
                expected.maximum_estimated_energy_factor(),
                expected.maximum_observed_energy_factor(),
                expected.maximum_structural_defect()
            );
        }
        assert_eq!(screened.accepted, expected_acceptance);
        drop(screen);
        drop(cycle);
        // Forced diagnostic solves also run after a quality rejection. They do
        // not turn a rejected screen into a promoted automatic route.
        let mut pcg = PreparedPcgWorkspace::try_new(&numerical, PreparedPcgOptions::default())?;
        #[cfg(feature = "lsmr")]
        let mut lsmr = multiway_mg::PreparedLsmrWorkspace::try_new(
            &numerical,
            multiway_mg::PreparedLsmrOptions::default(),
        )?;
        let offsets = t.topology().offsets();
        let exact: Vec<_> = (0..fine.diagonal().len())
            .map(|i| (i as f64 * 0.73).sin())
            .collect();
        for column in 0..3 {
            let targets: Vec<_> = t
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
            let result = solve_prepared_pcg_least_squares(&numerical, &targets, &mut pcg)?;
            println!(
                "adjacent diagnostic solve {label} generation={generation} column={column} pcg_accepted={} certificate={:.17e}",
                result.report.accepted, result.report.certified_normal_equation_residual
            );
            assert!(result.report.accepted);
            #[cfg(feature = "lsmr")]
            {
                let result = multiway_mg::solve_prepared_least_squares_with_certificate_gate(
                    &numerical, &targets, &mut lsmr,
                )?;
                println!(
                    "adjacent diagnostic solve {label} generation={generation} column={column} lsmr_accepted={}",
                    result.report.solve.accepted
                );
                assert!(result.report.solve.accepted);
            }
        }
    }
    Ok(())
}

#[test]
fn provisional_adjacent_maps_replay_screen_and_certify() -> Result<(), Box<dyn std::error::Error>> {
    for f in fixtures::recursive_holdout_fixtures()? {
        qualify_cycles(f.problem, &f.name)?;
    }
    let keys: Vec<_> = (0..128)
        .flat_map(|i| (0..128).map(move |j| [i, j, (i + j) % 128]))
        .collect();
    qualify_cycles(
        multiway_mg::ThreeWayProblem::from_observations([128; 3], &keys, &vec![1.; keys.len()])?,
        "tied-latin-128",
    )?;
    let mut keys: Vec<_> = (0..4)
        .flat_map(|i| (0..3).map(move |j| [i, j, j]))
        .collect();
    keys.push([4, 3, 3]);
    qualify_cycles(
        multiway_mg::ThreeWayProblem::from_observations([5, 4, 4], &keys, &[1.; 13])?,
        "nested-plus-singleton-additional-nullity",
    )?;
    qualify_cycles(
        multiway_mg::ThreeWayProblem::from_observations([1; 3], &[[0; 3]], &[1.])?,
        "singleton-zero-proposals",
    )?;
    Ok(())
}
