//! Full exact-range admission of fixed sparse terminal actions.
#![cfg(feature = "within-comparator")]

use multiway_incidence::{
    IncidenceKernelCertificate, IncidenceKernelLimits, IncidenceRangeProjector,
    PreparedThreeWayTopology,
};
use multiway_mg::{
    DenseRangeDecomposition, MultiwayError, Preconditioner, RangeProjectedAction,
    SpectralAnalysisOptions, ThreeWayProblem, WithinApproxCholOptions,
    WithinApproxCholPreconditioner, apply_range_projected,
};

const LIMITS: IncidenceKernelLimits = IncidenceKernelLimits {
    maximum_payload_bytes: 1 << 26,
};

struct Case {
    name: String,
    counts: [usize; 3],
    tuples: Vec<[u32; 3]>,
}

fn audit_cases() -> Vec<Case> {
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
    cases
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

fn random_cases(count: usize) -> Vec<Case> {
    let mut random = Lcg(0x2026_1020);
    (0..count)
        .map(|index| {
            let counts = [3 + random.next(8), 3 + random.next(8), 2 + random.next(6)];
            let maximum = counts.iter().copied().max().unwrap();
            let mut tuples: Vec<[u32; 3]> = (0..maximum)
                .map(|level| counts.map(|n| level.min(n - 1)))
                .collect();
            for _ in 0..random.next(2 * maximum) + 1 {
                tuples.push(counts.map(|n| random.next(n)));
            }
            Case {
                name: format!("random-{index}"),
                counts: counts.map(|n| n as usize),
                tuples,
            }
        })
        .collect()
}

fn canonical(tuples: &[[u32; 3]]) -> Vec<[u32; 3]> {
    let mut sorted = tuples.to_vec();
    sorted.sort_unstable();
    sorted.dedup();
    sorted
}

fn weights(kind: &str, count: usize) -> Vec<f64> {
    (0..count)
        .map(|i| match kind {
            "unit" => 1.0,
            "dyadic" => 2.0_f64.powi((i % 7) as i32 - 3),
            // Spans 2^-30..2^30, beyond the default dense rank tolerance's reach.
            _ => 2.0_f64.powi(((i * 7) % 61) as i32 - 30),
        })
        .collect()
}

/// Structural two-sided projection, the M6o audit's insufficient repair.
struct StructuralBoth<'a>(&'a ThreeWayProblem, &'a WithinApproxCholPreconditioner);
impl Preconditioner for StructuralBoth<'_> {
    fn dimension(&self) -> usize {
        self.0.dimension()
    }
    fn apply(&self, rhs: &[f64], out: &mut [f64]) -> Result<(), MultiwayError> {
        let mut projected = rhs.to_vec();
        self.0
            .components()
            .project_structural_range(&mut projected)?;
        self.1.apply(&projected, out)?;
        self.0.components().project_structural_range(out)?;
        Ok(())
    }
}

fn projector(owner: &PreparedThreeWayTopology) -> IncidenceRangeProjector<'_> {
    IncidenceKernelCertificate::try_certify(owner, LIMITS)
        .unwrap()
        .try_into_projector(owner, LIMITS, 1e-12)
        .unwrap()
}

#[test]
fn certified_full_range_projection_admits_every_audited_action() {
    let options = SpectralAnalysisOptions::default();
    let mut cases = audit_cases();
    cases.extend(random_cases(40));
    let mut extended_cases = 0;
    for case in &cases {
        let tuples = canonical(&case.tuples);
        let owner = PreparedThreeWayTopology::try_from_collapsed(case.counts, &tuples).unwrap();
        let projector = projector(&owner);
        let certificate = projector.certificate();
        if !projector.is_structural() {
            extended_cases += 1;
        }
        let unit = ThreeWayProblem::from_observations(
            case.counts,
            &tuples,
            &weights("unit", tuples.len()),
        )
        .unwrap();
        // The exact range of every weighting; unit weights resolve it numerically.
        let exact_range = DenseRangeDecomposition::from_problem(&unit, options).unwrap();
        assert_eq!(
            exact_range.nullity(),
            certificate.nullity(),
            "{}",
            case.name
        );
        for kind in ["unit", "dyadic", "extreme"] {
            let problem = ThreeWayProblem::from_observations(
                case.counts,
                &tuples,
                &weights(kind, tuples.len()),
            )
            .unwrap();
            let within = WithinApproxCholPreconditioner::build(
                problem.clone(),
                WithinApproxCholOptions::default(),
            )
            .unwrap();
            let action = RangeProjectedAction::new(&projector, within).unwrap();
            let report = exact_range.analyze(&action, options).unwrap();
            let label = format!("{} {kind}", case.name);
            assert!(report.numerically_symmetric(), "{label} {report:?}");
            assert!(report.range_leakage() <= 1e-12, "{label} {report:?}");
            assert!(report.preserves_range(), "{label}");
            assert_eq!(report.negative_preconditioner_directions(), 0, "{label}");
            if kind == "extreme" {
                // Energies inherit the 2^60 weight spread, beyond what a dense f64
                // eigensolver can resolve; only exclude negativity beyond roundoff.
                let floor = -64.0 * f64::EPSILON * report.maximum_preconditioner_energy();
                assert!(
                    report.minimum_preconditioner_energy() >= floor,
                    "{label} {report:?}"
                );
                continue;
            }
            assert!(
                report.minimum_preconditioner_energy() > 0.0,
                "{label} {report:?}"
            );
            assert!(report.positive_definite_on_range(), "{label} {report:?}");
            assert_eq!(report.near_zero_preconditioner_directions(), 0, "{label}");
            {
                // Moderate weights: the submitted Gramian's own numerical range agrees.
                let own = DenseRangeDecomposition::from_problem(&problem, options).unwrap();
                assert_eq!(own.nullity(), certificate.nullity(), "{label}");
                let own_report = own.analyze(&action, options).unwrap();
                assert!(own_report.preserves_range(), "{label} {own_report:?}");
                assert!(own_report.positive_definite_on_range(), "{label}");
            }
        }
    }
    assert!(extended_cases >= 10, "{extended_cases}");
}

