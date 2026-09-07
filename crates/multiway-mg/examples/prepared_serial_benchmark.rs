//! Bounded cold-process development probe; orchestrate with scripts/prepared_serial.py.
use multiway_incidence::{
    FactorAggregation, HierarchyWeightFrames, PreparedHierarchyBudget, PreparedHierarchyTopology,
    PreparedThreeWayTopology, ThreeWayWeightFrame, WeightFrameInput,
};
use multiway_mg::{
    LeastSquaresStopReason, PcgStopReason, PreparedHierarchyPayloadReport,
    PreparedLsmrGateWorkReport, PreparedLsmrOptions, PreparedLsmrWorkspace, PreparedMapHierarchy,
    PreparedPcgOptions, PreparedPcgWorkspace, solve_prepared_least_squares,
    solve_prepared_least_squares_with_certificate_gate, solve_prepared_pcg_least_squares,
};
use std::{
    error::Error,
    io::{self, BufReader, Read},
    mem::size_of,
    time::Instant,
};

#[path = "support/prepared_layout.rs"]
mod layout;
type Result<T> = std::result::Result<T, Box<dyn Error>>;
const MAX_PAYLOAD: usize = 1 << 30;
const PHASES: [&str; 7] = [
    "decode",
    "fine_topology",
    "fine_frame",
    "maps_topology",
    "coarse_frames",
    "terminal",
    "workspace_output",
];

