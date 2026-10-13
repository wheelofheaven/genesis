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

## Changed-LOD neighborhood checkpoint — 13 October 2026

The management fallback identified above is now removed for same-camera,
terrain-only edits. `terrain/refresh/topology.rs` retains ordered dyadic leaves,
replaces the changed root splits, and triangulates their bounding rectangle plus
one root of shared seams. A second root ring supplies corner membership at the
outer seam boundary. Unchanged triangles are copied in their original order;
only affected geometry samples are evaluated again. A dense remapping pass
compacts sample/index storage, including after repeated coarsening. Vertex storage
indices may change; ordered receiver triangles and complete presentation stay
identical to full reconstruction.

The earlier geometry halo is wider than this seam neighborhood, so it also covers
picker bucket invalidation and terrain/water intersections. Water remains a full
network rebuild when the halo touches a river, otherwise its immutable mesh is
reused. Camera changes, world changes and missing revision chains still reconstruct
the full cache. Distant simultaneous edits can produce a large conservative bounding
rectangle; no optimal sparse partitioning is claimed.

### Before/after evidence

Measured on the same Apple M4/macOS/toolchain/profile and nine-sample fixture as
above. The before run uses the published `579c016` executable, after the baseline
CI completed. The after run uses this checkpoint's executable, after final CI and
build completed. No game window or competing test/build was running during either
benchmark. Both use two warmups and twenty measured trials for each view:

```sh
target/debug/genesis-app --terrain-edit-benchmark /private/tmp/genesis-lod-before.json
# Build the changed-LOD checkpoint, then run after checks finish:
target/debug/genesis-app --terrain-edit-benchmark /private/tmp/genesis-lod-after.json
```

Raw reports: [before](measurements/terrain-lod-before.json),
[after](measurements/terrain-lod-after.json). The existing `--full-rebuild` option
remains available; every local benchmark also compares against fresh full
reconstruction and oblique picks outside timings.

| View | Before total median / p95 | After total median / p95 | Cache median before → after | Picker median before → after |
| --- | ---: | ---: | ---: | ---: |
| region | 188.705 / 194.279 ms | 186.496 / 194.272 ms | 126.950 → 124.730 ms | 0.681 → 0.640 ms |
| management | 433.081 / 470.349 ms | 279.629 / 299.686 ms | 331.877 → 180.049 ms | 6.391 → 3.077 ms |
| inspection | 359.263 / 437.915 ms | 318.159 / 324.704 ms | 228.067 → 196.574 ms | 5.298 → 4.722 ms |

Management median total preparation is **35.4% lower** and its p95 is 36.3% lower.
It now reports `local_refresh: true`; all three views do. Regional and inspection
views already reused topology before this change. Their timing differences are
reported but are not attributed to the new seam algorithm; sequential runs do not
isolate system/thermal variation, and the earlier inspection run has a larger tail.
The optimization claim concerns the changed-LOD management case.

All scene vertex fingerprints match `579c016`, the earlier full reference and
fresh full reconstruction: `f79d70c478f0d1cd`, `e39eb61b058c1437`,
`5ecdfd077487af73`. Prepared chunk counts and predicted upload bytes are unchanged.
The new diagnostic additionally records `retained_lod_bytes`: ordered-leaf
allocations are 1,572,864 / 5,866,920 / 6,291,456 bytes for the three edited caches.
These are additional retained allocations, not total or peak cache memory. The
rebuild temporarily holds a remapping array, local point/corner tables and new
flat buffers alongside the old immutable cache. Repeated edits compact live
samples; the test compares their count to a fresh full build after each edit.

The result still exceeds a 16.7 ms interactive frame budget. Sampling and
triangulation are scoped, but flat-buffer remapping, horizon reconstruction,
scene recoloring/extraction and partitioning still do global work. No GPU traffic,
FPS or end-to-end input latency improvement is claimed.

### Validation and compatibility

Baseline CI: 184 tests/doctests pass. Final `mise run ci`: 185 pass with 14 optional
app tests ignored, formatting and warning-free Clippy pass; `mise run build` passes.
Tests compare exact sample values, ordered receiver triangles, water and full scene
vertices, ground cover and picker buckets/rays. Repeated lowering/raising exercises
refinement/coarsening beside water, a wrapped boundary and a picker bucket seam,
including changed triangulation in roots whose elevations did not change. An early
test fixture failed its seam-coverage assertion because it did not force that case;
it was corrected to start coarse and refine locally. No baseline defect was found.

Domain rules, command/event/save schemas, content, dependencies and supported
platforms are unchanged. Native check results are recorded in the refactor log.
Next measured opportunity: cache the unchanged horizon geometry while refreshing
its environmental palette, then reduce global extraction work. A living-settlement
feature remains separate from this renderer optimization.

