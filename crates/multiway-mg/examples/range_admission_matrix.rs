//! Exact kernel certification and full-range terminal-action diagnostics.
//!
//! Reads one bounded `MG3AUT1` development input from stdin. Certifies the
//! exact incidence kernel, builds the range projector, then materializes three
//! fixed `within` actions against the dense oracle range of the unit-weight
//! Gramian on the same topology: the unchanged comparator (output-only
//! structural projection), two-sided structural projection, and two-sided
//! certified full-range projection. Diagnostic only; no routing or default.
use multiway_incidence::{
    IncidenceKernelCertificate, IncidenceKernelLimits, PreparedThreeWayTopology,
};
use multiway_mg::{
    DenseRangeDecomposition, MultiwayError, Preconditioner, RangeProjectedAction,
    SpectralAnalysisOptions, SpectralAnalysisReport, ThreeWayProblem, WithinApproxCholOptions,
    WithinApproxCholPreconditioner,
};
use std::{
    error::Error,
    io::{self, Read},
    time::Instant,
};
type Result<T = ()> = std::result::Result<T, Box<dyn Error>>;

const LIMITS: IncidenceKernelLimits = IncidenceKernelLimits {
    maximum_payload_bytes: 1 << 30,
};
const PROJECTOR_TOLERANCE: f64 = 1e-12;

fn bytes<const N: usize>(r: &mut impl Read) -> Result<[u8; N]> {
    let mut b = [0; N];
    r.read_exact(&mut b)?;
    Ok(b)
}

/// Factor level counts, canonical unique tuples and their positive weights.
type Input = ([usize; 3], Vec<[u32; 3]>, Vec<f64>);

fn decode(r: &mut impl Read) -> Result<Input> {
    if bytes::<8>(r)? != *b"MG3AUT1\0" {
        return Err("invalid automatic input magic".into());
    }
    let mut counts = [0; 3];
    for n in &mut counts {
        *n = u32::from_le_bytes(bytes(r)?) as usize;
    }
    let e = usize::try_from(u64::from_le_bytes(bytes(r)?))?;
    let rhs = u32::from_le_bytes(bytes(r)?) as usize;
    if counts.iter().any(|&n| n == 0 || n > 4096)
        || !(1..=100_000).contains(&e)
        || !(1..=32).contains(&rhs)
    {
        return Err("unsupported bounded development dimensions".into());
    }
    let mut tuples = vec![[0_u32; 3]; e];
    for key in &mut tuples {
        for x in key {
            *x = u32::from_le_bytes(bytes(r)?);
        }
    }
    let mut weights = vec![0.0; e];
    for w in &mut weights {
        *w = f64::from_le_bytes(bytes(r)?);
        if !w.is_finite() || *w <= 0. {
            return Err("invalid positive weight".into());
        }
    }
    // Targets are irrelevant to the fixed action; consume and discard them.
    let mut rest = Vec::new();
    r.read_to_end(&mut rest)?;
    if rest.len() != 8 * e * rhs {
        return Err("target payload length mismatch".into());
    }
    Ok((counts, tuples, weights))
}

/// Two-sided structural projection, the M6o audit's insufficient repair.
struct StructuralBoth<'a>(&'a ThreeWayProblem, &'a WithinApproxCholPreconditioner);
impl Preconditioner for StructuralBoth<'_> {
    fn dimension(&self) -> usize {
        self.0.dimension()
    }
    fn apply(&self, rhs: &[f64], out: &mut [f64]) -> std::result::Result<(), MultiwayError> {
        let mut projected = rhs.to_vec();
        self.0
            .components()
            .project_structural_range(&mut projected)?;
        self.1.apply(&projected, out)?;
        self.0.components().project_structural_range(out)?;
        Ok(())
    }
}

fn emit(action: &str, report: &SpectralAnalysisReport) {
    println!(
        "action\t{action}\t{:.17e}\t{:.17e}\t{:.17e}\t{:.17e}\t{:.17e}\t{}\t{}\t{}\t{}\t{}",
        report.preconditioner_symmetry_defect(),
        report.quotient_symmetry_defect(),
        report.range_leakage(),
        report.minimum_preconditioner_energy(),
        report.maximum_preconditioner_energy(),
        report.negative_preconditioner_directions(),
        report.near_zero_preconditioner_directions(),
        report.numerically_symmetric(),
        report.preserves_range(),
        report.positive_definite_on_range(),
    );
}

