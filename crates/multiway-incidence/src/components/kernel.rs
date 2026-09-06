//! Shared scalar projection arithmetic for ordinary and prepared owners.
use super::{StructuralProjectionScratch, neumaier_add};

pub(super) struct ProjectionData<'a> {
    pub(super) labels: &'a [usize],
    pub(super) factor_sizes: &'a [[usize; 3]],
    pub(super) offsets: [usize; 4],
}
impl ProjectionData<'_> {
    pub(super) fn project(
        &self,
        values: &mut [f64],
        scratch: &mut [StructuralProjectionScratch],
    ) -> f64 {
        #[cfg(feature = "profiling")]
        let _profile_span = crate::profiling::span(crate::profiling::Phase::Projection);

        self.accumulate_factor_sums(values, scratch);

        let mut removed_squared = 0.0;
        for (component, scratch) in scratch.iter_mut().enumerate() {
            let [n1, n2, n3] = self.factor_sizes[component];
            debug_assert!(n1 > 0 && n2 > 0 && n3 > 0);
            let [s1, s2, s3] = scratch.sums;
            let g1 = s1 - s2;
            let g2 = s1 - s3;
            let a11 = (n1 + n2) as f64;
            let a12 = n1 as f64;
            let a22 = (n1 + n3) as f64;
            let determinant = a11.mul_add(a22, -(a12 * a12));
            debug_assert!(determinant > 0.0);
            let alpha = (a22.mul_add(g1, -(a12 * g2))) / determinant;
            let beta = (a11.mul_add(g2, -(a12 * g1))) / determinant;
            let projection = [alpha + beta, -alpha, -beta];
            scratch.projection = projection;
            removed_squared += (n1 as f64).mul_add(
                projection[0] * projection[0],
                (n2 as f64).mul_add(
                    projection[1] * projection[1],
                    n3 as f64 * projection[2] * projection[2],
                ),
            );
        }

        for factor in 0..3 {
            for vertex in self.offsets[factor]..self.offsets[factor + 1] {
                values[vertex] -= scratch[self.labels[vertex]].projection[factor];
            }
        }
        removed_squared.sqrt()
    }
    pub(super) fn defect(
        &self,
        values: &[f64],
        scratch: &mut [StructuralProjectionScratch],
    ) -> f64 {
        self.accumulate_factor_sums(values, scratch);

        let mut maximum: f64 = 0.0;
        for scratch in scratch.iter() {
            let [a, b, c] = scratch.sums;
            maximum = maximum.max((a - b).abs()).max((a - c).abs());
        }
        maximum
    }
    fn accumulate_factor_sums(&self, values: &[f64], scratch: &mut [StructuralProjectionScratch]) {
        scratch.fill(StructuralProjectionScratch::default());
        for factor in 0..3 {
            for vertex in self.offsets[factor]..self.offsets[factor + 1] {
                let component = self.labels[vertex];
                let StructuralProjectionScratch {
                    sums, corrections, ..
                } = &mut scratch[component];
                neumaier_add(&mut sums[factor], &mut corrections[factor], values[vertex]);
            }
        }
        for scratch in scratch.iter_mut() {
            for factor in 0..3 {
                scratch.sums[factor] += scratch.corrections[factor];
            }
        }
    }
}
