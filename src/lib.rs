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

// Macro to implement BCsort for primitive float types without complex trait bounds.
// The `const _: () = { ... }` block ensures internal helper functions don't 
// cause namespace collisions when the macro is invoked multiple times.
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

                    // 1. NaN/Inf Quarantine (Single threaded pass)
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

                    // 2. ROOT MACRO SCALE
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

                    bcsort_recursive_par(valid_arr, min, max, sum, config.parallel_threshold);
                }
            }

            fn bcsort_recursive_par(arr: &mut [$t], min: $t, max: $t, sum: $t, threshold: usize) {
                let len = arr.len();

                // Base Case & Contraction Check
                if min == max || len <= 1 {
                    return;
                }

                // MESO SCALE: Drop to single-threaded BcSort to avoid thread-thrashing.
                if len < threshold {
                    bcsort_recursive_sync(arr, min, max, sum);
                    return;
                }

                let mean = sum / (len as $t);
                let t1 = (min + mean) / 2.0;
                let t2 = (mean + max) / 2.0;

                let mut low = 0;
                let mut mid = 0;
                let mut high = len;

                // Inherited Trackers for the next generation
                let mut min_l = $t::INFINITY; let mut max_l = $t::NEG_INFINITY; let mut sum_l = 0.0;
                let mut min_m = $t::INFINITY; let mut max_m = $t::NEG_INFINITY; let mut sum_m = 0.0;
                let mut min_r = $t::INFINITY; let mut max_r = $t::NEG_INFINITY; let mut sum_r = 0.0;

                // SECOND PASS: In-Place Dutch National Flag Partition + In-Flight Accumulation
                while mid < high {
                    let val = arr[mid];

                    if val < t1 {
                        min_l = min_l.min(val); max_l = max_l.max(val); sum_l += val;
                        arr.swap(low, mid);
                        low += 1;
                        mid += 1;
                    } else if val > t2 {
                        high -= 1;
                        min_r = min_r.min(val); max_r = max_r.max(val); sum_r += val;
                        arr.swap(mid, high);
                    } else {
                        min_m = min_m.min(val); max_m = max_m.max(val); sum_m += val;
                        mid += 1;
                    }
                }

                // SAFETY FALLBACK
                if low == len || (high - low) == len || (len - high) == len {
                    arr.par_sort_unstable_by(|a, b| a.partial_cmp(b).unwrap());
                    return;
                }

                let (left, rest) = arr.split_at_mut(low);
                let (middle, right) = rest.split_at_mut(high - low);

                // Fork-Join with inherited stats
                rayon::join(
                    || bcsort_recursive_par(left, min_l, max_l, sum_l, threshold),
                    || {
                        rayon::join(
                            || bcsort_recursive_par(middle, min_m, max_m, sum_m, threshold),
                            || bcsort_recursive_par(right, min_r, max_r, sum_r, threshold),
                        )
                    },
                );
            }

            fn bcsort_recursive_sync(arr: &mut [$t], min: $t, max: $t, sum: $t) {
                let len = arr.len();

                // Base Case & Contraction Check
                if min == max || len <= 1 {
                    return;
                }

              if len <= 32 {
                    for i in 1..len {
                        let mut j = i;
                        while j > 0 && (arr[j-1] > arr[j] || arr[j-1].is_nan()) {
                            arr.swap(j, j-1);
                            j -= 1;
                        }
                    }
                    return;
                }

                let mean = sum / (len as $t);
                let t1 = (min + mean) / 2.0;
                let t2 = (mean + max) / 2.0;

                let mut low = 0;
                let mut mid = 0;
                let mut high = len;

                // Inherited Trackers for the next generation
                let mut min_l = $t::INFINITY; let mut max_l = $t::NEG_INFINITY; let mut sum_l = 0.0;
                let mut min_m = $t::INFINITY; let mut max_m = $t::NEG_INFINITY; let mut sum_m = 0.0;
                let mut min_r = $t::INFINITY; let mut max_r = $t::NEG_INFINITY; let mut sum_r = 0.0;

                // In-Place Dutch National Flag Partition + In-Flight Accumulation
                while mid < high {
                    let val = arr[mid];

                    if val < t1 {
                        min_l = min_l.min(val); max_l = max_l.max(val); sum_l += val;
                        arr.swap(low, mid);
                        low += 1;
                        mid += 1;
                    } else if val > t2 {
                        high -= 1;
                        min_r = min_r.min(val); max_r = max_r.max(val); sum_r += val;
                        arr.swap(mid, high);
                    } else {
                        min_m = min_m.min(val); max_m = max_m.max(val); sum_m += val;
                        mid += 1;
                    }
                }

                // SAFETY FALLBACK: If partition fails to divide (Skew trap)
                if low == len || (high - low) == len || (len - high) == len {
                    arr.sort_unstable_by(|a, b| a.partial_cmp(b).unwrap());
                    return;
                }

                // RECURSIVE BOOM
                bcsort_recursive_sync(&mut arr[0..low], min_l, max_l, sum_l);
                bcsort_recursive_sync(&mut arr[low..high], min_m, max_m, sum_m);
                bcsort_recursive_sync(&mut arr[high..len], min_r, max_r, sum_r);
            }
        };
    };
}

// Generate implementation for both f32 and f64
impl_bcsort_for_float!(f32);
impl_bcsort_for_float!(f64);