## Horizon geometry and palette checkpoint — 13 October 2026

Fresh baseline: clean `4530d66`, same machine, toolchain, profile, seed, command and
three fixed cameras described above. Runs were sequential with no competing build,
test or Genesis window. Each of four runs contains two warmups and twenty trials
per view. Commands before and after the change:

```sh
target/debug/genesis-app --terrain-edit-benchmark /private/tmp/genesis-extraction-before.json
target/debug/genesis-app --terrain-edit-benchmark /private/tmp/genesis-extraction-before-full.json --full-rebuild
mise run build
target/debug/genesis-app --terrain-edit-benchmark /private/tmp/genesis-extraction-after.json
target/debug/genesis-app --terrain-edit-benchmark /private/tmp/genesis-extraction-after-full.json --full-rebuild
```

Raw reports: [scoped before](measurements/terrain-appearance-before.json),
[scoped after](measurements/terrain-appearance-after.json),
[full before](measurements/terrain-appearance-before-full.json),
[full after](measurements/terrain-appearance-after-full.json).

| View | Scoped total median before → after | Reduction | Scoped total p95 before → after | Full rebuild median before → after |
| --- | ---: | ---: | ---: | ---: |
| region | 176.556 → 133.398 ms | 24.4% | 183.901 → 137.136 ms | 262.974 → 257.003 ms |
| management | 269.561 → 223.459 ms | 17.1% | 272.550 → 229.486 ms | 419.271 → 410.530 ms |
| inspection | 316.474 → 267.347 ms | 15.5% | 328.268 → 280.150 ms | 530.660 → 525.569 ms |

Full-rebuild p95: 265.640 → 264.850, 427.969 → 413.553, and
535.420 → 531.803 ms respectively. These small fallback differences are not an
isolated algorithmic speedup claim. The important comparison is the scoped total:
normal terrain coloring now happens in cache refresh, so the reduced `scene`
timer alone would exaggerate the benefit. No FPS or display-latency claim.

`horizon/geometry.rs` stores positions, normals, seam color recipes, ordered
triangles and water independently of appearance. Eligible bounded interior edits
reuse that immutable geometry. Edge edits, wrapped worlds and changed horizon
parameters reconstruct it. Horizon ground is retained in immutable pages of at
most 2,048 triangles; unchanged pages are shared. Interpolated seam colors retain
the original operation order rather than being sampled at a new location.

`terrain/appearance.rs` compares exact plants/moisture bits and expands changes by
the B-spline interpolation footprint, including clamped and wrapped boundaries.
Sea level, drought and rock changes conservatively invalidate all colors. Changed
surface geometry and new LOD samples also recolor. A temporary source-index map
preserves palette correspondence when LOD storage is compacted. Survey colors
still use the complete extractor; ordinary surface extraction checks current
palette inputs even when geometry is reused across ticks.

The ordinary tick changes many cells outside the brush. It correctly recolors
77,275 / 123,585 regional samples, 201,794 / 248,104 management samples and
260,753 / 307,063 inspection samples, and rebuilds 49 horizon ground pages.
This is intentionally broader than the nine changed elevation samples. All views
reuse horizon geometry. Pure interior geometry edits with unchanged environmental
inputs retain every horizon ground page in the regression test.

Every trial retains the prior mesh fingerprints (`f79d70c478f0d1cd`,
`e39eb61b058c1437`, `5ecdfd077487af73`), vertex counts and upload estimates.
The same-tick control still changes two of 924 chunks; the tick-zero comparison
still changes 550. Rendering bytes and domain behavior are unchanged. The frozen
pre-cache horizon extractor also independently checks exact position/color bits,
including shoreline and seafloor modes.

Memory tradeoff: additional retained terrain palette/input allocations are
4,566,300 / 8,052,832 / 9,703,684 bytes per measured cache. Horizon metadata,
recipes, indices and colors use another 8,872,728 bytes, excluding the pre-existing
expanded ground/water buffers. These are capacity-based auxiliary allocation
counts, not net RSS or peak memory measurements; geometry and unchanged ground
pages share allocations between retained readers. The LOD remap adds a temporary
`usize` per output sample. Input comparisons, node/page checks, flat mesh assembly
and renderer partitioning still scan globally. Ground cover, contacts and affected
river networks retain their existing rebuilding rules.

The worker's existing bounded slots and revision/world guards remain unchanged.
Ordinary no-edit ticks retain the prior horizon palette until cache refresh, as
before; this slice preserves that behavior. Updating horizon colors on every
ordinary tick is a separate visual-behavior change. Surveys and camera/world
changes retain full reconstruction paths. Next profile local-refresh stages
(especially cover/contact/rivers and remaining global scans) before choosing a
further optimization. A broader incremental scene stream is not implemented here.
