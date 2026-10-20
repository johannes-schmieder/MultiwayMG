//! Checked `within` setup admission on one bounded development input.
//!
//! Reads one `MG3AUT1` input from stdin. Builds the frozen `within` comparator
//! unchecked and through `within`'s checked serial setup, reports the checked
//! setup peak and retained payload, checks the inclusive limit at the peak and one
//! byte below it, and compares the two `within` preconditioners' serial actions
//! bit for bit. Diagnostic only; no routing or default.
use multiway_mg::{
    MultiwayError, Preconditioner, ThreeWayProblem, WithinApproxCholOptions,
    WithinApproxCholPreconditioner,
};
use std::{
    error::Error,
    io::{self, Read},
    time::Instant,
};
use within::{Design, Effect, PreconditionerConfig, SetupLimit, Solver};
type Result<T = ()> = std::result::Result<T, Box<dyn Error>>;

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
    // Targets are irrelevant to setup; consume and discard them.
    let mut rest = Vec::new();
    r.read_to_end(&mut rest)?;
    if rest.len() != 8 * e * rhs {
        return Err("target payload length mismatch".into());
    }
    Ok((counts, tuples, weights))
}

fn limit(maximum_payload_bytes: usize) -> SetupLimit {
    SetupLimit {
        maximum_payload_bytes,
    }
}

/// Deterministic serial action of a `within` preconditioner.
fn serial_action(preconditioner: &within::Preconditioner, x: &[f64]) -> Result<Vec<f64>> {
    let bytes = preconditioner.serial_workspace_required_payload_bytes()?;
    let mut workspace = preconditioner.try_serial_workspace(bytes)?;
    let mut y = vec![0.0; x.len()];
    workspace.apply(x, &mut y)?;
    Ok(y)
}

fn main() -> Result {
    let (counts, tuples, weights) = decode(&mut io::stdin().lock())?;
    let problem = ThreeWayProblem::from_observations(counts, &tuples, &weights)?;
    let options = WithinApproxCholOptions::default();
    println!("dimension\t{}", problem.dimension());
    println!("tuples\t{}", tuples.len());

    // An untimed build first, so Rayon's global pool start is not charged to either.
    drop(WithinApproxCholPreconditioner::build(
        problem.clone(),
        options.clone(),
    )?);
    let started = Instant::now();
    let unchecked = WithinApproxCholPreconditioner::build(problem.clone(), options.clone())?;
    println!("unchecked_build_ns\t{}", started.elapsed().as_nanos());
    let started = Instant::now();
    let checked = WithinApproxCholPreconditioner::build_checked(
        problem.clone(),
        options.clone(),
        SetupLimit::UNLIMITED,
    )?;
    println!("checked_build_ns\t{}", started.elapsed().as_nanos());
    let memory = checked.memory_report();
    let peak = memory
        .within_setup_peak_bytes()
        .ok_or("checked build reports a peak")?;
    println!("setup_peak_bytes\t{peak}");
    println!(
        "retained_bytes\t{}",
        memory
            .within_retained_bytes()
            .ok_or("checked build reports retained payload")?
    );
    println!(
        "checked_factor_bytes\t{}",
        memory
            .within_local_factor_bytes()
            .ok_or("factor overflow")?
    );
    println!(
        "unchecked_factor_bytes\t{}",
        unchecked
            .memory_report()
            .within_local_factor_bytes()
            .ok_or("factor overflow")?
    );

    let at_peak = WithinApproxCholPreconditioner::build_checked(
        problem.clone(),
        options.clone(),
        limit(peak),
    )
    .is_ok();
    println!("admitted_at_peak\t{at_peak}");
    let refused = match WithinApproxCholPreconditioner::build_checked(
        problem.clone(),
        options.clone(),
        limit(peak - 1),
    ) {
        Err(MultiwayError::WithinSetupBudgetExceeded {
            required,
            limit: refused,
            ..
        }) => refused == peak - 1 && required > refused && required <= peak,
        _ => false,
    };
    println!("refused_one_under\t{refused}");

    // The `within` preconditioners both ways, compared through serial actions.
    let mut levels: [Vec<u32>; 3] = std::array::from_fn(|_| Vec::with_capacity(tuples.len()));
    for tuple in problem.topology().tuples() {
        for factor in 0..3 {
            levels[factor].push(tuple[factor]);
        }
    }
    let effects = || {
        levels
            .iter()
            .map(|codes| Effect::new(codes, true, std::iter::empty::<&[f64]>()))
            .collect::<std::result::Result<Vec<_>, _>>()
    };
    let solver = Solver::new(
        effects()?,
        Some(problem.weights().to_vec()),
        PreconditionerConfig::Additive {
            local_solver: options.local_solver.clone(),
            reduction: options.reduction,
        },
    )?;
    let reference = solver.preconditioner().ok_or("solver route builds none")?;
    let design = Design::new(effects()?)?;
    let (direct, _) = within::Preconditioner::build_checked(
        &design,
        Some(problem.weights()),
        &options.local_solver,
        options.reduction,
        SetupLimit::UNLIMITED,
    )?;
    let direct = direct.ok_or("checked route builds none")?;
    let mut identical = true;
    let mut difference = 0.0_f64;
    for phase in [0.41, 1.3, 2.9] {
        let x: Vec<f64> = (0..problem.dimension())
            .map(|i| ((i as f64 + 1.0) * phase).sin())
            .collect();
        let expected = serial_action(reference, &x)?;
        let actual = serial_action(&direct, &x)?;
        identical &= expected
            .iter()
            .zip(&actual)
            .all(|(a, b)| a.to_bits() == b.to_bits());
        let (mut a, mut b) = (vec![0.0; x.len()], vec![0.0; x.len()]);
        unchecked.apply(&x, &mut a)?;
        checked.apply(&x, &mut b)?;
        let scale = a.iter().fold(1.0_f64, |m, v| m.max(v.abs()));
        let gap = a
            .iter()
            .zip(&b)
            .fold(0.0_f64, |m, (u, v)| m.max((u - v).abs()));
        difference = difference.max(gap / scale);
    }
    println!("serial_actions_identical\t{identical}");
    println!("wrapper_action_difference\t{difference:.17e}");
    Ok(())
}
