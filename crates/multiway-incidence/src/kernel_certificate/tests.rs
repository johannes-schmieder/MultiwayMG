//! Exact kernel certification against an independent fraction-free rank.
use super::*;

const UNLIMITED: IncidenceKernelLimits = IncidenceKernelLimits {
    maximum_payload_bytes: usize::MAX,
};

fn prepare(counts: [usize; 3], tuples: &[[u32; 3]]) -> PreparedThreeWayTopology {
    let mut sorted = tuples.to_vec();
    sorted.sort_unstable();
    sorted.dedup();
    PreparedThreeWayTopology::try_from_collapsed(counts, &sorted).unwrap()
}

/// Independent rational rank of a dense integer matrix by Bareiss elimination.
fn bareiss_rank(mut rows: Vec<Vec<i128>>) -> usize {
    let columns = rows.first().map_or(0, Vec::len);
    let mut rank = 0;
    let mut previous = 1_i128;
    for column in 0..columns {
        let Some(pivot) = (rank..rows.len()).find(|&row| rows[row][column] != 0) else {
            continue;
        };
        rows.swap(rank, pivot);
        for row in rank + 1..rows.len() {
            for other in column + 1..columns {
                let value =
                    rows[rank][column] * rows[row][other] - rows[row][column] * rows[rank][other];
                assert_eq!(value % previous, 0, "Bareiss division is exact");
                rows[row][other] = value / previous;
            }
            rows[row][column] = 0;
        }
        previous = rows[rank][column];
        rank += 1;
    }
    rank
}

fn component_rank_reference(owner: &PreparedThreeWayTopology, component: usize) -> usize {
    let topology = owner.topology();
    let labels = owner.component_labels();
    let coordinates: Vec<usize> = (0..labels.len())
        .filter(|&v| labels[v] == component)
        .collect();
    let rows = topology
        .tuples()
        .iter()
        .filter(|tuple| labels[topology.global_index(0, tuple[0])] == component)
        .map(|tuple| {
            let mut row = vec![0_i128; coordinates.len()];
            for factor in 0..3 {
                let global = topology.global_index(factor, tuple[factor]);
                row[coordinates.binary_search(&global).unwrap()] = 1;
            }
            row
        })
        .collect();
    bareiss_rank(rows)
}

fn assert_matches_reference(owner: &PreparedThreeWayTopology) -> IncidenceKernelCertificate<'_> {
    let certificate = IncidenceKernelCertificate::try_certify(owner, UNLIMITED).unwrap();
    let topology = owner.topology();
    let labels = owner.component_labels();
    assert_eq!(certificate.dimension(), topology.total_levels());
    assert_eq!(
        certificate.component_count(),
        owner.component_factor_sizes().len()
    );
    for component in 0..certificate.component_count() {
        let width = certificate.component_coordinates(component).len();
        let rank = component_rank_reference(owner, component);
        assert_eq!(certificate.component_rank(component), rank);
        assert_eq!(certificate.component_nullity(component), width - rank);
        match certificate.component_basis(component) {
            None => assert_eq!(width - rank, 2),
            Some(basis) => {
                assert!(width - rank > 2);
                assert_eq!(basis.nullity(), width - rank);
                for vector in 0..basis.nullity() {
                    let values = basis.vector(vector);
                    for tuple in topology.tuples() {
                        let row = [0, 1, 2].map(|f| topology.global_index(f, tuple[f]));
                        if labels[row[0]] != component {
                            continue;
                        }
                        let sum: i64 = row
                            .iter()
                            .map(|c| values[basis.coordinates().binary_search(c).unwrap()])
                            .sum();
                        assert_eq!(sum, 0);
                    }
                }
                let mut matrix: Vec<Vec<i128>> = (0..basis.nullity())
                    .map(|v| basis.vector(v).iter().map(|&x| i128::from(x)).collect())
                    .collect();
                assert_eq!(bareiss_rank(core::mem::take(&mut matrix)), basis.nullity());
            }
        }
    }
    assert_eq!(
        certificate.additional_nullity(),
        certificate.nullity() - 2 * certificate.component_count()
    );
    certificate
}

fn audit_case(name: &str) -> ([usize; 3], Vec<[u32; 3]>) {
    let n = 4_u32;
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
    if name == "sparse-connected" {
        tuples = vec![
            [0, 0, 0],
            [1, 0, 0],
            [1, 1, 1],
            [2, 1, 1],
            [2, 2, 2],
            [3, 2, 3],
            [3, 3, 3],
        ];
    }
    ([4; 3], tuples)
}

