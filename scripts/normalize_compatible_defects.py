"""Normalize final compatible defects against the initial test-vector norm."""

from pathlib import Path


PATH = Path("crates/multiway-mg/src/compatible.rs")


def main() -> None:
    text = PATH.read_text(encoding="utf-8")
    old_coarse = '''    /// Relative maximum normalized coarse moment.
    ///
    /// A value near zero verifies `P' D x = 0` after complementary projection.
    pub fn relative_coarse_defect(&self, values: &[f64]) -> Result<f64, MultiwayError> {
        self.validate_values(
            "DiagonalAggregationProjector::relative_coarse_defect",
            values,
        )?;
        let norm = weighted_norm(values, self.problem.diagonal());
        let moments = self.coarse_moments(values);
        let maximum = moments
            .iter()
            .zip(&self.aggregate_diagonal)
            .map(|(&moment, &weight)| moment.abs() / weight.sqrt())
            .fold(0.0, f64::max);
        Ok(if norm == 0.0 {
            if maximum == 0.0 { 0.0 } else { f64::INFINITY }
        } else {
            maximum / norm
        })
    }
'''
    new_coarse = '''    /// Relative maximum normalized coarse moment.
    ///
    /// A value near zero verifies `P' D x = 0` after complementary projection.
    pub fn relative_coarse_defect(&self, values: &[f64]) -> Result<f64, MultiwayError> {
        self.validate_values(
            "DiagonalAggregationProjector::relative_coarse_defect",
            values,
        )?;
        let reference_norm = weighted_norm(values, self.problem.diagonal());
        self.coarse_defect_with_reference(values, reference_norm)
    }
'''
    if text.count(old_coarse) != 1:
        raise RuntimeError("coarse defect block was not unique")
    text = text.replace(old_coarse, new_coarse)

    old_structural = '''    /// Relative weighted defect against the two structural factor-shift modes
    /// in every incidence component.
    ///
    /// Because those modes lie in `range(P)`, a successful complementary
    /// projection drives this diagnostic to roundoff.
    pub fn relative_structural_defect(&self, values: &[f64]) -> Result<f64, MultiwayError> {
        self.validate_values(
            "DiagonalAggregationProjector::relative_structural_defect",
            values,
        )?;
        let norm = weighted_norm(values, self.problem.diagonal());
        let offsets = self.problem.topology().offsets();
        let counts = self.problem.topology().level_counts();
        let mut sums = vec![[0.0; 3]; self.problem.components().count()];
        for factor in 0..3 {
            for level in 0..counts[factor] {
                let index = offsets[factor] + level;
                let component = self.problem.components().component_of(factor, level);
                sums[component][factor] += self.problem.diagonal()[index] * values[index];
            }
        }
        let mut maximum = 0.0_f64;
        for (component, [first, second, third]) in sums.into_iter().enumerate() {
            let masses = self.component_factor_diagonal[component];
            maximum = maximum.max((first - second).abs() / (masses[0] + masses[1]).sqrt());
            maximum = maximum.max((first - third).abs() / (masses[0] + masses[2]).sqrt());
        }
        Ok(if norm == 0.0 {
            if maximum == 0.0 { 0.0 } else { f64::INFINITY }
        } else {
            maximum / norm
        })
    }
'''
    new_structural = '''    /// Relative weighted defect against the two structural factor-shift modes
    /// in every incidence component.
    ///
    /// Because those modes lie in `range(P)`, a successful complementary
    /// projection drives this diagnostic to roundoff.
    pub fn relative_structural_defect(&self, values: &[f64]) -> Result<f64, MultiwayError> {
        self.validate_values(
            "DiagonalAggregationProjector::relative_structural_defect",
            values,
        )?;
        let reference_norm = weighted_norm(values, self.problem.diagonal());
        self.structural_defect_with_reference(values, reference_norm)
    }

    fn coarse_defect_with_reference(
        &self,
        values: &[f64],
        reference_norm: f64,
    ) -> Result<f64, MultiwayError> {
        self.validate_values(
            "DiagonalAggregationProjector::coarse_defect_with_reference",
            values,
        )?;
        if !reference_norm.is_finite() || reference_norm < 0.0 {
            return Err(MultiwayError::CompatibleRelaxation {
                message: format!("invalid coarse-defect reference norm {reference_norm}"),
            });
        }
        let moments = self.coarse_moments(values);
        let maximum = moments
            .iter()
            .zip(&self.aggregate_diagonal)
            .map(|(&moment, &weight)| moment.abs() / weight.sqrt())
            .fold(0.0, f64::max);
        Ok(if reference_norm == 0.0 {
            if maximum == 0.0 { 0.0 } else { f64::INFINITY }
        } else {
            maximum / reference_norm
        })
    }

    fn structural_defect_with_reference(
        &self,
        values: &[f64],
        reference_norm: f64,
    ) -> Result<f64, MultiwayError> {
        self.validate_values(
            "DiagonalAggregationProjector::structural_defect_with_reference",
            values,
        )?;
        if !reference_norm.is_finite() || reference_norm < 0.0 {
            return Err(MultiwayError::CompatibleRelaxation {
                message: format!("invalid structural-defect reference norm {reference_norm}"),
            });
        }
        let offsets = self.problem.topology().offsets();
        let counts = self.problem.topology().level_counts();
        let mut sums = vec![[0.0; 3]; self.problem.components().count()];
        for factor in 0..3 {
            for level in 0..counts[factor] {
                let index = offsets[factor] + level;
                let component = self.problem.components().component_of(factor, level);
                sums[component][factor] += self.problem.diagonal()[index] * values[index];
            }
        }
        let mut maximum = 0.0_f64;
        for (component, [first, second, third]) in sums.into_iter().enumerate() {
            let masses = self.component_factor_diagonal[component];
            maximum = maximum.max((first - second).abs() / (masses[0] + masses[1]).sqrt());
            maximum = maximum.max((first - third).abs() / (masses[0] + masses[2]).sqrt());
        }
        Ok(if reference_norm == 0.0 {
            if maximum == 0.0 { 0.0 } else { f64::INFINITY }
        } else {
            maximum / reference_norm
        })
    }
'''
    if text.count(old_structural) != 1:
        raise RuntimeError("structural defect block was not unique")
    text = text.replace(old_structural, new_structural)

    text = text.replace(
        "    /// Final relative `P' D e` defect.\n",
        "    /// Final `P' D e` defect normalized by the initial compatible `D` norm.\n",
    )
    text = text.replace(
        "    /// Final weighted structural-shift defect.\n",
        "    /// Final structural-shift defect normalized by the initial compatible `D` norm.\n",
    )
    text = text.replace(
        "        final_coarse_defect: projector.relative_coarse_defect(&error)?,\n",
        "        final_coarse_defect: projector\n            .coarse_defect_with_reference(&error, initial_diagonal_norm)?,\n",
    )
    text = text.replace(
        "        final_structural_defect: projector.relative_structural_defect(&error)?,\n",
        "        final_structural_defect: projector\n            .structural_defect_with_reference(&error, initial_diagonal_norm)?,\n",
    )
    PATH.write_text(text, encoding="utf-8")


if __name__ == "__main__":
    main()
