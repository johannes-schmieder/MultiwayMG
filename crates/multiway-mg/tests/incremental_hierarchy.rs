//! Incremental admission produces identical supplied structure and complete solves.
#[allow(dead_code)]
#[path = "../examples/support/issue3_recursive_fixtures.rs"]
mod fixtures;
use multiway_incidence::{
    FactorAggregation, HierarchyWeightFrames, PreparedHierarchyBudget, PreparedHierarchyBuilder,
    PreparedHierarchyLimits, PreparedHierarchyTopology, PreparedThreeWayTopology,
    ThreeWayWeightFrame, WeightFrameInput,
};
use multiway_mg::{
    PreparedMapHierarchy, PreparedPcgOptions, PreparedPcgWorkspace,
    solve_prepared_pcg_least_squares,
};
const B: PreparedHierarchyBudget = PreparedHierarchyBudget::UNLIMITED;
fn limits(n: usize) -> PreparedHierarchyLimits {
    PreparedHierarchyLimits {
        maximum_transitions: n,
        maximum_total_tuples: usize::MAX,
        maximum_total_coefficients: usize::MAX,
        require_strict_dimension_reduction: true,
    }
}
fn bits(a: &[f64], b: &[f64]) {
    assert_eq!(a.len(), b.len());
    for (a, b) in a.iter().zip(b) {
        assert_eq!(a.to_bits(), b.to_bits());
    }
}
fn structure(a: &PreparedHierarchyTopology<'_>, b: &PreparedHierarchyTopology<'_>) {
    assert_eq!(a.level_count(), b.level_count());
    for i in 0..a.level_count() {
        let x = a.level(i).unwrap();
        let y = b.level(i).unwrap();
        assert_eq!(x.topology().level_counts(), y.topology().level_counts());
        assert_eq!(x.topology().tuples(), y.topology().tuples());
        assert_eq!(a.aggregation(i), b.aggregation(i));
        assert_eq!(
            a.coarse_to_fine_components(i),
            b.coarse_to_fine_components(i)
        );
        assert_eq!(
            a.fine_to_coarse_components(i),
            b.fine_to_coarse_components(i)
        );
        if let Some(x) = a.merge_groups(i) {
            let y = b.merge_groups(i).unwrap();
            assert_eq!(x.offsets(), y.offsets());
            assert_eq!(x.source_to_group(), y.source_to_group());
            assert_eq!(x.grouped_sources(), y.grouped_sources());
        }
    }
}
#[test]
fn complete_recursive_solvers_and_changed_weights_match_supplied_construction()
-> Result<(), Box<dyn std::error::Error>> {
    for fixture in fixtures::recursive_holdout_fixtures()? {
        let t = PreparedThreeWayTopology::try_from_collapsed(
            fixture.problem.topology().level_counts(),
            fixture.problem.topology().tuples(),
        )?;
        let expected = PreparedHierarchyTopology::try_new(&t, fixture.oracle_maps.clone())?;
        let mut b =
            PreparedHierarchyBuilder::try_new(&t, limits(fixture.oracle_maps.len() + 2), B)?;
        for (i, map) in fixture.oracle_maps.into_iter().enumerate() {
            let report = b.try_append(map, B)?;
            assert_eq!(report.transition, i);
            assert_eq!(
                report.coarse_tuples,
                Some(b.current_level().topology().tuple_count())
            );
        }
        let actual = b.finish();
        structure(&actual, &expected);
        for generation in 0..3 {
            let weights: Vec<_> = fixture
                .problem
                .weights()
                .iter()
                .enumerate()
                .map(|(i, w)| w * (1. + ((i + generation) % 7) as f64))
                .collect();
            let f = ThreeWayWeightFrame::try_new(&t, WeightFrameInput::Tuples(&weights))?;
            let a = HierarchyWeightFrames::try_new(&actual, &f)?;
            let e = HierarchyWeightFrames::try_new(&expected, &f)?;
            for i in 0..a.level_count() {
                bits(a.frame(i).unwrap().weights(), e.frame(i).unwrap().weights());
                bits(
                    a.frame(i).unwrap().diagonal(),
                    e.frame(i).unwrap().diagonal(),
                );
            }
            let ah = PreparedMapHierarchy::try_new(&a, 1e-12)?;
            let eh = PreparedMapHierarchy::try_new(&e, 1e-12)?;
            let mut aw = PreparedPcgWorkspace::try_new(&ah, PreparedPcgOptions::default())?;
            let mut ew = PreparedPcgWorkspace::try_new(&eh, PreparedPcgOptions::default())?;
            #[cfg(feature = "lsmr")]
            let (mut al, mut el) = {
                use multiway_mg::{PreparedLsmrOptions, PreparedLsmrWorkspace};
                (
                    PreparedLsmrWorkspace::try_new(&ah, PreparedLsmrOptions::default())?,
                    PreparedLsmrWorkspace::try_new(&eh, PreparedLsmrOptions::default())?,
                )
            };
            for column in 0..32 {
                let y: Vec<_> = (0..f.weights().len())
                    .map(|i| {
                        if column == 16 || column == 31 {
                            0.0
                        } else {
                            ((i + column) as f64 * 0.31).sin()
                        }
                    })
                    .collect();
                let av = solve_prepared_pcg_least_squares(&ah, &y, &mut aw)?;
                let ev = solve_prepared_pcg_least_squares(&eh, &y, &mut ew)?;
                bits(av.coefficients, ev.coefficients);
                assert_eq!(av.report, ev.report);
                assert!(av.report.accepted);
                #[cfg(feature = "lsmr")]
                {
                    let av = multiway_mg::solve_prepared_least_squares_with_certificate_gate(
                        &ah, &y, &mut al,
                    )?;
                    let ev = multiway_mg::solve_prepared_least_squares_with_certificate_gate(
                        &eh, &y, &mut el,
                    )?;
                    bits(av.coefficients, ev.coefficients);
                    assert_eq!(av.report, ev.report);
                    assert!(av.report.solve.accepted);
                }
            }
        }
    }
    Ok(())
}
#[test]
fn ragged_disconnected_relabeling_and_extra_nullity_remain_exact() {
    let t =
        PreparedThreeWayTopology::try_from_collapsed([3, 2, 3], &[[0, 0, 0], [1, 0, 1], [2, 1, 2]])
            .unwrap();
    let map =
        FactorAggregation::new([3, 2, 3], [vec![1, 1, 0], vec![1, 0], vec![1, 1, 0]]).unwrap();
    let mut b = PreparedHierarchyBuilder::try_new(&t, limits(3), B).unwrap();
    b.try_append(map.clone(), B).unwrap();
    let expected = PreparedHierarchyTopology::try_new(&t, vec![map]).unwrap();
    structure(&b.finish(), &expected);
    let keys: Vec<_> = (0..8)
        .flat_map(|a| (0..8).map(move |b| [a, b, (a + b) % 8]))
        .collect();
    let t = PreparedThreeWayTopology::try_from_collapsed([8; 3], &keys).unwrap();
    let mut b = PreparedHierarchyBuilder::try_new(&t, limits(3), B).unwrap();
    let mut maps = vec![];
    for n in [8, 4, 2] {
        let map = FactorAggregation::consecutive_halving([n; 3]).unwrap();
        b.try_append(map.clone(), B).unwrap();
        maps.push(map);
    }
    structure(
        &b.finish(),
        &PreparedHierarchyTopology::try_new(&t, maps).unwrap(),
    );
}
