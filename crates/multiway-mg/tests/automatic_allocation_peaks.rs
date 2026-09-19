//! Isolated single-thread peak-heap proof, including setup and released owners.
use allocation_counter::{AllocationInfo, measure};
use multiway_incidence::{
    PreparedComponentLayout, PreparedComponentRecoding, PreparedComponentRoot,
    PreparedHierarchyBudget, PreparedThreeWayTopology, ThreeWayWeightFrame, WeightFrameInput,
};
use std::hint::black_box;
type Result<T = ()> = std::result::Result<T, Box<dyn std::error::Error>>;
const B: PreparedHierarchyBudget = PreparedHierarchyBudget::UNLIMITED;

fn released(s: AllocationInfo) {
    assert_eq!(s.count_current, 0, "{s:?}");
    assert_eq!(s.bytes_current, 0, "{s:?}");
}
fn controls() {
    let s = measure(|| {
        let a = black_box(vec![0u8; black_box(1024)]);
        let b = black_box(vec![0u8; black_box(2048)]);
        black_box((&a, &b));
    });
    released(s);
    assert_eq!((s.count_total, s.bytes_total, s.bytes_max), (2, 3072, 3072));
    let s = measure(|| {
        drop(black_box(vec![0u8; black_box(3072)]));
        drop(black_box(vec![0u8; black_box(1024)]));
    });
    released(s);
    assert_eq!((s.bytes_total, s.bytes_max), (4096, 3072));
    let s = measure(|| {
        let mut v = black_box(vec![0u8; black_box(8)]);
        v.reserve_exact(black_box(4096));
        black_box(&v);
    });
    released(s);
    assert!(s.count_total >= 2 && s.bytes_max >= 4112);
}
fn roots() -> Result {
    let t = PreparedThreeWayTopology::try_from_collapsed(
        [4; 3],
        &[
            [0, 0, 1],
            [0, 2, 3],
            [1, 1, 0],
            [2, 0, 1],
            [2, 2, 3],
            [3, 3, 2],
        ],
    )?;
    let f = ThreeWayWeightFrame::try_new(&t, WeightFrameInput::UnitTuples)?;
    let l = PreparedComponentLayout::try_new(&t, B)?;
    let r = PreparedComponentRecoding::try_new(&l, B)?;
    let base =
        t.retained_payload_bytes()? + l.retained_payload_bytes()? + r.retained_payload_bytes()?;
    let bound = PreparedComponentRoot::setup_payload_bound(&r, 0, B)?;
    let s = measure(|| {
        let root = PreparedComponentRoot::try_new(&r, 0, B).unwrap();
        assert_eq!(root.component().tuple_count(), 4);
        assert_eq!(root.component().dimension(), 6);
        black_box(root);
    });
    released(s);
    assert_eq!(s.count_total, 3);
    assert_eq!(s.bytes_total as usize, bound - base);
    assert_eq!(s.bytes_max, s.bytes_total);
    let s = measure(|| {
        assert!(
            PreparedComponentRoot::try_new(
                &r,
                0,
                PreparedHierarchyBudget {
                    maximum_payload_bytes: bound - 1,
                    ..B
                }
            )
            .is_err()
        );
    });
    assert_eq!(s, AllocationInfo::default());
    let root = PreparedComponentRoot::try_new(&r, 0, B)?;
    let base = t.retained_payload_bytes()?
        + l.retained_payload_bytes()?
        + root.retained_payload_bytes()?
        + f.retained_payload_bytes()?;
    let bound = root.weight_frame_setup_payload_bound(&f, B)?;
    let s = measure(|| {
        black_box(root.try_weight_frame(&f, B).unwrap());
    });
    released(s);
    assert_eq!(s.count_total, 5);
    assert!(s.bytes_max as usize <= bound - base);
    let s = measure(|| {
        assert!(
            root.try_weight_frame(
                &f,
                PreparedHierarchyBudget {
                    maximum_payload_bytes: bound - 1,
                    ..B
                }
            )
            .is_err()
        );
    });
    assert_eq!(s, AllocationInfo::default());
    Ok(())
}

