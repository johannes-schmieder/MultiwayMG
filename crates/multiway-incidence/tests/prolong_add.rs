//! Fused transfer compared to independently materialized factor-local injection.
use multiway_incidence::FactorAggregation;

#[test]
fn fused_transfer_preserves_two_pass_bits_and_shape_failures() {
    let maps = [
        FactorAggregation::identity([3, 2, 4]).unwrap(),
        FactorAggregation::new(
            [5, 3, 4],
            [vec![2, 0, 2, 1, 0], vec![1, 0, 1], vec![0, 0, 0, 0]],
        )
        .unwrap(),
        FactorAggregation::consecutive_halving([7, 4, 3]).unwrap(),
    ];
    let values = [
        0.0,
        -0.0,
        f64::from_bits(1),
        -f64::from_bits(1),
        f64::MIN_POSITIVE,
        -f64::MIN_POSITIVE,
        1.0,
        -1.0,
        f64::MAX,
        -f64::MAX,
    ];
    for map in maps {
        let n: usize = map.fine_counts().iter().sum();
        let m: usize = map.coarse_counts().iter().sum();
        for shift in 0..values.len() {
            let coarse: Vec<_> = (0..m).map(|i| values[(i + shift) % values.len()]).collect();
            for start in 0..values.len() {
                let initial: Vec<_> = (0..n).map(|i| values[(i + start) % values.len()]).collect();
                let mut temporary = Vec::with_capacity(n);
                let mut offset = 0;
                for factor in 0..3 {
                    for &parent in map.parents(factor) {
                        temporary.push(coarse[offset + parent as usize]);
                    }
                    offset += map.coarse_counts()[factor];
                }
                let expected: Vec<_> = initial
                    .iter()
                    .zip(&temporary)
                    .map(|(&a, &b)| (a + b).to_bits())
                    .collect();
                let mut actual = initial.clone();
                map.prolong_add(&coarse, &mut actual).unwrap();
                assert_eq!(
                    actual.iter().map(|x| x.to_bits()).collect::<Vec<_>>(),
                    expected
                );
                // The independent overwrite transfer remains unchanged as well.
                map.prolong(&coarse, &mut actual).unwrap();
                assert_eq!(
                    actual.iter().map(|x| x.to_bits()).collect::<Vec<_>>(),
                    temporary.iter().map(|x| x.to_bits()).collect::<Vec<_>>()
                );
                for short_coarse in [false, true] {
                    let mut unchanged = initial.clone();
                    let result = if short_coarse {
                        map.prolong_add(&coarse[..m - 1], &mut unchanged)
                    } else {
                        map.prolong_add(&coarse, &mut unchanged[..n - 1])
                    };
                    assert!(result.is_err());
                    assert_eq!(
                        unchanged.iter().map(|x| x.to_bits()).collect::<Vec<_>>(),
                        initial.iter().map(|x| x.to_bits()).collect::<Vec<_>>()
                    );
                }
            }
        }
    }
}
