# Architecture assessment — 12 October 2026

## Scope and baseline

This is a source audit of `main` at `c055969d47663308a5452e32188843ae700c4279`, including the owner's uncommitted rendering and asset work.
It precedes the first implementation slice; the outcome is recorded in [the refactor log](refactor-log.md).
Instructions inspected: workspace and repository `AGENTS.md`, `.claude/CLAUDE.md`, workspace `.claude` overview/conventions/project context, `docs/engine-decision.md`, and `docs/vision.md`.
The accepted custom Rust/wgpu decision supersedes historical Bevy requirements.
No engine or dependency migration is needed.

The working tree already contains changes to 23 tracked files and untracked dwelling assets/import, rendering benchmark, contact/sky code, asset documentation, and Python authoring scripts.
Preserve these; do not stage or commit them with this slice.
Checkpoints here are focused diffs and documented validation, following the repository's conventional-commit practice if a commit is subsequently requested.
Generated Cargo output belongs in ignored `target/`; smoke captures/logs belong in `/private/tmp`.

## Dependency and ownership map

| Boundary | Existing owner and evidence |
| --- | --- |
| Workspace/toolchain | `Cargo.toml`: app, renderer, sim_core; app is default member; legacy and engine-lab excluded. `mise.toml`: Rust 1.93.1, edition 2024, native development/test/build tasks. No main-workspace CI workflow or target-specific build script found. |
| Simulation | `crates/sim_core/src/lib.rs`: `SimState`, action execution, ordered rule stages; depends only on serde 1.0.228 and rand 0.8.5. CPU-only, no authoritative GPU solver. |
| Domain stores | `grid.rs`: contiguous environmental samples; `settlement.rs`: continuous buildings/paths; `subsurface.rs`: columns, deposits, wells; `worldgen/`: region generation and geography. `catalog.rs` and `ecology.rs`: compiled planet/scenario/ecological definitions. No ECS. |
| Interactive owner | `crates/app/src/session.rs`: owns one `SimState`, ordered pending actions and an eight-event UI log; `clock.rs`: host pacing, 100 ms ticks. `main.rs`: winit application lifecycle/input/redraw and shutdown. |
| Extraction/client | `scene.rs`, `terrain/`, `landscape/`, `water/`, `interaction.rs`, `ui/`: immutable simulation reads mapped into geometry, picking and inspectors. `view/preparation.rs`: immutable cloned input for disposable background terrain work. |
| Renderer/host | `crates/renderer/`: GPU resources and explicit render passes, no sim_core dependency. `crates/app/src/view.rs`: surface creation/acquisition/configuration, window and egui composition. |
| Pinned graphics | `Cargo.lock`: wgpu 27.0.1, winit 0.30.13, glam 0.30.10, egui/egui-wgpu/egui-winit 0.33.3. Existing owner work adds gltf 1.4.1; image 0.25.9 and serde_json 1.0.151 already resolve. No upgrades proposed. |
| Assets | Procedural terrain/scenery plus owner's dwelling GLB/PNG pipeline in `crates/app/src/dwelling.rs`, `crates/renderer/src/assets/`, `scripts/build_dwelling.py`, `assets/models/`; F6 visual reload retains the previous asset on failure. |
| Storage | `frontend/settings.rs`: preferences parsed with defaults, temporary file then rename. No world save/load, schema, compatibility fixtures, or save promises implemented. Serde derives on component records are not a complete save format. |

## Feature preservation ledger

“Working” below means supported by the existing source/tests; native visual verification is recorded separately rather than inferred from compilation.

