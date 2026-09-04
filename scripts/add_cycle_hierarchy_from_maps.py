"""Add the validated supplied-map constructor needed by issue 3 holdouts."""

from pathlib import Path


PATH = Path("crates/multiway-mg/src/cycle_hierarchy.rs")
MARKER = """impl CycleScreenedMapHierarchy {
    fn from_plan(plan: &CycleScreenedHierarchyPlan) -> Result<Self, MultiwayError> {
        let smoothers = plan.problems[..plan.aggregations.len()]
            .iter()
            .cloned()
            .map(SymmetricMapPreconditioner::new)
            .collect();
        let terminal = DensePseudoinverse::from_problem(
            plan.terminal_problem(),
            plan.terminal_relative_tolerance,
        )?;
        Ok(Self {
            problems: plan.problems.clone(),
            aggregations: plan.aggregations.clone(),
            smoothers,
            terminal,
        })
    }
"""
REPLACEMENT = """impl CycleScreenedMapHierarchy {
    /// Build a fixed symmetric-MAP hierarchy from caller-supplied hard maps.
    ///
    /// Every map is validated against the current level, must preserve exact
    /// incidence components, and must strictly reduce coefficient dimension.
    /// This constructor is useful for oracle references and independently
    /// generated map sequences; automatic production-shaped construction should
    /// normally use [`CycleScreenedHierarchyPlan`].
    pub fn from_maps(
        finest: ThreeWayProblem,
        aggregations: Vec<FactorAggregation>,
        terminal_relative_tolerance: f64,
    ) -> Result<Self, MultiwayError> {
        if !terminal_relative_tolerance.is_finite() || terminal_relative_tolerance <= 0.0 {
            return Err(invalid(
                \"cycle_hierarchy_terminal_relative_tolerance\",
                format!(
                    \"must be finite and positive, got {terminal_relative_tolerance}\"
                ),
            ));
        }

        let mut problems = Vec::with_capacity(aggregations.len() + 1);
        problems.push(finest);
        for (level, aggregation) in aggregations.iter().enumerate() {
            let current = problems
                .last()
                .expect(\"a supplied hierarchy always retains its finest problem\");
            if aggregation.fine_counts() != current.topology().level_counts() {
                return Err(MultiwayError::InvalidSuppliedAggregation { level });
            }
            crate::DiagonalAggregationProjector::new(
                current.clone(),
                aggregation.clone(),
            )?;
            let coarse = aggregation.coarsen(current)?;
            if coarse.dimension() >= current.dimension() {
                return Err(MultiwayError::InvalidAggregation {
                    message: format!(
                        \"supplied hierarchy level {level} does not strictly reduce dimension: fine {}, coarse {}\",
                        current.dimension(),
                        coarse.dimension(),
                    ),
                });
            }
            problems.push(coarse);
        }

        let smoothers = problems[..aggregations.len()]
            .iter()
            .cloned()
            .map(SymmetricMapPreconditioner::new)
            .collect();
        let terminal_problem = problems
            .last()
            .expect(\"a supplied hierarchy always has a terminal problem\");
        let terminal = DensePseudoinverse::from_problem(
            terminal_problem,
            terminal_relative_tolerance,
        )?;
        Ok(Self {
            problems,
            aggregations,
            smoothers,
            terminal,
        })
    }

    fn from_plan(plan: &CycleScreenedHierarchyPlan) -> Result<Self, MultiwayError> {
        Self::from_maps(
            plan.finest_problem().clone(),
            plan.aggregations.clone(),
            plan.terminal_relative_tolerance,
        )
    }
"""


def main() -> None:
    text = PATH.read_text(encoding="utf-8")
    if "pub fn from_maps(" in text:
        return
    if text.count(MARKER) != 1:
        raise RuntimeError("CycleScreenedMapHierarchy insertion marker was not unique")
    PATH.write_text(text.replace(MARKER, REPLACEMENT), encoding="utf-8")


if __name__ == "__main__":
    main()
