#!/usr/bin/env python3
"""Finalize issue #3 documentation after both frozen gates pass."""

from __future__ import annotations

from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]


def replace_section(path: Path, start: str, end: str, replacement: str) -> None:
    text = path.read_text(encoding="utf-8")
    begin = text.find(start)
    if begin < 0:
        raise RuntimeError(f"{path}: missing start marker {start!r}")
    finish = text.find(end, begin)
    if finish < 0:
        raise RuntimeError(f"{path}: missing end marker {end!r}")
    path.write_text(text[:begin] + replacement + text[finish:], encoding="utf-8")


def update_readme() -> None:
    replacement = """## Current step

The first research MVP, the oracle two-grid/multilevel gate in issue #2, and
the automatic coarse-space gate in issue #3 are complete.

Issue #3 produced an important negative result before its final design. A
frozen smoother-only policy rejected supplied oracle maps whose complete MAP
two-grid cycles were nearly exact. That result is preserved rather than
retuned. The final automatic hierarchy therefore separates three authorities:

1. hard factor, component, dimension, and tuple-complexity gates;
2. conservative compatible relaxation for signatures, slow witnesses, and
   bounded split repair; and
3. a deterministic matrix-free probe of the **actual complete cycle** for final
   numerical acceptance.

The accepted candidate portfolio contains only the bootstrap/repair map and the
protected pair-neighborhood map. It cannot invent a new map during screening or
bypass structural budgets. Accepted maps are selected by compactness first and
then by measured complete-cycle quality. The same rule is applied recursively;
a rejected level terminates the hierarchy fail-closed.

Two separately frozen unseen-seed studies now cover one-level four-sheet
hypergraph covers and two- through three-level recursive covers. Both require
byte-identical repeated output, dense quotient-space calibration of the
matrix-free probe, traced original-Gramian PCG residuals, automatic-to-oracle
recovery, and explicit cumulative complexity bounds. See
[`docs/ISSUE3_FINAL_RESULTS.md`](docs/ISSUE3_FINAL_RESULTS.md),
[`docs/ISSUE3_CYCLE_HOLDOUT_RESULTS.md`](docs/ISSUE3_CYCLE_HOLDOUT_RESULTS.md),
and
[`docs/ISSUE3_RECURSIVE_HOLDOUT_RESULTS.md`](docs/ISSUE3_RECURSIVE_HOLDOUT_RESULTS.md).

The next two numerical priorities are complementary:

- [issue #4](https://github.com/johannes-schmieder/MultiwayMG/issues/4)
  determines whether CMG, symmetric MAP, or the existing approximate-Cholesky
  local solver is best on large identical pair domains; and
- [issue #5](https://github.com/johannes-schmieder/MultiwayMG/issues/5)
  converts the selected hierarchy into prepared, allocation-free, repeated-RHS
  and changing-weight state.

A private, certified fereg experiment remains issue #6 and stays downstream of
those production-engineering gates.

"""
    replace_section(
        ROOT / "README.md",
        "## Current step\n",
        "## Current implementation and evidence\n",
        replacement,
    )


def update_roadmap() -> None:
    replacement = """## Milestone 3 — adaptive coarse spaces

Tracked by issue #3. The research milestone is complete.

- [x] Deterministic relaxed test vectors and sparse signature candidates.
- [x] Diagonal-energy compatible projection derived and dense-reference tested.
- [x] Compatible-relaxation histories, defects, factor diagnostics, and explicit
      acceptance decisions.
- [x] Slow compatible witnesses retained for deterministic enrichment.
- [x] Bounded witness-driven rematching and monotone aggregate splitting.
- [x] Protected pair-neighborhood baseline preventing destructive rematching.
- [x] Hard factor/component, coarse-dimension, tuple-reduction, and cumulative
      hierarchy-complexity gates.
- [x] Frozen negative evidence showing that smoother-only compatible relaxation
      is not a valid final cycle authority.
- [x] Matrix-free complete-cycle energy probe calibrated against dense quotient
      spectra and retaining complete-cycle slow witnesses.
- [x] Deterministic complete-cycle candidate portfolio with fail-closed build
      errors and compactness-first selection.
- [x] Recursive complete-cycle-screened hierarchy planning and symmetric-MAP
      V-cycle construction.
- [x] Observation-order determinism, extra-nullity, budget rejection, and
      byte-identical repeated evidence.
- [x] Frozen one-level and recursive automatic-to-oracle holdout gates.

The remaining interpolation question—energy-corrected or richer interpolation—
is deferred unless realistic data show that hard one-parent maps cannot recover
sufficient oracle benefit.

"""
    replace_section(
        ROOT / "docs/ROADMAP.md",
        "## Milestone 3 — adaptive coarse spaces\n",
        "## Milestone 4 — pair solver and production engineering\n",
        replacement,
    )


