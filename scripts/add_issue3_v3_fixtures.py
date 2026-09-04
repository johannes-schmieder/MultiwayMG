"""Add the predeclared issue #3 v3 seeds to the shared cover generator."""

from pathlib import Path


PATH = Path("crates/multiway-mg/examples/support/issue3_cycle_fixtures.rs")
OLD = '''/// Build the predeclared seeds 700--709 without conditioning on solver results.
pub fn cycle_holdout_fixtures() -> Result<Vec<CycleHoldoutFixture>, DynError> {
    let specifications = [
        ("cover-latin", 700_u64),
        ("cover-latin", 701),
        ("cover-weak-chain", 702),
        ("cover-weak-chain", 703),
        ("cover-nearly-nested", 704),
        ("cover-nearly-nested", 705),
        ("cover-dominant-pair", 706),
        ("cover-dominant-pair", 707),
        ("cover-communities", 708),
        ("cover-communities", 709),
    ];
    specifications
        .into_iter()
        .map(|(family, requested_seed)| build_fixture(family, requested_seed))
        .collect()
}

fn build_fixture(
    family: &'static str,
    requested_seed: u64,
) -> Result<CycleHoldoutFixture, DynError> {
'''
NEW = '''/// Build the predeclared v2 seeds 700--709 without conditioning on solver results.
pub fn cycle_holdout_fixtures() -> Result<Vec<CycleHoldoutFixture>, DynError> {
    build_fixtures(
        "cycle-holdout-v2",
        &[
            ("cover-latin", 700_u64),
            ("cover-latin", 701),
            ("cover-weak-chain", 702),
            ("cover-weak-chain", 703),
            ("cover-nearly-nested", 704),
            ("cover-nearly-nested", 705),
            ("cover-dominant-pair", 706),
            ("cover-dominant-pair", 707),
            ("cover-communities", 708),
            ("cover-communities", 709),
        ],
    )
}

/// Build the predeclared v3 seeds 900--909 without conditioning on solver results.
pub fn cycle_holdout_v3_fixtures() -> Result<Vec<CycleHoldoutFixture>, DynError> {
    build_fixtures(
        "cycle-holdout-v3",
        &[
            ("cover-latin", 900_u64),
            ("cover-latin", 901),
            ("cover-weak-chain", 902),
            ("cover-weak-chain", 903),
            ("cover-nearly-nested", 904),
            ("cover-nearly-nested", 905),
            ("cover-dominant-pair", 906),
            ("cover-dominant-pair", 907),
            ("cover-communities", 908),
            ("cover-communities", 909),
        ],
    )
}

fn build_fixtures(
    set: &'static str,
    specifications: &[(&'static str, u64)],
) -> Result<Vec<CycleHoldoutFixture>, DynError> {
    specifications
        .iter()
        .copied()
        .map(|(family, requested_seed)| build_fixture(set, family, requested_seed))
        .collect()
}

fn build_fixture(
    set: &'static str,
    family: &'static str,
    requested_seed: u64,
) -> Result<CycleHoldoutFixture, DynError> {
'''


def main() -> None:
    text = PATH.read_text(encoding="utf-8")
    if "pub fn cycle_holdout_v3_fixtures" in text:
        return
    if text.count(OLD) != 1:
        raise RuntimeError("v2 fixture block was not unique")
    text = text.replace(OLD, NEW)
    text = text.replace('                    set: "cycle-holdout-v2",\n', '                    set,\n', 1)
    PATH.write_text(text, encoding="utf-8")


if __name__ == "__main__":
    main()