| Feature | Baseline status | Evidence/reproduction |
| --- | --- | --- |
| Camera/controls | Working | Camera/input tests; native `--smoke-test` route exercises orbit, pan, zoom and focus behavior. |
| Generated worlds | Working procedural implementation | `worldgen::generate_scenario`, recipe 9, default seed 7319, 384 × 320 field; bounded coast/highlands, wrapped island; generated regions remain synthetic. |
| Selection/picking | Working | CPU terrain picker and stable building-ID lookup; paths use vector indices; interaction/picking tests. |
| Placement/paths | Working | Preview plus execution validation, rotated continuous footprints, path construction and clearance; ordered duplicate-placement test. |
| Construction | Partial | Accepted buildings appear immediately; no materials, worker progress, cancellation or demolition mechanics. |
| Resources | Partial | Conservative aquifer withdrawal, prescribed recharge and recovered water; research proxy from legacy outposts. No inventories, transfers between households, production or trade. |
| People | Intentionally absent at this checkpoint | Dwellings are not simulated inhabitants; no jobs, needs or navigation agents. Future product priority. |
| Environment | Partial | Deterministic climate/life proxies, sampled physical terrain and generated drainage. Rivers/drainage are generation snapshots; terrain edits do not recompute hydrological connectivity. Visual weather controls do not drive domain rain. |
| Interface | Working prototype | Menu/expedition, settings, pause/speed/single-step, survey, placement and HUD tests; UI log retains eight events. |
| Assets/rendering | Working evolving implementation | Existing procedural meshes plus uncommitted imported dwelling work; ignored GPU tests require explicit native execution. |
| Persistence | Intentionally absent for worlds | Continue retains the live session only. Preferences have load/save tests, not world saves. |
| Platforms | macOS verified baseline | Apple M4/Metal, macOS 26.5.1, 16 GB, aarch64-apple-darwin, Rust 1.93.1. Browser/console/XR are unimplemented host paths; Linux/Windows not verified here. Legacy remains excluded and unbuilt. |

## Traced paths and findings

Building input flows through `Session::build_hovered` → `queue(Action::PlaceBuilding)` → `Session::tick` → `SimState::step` → `validate_building` → building/cover commit → `BuildingPlaced` or `BuildingRejected` → session selection/message/dirty state → scene/instance extraction in `View`.
Execution revalidates occupancy after each preceding action; a preview cannot bypass it.
No resource debit is involved because construction materials are absent.

Terraform input queues `Action::Terraform`; `rules/terraform.rs` applies finite bounded elevation changes, then `step` updates sea-depth/climate/life and subsurface behavior.
The session increments its terrain revision, and extraction/picking refresh from the committed terrain.
Underground datum stays fixed; existing buildings and generated rivers are not repositioned/re-solved as coupled terrain consequences.
Those are existing model limitations, not regressions to repair during this extraction.

The implemented resource transfer is aquifer → recovered well water in `Subsurface::step`.
Recharge occurs first; wells then withdraw in vector order through sampled taps, bounded by stored volume, and increment `Well::recovered` from the actual extracted amount.
The conservation test disables recharge and compares total stock loss with recovery.
There is no household resource transfer or world save/load cycle to trace.
The separate preferences cycle is `Settings::load/parse` → UI apply → `save/encode` → temporary write/rename; it does not contain simulation data.

Confirmed architecture gaps and retained strengths:

- World advancement is already isolated. The missing reusable piece is the running-world boundary: the app owns tick policy, mutable public state, and pending input; the existing scale-audit example only generates worlds.
- The app uses fixed 0.1-second steps, while `SimState::step(dt, actions)` and mutable state remain suitable for fixture/generator setup. `time: f64` is the only running-world time record; `SimConfig::tick_dt` does not control the app clock. Preserve numerical dt/order and add an integer boundary counter without rewriting rules.
- Building/well IDs use length + 1; path references are vector indices. Currently append-only, so no demonstrated runtime deletion bug; future deletion/repacking/saves must first establish durable allocation/references.
- Production UI operations inspected enqueue actions; broad mutable access is a capability risk, not evidence that the UI currently implements a second rules engine. A read-only running-world view can enforce the existing intent.
- `View` already separates immutable scene extraction and GPU submission. One scene request is held at a time; its worker sends one result, tagged by terrain revision, camera bucket, survey and edit epoch. Tests reject obsolete edits/views. Retain this working equivalent; no new job system or delta protocol is justified.
- Extraction clones a whole `SimState` for a dispatched worker, not every frame; reuse/throttling already exist. No performance change is justified without measurement.
- Resize skips zero-sized surfaces; lost/outdated surfaces reconfigure, timeout skips the frame. Full device-loss reconstruction is not implemented or verified. Preserve current paths; don't combine lifecycle redesign with simulation extraction.
- Current clock caps wall input at 250 ms and eight ticks per frame; dropped wall-time debt slows simulation, never skips committed logical steps. Hidden windows suspend it. Preserve this documented pacing behavior; no offline progress is promised.

