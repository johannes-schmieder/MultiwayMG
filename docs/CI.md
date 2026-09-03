# Continuous integration

The authoritative first-version checks run on Rust 1.85 and include formatting,
Clippy with warnings denied, all-feature and minimal-feature tests, rustdoc, and
release-mode feasibility probes. Temporary source-repair jobs are removed before
the pull request is marked ready for review.
