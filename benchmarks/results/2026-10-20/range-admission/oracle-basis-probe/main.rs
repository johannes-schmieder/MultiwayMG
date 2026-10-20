use multiway_incidence::{IncidenceKernelCertificate, IncidenceKernelLimits, PreparedThreeWayTopology};
use multiway_mg::{Preconditioner, ThreeWayProblem, WithinApproxCholOptions, WithinApproxCholPreconditioner};
use std::io::Read;
fn main() {
    let mut d = Vec::new(); std::io::stdin().read_to_end(&mut d).unwrap();
    let u = |o: usize| u32::from_le_bytes(d[o..o+4].try_into().unwrap()) as usize;
    let counts = [u(8), u(12), u(16)]; let e = u64::from_le_bytes(d[20..28].try_into().unwrap()) as usize;
    let mut t = Vec::new(); for i in 0..e { let o = 32 + 12*i; t.push([u(o) as u32, u(o+4) as u32, u(o+8) as u32]); }
    let wo = 32 + 12*e; let w: Vec<f64> = (0..e).map(|i| f64::from_le_bytes(d[wo+8*i..wo+8*i+8].try_into().unwrap())).collect();
    let owner = PreparedThreeWayTopology::try_from_collapsed(counts, &t).unwrap();
    let lim = IncidenceKernelLimits { maximum_payload_bytes: 1 << 30 };
    let p = IncidenceKernelCertificate::try_certify(&owner, lim).unwrap().try_into_projector(&owner, lim, 1e-12).unwrap();
    let problem = ThreeWayProblem::from_observations(counts, &t, &w).unwrap();
    let within = WithinApproxCholPreconditioner::build(problem.clone(), WithinApproxCholOptions::default()).unwrap();
    let n = problem.dimension(); let mut worst: f64 = 0.0; let mut worst2: f64 = 0.0;
    let norm = |v: &[f64]| v.iter().map(|x| x*x).sum::<f64>().sqrt();
    for j in 0..n { let mut x = vec![0.0; n]; x[j] = 1.0; p.project(&mut x).unwrap();
        let mut y = vec![0.0; n]; within.apply(&x, &mut y).unwrap();
        let mut r = y.clone(); p.project(&mut r).unwrap(); let nul: Vec<f64> = y.iter().zip(&r).map(|(a,b)| a-b).collect();
        worst = worst.max(norm(&nul)/norm(&r));
        let mut r2 = r.clone(); p.project(&mut r2).unwrap(); let d2: Vec<f64> = r.iter().zip(&r2).map(|(a,b)| a-b).collect();
        worst2 = worst2.max(norm(&d2)/norm(&r)); }
    let unit = ThreeWayProblem::from_observations(counts, &t, &vec![1.0; e]).unwrap();
    let range = multiway_mg::DenseRangeDecomposition::from_problem(&unit, multiway_mg::SpectralAnalysisOptions{maximum_dimension:1024, ..Default::default()}).unwrap();
    let q = range.basis(); let mut basis_leak: f64 = 0.0;
    for c in 0..q.ncols() { let col: Vec<f64> = (0..n).map(|i| q[(i,c)]).collect(); let mut pc = col.clone(); p.project(&mut pc).unwrap(); basis_leak = basis_leak.max(norm(&col.iter().zip(&pc).map(|(a,b)| a-b).collect::<Vec<_>>())); }
    println!("oracle range basis kernel component max {basis_leak:.3e}; eigen threshold {:.3e}", range.threshold());
    println!("max null/range of raw within output {worst:.3e}; residual null after one projection {worst2:.3e}");
}
