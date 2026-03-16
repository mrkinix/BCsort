use bcsort::Bcsort;
use rand::Rng;
use rayon::prelude::*;
use std::time::Instant;

// ─── Distribution generators ──────────────────────────────────────────────────

fn gen_uniform(n: usize, rng: &mut impl Rng) -> Vec<f64> {
    (0..n).map(|_| rng.gen_range(0.0_f64..1_000_000.0)).collect()
}

fn gen_gaussian(n: usize, rng: &mut impl Rng) -> Vec<f64> {
    use std::f64::consts::PI;
    (0..n)
        .map(|_| {
            let u1: f64 = rng.gen_range(f64::EPSILON..1.0);
            let u2: f64 = rng.gen_range(0.0..1.0);
            (-2.0 * u1.ln()).sqrt() * (2.0 * PI * u2).cos() * 100_000.0 + 500_000.0
        })
        .collect()
}

fn gen_skewed(n: usize, rng: &mut impl Rng) -> Vec<f64> {
    // Pareto / power-law: mean pulled far from median — hardest case for
    // mean-guided partitioning
    (0..n)
        .map(|_| {
            let u: f64 = rng.gen_range(f64::EPSILON..1.0);
            1.0 / u
        })
        .collect()
}

fn gen_nearly_sorted(n: usize, rng: &mut impl Rng) -> Vec<f64> {
    let mut v: Vec<f64> = (0..n).map(|i| i as f64).collect();
    for _ in 0..n / 100 {
        let i = rng.gen_range(0..n);
        let j = rng.gen_range(0..n);
        v.swap(i, j);
    }
    v
}

fn gen_with_nans(n: usize, rng: &mut impl Rng) -> Vec<f64> {
    let mut v = gen_uniform(n, rng);
    for _ in 0..n / 20 {
        v[rng.gen_range(0..n)] = f64::NAN;
    }
    v
}

// ─── Timing helpers ───────────────────────────────────────────────────────────

fn time_sort<F>(data: &[f64], mut f: F) -> (f64, Vec<f64>)
where
    F: FnMut(&mut Vec<f64>),
{
    let mut v = data.to_vec();
    let t = Instant::now();
    f(&mut v);
    (t.elapsed().as_secs_f64(), v)
}

fn bench_avg<F>(data: &[f64], runs: usize, mut f: F) -> f64
where
    F: FnMut(&mut Vec<f64>),
{
    (0..runs).map(|_| time_sort(data, &mut f).0).sum::<f64>() / runs as f64
}

// Verify correctness.
// BCsort quarantines NaN/Inf to the tail (unsorted). radsort puts NaNs
// at the beginning AND end per its docs. We verify only the finite run
// for BCsort, and skip NaN position checking for radsort.
fn verify_bc(v: &[f64], label: &str) {
    // finite values are in the prefix; tail may contain NaN/Inf
    let finite_end = v.partition_point(|x| x.is_finite());
    assert!(
        v[..finite_end].windows(2).all(|w| w[0] <= w[1]),
        "BCsort FAILED correctness check for scenario '{label}'"
    );
}

fn verify_rayon(v: &[f64], label: &str) {
    // partial_cmp treats NaN as unordered; just check finite windows
    let finite: Vec<f64> = v.iter().copied().filter(|x| x.is_finite()).collect();
    assert!(
        finite.windows(2).all(|w| w[0] <= w[1]),
        "Rayon FAILED correctness check for scenario '{label}'"
    );
}

fn speedup(bc: f64, other: f64) -> String {
    if bc <= other {
        format!("BC +{:.0}%", (other / bc - 1.0) * 100.0)
    } else {
        format!("BC -{:.0}%", (bc / other - 1.0) * 100.0)
    }
}

// ─── Main ─────────────────────────────────────────────────────────────────────

