"""Apply strict-Clippy fixes to the observed-seed smoother fallback example."""

from pathlib import Path


EXAMPLE = Path("crates/multiway-mg/examples/issue3_smoother_fallback_development.rs")
FIXTURES = Path("crates/multiway-mg/examples/support/issue3_cycle_fixtures.rs")


def main() -> None:
    text = EXAMPLE.read_text(encoding="utf-8")
    text = text.replace("    path::{Path, PathBuf},\n", "    path::PathBuf,\n")
    text = text.replace(
        "    SymmetricMap(SymmetricTwoGridPreconditioner<SymmetricMapPreconditioner>),\n"
        "    PairCmg(SymmetricTwoGridPreconditioner<PairCmgPreconditioner>),\n",
        "    SymmetricMap(Box<SymmetricTwoGridPreconditioner<SymmetricMapPreconditioner>>),\n"
        "    PairCmg(Box<SymmetricTwoGridPreconditioner<PairCmgPreconditioner>>),\n",
    )
    text = text.replace(
        ".map(DevelopmentCycle::SymmetricMap),",
        ".map(Box::new)\n        .map(DevelopmentCycle::SymmetricMap),",
    )
    text = text.replace(
        ".map(DevelopmentCycle::PairCmg),",
        ".map(Box::new)\n        .map(DevelopmentCycle::PairCmg),",
    )
    EXAMPLE.write_text(text, encoding="utf-8")

    fixtures = FIXTURES.read_text(encoding="utf-8")
    allow = "#![allow(dead_code)]\n\n"
    if not fixtures.startswith(allow):
        FIXTURES.write_text(allow + fixtures, encoding="utf-8")


if __name__ == "__main__":
    main()