Coordinates retain x/z horizontal, y vertical, world units = 20 metres, rotations in radians, bounded continental region edges and wrapped legacy/island samples (`units.rs`, `spatial.rs`, `surface.rs`).
Time and resource volumes remain prototype units; don't reinterpret them as calibrated SI models.
Rules execute serially: ordered actions → water depth → climate → life → subsurface recharge/withdrawal → research → stability/events → time.
Later stages see earlier writes; generation uses zero-duration steps to initialize derived fields and place fixtures.
Generation randomness comes from explicit seed/noise inputs; ongoing rules have no worker-timed randomness.
Determinism here means the same build, target, initial state and tick-assigned action order; no cross-architecture or future-version replay promise.

## Validation baseline

- `mise run ci`: passed, including formatting, warning-free Clippy and 143 tests (14 ignored). There was a temporary pause before renderer test output; the original process eventually completed successfully in 75.60 s.
- `mise run test` outside the sandbox: also passed; exact suite totals and native smoke results are recorded in the refactor log. No source failure was found.
- `mise run build`: passes (existing incremental cache).
- Native `target/debug/genesis-app --smoke-test /private/tmp/genesis-architecture-before.png`: passed on Metal; capture visually inspected. The checkpoint log records the repeated post-change check.
- No known failing assertion found. Ignored tests and device loss/browser/legacy execution are not claimed as tested.

## Retained scenario and first slices

Use the existing `scenarios::coastal_outpost` for the fast gameplay regression: 48 × 40, seed 42, geography version 0, six dwellings and an authored path, no people.
Run 100 fixed ticks, with terraform at tick 1, a dwelling and duplicate request at tick 2, drilling at tick 3, stop pumping at tick 50; store commands with explicit ticks and file order.
Native visual route is the existing `--smoke-test` (scripted camera/pause/terraform/build/zero-size/resize), dev profile, 1280 × 800 startup then 760 × 540 logical pixels, current default quality and existing assets.
Also exercise generated coast seed 7319, recipe 9, through the CPU runner; this is environmental/settlement work, not a populated economy benchmark.

1. **Adopted slice:** add a small running-world owner in sim_core, with immutable reads, the existing fixed dt and integer completed ticks; wire Session to it and add a CPU-only runner with scenario/seed/ticks, assigned-tick action playback and metrics. Preserve `SimState::step` for setup/legacy consumers. Acceptance: original numerical state/events match, same-time conflicts retain order, frame partitions match headless execution, app/native smoke and normal checks pass, runner dependency tree excludes graphical/Bevy crates.
2. **Next:** protect world meaning with a modest versioned save record and staged validation before ID/deletion/storage changes. Acceptance: world round-trip plus future behavior, failed loads retain the active world, allocator/reference policy explicit. This slice does not introduce saves or claim save compatibility.
3. **Later:** close demonstrated command/terrain consistency gaps and profile worker snapshots on the existing generated scenario before changing extraction. Acceptance: narrow invariant tests and comparable timings for any optimization. Keep existing renderer and worker validation.

No new gameplay, scheduler, ECS, hydrology, threading, or performance optimization is part of the adopted slice.
Content IDs/manifests and non-macOS hosting remain follow-up decisions when their corresponding work is selected.

## Second slice: persistence audit, 12 October 2026

The first checkpoint's 147 tests/doctests, Clippy and formatting were rerun with `mise run ci` and pass before persistence edits.
There is still no world save to migrate; preferences are separate.
`SimState` holds all current consequential state, with no in-flight authoritative jobs or ongoing random generator beyond the stored seed.
Pending app commands must be saved separately because they have not yet reached `Simulation::advance`.
Channel bins and river bounding boxes are reconstructible; environmental proxy fields are retained exactly to avoid recomputation changing the saved boundary.
The sub-metre surface still uses the compiled noise/geometry implementation; compatibility must therefore identify rules/content, not just generator seed.

Adopt schema v1, a shared CPU-only host storage crate, headless load/save and app quicksave/quickload.
Validate staged records, dimensions, finite/ranged values, IDs, references and bounded counts before cache reconstruction or replacing a session.
Retain append-only building/well IDs and path vector order; explicitly validate their existing allocator assumptions rather than adding deletion or repacking in this slice.
Persist a host-assigned world identifier independently of the scenario seed, and pending actions without executing them for saving.
Use sibling temporary files and a prior-file backup for native commits.
Acceptance: exact continuation across a save with pumping/pending placement, malformed loads preserve active state, failed writes preserve the old file, schema fixture loads, generated region cache queries match, and normal/native checks pass.

