//! Isolated single-thread proof that sparse-terminal admission bounds its heap.
//!
//! A counting allocator measures live bytes and their peak on this thread. Every
//! build must stay within its reported peak and retain exactly its reported
//! payload; a refused build must stay within its limit and release everything;
//! application and its workspace reuse allocate nothing.
#![cfg(feature = "sparse-terminal")]
use allocation_counter::{AllocationInfo, measure};
use multiway_incidence::{PreparedThreeWayTopology, ThreeWayWeightFrame, WeightFrameInput};
use multiway_mg::{
    MultiwayError, PreparedSparseTerminal, PreparedSparseTerminalOptions,
    SPARSE_TERMINAL_DESIGN_DESCRIPTOR_BOUND,
};
use std::{borrow::Cow, hint::black_box};

fn measured<R>(run: impl FnOnce() -> R) -> (R, AllocationInfo) {
    let mut result = None;
    let info = measure(|| result = Some(run()));
    (result.unwrap(), info)
}

/// Name, factor level counts and canonical tuples.
type Support = (&'static str, [usize; 3], Vec<[u32; 3]>);

fn supports() -> Vec<Support> {
    let mut nested = Vec::new();
    for a in 0..120_u32 {
        for r in 0..2 {
            nested.push([a, 2 * a + r, a % 40]);
        }
    }
    let mut state = 0x5354_u64;
    let mut next = |bound: u32| {
        state = state
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        ((state >> 33) % u64::from(bound)) as u32
    };
    let mut random: Vec<[u32; 3]> = (0..150).map(|i| [i, i % 90, i % 60]).collect();
    for _ in 0..900 {
        random.push([next(150), next(90), next(60)]);
    }
    let tensor: Vec<[u32; 3]> = (0..6)
        .flat_map(|a| (0..5).flat_map(move |b| (0..4).map(move |c| [a, b, c])))
        .collect();
    let mut out = vec![
        ("nested-400", [120, 240, 40], nested),
        ("random-300", [150, 90, 60], random),
        ("tensor-15", [6, 5, 4], tensor),
    ];
    for (_, _, tuples) in &mut out {
        tuples.sort_unstable();
        tuples.dedup();
    }
    out
}

fn options(schur: within::SchurMode) -> PreparedSparseTerminalOptions {
    PreparedSparseTerminalOptions {
        maximum_dimension: 4096,
        local_solver: within::LocalSolverConfig {
            schur,
            ..within::LocalSolverConfig::default()
        },
        projector_tolerance: 1e-12,
    }
}

fn design_descriptors_fit_their_fixed_bound() {
    for e in [1usize, 17, 4096] {
        let columns: [Vec<u32>; 3] =
            std::array::from_fn(|f| (0..e as u32).map(|i| i % (f as u32 + 2)).collect());
        let ((), info) = measured(|| {
            let borrowed: Vec<Cow<'_, [u32]>> = columns
                .iter()
                .map(|c| Cow::Borrowed(c.as_slice()))
                .collect();
            let frame = within::observation::ObservationFrame::new(borrowed, Vec::new()).unwrap();
            let design = within::Design::from_frame_unsorted(frame).unwrap();
            black_box(&design);
        });
        assert_eq!(info.bytes_current, 0);
        assert!(
            info.bytes_max as usize <= SPARSE_TERMINAL_DESIGN_DESCRIPTOR_BOUND,
            "E={e}: {info:?}"
        );
        println!("SPARSE_TERMINAL_DESIGN e={e} peak={}", info.bytes_max);
    }
}

fn main() {
    design_descriptors_fit_their_fixed_bound();
    let mut builds = 0;
    let mut slack = 0usize;
    for (name, counts, tuples) in supports() {
        let topology = PreparedThreeWayTopology::try_from_collapsed(counts, &tuples).unwrap();
        let weights: Vec<f64> = (0..tuples.len())
            .map(|i| 2.0_f64.powi((i % 7) as i32 - 3))
            .collect();
        let frame =
            ThreeWayWeightFrame::try_new(&topology, WeightFrameInput::Tuples(&weights)).unwrap();
        for schur in [within::SchurMode::default(), within::SchurMode::Exact] {
            let options = options(schur.clone());
            let label = format!("{name} {schur:?}");
            let (built, info) =
                measured(|| PreparedSparseTerminal::try_new(&frame, &options, usize::MAX));
            let terminal = built.unwrap();
            let report = terminal.report();
            assert!(
                info.bytes_max as usize <= report.peak_payload_bytes,
                "{label}: allocator peak {} exceeds ledger peak {}",
                info.bytes_max,
                report.peak_payload_bytes
            );
            assert_eq!(
                info.bytes_current, report.retained_payload_bytes as i64,
                "{label}: retained payload"
            );
            slack = slack.max(report.peak_payload_bytes - info.bytes_max as usize);
            let build_peak = info.bytes_max;

            let (workspace, info) = measured(|| terminal.application_workspace().unwrap());
            let mut workspace = workspace;
            assert_eq!(
                info.bytes_current as usize,
                terminal.workspace_required_bytes().unwrap(),
                "{label}: workspace"
            );
            assert_eq!(
                workspace.retained_payload_bytes().unwrap(),
                terminal.workspace_required_bytes().unwrap()
            );
            let n = terminal.dimension();
            let rhs: Vec<f64> = (0..n).map(|i| ((i + 1) as f64 * 0.31).sin()).collect();
            let mut out = vec![0.0; n];
            for _ in 0..3 {
                let (result, info) =
                    measured(|| terminal.apply_with_workspace(&rhs, &mut out, &mut workspace));
                result.unwrap();
                assert_eq!(info.count_total, 0, "{label}: application allocated");
            }
            let (_, info) = measured(|| drop(workspace));
            assert_eq!(
                info.bytes_current,
                -(terminal.workspace_required_bytes().unwrap() as i64)
            );
            let (_, info) = measured(|| drop(terminal));
            assert_eq!(
                info.bytes_current,
                -(report.retained_payload_bytes as i64),
                "{label}: drop releases the payload"
            );

            for limit in [report.peak_payload_bytes - 1, report.peak_payload_bytes / 3] {
                let (refused, info) =
                    measured(|| PreparedSparseTerminal::try_new(&frame, &options, limit));
                match refused {
                    Err(MultiwayError::WithinSetupBudgetExceeded { .. })
                    | Err(MultiwayError::Incidence(_)) => {}
                    other => panic!("{label} at {limit}: {other:?}"),
                }
                assert!(
                    info.bytes_max as usize <= limit,
                    "{label}: refused build peaked at {} > {limit}",
                    info.bytes_max
                );
                assert_eq!(info.bytes_current, 0, "{label}: refused build leaked");
            }
            println!(
                "SPARSE_TERMINAL case={label} peak={} allocator_peak={build_peak} retained={}",
                report.peak_payload_bytes, report.retained_payload_bytes
            );
            builds += 1;
        }
    }
    assert_eq!(builds, 6);
    println!("SPARSE_TERMINAL_SUMMARY builds={builds} maximum_conservative_slack_bytes={slack}");
}