#[test]
fn range_audit_cases_are_classified_exactly() {
    for (name, rank, nullity) in [
        ("tensor", 10, 2),
        ("latin", 10, 2),
        ("nested", 7, 5),
        ("two-components", 6, 6),
        ("sparse-connected", 7, 5),
    ] {
        let (counts, tuples) = audit_case(name);
        let owner = prepare(counts, &tuples);
        let certificate = assert_matches_reference(&owner);
        assert_eq!(
            (certificate.rank(), certificate.nullity()),
            (rank, nullity),
            "{name}"
        );
    }
    let (counts, tuples) = audit_case("sparse-connected");
    let owner = prepare(counts, &tuples);
    let certificate = IncidenceKernelCertificate::try_certify(&owner, UNLIMITED).unwrap();
    assert_eq!(certificate.component_kind(0), ComponentKernelKind::Extended);
    assert_eq!(certificate.additional_nullity(), 3);
}

#[test]
fn additional_nullity_with_more_tuples_than_coordinates_is_found() {
    // Two disjoint 2x2 Latin blocks share every factor-2 level: E = 8 > V - 2 = 8 - 2,
    // but the blocks' factor-0/1 shifts differ independently.
    let tuples = [
        [0, 0, 0],
        [0, 1, 1],
        [1, 0, 1],
        [1, 1, 0],
        [2, 2, 0],
        [2, 3, 1],
        [3, 2, 1],
        [3, 3, 0],
    ];
    let owner = prepare([4, 4, 2], &tuples);
    let certificate = assert_matches_reference(&owner);
    assert_eq!(certificate.component_count(), 1);
    assert!(tuples.len() >= certificate.dimension() - 2);
    assert!(certificate.additional_nullity() > 0);
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

fn random_case(random: &mut Lcg) -> ([usize; 3], Vec<[u32; 3]>) {
    let counts = [2 + random.next(6), 2 + random.next(6), 1 + random.next(6)];
    let mut tuples = Vec::new();
    let maximum = counts.iter().copied().max().unwrap();
    // Cover every level, then add a random number of extra tuples.
    for level in 0..maximum {
        tuples.push(counts.map(|n| level.min(n - 1)));
    }
    for _ in 0..random.next(3 * maximum) {
        tuples.push(counts.map(|n| random.next(n)));
    }
    (counts.map(|n| n as usize), tuples)
}

#[test]
fn random_supports_match_independent_rational_rank() {
    let mut random = Lcg(0x5eed_cafe);
    let mut extended = 0;
    let mut structural = 0;
    for _ in 0..400 {
        let (counts, tuples) = random_case(&mut random);
        let owner = prepare(counts, &tuples);
        let certificate = assert_matches_reference(&owner);
        for component in 0..certificate.component_count() {
            match certificate.component_kind(component) {
                ComponentKernelKind::Extended => extended += 1,
                ComponentKernelKind::Structural => structural += 1,
            }
        }
        let report = certificate.report();
        assert!(report.peak_requested_payload_bytes > 0);
        assert!(certificate.retained_payload_bytes().unwrap() > 0);
    }
    assert!(extended > 50 && structural > 50, "{extended} {structural}");
}

#[test]
fn certification_is_deterministic() {
    let mut random = Lcg(17);
    for _ in 0..50 {
        let (counts, tuples) = random_case(&mut random);
        let owner = prepare(counts, &tuples);
        let first = IncidenceKernelCertificate::try_certify(&owner, UNLIMITED).unwrap();
        let second = IncidenceKernelCertificate::try_certify(&owner, UNLIMITED).unwrap();
        assert_eq!(first.basis, second.basis);
        assert_eq!(first.summaries, second.summaries);
        assert_eq!(first.report(), second.report());
    }
}

#[test]
fn budget_boundary_is_exact_and_rejection_is_typed() {
    let mut random = Lcg(99);
    for _ in 0..40 {
        let (counts, tuples) = random_case(&mut random);
        let owner = prepare(counts, &tuples);
        let peak = IncidenceKernelCertificate::try_certify(&owner, UNLIMITED)
            .unwrap()
            .report()
            .peak_requested_payload_bytes;
        let at = IncidenceKernelLimits {
            maximum_payload_bytes: peak,
        };
        assert_eq!(
            IncidenceKernelCertificate::try_certify(&owner, at)
                .unwrap()
                .report()
                .peak_requested_payload_bytes,
            peak
        );
        let under = IncidenceKernelLimits {
            maximum_payload_bytes: peak - 1,
        };
        assert!(matches!(
            IncidenceKernelCertificate::try_certify(&owner, under),
            Err(IncidenceError::KernelBudgetExceeded { budget, .. }) if budget == peak - 1
        ));
    }
}

#[test]
fn every_reservation_failure_and_unwind_is_clean() {
    let (counts, tuples) = audit_case("sparse-connected");
    let owner = prepare(counts, &tuples);
    let mut total = 0;
    IncidenceKernelCertificate::certify_with(&owner, UNLIMITED, &mut |_| {
        total += 1;
        Ok(())
    })
    .unwrap();
    assert!(total >= 10);
    for unwind in [false, true] {
        for fail_at in 0..total {
            let mut reached = 0;
            let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                IncidenceKernelCertificate::certify_with(&owner, UNLIMITED, &mut |context| {
                    let index = reached;
                    reached += 1;
                    if index == fail_at {
                        assert!(!unwind, "injected kernel reservation unwind");
                        return Err(IncidenceError::KernelAllocation { context });
                    }
                    Ok(())
                })
            }));
            assert_eq!(reached, fail_at + 1);
            if unwind {
                assert!(outcome.is_err());
            } else {
                assert!(matches!(
                    outcome.unwrap(),
                    Err(IncidenceError::KernelAllocation { .. })
                ));
            }
        }
    }
    let fresh = IncidenceKernelCertificate::try_certify(&owner, UNLIMITED).unwrap();
    assert_eq!(fresh.nullity(), 5);
}