## Durable identity slice audit (12 October 2026)

The preceding save checkpoint passed `mise run ci` (153 tests/doc checks,
14 ignored), workspace build, and native save/reload smoke. Source inspection
now finds `SimState::step` assigning buildings `len + 1`, `Subsurface::drill`
assigning wells `len + 1`, and `PathBuilt`/app `Object::Path` carrying vector
indices. Removing the final object would reuse its identity; repacking paths
would redirect selection. Well withdrawal arbitration also follows vector order.
No deletion mechanic is being added. Existing numeric building/well IDs and
all geometry/placement rules will be retained. The next slice persists monotonic
allocators, adds path IDs, resolves presentation references by ID, and preserves
well arbitration in ascending ID order. Save v1 must migrate before replacement;
malformed or inconsistent v2 identity data must fail. Acceptance covers highest-ID
removal, stale references, reordering, allocation exhaustion, and continued
creation after save/load, plus the existing native interaction/save smoke.

## Terrain command audit, 12 October 2026

The durable-identity baseline passed 159 tests/doctests, Clippy, formatting,
workspace build, and native interaction/save checks. Before terrain edits,
inspection traced `rules/terraform.rs` → `SimState::step` → `Session::tick` →
revision-tagged scene preparation. Three concrete gaps are present: brush input
has no structured outcome, bounded-world `cx + dx`/`cy + dy` can overflow for
extreme command coordinates, and the host advances terrain revision from queued
intent even if the command changes nothing. The original brush mutates each cell
as it walks the stencil, preventing validation of the entire patch before commit.

The source also constrains the useful scope. `spatial::sample` uses a 4×4 cubic
B-spline, not a bilinear grid. `surface::vertex_height` derives current ground;
`scene/buildings.rs` intentionally handles exposed foundations after edits.
`surface/channels::level` samples current elevation so channel presentation follows
edits; generated routes and rainfall/drainage remain fixed scenario data, not a
live hydrology solver. Existing footpaths retain centerlines and derive ground
heights. Aquifer formations keep fixed underground elevations; submerged wells
stop pumping. No structural damage, path routing, river rerouting or displaced
water simulation exists. Blanket exclusion zones would change useful sandbox
behavior without an established product rule.

Adopt the smallest command-boundary slice: bounded sparse staging, finite input
and safe coordinate validation, an all-at-once elevation commit, explicit command
outcomes, and host invalidation only on a real commit. Preserve brush falloff,
clamping, wrapping (including repeated visits on small grids), cropped bounded
brushes, normal construction interactions and existing time/rule ordering.
Acceptance includes no partial terrain effects on rejected commands, extreme
coordinates, wrapping and bounded edges, same-tick command ordering, no-op cache
behavior, save continuation and native terrain interaction. Foundation damage
and channel routing policy remain documented follow-ups rather than silently
implemented gameplay changes.

## World replacement and derived-work audit, 12 October 2026

The staged-terrain checkpoint passed 167 tests/doctests, formatting, Clippy,
workspace build, native save smoke and exact fixture-state comparison. Inspection
of `view.rs`, `view/preparation.rs`, `Session::load_from/regenerate`, frontend
loading and executable setup finds a working single-request design: workers own
cloned immutable input, terrain revision advances on every current replacement,
and edit epochs prevent an older completion overwriting urgent publication.
No stale-world overwrite was reproduced. Geometry, picking, contacts, scenery
and dwelling caches already rebuild when replacement advances terrain revision.

The concrete maintainability gap is duplicated replacement invalidation in load,
regeneration and frontend arrival. Raw completion access and the one-worker slot
are also managed manually in `View`; existing tests check tag comparisons and
mesh generation but do not drive delayed completion across an actual staged
load. Preserve the existing revision as the world/terrain invalidation generation
rather than adding a redundant epoch or replacing the renderer.

