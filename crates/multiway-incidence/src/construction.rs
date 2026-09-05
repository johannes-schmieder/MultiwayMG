//! Checked, fallible array reservation at explicit construction boundaries.

use crate::IncidenceError;

pub(crate) fn array_bytes<T>(count: usize) -> Result<usize, IncidenceError> {
    count
        .checked_mul(core::mem::size_of::<T>())
        .filter(|&bytes| bytes <= isize::MAX as usize)
        .ok_or(IncidenceError::DimensionOverflow {
            context: "topology construction array",
        })
}

pub(crate) fn sum_bytes(parts: &[usize]) -> Result<usize, IncidenceError> {
    parts.iter().try_fold(0usize, |sum, &bytes| {
        sum.checked_add(bytes)
            .ok_or(IncidenceError::DimensionOverflow {
                context: "topology construction payload",
            })
    })
}

pub(crate) fn reserve<T, F>(
    count: usize,
    context: &'static str,
    before_reservation: &mut F,
) -> Result<Vec<T>, IncidenceError>
where
    F: FnMut(&'static str) -> Result<(), IncidenceError>,
{
    array_bytes::<T>(count)?;
    let mut values = Vec::new();
    if count != 0 {
        before_reservation(context)?;
        values
            .try_reserve_exact(count)
            .map_err(|_| IncidenceError::TopologyAllocation { context })?;
    }
    Ok(values)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn impossible_arrays_reject_before_reservation() {
        assert!(array_bytes::<f64>(usize::MAX).is_err());
        assert!(array_bytes::<u8>(isize::MAX as usize + 1).is_err());
        assert!(sum_bytes(&[usize::MAX, 1]).is_err());
        assert!(reserve::<f64, _>(usize::MAX, "test", &mut |_| panic!("not admitted")).is_err());
    }
}
