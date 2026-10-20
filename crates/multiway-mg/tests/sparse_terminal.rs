//! Fixed sparse terminals: full exact range, symmetry, positivity and admission.
#![cfg(feature = "sparse-terminal")]

use std::sync::Mutex;

use multiway_incidence::{
    FactorAggregation, HierarchyWeightFrames, PreparedHierarchyTopology, PreparedThreeWayTopology,
    ThreeWayWeightFrame, WeightFrameInput,
};
use multiway_mg::{
    DenseRangeDecomposition, MultiwayError, PreparedMapHierarchy, PreparedSparseTerminal,
    PreparedSparseTerminalOptions, SparseTerminalWorkspace, SpectralAnalysisOptions,
    ThreeWayProblem,
};

struct Case {
    name: String,
    counts: [usize; 3],
    tuples: Vec<[u32; 3]>,
}

struct Lcg(u64);
impl Lcg {
    fn next(&mut self, bound: u32) -> u32 {
        self.0 = self
            .0
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        ((self.0 >> 33) % u64::from(bound)) as u32
    }
}

fn canonical(tuples: &[[u32; 3]]) -> Vec<[u32; 3]> {
    let mut sorted = tuples.to_vec();
    sorted.sort_unstable();
    sorted.dedup();
    sorted
}

/// The M6o/M6p audit supports, random supports, and two larger coarse-sized ones.
fn cases() -> Vec<Case> {
    let n = 4_u32;
    let mut cases = Vec::new();
    for name in ["tensor", "latin", "nested", "two-components"] {
        let mut tuples = Vec::new();
        for a in 0..n {
            for b in 0..n {
                match name {
                    "tensor" => (0..n).for_each(|c| tuples.push([a, b, c])),
                    "latin" => tuples.push([a, b, (a + b) % n]),
                    "nested" => tuples.push([a, b, a]),
                    _ => {
                        if a / 2 == b / 2 {
                            tuples.push([a, b, a]);
                        }
                    }
                }
            }
        }
        cases.push(Case {
            name: name.to_owned(),
            counts: [4; 3],
            tuples,
        });
    }
    cases.push(Case {
        name: "sparse-connected".to_owned(),
        counts: [4; 3],
        tuples: vec![
            [0, 0, 0],
            [1, 0, 0],
            [1, 1, 1],
            [2, 1, 1],
            [2, 2, 2],
            [3, 2, 3],
            [3, 3, 3],
        ],
    });
    let mut random = Lcg(0x5350_4152);
    for index in 0..30 {
        let counts = [3 + random.next(8), 3 + random.next(8), 2 + random.next(6)];
        let maximum = counts.iter().copied().max().unwrap();
        let mut tuples: Vec<[u32; 3]> = (0..maximum)
            .map(|level| counts.map(|n| level.min(n - 1)))
            .collect();
        for _ in 0..random.next(2 * maximum) + 1 {
            tuples.push(counts.map(|n| random.next(n)));
        }
        cases.push(Case {
            name: format!("random-{index}"),
            counts: counts.map(|n| n as usize),
            tuples,
        });
    }
    // Above the 256-coordinate dense cap: a nested support (additional nullity)
    // and a random sparse one.
    let mut nested = Vec::new();
    for a in 0..120_u32 {
        for r in 0..2 {
            nested.push([a, 2 * a + r, a % 40]);
        }
    }
    cases.push(Case {
        name: "nested-400".to_owned(),
        counts: [120, 240, 40],
        tuples: nested,
    });
    let counts = [150_u32, 90, 60];
    let mut tuples: Vec<[u32; 3]> = (0..150).map(|i| [i, i % 90, i % 60]).collect();
    for _ in 0..900 {
        tuples.push(counts.map(|n| random.next(n)));
    }
    cases.push(Case {
        name: "random-300".to_owned(),
        counts: [150, 90, 60],
        tuples,
    });
    cases
}

fn weights(kind: &str, count: usize) -> Vec<f64> {
    (0..count)
        .map(|i| match kind {
            "unit" => 1.0,
            "dyadic" => 2.0_f64.powi((i % 7) as i32 - 3),
            _ => 2.0_f64.powi(((i * 7) % 61) as i32 - 30),
        })
        .collect()
}

fn options() -> PreparedSparseTerminalOptions {
    PreparedSparseTerminalOptions {
        maximum_dimension: 4096,
        local_solver: within::LocalSolverConfig::default(),
        projector_tolerance: 1e-12,
    }
}

/// The terminal as an ordinary `&self` action for the dense spectral oracle.
struct Action<'a> {
    terminal: &'a PreparedSparseTerminal<'a>,
    workspace: Mutex<SparseTerminalWorkspace<'a>>,
}
impl multiway_mg::Preconditioner for Action<'_> {
    fn dimension(&self) -> usize {
        self.terminal.dimension()
    }
    fn apply(&self, rhs: &[f64], out: &mut [f64]) -> Result<(), MultiwayError> {
        self.terminal
            .apply_with_workspace(rhs, out, &mut self.workspace.lock().unwrap())
    }
}

