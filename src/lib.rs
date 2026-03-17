//! BCsort: A fast, statistical, in-place ternary distribution sort.

use rayon::prelude::*;

/// Configuration for tuning BCsort performance.
#[derive(Clone, Debug)]
pub struct BcsortConfig {
    /// The minimum slice length required to spawn parallel tasks.
    /// Can be tuned for 128-core servers vs 2-core IoT devices.
    pub parallel_threshold: usize,
}

impl Default for BcsortConfig {
    fn default() -> Self {
        Self {
            parallel_threshold: 10_000,
        }
    }
}

/// Extension trait providing in-place BCsort operations for slices.
pub trait Bcsort {
    /// Sorts the slice in-place using the default configuration.
    ///
    /// # Example
    /// ```
    /// use bcsort::Bcsort;
    /// let mut data = vec![5.0, 1.0, 4.0, 2.0];
    /// data.bcsort();
    /// assert_eq!(data, vec![1.0, 2.0, 4.0, 5.0]);
    /// ```
    fn bcsort(&mut self);

    /// Sorts the slice in-place using a custom configuration.
    fn bcsort_with_config(&mut self, config: &BcsortConfig);
}

macro_rules! impl_bcsort_for_float {
    ($t:ident) => {
        const _: () = {
            impl Bcsort for [$t] {
                fn bcsort(&mut self) {
                    self.bcsort_with_config(&BcsortConfig::default());
                }

                fn bcsort_with_config(&mut self, config: &BcsortConfig) {
                    let len = self.len();
                    if len <= 1 {
                        return;
                    }

                    // 1. NaN/Inf Quarantine (single-threaded pass).
                    let mut valid_len = len;
                    let mut i = 0;
                    while i < valid_len {
                        if self[i].is_nan() || self[i].is_infinite() {
                            valid_len -= 1;
                            self.swap(i, valid_len);
                        } else {
                            i += 1;
                        }
                    }

                    if valid_len <= 1 {
                        return;
                    }
                    let valid_arr = &mut self[0..valid_len];

                    // 2. ROOT MACRO SCALE: parallel reduction for initial stats.
                    let (min, max, sum) = valid_arr
                        .par_iter()
                        .copied()
                        .fold(
                            || ($t::INFINITY, $t::NEG_INFINITY, 0.0),
                            |acc, x| (acc.0.min(x), acc.1.max(x), acc.2 + x),
                        )
                        .reduce(
                            || ($t::INFINITY, $t::NEG_INFINITY, 0.0),
                            |a, b| (a.0.min(b.0), a.1.max(b.1), a.2 + b.2),
                        );

                    bcsort_recursive_par(valid_arr, min, max, sum, 0, config.parallel_threshold);
                }
            }

            /// Minimum array length at which adaptive pivot sampling is worth attempting.
            /// Below this, the 9-element sample is too coarse to improve over arithmetic
            /// pivots, and the sampling overhead is not amortised.
            const ADAPTIVE_PIVOT_MIN_LEN: usize = 512;

            /// Compute two pivot thresholds for a ternary split.
            ///
            /// Fast path (bad_splits < 3): arithmetic mean-derived pivots, zero
            /// memory reads beyond what the caller already holds.
            ///
            /// Adaptive path (bad_splits >= 3, len >= ADAPTIVE_PIVOT_MIN_LEN): 9
            /// pseudo-random samples sorted via a branchless network; s[3]/s[6] used
            /// as ~33rd/~66th percentile pivots. Samples use an LCG stride rather than
            /// evenly spaced indices to resist adversarial patterns.
            #[inline]
            fn compute_pivots(arr: &[$t], min: $t, max: $t, sum: $t, bad_splits: u32) -> ($t, $t) {
                let mean = sum / (arr.len() as $t);

                if bad_splits < 3 || arr.len() < ADAPTIVE_PIVOT_MIN_LEN {
                    return ((min + mean) / 2.0, (mean + max) / 2.0);
                }

                // Adaptive path: sample 9 elements at pseudo-random positions.
                let len = arr.len();
                let stride = (len / 9).max(1);
                let mut idx = len / 7;
                let mut s = [0.0 as $t; 9];
                for i in 0..9 {
                    s[i] = arr[idx % len];
                    idx = idx.wrapping_mul(6_364_136_223).wrapping_add(stride) % len;
                }

                // Branchless sorting network for 9 elements (Batcher odd-even, 25 swaps).
                macro_rules! cswap {
                    ($a:expr, $b:expr) => {
                        if s[$a] > s[$b] { s.swap($a, $b); }
                    };
                }
                cswap!(0,1); cswap!(3,4); cswap!(6,7);
                cswap!(1,2); cswap!(4,5); cswap!(7,8);
                cswap!(0,1); cswap!(3,4); cswap!(6,7);
                cswap!(0,3); cswap!(3,6); cswap!(0,3);
                cswap!(1,4); cswap!(4,7); cswap!(1,4);
                cswap!(2,5); cswap!(5,8); cswap!(2,5);
                cswap!(1,3); cswap!(5,7);
                cswap!(2,6); cswap!(4,6); cswap!(2,4);
                cswap!(2,3); cswap!(5,6);

                // s[3] ≈ 33rd percentile, s[6] ≈ 66th percentile.
                (s[3], s[6])
            }

            fn bcsort_recursive_par(
                arr: &mut [$t],
                min: $t,
                max: $t,
                sum: $t,
                bad_splits: u32,
                threshold: usize,
            ) {
                let len = arr.len();

                if min == max || len <= 1 {
                    return;
                }

                // MESO SCALE: drop to single-threaded BCsort to avoid thread-thrashing.
                if len < threshold {
                    bcsort_recursive_sync(arr, min, max, sum, bad_splits);
                    return;
                }

                let (t1, t2) = compute_pivots(arr, min, max, sum, bad_splits);

                let mut low = 0;
                let mut mid = 0;
                let mut high = len;

                // min_l == min and max_r == max are guaranteed: min < t1 and max > t2.
                let mut max_l = $t::NEG_INFINITY;
                let mut sum_l = 0.0;
                let mut min_m = $t::INFINITY;
                let mut max_m = $t::NEG_INFINITY;
                let mut sum_m = 0.0;
                let mut min_r = $t::INFINITY;
                let mut sum_r = 0.0;

                // In-place Dutch National Flag partition + in-flight accumulation.
                while mid < high {
                    let val = arr[mid];
                    if val < t1 {
                        max_l = max_l.max(val);
                        sum_l += val;
                        arr.swap(low, mid);
                        low += 1;
                        mid += 1;
                    } else if val > t2 {
                        high -= 1;
                        min_r = min_r.min(val);
                        sum_r += val;
                        arr.swap(mid, high);
                    } else {
                        min_m = min_m.min(val);
                        max_m = max_m.max(val);
                        sum_m += val;
                        mid += 1;
                    }
                }

                // Contraction trigger: if the largest child holds >80% of the parent,
                // arithmetic pivots failed to balance. Increment so deeper levels
                // escalate to adaptive sampling. Never reset: prevents oscillation on
                // pathological inputs.
                let max_child = low.max(high - low).max(len - high);
                let next_bad = if max_child > len * 4 / 5 { bad_splits + 1 } else { bad_splits };

                let (left, rest) = arr.split_at_mut(low);
                let (middle, right) = rest.split_at_mut(high - low);

                rayon::join(
                    || bcsort_recursive_par(left, min, max_l, sum_l, next_bad, threshold),
                    || rayon::join(
                        || bcsort_recursive_par(middle, min_m, max_m, sum_m, next_bad, threshold),
                        || bcsort_recursive_par(right, min_r, max, sum_r, next_bad, threshold),
                    ),
                );
            }

            fn bcsort_recursive_sync(arr: &mut [$t], min: $t, max: $t, sum: $t, bad_splits: u32) {
                let len = arr.len();

                if min == max || len <= 1 {
                    return;
                }

                let (t1, t2) = compute_pivots(arr, min, max, sum, bad_splits);

                let mut low = 0;
                let mut mid = 0;
                let mut high = len;

                // min_l == min and max_r == max are guaranteed: min < t1 and max > t2.
                let mut max_l = $t::NEG_INFINITY;
                let mut sum_l = 0.0;
                let mut min_m = $t::INFINITY;
                let mut max_m = $t::NEG_INFINITY;
                let mut sum_m = 0.0;
                let mut min_r = $t::INFINITY;
                let mut sum_r = 0.0;

                // In-place Dutch National Flag partition + in-flight accumulation.
                while mid < high {
                    let val = arr[mid];
                    if val < t1 {
                        max_l = max_l.max(val);
                        sum_l += val;
                        arr.swap(low, mid);
                        low += 1;
                        mid += 1;
                    } else if val > t2 {
                        high -= 1;
                        min_r = min_r.min(val);
                        sum_r += val;
                        arr.swap(mid, high);
                    } else {
                        min_m = min_m.min(val);
                        max_m = max_m.max(val);
                        sum_m += val;
                        mid += 1;
                    }
                }

                // Contraction trigger: same logic as parallel path.
                let max_child = low.max(high - low).max(len - high);
                let next_bad = if max_child > len * 4 / 5 { bad_splits + 1 } else { bad_splits };

                bcsort_recursive_sync(&mut arr[0..low], min, max_l, sum_l, next_bad);
                bcsort_recursive_sync(&mut arr[low..high], min_m, max_m, sum_m, next_bad);
                bcsort_recursive_sync(&mut arr[high..len], min_r, max, sum_r, next_bad);
            }
        };
    };
}

// Generate implementation for both f32 and f64
impl_bcsort_for_float!(f32);
impl_bcsort_for_float!(f64);