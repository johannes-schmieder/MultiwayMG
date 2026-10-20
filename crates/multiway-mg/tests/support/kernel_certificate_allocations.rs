//! Exact kernel certificate payload and allocation-free range projection.
use super::{GLOBAL, Result, no_events};
use multiway_incidence::{
    IncidenceError, IncidenceKernelCertificate, IncidenceKernelLimits, PreparedThreeWayTopology,
};
use multiway_mg::{MultiwayError, Preconditioner, apply_range_projected};
use std::hint::black_box;

/// Allocation-free inner control so only wrapper allocations can appear.
struct Doubling(usize);
impl Preconditioner for Doubling {
    fn dimension(&self) -> usize {
        self.0
    }
    fn apply(&self, rhs: &[f64], out: &mut [f64]) -> std::result::Result<(), MultiwayError> {
        for (out, &value) in out.iter_mut().zip(rhs) {
            *out = 2.0 * value;
        }
        Ok(())
    }
}

fn net(stats: stats_alloc::Stats) -> isize {
    stats.bytes_allocated as isize + stats.bytes_reallocated - stats.bytes_deallocated as isize
}

pub(super) fn run() -> Result<()> {
    let structural: Vec<[u32; 3]> = (0..4)
        .flat_map(|a| (0..4).map(move |b| [a, b, (a + b) % 4]))
        .collect();
    let extended = vec![
        [0, 0, 0],
        [1, 0, 0],
        [1, 1, 1],
        [2, 1, 1],
        [2, 2, 2],
        [3, 2, 3],
        [3, 3, 3],
    ];
    let unlimited = IncidenceKernelLimits {
        maximum_payload_bytes: usize::MAX,
    };
    for (name, tuples, nullity) in [("latin", structural, 2), ("sparse", extended, 5)] {
        let owner = PreparedThreeWayTopology::try_from_collapsed([4; 3], &tuples)?;
        let peak = IncidenceKernelCertificate::try_certify(&owner, unlimited)?
            .report()
            .peak_requested_payload_bytes;
        let before = GLOBAL.stats();
        let rejected = IncidenceKernelCertificate::try_certify(
            black_box(&owner),
            IncidenceKernelLimits {
                maximum_payload_bytes: peak - 1,
            },
        );
        let failed = GLOBAL.stats() - before;
        assert!(matches!(
            rejected,
            Err(IncidenceError::KernelBudgetExceeded { .. })
        ));
        assert_eq!(net(failed), 0, "{name} rejected certification leaked");
        assert!(failed.bytes_allocated <= peak);
        let before = GLOBAL.stats();
        let certificate = black_box(IncidenceKernelCertificate::try_certify(
            &owner,
            IncidenceKernelLimits {
                maximum_payload_bytes: peak,
            },
        )?);
        let setup = GLOBAL.stats() - before;
        assert_eq!(certificate.nullity(), nullity);
        assert_eq!(net(setup) as usize, certificate.retained_payload_bytes()?);
        let before = GLOBAL.stats();
        let projector = black_box(certificate.try_into_projector(&owner, unlimited, 1e-12)?);
        let built = GLOBAL.stats() - before;
        let added = projector.retained_payload_bytes()?
            - projector.certificate().retained_payload_bytes()?;
        assert_eq!(net(built) as usize, added, "{name} projector payload");
        let n = projector.dimension();
        let rhs: Vec<f64> = (0..n).map(|i| ((i as f64 + 0.5) * 0.41).cos()).collect();
        let mut values = rhs.clone();
        let mut scratch = vec![0.0; n];
        let mut out = vec![0.0; n];
        let inner = Doubling(n);
        no_events(super::measure(|| {
            projector.project(black_box(&mut values))?;
            for _ in 0..64 {
                projector.project(black_box(&mut values))?;
                apply_range_projected(
                    &projector,
                    &inner,
                    black_box(&rhs),
                    &mut scratch,
                    black_box(&mut out),
                )?;
            }
            Ok(())
        })?);
        println!(
            "kernel-certificate {name} peak={peak} retained={} projector-added={added} apply-allocations=0",
            projector.certificate().retained_payload_bytes()?
        );
    }
    Ok(())
}