#[cfg(feature = "lsmr")]
mod automatic {
    use super::*;
    use multiway_mg::*;
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
    fn run(
        t: &PreparedThreeWayTopology,
        o: PreparedAutomaticOptions,
        expected: (usize, usize, usize, usize),
    ) -> Result {
        let f = ThreeWayWeightFrame::try_new(t, WeightFrameInput::UnitTuples)?;
        let e = f.weights().len();
        let v = f.diagonal().len();
        let mut reference = None;
        for k in [1, 2, 4, 8, 16, 17, 32] {
            let y: Vec<_> = (0..e * k)
                .map(|i| {
                    if i / e % 3 == 2 {
                        0.
                    } else {
                        (i % e) as f64 / e as f64
                    }
                })
                .collect();
            let mut x = vec![f64::NAN; v * k];
            let mut reports = vec![None; k];
            let base = t.retained_payload_bytes()?
                + f.retained_payload_bytes()?
                + 8 * (y.len() + x.len())
                + std::mem::size_of_val(reports.as_slice());
            for repeat in 0..2 {
                let mut admitted = 0;
                let s = measure(|| {
                    let mut p = PreparedAutomaticProgress::default();
                    solve_prepared_automatic_batch_into(
                        &f,
                        PreparedAutomaticBatch {
                            targets: &y,
                            columns: k,
                            coefficients: &mut x,
                            reports: &mut reports,
                        },
                        o,
                        B,
                        &mut p,
                    )
                    .unwrap();
                    assert_eq!(p.stage, PreparedAutomaticStage::Complete);
                    assert_eq!(
                        (
                            p.singleton_components,
                            p.dense_components,
                            p.accepted_hierarchies,
                            p.baseline_components
                        ),
                        expected
                    );
                    assert_eq!(p.global_fallback_columns, 0);
                    admitted = p.maximum_admitted_payload_bytes;
                });
                released(s);
                assert!(reports.iter().all(|r| r.is_some_and(|r| r.accepted)));
                {
                    assert!(
                        base + s.bytes_max as usize <= admitted,
                        "V={v} K={k}: {s:?}, base={base}, admitted={admitted}"
                    );
                }
                if let Some(r) = reference {
                    assert_eq!(s, r, "V={v} K={k} repeat={repeat}");
                } else {
                    reference = Some(s);
                }
            }
        }
        if expected == (0, 1, 0, 0) && t.component_factor_sizes().len() == 1 {
            assert_eq!(
                reference.unwrap().bytes_max as usize,
                8 * (2 * v * v + 3 * v - 2)
            );
        }
        println!(
            "V={v} E={e} route={expected:?} peak={:?}",
            reference.unwrap()
        );
        Ok(())
    }
    fn grouped_run(
        t: &PreparedThreeWayTopology,
        o: PreparedAutomaticOptions,
        expected: (usize, usize, usize, usize),
        layout: PreparedAutomaticLayout,
        changed_weights: bool,
    ) -> Result {
        let weights: Vec<_> = (0..t.topology().tuple_count())
            .map(|i| {
                1. + if changed_weights {
                    (i % 7) as f64 / 8.
                } else {
                    0.
                }
            })
            .collect();
        let f = ThreeWayWeightFrame::try_new(t, WeightFrameInput::Tuples(&weights))?;
        drop(weights);
        let e = f.weights().len();
        let v = f.diagonal().len();
        let mut reference = None;
        let mut layout_reference = None;
        for k in [1, 2, 4, 8, 16, 17, 32] {
            let y: Vec<_> = (0..e * k)
                .map(|i| {
                    if i / e % 3 == 2 {
                        0.
                    } else {
                        (i % e) as f64 / e as f64
                    }
                })
                .collect();
            let mut x = vec![f64::NAN; v * k];
            let mut reports = vec![None; k];
            let base = t.retained_payload_bytes()?
                + f.retained_payload_bytes()?
                + 8 * (y.len() + x.len())
                + std::mem::size_of_val(reports.as_slice());
            for _ in 0..2 {
                let mut admitted = 0;
                let mut layout_record = PreparedAutomaticLayoutProgress::default();
                let measured = measure(|| {
                    let mut progress = PreparedAutomaticProgress::default();
                    solve_prepared_automatic_batch_into_with_layout(
                        &f,
                        PreparedAutomaticBatch {
                            targets: &y,
                            columns: k,
                            coefficients: &mut x,
                            reports: &mut reports,
                        },
                        o,
                        B,
                        layout,
                        &mut progress,
                        &mut layout_record,
                    )
                    .unwrap();
                    assert_eq!(
                        (
                            progress.singleton_components,
                            progress.dense_components,
                            progress.accepted_hierarchies,
                            progress.baseline_components
                        ),
                        expected
                    );
                    assert_eq!(progress.global_fallback_columns, 0);
                    admitted = progress.maximum_admitted_payload_bytes;
                });
                released(measured);
                assert!(reports.iter().all(|r| r.is_some_and(|r| r.accepted)));
                assert!(
                    base + measured.bytes_max as usize <= admitted,
                    "base={base} peak={measured:?} admitted={admitted} layout={layout:?}"
                );
                if let Some(prior) = reference {
                    assert_eq!(measured, prior);
                } else {
                    reference = Some(measured);
                }
                if let Some(prior) = layout_reference {
                    assert_eq!(layout_record, prior);
                } else {
                    layout_reference = Some(layout_record);
                }
            }
        }
        println!(
            "grouped layout={layout:?} V={v} E={e} changed={changed_weights} route={expected:?} peak={:?} inventory={:?}",
            reference.unwrap(),
            layout_reference.unwrap()
        );
        Ok(())
    }
    fn denied_grouping(layout: PreparedAutomaticLayout) -> Result {
        use multiway_incidence::PreparedTupleGrouping;
        let keys: Vec<_> = (0..512)
            .flat_map(|i| (0..2).flat_map(move |j| (0..2).map(move |k| [i, j, k])))
            .collect();
        let t = PreparedThreeWayTopology::try_from_collapsed([512, 2, 2], &keys)?;
        let f = ThreeWayWeightFrame::try_new(&t, WeightFrameInput::UnitTuples)?;
        let y = vec![1.; keys.len()];
        let mut x = vec![17.; 516];
        let mut reports = [None];
        let caller = 8 * (y.len() + x.len()) + std::mem::size_of_val(&reports);
        let cap =
            PreparedTupleGrouping::setup_payload_bound(&t, caller + f.retained_payload_bytes()?)?
                - 1;
        let mut o = options();
        o.hierarchy = None;
        let measured = measure(|| {
            let mut p = PreparedAutomaticProgress::default();
            let mut g = PreparedAutomaticLayoutProgress::default();
            assert!(
                solve_prepared_automatic_batch_into_with_layout(
                    &f,
                    PreparedAutomaticBatch {
                        targets: &y,
                        columns: 1,
                        coefficients: &mut x,
                        reports: &mut reports,
                    },
                    o,
                    PreparedHierarchyBudget {
                        maximum_payload_bytes: cap,
                        ..B
                    },
                    layout,
                    &mut p,
                    &mut g
                )
                .is_err()
            );
            assert_eq!(g.grouping_attempts, 2);
            assert_eq!(g.rejected_groupings, 2);
            assert_eq!(g.completed_groupings, 0);
            assert_eq!(g.maximum_tuple_image_len, 0);
            assert!(p.maximum_requested_payload_bytes > cap);
            assert!(p.maximum_admitted_payload_bytes <= cap);
        });
        assert_eq!(measured, AllocationInfo::default());
        assert_eq!(reports, [None]);
        println!("denied grouped layout={layout:?} allocations={measured:?}");
        Ok(())
    }
    fn grouped() -> Result {
        let layouts = [
            PreparedAutomaticLayout::FineRow,
            PreparedAutomaticLayout::AllRow,
            PreparedAutomaticLayout::FineImage,
            PreparedAutomaticLayout::AllImage,
        ];
        for layout in layouts {
            denied_grouping(layout)?;
        }
        for n in [130, 516, 1028] {
            let keys: Vec<_> = (0..n - 4)
                .flat_map(|i| (0..2).flat_map(move |j| (0..2).map(move |k| [i, j, k])))
                .collect();
            let t = PreparedThreeWayTopology::try_from_collapsed([(n - 4) as usize, 2, 2], &keys)?;
            for layout in layouts {
                grouped_run(
                    &t,
                    options(),
                    if n <= 256 { (0, 1, 0, 0) } else { (0, 0, 1, 0) },
                    layout,
                    false,
                )?;
                if n == 516 {
                    grouped_run(&t, options(), (0, 0, 1, 0), layout, true)?;
                    let mut o = options();
                    o.hierarchy.as_mut().unwrap().maximum_transitions = 0;
                    grouped_run(&t, o, (0, 0, 0, 1), layout, false)?;
                }
            }
        }
        let mut keys: Vec<_> = (0..2)
            .flat_map(|c| {
                (0..256).flat_map(move |i| {
                    (0..2)
                        .flat_map(move |j| (0..2).map(move |k| [256 * c + i, 2 * c + j, 2 * c + k]))
                })
            })
            .collect();
        keys.extend(
            (0..2).flat_map(|i| {
                (0..2).flat_map(move |j| (0..2).map(move |k| [512 + i, 4 + j, 4 + k]))
            }),
        );
        keys.push([514, 6, 6]);
        keys.sort_unstable();
        let t = PreparedThreeWayTopology::try_from_collapsed([515, 7, 7], &keys)?;
        for layout in layouts {
            grouped_run(&t, options(), (1, 1, 2, 0), layout, false)?;
            let mut o = options();
            o.hierarchy = None;
            grouped_run(&t, o, (1, 1, 0, 2), layout, false)?;
        }
        Ok(())
    }
    pub fn check() -> Result {
        let keys: Vec<_> = (0..300).map(|i| [i, 299 - i, (i + 117) % 300]).collect();
        run(
            &PreparedThreeWayTopology::try_from_collapsed([300; 3], &keys)?,
            options(),
            (300, 0, 0, 0),
        )?;
        for n in [6, 130, 255, 256, 516, 1028] {
            let keys: Vec<_> = (0..n - 4)
                .flat_map(|i| (0..2).flat_map(move |j| (0..2).map(move |k| [i, j, k])))
                .collect();
            let t = PreparedThreeWayTopology::try_from_collapsed([(n - 4) as usize, 2, 2], &keys)?;
            run(
                &t,
                options(),
                if n <= 256 { (0, 1, 0, 0) } else { (0, 0, 1, 0) },
            )?;
            if n > 256 {
                let mut o = options();
                o.hierarchy.as_mut().unwrap().maximum_transitions = 0;
                run(&t, o, (0, 0, 0, 1))?;
            }
        }
        // Release each dense factor after all K columns, before the next component.
        let keys: Vec<_> = (0..40)
            .flat_map(|c| {
                (0..2).flat_map(move |i| {
                    (0..2).flat_map(move |j| (0..2).map(move |k| [2 * c + i, 2 * c + j, 2 * c + k]))
                })
            })
            .collect();
        run(
            &PreparedThreeWayTopology::try_from_collapsed([80; 3], &keys)?,
            options(),
            (0, 40, 0, 0),
        )?;
        // Independently materialized large roots also have disjoint lifetimes.
        let keys: Vec<_> = (0..2)
            .flat_map(|c| {
                (0..512).flat_map(move |i| {
                    (0..2)
                        .flat_map(move |j| (0..2).map(move |k| [512 * c + i, 2 * c + j, 2 * c + k]))
                })
            })
            .collect();
        run(
            &PreparedThreeWayTopology::try_from_collapsed([1024, 4, 4], &keys)?,
            options(),
            (0, 0, 2, 0),
        )?;
        grouped()?;
        Ok(())
    }
}
fn main() -> Result {
    controls();
    roots()?;
    #[cfg(all(feature = "lsmr", not(feature = "profiling")))]
    automatic::check()?;
    #[cfg(all(feature = "lsmr", feature = "profiling"))]
    {
        // Initialize fixed profiler TLS before individual allocation regions.
        // Every measured solve then executes active hooks; their heap delta must
        // remain exactly the same as the frozen uninstrumented controls.
        let (result, report) = multiway_mg::automatic_profiling::collect(automatic::check)?;
        result?;
        assert!(report.valid, "{report:?}");
        assert_eq!(
            report.elapsed_ns,
            report.unattributed_ns + report.regions.iter().map(|r| r.elapsed_ns).sum::<u128>()
        );
        println!(
            "automatic diagnostic regions valid; TLS bytes={} report={report:?}",
            multiway_mg::automatic_profiling::thread_local_payload_bytes()
        );
    }
    println!("component roots and automatic live-allocation peaks passed");
    Ok(())
}