The adopted slice centralizes replacement, puts admission and completion
validation behind one small preparation owner, and uses a capacity-one result
channel. Acceptance: a delayed old-world result cannot publish after loading the
same save, repeated replacements cannot create a worker backlog, pending failed
loads retain the valid world/work identity, and the next admitted result rebuilds
matching terrain and picking. Native load/regeneration routes and existing
camera/survey/edit guards remain usable. No scheduling or domain rule change is
needed.

## CPU extraction audit — 12 October 2026

`view/preparation.rs` clones `SimState` only on successful worker admission,
then builds `terrain::Cache`, optional `terrain::Picker`, `scene::mesh`, and
`PreparedWorld`. Cache reuse skips geometry and picking but still recolors the
scene and partitions vertices. One outstanding request already bounds this work.
There is no evidence yet that changing simulation storage or adding threading
would improve this path.

`terrain::colored_vertex` requests a full subsurface `Reading` for its `rock`
field. That query also interpolates six numerical fields and derives a water
level. This is an avoidable-work hypothesis, subject to compiler optimization
and measurement. Preserve the exact weighted-tap rock selection and tie behavior
if a narrower query proves useful. No domain rule or save change is required.

The existing `benchmark.rs` measures rendering and bundled initial preparation.
Retain it. A separate `--extraction-benchmark` diagnostic measures the existing
worker stages with a generated coast (seed 7319), region and settlement LOD,
surface and water surveys. It runs CPU code without opening graphics, but belongs
to the graphical app package; it is not the graphics-independent headless target.
The workload contains existing dwellings/paths/environment, no invented people
or economy. Require comparable timings and identical presentation vertex bits
before retaining an optimization. Snapshot representation, scheduling and
rendering quality remain unchanged in this slice.

## Cold terrain-cache audit — 12 October 2026

The preceding measured checkpoint found cold `terrain::Cache::at` construction
at roughly 290 ms (regional camera) and 490 ms (settlement camera) in the native
development profile. Its stages are adaptive topology, heights/normals, ground
cover, contact baking, horizon context and rivers. Before selecting another
optimization, extend the existing extraction diagnostic to time these actual
stages. Retain the same seeded workload and compare presentation fingerprints.
Timing must remain a host-side observer; it must never affect simulation or
choose LOD, geometry, scheduling or cache contents. No new threading or authority
is needed. Keep the existing dirty renderer/assets untouched.

The staged baseline identifies river geometry as the largest component: median
167.172 ms regionally and 190.444 ms near the settlement. Source tracing from
`water::rivers` through `Sheet::add` reaches `Cache::indexed_triangles_in`, which
queries every finest-lattice cell in a ribbon triangle's bounding box and then
deduplicates coarse patches. Adopt a local traversal change: skip horizontally
to each patch edge and vertically to the earliest edge found across that row.
Keep the same receiver IDs, sorted visitation, inclusive bounds and negative-edge
clamping. No spatial index, cache retention or water algorithm change is needed.
Verify against the old exhaustive traversal at mixed LOD and map boundaries,
then compare the full generated mesh fingerprints and timings.

## Continuation audit: life-seeding command boundary, 12 October 2026

Re-inspected workspace/repository instructions, engine decision, vision, manifests,
existing architecture records, fixed runtime, action execution, terrain brush,
save validation/storage, playback, host clock and session feedback.
The owner brief already exists in full at `GENESIS_ENGINE_ARCHITECTURE.md`; retain it.
Current `main` remains `c055969d47663308a5452e32188843ae700c4279`, with 82 dirty/untracked
status entries at entry, including all six previous checkpoints and asset work.
These are the baseline, not new changes in this continuation. No staging or commit.

Current ownership supersedes the initial historical table above: five workspace
packages include CPU-only `headless` and native `storage`; `Session` owns a read-only
`runtime::Simulation`; schema 2 saves support v1 migration and persisted allocators.
The feature ledger otherwise remains applicable: people, staged construction,
inventories/trade and live river routing are absent; world persistence is now working
with staged load and native backup replacement. Existing supported target remains
macOS; other hosts and full GPU recovery are unverified. Dependencies are unchanged.

Fresh `mise run test`, `mise run fmt-check`, and `mise run clippy` pass: 172 tests/doc
checks, 14 optional tests ignored. No baseline assertion or lint failure.
Pinned/active Rust is 1.93.1 on aarch64-apple-darwin. Existing fixture seed 42,
100 ticks and `coastal.commands.json` remain the representative domain workload.

