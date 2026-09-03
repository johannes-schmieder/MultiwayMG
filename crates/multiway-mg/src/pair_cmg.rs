//! Pairwise graph-Laplacian corrections powered by CMG.

use std::collections::BTreeMap;

use cmg::{CmgOptions, CmgPreconditioner, Components, Laplacian};

use crate::{HierarchyOptions, MultiwayError, Preconditioner, ThreeWayHierarchy, ThreeWayProblem};

/// Options for the three pairwise CMG corrections.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PairCmgOptions {
    /// CMG hierarchy options used for each pair marginal.
    pub cmg: CmgOptions,
    /// Symmetric restriction/prolongation weight for each pair occurrence.
    /// The natural three-way partition-of-unity value is `1/sqrt(2)`.
    pub partition_weight: f64,
}

impl Default for PairCmgOptions {
    fn default() -> Self {
        Self {
            cmg: CmgOptions::default(),
            partition_weight: std::f64::consts::FRAC_1_SQRT_2,
        }
    }
}

impl PairCmgOptions {
    fn validate(self) -> Result<Self, MultiwayError> {
        self.cmg
            .validate()
            .map_err(|error| MultiwayError::Cmg(error.to_string()))?;
        if !self.partition_weight.is_finite() || self.partition_weight <= 0.0 {
            return Err(MultiwayError::InvalidOption {
                name: "partition_weight",
                message: format!("must be finite and positive, got {}", self.partition_weight),
            });
        }
        Ok(self)
    }
}

/// Additive pairwise approximate inverse using one fixed CMG cycle per pair.
#[derive(Debug, Clone)]
pub struct PairCmgPreconditioner {
    problem: ThreeWayProblem,
    pairs: [PairSystem; 3],
    partition_weight: f64,
}

impl PairCmgPreconditioner {
    /// Build worker--firm-style graph corrections for all three factor pairs.
    pub fn build(problem: ThreeWayProblem, options: PairCmgOptions) -> Result<Self, MultiwayError> {
        let options = options.validate()?;
        let pairs = [
            PairSystem::build(&problem, 0, 1, options.cmg)?,
            PairSystem::build(&problem, 0, 2, options.cmg)?,
            PairSystem::build(&problem, 1, 2, options.cmg)?,
        ];
        Ok(Self {
            problem,
            pairs,
            partition_weight: options.partition_weight,
        })
    }

    /// Underlying three-way problem.
    #[must_use]
    pub const fn problem(&self) -> &ThreeWayProblem {
        &self.problem
    }

    /// Number of pair systems.
    #[must_use]
    pub const fn pair_count(&self) -> usize {
        3
    }
}

impl Preconditioner for PairCmgPreconditioner {
    fn dimension(&self) -> usize {
        self.problem.dimension()
    }

    fn apply(&self, rhs: &[f64], out: &mut [f64]) -> Result<(), MultiwayError> {
        let dimension = self.dimension();
        if rhs.len() != dimension {
            return Err(crate::error::dimension(
                "PairCmgPreconditioner::apply rhs",
                dimension,
                rhs.len(),
            ));
        }
        if out.len() != dimension {
            return Err(crate::error::dimension(
                "PairCmgPreconditioner::apply output",
                dimension,
                out.len(),
            ));
        }
        out.fill(0.0);
        for pair in &self.pairs {
            pair.accumulate(rhs, out, self.partition_weight)?;
        }
        self.problem.components().project_structural_range(out)?;
        Ok(())
    }
}

/// Symmetric composition of pair-CMG smoothing and a three-way coarse hierarchy.
#[derive(Debug, Clone)]
pub struct HybridPairVcycle {
    problem: ThreeWayProblem,
    pair: PairCmgPreconditioner,
    hierarchy: ThreeWayHierarchy,
}

impl HybridPairVcycle {
    /// Build a hybrid preconditioner on one fixed weighted problem.
    pub fn build(
        problem: ThreeWayProblem,
        hierarchy_options: HierarchyOptions,
        pair_options: PairCmgOptions,
    ) -> Result<Self, MultiwayError> {
        let pair = PairCmgPreconditioner::build(problem.clone(), pair_options)?;
        let hierarchy = ThreeWayHierarchy::build(problem.clone(), hierarchy_options)?;
        Ok(Self {
            problem,
            pair,
            hierarchy,
        })
    }

    /// Pairwise smoother.
    #[must_use]
    pub const fn pair_preconditioner(&self) -> &PairCmgPreconditioner {
        &self.pair
    }

