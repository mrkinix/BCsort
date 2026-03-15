# BCsort (Ben Chiboub Sort)

**BCsort** is a high-performance, in-place ternary distribution sorting algorithm engineered for Rust. Created by **Hédi Ben Chiboub**, it is designed to outperform standard comparison sorts on massive numeric datasets where memory bandwidth becomes the primary bottleneck.

By utilizing **Inherited Stats Partitioning**, BCsort eliminates redundant memory scans, allowing it to destroy information entropy at a rate of $\approx 1.58$ bits per pass ($\log_2 3$) while remaining cache-resident.

## 🚀 Performance: Breaking the Rayon Barrier

BCsort dominates on chaotic, uniform, and scientific datasets. The following benchmarks compare **BCsort** against **Rayon's Standard Parallel Unstable Sort** (`pdqsort` variant).

**Hardware:** `[Insert Your CPU - e.g., Ryzen 9 5950X]`  
**Data Type:** `f64` (Numeric)

### Macro-Scale (Absolute Performance)
| N (Elements) | BCsort (s) | Rayon Std (s) | Speedup |
| :--- | :--- | :--- | :--- |
| 100,000 | 0.0117s | 0.0145s | **1.23x** |
| 1,000,000 | 0.1302s | 0.1357s | **1.04x** |
| 10,000,000 | 1.3463s | 1.7393s | **1.29x** |
| **100,000,000** | **16.169s** | **20.973s** | **1.30x** |

### Domain-Specific Dominance (1M Elements)
| Dataset Scenario | BCsort (s) | Rayon Std (s) | Speedup |
| :--- | :--- | :--- | :--- |
| **Financial Ticks (Random Walk)** | 0.1007s | 0.1403s | **1.39x** |
| **Scientific Computing (5% NaN)** | 0.1113s | 0.1320s | **1.19x** |
| **Monte-Carlo (Uniform)** | 0.1423s | 0.1602s | **1.13x** |
| **ETL Pipelines (Skewed)** | 0.1158s | 0.1320s | **1.14x** |

## 🧠 The "Inherited Stats" Breakthrough

The fatal flaw of most distribution sorts is the $O(N)$ pass required to find the dataset statistics (Min, Max, Mean). 

BCsort bypasses this via **In-Flight Accumulation**:
1. **Root Pass**: The only standalone $O(N)$ scan to establish initial gravity wells.
2. **The 3-Way DNF Partition**: Elements are routed into `near_min`, `near_mean`, and `near_max` territories.
3. **In-Flight Stats**: As elements are swapped in the CPU registers, their `min`, `max`, and `sum` are accumulated for the *next* recursive generation.
4. **Zero-Scan Recursion**: Child chunks receive their stats via inheritance. They begin partitioning immediately, cutting memory reads by ~50% compared to traditional multi-pass algorithms.



## 🛠️ Hybrid Architecture

BCsort is a pragmatic engine. It uses a **Small-Scale Cutoff** ($N \le 32$) to fall back to instruction-level optimal standard sorts, avoiding FPU and recursion overhead on tiny chunks. This ensures that BCsort remains competitive even on the micro-scale while absolutely dominating on the macro-scale.

## 👤 Author

**Hédi Ben Chiboub** *Systems Architect & Pragmatic Engineer* 

 [Portfolio](https://benchiboub.com)

## ⚖️ License

Distributed under the MIT License. See `LICENSE` for more information.