Finding: `SimState::seed_life` adds `cx + dx` and `cy + dy` in i32 before boundary
validation and indexing. Both public seed actions accept arbitrary i32 coordinates
from headless playback; debug builds can panic, while unchecked overflow can choose
the wrong wrapped sample. Terraform's widened addressing already avoids this.
The seed radius is capped by map extent; water eligibility uses the previously
committed depth, and repeated wrapped visits deliberately accumulate coverage.

Adopt a small rule extraction into `rules/seeding.rs`, using widened coordinates,
wrapping before offsets and bounded-region clipping. Retain the 0.3 per-visit
increment, saturation, underwater exclusion, radius cap and existing silent no-op
behavior. Do not redesign all command events or change same-tick water ordering.
Acceptance: reproduce overflow before the fix; cover extreme coordinates on both
topologies, negative/cropped boundaries, repeated visits, saturation and water;
prove pending seed continuation through storage; run established checks and native
smoke. No performance claim, schema/content change or library upgrade.

Next slices remain independent: structured outcomes for legacy seed/outpost actions,
then measured cold-cache preparation or scoped GPU lifecycle recovery if selected.
Absent gameplay systems are outside this refactor.

## Edit-preparation measurement audit — 12 October 2026

Published baseline `133d873` includes the existing stage profiler and adaptive
terrain query optimization. The remaining measured gap is after a real committed
edit: `View::ensure_terrain` rebuilds cache/picker, then urgent drawing extracts
and partitions the scene. Existing diagnostics time initial construction or
resident draws; neither isolates this action-to-prepared-geometry path.

Add a CPU diagnostic using the actual Session queue/single-step and existing
cache/picker/scene/partition stages on the generated coast. Compare both the
previous presentation and a no-command control at the same logical tick, because
ecology changes colors independently of terraforming. Estimate chunk upload
bytes from the renderer's fingerprint criteria without opening a GPU. Report
this as an estimate, excluding instance/contact uploads, driver work and display
latency. Preserve rules, frame behavior and schemas. Acceptance: real commit and
revision advancement, repeated presentation fingerprints, exact changed-cell
count, bounded sample count, meaningful chunk-estimate regression, and normal CI.
Use measured results to select the next optimization; do not claim responsiveness
has improved merely because timing infrastructure was added.

## Local terrain refresh audit — 13 October 2026

Baseline is clean `3195938`; normal CI is rerun before edits. Source tracing covers
committed brush staging, Session revisions, adaptive dyadic roots, B-spline height
support, channel incision/endpoint slopes, cosmetic cover, horizon colors and picking.
The critical constraint is that a changed elevation can change the root's LOD
split and therefore neighboring seams. Channel levels sample the projected center;
water shading additionally interpolates endpoint levels. Appearance also changes
with ecology outside the edit.

Adopt committed inclusive sample bounds plus a bounded Session revision history.
If history is missing, contains construction/world replacement, or a camera key
changes, rebuild fully. Verify root split stability against cached elevations;
keep full reconstruction for topology changes. Otherwise reuse topology and
recompute geometry samples only within a conservative interpolation/normal/channel
halo, update spatial picking buckets, and reuse river geometry only when its
entire support is untouched. Rebuild affected river networks conservatively.
Refresh environmental colors, horizon palette, cover and contacts from current
state rather than retaining stale appearance. The slice localizes expensive
surface sampling, not every extraction or upload.

Acceptance: complete mesh/picker equality against full reconstruction across
interior/edge/wrapped/river edits, batches and LOD transitions; missing-history
and replacement fallback; stale-worker protection; comparable committed-edit
measurements with identical fingerprints. No domain rule or save-schema change.

This slice is implemented and measured in the scoped-refresh entry of
[refactor-log.md](refactor-log.md). Full-reference geometry remains exact;
regional/inspection CPU preparation improve, while the topology-changing management
case retains full reconstruction. See [the comparison](terrain-edit-performance.md)
for raw timings, fallback cost, memory tradeoffs and next steps.

## Changed-LOD neighborhood audit — 13 October 2026

