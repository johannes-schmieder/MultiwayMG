"""Instrument bootstrap aggregation setup phases without changing decisions."""

from pathlib import Path

PATH = Path("crates/multiway-mg/src/bootstrap.rs")
LIB = Path("crates/multiway-mg/src/lib.rs")


def replace_once(text: str, old: str, new: str, label: str) -> str:
    count = text.count(old)
    if count != 1:
        raise RuntimeError(f"{label}: expected one match, found {count}")
    return text.replace(old, new)


def patch_bootstrap() -> None:
    text = PATH.read_text(encoding="utf-8")
    text = replace_once(
        text,
        "use std::collections::BTreeMap;\n",
        "use std::{collections::BTreeMap, time::{Duration, Instant}};\n",
        "time import",
    )
    marker = "/// Complete deterministic bootstrap aggregation result.\n"
    timing = """/// Phase-separated wall-clock diagnostics for bootstrap construction.
///
/// Timings are descriptive only. They are never consumed by matching,
/// acceptance, repair, or portfolio decisions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BootstrapAggregationBuildTiming {
    setup_test_vectors: Duration,
    candidate_matching: Duration,
    tuple_remapping: Duration,
    compatible_relaxation: Duration,
    witness_enrichment: Duration,
    split_repair: Duration,
    structural_baseline: Duration,
    total: Duration,
}

impl BootstrapAggregationBuildTiming {
    /// Deterministic initial test-vector generation and relaxation.
    #[must_use]
    pub const fn setup_test_vectors(self) -> Duration {
        self.setup_test_vectors
    }

    /// Candidate generation, scoring, pruning, and greedy matching.
    #[must_use]
    pub const fn candidate_matching(self) -> Duration {
        self.candidate_matching
    }

    /// Exact coarse tuple mapping and structural metric construction.
    #[must_use]
    pub const fn tuple_remapping(self) -> Duration {
        self.tuple_remapping
    }

    /// Projected compatible-relaxation screens and explicit decisions.
    #[must_use]
    pub const fn compatible_relaxation(self) -> Duration {
        self.compatible_relaxation
    }

    /// Slow-witness extraction, range filtering, normalization, and retention.
    #[must_use]
    pub const fn witness_enrichment(self) -> Duration {
        self.witness_enrichment
    }

    /// Optional monotone split-repair stage.
    #[must_use]
    pub const fn split_repair(self) -> Duration {
        self.split_repair
    }

    /// Protected pair-neighborhood baseline construction and screening.
    #[must_use]
    pub const fn structural_baseline(self) -> Duration {
        self.structural_baseline
    }

    /// Complete constructor time, including validation and bookkeeping.
    #[must_use]
    pub const fn total(self) -> Duration {
        self.total
    }
}

"""
    text = replace_once(text, marker, timing + marker, "timing struct")

    old_signature = """/// Build and screen one hard factor-respecting aggregation.
pub fn build_bootstrap_aggregation<P: Preconditioner + ?Sized>(
    problem: &ThreeWayProblem,
    screen_smoother: &P,
    options: BootstrapAggregationOptions,
) -> Result<BootstrapAggregationResult, MultiwayError> {
    let options = options.validate()?;
"""
    new_signature = """/// Build and screen one hard factor-respecting aggregation.
pub fn build_bootstrap_aggregation<P: Preconditioner + ?Sized>(
    problem: &ThreeWayProblem,
    screen_smoother: &P,
    options: BootstrapAggregationOptions,
) -> Result<BootstrapAggregationResult, MultiwayError> {
    build_bootstrap_aggregation_with_timing(problem, screen_smoother, options)
        .map(|(result, _timing)| result)
}

/// Build one aggregation and return descriptive phase-separated setup timing.
pub fn build_bootstrap_aggregation_with_timing<P: Preconditioner + ?Sized>(
    problem: &ThreeWayProblem,
    screen_smoother: &P,
    options: BootstrapAggregationOptions,
) -> Result<(BootstrapAggregationResult, BootstrapAggregationBuildTiming), MultiwayError> {
    let total_start = Instant::now();
    let options = options.validate()?;
"""
    text = replace_once(text, old_signature, new_signature, "function wrapper")

    text = replace_once(
        text,
        """    let mut test_vectors = relaxed_range_test_vectors(problem, options)?;
    let initial_vector_count = test_vectors.len();
""",
        """    let setup_test_vectors_start = Instant::now();
    let mut test_vectors = relaxed_range_test_vectors(problem, options)?;
    let setup_test_vectors = setup_test_vectors_start.elapsed();
    let initial_vector_count = test_vectors.len();
    let mut candidate_matching = Duration::ZERO;
    let mut tuple_remapping = Duration::ZERO;
    let mut compatible_relaxation = Duration::ZERO;
    let mut witness_enrichment = Duration::ZERO;
    let mut split_repair_timing = Duration::ZERO;
    let mut structural_baseline_timing = Duration::ZERO;
""",
        "initial timing state",
    )

    text = replace_once(
        text,
        """    for round_index in 0..=options.maximum_bootstrap_witnesses {
        let matching = build_matching(problem, &test_vectors, options)?;
""",
        """    for round_index in 0..=options.maximum_bootstrap_witnesses {
        let matching_start = Instant::now();
        let matching = build_matching(problem, &test_vectors, options)?;
        candidate_matching += matching_start.elapsed();
""",
        "matching timer",
    )

    text = replace_once(
        text,
        """        let structural = structural_metrics(problem, &matching.aggregation)?;
        if let Some(reason) = structural_rejection(problem, structural, options) {
""",
        """        let tuple_remapping_start = Instant::now();
        let structural = structural_metrics(problem, &matching.aggregation)?;
        tuple_remapping += tuple_remapping_start.elapsed();
        if let Some(reason) = structural_rejection(problem, structural, options) {
""",
        "tuple timer",
    )

    text = replace_once(
        text,
        """        let compatible_report = analyze_compatible_relaxation(
            problem,
            &matching.aggregation,
            screen_smoother,
            options.compatible_relaxation,
        )?;
        let compatible_decision =
            evaluate_compatible_relaxation(&compatible_report, options.compatible_criteria)?;
""",
        """        let compatible_start = Instant::now();
        let compatible_report = analyze_compatible_relaxation(
            problem,
            &matching.aggregation,
            screen_smoother,
            options.compatible_relaxation,
        )?;
        let compatible_decision =
            evaluate_compatible_relaxation(&compatible_report, options.compatible_criteria)?;
        compatible_relaxation += compatible_start.elapsed();
""",
        "compatible timer",
    )

    text = replace_once(
        text,
        """        let report = &rounds
            .last()
            .expect("current compatible-relaxation round was just appended")
            .compatible_report;
""",
        """        let witness_start = Instant::now();
        let report = &rounds
            .last()
            .expect("current compatible-relaxation round was just appended")
            .compatible_report;
""",
        "witness start",
    )
    text = replace_once(
        text,
        """        test_vectors.push(witness);
        previous = Some(matching.aggregation);
""",
        """        test_vectors.push(witness);
        witness_enrichment += witness_start.elapsed();
        previous = Some(matching.aggregation);
""",
        "witness end",
    )

    text = replace_once(
        text,
        """        if let Some(repair_options) = options.split_repair {
            let repair = repair_aggregation_by_splitting(
""",
        """        if let Some(repair_options) = options.split_repair {
            let split_repair_start = Instant::now();
            let repair = repair_aggregation_by_splitting(
""",
        "repair start",
    )
    text = replace_once(
        text,
        """            split_repair = Some(repair);
        }
    }

    let structural_baseline = build_pair_neighborhood_aggregation(
""",
        """            split_repair = Some(repair);
            split_repair_timing += split_repair_start.elapsed();
        }
    }

    let structural_baseline_start = Instant::now();
    let structural_baseline = build_pair_neighborhood_aggregation(
""",
        "repair end and baseline start",
    )
    text = replace_once(
        text,
        """        structural_baseline_report = Some(report);
        structural_baseline_decision = Some(decision);
    }

    let retained_test_vector_bytes = test_vectors
""",
        """        structural_baseline_report = Some(report);
        structural_baseline_decision = Some(decision);
    }
    structural_baseline_timing += structural_baseline_start.elapsed();

    let retained_test_vector_bytes = test_vectors
""",
        "baseline end",
    )

    old_return = """    Ok(BootstrapAggregationResult {
        initial_aggregation: initial_aggregation.expect("first matching is always retained"),
        final_aggregation,
        accepted,
        stop_reason,
        rounds,
        split_repair,
        structural_baseline_selected,
        structural_baseline_report,
        structural_baseline_decision,
        work,
    })
}
"""
    new_return = """    let result = BootstrapAggregationResult {
        initial_aggregation: initial_aggregation.expect("first matching is always retained"),
        final_aggregation,
        accepted,
        stop_reason,
        rounds,
        split_repair,
        structural_baseline_selected,
        structural_baseline_report,
        structural_baseline_decision,
        work,
    };
    let timing = BootstrapAggregationBuildTiming {
        setup_test_vectors,
        candidate_matching,
        tuple_remapping,
        compatible_relaxation,
        witness_enrichment,
        split_repair: split_repair_timing,
        structural_baseline: structural_baseline_timing,
        total: total_start.elapsed(),
    };
    Ok((result, timing))
}
"""
    text = replace_once(text, old_return, new_return, "timed return")
    PATH.write_text(text, encoding="utf-8")


def patch_lib() -> None:
    text = LIB.read_text(encoding="utf-8")
    old = """pub use bootstrap::{
    BootstrapAggregationOptions, BootstrapAggregationResult, BootstrapAggregationRound,
    BootstrapAggregationStopReason, BootstrapAggregationWorkReport, BootstrapStructuralMetrics,
    build_bootstrap_aggregation,
};
"""
    new = """pub use bootstrap::{
    BootstrapAggregationBuildTiming, BootstrapAggregationOptions, BootstrapAggregationResult,
    BootstrapAggregationRound, BootstrapAggregationStopReason, BootstrapAggregationWorkReport,
    BootstrapStructuralMetrics, build_bootstrap_aggregation,
    build_bootstrap_aggregation_with_timing,
};
"""
    text = replace_once(text, old, new, "lib exports")
    LIB.write_text(text, encoding="utf-8")


def main() -> None:
    patch_bootstrap()
    patch_lib()


if __name__ == "__main__":
    main()
