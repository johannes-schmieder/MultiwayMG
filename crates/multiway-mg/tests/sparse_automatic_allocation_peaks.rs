//! Isolated single-thread peak-heap proof for automatic sparse-terminal routes.
//!
//! The whole call, including sparse-terminal construction, screening, LSMR,
//! original certification, fallback and drop, runs inside one measured region.
//! Its observed live peak plus the caller's fixed owners must fit the driver's
//! admitted payload, nothing may be retained, and records repeat across K.
#![cfg(feature = "sparse-terminal")]
use allocation_counter::{AllocationInfo, measure};
use multiway_incidence::{
    PreparedHierarchyBudget, PreparedThreeWayTopology, ThreeWayWeightFrame, WeightFrameInput,
};
use multiway_mg::*;
type Result<T = ()> = std::result::Result<T, Box<dyn std::error::Error>>;

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

fn run(
    t: &PreparedThreeWayTopology,
    transitions: usize,
    layout: PreparedAutomaticLayout,
) -> Result {
    let policy = PreparedSparseTerminalOptions {
        maximum_dimension: 4096,
        local_solver: within::LocalSolverConfig::default(),
        projector_tolerance: 1e-12,
    };
    let f = ThreeWayWeightFrame::try_new(t, WeightFrameInput::UnitTuples)?;
    let (e, v) = (f.weights().len(), f.diagonal().len());
    let mut reference: Option<AllocationInfo> = None;
    let mut sparse_terminals = 0;
    for k in [1, 2, 8, 32] {
        let y: Vec<_> = (0..e * k).map(|i| ((i % e) as f64 * 0.37).sin()).collect();
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
                let mut l = PreparedAutomaticLayoutProgress::default();
                solve_prepared_automatic_batch_into_with_sparse_terminal(
                    &f,
                    PreparedAutomaticBatch {
                        targets: &y,
                        columns: k,
                        coefficients: &mut x,
                        reports: &mut reports,
                    },
                    options(transitions),
                    &policy,
                    PreparedHierarchyBudget::UNLIMITED,
                    layout,
                    &mut p,
                    &mut l,
                )
                .unwrap();
                assert_eq!(p.stage, PreparedAutomaticStage::Complete);
                assert_eq!(p.global_fallback_columns, 0);
                sparse_terminals = p.sparse_terminals;
                admitted = p.maximum_admitted_payload_bytes;
            });
            assert_eq!((s.count_current, s.bytes_current), (0, 0), "{s:?}");
            assert!(reports.iter().all(|r| r.is_some_and(|r| r.accepted)));
            assert!(
                base + s.bytes_max as usize <= admitted,
                "V={v} K={k}: {s:?}, base={base}, admitted={admitted}"
            );
            // Repeated calls at the same K are identical allocation records.
            if repeat == 0 {
                reference = Some(s);
            } else {
                assert_eq!(Some(s), reference, "V={v} K={k}: repeated call differs");
            }
        }
    }
    assert!(sparse_terminals >= 1, "the sparse terminal is engaged");
    println!(
        "SPARSE_AUTOMATIC V={v} E={e} transitions={transitions} layout={layout:?} peak={:?}",
        reference.unwrap()
    );
    Ok(())
}

fn main() -> Result {
    for n in [516u32, 1028] {
        let keys: Vec<_> = (0..n - 4)
            .flat_map(|i| (0..2).flat_map(move |j| (0..2).map(move |k| [i, j, k])))
            .collect();
        let t = PreparedThreeWayTopology::try_from_collapsed([(n - 4) as usize, 2, 2], &keys)?;
        run(&t, 0, PreparedAutomaticLayout::Scalar)?;
        run(&t, 0, PreparedAutomaticLayout::AllImage)?;
    }
    // Two large components: each sparse terminal is released before the next.
    let keys: Vec<_> = (0..2)
        .flat_map(|c| {
            (0..512).flat_map(move |i| {
                (0..2).flat_map(move |j| (0..2).map(move |k| [512 * c + i, 2 * c + j, 2 * c + k]))
            })
        })
        .collect();
    run(
        &PreparedThreeWayTopology::try_from_collapsed([1024, 4, 4], &keys)?,
        0,
        PreparedAutomaticLayout::Scalar,
    )?;
    println!("sparse-terminal automatic live-allocation peaks passed");
    Ok(())
}
