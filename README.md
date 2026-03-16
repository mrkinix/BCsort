# BCsort (Ben Chiboub Sort)

**BCsort** is an experimental, high-performance, in-place ternary distribution sort engineered in Rust. Created by **Hédi Ben Chiboub**, it explores a novel "Continuous-Space Spatial Partitioning" approach to numeric sorting. 

Instead of traditional median-pivot guessing (like QuickSort) or bitwise bucketing (like Radix Sort), BCsort calculates the spatial bounding box of the data and geometrically bisects it, routing elements into dynamically scaling "Gravity Wells."

## 🚀 The Goal: Combating the Memory Wall

While standard parallel comparison sorts (like Rayon's `pdqsort`) dominate in raw ALU instruction efficiency, they ignore spatial value distribution. Conversely, $O(N)$ Radix sorts are incredibly fast but require massive $O(N)$ auxiliary memory buffers.

**BCsort bridges this gap.** By strictly maintaining an in-place $O(1)$ memory footprint, BCsort leverages L2/L3 cache locality to match the speed of linear-time Radix sorts on massive datasets without the RAM allocation penalty.

## 📊 Benchmarks (Real-World Hardware)

**Hardware:** `[Insert Your CPU - e.g., i7-7900]`  
**Data Type:** `f64` (Numeric)  
**Execution:** `cargo run --release` with AVX2 enabled (`target-cpu=native`).

| N (Elements) | BCsort | Rayon (pdqsort) | Radsort ($O(N)$) | BC vs Radsort |
| :--- | :--- | :--- | :--- | :--- |
| 100,000 | 0.0016s | **0.0008s** | 0.0013s | *-25%* |
| 1,000,000 | **0.0140s** | **0.0054s** | 0.0157s | **+13% (BC Wins)** |
| 10,000,000 | **0.1416s** | **0.0702s** | 0.1677s | **+18% (BC Wins)** |
| 100,000,000| **1.6493s** | **0.9415s** | 1.6659s | **+1% (Tie)** |


## 🧠 Architecture: Geometric Bisection

1. **Root Bounds Pass**: A single parallel scan finds the absolute dataset minimum and maximum.
2. **Theoretical Thresholds**: The spatial zone is bisected.
   - $Mean = \frac{Min + Max}{2}$
   - $T_1 = \frac{Min + Mean}{2}$, $T_2 = \frac{Mean + Max}{2}$
3. **In-Place 3-Way DNF Partition**: Elements are physically routed into three contiguous memory arenas (`< T1`, `Between`, `> T2`).
4. **Zero-Scan Recursion**: Because the spatial bounds are theoretical, child chunks inherit their bounding box mathematically. No further $O(N)$ statistical scans are required.

## ⚠️ Distribution Stress & Robustness

BCsort features a strict, upfront $O(N)$ quarantine pass for `NaN` and `Infinity` values, making it highly robust for raw sensor telemetry or scientific data where standard floating-point sorts often panic.

| Scenario (1M floats) | BCsort Time | Behavior Note |
| :--- | :--- | :--- |
| **Uniform** | 0.018s | Standard baseline |
| **Gaussian** | 0.013s | Faster: Center-weighted bisection |
| **Pareto (Skewed)** | 0.013s | Faster: Immediate outlier isolation |
| **Nearly Sorted** | 0.004s | $O(N)$ collapse via contraction check |

## 👤 Author

**Hédi Ben Chiboub** *Pragmatic Systems Architect* [Portfolio](https://benchiboub.com)

## ⚖️ License
MIT License.