#[cfg(feature = "profiling")]
type CapturedProfile = multiway_incidence::profiling::ProfileReport;
#[cfg(not(feature = "profiling"))]
type CapturedProfile = ();
fn profiled<T>(f: impl FnOnce() -> T) -> Result<(T, CapturedProfile)> {
    #[cfg(feature = "profiling")]
    {
        Ok(multiway_incidence::profiling::collect(f)?)
    }
    #[cfg(not(feature = "profiling"))]
    {
        Ok((f(), ()))
    }
}
struct Input {
    counts: [usize; 3],
    tuples: Vec<[u32; 3]>,
    weights: Vec<f64>,
    targets: Vec<f64>,
    rhs: usize,
    depth: usize,
}
impl Input {
    fn payload(&self) -> usize {
        self.tuples.capacity() * 12 + (self.weights.capacity() + self.targets.capacity()) * 8
    }
}
fn vector<T: Default + Clone>(n: usize) -> Result<Vec<T>> {
    n.checked_mul(size_of::<T>())
        .filter(|b| *b <= MAX_PAYLOAD)
        .ok_or("input too large")?;
    let mut result = Vec::new();
    result.try_reserve_exact(n)?;
    result.resize(n, T::default());
    Ok(result)
}
fn bytes<const N: usize>(r: &mut impl Read) -> Result<[u8; N]> {
    let mut value = [0; N];
    r.read_exact(&mut value)?;
    Ok(value)
}
fn decode(r: &mut impl Read) -> Result<Input> {
    if bytes::<8>(r)? != *b"MG3BEN1\0" {
        return Err("invalid magic".into());
    }
    let mut counts = [0; 3];
    for count in &mut counts {
        *count = u32::from_le_bytes(bytes(r)?) as usize;
    }
    let e = usize::try_from(u64::from_le_bytes(bytes(r)?))?;
    let rhs = u32::from_le_bytes(bytes(r)?) as usize;
    let depth = u32::from_le_bytes(bytes(r)?) as usize;
    if counts.iter().any(|&n| n == 0 || n > 4096)
        || !(1..=100_000).contains(&e)
        || !(1..=32).contains(&rhs)
        || depth > 8
        || counts.iter().any(|&n| n % (1 << depth) != 0)
    {
        return Err("unsupported bounded development dimensions".into());
    }
    let mut tuples: Vec<[u32; 3]> = vector(e)?;
    for tuple in &mut tuples {
        for value in tuple {
            *value = u32::from_le_bytes(bytes(r)?);
        }
    }
    let mut weights = vector(e)?;
    for value in &mut weights {
        *value = f64::from_le_bytes(bytes(r)?);
        if !value.is_finite() || *value <= 0.0 {
            return Err("invalid weight".into());
        }
    }
    let mut targets = vector(e.checked_mul(rhs).ok_or("target count overflow")?)?;
    for value in &mut targets {
        *value = f64::from_le_bytes(bytes(r)?);
        if !value.is_finite() {
            return Err("invalid target".into());
        }
    }
    let mut trailing = [0];
    if r.read(&mut trailing)? != 0 {
        return Err("trailing input".into());
    }
    Ok(Input {
        counts,
        tuples,
        weights,
        targets,
        rhs,
        depth,
    })
}
fn maps(input: &Input) -> Result<Vec<FactorAggregation>> {
    let mut result = Vec::new();
    result.try_reserve_exact(input.depth)?;
    let mut counts = input.counts;
    for _ in 0..input.depth {
        let mut parents = [Vec::new(), Vec::new(), Vec::new()];
        for q in 0..3 {
            parents[q] = vector(counts[q])?;
            for (i, parent) in parents[q].iter_mut().enumerate() {
                *parent = (i / 2) as u32;
            }
        }
        result.push(FactorAggregation::new(counts, parents)?);
        counts = counts.map(|n| n / 2);
    }
    Ok(result)
}
#[derive(Clone, Copy)]
enum NativeStop {
    Pcg(PcgStopReason),
    Lsmr(LeastSquaresStopReason),
}
#[derive(Clone, Copy)]
struct Column {
    nanos: u128,
    prefix_nanos: u128,
    accepted: bool,
    native: bool,
    stop: NativeStop,
    iterations: usize,
    certificate: f64,
    residual: f64,
    secondary: f64,
    projection: Option<f64>,
    work: [usize; 8],
    fingerprint: u64,
    gate: Option<PreparedLsmrGateWorkReport>,
    #[cfg(feature = "profiling")]
    profile: CapturedProfile,
}
struct Record {
    start: Instant,
    layout: Option<layout::Record>,
    stage: &'static str,
    phases: [u128; 7],
    columns: [Option<Column>; 32],
    failed_work: Option<[usize; 8]>,
    failed_nanos: u128,
    failed_gate: Option<PreparedLsmrGateWorkReport>,
    #[cfg(feature = "profiling")]
    failed_profile: Option<CapturedProfile>,
    #[cfg(feature = "profiling")]
    profile_levels: [Option<[usize; 2]>; 9],
    dimensions: Option<[usize; 5]>,
    payload: Option<[usize; 9]>,
    total: u128,
}
impl Record {
    fn new() -> Self {
        Self {
            start: Instant::now(),
            layout: None,
            stage: "decode",
            phases: [0; 7],
            columns: [None; 32],
            failed_work: None,
            failed_nanos: 0,
            failed_gate: None,
            #[cfg(feature = "profiling")]
            failed_profile: None,
            #[cfg(feature = "profiling")]
            profile_levels: [None; 9],
            dimensions: None,
            payload: None,
            total: 0,
        }
    }
    fn payload(&mut self, h: PreparedHierarchyPayloadReport, outer: usize, caller: usize) {
        self.payload = Some([
            h.fine_topology_payload_bytes,
            h.coarse_topology_payload_bytes,
            h.fine_frame_payload_bytes,
            h.coarse_frame_payload_bytes,
            h.terminal_payload_bytes,
            h.workspace_payload_bytes,
            outer,
            caller,
            h.total_payload_bytes + outer + caller,
        ]);
    }
}
macro_rules! phase {
    ($record:expr, $index:expr, $expression:expr) => {{
        $record.stage = PHASES[$index];
        let start = Instant::now();
        let value = $expression;
        $record.phases[$index] += start.elapsed().as_nanos();
        value?
    }};
}
fn fingerprint(values: &[f64]) -> u64 {
    values
        .iter()
        .fold(0xcbf29ce484222325_u64, |mut hash, value| {
            for byte in value.to_bits().to_le_bytes() {
                hash = (hash ^ u64::from(byte)).wrapping_mul(0x100000001b3);
            }
            hash
        })
}
fn pcg_work(w: multiway_mg::PreparedPcgWorkReport) -> [usize; 8] {
    [
        0,
        0,
        w.gramian_applications,
        w.rhs_adjoint_applications,
        w.hierarchy_applications,
        w.projection_applications,
        w.certificate.incidence_applications,
        w.certificate.adjoint_applications,
    ]
}
fn lsmr_work(w: multiway_mg::PreparedLsmrWorkReport) -> [usize; 8] {
    [
        w.weighted_incidence_applications,
        w.weighted_adjoint_applications,
        0,
        0,
        w.hierarchy_applications,
        0,
        w.certificate.incidence_applications,
        w.certificate.adjoint_applications,
    ]
}
fn lsmr_full_work(
    w: multiway_mg::PreparedLsmrWorkReport,
    gate: PreparedLsmrGateWorkReport,
) -> [usize; 8] {
    let mut result = lsmr_work(w);
    result[5] = gate.projection_applications;
    result[6] += gate.candidate_certificate.incidence_applications;
    result[7] += gate.candidate_certificate.adjoint_applications;
    result
}
fn emit_gate(tag: &str, prefix: &str, gate: PreparedLsmrGateWorkReport) {
    println!(
        "{tag}{prefix}\t{}\t{}\t{}\t{}\t{}",
        gate.candidate_checks,
        gate.candidate_vetoes,
        gate.projection_applications,
        gate.candidate_certificate.incidence_applications,
        gate.candidate_certificate.adjoint_applications
    );
}
#[cfg(feature = "profiling")]
fn emit_profile(tag: &str, index: usize, profile: CapturedProfile) {
    println!(
        "{tag}\t{index}\t{}\t{}\t{}",
        profile.valid, profile.elapsed_ns, profile.maximum_depth
    );
    for (phase, record) in multiway_incidence::profiling::PHASES
        .iter()
        .zip(profile.phases)
    {
        println!(
            "{tag}_phase\t{index}\t{}\t{}\t{}\t{}",
            phase.name(),
            record.calls,
            record.inclusive_ns,
            record.exclusive_ns
        );
    }
}
fn run(route: &str, r: &mut Record) -> Result<()> {
    let input = phase!(
        r,
        0,
        decode(&mut BufReader::with_capacity(65536, io::stdin().lock()))
    );
    let e = input.tuples.len();
    let n = input.counts.iter().sum::<usize>();
    r.dimensions = Some([e, n, input.rhs, input.depth, 0]);
    let fine = phase!(
        r,
        1,
        PreparedThreeWayTopology::try_from_collapsed_with_budget(
            input.counts,
            &input.tuples,
            MAX_PAYLOAD - input.payload()
        )
    );
    let frame = phase!(
        r,
        2,
        ThreeWayWeightFrame::try_new(&fine, WeightFrameInput::Tuples(&input.weights))
    );
    let topology = phase!(
        r,
        3,
        (|| -> Result<_> {
            Ok(PreparedHierarchyTopology::try_new_with_budget(
                &fine,
                maps(&input)?,
                PreparedHierarchyBudget {
                    maximum_payload_bytes: MAX_PAYLOAD,
                    additional_live_payload_bytes: input.payload()
                        + frame.retained_payload_bytes()?,
                },
            )?)
        })()
    );
    // Grouping is structural: build before coarse frames and declare all live
    // input/fine-frame arrays in its separate construction admission.
    let grouping = if let Some(layout) = r.layout.as_mut() {
        r.stage = "grouping";
        let start = Instant::now();
        let result = layout.prepare(
            &topology,
            input.payload() + frame.retained_payload_bytes()?,
            MAX_PAYLOAD,
        );
        layout.nanos = start.elapsed().as_nanos();
        result?
    } else {
        None
    };
    let group_bytes = grouping
        .as_ref()
        .map_or(Ok(0), |g| g.retained_payload_bytes())?;
    let frames = phase!(
        r,
        4,
        HierarchyWeightFrames::try_new_with_budget(
            &topology,
            &frame,
            PreparedHierarchyBudget {
                maximum_payload_bytes: MAX_PAYLOAD,
                additional_live_payload_bytes: input.payload() + group_bytes
            }
        )
    );
    #[cfg(feature = "profiling")]
    for level in 0..frames.level_count() {
        let frame = frames.frame(level).expect("prepared level");
        r.profile_levels[level] = Some([frame.weights().len(), frame.diagonal().len()]);
    }
    let hierarchy = phase!(
        r,
        5,
        if let Some(groups) = grouping.as_ref() {
            PreparedMapHierarchy::try_new_with_grouping(
                &frames,
                groups,
                r.layout.as_ref().expect("explicit layout").layout.mode(),
                1e-12,
            )
        } else {
            PreparedMapHierarchy::try_new(&frames, 1e-12)
        }
    );
    if let Some(layout) = r.layout.as_mut() {
        layout.image_len = hierarchy.tuple_image_len();
    }
    r.dimensions = Some([e, n, input.rhs, input.depth, hierarchy.terminal_rank()]);
    let mut output: Vec<f64> = phase!(r, 6, vector(n * input.rhs));
    let caller = input.payload() + output.capacity() * 8;
    if route == "pcg" {
        let options = PreparedPcgOptions::default();
        let mut workspace = phase!(
            r,
            6,
            PreparedPcgWorkspace::try_new_with_payload_budget(
                &hierarchy,
                options,
                MAX_PAYLOAD,
                caller
            )
        );
        let p = workspace.payload_report(caller)?;
        r.payload(p.hierarchy, p.outer_workspace_payload_bytes, caller);
        for j in 0..input.rhs {
            r.stage = "solve_certificate_output";
            let start = Instant::now();
            let (solved, _profile) = profiled(|| {
                solve_prepared_pcg_least_squares(
                    &hierarchy,
                    &input.targets[j * e..(j + 1) * e],
                    &mut workspace,
                )
                .map(|result| {
                    output[j * n..(j + 1) * n].copy_from_slice(result.coefficients);
                    (result.report, fingerprint(&output[j * n..(j + 1) * n]))
                })
            })?;
            match solved {
                Ok((p, fingerprint)) => {
                    r.columns[j] = Some(Column {
                        nanos: start.elapsed().as_nanos(),
                        prefix_nanos: r.start.elapsed().as_nanos(),
                        accepted: p.accepted,
                        native: p.native_converged,
                        stop: NativeStop::Pcg(p.native_stop_reason),
                        iterations: p.iterations,
                        certificate: p.certified_normal_equation_residual,
                        residual: p.native_residual_norm,
                        secondary: p.native_relative_residual,
                        projection: Some(p.rhs_projection_norm),
                        work: pcg_work(p.work),
                        fingerprint,
                        gate: None,
                        #[cfg(feature = "profiling")]
                        profile: _profile,
                    });
                }
                Err(error) => {
                    r.failed_nanos = start.elapsed().as_nanos();
                    #[cfg(feature = "profiling")]
                    {
                        r.failed_profile = Some(_profile);
                    }
                    r.failed_work = Some(pcg_work(workspace.last_work()));
                    return Err(error.into());
                }
            }
        }
    } else {
        let options = PreparedLsmrOptions::default();
        let mut workspace = phase!(
            r,
            6,
            PreparedLsmrWorkspace::try_new_with_payload_budget(
                &hierarchy,
                options,
                MAX_PAYLOAD,
                caller
            )
        );
        let p = workspace.payload_report(caller)?;
        r.payload(p.hierarchy, p.outer_workspace_payload_bytes, caller);
        for j in 0..input.rhs {
            r.stage = "solve_certificate_output";
            let start = Instant::now();
            let (solved, _profile) = profiled(|| {
                let solved = if route == "lsmr-gated" {
                    solve_prepared_least_squares_with_certificate_gate(
                        &hierarchy,
                        &input.targets[j * e..(j + 1) * e],
                        &mut workspace,
                    )
                    .map(|r| (r.coefficients, r.report.solve))
                } else {
                    solve_prepared_least_squares(
                        &hierarchy,
                        &input.targets[j * e..(j + 1) * e],
                        &mut workspace,
                    )
                    .map(|r| (r.coefficients, r.report))
                };
                solved.map(|(coefficients, p)| {
                    output[j * n..(j + 1) * n].copy_from_slice(coefficients);
                    (p, fingerprint(&output[j * n..(j + 1) * n]))
                })
            })?;
            match solved {
                Ok((p, fingerprint)) => {
                    let gate = workspace.last_gate_work();
                    r.columns[j] = Some(Column {
                        nanos: start.elapsed().as_nanos(),
                        prefix_nanos: r.start.elapsed().as_nanos(),
                        accepted: p.accepted,
                        native: p.native_converged,
                        stop: NativeStop::Lsmr(p.native_stop_reason),
                        iterations: p.iterations,
                        certificate: p.certified_normal_equation_residual,
                        residual: p.native_residual_norm,
                        secondary: p.native_normal_equation_residual,
                        projection: None,
                        work: lsmr_full_work(p.work, gate),
                        fingerprint,
                        gate: Some(gate),
                        #[cfg(feature = "profiling")]
                        profile: _profile,
                    });
                }
                Err(error) => {
                    r.failed_nanos = start.elapsed().as_nanos();
                    #[cfg(feature = "profiling")]
                    {
                        r.failed_profile = Some(_profile);
                    }
                    r.failed_gate = Some(workspace.last_gate_work());
                    r.failed_work = Some(lsmr_full_work(
                        workspace.last_work(),
                        workspace.last_gate_work(),
                    ));
                    return Err(error.into());
                }
            }
        }
    }
    std::hint::black_box(&output);
    Ok(())
}
fn main() {
    let route = std::env::args().nth(1).unwrap_or_default();
    if route != "pcg" && route != "lsmr" && route != "lsmr-gated" {
        eprintln!("usage: prepared_serial_benchmark pcg|lsmr|lsmr-gated < canonical-input.bin");
        std::process::exit(2);
    }
    let requested = std::env::args().nth(2);
    let selected = requested.as_deref().and_then(layout::Layout::parse);
    if (requested.is_some() && selected.is_none())
        || std::env::args().count() > 3
        || (cfg!(feature = "profiling") && selected.is_some())
    {
        eprintln!(
            "optional explicit layout: scalar|fine-row|all-row|fine-image|all-image; profiling must be disabled"
        );
        std::process::exit(2);
    }
    let mut r = Record::new();
    r.layout = selected.map(layout::Record::new);
    let outcome = run(&route, &mut r);
    r.total = r.start.elapsed().as_nanos(); // Includes failure unwind and all owned-state destruction.
    println!("schema\t{}", if r.layout.is_some() { 3 } else { 2 });
    if let Some(layout) = r.layout.as_ref() {
        layout.emit(size_of::<Record>());
    }
    #[cfg(feature = "profiling")]
    println!(
        "profiling_metadata\t1\t{}\t{}\t{}",
        multiway_incidence::profiling::thread_local_payload_bytes(),
        size_of::<Record>(),
        size_of::<CapturedProfile>()
    );
    #[cfg(feature = "profiling")]
    for (index, dimensions) in r.profile_levels.iter().enumerate() {
        if let Some([e, n]) = dimensions {
            println!("profile_level\t{index}\t{e}\t{n}");
        }
    }
    println!("route\t{route}");
    let pcg = PreparedPcgOptions::default();
    let lsmr = PreparedLsmrOptions::default();
    let (native_tolerance, certificate_tolerance, max_iterations) = if route == "pcg" {
        (
            pcg.pcg.relative_tolerance,
            pcg.certificate_tolerance,
            pcg.pcg.max_iterations,
        )
    } else {
        (
            lsmr.tolerance,
            lsmr.certificate_tolerance,
            lsmr.max_iterations,
        )
    };
    println!(
        "config\t{native_tolerance:.17e}\t{certificate_tolerance:.17e}\t{max_iterations}\t{}\t{}\t{:.17e}\t{MAX_PAYLOAD}",
        lsmr.local_size.unwrap_or(0),
        pcg.pcg.residual_recompute_interval,
        1e-12
    );

    if let Some(d) = r.dimensions {
        println!(
            "dimensions\t{}\t{}\t{}\t{}\t{}",
            d[0], d[1], d[2], d[3], d[4]
        );
    }
    for (name, nanos) in PHASES.iter().zip(r.phases) {
        println!("phase\t{name}\t{nanos}");
    }
    if let Some(payload) = r.payload {
        print!("payload");
        for value in payload {
            print!("\t{value}");
        }
        println!();
    }
    for (index, c) in r
        .columns
        .iter()
        .enumerate()
        .filter_map(|(i, c)| c.as_ref().map(|c| (i, c)))
    {
        let stop = match c.stop {
            NativeStop::Pcg(s) => format!("{s:?}"),
            NativeStop::Lsmr(s) => format!("{s:?}"),
        };
        print!(
            "column\t{index}\t{}\t{}\t{}\t{}\t{stop}\t{}\t{:.17e}\t{:.17e}\t{:.17e}\t{}\t{:016x}",
            c.nanos,
            c.prefix_nanos,
            c.accepted,
            c.native,
            c.iterations,
            c.certificate,
            c.residual,
            c.secondary,
            c.projection
                .map_or_else(|| "NA".to_owned(), |v| format!("{v:.17e}")),
            c.fingerprint
        );
        for count in c.work {
            print!("\t{count}");
        }
        println!();
        #[cfg(feature = "profiling")]
        emit_profile("profile", index, c.profile);
        if let Some(gate) = c.gate {
            emit_gate("gate", &format!("\t{index}"), gate);
        }
    }
    if let Some(work) = r.failed_work {
        print!("failed_work\t{}", r.failed_nanos);
        for count in work {
            print!("\t{count}");
        }
        println!();
    }
    #[cfg(feature = "profiling")]
    if let Some(profile) = r.failed_profile {
        emit_profile(
            "failed_profile",
            r.columns.iter().flatten().count(),
            profile,
        );
    }
    if let Some(gate) = r.failed_gate {
        emit_gate("failed_gate", "", gate);
    }
    let measured: u128 = r.failed_nanos
        + r.layout.as_ref().map_or(0, |layout| layout.nanos)
        + r.phases.iter().sum::<u128>()
        + r.columns.iter().flatten().map(|c| c.nanos).sum::<u128>();
    // Remaining time contains setup bookkeeping, inter-phase gaps and teardown.
    println!("total\t{}\t{}", r.total, r.total - measured);
    match outcome {
        Ok(()) => println!("status\tcomplete"),
        Err(error) => {
            println!(
                "status\terror\t{}\t{}",
                r.stage,
                error.to_string().replace(['\t', '\r', '\n'], " ")
            );
            std::process::exit(1);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn tiny() -> Vec<u8> {
        let mut result = b"MG3BEN1\0".to_vec();
        for value in [1_u32; 3] {
            result.extend(value.to_le_bytes());
        }
        result.extend(1_u64.to_le_bytes());
        result.extend(1_u32.to_le_bytes());
        result.extend(0_u32.to_le_bytes());
        for value in [0_u32; 3] {
            result.extend(value.to_le_bytes());
        }
        result.extend(1.0_f64.to_le_bytes());
        result.extend(0.0_f64.to_le_bytes());
        result
    }
    #[test]
    fn bounded_decoder_rejects_truncation_trailing_and_nonfinite() {
        let data = tiny();
        assert_eq!(decode(&mut &data[..]).unwrap().targets, [0.0]);
        for end in 0..data.len() {
            assert!(decode(&mut &data[..end]).is_err());
        }
        let mut changed = data.clone();
        changed.push(0);
        assert!(decode(&mut &changed[..]).is_err());
        for offset in [48, 56] {
            let mut changed = data.clone();
            changed[offset..offset + 8].copy_from_slice(&f64::INFINITY.to_le_bytes());
            assert!(decode(&mut &changed[..]).is_err());
        }
        let mut changed = data;
        changed[20..28].copy_from_slice(&u64::MAX.to_le_bytes());
        assert!(decode(&mut &changed[..]).is_err());
    }
}
