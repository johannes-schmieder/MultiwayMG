# Issue #3 final review checklist

Before merge, the issue #3 branch must satisfy all of the following:

- final README, roadmap, changelog, results, policy, and ADR are present;
- every frozen positive and negative matrix, trace, status file, and checksum is
  retained;
- learned maps and protected structural maps remain distinct for fair
  comparison;
- the automatic research decision is the bounded pair-neighborhood map plus
  fail-closed complete-cycle screening;
- bootstrap and witness-repair APIs remain explicitly experimental;
- only permanent read-only CI remains under `.github/workflows`;
- Rust 1.85 formatting, strict Clippy, all-feature tests, minimal-feature tests,
  warning-free rustdoc, and the deterministic completion gate pass;
- no frozen policy, seed, or threshold is changed after observing its result.
