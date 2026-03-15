use rayon::prelude::*;

// Tune this based on your hardware. 10_000 is a safe starting point.
const PARALLEL_THRESHOLD: usize = 10_000;

pub fn bcsort(arr: &mut [f64]) {
    let len = arr.len();
    if len <= 1 { return; }

    // 1. NaN Quarantine (Single threaded pass, too fast to bother parallelizing)
    let mut valid_len = len;
    let mut i = 0;
    while i < valid_len {
        if arr[i].is_nan() || arr[i].is_infinite() {
            valid_len -= 1;
            arr.swap(i, valid_len);
        } else {
            i += 1;
        }
    }

    if valid_len <= 1 { return; }
    let valid_arr = &mut arr[0..valid_len];

    // 2. ROOT MACRO SCALE: The ONLY standalone stats pass in the entire algorithm
    let (min, max, sum) = valid_arr.par_iter().copied().fold(
        || (f64::INFINITY, f64::NEG_INFINITY, 0.0),
        |acc, x| (acc.0.min(x), acc.1.max(x), acc.2 + x)
    ).reduce(
        || (f64::INFINITY, f64::NEG_INFINITY, 0.0),
        |a, b| (a.0.min(b.0), a.1.max(b.1), a.2 + b.2)
    );

    // Launch Parallel Engine with Inherited Stats
    bcsort_recursive_par(valid_arr, min, max, sum);
}

fn bcsort_recursive_par(arr: &mut [f64], min: f64, max: f64, sum: f64) {
    let len = arr.len();
    
    // Base Case & Contraction Check
    if min == max || len <= 1 { return; }

    // MESO SCALE: Drop to single-threaded BcSort to avoid thread-thrashing.
    if len < PARALLEL_THRESHOLD {
        bcsort_recursive_sync(arr, min, max, sum); 
        return;
    }

    let mean = sum / (len as f64);
    let t1 = (min + mean) / 2.0;
    let t2 = (mean + max) / 2.0;

    let mut low = 0;
    let mut mid = 0;
    let mut high = len; 

    // Inherited Trackers for the next generation
    let mut min_l = f64::INFINITY; let mut max_l = f64::NEG_INFINITY; let mut sum_l = 0.0;
    let mut min_m = f64::INFINITY; let mut max_m = f64::NEG_INFINITY; let mut sum_m = 0.0;
    let mut min_r = f64::INFINITY; let mut max_r = f64::NEG_INFINITY; let mut sum_r = 0.0;

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
            // mid is not incremented; the newly swapped value will be evaluated next
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
        || bcsort_recursive_par(left, min_l, max_l, sum_l),
        || rayon::join(
            || bcsort_recursive_par(middle, min_m, max_m, sum_m),
            || bcsort_recursive_par(right, min_r, max_r, sum_r)
        )
    );
}

fn bcsort_recursive_sync(arr: &mut [f64], min: f64, max: f64, sum: f64) {
    let len = arr.len();
    
    // Base Case & Contraction Check
    if min == max || len <= 1 { return; }

    // CUTOFF: Defer to standard sort for small arrays (Hardware Efficiency)
    if len <= 32 {
        arr.sort_unstable_by(|a, b| a.partial_cmp(b).unwrap());
        return;
    }

    let mean = sum / (len as f64);
    let t1 = (min + mean) / 2.0;
    let t2 = (mean + max) / 2.0;

    let mut low = 0;
    let mut mid = 0;
    let mut high = len; 

    // Inherited Trackers for the next generation
    let mut min_l = f64::INFINITY; let mut max_l = f64::NEG_INFINITY; let mut sum_l = 0.0;
    let mut min_m = f64::INFINITY; let mut max_m = f64::NEG_INFINITY; let mut sum_m = 0.0;
    let mut min_r = f64::INFINITY; let mut max_r = f64::NEG_INFINITY; let mut sum_r = 0.0;

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
    bcsort_recursive_sync(&mut arr[0..low], min_l, max_l, sum_l);       // near_min
    bcsort_recursive_sync(&mut arr[low..high], min_m, max_m, sum_m);    // near_mean
    bcsort_recursive_sync(&mut arr[high..len], min_r, max_r, sum_r);    // near_max
}