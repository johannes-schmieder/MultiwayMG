//! Exact arithmetic over the Mersenne prime field and rational reconstruction.

/// The Mersenne prime `2^61 - 1`; products fit in `u128`.
pub(super) const PRIME: u64 = (1 << 61) - 1;

/// Reconstruction bound `floor(sqrt((PRIME - 1) / 2))` for numerators and denominators.
pub(super) const RATIONAL_BOUND: i128 = (1 << 30) - 1;

#[inline]
pub(super) fn mul(left: u64, right: u64) -> u64 {
    ((u128::from(left) * u128::from(right)) % u128::from(PRIME)) as u64
}

#[inline]
pub(super) fn add(left: u64, right: u64) -> u64 {
    let sum = left + right;
    if sum >= PRIME { sum - PRIME } else { sum }
}

#[inline]
pub(super) fn sub(left: u64, right: u64) -> u64 {
    if left >= right {
        left - right
    } else {
        left + (PRIME - right)
    }
}

#[inline]
pub(super) fn neg(value: u64) -> u64 {
    if value == 0 { 0 } else { PRIME - value }
}

/// Multiplicative inverse of a nonzero residue by Fermat's little theorem.
pub(super) fn inverse(value: u64) -> u64 {
    debug_assert!(value != 0 && value < PRIME);
    let mut result = 1;
    let mut base = value;
    let mut exponent = PRIME - 2;
    while exponent != 0 {
        if exponent & 1 == 1 {
            result = mul(result, base);
        }
        base = mul(base, base);
        exponent >>= 1;
    }
    result
}

/// Smallest-denominator rational `numerator / denominator` congruent to `value`.
///
/// Returns `None` unless both parts are within [`RATIONAL_BOUND`] and coprime.
/// A wrong reconstruction is possible when the true rational is larger; callers
/// must verify the reconstructed integers exactly against the original matrix.
pub(super) fn reconstruct(value: u64) -> Option<(i64, i64)> {
    debug_assert!(value < PRIME);
    let (mut r0, mut r1) = (i128::from(PRIME), i128::from(value));
    let (mut t0, mut t1) = (0_i128, 1_i128);
    while r1 > RATIONAL_BOUND {
        let quotient = r0 / r1;
        (r0, r1) = (r1, r0 - quotient * r1);
        (t0, t1) = (t1, t0 - quotient * t1);
    }
    if t1 == 0 || t1.abs() > RATIONAL_BOUND || gcd(r1.unsigned_abs(), t1.unsigned_abs()) != 1 {
        return None;
    }
    let (numerator, denominator) = if t1 < 0 { (-r1, -t1) } else { (r1, t1) };
    Some((
        i64::try_from(numerator).ok()?,
        i64::try_from(denominator).ok()?,
    ))
}

pub(super) fn gcd(mut left: u128, mut right: u128) -> u128 {
    while right != 0 {
        (left, right) = (right, left % right);
    }
    left
}

/// Residue of a signed integer.
#[cfg(test)]
pub(super) fn residue(value: i64) -> u64 {
    let modulus = i128::from(PRIME);
    (i128::from(value).rem_euclid(modulus)) as u64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn field_identities_hold_at_boundaries() {
        for value in [1, 2, 3, 12_345, PRIME - 2, PRIME - 1, 1 << 60] {
            assert_eq!(mul(value, inverse(value)), 1);
            assert_eq!(sub(value, value), 0);
            assert_eq!(add(value, neg(value)), 0);
            assert_eq!(sub(0, value), neg(value));
            assert_eq!(sub(neg(value), PRIME - value), 0);
        }
        assert_eq!(neg(0), 0);
        assert_eq!(mul(PRIME - 1, PRIME - 1), 1);
    }

    #[test]
    fn reconstruction_recovers_bounded_rationals_and_rejects_large_ones() {
        let bound = RATIONAL_BOUND as i64;
        for (numerator, denominator) in [
            (0, 1),
            (1, 1),
            (-1, 1),
            (3, 7),
            (-5, 2),
            (bound, 1),
            (-bound, bound - 1),
            (1, bound),
        ] {
            let value = mul(residue(numerator), inverse(residue(denominator)));
            assert_eq!(reconstruct(value), Some((numerator, denominator)));
        }
        // A residue far from every small rational is refused rather than guessed.
        let large = mul(residue(bound + 2), inverse(residue(bound + 7)));
        assert_ne!(reconstruct(large), Some((bound + 2, bound + 7)));
    }
}
