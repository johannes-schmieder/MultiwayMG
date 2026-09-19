use multiway_mg::{DenseRangeDecomposition, SpectralAnalysisOptions, ThreeWayProblem,
    WithinApproxCholOptions, WithinApproxCholPreconditioner, Preconditioner, MultiwayError};
struct Both<'a>(&'a ThreeWayProblem, &'a WithinApproxCholPreconditioner);
impl Preconditioner for Both<'_> {
 fn dimension(&self)->usize {self.0.dimension()}
 fn apply(&self,r:&[f64],out:&mut[f64])->Result<(),MultiwayError>{
  let mut p=r.to_vec();self.0.components().project_structural_range(&mut p)?;self.1.apply(&p,out)
 }
}
fn main() -> Result<(),Box<dyn std::error::Error>> {
 println!("case,heterogeneous,action,dimension,rank,nullity,full_symmetry,quotient_symmetry,range_leakage,symmetric,preserves_range,positive_on_range");
 for name in ["tensor","latin","nested","two-components","sparse-connected"] {
  let n=4_u32; let mut tuples=Vec::new();
  for a in 0..n {for b in 0..n {match name {
   "tensor" => for c in 0..n {tuples.push([a,b,c]);},
   "latin" => tuples.push([a,b,(a+b)%n]),
   "nested" => tuples.push([a,b,a]),
   _ => if a/2==b/2 {tuples.push([a,b,a]);}
  }}}
  if name=="sparse-connected" {tuples=vec![[0,0,0],[1,0,0],[1,1,1],[2,1,1],[2,2,2],[3,2,3],[3,3,3]];}
  for heterogeneous in [false,true] {
   let weights:Vec<f64>=(0..tuples.len()).map(|i|if heterogeneous {2.0_f64.powi((i%7) as i32-3)}else{1.0}).collect();
   let problem=ThreeWayProblem::from_observations([n as usize;3],&tuples,&weights)?;
   let opt=SpectralAnalysisOptions::default();let range=DenseRangeDecomposition::from_problem(&problem,opt)?;
   let pre=WithinApproxCholPreconditioner::build(problem.clone(),WithinApproxCholOptions::default())?;
   let both=Both(&problem,&pre);
   for (action,p) in [("existing-output-projection",&pre as &dyn Preconditioner),("both-structural-projections",&both as &dyn Preconditioner)] {
    let d=range.analyze(p,opt)?;
    println!("{name},{heterogeneous},{action},{},{},{},{:.17e},{:.17e},{:.17e},{},{},{}",problem.dimension(),range.rank(),range.nullity(),d.preconditioner_symmetry_defect(),d.quotient_symmetry_defect(),d.range_leakage(),d.numerically_symmetric(),d.preserves_range(),d.positive_definite_on_range());
   }
  }
 }
 Ok(())
}