#[test]
fn sparse_terminals_preserve_the_exact_range_symmetrically_and_positively() {
    let analysis = SpectralAnalysisOptions {
        maximum_dimension: 1024,
        ..SpectralAnalysisOptions::default()
    };
    let mut extended = 0;
    let mut large = 0;
    for case in cases() {
        let tuples = canonical(&case.tuples);
        let topology = PreparedThreeWayTopology::try_from_collapsed(case.counts, &tuples).unwrap();
        let unit = ThreeWayProblem::from_observations(
            case.counts,
            &tuples,
            &weights("unit", tuples.len()),
        )
        .unwrap();
        let exact_range = DenseRangeDecomposition::from_problem(&unit, analysis).unwrap();
        for kind in ["unit", "dyadic", "extreme"] {
            let label = format!("{} {kind}", case.name);
            let w = weights(kind, tuples.len());
            let frame =
                ThreeWayWeightFrame::try_new(&topology, WeightFrameInput::Tuples(&w)).unwrap();
            let terminal = PreparedSparseTerminal::try_new(&frame, &options(), 1 << 30).unwrap();
            let report = terminal.report();
            assert_eq!(report.nullity, exact_range.nullity(), "{label}");
            assert_eq!(terminal.rank() + report.nullity, report.dimension);
            assert!(report.peak_payload_bytes >= report.retained_payload_bytes);
            if kind == "unit" {
                extended += usize::from(report.additional_nullity > 0);
                large += usize::from(report.dimension > 256);
            }
            let action = Action {
                terminal: &terminal,
                workspace: Mutex::new(terminal.application_workspace().unwrap()),
            };
            let result = exact_range.analyze(&action, analysis).unwrap();
            assert!(result.numerically_symmetric(), "{label} {result:?}");
            assert!(result.range_leakage() <= 1e-12, "{label} {result:?}");
            assert!(result.preserves_range(), "{label}");
            assert_eq!(result.negative_preconditioner_directions(), 0, "{label}");
            if kind == "extreme" {
                // The 2^60 weight spread is beyond a dense f64 eigensolver's
                // resolution; exclude only negativity beyond roundoff.
                let floor = -64.0 * f64::EPSILON * result.maximum_preconditioner_energy();
                assert!(
                    result.minimum_preconditioner_energy() >= floor,
                    "{label} {result:?}"
                );
            } else {
                assert!(result.positive_definite_on_range(), "{label} {result:?}");
            }
        }
    }
    assert!(extended >= 8, "{extended} supports with additional nullity");
    assert_eq!(large, 2, "both supports above the dense cap");
}

#[test]
fn a_depth_zero_hierarchy_applies_exactly_its_sparse_terminal() {
    let case = cases()
        .into_iter()
        .find(|c| c.name == "random-300")
        .unwrap();
    let tuples = canonical(&case.tuples);
    let topology = PreparedThreeWayTopology::try_from_collapsed(case.counts, &tuples).unwrap();
    let w = weights("dyadic", tuples.len());
    let fine = ThreeWayWeightFrame::try_new(&topology, WeightFrameInput::Tuples(&w)).unwrap();
    let structure = PreparedHierarchyTopology::try_new(&topology, Vec::new()).unwrap();
    let frames = HierarchyWeightFrames::try_new(&structure, &fine).unwrap();
    let last = frames.frame(0).unwrap();
    let terminal = PreparedSparseTerminal::try_new(last, &options(), 1 << 30).unwrap();
    let n = terminal.dimension();
    let rhs: Vec<f64> = (0..n).map(|i| ((i + 3) as f64 * 0.29).sin()).collect();
    let mut expected = vec![0.0; n];
    terminal
        .apply_with_workspace(
            &rhs,
            &mut expected,
            &mut terminal.application_workspace().unwrap(),
        )
        .unwrap();
    // The cycle's terminal level also applies the structural projection; the
    // exact range is inside the structural range, so this changes only roundoff.
    let hierarchy =
        PreparedMapHierarchy::try_new_with_sparse_terminal(&frames, None, terminal).unwrap();
    assert!(hierarchy.sparse_terminal().is_some());
    let mut workspace = hierarchy.application_workspace().unwrap();
    let mut actual = vec![0.0; n];
    hierarchy
        .apply_with_workspace(&rhs, &mut actual, &mut workspace)
        .unwrap();
    let scale = expected.iter().fold(1.0_f64, |m, v| m.max(v.abs()));
    for (a, b) in actual.iter().zip(&expected) {
        assert!((a - b).abs() <= 1e-13 * scale);
    }
    let mut again = vec![0.0; n];
    hierarchy
        .apply_with_workspace(&rhs, &mut again, &mut workspace)
        .unwrap();
    assert_eq!(actual, again, "repeated application is deterministic");
}

