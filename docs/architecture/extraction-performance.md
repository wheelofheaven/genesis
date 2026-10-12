# CPU scene extraction checkpoint

Measured 12 October 2026. This slice narrows terrain's read-only rock query;
it does not change simulation rules, storage, scheduling, geometry or quality.

## Workload and method

- Apple M4, 16 GB, macOS 26.5.1; aarch64-apple-darwin, Rust 1.93.1.
- `mise run build`, default development profile: workspace opt-level 1,
  dependency opt-level 2. **Release performance was not measured.**
- Existing dirty working tree based on `c055969d47663308a5452e32188843ae700c4279`,
  including the preceding architecture checkpoints and owner's renderer work.
  Dependencies unchanged: wgpu 27.0.1, winit 0.30.13, glam 0.30.10.
- Generated Levant coast, seed 7319, generator 9, content `genesis:builtin/1`,
  384 × 320 samples, six dwellings, five paths, no wells. Zero logical ticks,
  no commands, no represented human workers or invented economic workload.
- Two fixed camera samples: regional home and ground-level beside the first
  dwelling. LOD keys `[156,127,185]` and `[12,0,-114]`; exact eyes in JSON.
  Surface and water survey (opacity 0.8). This is a preparation workload,
  not a moving-camera frame-rate or populated-economy benchmark.
- Two warmups, twenty measured iterations per stage/configuration. Generation,
  destruction, fingerprint checks and report writing are outside stage timers.
  Real worker stage functions run synchronously, without GPU/window initialization,
  worker scheduling, uploads, asset import or simulation advancement. This app
  diagnostic still links graphics libraries; the separate headless runner retains
  its graphics-independent dependency closure.
- Sequential baseline then candidate runs, with no build/test/smoke commands
  running concurrently. Ordinary host scheduling and thermal noise remain.
  Median is the lower middle order statistic; p95 uses nearest rank.

Reproduce the candidate:

```sh
mise run build
target/debug/genesis-app --extraction-benchmark /private/tmp/genesis-extraction.json
```

The baseline used `sim.subsurface.reading(p).rock` in `terrain::colored_vertex`;
the candidate uses `sim.subsurface.rock_at(p)`. For a local comparison, change
only that call back, rebuild and rerun, then restore the candidate. The original
baseline also predates extraction of the unchanged tap-selection expression
into `rock_from_taps`. Reports retain every measured sample:
[before](measurements/extraction-before.json) and
[after](measurements/extraction-after.json).

## Finding and retained change

`reading` interpolates aquifer top, base, capacity, stock, permeability and
recharge and calculates water level, while ordinary terrain shading only needs
rock. `Subsurface::rock_at` shares the exact clamped weighted-tap selection with
`reading`, including its last-equal-weight tie. Terrain uses this narrower query.
No categorical interpolation rule was replaced by coordinate rounding.

Scene extraction times in milliseconds:

| View / survey | Before median / p95 | After median / p95 | Median reduction |
| --- | ---: | ---: | ---: |
| Region / surface | 23.657 / 24.660 | 21.389 / 22.200 | 9.6% |
| Region / water | 29.136 / 29.838 | 26.903 / 28.135 | 7.7% |
| Settlement / surface | 62.102 / 63.115 | 56.738 / 58.805 | 8.6% |
| Settlement / water | 72.152 / 73.019 | 66.370 / 66.801 | 8.0% |

Other stage medians, before → after (milliseconds):

| View | Snapshot | Cold terrain cache | Picker | Surface partition | Water partition |
| --- | ---: | ---: | ---: | ---: | ---: |
| Region | 0.562 → 0.539 | 295.173 → 288.573 | 2.925 → 2.941 | 27.044 → 28.113 | 29.886 → 30.737 |
| Settlement | 0.526 → 0.529 | 490.371 → 485.658 | 8.196 → 7.993 | 47.713 → 47.704 | 50.549 → 50.628 |

Do not attribute unrelated-stage differences to the query change. The small
regional partition increase partially offsets extraction savings. Ground-level
surface extraction plus partition median sums fall from 109.815 to 104.442 ms;
these sums are descriptive, not directly measured end-to-end percentiles.
Cold cache construction remains the largest cost. No FPS, release-build,
GPU, peak-memory or long-session improvement is claimed.

The snapshot costs provide no reason to replace world storage in this slice.
The retained optimization adds one explicit read API and shares a private
selector; it adds no cache, allocation, queue, mutable authority or dependency.

## Preservation and checks

Each repeated extraction checks a deterministic FNV-1a fingerprint over explicit
little-endian position/color float bits, outside timing. This is a diagnostic
presentation fingerprint, not a canonical domain hash or a cryptographic proof.
Before/after vertex counts and fingerprints match in all four configurations:

| View / survey | Vertices | Fingerprint |
| --- | ---: | --- |
| Region / surface | 1,637,667 | `67dafaf8c281fc23` |
| Region / water | 1,770,903 | `5b56901ef7f62c29` |
| Settlement / surface | 2,798,667 | `fdae6ce3da5f9a5c` |
| Settlement / water | 2,931,903 | `1a32c742cae2a536` |

The new unit check covers negative/outside positions, map edges, and weighted
selection ties against explicit expected rock types for both queries.
`mise run ci` passes: 171 tests/doctests, 14 existing optional tests ignored,
formatting and warning-free Clippy. `mise run build` passes.
Native `--survey-test /private/tmp/genesis-extraction-survey.png` exits 0 on
Apple M4/Metal; its seven dwellings, one active borehole, terrain and water/mineral
survey capture were visually inspected. This native fixture complements the
full generated-region CPU benchmark; it is not a render benchmark of that region.

Save schema 2, rules/content identifiers, aquifer quantities and all supported
save migrations remain unchanged. No new platform requirements; browser,
non-macOS native paths, GPU recovery and release timings remain unverified.
The owner's existing renderer/asset work and render benchmark remain intact.

The next performance step, if preparation latency is selected for further work,
is to instrument the cold cache's topology, height/normal sampling, contacts and
cover construction separately before choosing another optimization. No new
threading or simulation representation change is justified by this measurement.
