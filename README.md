# BCsort (Ben Chiboub Sort)

**BCsort** is a high-performance, in-place ternary distribution sorting algorithm written in Rust. It bridges the gap between statistical distribution sorts (like FlashSort) and exchange-based comparison sorts (like 3-Way QuickSort) by utilizing dynamic "Gravity Wells" derived from the dataset's center of mass.

By replacing traditional random/median pivot guessing with mathematically calculated spatial thresholds, BCsort maximizes the information entropy destroyed per recursion, strictly maintaining an $O(N \log N)$ time complexity while minimizing the constant hardware factor.

## 🚀 Performance & Benchmarks

BCsort is engineered for cache-locality and parallel processing. The following benchmarks demonstrate the algorithmic stability and flat complexity ratio $T / (N \log N)$ across massive datasets.

**Hardware Profile:** `i7-9700`
**Data Type:** `f64` (Uniform Random Distribution)

| N (Elements) | Time (s) | Ratio T / (N log2 N) |
| :--- | :--- | :--- |
| 100 | 0.000051 | 7.69e-8 |
| 10,000 | 0.003896 | 2.93e-8 |
| 100,000 | 0.028761 | 1.73e-8 |
| 1,000,000 | 0.154423 | 7.75e-9 |
| 10,000,000 | 1.929839 | 8.30e-9 |
| 100,000,000 | 20.726579 | 7.80e-9 |
| **500,000,000** | **108.787915** | **7.53e-9** |

*Note: The collapse of the constant factor at $N \ge 1,000,000$ demonstrates high efficiency in CPU pre-fetching and multi-core thread saturation.*

## 🧠 The Engine: How It Works

Traditional QuickSort removes 1 bit of entropy per comparison (Is $X > Pivot$?). BCsort removes $\approx 1.58$ bits ($\log_2 3$) by routing elements into three deterministic spatial zones based on proximity to the data's absolute boundaries and arithmetic mean.

1. **Stats Pass ($O(N)$)**: Calculate `min`, `max`, and `mean`.
2. **Threshold Generation**: 
   - $T_1 = \frac{min + mean}{2}$
   - $T_2 = \frac{mean + max}{2}$
3. **In-Place Dutch National Flag Partition ($O(N)$)**: Elements are swapped in-place into three mutually exclusive memory blocks:
   - `< T1` (Near Min)
   - `> T2` (Near Max)
   - Between $T_1$ and $T_2$ (Near Mean)
4. **Fork-Join Parallelism**: The three memory blocks are passed to parallel threads (via `rayon`) for recursive sorting.
5. **Hardware Fallback**: For subsets $N \le 32$, BCsort drops to an instruction-level optimal standard sort to avoid thread-thrashing.

## Robustness

BCsort includes a mandatory $O(N)$ quarantine pass for `NaN` and `Infinity` values. Floating-point datasets with `NaN` will destroy statistical thresholds. BCsort isolates these values at the array boundary instantly, ensuring zero panics and mathematically sound partitions.

## 🗺️ Roadmap

- [x] Python Proof of Concept (V1)
- [x] Rust In-Place Pointer Swap (V2)
- [x] Rayon Multi-Threading Integration
- [ ] **GPGPU / CUDA Port**: Implementing the First Pass (Min, Max, Sum) and Threshold distribution as compute shaders targeting architectures for extreme parallel sorting.
- [ ] Generic Struct sorting via derived numerical keys.

## 👤 About the Author

**Hédi Ben Chiboub** [Portfolio](https://benchiboub.com)

## ⚖️ License

Distributed under the MIT License. See `LICENSE` for more information.