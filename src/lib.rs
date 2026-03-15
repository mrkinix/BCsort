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

    // 2. Launch Parallel Engine
    bcsort_recursive_par(&mut arr[0..valid_len]);
}

fn bcsort_recursive_par(arr: &mut [f64]) {
    let len = arr.len();


    // MESO SCALE: If chunk is small enough, drop to single-threaded BcSort 
    // to avoid thread-thrashing. (Assume bcsort_recursive from previous iteration exists here)
    if len < PARALLEL_THRESHOLD {
        bcsort_recursive_sync(arr); 
        return;
    }

    // MACRO SCALE: Parallel Stats Gathering
    // We can even parallelize the min/max/sum calculation!
    let (min, max, sum) = arr.par_iter().copied().fold(
        || (f64::INFINITY, f64::NEG_INFINITY, 0.0),
        |acc, x| (acc.0.min(x), acc.1.max(x), acc.2 + x)
    ).reduce(
        || (f64::INFINITY, f64::NEG_INFINITY, 0.0),
        |a, b| (a.0.min(b.0), a.1.max(b.1), a.2 + b.2)
    );

    if min == max { return; }

    let mean = sum / (len as f64);
    let t1 = (min + mean) / 2.0;
    let t2 = (mean + max) / 2.0;

    // SECOND PASS: In-Place Dutch National Flag Partition (Sequential per chunk)
    let mut low = 0;
    let mut mid = 0;
    let mut high = len; 

    while mid < high {
        if arr[mid] < t1 {
            arr.swap(low, mid);
            low += 1;
            mid += 1;
        } else if arr[mid] > t2 {
            high -= 1;
            arr.swap(mid, high);
        } else {
            mid += 1;
        }
    }

    // SAFETY FALLBACK
    if low == len || (high - low) == len || (len - high) == len {
        arr.par_sort_unstable_by(|a, b| a.partial_cmp(b).unwrap());
        return;
    }

    // RECURSIVE BOOM: The Parallel Borrow-Checker Bypass
    // We MUST use split_at_mut to prove to Rust that the slices do not overlap.
    let (left, rest) = arr.split_at_mut(low);
    let (middle, right) = rest.split_at_mut(high - low);

    // Fork-Join the three mutually exclusive blocks across CPU cores
    rayon::join(
        || bcsort_recursive_par(left),
        || rayon::join(
            || bcsort_recursive_par(middle),
            || bcsort_recursive_par(right)
        )
    );
}

fn bcsort_recursive_sync(arr: &mut [f64]) {
    let len = arr.len();
    
    // CUTOFF: Defer to standard sort for small arrays (Hardware Efficiency)
    if len <= 32 {
        arr.sort_unstable_by(|a, b| a.partial_cmp(b).unwrap());
        return;
    }

    // FIRST PASS: Stats
    let mut min = arr[0];
    let mut max = arr[0];
    let mut sum = 0.0;

    for &x in arr.iter() {
        if x < min { min = x; }
        if x > max { max = x; }
        sum += x;
    }

    // Contraction Check
    if min == max { return; }

    // Thresholds
    let mean = sum / (len as f64);
    let t1 = (min + mean) / 2.0;
    let t2 = (mean + max) / 2.0;

    // SECOND PASS: In-Place Dutch National Flag Partition
    let mut low = 0;
    let mut mid = 0;
    let mut high = len; 

    while mid < high {
        if arr[mid] < t1 {
            arr.swap(low, mid);
            low += 1;
            mid += 1;
        } else if arr[mid] > t2 {
            high -= 1;
            arr.swap(mid, high);
        } else {
            mid += 1;
        }
    }

    // SAFETY FALLBACK: If partition fails to divide (Skew trap)
    if low == len || (high - low) == len || (len - high) == len {
        arr.sort_unstable_by(|a, b| a.partial_cmp(b).unwrap());
        return;
    }

    // RECURSIVE BOOM
    bcsort_recursive_sync(&mut arr[0..low]);      // near_min
    bcsort_recursive_sync(&mut arr[low..high]);   // near_mean
    bcsort_recursive_sync(&mut arr[high..len]);   // near_max
}