    /// Three-way hierarchy supplying the global coarse correction.
    #[must_use]
    pub const fn hierarchy(&self) -> &ThreeWayHierarchy {
        &self.hierarchy
    }
}

impl Preconditioner for HybridPairVcycle {
    fn dimension(&self) -> usize {
        self.problem.dimension()
    }

    fn apply(&self, rhs: &[f64], out: &mut [f64]) -> Result<(), MultiwayError> {
        let dimension = self.dimension();
        if rhs.len() != dimension {
            return Err(crate::error::dimension(
                "HybridPairVcycle::apply rhs",
                dimension,
                rhs.len(),
            ));
        }
        if out.len() != dimension {
            return Err(crate::error::dimension(
                "HybridPairVcycle::apply output",
                dimension,
                out.len(),
            ));
        }

        let mut compatible_rhs = rhs.to_vec();
        self.problem
            .components()
            .project_structural_range(&mut compatible_rhs)?;

        self.pair.apply(&compatible_rhs, out)?;
        let mut residual = self.problem.residual(&compatible_rhs, out)?;
        self.problem
            .components()
            .project_structural_range(&mut residual)?;

        let mut coarse = vec![0.0; dimension];
        self.hierarchy
            .apply_coarse_correction(&residual, &mut coarse)?;
        add_assign(out, &coarse);

        residual = self.problem.residual(&compatible_rhs, out)?;
        self.problem
            .components()
            .project_structural_range(&mut residual)?;
        let mut post = vec![0.0; dimension];
        self.pair.apply(&residual, &mut post)?;
        add_assign(out, &post);
        self.problem.components().project_structural_range(out)?;
        Ok(())
    }
}

#[derive(Debug, Clone)]
struct PairSystem {
    first: usize,
    second: usize,
    first_count: usize,
    second_count: usize,
    first_offset: usize,
    second_offset: usize,
    components: Components,
    preconditioner: CmgPreconditioner,
}

impl PairSystem {
    fn build(
        problem: &ThreeWayProblem,
        first: usize,
        second: usize,
        options: CmgOptions,
    ) -> Result<Self, MultiwayError> {
        let counts = problem.topology().level_counts();
        let offsets = problem.topology().offsets();
        let first_count = counts[first];
        let second_count = counts[second];
        let mut marginal: BTreeMap<(u32, u32), f64> = BTreeMap::new();
        for (&tuple, &weight) in problem.topology().tuples().iter().zip(problem.weights()) {
            *marginal.entry((tuple[first], tuple[second])).or_insert(0.0) += weight;
        }
        let edges = marginal
            .into_iter()
            .map(|((left, right), weight)| (left as usize, first_count + right as usize, weight));
        let graph = Laplacian::from_edges(first_count + second_count, edges)
            .map_err(|error| MultiwayError::Cmg(error.to_string()))?;
        let components = Components::from_laplacian(&graph);
        let preconditioner = CmgPreconditioner::build(&graph, options)
            .map_err(|error| MultiwayError::Cmg(error.to_string()))?;
        Ok(Self {
            first,
            second,
            first_count,
            second_count,
            first_offset: offsets[first],
            second_offset: offsets[second],
            components,
            preconditioner,
        })
    }

    fn accumulate(
        &self,
        rhs: &[f64],
        out: &mut [f64],
        partition_weight: f64,
    ) -> Result<(), MultiwayError> {
        let local_dimension = self.first_count + self.second_count;
        let mut local_rhs = vec![0.0; local_dimension];
        for level in 0..self.first_count {
            local_rhs[level] = partition_weight * rhs[self.first_offset + level];
        }
        for level in 0..self.second_count {
            local_rhs[self.first_count + level] =
                -partition_weight * rhs[self.second_offset + level];
        }
        self.components
            .center_in_place(&mut local_rhs)
            .map_err(|error| MultiwayError::Cmg(error.to_string()))?;

        let mut local_solution = vec![0.0; local_dimension];
        let mut workspace = self.preconditioner.workspace();
        self.preconditioner
            .apply_compatible_into(&local_rhs, &mut local_solution, &mut workspace)
            .map_err(|error| MultiwayError::Cmg(error.to_string()))?;

        for level in 0..self.first_count {
            out[self.first_offset + level] += partition_weight * local_solution[level];
        }
        for level in 0..self.second_count {
            out[self.second_offset + level] -=
                partition_weight * local_solution[self.first_count + level];
        }
        debug_assert!(self.first < self.second);
        Ok(())
    }
}

fn add_assign(destination: &mut [f64], source: &[f64]) {
    for (left, &right) in destination.iter_mut().zip(source) {
        *left += right;
    }
}