fn dense_projector(projector: &IncidenceRangeProjector<'_>) -> Vec<Vec<f64>> {
    let n = projector.dimension();
    (0..n)
        .map(|column| {
            let mut values = vec![0.0; n];
            values[column] = 1.0;
            projector.project(&mut values).unwrap();
            values
        })
        .collect()
}

#[test]
fn projector_is_the_orthogonal_range_projector() {
    let mut random = Lcg(4242);
    let mut cases: Vec<_> = [
        "tensor",
        "latin",
        "nested",
        "two-components",
        "sparse-connected",
    ]
    .iter()
    .map(|name| audit_case(name))
    .collect();
    cases.extend((0..120).map(|_| random_case(&mut random)));
    for (counts, tuples) in cases {
        let owner = prepare(counts, &tuples);
        let certificate = IncidenceKernelCertificate::try_certify(&owner, UNLIMITED).unwrap();
        let nullity = certificate.nullity();
        let projector = certificate
            .try_into_projector(&owner, UNLIMITED, 1e-12)
            .unwrap();
        let report = projector.report();
        assert!(report.maximum_orthogonality_defect <= 1e-13, "{report:?}");
        assert!(report.maximum_incidence_residual <= 1e-13, "{report:?}");
        let matrix = dense_projector(&projector);
        let n = matrix.len();
        let mut trace = 0.0;
        for i in 0..n {
            trace += matrix[i][i];
            for j in 0..n {
                assert!((matrix[i][j] - matrix[j][i]).abs() <= 1e-14);
            }
        }
        assert!((trace - (n - nullity) as f64).abs() <= 1e-12);
        // Every row image B^T e_t lies in the range and is left unchanged.
        let topology = owner.topology();
        for tuple in topology.tuples() {
            let mut values = vec![0.0; n];
            for f in 0..3 {
                values[topology.global_index(f, tuple[f])] = 1.0;
            }
            let original = values.clone();
            projector.project(&mut values).unwrap();
            for (left, right) in values.iter().zip(&original) {
                assert!((left - right).abs() <= 1e-14);
            }
        }
        // Idempotence on an arbitrary vector.
        let mut values: Vec<f64> = (0..n).map(|i| ((i * 37 % 11) as f64) - 5.0).collect();
        projector.project(&mut values).unwrap();
        let once = values.clone();
        projector.project(&mut values).unwrap();
        for (left, right) in values.iter().zip(&once) {
            assert!((left - right).abs() <= 1e-13);
        }
        assert!(matches!(
            projector.project(&mut vec![0.0; n + 1]),
            Err(IncidenceError::DimensionMismatch { .. })
        ));
        assert!(projector.retained_payload_bytes().unwrap() > 0);
    }
}

#[test]
fn projector_rejects_other_owners_and_invalid_tolerance() {
    let (counts, tuples) = audit_case("latin");
    let owner = prepare(counts, &tuples);
    let other = prepare(counts, &tuples);
    let certificate = IncidenceKernelCertificate::try_certify(&owner, UNLIMITED).unwrap();
    assert!(matches!(
        certificate.try_into_projector(&other, UNLIMITED, 1e-12),
        Err(IncidenceError::KernelBindingMismatch)
    ));
    for tolerance in [0.0, -1.0, f64::NAN, f64::INFINITY] {
        let certificate = IncidenceKernelCertificate::try_certify(&owner, UNLIMITED).unwrap();
        assert!(matches!(
            certificate.try_into_projector(&owner, UNLIMITED, tolerance),
            Err(IncidenceError::KernelProjectorDefect { .. })
        ));
    }
    let certificate = IncidenceKernelCertificate::try_certify(&owner, UNLIMITED).unwrap();
    let retained = certificate.retained_payload_bytes().unwrap();
    assert!(matches!(
        certificate.try_into_projector(
            &owner,
            IncidenceKernelLimits {
                maximum_payload_bytes: retained
            },
            1e-12
        ),
        Err(IncidenceError::KernelBudgetExceeded { .. })
    ));
}
