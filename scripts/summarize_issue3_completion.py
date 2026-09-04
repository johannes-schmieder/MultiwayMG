#!/usr/bin/env python3
"""Evaluate and summarize the frozen issue #3 calibration and holdout matrix."""

from __future__ import annotations

import argparse
import csv
import math
from collections import Counter
from pathlib import Path
from statistics import median

SETS = ("calibration", "holdout")
METHODS = (
    "baseline-symmetric-map",
    "oracle",
    "one-shot-pair-neighborhood",
    "bootstrap-initial",
    "bootstrap-final",
)
MINIMUM_MEDIAN_RECOVERY = 0.60
MINIMUM_CASE_FRACTION_AT_TARGET = 0.60
TARGET_RECOVERY = 0.60
MAXIMUM_DEFICIT_VERSUS_ONE_SHOT = 0.10
MATERIAL_CALIBRATION_IMPROVEMENT = 0.08
MINIMUM_MATERIAL_CALIBRATION_CASES = 2
MAXIMUM_TRUE_RESIDUAL = 1.0e-9


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("directory", type=Path)
    parser.add_argument("output", type=Path)
    return parser.parse_args()


def read_tsv(path: Path) -> list[dict[str, str]]:
    with path.open(newline="", encoding="utf-8") as handle:
        rows = list(csv.DictReader(handle, delimiter="\t"))
    if not rows:
        raise SystemExit(f"no rows in {path}")
    return rows


def read_policy(path: Path) -> dict[str, float]:
    rows = read_tsv(path)
    return {row["name"]: float(row["value"]) for row in rows}


def number(row: dict[str, str], key: str) -> float | None:
    value = row[key]
    if value in {"", "NA", "NaN", "nan"}:
        return None
    parsed = float(value)
    if not math.isfinite(parsed):
        return None
    return parsed


def integer(row: dict[str, str], key: str) -> int | None:
    value = row[key]
    return None if value in {"", "NA"} else int(value)


def flag(row: dict[str, str], key: str) -> bool | None:
    value = row[key].lower()
    if value == "true":
        return True
    if value == "false":
        return False
    return None


def fmt(value: float | None, digits: int = 3) -> str:
    return "NA" if value is None else f"{value:.{digits}f}"


def require(condition: bool, message: str, failures: list[str]) -> None:
    if not condition:
        failures.append(message)


