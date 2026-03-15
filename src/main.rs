use rand::Rng;
use bcsort::bcsort;
use std::time::Instant;
use rayon::prelude::*;

// Assuming your bcsort and bcsort_recursive_par are in the same scope or imported
// mod bcsort_module; use bcsort_module::bcsort;

fn main() {
    // Note: 1B f64s = 8GB RAM. Ensure your system has ~16GB+ to run the full suite safely.
    let sizes = vec![
        10,
        100,
        1_000,
        100_000, 
        1_000_000, 
        10_000_000, 
        100_000_000, 
        500_000_000
    ];

    println!(
        "{:<12} | {:<12} | {:<12} | {:<12} | {:<12}",
        "N", "BCsort (s)", "Unstable (s)", "Speedup", "Ratio (BC)"
    );
    println!("{:-<75}", "");

    for n in sizes {
        let mut rng = rand::thread_rng();
        let original_data: Vec<f64> = (0..n).map(|_| rng.gen_range(0.0..1000.0)).collect();

        // --- Benchmark BCsort ---
        let mut data_bc = original_data.clone();
        let start_bc = Instant::now();
        bcsort(&mut data_bc); // Your custom function
        let dur_bc = start_bc.elapsed().as_secs_f64();

        // --- Benchmark slice::sort_unstable (Standard) ---
        let mut data_std = original_data.clone();
        let start_std = Instant::now();
        data_std.sort_unstable_by(|a, b| a.partial_cmp(b).unwrap());
        let dur_std = start_std.elapsed().as_secs_f64();

        // Note: Rust's sort_unstable IS currently a variation of pdqsort.
        // To compare against PARALLEL standard sort, use Rayon's par_sort_unstable:
        let mut data_par = original_data.clone();
        let start_par = Instant::now();
        data_par.par_sort_unstable_by(|a, b| a.partial_cmp(b).unwrap());
        let dur_par = start_par.elapsed().as_secs_f64();

        // --- Calculations ---
        let speedup = dur_par / dur_bc; // Comparing Parallel vs Parallel for fairness
        let n_f = n as f64;
        let ratio = dur_bc / (n_f * n_f.log2());

        println!(
            "{:<12} | {:<12.4} | {:<12.4} | {:<12.2}x | {:<12.2e}",
            n, dur_bc, dur_par, speedup, ratio
        );

        // Verification Gate
        assert!(data_bc.windows(2).all(|w| w[0] <= w[1]), "BCsort failed to sort N={}", n);
    }
}