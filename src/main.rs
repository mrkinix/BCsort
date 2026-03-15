use rand::Rng;
use bcsort::bcsort;
use std::time::Instant;
use rayon::prelude::*;

fn main() {
    // 1B f64 = ~8GB RAM
    let sizes = vec![10, 100, 32];

    println!(
        "{:<12} | {:<12} | {:<12} | {:<12} | {:<12}",
        "N", "BCsort (s)", "Rayon (s)", "Speedup", "Ratio (BC)"
    );
    println!("{:-<75}", "");

    for n in sizes {
        let mut rng = rand::thread_rng();

        let original_data: Vec<f64> =
            (0..n).map(|_| rng.gen_range(0.0..1000.0)).collect();

        println!("{:?}", original_data);

        // --- Benchmark BCsort ---
        let mut data_bc = original_data.clone();

        let start_bc = Instant::now();
        bcsort(&mut data_bc);
        let dur_bc = start_bc.elapsed().as_secs_f64();

        println!("{:?}", data_bc);

        // --- Standard unstable sort ---
        let mut data_std = original_data.clone();

        let start_std = Instant::now();
        data_std.sort_unstable_by(|a, b| a.partial_cmp(b).unwrap());
        let dur_std = start_std.elapsed().as_secs_f64();

        // --- Rayon parallel sort ---
        let mut data_par = original_data.clone();

        let start_par = Instant::now();
        data_par.par_sort_unstable_by(|a, b| a.partial_cmp(b).unwrap());
        let dur_par = start_par.elapsed().as_secs_f64();

        // --- Metrics ---
        let speedup = dur_par / dur_bc;
        let n_f = n as f64;
        let ratio = dur_bc / (n_f * n_f.log2());

        println!(
            "{:<12} | {:<12.8} | {:<12.8} | {:<12.2}x | {:<12.2e}",
            n, dur_bc, dur_par, speedup, ratio
        );

        // Verify sorted
        assert!(
            data_bc.windows(2).all(|w| w[0] <= w[1]),
            "BCsort failed for N={}",
            n
        );
    }
}