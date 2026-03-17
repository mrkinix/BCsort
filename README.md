# BCsort (Ben Chiboub Sort)

**BCsort** is a high-performance, in-place ternary distribution sorting algorithm for Rust, created by **Hédi Ben Chiboub**. It is designed around a single core idea: extract the next level's pivot statistics during the same memory pass that performs the current partition, so the algorithm never reads the data a second time just to compute pivots.

## The Memory Wall Problem

Modern CPUs can execute arithmetic far faster than RAM can supply data. A sorting algorithm that makes multiple sequential passes over an array pays the full DRAM latency cost each time. Standard dual-pivot quicksort reads elements to compare them, then reads again to find the next pivot. Radix sort avoids comparisons but requires an O(N) auxiliary buffer, doubling the memory traffic and evicting working data from cache.

BCsort attacks this differently. The partition loop does two things at once: it physically moves elements into their buckets (the DNF sweep), and it accumulates the `min`, `max`, and `sum` for each child partition as a byproduct of reading each element. By the time the partition finishes, the next level's pivots are already known — no second pass, no extra reads.

The tradeoff is a few extra FPU instructions per element (comparisons and additions for the in-flight accumulators). On hardware where DRAM latency dominates, those instructions execute in the latency shadow of the memory fetch that was happening anyway, making them effectively free.

## Design

**Root pass.** A single parallel reduction over the full array establishes `min`, `max`, and `sum`. This is the only dedicated stats pass BCsort performs.

**In-flight accumulation.** The DNF partition loop reads each element once. While deciding which of the three buckets it belongs to, it simultaneously updates the accumulator for that bucket. Each child partition inherits exact `min`, `max`, and `sum` without any additional scan.

**Inherited extrema.** Since `min` is guaranteed to route into the left partition and `max` into the right (both are outside the pivot range by construction), these values are passed directly to child calls rather than re-derived. Only `max_l`, `min_m`, `max_m`, and `min_r` need accumulation.

**Adaptive pivots.** The default pivot formula is `t1 = (min + mean) / 2`, `t2 = (mean + max) / 2`, which bisects the range symmetrically around the mean. On skewed distributions the mean can sit far from the median, producing unbalanced partitions. BCsort tracks this: if any child exceeds 80% of the parent's size, a `bad_splits` counter increments. After three consecutive bad splits on arrays larger than 512 elements, the pivot strategy switches to a 9-element pseudo-random sample sorted by a branchless network, using the 3rd and 6th order statistics as pivots instead. The counter never resets, preventing oscillation on pathological inputs.

**Scale-aware parallelism.** Above `parallel_threshold` (default 10,000) partitions are forked with Rayon. Below it the algorithm drops to single-threaded recursion to avoid thread management overhead on arrays that fit comfortably in L2/L3 cache. The threshold is configurable for different hardware profiles.

**NaN/Inf quarantine.** A single-threaded Hoare-style scan moves non-finite values to the end of the slice before any sorting begins, using a two-pointer approach that skips NaN-to-NaN swaps. Subsequent arithmetic is guaranteed finite. Note: quarantine swaps alter the original positions of affected elements, so callers cannot rely on post-sort indices mapping back to a parallel metadata array.

## Performance

**Hardware:** i7-7900 | 24GB RAM
**Environment:** `cargo run --release` | `RUSTFLAGS="-C target-cpu=native"`
**Data type:** `f64`

### Size sweep — uniform random — avg of 3 runs

| N | BCsort (s) | Rayon par (s) | radsort (s) | vs Rayon | vs radsort |
| :--- | :--- | :--- | :--- | :--- | :--- |
| 1,000 | 0.000118 | 0.000020 | 0.000014 | BC -485% | BC -730% |
| 10,000 | 0.000279 | 0.000095 | 0.000134 | BC -193% | BC -108% |
| 100,000 | 0.001491 | 0.000552 | 0.001561 | BC -170% | BC +5% |
| 1,000,000 | 0.014325 | 0.005696 | 0.017168 | BC -151% | BC +20% |
| 10,000,000 | 0.155212 | 0.074984 | 0.180037 | BC -107% | BC +16% |
| 100,000,000 | 1.781203 | 0.948741 | 1.720640 | BC -88% | BC -4% |

BCsort beats radsort from 100K elements upward. radsort is O(N) but requires an O(N) buffer; as the dataset grows past L3 cache, that buffer becomes a second full DRAM sweep. BCsort's in-place design means only one copy of the data is ever in play.

### Distribution stress — N = 1,000,000 — avg of 3 runs

| Scenario | BCsort (s) | Rayon par (s) | radsort (s) | BC vs best |
| :--- | :--- | :--- | :--- | :--- |
| Uniform | 0.014610 | 0.005696 | 0.017911 | BC -157% |
| Gaussian | 0.015569 | 0.005583 | 0.017281 | BC -179% |
| Pareto (skewed) | 0.015634 | 0.005867 | 0.017967 | BC -166% |
| Nearly sorted | 0.006817 | 0.003280 | 0.016605 | BC -108% |
| 5% NaN | 0.014286 | 0.006362 | 0.018426 | BC -125% |

The Pareto result is notable — heavily skewed data is where mean-based partitioning typically degrades. The adaptive pivot mechanism keeps it within range of the uniform case.

### 10M variance — 5 individual runs, uniform f64

| Run | BCsort (s) | Rayon par (s) | radsort (s) |
| :--- | :--- | :--- | :--- |
| 1 | 0.155752 | 0.077496 | 0.191828 |
| 2 | 0.154245 | 0.073859 | 0.183079 |
| 3 | 0.153109 | 0.068050 | 0.177487 |
| 4 | 0.150885 | 0.066707 | 0.186117 |
| 5 | 0.151118 | 0.066870 | 0.172233 |

BCsort's variance spread is ~5ms across runs. radsort's is ~20ms, likely driven by allocator variance in its auxiliary buffer.

## Usage
```rust
use bcsort::Bcsort;

let mut data: Vec<f64> = vec![3.0, 1.0, 4.0, 1.0, 5.0, 9.0];
data.bcsort();

// Custom parallelism threshold
use bcsort::BcsortConfig;
let config = BcsortConfig { parallel_threshold: 50_000 };
data.bcsort_with_config(&config);
```

Supported types: `f32`, `f64`.

NaN and Inf values are moved to the end of the slice in unspecified order. All finite values are sorted correctly.

## Author

**Hédi Ben Chiboub** — [benchiboub.com](https://benchiboub.com)

## License

MIT. See `LICENSE`.