def main() -> None:
    args = parse_args()
    matrix_path = args.directory / "issue3-completion-matrix.tsv"
    traces_path = args.directory / "issue3-pcg-traces.tsv"
    policy_path = args.directory / "issue3-policy.tsv"
    rows = read_tsv(matrix_path)
    traces = read_tsv(traces_path)
    policy = read_policy(policy_path)
    by_case: dict[tuple[str, str], dict[str, dict[str, str]]] = {}
    for row in rows:
        key = (row["set"], row["case"])
        by_case.setdefault(key, {})[row["method"]] = row

    failures: list[str] = []
    set_counts = Counter(key[0] for key in by_case)
    require(set_counts["calibration"] == 6, "expected six calibration fixtures", failures)
    require(set_counts["holdout"] == 10, "expected ten holdout fixtures", failures)
    for key, methods in by_case.items():
        missing = set(METHODS).difference(methods)
        require(not missing, f"{key} missing methods {sorted(missing)}", failures)

    dimension_limit = policy["maximum_coarse_dimension_ratio"]
    tuple_reduction_limit = policy["minimum_tuple_reduction"]
    complexity_limit = policy["maximum_two_level_tuple_complexity"]
    diagonal_limit = policy["compatible_diagonal_limit"]
    energy_limit = policy["compatible_energy_limit"]

    final_rows: dict[tuple[str, str], dict[str, str]] = {}
    recoveries: dict[str, list[float]] = {name: [] for name in SETS}
    cases_at_target: dict[str, int] = {name: 0 for name in SETS}
    calibration_material_improvements = 0
    structural_baseline_selections = 0
    witness_round_cases = 0
    split_repair_cases = 0
    maximum_final_residual = 0.0
    maximum_final_complexity = 0.0
    minimum_final_recovery = math.inf
    maximum_final_recovery = -math.inf
    candidate_pairs = 0
    retained_bytes = 0

    for key, methods in sorted(by_case.items()):
        if set(METHODS).difference(methods):
            continue
        final = methods["bootstrap-final"]
        one_shot = methods["one-shot-pair-neighborhood"]
        final_rows[key] = final
        require(flag(final, "accepted") is True, f"{key} final map was not accepted", failures)
        require(
            flag(final, "structural_admissible") is True,
            f"{key} final map was not structurally admissible",
            failures,
        )
        dimension_ratio = number(final, "coarse_dimension_ratio")
        tuple_reduction = number(final, "tuple_reduction")
        complexity = number(final, "two_level_tuple_complexity")
        diagonal_factor = number(final, "compatible_diagonal_factor")
        energy_factor = number(final, "compatible_energy_factor")
        recovery = number(final, "oracle_improvement_recovered")
        one_shot_recovery = number(one_shot, "oracle_improvement_recovered")
        residual = number(final, "pcg_final_relative_residual")
        converged = flag(final, "pcg_converged")
        require(
            dimension_ratio is not None and dimension_ratio <= dimension_limit + 1.0e-12,
            f"{key} dimension ratio exceeded policy",
            failures,
        )
        require(
            tuple_reduction is not None and tuple_reduction + 1.0e-12 >= tuple_reduction_limit,
            f"{key} tuple reduction missed policy",
            failures,
        )
        require(
            complexity is not None and complexity <= complexity_limit + 1.0e-12,
            f"{key} two-level tuple complexity exceeded policy",
            failures,
        )
        require(
            diagonal_factor is not None and diagonal_factor <= diagonal_limit + 1.0e-12,
            f"{key} diagonal compatible factor exceeded policy",
            failures,
        )
        require(
            energy_factor is not None and energy_factor <= energy_limit + 1.0e-12,
            f"{key} energy compatible factor exceeded policy",
            failures,
        )
        require(converged is True, f"{key} traced PCG did not converge", failures)
        require(
            residual is not None and residual <= MAXIMUM_TRUE_RESIDUAL,
            f"{key} true residual exceeded {MAXIMUM_TRUE_RESIDUAL:g}",
            failures,
        )
        require(recovery is not None, f"{key} has no oracle recovery statistic", failures)
        if recovery is not None:
            recoveries[key[0]].append(recovery)
            cases_at_target[key[0]] += int(recovery >= TARGET_RECOVERY)
            minimum_final_recovery = min(minimum_final_recovery, recovery)
            maximum_final_recovery = max(maximum_final_recovery, recovery)
        if recovery is not None and one_shot_recovery is not None:
            require(
                recovery + MAXIMUM_DEFICIT_VERSUS_ONE_SHOT >= one_shot_recovery,
                f"{key} lost more than {MAXIMUM_DEFICIT_VERSUS_ONE_SHOT:.2f} recovery versus one-shot",
                failures,
            )
            if key[0] == "calibration" and recovery - one_shot_recovery >= MATERIAL_CALIBRATION_IMPROVEMENT:
                calibration_material_improvements += 1
        maximum_final_residual = max(maximum_final_residual, residual or 0.0)
        maximum_final_complexity = max(maximum_final_complexity, complexity or 0.0)
        structural_baseline_selections += int(flag(final, "structural_baseline_selected") is True)
        witness_round_cases += int((integer(final, "bootstrap_witnesses") or 0) > 0)
        split_repair_cases += int((integer(final, "repair_splits") or 0) > 0)
        candidate_pairs += integer(final, "candidate_pairs_generated") or 0
        retained_bytes += (integer(final, "retained_test_vector_bytes") or 0) + (
            integer(final, "retained_report_bytes_estimate") or 0
        )

    for set_name in SETS:
        values = recoveries[set_name]
        require(values, f"{set_name} has no final recovery values", failures)
        if values:
            require(
                median(values) >= MINIMUM_MEDIAN_RECOVERY,
                f"{set_name} median recovery below {MINIMUM_MEDIAN_RECOVERY:.2f}",
                failures,
            )
            require(
                cases_at_target[set_name] / len(values) >= MINIMUM_CASE_FRACTION_AT_TARGET,
                f"fewer than {MINIMUM_CASE_FRACTION_AT_TARGET:.0%} of {set_name} cases reached {TARGET_RECOVERY:.0%} recovery",
                failures,
            )
    require(
        calibration_material_improvements >= MINIMUM_MATERIAL_CALIBRATION_CASES,
        f"only {calibration_material_improvements} calibration cases materially improved one-shot",
        failures,
    )
    require(candidate_pairs > 0, "no sparse candidates were generated", failures)
    require(retained_bytes > 0, "retained setup memory was not reported", failures)
    require(witness_round_cases > 0, "no fixture exercised bootstrap witness enrichment", failures)
    require(split_repair_cases > 0, "no fixture exercised monotone split repair", failures)
    require(structural_baseline_selections > 0, "no fixture exercised the protected structural baseline", failures)

    trace_keys = {(row["set"], row["case"], row["method"]) for row in traces}
    for key in by_case:
        for method in ("baseline-symmetric-map", "oracle", "one-shot-pair-neighborhood", "bootstrap-final"):
            require((*key, method) in trace_keys, f"missing trace for {key} {method}", failures)

    lines = [
        "# Issue #3 automatic coarsening, repair, and bootstrap results",
        "",
        "## Verdict",
        "",
    ]
    if failures:
        lines.append("**The frozen issue #3 scientific gate does not pass.**")
    else:
        lines.append("**The frozen issue #3 research gate passes.**")
    lines.extend(
        [
            "",
            "The matrix compares a protected pair-neighborhood baseline, a relaxed-signature",
            "bootstrap matcher, compatible-witness rematching, bounded split repair, and exact",
            "supplied maps on six searched calibration covers and ten fixed unseen-seed holdouts.",
            "Labels are diagnostic only: the authority is recovered two-grid spectral benefit",
            "subject to compatible-relaxation and structural-complexity gates.",
            "",
            "## Frozen policy",
            "",
            f"- Conservative compatible limits: diagonal `{diagonal_limit:.3f}`, energy `{energy_limit:.3f}` per sweep.",
            f"- Maximum coarse-dimension ratio: `{dimension_limit:.3f}`.",
            f"- Minimum unique-tuple reduction: `{tuple_reduction_limit:.3f}`.",
            f"- Maximum two-level tuple complexity: `{complexity_limit:.3f}`.",
            f"- Scientific target: median oracle-improvement recovery at least `{MINIMUM_MEDIAN_RECOVERY:.0%}` separately on calibration and holdout.",
            f"- At least `{MINIMUM_CASE_FRACTION_AT_TARGET:.0%}` of each set must individually recover `{TARGET_RECOVERY:.0%}`.",
            f"- Final recovery may not trail one-shot matching by more than `{MAXIMUM_DEFICIT_VERSUS_ONE_SHOT:.0%}`.",
            "",
            "## Final-map matrix",
            "",
            "| Set | Case | Source | One-shot recovery | Final recovery | Compatible D factor | Coarse dim / tuples | Tuple complexity | PCG iterations | True residual |",
            "|---|---|---|---:|---:|---:|---:|---:|---:|---:|",
        ]
    )
    for key, final in sorted(final_rows.items()):
        one_shot = by_case[key]["one-shot-pair-neighborhood"]
        lines.append(
            "| {set_name} | {case} | {source} | {one_shot} | {final_recovery} | {factor} | {dimension} / {tuples} | {complexity} | {iterations} | {residual} |".format(
                set_name=key[0],
                case=key[1],
                source=final["selected_source"],
                one_shot=fmt(number(one_shot, "oracle_improvement_recovered")),
                final_recovery=fmt(number(final, "oracle_improvement_recovered")),
                factor=fmt(number(final, "compatible_diagonal_factor")),
                dimension=final["coarse_dimension"],
                tuples=final["coarse_tuples"],
                complexity=fmt(number(final, "two_level_tuple_complexity")),
                iterations=final["pcg_iterations"],
                residual=f"{number(final, 'pcg_final_relative_residual'):.2e}",
            )
        )

    lines.extend(["", "## Aggregate gates", ""])
    for set_name in SETS:
        values = recoveries[set_name]
        lines.append(
            f"- `{set_name}`: median recovery `{median(values):.3f}`; "
            f"{cases_at_target[set_name]} of {len(values)} cases reached at least `{TARGET_RECOVERY:.0%}`."
        )
    lines.extend(
        [
            f"- Calibration cases improving one-shot by at least `{MATERIAL_CALIBRATION_IMPROVEMENT:.0%}`: `{calibration_material_improvements}`.",
            f"- Maximum final two-level tuple complexity: `{maximum_final_complexity:.3f}`.",
            f"- Maximum final recomputed true residual: `{maximum_final_residual:.3e}`.",
            f"- Recovery range across final maps: `{minimum_final_recovery:.3f}` to `{maximum_final_recovery:.3f}`.",
            f"- Fixtures using appended witnesses: `{witness_round_cases}`; split repair: `{split_repair_cases}`; protected structural baseline: `{structural_baseline_selections}`.",
            f"- Total generated sparse candidates across final builds: `{candidate_pairs}`.",
            f"- Principal retained test/report bytes across final builds: `{retained_bytes}`.",
            "",
            "## Interpretation",
            "",
            "The gate is intentionally about functional coarse-space quality, not exact label",
            "recovery. Alternative hard partitions can equal or outperform the supplied map.",
            "Compatible relaxation screens for clearly missing modes, but the complete",
            "symmetric-MAP two-grid spectrum and traced original-Gramian PCG solve determine",
            "how much useful oracle benefit was recovered.",
            "",
            "The protected structural baseline prevents the bootstrap portfolio from trading a",
            "materially better conservative contraction factor for a negligible size saving.",
            "Witness enrichment and monotone splitting remain bounded by explicit rounds,",
            "candidate degree, coefficient dimension, tuple contraction, and memory reporting.",
            "",
            "## Limitations",
            "",
            "These are manufactured graph-cover fixtures small enough for dense quotient-space",
            "analysis. Passing the gate supports the automatic-coarsening research direction;",
            "it does not establish production runtime superiority or real-data generality.",
            "Large pair-solver comparisons remain issue #4, prepared allocation-free state is",
            "issue #5, and certified fereg integration remains issue #6.",
            "",
        ]
    )
    if failures:
        lines.extend(["## Failed gates", ""])
        lines.extend(f"- {failure}" for failure in failures)
        lines.append("")

    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text("\n".join(lines), encoding="utf-8")
    if failures:
        raise SystemExit("issue #3 gates failed:\n- " + "\n- ".join(failures))


if __name__ == "__main__":
    main()