def update_changelog() -> None:
    path = ROOT / "CHANGELOG.md"
    text = path.read_text(encoding="utf-8")
    marker = "- Dense complete-range decomposition for small singular three-way Gramians,\n"
    addition = """- Bounded deterministic bootstrap aggregation from relaxed signatures,
  pair-neighborhood candidates, retained compatible witnesses, and monotone
  split repair.
- Protected structural-baseline selection and exact structural admission gates.
- Preserved negative holdout demonstrating that smoother-only compatible
  relaxation can reject excellent complete cycles.
- Matrix-free complete-cycle `G`-energy power probes calibrated against dense
  quotient spectra, with retained slow witnesses and exact work reports.
- Compactness-first complete-cycle candidate screening with fail-closed cycle
  construction errors.
- Recursive cycle-screened hierarchy planning and symmetric-MAP V-cycle
  construction under cumulative dimension and tuple budgets.
- Frozen one-level and recursive unseen graph-cover holdouts, byte-identical
  reruns, traced true PCG residuals, automatic-to-oracle recovery gates, and
  machine-readable evidence.
"""
    if addition not in text:
        if marker not in text:
            raise RuntimeError("CHANGELOG insertion marker missing")
        text = text.replace(marker, addition + marker, 1)
    path.write_text(text, encoding="utf-8")


def write_final_results() -> None:
    one_level = (ROOT / "docs/ISSUE3_CYCLE_HOLDOUT_RESULTS.md").read_text(
        encoding="utf-8"
    )
    recursive = (ROOT / "docs/ISSUE3_RECURSIVE_HOLDOUT_RESULTS.md").read_text(
        encoding="utf-8"
    )
    negative = (ROOT / "docs/ISSUE3_PORTFOLIO_V1_NEGATIVE.md").read_text(
        encoding="utf-8"
    )
    text = """# Issue #3 final automatic-coarsening results

## Verdict

Issue #3 is complete within its declared research scope. MultiwayMG now has a
deterministic automatic hard-coarsening pipeline that:

- builds sparse candidates without all-pairs factor comparisons;
- uses compatible relaxation to expose missed slow modes;
- performs bounded bootstrap rematching and monotone split repair;
- preserves a protected structural baseline;
- enforces non-negotiable factor, component, dimension, and tuple budgets;
- evaluates the actual fixed complete cycle before accepting a map; and
- applies the same rule recursively, terminating fail-closed when no admitted
  level exists.

The scientific process included a frozen negative holdout. It showed that
smoother-only compatible-relaxation thresholds could reject supplied oracle
maps despite excellent complete-cycle spectra. The final architecture was
therefore evaluated under new, predeclared unseen seeds rather than by retuning
the failed evidence.

Issue #3 establishes automatic-coarsening feasibility on synthetic graph-cover
families. It does not establish production runtime superiority on real data.
Pair-solver selection, allocation-free state, changing weights, and fereg's
observation-space certificate remain issues #4, #5, and #6.

## Evidence index

- `benchmarks/policies/issue3-portfolio-holdout.tsv` — frozen failed v1 policy;
- `benchmarks/policies/issue3-cycle-portfolio-v2.tsv` — final one-level policy;
- `benchmarks/policies/issue3-recursive-cycle-v1.tsv` — recursive policy;
- `benchmarks/results/2026-09-04/issue3-cycle-holdout.tsv`;
- `benchmarks/results/2026-09-04/issue3-cycle-traces.tsv`;
- `benchmarks/results/2026-09-04/issue3-recursive-holdout.tsv`;
- `benchmarks/results/2026-09-04/issue3-recursive-traces.tsv`;
- corresponding SHA-256 manifests and generated reports.

---

"""
    text += negative + "\n\n---\n\n" + one_level + "\n\n---\n\n" + recursive
    (ROOT / "docs/ISSUE3_FINAL_RESULTS.md").write_text(text, encoding="utf-8")


def main() -> None:
    update_readme()
    update_roadmap()
    update_changelog()
    write_final_results()


if __name__ == "__main__":
    main()
