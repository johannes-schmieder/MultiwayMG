//! Complete cold automatic/control probe; independent of frozen supplied-map protocols.
use multiway_incidence::{
    PreparedHierarchyBudget, PreparedThreeWayTopology, PreparedTopologySource, ThreeWayWeightFrame,
    WeightFrameInput, WeightFramePayloadBudget,
};
use multiway_mg::*;
use std::{
    error::Error,
    io::{self, BufReader, Read},
    mem::size_of,
    time::Instant,
};
type Result<T = ()> = std::result::Result<T, Box<dyn Error>>;
const MAX_PAYLOAD: usize = 1 << 30;
const PHASES: [&str; 6] = [
    "decode",
    "fine_topology",
    "fine_frame",
    "output",
    "driver",
    "materialize",
];
const ROUTES: [&str; 5] = [
    "automatic",
    "components-map",
    "global-map",
    "global-diagonal",
    "global-identity",
];

struct Input {
    counts: [usize; 3],
    tuples: Vec<[u32; 3]>,
    weights: Vec<f64>,
    targets: Vec<f64>,
    rhs: usize,
}
impl Input {
    fn payload(&self) -> usize {
        12 * self.tuples.capacity() + 8 * (self.weights.capacity() + self.targets.capacity())
    }
}
fn vector<T: Clone + Default>(n: usize) -> Result<Vec<T>> {
    n.checked_mul(size_of::<T>())
        .filter(|&n| n <= MAX_PAYLOAD)
        .ok_or("input allocation too large")?;
    let mut v = Vec::new();
    v.try_reserve_exact(n)?;
    v.resize(n, T::default());
    Ok(v)
}
fn bytes<const N: usize>(r: &mut impl Read) -> Result<[u8; N]> {
    let mut b = [0; N];
    r.read_exact(&mut b)?;
    Ok(b)
}
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
    let mut tuples: Vec<[u32; 3]> = vector(e)?;
    for key in &mut tuples {
        for x in key {
            *x = u32::from_le_bytes(bytes(r)?);
        }
    }
    let mut weights: Vec<f64> = vector(e)?;
    for w in &mut weights {
        *w = f64::from_le_bytes(bytes(r)?);
        if !w.is_finite() || *w <= 0. {
            return Err("invalid positive weight".into());
        }
    }
    let mut targets = vector(e.checked_mul(rhs).ok_or("target size overflow")?)?;
    for y in &mut targets {
        *y = f64::from_le_bytes(bytes(r)?);
        if !y.is_finite() {
            return Err("invalid finite target".into());
        }
    }
    if r.read(&mut [0u8])? != 0 {
        return Err("trailing input".into());
    }
    Ok(Input {
        counts,
        tuples,
        weights,
        targets,
        rhs,
    })
}
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
                correction_damping: 1.,
                seed: 0x4d57_4d47_4359_4331,
                relative_zero_tolerance: 1e-13,
            },
            criteria: CycleQualityCriteria {
                maximum_estimated_energy_factor: 0.8,
                maximum_observed_energy_factor: Some(1.05),
                maximum_structural_defect: 1e-10,
            },
        }),
        terminal_relative_tolerance: 1e-12,
        fallback: PreparedBaselineKind::SymmetricMap,
        lsmr: PreparedLsmrOptions {
            tolerance: 1e-8,
            certificate_tolerance: 1e-8,
            max_iterations: 1000,
            local_size: Some(8),
        },
    }
}
#[derive(Clone, Copy)]
struct Column {
    report: PreparedAutomaticColumnReport,
    fingerprint: u64,
}
struct Record {
    stage: &'static str,
    phases: [u128; 6],
    total: u128,
    dimensions: Option<[usize; 6]>,
    max_requested: usize,
    max_admitted: usize,
    fine: usize,
    caller: usize,
    progress: PreparedAutomaticProgress,
    columns: [Option<Column>; 32],
    native: [Option<PreparedGatedLsmrReport>; 32],
    baseline_work: [usize; 10],
}
impl Record {
    fn new() -> Self {
        Self {
            stage: "cli",
            phases: [0; 6],
            total: 0,
            dimensions: None,
            max_requested: 0,
            max_admitted: 0,
            fine: 0,
            caller: 0,
            progress: PreparedAutomaticProgress::default(),
            columns: [None; 32],
            native: [None; 32],
            baseline_work: [0; 10],
        }
    }
    fn admit(&mut self, n: usize) -> Result {
        self.max_requested = self.max_requested.max(n);
        if n > MAX_PAYLOAD {
            return Err("benchmark payload budget exceeded".into());
        }
        self.max_admitted = self.max_admitted.max(n);
        Ok(())
    }
}
macro_rules! phase {
    ($r:expr,$i:expr,$expression:expr) => {{
        $r.stage = PHASES[$i];
        let start = Instant::now();
        let result = $expression;
        $r.phases[$i] += start.elapsed().as_nanos();
        result?
    }};
}
fn fingerprint(values: &[f64]) -> u64 {
    values.iter().fold(0xcbf29ce484222325_u64, |mut h, x| {
        for b in x.to_bits().to_le_bytes() {
            h = (h ^ u64::from(b)).wrapping_mul(0x100000001b3);
        }
        h
    })
}
fn work(w: PreparedLsmrWorkReport, g: PreparedLsmrGateWorkReport) -> [usize; 10] {
    [
        w.weighted_incidence_applications,
        w.weighted_adjoint_applications,
        w.hierarchy_applications,
        w.certificate.incidence_applications,
        w.certificate.adjoint_applications,
        g.candidate_checks,
        g.candidate_vetoes,
        g.projection_applications,
        g.candidate_certificate.incidence_applications,
        g.candidate_certificate.adjoint_applications,
    ]
}
fn accumulate(dst: &mut [usize; 10], src: [usize; 10]) -> Result {
    for (x, y) in dst.iter_mut().zip(src) {
        *x = x.checked_add(y).ok_or("work counter overflow")?;
    }
    Ok(())
}
fn baseline(
    frame: &ThreeWayWeightFrame<'_>,
    input: &Input,
    x: &mut [f64],
    reports: &mut [Option<PreparedAutomaticColumnReport>],
    kind: PreparedBaselineKind,
    r: &mut Record,
) -> Result {
    let owner = PreparedBaseline::new(frame, kind);
    let opts = options().lsmr;
    r.admit(PreparedLsmrWorkspace::setup_payload_bound(
        &owner, opts, r.caller,
    )?)?;
    let mut ws =
        PreparedLsmrWorkspace::try_new_with_payload_budget(&owner, opts, MAX_PAYLOAD, r.caller)?;
    r.admit(ws.payload_report(r.caller)?.total_payload_bytes)?;
    let e = input.tuples.len();
    let v = frame.diagonal().len();
    for j in 0..input.rhs {
        match solve_prepared_least_squares_with_certificate_gate(
            &owner,
            &input.targets[j * e..(j + 1) * e],
            &mut ws,
        ) {
            Ok(s) => {
                accumulate(
                    &mut r.baseline_work,
                    work(s.report.solve.work, s.report.gate),
                )?;
                x[j * v..(j + 1) * v].copy_from_slice(s.coefficients);
                r.native[j] = Some(s.report);
                reports[j] = Some(PreparedAutomaticColumnReport {
                    initial_certificate: None,
                    certified_normal_equation_residual: s
                        .report
                        .solve
                        .certified_normal_equation_residual,
                    accepted: s.report.solve.accepted,
                    global_fallback: false,
                });
            }
            Err(e) => {
                accumulate(
                    &mut r.baseline_work,
                    work(ws.last_work(), ws.last_gate_work()),
                )?;
                return Err(e.into());
            }
        }
    }
    Ok(())
}
fn run(route: &str, reader: &mut impl Read, r: &mut Record) -> Result {
    if !ROUTES.contains(&route) {
        return Err("unknown route".into());
    }
    let input = phase!(r, 0, decode(&mut BufReader::with_capacity(65536, reader)));
    let e = input.tuples.len();
    let v = input.counts.iter().sum::<usize>();
    r.caller = input.payload();
    r.dimensions = Some([
        input.counts[0],
        input.counts[1],
        input.counts[2],
        e,
        input.rhs,
        0,
    ]);
    r.admit(input.payload() + 65536)?;
    r.admit(
        input.payload()
            + PreparedThreeWayTopology::setup_payload_bound(
                input.counts,
                e,
                PreparedTopologySource::Collapsed,
            )?,
    )?;
    let fine = phase!(
        r,
        1,
        PreparedThreeWayTopology::try_from_collapsed_with_budget(
            input.counts,
            &input.tuples,
            MAX_PAYLOAD - input.payload()
        )
    );
    r.dimensions.as_mut().unwrap()[5] = fine.component_factor_sizes().len();
    r.fine = fine.retained_payload_bytes()?;
    let input_weights = WeightFrameInput::Tuples(&input.weights);
    let other = input.payload() - size_of_val(input.weights.as_slice());
    r.admit(
        ThreeWayWeightFrame::setup_payload_report(&fine, input_weights, other)?.total_payload_bytes,
    )?;
    let frame = phase!(
        r,
        2,
        ThreeWayWeightFrame::try_new_with_budget(
            &fine,
            input_weights,
            WeightFramePayloadBudget {
                maximum_payload_bytes: MAX_PAYLOAD,
                additional_live_payload_bytes: other
            }
        )
    );
    r.fine = fine.retained_payload_bytes()? + frame.retained_payload_bytes()?;
    r.admit(
        r.fine
            + input.payload()
            + 8 * v * input.rhs
            + size_of::<Option<PreparedAutomaticColumnReport>>() * input.rhs,
    )?;
    let (mut x, mut reports) = phase!(
        r,
        3,
        (|| -> Result<_> {
            Ok((
                vector::<f64>(v * input.rhs)?,
                vector::<Option<PreparedAutomaticColumnReport>>(input.rhs)?,
            ))
        })()
    );
    r.caller = input.payload()
        + 8 * x.capacity()
        + size_of::<Option<PreparedAutomaticColumnReport>>() * reports.capacity();
    r.admit(r.fine + r.caller)?;
    r.stage = "driver";
    let start = Instant::now();
    let result = if route == "automatic" || route == "components-map" {
        let mut o = options();
        if route == "components-map" {
            o.hierarchy = None;
        }
        let additional = r.caller
            - size_of_val(input.targets.as_slice())
            - size_of_val(x.as_slice())
            - size_of_val(reports.as_slice());
        let outcome = solve_prepared_automatic_batch_into(
            &frame,
            PreparedAutomaticBatch {
                targets: &input.targets,
                columns: input.rhs,
                coefficients: &mut x,
                reports: &mut reports,
            },
            o,
            PreparedHierarchyBudget {
                maximum_payload_bytes: MAX_PAYLOAD,
                additional_live_payload_bytes: additional,
            },
            &mut r.progress,
        );
        r.max_requested = r
            .max_requested
            .max(r.progress.maximum_requested_payload_bytes);
        r.max_admitted = r
            .max_admitted
            .max(r.progress.maximum_admitted_payload_bytes);
        outcome.map_err(|e| Box::new(e) as Box<dyn Error>)
    } else {
        let kind = match route {
            "global-map" => PreparedBaselineKind::SymmetricMap,
            "global-diagonal" => PreparedBaselineKind::InverseDiagonal,
            "global-identity" => PreparedBaselineKind::Identity,
            _ => unreachable!(),
        };
        baseline(&frame, &input, &mut x, &mut reports, kind, r)
    };
    r.phases[4] = start.elapsed().as_nanos();
    // Preserve completed-column records even on a later failed column. Incomplete
    // outputs never receive a fabricated report or successful fingerprint.
    let start = Instant::now();
    for (j, report) in reports.iter().enumerate() {
        if let Some(report) = report {
            r.columns[j] = Some(Column {
                report: *report,
                fingerprint: fingerprint(&x[j * v..(j + 1) * v]),
            });
        }
    }
    r.phases[5] = start.elapsed().as_nanos();
    result?;
    r.stage = "complete";
    Ok(())
}
fn emit(route: &str, r: &Record, outcome: &Result) {
    println!(
        "schema\t1\nroute\t{route}\nprofiling\t{}\nrecord_bytes\t{}",
        cfg!(feature = "profiling"),
        size_of::<Record>()
    );
    let o = options();
    let h = o.hierarchy.unwrap();
    for (name, value) in [
        ("native_tolerance", o.lsmr.tolerance),
        ("certificate_tolerance", o.lsmr.certificate_tolerance),
        ("terminal_relative_tolerance", o.terminal_relative_tolerance),
        ("minimum_affinity", h.candidate.minimum_affinity),
        (
            "maximum_estimated_energy_factor",
            h.criteria.maximum_estimated_energy_factor,
        ),
        (
            "maximum_observed_energy_factor",
            h.criteria.maximum_observed_energy_factor.unwrap(),
        ),
        (
            "maximum_structural_defect",
            h.criteria.maximum_structural_defect,
        ),
        ("correction_damping", h.screen.correction_damping),
        ("relative_zero_tolerance", h.screen.relative_zero_tolerance),
    ] {
        println!("parameter\t{name}\t{value:.17e}");
    }
    for (name, value) in [
        ("max_iterations", o.lsmr.max_iterations as u64),
        ("local_window", o.lsmr.local_size.unwrap() as u64),
        ("maximum_transitions", h.maximum_transitions as u64),
        (
            "maximum_tuple_multiplier",
            h.maximum_tuple_multiplier as u64,
        ),
        (
            "maximum_coefficient_multiplier",
            h.maximum_coefficient_multiplier as u64,
        ),
        ("test_vectors", h.screen.test_vectors as u64),
        ("power_iterations", h.screen.power_iterations as u64),
        ("tail_iterations", h.screen.tail_iterations as u64),
        ("screen_seed", h.screen.seed),
        ("process_budget_bytes", MAX_PAYLOAD as u64),
        ("usize_bytes", size_of::<usize>() as u64),
        (
            "report_slot_bytes",
            size_of::<Option<PreparedAutomaticColumnReport>>() as u64,
        ),
    ] {
        println!("parameter\t{name}\t{value}");
    }
    if let Some([a, b, c, e, k, components]) = r.dimensions {
        println!("dimensions\t{a}\t{b}\t{c}\t{e}\t{k}\t{components}");
    }
    for (name, value) in PHASES.iter().zip(r.phases) {
        println!("phase\t{name}\t{value}");
    }
    if r.dimensions.is_some() {
        println!(
            "payload\t{}\t{}\t{}\t{}",
            r.fine, r.caller, r.max_requested, r.max_admitted
        );
    } else {
        println!("payload\tNA\tNA\tNA\tNA");
    }
    let p = &r.progress;
    for (name, value) in [
        ("components", p.components),
        ("singleton_components", p.singleton_components),
        ("dense_components", p.dense_components),
        ("large_components", p.large_components),
        ("hierarchy_attempts", p.hierarchy_attempts),
        ("accepted_hierarchies", p.accepted_hierarchies),
        ("hierarchy_rejections", p.hierarchy_rejections),
        ("quality_rejections", p.quality_rejections),
        ("baseline_components", p.baseline_components),
        ("global_fallback_columns", p.global_fallback_columns),
        ("rejections", p.rejections),
        ("structural_attempts", p.structural_attempts),
        ("structural_input_tuples", p.structural_input_tuples),
        ("provisional_input_tuples", p.provisional_input_tuples),
        ("candidate_tuple_visits", p.candidate_work.tuple_visits),
        ("candidate_pair_entries", p.candidate_work.pair_entries),
        ("candidate_proposals", p.candidate_work.proposals),
        (
            "candidate_unique_candidates",
            p.candidate_work.unique_candidates,
        ),
        (
            "candidate_truncated_neighbors",
            p.candidate_work.truncated_neighbors,
        ),
        ("candidate_accepted_pairs", p.candidate_work.accepted_pairs),
        ("screen_gramian", p.screen_work.gramian_applications),
        ("screen_cycle", p.screen_work.cycle_applications),
        ("screen_energy", p.screen_work.energy_evaluations),
        ("screen_projections", p.screen_work.projections),
        ("screen_defects", p.screen_work.defect_evaluations),
        ("global_projection", p.global_projection_applications),
        (
            "global_certificate_incidence",
            p.global_certificate_work.incidence_applications,
        ),
        (
            "global_certificate_adjoint",
            p.global_certificate_work.adjoint_applications,
        ),
    ] {
        println!("count\t{name}\t{value}");
    }
    let w = if route.starts_with("global-") {
        r.baseline_work
    } else {
        work(p.solve_work, p.gate_work)
    };
    print!("solve_work");
    for x in w {
        print!("\t{x}");
    }
    println!();
    if route.starts_with("global-") {
        println!("progress\tNA\tNA\tNA");
    } else {
        println!(
            "progress\t{:?}\t{}\t{}",
            p.stage,
            p.component.map_or("NA".to_owned(), |x| x.to_string()),
            p.column.map_or("NA".to_owned(), |x| x.to_string())
        );
    }
    if let Some(rejection) = &p.last_rejection {
        println!(
            "rejection\t{:?}\t{}\t{}",
            rejection.stage,
            rejection
                .component
                .map_or("NA".to_owned(), |x| x.to_string()),
            rejection
                .source
                .to_string()
                .replace(['\t', '\r', '\n'], " ")
        );
    }
    if let Some((component, q)) = p.last_quality_rejection {
        println!(
            "quality\t{component}\t{}\t{}\t{}\t{}\t{:.17e}\t{:.17e}\t{:.17e}\t{:.17e}\t{}",
            q.level,
            q.dimension,
            q.completed_starts,
            q.annihilated_starts,
            q.maximum_estimated_energy_factor,
            q.maximum_observed_energy_factor,
            q.maximum_absolute_final_rayleigh,
            q.maximum_structural_defect,
            q.accepted
        );
    }
    for (j, col) in r.columns.iter().enumerate() {
        if let Some(c) = col {
            println!(
                "column\t{j}\t{}\t{:.17e}\t{}\t{}\t{:016x}",
                c.report.accepted,
                c.report.certified_normal_equation_residual,
                c.report
                    .initial_certificate
                    .map_or("NA".to_owned(), |x| format!("{x:.17e}")),
                c.report.global_fallback,
                c.fingerprint
            );
            if let Some(n) = r.native[j] {
                println!(
                    "native\t{j}\t{}\t{:?}\t{}\t{:.17e}\t{:.17e}",
                    n.solve.native_converged,
                    n.solve.native_stop_reason,
                    n.solve.iterations,
                    n.solve.native_residual_norm,
                    n.solve.native_normal_equation_residual
                );
            }
        }
    }
    println!(
        "total\t{}\t{}",
        r.total,
        r.total - r.phases.iter().sum::<u128>()
    );
    match outcome {
        Ok(()) => println!("status\tcomplete"),
        Err(e) => println!(
            "status\terror\t{}\t{}",
            r.stage,
            e.to_string().replace(['\t', '\r', '\n'], " ")
        ),
    }
}
fn main() {
    let mut r = Record::new();
    let start = Instant::now();
    let mut args = std::env::args().skip(1);
    let route = args.next().unwrap_or_default();
    let result = if args.next().is_some() {
        Err("exactly one route required".into())
    } else {
        // Drop the input buffer before ending the measured inner interval.
        run(&route, &mut io::stdin().lock(), &mut r)
    };
    r.total = start.elapsed().as_nanos();
    emit(&route, &r, &result);
    if result.is_err() {
        std::process::exit(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn input(k: usize) -> Vec<u8> {
        let mut b = b"MG3AUT1\0".to_vec();
        for n in [2u32; 3] {
            b.extend(n.to_le_bytes());
        }
        b.extend(8u64.to_le_bytes());
        b.extend((k as u32).to_le_bytes());
        for i in 0u32..2 {
            for j in 0u32..2 {
                for z in 0u32..2 {
                    for n in [i, j, z] {
                        b.extend(n.to_le_bytes());
                    }
                }
            }
        }
        for _ in 0..8 {
            b.extend(1f64.to_le_bytes());
        }
        for j in 0..k {
            for i in 0..8 {
                b.extend((if j == k - 1 { 0. } else { i as f64 / 8. }).to_le_bytes());
            }
        }
        b
    }
    #[test]
    fn every_route_certifies_mixed_columns_and_preserves_input_boundary() {
        for route in ROUTES {
            let mut r = Record::new();
            run(route, &mut &input(32)[..], &mut r).unwrap();
            assert!(
                r.columns
                    .iter()
                    .all(|c| c.is_some_and(|c| c.report.accepted))
            );
            assert_eq!(
                r.columns[31]
                    .unwrap()
                    .report
                    .certified_normal_equation_residual,
                0.
            );
            assert!(r.max_admitted <= MAX_PAYLOAD);
        }
        let good = input(2);
        for end in [0, 8, 31, 32, 47, good.len() - 1] {
            assert!(decode(&mut &good[..end]).is_err());
        }
        let mut trailing = good.clone();
        trailing.push(0);
        assert!(decode(&mut &trailing[..]).is_err());
        let mut r = Record::new();
        assert!(run("typo", &mut &good[..], &mut r).is_err());
        assert!(r.columns.iter().all(Option::is_none));
    }
}
