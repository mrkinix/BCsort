use std::time::Instant;
use bcsort::bcsort;
use rand::Rng;

fn main() {
    let sizes = vec![100,10_000, 100_000, 1_000_000, 10_000_000, 100_000_000, 500_000_000, 1_000_000_000];
    println!("{:<12} | {:<15} | {:<15}", "N", "Time (s)", "Ratio T/(N log2 N)");
    println!("{:-<48}", "");

    for n in sizes {
        // Generate Uniform Data
        let mut rng = rand::thread_rng();
        let mut data: Vec<f64> = (0..n).map(|_| rng.gen_range(0.0..1000.0)).collect();

        let start = Instant::now();
        bcsort(&mut data);
        let duration = start.elapsed().as_secs_f64();

        // Check Complexity Ratio
        let n_f = n as f64;
        let ratio = duration / (n_f * n_f.log2());

        println!("{:<12} | {:<15.6} | {:<15.2e}", n, duration, ratio);
    }
}