#[test]
fn structural_projection_alone_still_leaks_on_additional_nullity() {
    let options = SpectralAnalysisOptions::default();
    let case = audit_cases().pop().unwrap();
    assert_eq!(case.name, "sparse-connected");
    let tuples = canonical(&case.tuples);
    let owner = PreparedThreeWayTopology::try_from_collapsed(case.counts, &tuples).unwrap();
    let projector = projector(&owner);
    assert_eq!(projector.certificate().additional_nullity(), 3);
    for (kind, recorded) in [
        ("unit", 0.252_388_956_280_031_3),
        ("dyadic", 0.176_209_718_819_095_72),
    ] {
        let problem =
            ThreeWayProblem::from_observations(case.counts, &tuples, &weights(kind, tuples.len()))
                .unwrap();
        let range = DenseRangeDecomposition::from_problem(&problem, options).unwrap();
        let within = WithinApproxCholPreconditioner::build(
            problem.clone(),
            WithinApproxCholOptions::default(),
        )
        .unwrap();
        let structural = range
            .analyze(&StructuralBoth(&problem, &within), options)
            .unwrap();
        // The preserved M6o negative: structural projection is not the range projector.
        assert!(!structural.preserves_range());
        assert!(
            (structural.range_leakage() - recorded).abs() <= 1e-6 * recorded,
            "{kind}"
        );
        let full = range
            .analyze(
                &RangeProjectedAction::new(&projector, within).unwrap(),
                options,
            )
            .unwrap();
        assert!(full.preserves_range() && full.numerically_symmetric());
        assert!(full.positive_definite_on_range());
    }
}

#[test]
fn workspace_application_validates_before_mutation_and_matches_wrapper() {
    let case = &audit_cases()[4];
    let tuples = canonical(&case.tuples);
    let owner = PreparedThreeWayTopology::try_from_collapsed(case.counts, &tuples).unwrap();
    let projector = projector(&owner);
    let problem =
        ThreeWayProblem::from_observations(case.counts, &tuples, &weights("dyadic", tuples.len()))
            .unwrap();
    let within =
        WithinApproxCholPreconditioner::build(problem.clone(), WithinApproxCholOptions::default())
            .unwrap();
    let n = problem.dimension();
    let rhs: Vec<f64> = (0..n).map(|i| ((i + 1) as f64 * 0.37).sin()).collect();
    for (scratch_len, out_len) in [(n - 1, n), (n, n + 1)] {
        let mut scratch = vec![7.0; scratch_len];
        let mut out = vec![9.0; out_len];
        assert!(matches!(
            apply_range_projected(&projector, &within, &rhs, &mut scratch, &mut out),
            Err(MultiwayError::DimensionMismatch { .. })
        ));
        assert!(scratch.iter().all(|&v| v == 7.0) && out.iter().all(|&v| v == 9.0));
    }
    let mut scratch = vec![0.0; n];
    let mut first = vec![0.0; n];
    apply_range_projected(&projector, &within, &rhs, &mut scratch, &mut first).unwrap();
    let mut repeated = vec![0.0; n];
    apply_range_projected(&projector, &within, &rhs, &mut scratch, &mut repeated).unwrap();
    assert_eq!(first, repeated);
    let action = RangeProjectedAction::new(&projector, within).unwrap();
    let mut wrapped = vec![0.0; n];
    action.apply(&rhs, &mut wrapped).unwrap();
    assert_eq!(first, wrapped);
    let other = ThreeWayProblem::from_observations(
        [5, 4, 4],
        &[[4, 3, 3], [0, 0, 0], [1, 1, 1], [2, 2, 2], [3, 3, 3]],
        &[1.0; 5],
    )
    .unwrap();
    let mismatched =
        WithinApproxCholPreconditioner::build(other, WithinApproxCholOptions::default()).unwrap();
    assert!(RangeProjectedAction::new(&projector, mismatched).is_err());
}
