# BCsort (Ben Chiboub Sort)

**BCsort** is a high-performance, in-place ternary distribution sorting algorithm engineered for Rust. Created by **Hédi Ben Chiboub**, it is designed to combat the **Memory Wall** by utilizing continuous-space spatial partitioning. 

Instead of traditional median-pivot guessing (QuickSort) or bitwise bucketing (Radix Sort), BCsort calculates the spatial bounding box of the data and geometrically bisects it, routing elements into dynamically scaling "Gravity Wells."

## Performance: The Radix Intercept

BCsort achieves its speed by maintaining a strict $O(1)$ auxiliary memory footprint. While memory-bound $O(N)$ sorts like `radsort` require massive $O(N)$ buffers, BCsort remains cache-resident, allowing it to outperform linear-time algorithms on datasets between $10^6$ and $10^8$ elements.

**Hardware:** i7-7900 | 24GB RAM  
**Environment:** `cargo run --release` | `RUSTFLAGS="-C target-cpu=native"`  
**Data Type:** `f64` (Uniform Random Distribution)

### SIZE SWEEP — Uniform random f64 — avg of 3 runs
| N | BCsort v2(s) | Rayon par(s) | radsort (s) | vs Rayon | vs radsort |
| :--- | :--- | :--- | :--- | :--- | :--- |
| 1000 | 0.000230 | 0.000020 | 0.000015 | BC -1053% | BC -1429% |
| 10000 | 0.000422 | 0.000172 | 0.000146 | BC -146% | BC -189% |
| 100000 | 0.001506 | 0.000719 | 0.001686 | BC -109% | BC +12% |
| 1000000 | 0.014371 | 0.007046 | 0.020077 | BC -104% | BC +40% |
| 10000000 | 0.143960 | 0.063030 | 0.171723 | BC -128% | BC +19% |
| 100000000 | 1.683463 | 0.891120 | 1.694845 | BC -89% | BC +1% |

### DISTRIBUTION STRESS — N = 1,000,000 — avg of 3 runs
| Scenario | BCsort v2(s) | Rayon par(s) | radsort (s) | BC vs best |
| :--- | :--- | :--- | :--- | :--- |
| Uniform | 0.014340 | 0.006156 | 0.031033 | BC -133% |
| Gaussian | 0.013630 | 0.005672 | 0.015943 | BC -140% |
| Pareto (skewed) | 0.013358 | 0.005585 | 0.015762 | BC -139% |
| Nearly sorted | 0.004368 | 0.003366 | 0.015141 | BC -30% |
| 5% NaN | 0.013518 | 0.005486 | 0.016010 | BC -146% |

### 10M VARIANCE REPORT — 5 individual runs, uniform f64
| Run | BCsort v2(s) | Rayon par(s) | radsort (s) |
| :--- | :--- | :--- | :--- |
| 1 | 0.147147 | 0.085279 | 0.174830 |
| 2 | 0.149357 | 0.085369 | 0.174691 |
| 3 | 0.151786 | 0.080296 | 0.173559 |
| 4 | 0.146650 | 0.079965 | 0.179120 |
| 5 | 0.148543 | 0.079260 | 0.177187 |

## Architecture: 1D Octree Partitioning

BCsort treats sorting as a spatial subdivision problem rather than a comparison problem:

1. **Root Stats**: A single parallel scan identifies the absolute `min`, `max`, and `mean` to anchor the data's center of gravity.
2. **Zero-Scan Recursion**: After the root pass, BCsort uses **Theoretical Bisection**. Child chunks inherit their bounding boxes mathematically ($T_1 = \frac{min+mean}{2}$, $T_2 = \frac{mean+max}{2}$). This eliminates subsequent $O(N)$ stats scans, cutting memory bandwidth usage by ~50%.
3. **Cache-Aware DNF**: Using an in-place 3-way Dutch National Flag partition, BCsort groups data into territories that remain resident in L2/L3 cache, bypassing the RAM latency of out-of-place distribution sorts.
4. A mandatory $O(N)$ quarantine pass handles `NaN` and `Inf` values using `total_cmp` logic, ensuring zero-panic execution on scientific or sensor telemetry data.

## 👤 Author

**Hédi Ben Chiboub** *Systems Architect & Pragmatic Engineer* [Portfolio](https://benchiboub.com)

## ⚖️ License

Distributed under the MIT License. See `LICENSE` for more information.
