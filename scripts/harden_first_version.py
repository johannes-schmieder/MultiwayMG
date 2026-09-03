"""Apply deterministic one-time source repairs before the first release."""

from pathlib import Path


HIERARCHY = Path("crates/multiway-mg/src/hierarchy.rs")
MARKER = "        Ok(())\n    }\n}\n\n/// Observable dimensions"
INSERTION = """        if self.pre_sweeps != self.post_sweeps {
            return Err(MultiwayError::InvalidOption {
                name: "smoothing_sweeps",
                message: format!(
                    "pre_sweeps ({}) must equal post_sweeps ({}) for a symmetric V-cycle",
                    self.pre_sweeps, self.post_sweeps
                ),
            });
        }
"""


def main() -> None:
    text = HIERARCHY.read_text(encoding="utf-8")
    if INSERTION.strip() in text:
        return
    if text.count(MARKER) != 1:
        raise RuntimeError("hierarchy validation marker was not unique")
    HIERARCHY.write_text(text.replace(MARKER, INSERTION + MARKER), encoding="utf-8")


if __name__ == "__main__":
    main()
