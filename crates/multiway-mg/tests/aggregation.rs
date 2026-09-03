//! Aggregation tests outside the crate-internal manufactured clone family.

use multiway_mg::{
    AffinityAggregationOptions, PairNeighborhoodAggregationOptions, ThreeWayProblem,
    build_affinity_aggregation, build_pair_neighborhood_aggregation,
};

#[test]
fn pair_neighborhood_fallback_coarsens_a_latin_square() {
    let problem = latin_square(8);
    let exact = build_affinity_aggregation(&problem, AffinityAggregationOptions::default())
        .expect("exact-context aggregation succeeds");
    assert_eq!(exact.coarse_counts(), [8, 8, 8]);

    let neighborhood = build_pair_neighborhood_aggregation(
        &problem,
        PairNeighborhoodAggregationOptions::default(),
    )
    .expect("pair-neighborhood aggregation succeeds");
    assert_eq!(neighborhood.coarse_counts(), [4, 4, 4]);
    let coarse = neighborhood.coarsen(&problem).expect("coarsening succeeds");
    assert!(coarse.dimension() < problem.dimension());
    assert!(coarse.tuple_count() < problem.tuple_count());
}

#[test]
fn pair_neighborhood_matching_never_crosses_components() {
    let mut tuples = latin_square_tuples(4, 0);
    tuples.extend(latin_square_tuples(4, 4));
    let weights: Vec<f64> = (0..tuples.len())
        .map(|index| 1.0 + (index % 7) as f64 / 10.0)
        .collect();
    let problem = ThreeWayProblem::from_observations([8, 8, 8], &tuples, &weights)
        .expect("disconnected problem is valid");
    let aggregation = build_pair_neighborhood_aggregation(
        &problem,
        PairNeighborhoodAggregationOptions::default(),
    )
    .expect("pair-neighborhood aggregation succeeds");

    for factor in 0..3 {
        for left in 0..8 {
            for right in (left + 1)..8 {
                if aggregation.parents(factor)[left] == aggregation.parents(factor)[right] {
                    assert_eq!(
                        problem.components().component_of(factor, left),
                        problem.components().component_of(factor, right)
                    );
                }
            }
        }
    }
}

fn latin_square(levels: u32) -> ThreeWayProblem {
    let tuples = latin_square_tuples(levels, 0);
    let weights: Vec<f64> = tuples
        .iter()
        .enumerate()
        .map(|(index, _)| 0.75 + (index % 11) as f64 / 10.0)
        .collect();
    ThreeWayProblem::from_observations([levels as usize; 3], &tuples, &weights)
        .expect("Latin-square problem is valid")
}

fn latin_square_tuples(levels: u32, offset: u32) -> Vec<[u32; 3]> {
    let mut tuples = Vec::new();
    for first in 0..levels {
        for second in 0..levels {
            tuples.push([
                first + offset,
                second + offset,
                (first + second) % levels + offset,
            ]);
        }
    }
    tuples
}