#[test]
fn a_one_transition_cycle_with_a_sparse_terminal_is_symmetric() {
    let case = cases()
        .into_iter()
        .find(|c| c.name == "random-300")
        .unwrap();
    let tuples = canonical(&case.tuples);
    let topology = PreparedThreeWayTopology::try_from_collapsed(case.counts, &tuples).unwrap();
    let w = weights("unit", tuples.len());
    let fine = ThreeWayWeightFrame::try_new(&topology, WeightFrameInput::Tuples(&w)).unwrap();
    let structure = PreparedHierarchyTopology::try_new(
        &topology,
        vec![FactorAggregation::consecutive_halving(case.counts).unwrap()],
    )
    .unwrap();
    let frames = HierarchyWeightFrames::try_new(&structure, &fine).unwrap();
    let terminal =
        PreparedSparseTerminal::try_new(frames.frame(1).unwrap(), &options(), 1 << 30).unwrap();
    let hierarchy =
        PreparedMapHierarchy::try_new_with_sparse_terminal(&frames, None, terminal).unwrap();
    assert_eq!(hierarchy.depth(), 1);
    let mut workspace = hierarchy.application_workspace().unwrap();
    let n = hierarchy.dimension();
    let mut x: Vec<f64> = (0..n).map(|i| ((i + 1) as f64 * 0.41).sin()).collect();
    let mut y: Vec<f64> = (0..n).map(|i| ((i + 2) as f64 * 0.73).cos()).collect();
    let mut projection = topology.try_projection_workspace().unwrap();
    topology
        .project_structural_range_with_workspace(&mut x, &mut projection)
        .unwrap();
    topology
        .project_structural_range_with_workspace(&mut y, &mut projection)
        .unwrap();
    let (mut bx, mut by) = (vec![0.0; n], vec![0.0; n]);
    hierarchy
        .apply_with_workspace(&x, &mut bx, &mut workspace)
        .unwrap();
    hierarchy
        .apply_with_workspace(&y, &mut by, &mut workspace)
        .unwrap();
    let dot = |a: &[f64], b: &[f64]| a.iter().zip(b).map(|(u, v)| u * v).sum::<f64>();
    let (xby, bxy) = (dot(&x, &by), dot(&bx, &y));
    assert!(
        (xby - bxy).abs() <= 1e-12 * xby.abs().max(bxy.abs()),
        "{xby} {bxy}"
    );
    assert!(dot(&x, &bx) > 0.0);
}

#[test]
fn limits_are_inclusive_and_inputs_are_validated() {
    let case = cases()
        .into_iter()
        .find(|c| c.name == "nested-400")
        .unwrap();
    let tuples = canonical(&case.tuples);
    let topology = PreparedThreeWayTopology::try_from_collapsed(case.counts, &tuples).unwrap();
    let w = weights("dyadic", tuples.len());
    let frame = ThreeWayWeightFrame::try_new(&topology, WeightFrameInput::Tuples(&w)).unwrap();
    let report = PreparedSparseTerminal::try_new(&frame, &options(), usize::MAX)
        .unwrap()
        .report();
    let at_peak =
        PreparedSparseTerminal::try_new(&frame, &options(), report.peak_payload_bytes).unwrap();
    assert_eq!(at_peak.report(), report, "accounting is deterministic");
    for limit in [
        report.peak_payload_bytes - 1,
        report.retained_payload_bytes / 2,
        0,
    ] {
        match PreparedSparseTerminal::try_new(&frame, &options(), limit) {
            Err(MultiwayError::WithinSetupBudgetExceeded {
                required, limit: l, ..
            }) => {
                assert_eq!(l, limit);
                assert!(required > limit, "{required} {limit}");
            }
            Err(MultiwayError::Incidence(error)) => {
                assert!(error.to_string().contains("payload"), "{error}");
            }
            other => panic!("limit {limit}: {other:?}"),
        }
    }
    let capped = PreparedSparseTerminalOptions {
        maximum_dimension: report.dimension - 1,
        ..options()
    };
    assert!(matches!(
        PreparedSparseTerminal::try_new(&frame, &capped, usize::MAX),
        Err(MultiwayError::InvalidOption { .. })
    ));
    for tolerance in [0.0, f64::NAN, -1.0] {
        let bad = PreparedSparseTerminalOptions {
            projector_tolerance: tolerance,
            ..options()
        };
        assert!(PreparedSparseTerminal::try_new(&frame, &bad, usize::MAX).is_err());
    }
    let other = ThreeWayWeightFrame::try_new(&topology, WeightFrameInput::UnitTuples).unwrap();
    assert!(at_peak.validate_for(&other).is_err());
    assert!(at_peak.validate_for(&frame).is_ok());
    let n = at_peak.dimension();
    let mut workspace = at_peak.application_workspace().unwrap();
    let mut out = vec![5.0; n + 1];
    assert!(matches!(
        at_peak.apply_with_workspace(&vec![1.0; n], &mut out, &mut workspace),
        Err(MultiwayError::DimensionMismatch { .. })
    ));
    assert!(
        out.iter().all(|&v| v == 5.0),
        "static errors publish nothing"
    );
}