Starting from clean `579c016`, the previous local-refresh benchmark identifies
management-view fallback when an edited root changes its dyadic splits. Inspected
`terrain/lod.rs`, `terrain/refresh.rs`, `terrain.rs`, receiver queries and
`water/sheet.rs`: split choice depends on each root's own elevation and camera;
triangulation depends on leaf corner membership, including neighboring roots.
Triangulation does not insert new corners into that membership set, so seam
invalidation needs one root halo, not recursive world-wide propagation.

Adopted slice: retain ordered leaves, replace changed root splits, triangulate the
changed roots and one-root seam neighborhood using corners from a second ring,
and compact/remap the flat vertex/index storage in original triangle order.
Reuse unaffected sample geometry and picker buckets. The existing wider geometry
halo covers seam changes and continues to govern water invalidation. Camera/world
changes retain full reconstruction. Receiver order must remain identical because
water unions and interpolated shading depend on that order.

Acceptance: exact ordered triangles, water/scene vertices and picking against full
reconstruction after refinement and coarsening, boundaries, adjacent edits and
repeated edits; no orphan accumulation; current CI/native paths; comparable
before/after measurements. No rules, save/content formats or dependencies change.
The flat assembly scan remains global and will be reported rather than described
as a fully local pipeline.

Implemented result: management-view preparation now uses local seam reconstruction,
with exact full-reference output and 35.4% lower measured median CPU preparation.
See [the checkpoint log](refactor-log.md) and [timings/memory limits](terrain-edit-performance.md).
The retained ordered leaves cost additional memory, while flat buffer assembly and
other extraction stages remain global. Saves and simulation behavior are unchanged.

## Horizon geometry and appearance extraction audit — 13 October 2026

Starting from clean `4530d66`. Inspected horizon generation, terrain coloring,
local refresh, scene extraction, worker publication and renderer partitioning.
`horizon::Cache::new` recomputes procedural relief/normals and expanded mesh after
every local edit; interior edits cannot affect a bounded region's outer geometry.
`terrain::Cache::mesh` also recolors every sample, even when its plants/moisture
interpolation taps are unchanged. Colors depend on those fields, sea level,
drought, rock, geometry and channel detail; survey layers have additional inputs.
Ordinary ecology may change appearance broadly, so brush bounds alone are unsafe.

Adopted slice: separate immutable horizon geometry/color recipes from its palette;
reuse geometry on eligible interior terrain edits and reconstruct on edge/world
changes. Track exact surface-palette inputs and their interpolation halos, caching
normal surface colors and recomputing affected samples. Surveys retain full color
extraction. Retain worker revision guards and global flat mesh assembly/partitioning.
No claim of a wholly incremental renderer or lower upload traffic. Acceptance is
bit-identical reference output, field/edge/wrap/LOD/survey checks, preserved native
interaction/persistence and before/after measurements, including memory tradeoffs.

Implemented: interior edits share immutable horizon geometry and unaffected ground
pages; normal terrain colors track actual field dependencies and LOD remapping.
Exact reference output and picking are retained. Fresh scoped median CPU preparation
improves 15.5–24.4% across the three recorded views, with additional bounded cache
memory. Full rebuild and ordinary horizon-palette behavior remain compatible.
See [measurements and limits](terrain-edit-performance.md) and the checkpoint log.

## River surface extraction audit — 13 October 2026

Starting from clean `72e65d4`. Read applicable instructions, engine/vision decisions,
previous performance evidence, terrain refresh, ground cover, contacts, river
projection/slope/surface code and receiver queries. Baseline CI passes 189 tests.
Added observational stage hooks to local refresh and its existing benchmark before
selecting an optimization. The unchanged fixed scenario measures management CPU
preparation at 228.081 ms median: river projection 31.468 ms, slope setup 2.710 ms,
final river surface 53.728 ms, terrain geometry 38.894 ms and renderer partition
42.595 ms. Ground cover is inactive at that camera and contact shading is 1.435 ms.

`water/sheet.rs::finish` evaluates physical channel depth at each emitted polygon
vertex. Shared polygon boundaries repeatedly query the same exact coordinates;
`water/surface.rs::channel_depth` performs channel queries and B-spline sampling.
Adopted slice: memoize that pure depth query within one extraction using an exact
coordinate key and a fixed-capacity replacement table. No retained world cache,
quantization, asynchronous work or simulation ownership change. Cache collisions
must only cause recomputation. Preserve polygon order, interpolation, clipping,
materials, geometry and picking. Acceptance: exact cached/uncached output across
river edits, seams and confluences, bounded memory, existing CI/native checks and
comparable before/after stage and total timings. Broader river invalidation and
whole-scene incrementality remain outside this slice.

