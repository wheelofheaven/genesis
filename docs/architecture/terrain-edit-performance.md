# Committed terrain-edit preparation baseline

Measured 12 October 2026, after publishing `133d873`.
This checkpoint adds a repeatable diagnostic; it does not change gameplay or claim a speedup.

## Workload and method

Run after building, without competing builds/tests:

```sh
mise run build
target/debug/genesis-app --terrain-edit-benchmark /private/tmp/genesis-terrain-edit.json
```

Machine: Apple M4, 16 GB, macOS 26.5.1, aarch64, Rust 1.93.1, development profile
(workspace optimization 1, dependencies 2), wgpu 27.0.1.
This CPU-only diagnostic opens no GPU or window, though the app package links graphics.
Use `genesis-headless` for the graphics-independent dependency closure.

The generated Levant coast uses seed 7319, generator 9, content `genesis:builtin/1`,
384 × 320 samples, six dwellings and five paths, with no people or economy.
At tick 1 it queues `Terraform` through the real `Session` and steps the paused
simulation once. The brush is centered at the first dwelling's floored position,
radius 2, strength 0.4. At 20 metres per world unit this is a 40 m radius and an
8 m center lift: a terrain-intervention stress fixture, not a canal-digging model.
Circular falloff changes exactly nine environmental samples.
The report records the exact action, camera eyes and LOD keys.

Three fixed views cover regional overview, management and close inspection.
Management uses an eye offset (4, 5, 6) world units from the first dwelling;
inspection uses (0.4, 0.085, 0.4), matching the earlier near-house diagnostic.
Two warmups precede twenty independent trials per view, each starting from the
same world. Setup cloning, validation, report encoding and destruction are outside
the timers. Cache stage hooks are observers and never decide geometry.

The combined `total` timer measures actual elapsed CPU time for the action tick,
cache construction, picker construction, scene extraction and world partitioning.
It is not a sum of independently calculated percentile values.
Per-stage distributions retain median, nearest-rank p95, maximum and all samples.
The median follows the existing extraction benchmark's lower-middle convention.

Every trial verifies the committed tick/revision, consumed intent, nine changed
samples, matching domain event count, picking coordinates and repeatable edited
presentation fingerprints. The edit must differ from both the old presentation
and the same-tick no-edit control. Timing thresholds are not test assertions.

## Upload estimates and control

`PreparedWorld::upload_estimate` compares fingerprints, vertex counts and shadow
counts using the renderer's existing reuse criteria. It reports changed chunk
count and world vertex bytes, assuming the comparison snapshot was fully resident.
Deleting a chunk removes its residency without uploading replacement bytes.
The estimator does not change normal GPU submission.

Two comparisons answer different questions:

- **Previous presentation:** includes ordinary environmental changes in the tick
  as well as the edit and cold-cache refresh, representing replacement of tick-zero
  world geometry.
- **Same-tick rebuilt control:** advances the same world without a command, then
  rebuilds its cache too. This avoids attributing refreshed horizon/environment
  colors to the terrain edit merely because the ordinary no-edit path reuses them.

These are predicted world-vertex uploads, not observed bus traffic or a complete
upload budget. Instance/contact publication, GPU allocation/submission, UI,
worker scheduling, wall-clock pacing and input-to-display latency are excluded.
The baseline samples one action/location and fixed camera keys, not a moving-camera
performance certification. Changes to the model or camera require a new report.

## Results

Results are in [the raw baseline](measurements/terrain-edit-baseline.json).

| View | Total median / p95 | Cache median | Tick median | Edit-only chunks | Edit-only vertex bytes |
| --- | ---: | ---: | ---: | ---: | ---: |
| region | 261.447 / 269.065 ms | 201.775 ms | 5.475 ms | 2 / 924 | 1,719,396 |
| management | 435.897 / 509.933 ms | 327.269 ms | 5.515 ms | 2 / 924 | 8,161,860 |
| inspection | 526.035 / 533.583 ms | 403.768 ms | 5.453 ms | 2 / 924 | 19,982,676 |

A nine-sample edit changes only two of 924 prepared world chunks relative to the
rebuilt same-tick control, yet the entire CPU cache is constructed again.
The normal tick itself is roughly 5.5 ms here; cache construction dominates.
Against the actual tick-zero presentation, all three views predict 550 changed
chunks, including broad environmental recoloring and cache refresh, with roughly
23.5 / 45.6 / 56.0 MB of world vertices respectively.
Therefore local terrain geometry and frequently changing appearance need separate
invalidation policies; local mesh work alone will not eliminate all uploads.

Management-view tail timings are worse than its median; raw samples retain that
variation. These observations are a baseline on one development machine and one
edit fixture, not an improvement or a release-performance guarantee.

## Validation and next slice

`mise run ci` passes 178 tests/doctests (14 optional app tests ignored), formatting
and warning-free Clippy. `mise run build` passes. The new renderer regression
covers unchanged reuse, color edits, new chunks and removals.
The diagnostic itself validates actual committed edits in all 66 trials.
The six native GPU regressions passed immediately before this diagnostic-only
slice; rendering/host behavior was not changed or newly claimed as optimized.

No save, content, rule, dependency or platform compatibility changes.
Next carry changed sample bounds from committed terrain commands into scoped
cache rebuilding. Include interpolation/normal halos, topology transitions,
river/shore geometry and picking dependencies. Compare against this full-rebuild
reference and retain revision guards; removing work without matching geometry
and interaction behavior is not an accepted optimization.
