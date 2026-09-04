"""Expose retained compatible witnesses and make repair reuse the measured report."""

from pathlib import Path

COMPATIBLE = Path("crates/multiway-mg/src/compatible.rs")
REPAIR = Path("crates/multiway-mg/src/repair.rs")


def replace_once(text: str, old: str, new: str, label: str) -> str:
    count = text.count(old)
    if count != 1:
        raise RuntimeError(f"{label}: expected one match, found {count}")
    return text.replace(old, new)


def patch_compatible() -> None:
    text = COMPATIBLE.read_text(encoding="utf-8")
    text = replace_once(
        text,
        "    energy_contraction: Option<f64>,\n}",
        "    energy_contraction: Option<f64>,\n    final_error: Vec<f64>,\n}",
        "vector field",
    )
    text = replace_once(
        text,
        "    pub const fn energy_contraction(&self) -> Option<f64> {\n        self.energy_contraction\n    }\n}",
        """    pub const fn energy_contraction(&self) -> Option<f64> {
        self.energy_contraction
    }

    /// Final projected compatible error after all smoothing sweeps.
    ///
    /// Retaining this witness lets bounded bootstrap and repair code attribute
    /// the measured slow mode without rerunning a separate relaxation path.
    #[must_use]
    pub fn final_error(&self) -> &[f64] {
        &self.final_error
    }

    /// Principal retained-memory estimate for owned vector storage.
    #[must_use]
    pub fn retained_bytes_estimate(&self) -> usize {
        core::mem::size_of::<Self>()
            .saturating_add(
                self.diagonal_norm_history
                    .capacity()
                    .saturating_mul(core::mem::size_of::<f64>()),
            )
            .saturating_add(
                self.energy_norm_history
                    .capacity()
                    .saturating_mul(core::mem::size_of::<f64>()),
            )
            .saturating_add(
                self.coarse_drift_norm_history
                    .capacity()
                    .saturating_mul(core::mem::size_of::<f64>()),
            )
            .saturating_add(
                self.final_error
                    .capacity()
                    .saturating_mul(core::mem::size_of::<f64>()),
            )
    }
}
""",
        "vector accessors",
    )
    text = replace_once(
        text,
        "    pub fn vectors(&self) -> &[CompatibleRelaxationVectorReport] {\n        &self.vectors\n    }\n}",
        """    pub fn vectors(&self) -> &[CompatibleRelaxationVectorReport] {
        &self.vectors
    }

    /// Index of the test vector with the largest final diagonal contraction.
    /// Ties are resolved by the lowest deterministic test-vector index.
    #[must_use]
    pub fn slowest_vector_index(&self) -> Option<usize> {
        self.vectors
            .iter()
            .enumerate()
            .max_by(|(left_index, left), (right_index, right)| {
                left.diagonal_contraction()
                    .total_cmp(&right.diagonal_contraction())
                    .then_with(|| right_index.cmp(left_index))
            })
            .map(|(index, _)| index)
    }

    /// Principal retained-memory estimate for all owned diagnostic vectors.
    #[must_use]
    pub fn retained_bytes_estimate(&self) -> usize {
        core::mem::size_of::<Self>()
            .saturating_add(
                self.vectors
                    .capacity()
                    .saturating_mul(core::mem::size_of::<CompatibleRelaxationVectorReport>()),
            )
            .saturating_add(
                self.vectors
                    .iter()
                    .map(CompatibleRelaxationVectorReport::retained_bytes_estimate)
                    .sum::<usize>(),
            )
    }
}
""",
        "report accessors",
    )
    text = replace_once(
        text,
        "        energy_contraction,\n    })",
        "        energy_contraction,\n        final_error: error,\n    })",
        "final error construction",
    )
    COMPATIBLE.write_text(text, encoding="utf-8")


def patch_repair() -> None:
    text = REPAIR.read_text(encoding="utf-8")
    old = """        let witness = slowest_compatible_witness(problem, &current, smoother, options.relaxation)?;
        let Some(split) = choose_split(
            problem,
            &current,
            &witness,
            options.minimum_split_score_fraction,
        ) else {
"""
    new = """        let witness_index = report.slowest_vector_index().ok_or_else(|| {
            MultiwayError::CompatibleRelaxation {
                message: "compatible-relaxation report contained no witnesses".to_owned(),
            }
        })?;
        let witness_report = &report.vectors()[witness_index];
        let witness = SlowWitness {
            index: witness_index,
            values: witness_report.final_error().to_vec(),
            diagonal_contraction: witness_report.diagonal_contraction(),
        };
        let Some(split) = choose_split(
            problem,
            &current,
            &witness,
            options.minimum_split_score_fraction,
        ) else {
"""
    text = replace_once(text, old, new, "reuse witness")
    start = text.index("fn slowest_compatible_witness")
    end = text.index("fn choose_split", start)
    text = text[:start] + text[end:]
    for function in ["fn fill_deterministic", "fn splitmix64", "fn scale_in_place", "fn ensure_finite"]:
        if function in text:
            start = text.index(function)
            next_positions = [
                text.find(candidate, start + 1)
                for candidate in [
                    "fn fill_deterministic",
                    "fn splitmix64",
                    "fn scale_in_place",
                    "fn ensure_finite",
                ]
                if text.find(candidate, start + 1) != -1
            ]
            end = min(next_positions) if next_positions else len(text)
            text = text[:start] + text[end:]
    REPAIR.write_text(text.rstrip() + "\n", encoding="utf-8")


def main() -> None:
    patch_compatible()
    patch_repair()


if __name__ == "__main__":
    main()
