// bcsort/src/lib.rs — BCsort v4 (Pure Spatial Partitioning / Zero-Accumulation Inner Loop)

use rayon::prelude::*;

const PARALLEL_THRESHOLD: usize = 8_000;
const SMALL_SORT_THRESHOLD: usize = 32;

// ─── Public entry point ───────────────────────────────────────────────────────

pub fn bcsort(arr: &mut [f64]) {
    let len = arr.len();
    if len <= 1 { return; }

    // Quarantine NaN / Inf to the tail.
    let mut valid_len = len;
    let mut i = 0;
    while i < valid_len {
        if !arr[i].is_finite() {
            valid_len -= 1;
            arr.swap(i, valid_len);
        } else {
            i += 1;
        }
    }
    if valid_len <= 1 { return; }
    let valid_arr = &mut arr[..valid_len];

    // ROOT STATS — The ONLY O(N) accumulation in the entire algorithm.
    let (min, max, sum) = if valid_len >= PARALLEL_THRESHOLD * 2 {
        stats_par(valid_arr)
    } else {
        stats_scalar(valid_arr)
    };

    if min == max { return; }

    // Calculate the TRUE root thresholds
    let root_mean = sum / valid_len as f64;
    let t1 = (min + root_mean) / 2.0;
    let t2 = (root_mean + max) / 2.0;

    // First partition using true stats
    let (low, high) = partition_inplace(valid_arr, t1, t2);

    if low == valid_len || (high - low) == valid_len || (valid_len - high) == valid_len {
        valid_arr.par_sort_unstable_by(|a, b| a.total_cmp(b));
        return;
    }

    let (left, rest) = valid_arr.split_at_mut(low);
    let (middle, right) = rest.split_at_mut(high - low);

    // Fork-Join passing THEORETICAL BOUNDS
    rayon::join(
        || bcsort_par(left, min, t1),
        || rayon::join(
            || bcsort_par(middle, t1, t2),
            || bcsort_par(right, t2, max),
        ),
    );
}

// ─── Stats helpers ────────────────────────────────────────────────────────────

#[inline(always)]
fn stats_scalar(arr: &[f64]) -> (f64, f64, f64) {
    let mut mn = f64::INFINITY;
    let mut mx = f64::NEG_INFINITY;
    let mut s  = 0.0_f64;
    for &x in arr {
        mn = mn.min(x);
        mx = mx.max(x);
        s += x;
    }
    (mn, mx, s)
}

fn stats_par(arr: &[f64]) -> (f64, f64, f64) {
    arr.par_iter()
        .copied()
        .fold(
            || (f64::INFINITY, f64::NEG_INFINITY, 0.0_f64),
            |acc, x| (acc.0.min(x), acc.1.max(x), acc.2 + x),
        )
        .reduce(
            || (f64::INFINITY, f64::NEG_INFINITY, 0.0_f64),
            |a, b| (a.0.min(b.0), a.1.max(b.1), a.2 + b.2),
        )
}

// ─── Parallel layer ───────────────────────────────────────────────────────────

fn bcsort_par(arr: &mut [f64], b_min: f64, b_max: f64) {
    let len = arr.len();
    if len <= 1 { return; }
    if len < PARALLEL_THRESHOLD {
        bcsort_sync(arr, b_min, b_max);
        return;
    }

    // GEOMETRIC BISECTION: We derive the mean and thresholds purely from the bounding box.
    let mean = (b_min + b_max) / 2.0;
    let t1 = (b_min + mean) / 2.0;
    let t2 = (mean + b_max) / 2.0;

    let (low, high) = partition_inplace(arr, t1, t2);

    // SAFETY GUARD: If theoretical bounds fail to split the empirical data, delegate to hardware sort.
    if low == len || (high - low) == len || (len - high) == len {
        arr.par_sort_unstable_by(|a, b| a.total_cmp(b));
        return;
    }

    let (left, rest) = arr.split_at_mut(low);
    let (middle, right) = rest.split_at_mut(high - low);

    rayon::join(
        || bcsort_par(left, b_min, t1),
        || rayon::join(
            || bcsort_par(middle, t1, t2),
            || bcsort_par(right, t2, b_max),
        ),
    );
}

// ─── Single-threaded layer ────────────────────────────────────────────────────

fn bcsort_sync(arr: &mut [f64], b_min: f64, b_max: f64) {
    let len = arr.len();
    if len <= SMALL_SORT_THRESHOLD {
        arr.sort_unstable_by(|a, b| a.total_cmp(b));
        return;
    }

    let mean = (b_min + b_max) / 2.0;
    let t1 = (b_min + mean) / 2.0;
    let t2 = (mean + b_max) / 2.0;

    let (low, high) = partition_inplace(arr, t1, t2);

    if low == len || (high - low) == len || (len - high) == len {
        arr.sort_unstable_by(|a, b| a.total_cmp(b));
        return;
    }

    bcsort_sync(&mut arr[..low], b_min, t1);
    bcsort_sync(&mut arr[low..high], t1, t2);
    bcsort_sync(&mut arr[high..], t2, b_max);
}

// ─── Core partition ───────────────────────────────────────────────────────────
//
// Pure data movement. Zero accumulation.
#[inline(always)]
fn partition_inplace(arr: &mut [f64], t1: f64, t2: f64) -> (usize, usize) {
    let len = arr.len();
    let mut low  = 0;
    let mut mid  = 0;
    let mut high = len;

    while mid < high {
        // SAFETY: mid < high <= len ensures mid is within bounds.
        let val = unsafe { *arr.get_unchecked(mid) };

        if val < t1 {
            unsafe {
                let p_low = arr.as_mut_ptr().add(low);
                let p_mid = arr.as_mut_ptr().add(mid);
                std::ptr::swap(p_low, p_mid);
            }
            low += 1;
            mid += 1;
        } else if val > t2 {
            high -= 1;
            unsafe {
                let p_mid  = arr.as_mut_ptr().add(mid);
                let p_high = arr.as_mut_ptr().add(high);
                std::ptr::swap(p_mid, p_high);
            }
        } else {
            mid += 1;
        }
    }

    (low, high)
}