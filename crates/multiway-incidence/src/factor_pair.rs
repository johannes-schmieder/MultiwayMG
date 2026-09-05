//! Weight-independent factor-pair identifiers.

/// One of the three bipartite factor pairs in a three-way problem.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum FactorPair {
    /// Factors one and two.
    OneTwo,
    /// Factors one and three.
    OneThree,
    /// Factors two and three.
    TwoThree,
}

impl FactorPair {
    /// Canonical list of all factor pairs.
    pub const ALL: [Self; 3] = [Self::OneTwo, Self::OneThree, Self::TwoThree];

    /// Zero-based factor indices.
    #[must_use]
    pub const fn factors(self) -> (usize, usize) {
        match self {
            Self::OneTwo => (0, 1),
            Self::OneThree => (0, 2),
            Self::TwoThree => (1, 2),
        }
    }

    /// Stable human-readable label.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::OneTwo => "1-2",
            Self::OneThree => "1-3",
            Self::TwoThree => "2-3",
        }
    }
}