fn main() -> Result {
    let (counts, tuples, weights) = decode(&mut io::stdin().lock())?;
    let owner = PreparedThreeWayTopology::try_from_collapsed(counts, &tuples)?;
    let started = Instant::now();
    let certificate = IncidenceKernelCertificate::try_certify(&owner, LIMITS)?;
    let certification_ns = started.elapsed().as_nanos();
    let report = certificate.report();
    let largest = (0..certificate.component_count())
        .map(|c| certificate.component_coordinates(c).len())
        .max()
        .unwrap_or(0);
    let extended = (0..certificate.component_count())
        .filter(|&c| certificate.component_basis(c).is_some())
        .count();
    let large_extended = (0..certificate.component_count())
        .filter(|&c| {
            certificate.component_basis(c).is_some()
                && certificate.component_coordinates(c).len() > 256
        })
        .count();
    println!("dimension\t{}", certificate.dimension());
    println!("tuples\t{}", tuples.len());
    println!("components\t{}", certificate.component_count());
    println!("largest_component\t{largest}");
    println!("exact_rank\t{}", certificate.rank());
    println!("exact_nullity\t{}", certificate.nullity());
    println!("additional_nullity\t{}", certificate.additional_nullity());
    println!("extended_components\t{extended}");
    println!("large_extended_components\t{large_extended}");
    println!("eliminated_tuples\t{}", report.eliminated_tuples);
    println!("stored_pivot_entries\t{}", report.stored_pivot_entries);
    println!(
        "maximum_basis_magnitude\t{}",
        report.maximum_basis_magnitude
    );
    println!(
        "certificate_peak_bytes\t{}",
        report.peak_requested_payload_bytes
    );
    println!(
        "certificate_retained_bytes\t{}",
        certificate.retained_payload_bytes()?
    );
    println!("certification_ns\t{certification_ns}");
    let started = Instant::now();
    let projector = certificate.try_into_projector(&owner, LIMITS, PROJECTOR_TOLERANCE)?;
    let projector_ns = started.elapsed().as_nanos();
    let built = projector.report();
    println!(
        "projector_peak_bytes\t{}",
        built.peak_requested_payload_bytes
    );
    println!(
        "projector_retained_bytes\t{}",
        projector.retained_payload_bytes()?
    );
    println!(
        "projector_orthogonality\t{:.17e}",
        built.maximum_orthogonality_defect
    );
    println!(
        "projector_residual\t{:.17e}",
        built.maximum_incidence_residual
    );
    println!("projector_ns\t{projector_ns}");

    let options = SpectralAnalysisOptions {
        maximum_dimension: 1024,
        ..SpectralAnalysisOptions::default()
    };
    let unit = ThreeWayProblem::from_observations(counts, &tuples, &vec![1.0; tuples.len()])?;
    let exact_range = DenseRangeDecomposition::from_problem(&unit, options)?;
    println!("oracle_unit_nullity\t{}", exact_range.nullity());
    let problem = ThreeWayProblem::from_observations(counts, &tuples, &weights)?;
    let own_range = DenseRangeDecomposition::from_problem(&problem, options)?;
    println!("oracle_weighted_nullity\t{}", own_range.nullity());
    let within =
        WithinApproxCholPreconditioner::build(problem.clone(), WithinApproxCholOptions::default())?;
    println!(
        "action_header\tname\tfull_symmetry\tquotient_symmetry\trange_leakage\tminimum_energy\tmaximum_energy\tnegative\tnear_zero\tsymmetric\tpreserves_range\tpositive_on_range"
    );
    emit("within-output", &exact_range.analyze(&within, options)?);
    emit(
        "structural-both",
        &exact_range.analyze(&StructuralBoth(&problem, &within), options)?,
    );
    let full = RangeProjectedAction::new(&projector, within)?;
    emit("certified-full-both", &exact_range.analyze(&full, options)?);
    Ok(())
}
