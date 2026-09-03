"""Apply the first compatible-relaxation compile repair."""

from pathlib import Path


PATH = Path("crates/multiway-mg/src/compatible.rs")


def main() -> None:
    text = PATH.read_text(encoding="utf-8")
    old = "        let mut maximum = 0.0;\n"
    new = "        let mut maximum = 0.0_f64;\n"
    if text.count(old) != 1:
        raise RuntimeError(f"expected one ambiguous accumulator, found {text.count(old)}")
    PATH.write_text(text.replace(old, new), encoding="utf-8")


if __name__ == "__main__":
    main()