fn main() {
    // Report whether AVX2 SIMD path compiled in
    #[cfg(target_feature = "avx2")]
    println!("\n  [OK] AVX2 SIMD scatter active");
    #[cfg(not(target_feature = "avx2"))]
    println!("\n  [!!] AVX2 NOT active — for best results run:\
              \n       PowerShell: $env:RUSTFLAGS=\"-C target-cpu=native\"; cargo run --release\
              \n       bash:       RUSTFLAGS=\"-C target-cpu=native\" cargo run --release\n");

    let mut rng = rand::thread_rng();
    let runs = 3; // averaged runs per cell to reduce noise

    // ── TABLE 1: Size sweep — uniform random f64 ─────────────────────────────
    let sizes: &[usize] = &[
        1_000,
        10_000,
        100_000,
        1_000_000,
        10_000_000,
        100_000_000,
        // 500_000_000,  // ~24 GB working set; uncomment if you have 32 GB+
    ];

    println!("\n SIZE SWEEP — Uniform random f64 — avg of {runs} runs");
    println!(" {:<14} {:<14} {:<14} {:<14} {:<13} {:<13}",
        "N", "BCsort v2(s)", "Rayon par(s)", "radsort (s)", "vs Rayon", "vs radsort");
    println!(" {}", "-".repeat(84));

    for &n in sizes {
        let data = gen_uniform(n, &mut rng);

        let t_bc  = bench_avg(&data, runs, |v| v.bcsort());
        let t_par = bench_avg(&data, runs, |v| {
            v.par_sort_unstable_by(|a, b| a.total_cmp(b));
        });
        let t_rad = bench_avg(&data, runs, |v| radsort::sort(v));

        // correctness spot-check
        let (_, s_bc)  = time_sort(&data, |v| v.bcsort());
        let (_, s_par) = time_sort(&data, |v| {
            v.par_sort_unstable_by(|a, b| a.total_cmp(b));
        });
        verify_bc(&s_bc, "uniform");
        verify_rayon(&s_par, "uniform");

        println!(
            " {:<14} {:<14.6} {:<14.6} {:<14.6} {:<13} {:<13}",
            n, t_bc, t_par, t_rad,
            speedup(t_bc, t_par),
            speedup(t_bc, t_rad),
        );
    }

    // ── TABLE 2: Distribution stress — N = 1M ────────────────────────────────
    let stress_n = 1_000_000;

    // (label, generator fn pointer)
    type GenFn = fn(usize, &mut rand::rngs::ThreadRng) -> Vec<f64>;
    let scenarios: &[(&str, GenFn)] = &[
        ("Uniform",          gen_uniform       as GenFn),
        ("Gaussian",         gen_gaussian      as GenFn),
        ("Pareto (skewed)",  gen_skewed        as GenFn),
        ("Nearly sorted",    gen_nearly_sorted as GenFn),
        ("5% NaN",           gen_with_nans     as GenFn),
    ];

    println!("\n DISTRIBUTION STRESS — N = 1,000,000 — avg of {runs} runs");
    println!(" {:<22} {:<14} {:<14} {:<14} {:<16}",
        "Scenario", "BCsort v2(s)", "Rayon par(s)", "radsort (s)", "BC vs best");
    println!(" {}", "-".repeat(82));

    for &(label, gen) in scenarios {
        let data = gen(stress_n, &mut rng);

        let t_bc  = bench_avg(&data, runs, |v| v.bcsort());
        let t_par = bench_avg(&data, runs, |v| {
            v.par_sort_unstable_by(|a, b| a.total_cmp(b));
        });
        let t_rad = bench_avg(&data, runs, |v| radsort::sort(v));

        let (_, s_bc) = time_sort(&data, |v| v.bcsort());
        verify_bc(&s_bc, label);

        let best_other = t_par.min(t_rad);
        println!(
            " {:<22} {:<14.6} {:<14.6} {:<14.6} {:<16}",
            label, t_bc, t_par, t_rad,
            speedup(t_bc, best_other),
        );
    }

    // ── TABLE 3: 10M variance — 5 individual runs ─────────────────────────────
    let n = 10_000_000;
    let data = gen_uniform(n, &mut rng);
    let detail_runs = 5;

    println!("\n 10M VARIANCE REPORT — {detail_runs} individual runs, uniform f64");
    println!(" {:<6} {:<14} {:<14} {:<14}", "Run", "BCsort v2(s)", "Rayon par(s)", "radsort (s)");
    println!(" {}", "-".repeat(50));

    for r in 1..=detail_runs {
        let (t_bc,  _) = time_sort(&data, |v| v.bcsort());
        let (t_par, _) = time_sort(&data, |v| {
            v.par_sort_unstable_by(|a, b| a.total_cmp(b));
        });
        let (t_rad, _) = time_sort(&data, |v| radsort::sort(v));
        println!(" {:<6} {:<14.6} {:<14.6} {:<14.6}", r, t_bc, t_par, t_rad);
    }
    println!();
}