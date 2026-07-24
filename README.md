# BCsort

**BCsort** is an in-place, parallel ternary distribution sorting algorithm for Rust. It optimizes memory-bandwidth utilization on modern hardware by interleaving child-partition pivot statistics extraction (`min`, `max`, `sum`) directly into the element partitioning sweep, eliminating redundant memory reads.

## Core Mechanism

In bandwidth-bound sorting tasks, memory latency dominates CPU execution time. Standard multi-pivot algorithms require secondary scans to compute splitters or histograms, paying a DRAM fetch latency penalty on each pass. Out-of-place radix sorts avoid comparison bottlenecks but require an $O(N)$ auxiliary buffer, doubling memory traffic and thrashing CPU caches on large arrays.

BCsort mitigates this bottleneck by accumulating partition metrics in-flight during a Dutch National Flag (DNF) sweep:
* **In-Flight Accumulation:** The partition loop moves elements into three buckets (less than $t_1$, between $t_1$ and $t_2$, greater than $t_2$) while updating running `min`, `max`, and `sum` accumulators for each child bucket.
* **Inherited Extrema:** Child partitions inherit exact boundary stats without secondary scans.
* **Latency Hiding:** FPU comparison and addition operations execute in the latency shadow of DRAM fetches, rendering statistic computation near-zero cost on memory-bound workloads.

## Architecture & Strategy

* **Root Initialization:** A parallel reduction computes initial `min`, `max`, and `sum` values to set starting pivots ($t_1, t_2$). This is the only dedicated dataset scan in the entire sorting pipeline.
* **Adaptive Pivot Escalation:** Pivots default to bisecting around the segment mean ($t_1 = \frac{\text{min} + \text{mean}}{2}$, $t_2 = \frac{\text{mean} + \text{max}}{2}$). If skewed data produces unbalanced partitions (where any child exceeds 80% of parent size across 3 consecutive calls on sub-arrays $>512$ elements), the strategy switches to a 9-element pseudo-random sample, using the 3rd and 6th order statistics as pivots.
* **Scale-Aware Parallelism:** Partitions exceeding a configurable threshold (`parallel_threshold`, default: 10,000) spawn parallel tasks via Rayon. Below this threshold, execution switches to single-threaded recursion to eliminate thread-pool dispatch overhead on sub-arrays that fit within local L2/L3 caches.
* **NaN / Inf Quarantine:** A single-threaded Hoare-style two-pointer scan moves non-finite floating-point values (`NaN`, `Inf`) to the trailing end of the slice before sorting begins. This removes branch-heavy non-finite checks from the primary partition loop and ensures numeric stability during scalar accumulator updates. *(Note: Quarantine swapping alters the relative ordering of non-finite elements, breaking original index mappings).*

## Performance Benchmarks

**Environment:** Intel i7-7900 | 24GB RAM | Linux  
**Compilation:** `RUSTFLAGS="-C target-cpu=native"` | Rust 1.78+  
**Comparisons:** `rayon::slice::par_sort_unstable` and `radsort` (v0.1.0) on 64-bit float (`f64`) arrays.

### Array Size Sweep (Uniform Distribution)

BCsort is engineered for large-scale, memory-constrained inputs. While `radsort` exhibits lower constant overhead on small arrays ($N \le 10,000$), BCsort achieves higher throughput on arrays between $100K$ and $10M$ elements by remaining strictly in-place ($O(1)$ auxiliary memory versus `radsort`'s $O(N)$ allocation).

| N | BCsort | Rayon `par_sort_unstable` | `radsort` | vs Rayon | vs `radsort` |
| :--- | :--- | :--- | :--- | :--- | :--- |
| 1,000 | 0.118 ms | 0.020 ms | 0.014 ms | 5.9x slower | 8.4x slower |
| 10,000 | 0.279 ms | 0.095 ms | 0.134 ms | 2.9x slower | 2.1x slower |
| 100,000 | 1.491 ms | 0.552 ms | 1.561 ms | 2.7x slower | **1.05x faster** |
| 1,000,000 | 14.33 ms | 5.70 ms | 17.17 ms | 2.5x slower | **1.20x faster** |
| 10,000,000 | 155.2 ms | 75.0 ms | 180.0 ms | 2.1x slower | **1.16x faster** |
| 100,000,000 | 1.781 s | 0.949 s | 1.721 s | 1.88x slower | 1.03x slower |

### Distribution Stress Test (N = 1,000,000)

Mean-based pivoting usually degrades on non-uniform datasets. The adaptive fallback mechanism keeps execution time within 7% of the uniform baseline under heavy distribution skew.

| Scenario | BCsort | Rayon `par_sort_unstable` | `radsort` | vs `radsort` |
| :--- | :--- | :--- | :--- | :--- |
| **Uniform** | 14.61 ms | 5.70 ms | 17.91 ms | **1.23x faster** |
| **Gaussian** | 15.57 ms | 5.58 ms | 17.28 ms | **1.11x faster** |
| **Pareto (Skewed)** | 15.63 ms | 5.87 ms | 17.97 ms | **1.15x faster** |
| **Nearly Sorted** | 6.82 ms | 3.28 ms | 16.61 ms | **2.43x faster** |
| **5% NaN Content** | 14.29 ms | 6.36 ms | 18.43 ms | **1.29x faster** |

### Execution Stability (N = 10,000,000 `f64`, 5 Trials)

By avoiding dynamic heap allocations during recursion, BCsort demonstrates lower run-to-run variance than buffer-allocated radix variants.

| Trial | BCsort | Rayon `par_sort_unstable` | `radsort` |
| :--- | :--- | :--- | :--- |
| 1 | 0.1558 s | 0.0775 s | 0.1918 s |
| 2 | 0.1542 s | 0.0739 s | 0.1831 s |
| 3 | 0.1531 s | 0.0681 s | 0.1775 s |
| 4 | 0.1509 s | 0.0667 s | 0.1861 s |
| 5 | 0.1511 s | 0.0669 s | 0.1722 s |
| **Variance Spread** | **~4.9 ms** | **~10.8 ms** | **~19.6 ms** |

## Usage

Add `bcsort` to your `Cargo.toml` dependencies:

```rust
use bcsort::{Bcsort, BcsortConfig};

fn main() {
    let mut data: Vec<f64> = vec![3.0, 1.0, 4.0, 1.0, 5.0, 9.0];
    
    // Standard execution
    data.bcsort();

    // Configured parallel threshold for specific hardware cache sizes
    let config = BcsortConfig { 
        parallel_threshold: 50_000 
    };
    data.bcsort_with_config(&config);
}
```

Supported types: `f32`, `f64`.

## License

MIT License. See `LICENSE` for details.
