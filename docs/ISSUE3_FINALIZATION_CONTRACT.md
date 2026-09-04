# Issue #3 finalization contract

Before the branch is proposed for merge, the one-time finalizer must:

1. promote the staged README, roadmap, and changelog;
2. preserve every frozen policy, result matrix, true-residual trace, checksum,
   gate status, and scientific report;
3. remove obsolete one-time orchestration and patch helpers only;
4. pass Rust 1.85 formatting, strict Clippy, all-feature and minimal-feature
   tests, and warning-free rustdoc;
5. rerun the deterministic issue #3 completion gate without changing any
   frozen policy or threshold; and
6. leave only the permanent read-only CI workflow under `.github/workflows`.
