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

## Local refresh checkpoint — 13 October 2026

The app now defaults to scoped cache refresh for complete terrain-only revision
ranges at the same camera key. The simulation's committed `TerrainChanged` fact
carries inclusive canonical sample bounds. The session retains 32 changes,
separately from its eight-entry HUD log. A missed revision, construction, well
installation, world replacement or camera-key change requires full reconstruction.
No preview or rejected command supplies invalidation bounds.

For an eligible edit, compare leaf splits only at roots whose elevation changed.
Identical splits preserve global seams, triangle ordering and receiver IDs, allowing
immutable topology reuse. Height/normal samples outside a conservative B-spline,
normal-offset and channel-projection halo are copied without resampling. Wrapped
halos also cover the opposite edge. Picking rebuilds only intersecting 16-unit
buckets and shares the other immutable buckets. If any split changes, rebuild the
whole cache and picker; local seam reconstruction is deferred.

River geometry is reused only when the halo misses every river's expanded bounds.
Otherwise rebuild the whole river network, including confluences and endpoint-based
slopes. Horizon, ground-cover placements and contact fields still rebuild; terrain
and scene colors still read current ecological state. This preserves the full
reference appearance rather than freezing distant environmental colors. Cache,
picker and presentation publication retain the existing revision/epoch guards.

### Same-build comparison

The original October 12 measurements above remain historical. These fresh runs
use the same machine/profile/scenario and identical commands/camera keys, with no
running game or competing builds/tests. Both modes retain the initial cache and
picker, so resident input memory is comparable. Two warmups and twenty timed trials
per view remain unchanged. CPU stage measurements are available for the full build;
the local report records the combined cache stage.

```sh
mise run build
target/debug/genesis-app --terrain-edit-benchmark /private/tmp/genesis-terrain-full.json --full-rebuild
target/debug/genesis-app --terrain-edit-benchmark /private/tmp/genesis-terrain-local.json
```

Raw reports: [full reference](measurements/terrain-edit-full-reference.json) and
[local refresh](measurements/terrain-edit-local.json). Full was run first, local
second; these sequential runs do not isolate thermal/frequency variation.

| View | Full total median / p95 | Local total median / p95 | Cache median full → local | Picker median full → local | Result |
| --- | ---: | ---: | ---: | ---: | --- |
| region | 253.305 / 256.169 ms | 184.111 / 186.422 ms | 194.479 → 125.382 ms | 3.163 → 0.640 ms | 27.3% lower median total |
| management | 406.581 / 412.304 ms | 420.088 / 427.362 ms | 309.965 → 321.049 ms | 6.204 → 6.415 ms | full fallback; 3.3% higher median total |
| inspection | 514.929 / 520.931 ms | 324.873 / 332.286 ms | 392.064 → 201.246 ms | 8.424 → 4.962 ms | 36.9% lower median total |

The management edit crosses a topology threshold. It performs the split check plus
a full reconstruction; no improvement is claimed for that case. The observed
13.5 ms difference includes run variation and is not an isolated measurement of
split-check overhead. Regional and inspection gains justify retaining the scoped
path, with this fallback limitation explicit.

All three complete vertex fingerprints match the original baseline and the new
full reference: `f79d70c478f0d1cd`, `e39eb61b058c1437`, `5ecdfd077487af73`.
The local diagnostic additionally compares against a fresh full mesh and 64 oblique
picker queries per view outside timers; subsequent trials require the same hash.
Prepared chunk counts and upload estimates are unchanged. This is a CPU preparation
improvement, not a measured reduction in GPU traffic or input-to-display latency.
It does not meet a 16.7 ms frame budget for synchronous terrain edits.

### Costs, compatibility and remaining work

Each cache retains one extra elevation float per environmental sample (491,520 bytes
for this world), plus small Arc/bucket bookkeeping. Topology and unaffected picker
buckets can be shared with the previous cache. The existing single background job
slot remains bounded; the new revision history cannot grow past 32 records. Samples
are still scanned/copied and picker triangle indices are still scanned. This is
scoped expensive computation, not a fully partitioned extraction pipeline.

No world/save schema, content ID, rule, dependency or supported-platform changes.
`TerrainChanged` gains a serialized `bounds` field; events are transient and are
not stored in the supported saves. Consumers constructing/deserializing that event
must use the new shape. Persistent command/save fixtures remain supported.

Next useful slice: preserve local refresh across changed LOD roots by reconstructing
the affected seam neighborhood, with exact full-reference comparisons. Separately,
horizon/color invalidation and global scene extraction still account for substantial
cost; reducing their work must preserve environmental appearance.

Validation for this checkpoint: `mise run ci` passes 184 tests/doctests (14 optional
app tests ignored), formatting and warning-free Clippy; `mise run build` passes.
Native world, interaction and save checks exit 0 on Apple M4/Metal, with captures
visually inspected. Exact commands and preserved paths are recorded in
[the refactor log](refactor-log.md).