Implemented and verified on 14 October: river surface extraction is 58.4–60.3%
faster in the three fixed views, with identical output. Total preparation improves
14.5–21.2%; management median is 187.668 ms. CI passes 190 tests and the native
world/interaction/save checks pass. See the [checkpoint report](refactor-log.md)
and [stage timings and limitations](terrain-edit-performance.md). The scratch
cache is bounded and discarded per extraction; broader incremental river work
and a frame-budget editing guarantee are not claimed.

## First residents audit — 15 October 2026

Starting from clean `b636c63`; baseline `mise run ci` passes (190 tests,
14 optional tests ignored). The authoritative `SimState`/`Simulation` already
supports the client and CPU-only runner. `identity.rs` provides persistent
allocators; `surface/walking.rs` supplies continuous obstacle and slope queries.
There are dwellings and paths but no represented humans, needs, jobs or economy.
No ECS replacement is justified by these stores.

Adopted slice: a bounded initial population with typed persistent IDs, home
references, continuous positions and a home/meeting/rest routine. Navigation is
local, deterministic and bounded; movement checks current passability. The app
adds disposable figures, picking and an activity inspector. Existing landscape,
placement, camera, water and save behavior remain acceptance gates. Schema 3
will preserve routine commitments and migrate schemas 1/2 with zero residents;
loading an old save must not silently introduce a population. The existing
unpopulated extraction benchmark remains explicitly unpopulated. No performance,
full economy, skeletal animation or large-population capability is claimed.

Implemented checkpoint: the default new settlement has six persistent residents
with a home/meeting/rest routine, derived figures and an activity inspector.
CI now passes 198 tests; headless save continuation and native resident/world/
interaction/save checks pass. Schema 3 loads V1/V2 worlds without adding people.
The existing camera, terrain, paths, water, boreholes and native save paths remain
usable; the people ledger changes from absent to **partial** (routine only).
Needs, jobs, inventories, households, skeletal animation and other platforms remain
outside this checkpoint. See [the decision and limits](residents.md) and
[checkpoint evidence](refactor-log.md).

## Household water audit — 15 October 2026

Starting from clean `203433b`. Baseline `mise run ci` passes 198 tests, with
14 optional tests ignored. Re-read project/workspace instructions, engine and
vision decisions, resident/save decisions, manifests and locked versions.
`Subsurface::step` removes aquifer water and increments cumulative `Well::recovered`;
there is no bounded well stock, household inventory or collection commitment.
Residents have persistent homes and checked routes, but only a social routine.
The survey already queues drilling and pump-state commands through `SimState::step`.

Adopted slice: explicit bounded well stock with extraction/outflow accounting,
one water store per occupied dwelling, resident-owned collection reservations
and carried water, and transactional pickup/delivery. Preserve the existing
pumping rate and aquifer behavior; excess buffer inflow becomes an explicit
external outflow. Add small fixed-time household use and shortage reporting,
without health/death or a general economy. Reuse serial resident advancement,
local navigation, inspector and shared native/headless save path. Cancelled
collections release claims; carried water remains owned until delivered.
Schema 4 will migrate V1–V3 without inventing water from historical recovery.
Acceptance includes conservation, scarce-stock arbitration, cancellation/stopped
wells, route failures, save continuation with cargo, and native interaction.

Implemented checkpoint: bounded well supply, household stocks/use, reservations,
pickup and delivery are authoritative in `sim_core`. Resource logistics and
households change from absent to **partial** (water only); resident presentation
now exposes cargo, supply and blocking reasons. Pumping and existing terrain,
selection, placement and save behavior remain usable. Schema 4 supports staged
V1–V3 migration; old recovery is accounted as historical outflow.
`mise run ci` passes 206 tests with 14 optional tests ignored; build and native
water/resident/world/interaction/save checks pass on Apple M4/Metal. CPU-only
save continuation matches uninterrupted advancement with cargo in transit.
No new platform or performance claim. Full jobs, health, production and social
household models remain outside the slice. See [ownership and limits](household-water.md)
and [checkpoint evidence](refactor-log.md).
