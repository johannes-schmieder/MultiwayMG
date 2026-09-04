"""Insert the predeclared issue #3 portfolio holdout fixture set."""

from pathlib import Path

PATH = Path("crates/multiway-mg/examples/support/issue3_fixtures.rs")
MARKER = "\nfn build_fixtures(specifications: &[FixtureSpec]) -> Result<Vec<Issue3Fixture>, DynError> {\n"
INSERTION = r'''
/// Second frozen unseen-seed holdout for the two-stage acceptance portfolio.
///
/// These seeds and all acceptance thresholds were committed in
/// `benchmarks/policies/issue3-portfolio-holdout.tsv` before this set was
/// evaluated. The earlier 512--521 holdout remains development evidence and is
/// not overwritten.
pub fn portfolio_holdout_fixtures() -> Result<Vec<Issue3Fixture>, DynError> {
    build_fixtures(&[
        FixtureSpec {
            set: "portfolio-holdout",
            family: "dominant-pair-weak-third",
            requested_seed: 600,
        },
        FixtureSpec {
            set: "portfolio-holdout",
            family: "dominant-pair-weak-third",
            requested_seed: 601,
        },
        FixtureSpec {
            set: "portfolio-holdout",
            family: "nearly-nested",
            requested_seed: 602,
        },
        FixtureSpec {
            set: "portfolio-holdout",
            family: "nearly-nested",
            requested_seed: 603,
        },
        FixtureSpec {
            set: "portfolio-holdout",
            family: "weak-chain",
            requested_seed: 604,
        },
        FixtureSpec {
            set: "portfolio-holdout",
            family: "planted-communities",
            requested_seed: 605,
        },
        FixtureSpec {
            set: "portfolio-holdout",
            family: "hub-power-law",
            requested_seed: 606,
        },
        FixtureSpec {
            set: "portfolio-holdout",
            family: "weight-dynamic-range",
            requested_seed: 607,
        },
        FixtureSpec {
            set: "portfolio-holdout",
            family: "latin-square",
            requested_seed: 608,
        },
        FixtureSpec {
            set: "portfolio-holdout",
            family: "tensor-grid",
            requested_seed: 609,
        },
    ])
}
'''


def main() -> None:
    text = PATH.read_text(encoding="utf-8")
    if "pub fn portfolio_holdout_fixtures" in text:
        return
    if text.count(MARKER) != 1:
        raise RuntimeError("fixture insertion marker was not unique")
    PATH.write_text(text.replace(MARKER, INSERTION + MARKER), encoding="utf-8")


if __name__ == "__main__":
    main()
