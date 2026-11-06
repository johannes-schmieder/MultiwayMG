# CMG consolidation qualification

The candidate uses unmodified CMG `a4c927f49eb7a5391a63963d2c4c651a51cf1af5`. The fixed six-block comparison passed every cell with all scientific checks satisfied. No timings were excluded or rerun. `receipt.json` binds the actual candidate source and the original frozen plan; `raw.json` and `summary.json` retain every observation and decision.

All forty fixture/RHS-count cells pass. Independent submitted-operator certificates pass for every measured solve. Rust 1.85 workspace formatting, Clippy, all/minimal-feature tests and documentation pass. GitHub workflow 42299623122 passed all 15 jobs; allocation workflow 42299623126 passed on macOS, Linux and Windows. The existing unrelated within/Schwarz dependency updates remain unchanged.

This evidence-only addition does not change runtime or build inputs. It does not publish a release or establish CMG main as frozen.
