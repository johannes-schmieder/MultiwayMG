"""Separate witness-learned and protected structural maps before cycle screening."""

from pathlib import Path


BOOTSTRAP = Path("crates/multiway-mg/src/bootstrap.rs")
PORTFOLIO = Path("crates/multiway-mg/src/cycle_portfolio.rs")
TESTS = Path("crates/multiway-mg/tests/bootstrap.rs")


def patch_bootstrap() -> None:
    text = BOOTSTRAP.read_text(encoding="utf-8")
    text = text.replace(
        "pub struct BootstrapAggregationResult {\n    initial_aggregation: FactorAggregation,\n    final_aggregation: FactorAggregation,\n",
        "pub struct BootstrapAggregationResult {\n    initial_aggregation: FactorAggregation,\n    learned_aggregation: FactorAggregation,\n    final_aggregation: FactorAggregation,\n",
        1,
    )
    getter_marker = '''    pub const fn initial_aggregation(&self) -> &FactorAggregation {
        &self.initial_aggregation
    }

    /// Final aggregation returned by bootstrap and optional split repair.
'''
    getter_replacement = '''    pub const fn initial_aggregation(&self) -> &FactorAggregation {
        &self.initial_aggregation
    }

    /// Witness-learned map after bootstrap and optional split repair, before
    /// protected structural-baseline arbitration.
    #[must_use]
    pub const fn learned_aggregation(&self) -> &FactorAggregation {
        &self.learned_aggregation
    }

    /// Final aggregation returned after protected structural-baseline arbitration.
'''
    if getter_marker not in text:
        raise RuntimeError("bootstrap getter marker was not found")
    text = text.replace(getter_marker, getter_replacement, 1)
    baseline_marker = "    let structural_baseline_start = Instant::now();\n"
    if text.count(baseline_marker) != 1:
        raise RuntimeError("structural baseline marker was not unique")
    text = text.replace(
        baseline_marker,
        "    let learned_aggregation = final_aggregation.clone();\n\n"
        + baseline_marker,
        1,
    )
    result_marker = '''    let result = BootstrapAggregationResult {
        initial_aggregation: initial_aggregation.expect("first matching is always retained"),
        final_aggregation,
'''
    result_replacement = '''    let result = BootstrapAggregationResult {
        initial_aggregation: initial_aggregation.expect("first matching is always retained"),
        learned_aggregation,
        final_aggregation,
'''
    if result_marker not in text:
        raise RuntimeError("bootstrap result marker was not found")
    text = text.replace(result_marker, result_replacement, 1)
    BOOTSTRAP.write_text(text, encoding="utf-8")


def patch_portfolio() -> None:
    text = PORTFOLIO.read_text(encoding="utf-8")
    text = text.replace(
        "    /// Final map returned by bootstrap and optional monotone repair.\n    BootstrapFinal,\n",
        "    /// Witness-learned map before protected structural-baseline arbitration.\n    BootstrapFinal,\n",
        1,
    )
    old = '''    let mut candidates = vec![(
        CyclePortfolioCandidateSource::BootstrapFinal,
        primary.final_aggregation().clone(),
    )];
    if structural_baseline != candidates[0].1 {
        candidates.push((
            CyclePortfolioCandidateSource::StructuralBaseline,
            structural_baseline,
        ));
    }
'''
    new = '''    let learned_aggregation = primary.learned_aggregation().clone();
    let candidates = if learned_aggregation == structural_baseline {
        vec![(
            CyclePortfolioCandidateSource::StructuralBaseline,
            structural_baseline,
        )]
    } else {
        vec![
            (
                CyclePortfolioCandidateSource::BootstrapFinal,
                learned_aggregation,
            ),
            (
                CyclePortfolioCandidateSource::StructuralBaseline,
                structural_baseline,
            ),
        ]
    };
'''
    if old not in text:
        raise RuntimeError("cycle portfolio candidate block was not found")
    text = text.replace(old, new, 1)
    PORTFOLIO.write_text(text, encoding="utf-8")


def patch_tests() -> None:
    text = TESTS.read_text(encoding="utf-8")
    marker = '''#[test]
fn structural_dimension_budget_rejects_before_compatible_acceptance() {
'''
    test = '''#[test]
fn learned_map_is_retained_before_structural_baseline_arbitration() {
    let (problem, _) = refined_weak_chain(8, 2, 0.01, true);
    let smoother = DiagonalPreconditioner::new(&problem, 0.5).expect("smoother succeeds");
    let mut options = bootstrap_options();
    options.minimum_combined_affinity = 1.0;
    options.maximum_bootstrap_witnesses = 0;
    options.split_repair = None;
    options.minimum_tuple_reduction = 0.0;
    let result = build_bootstrap_aggregation(&problem, &smoother, options)
        .expect("baseline arbitration succeeds");

    assert!(result.structural_baseline_selected());
    assert_ne!(result.learned_aggregation(), result.final_aggregation());
    assert_eq!(
        result.learned_aggregation(),
        result.initial_aggregation(),
        "zero witness budget retains the initial learned matching",
    );
}

'''
    if marker not in text:
        raise RuntimeError("bootstrap test insertion marker was not found")
    text = text.replace(marker, test + marker, 1)
    TESTS.write_text(text, encoding="utf-8")


def main() -> None:
    patch_bootstrap()
    patch_portfolio()
    patch_tests()


if __name__ == "__main__":
    main()
