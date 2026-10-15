# Refactor checkpoint — 12 October 2026

## Outcome and scope

Completed the audit and first coherent migration slice from the [owner's brief](GENESIS_ENGINE_ARCHITECTURE.md).
This is not completion of all architecture phases.
The [source assessment and preservation ledger](current-state.md) identify instructions inspected, ownership, action/terrain/water traces, supported environment and remaining gaps.

Retained the existing simulation, renderer, action rules, storage and generation.
The concrete gap was a reusable running-world boundary and a CPU-only execution tool, not a missing simulation crate.
See the [fixed-tick decision](fixed-tick-runtime.md).

## Changed boundaries

| Files | Change and reason |
| --- | --- |
| `crates/sim_core/src/runtime.rs`, `lib.rs` | `Simulation` owns initialized domain state, exposes immutable queries and fixed 100 ms action advancement with an integer completed-tick counter. No mutable client access. |
| `crates/app/src/session.rs`, `clock.rs` | Session hands state to the shared runtime; pacing imports its tick duration. Pending actions, UI events, selection, pause/speed and dirty/revision handling remain in the app. |
| `crates/headless/` | Separate CPU host accepts scenario, seed, logical tick count and versioned, bounded JSON action files; reports JSON metrics. Same action validation and rules as the app. |
| `crates/sim_core/fixtures/coastal.commands.json` | Repeatable current-game workload: terraform, conflicting placement, drill, stop pumping. No invented people/economy workload. |
| `Cargo.toml`, `Cargo.lock` | Add one executable package; reuse existing serde/serde_json versions. The only new lockfile block is `genesis-headless`; preserve the owner's existing lockfile changes. |
| `README.md`, architecture/headless documentation | Store the supplied brief, source findings, commands, decision and this checkpoint. |

No intended gameplay changes.
The new observable behavior is the headless CLI and integer tick query.
`SimState::step` and mutable setup records remain for generation/tests/legacy; the running-world wrapper enforces the interactive and headless boundary.
Immutable dereferencing lets existing scene extraction and worker cloning continue without renderer edits.

## Verification

Environment: Apple M4 (10 CPU cores), 16 GB, macOS 26.5.1, aarch64-apple-darwin, Rust 1.93.1, existing dev profile; native smoke uses Metal and the pinned wgpu 27.0.1.

| Command/check | Baseline | After slice |
| --- | --- | --- |
| `mise run ci` | Passed: format, Clippy, app 72, renderer 28, sim_core 43 tests; 14 app tests ignored. Temporary delay before renderer output, then exit 0. | Components run individually below. |
| `mise run test` | Native rerun also passed, no failed assertions. | App 72, headless 2, renderer 28, sim_core 44, plus one compile-fail doctest: **147 passed, 14 ignored**. |
| `mise run clippy` | Warning-free through CI. | Passed with `--workspace --all-targets -- -D warnings`. |
| `mise run fmt-check` | Passed through CI. | Passed. |
| `mise run check` | Build/test supplied baseline type checks. | Passed. |
| `mise run build` | Passed. | Passed, all four workspace packages. |
| `target/debug/genesis-app --smoke-test /private/tmp/genesis-architecture-{before,after}.png` (two invocations) | Exit 0, Metal capture inspected. | Exit 0, capture inspected: same terrain, seven dwellings, path, inspector and controls. Script checks camera, pause, queued terraform, step, placement, zero-sized surface and resize. |
| `mise exec -- cargo tree -p genesis-headless --edges normal` | Runner absent. | Only sim_core/serde/serde_json and their CPU dependencies; no renderer, wgpu, winit, GUI or Bevy. |
| `git diff --check` | — | Passed. |

New/extended checks cover:

- 100 ticks produce the same full state and events as the previous `step(0.1, actions)` path, including accepted/rejected overlapping buildings and well withdrawal/stop.
- Interactive 10 ms and 100 ms frame partitions match direct headless stepping with drilling and terraforming.
- A compile-fail doctest prevents mutable grid access through a running `Simulation`.
- Playback retains same-tick order and exact command boundaries, consumes actions once, and rejects invalid scheduling/version/size metadata before running.
- Executable-level checks reject unknown scenarios, negative ticks, `--seed` on the authored fixture, and commands outside the selected run.

The local full-state comparisons use current-build `Debug` equality as a regression oracle; they do not publish hashes or claim a persistent canonical encoding.

CPU runs performed:

```sh
mise exec -- cargo run -p genesis-headless -- --scenario fixture --ticks 100 --commands crates/sim_core/fixtures/coastal.commands.json
target/debug/genesis-headless --scenario coast --seed 7319 --ticks 100
```

The fixture was also repeated through the executable; JSON metrics were byte-identical.
Fixture result: seed 42, recipe 0, 100 ticks, five commands, one explicit rejection, seven dwellings, one path, one well, 8.45999963581562 recovered prototype water units.
Generated coast result: seed 7319, recipe 9, 122,880 cells, 100 ticks, six dwellings, five paths, no wells or command rejections.
These are summaries of actual work, not canonical fingerprints, conservation proofs, or performance benchmarks.

Local evidence is in `/private/tmp/genesis-architecture-*.log`, `*-before.png`, `*-after.png`, `*-fixture.json`, `*-coast.json`, and `*-headless-tree.txt`; these are temporary run artifacts, not committed assets.
There are no failing baseline assertions to attribute to this slice.
Ignored native GPU/timing tests, device loss, browser/console/XR, other desktop targets and the excluded legacy build were not exercised.

## Compatibility, performance and next step

No world save format exists, so no saved world was rewritten or migrated.
Preferences and visual assets are unchanged.
The action log's version 1 describes its envelope only; it does not promise replay under future rules/content/generators, and it is not a save.
Tick zero is the initialized world; a future save API must restore its timeline explicitly.
The old floating time accumulator and all numerical formulas/order remain unchanged.

This is an ownership/testability change, not an optimization.
No before/after throughput, FPS, population-scale or memory improvement is claimed.
The existing worker snapshot copies and bounded presentation pipeline are retained pending profiling.

The original pending input queue remains host-owned, and not every legacy action returns a structured rejection.
Length-derived object IDs/path indices, terrain-edit coupling to generated rivers/building foundations, durable content identity and device recovery remain gaps from the audit.
Existing placement/path/well validation is preserved; this slice is not comprehensive action/import hardening.

The next useful slice is a modest versioned world save record with staged load validation and continuation tests, defining allocator/reference semantics before adding deletion or repacking.
No absent simulation subsystem is required to finish this checkpoint.

No commit or push was made.
Pre-existing tracked and untracked work was left intact; only additive README documentation and the new headless lockfile block overlap files already dirty at entry.

## Second checkpoint — native world persistence

Continued on 12 October 2026 following the owner's instruction.
The first slice's `mise run ci` baseline still passed: 147 tests/doctests, 14 ignored, formatting and warning-free Clippy.
The [persistence audit](current-state.md#second-slice-persistence-audit-12-october-2026) preceded substantial edits; the [v1 save decision](saves-v1.md) documents format, units/identity assumptions, budgets and platform limits.

### Changes and retained behavior

- `sim_core/src/save.rs`, `save/validation.rs`, and serde on `SimState` define schema/rules/content metadata, world identity, tick and pending-action snapshots, validation and staged restoration.
- `worldgen/mod.rs` and `worldgen/drainage.rs` omit channel bins and river bounds from serialized data; restoration rebuilds them. `Subsurface` exposes internal dimensions to validation. All consequential field values, object order and current IDs are retained.
- New CPU-only `crates/storage` shares JSON encoding, bounded reads and recoverable writes between the two hosts. It reuses rand/serde_json; no library version upgrades. `float_roundtrip` is enabled for exact floating decode.
- `Session`, app input/help, `persistence.rs` and `save_smoke.rs` add F5 quicksave/F9 quickload and native persistence verification. Successful loads pause, clear obsolete UI state and advance presentation revision; failed loads preserve the active world. Main-menu Continue still resumes the live session.
- Headless `--load`/`--save` support continuation and enforce absolute action-log ticks after the saved boundary. Saved pending intent executes once at the next tick before newly supplied intent.

No ecological, placement, water, clock or generation formulas changed.
World saving does not process pending commands or advance time.
IDs and vectors retain the append-only model; deletion/repacking is not implemented by this slice.
Random host world IDs are metadata, not rule/RNG inputs.

### Evidence

| Check | Result |
| --- | --- |
| `mise run ci` after final code | Passed: app 73, headless 2, renderer 28, sim_core 44, storage 5, one compile-fail doctest = **153 passed**, 14 optional app tests ignored; formatting and warning-free Clippy pass. |
| `mise run build` | All five workspace packages build. |
| `mise exec -- cargo tree -p genesis-headless --edges normal` | Runner/storage remain CPU-only; no wgpu, winit, egui, renderer or Bevy. |
| `target/debug/genesis-app --save-test /private/tmp/genesis-save-native.png` | Exit 0 on Apple M4/Metal. Saves pending drilling into a temporary sibling, reloads the old boundary, resumes exactly, and renders seven dwellings/one borehole. Capture visually inspected; existing pause/camera/resize/placement checks pass. |
| Fixture run 100 ticks vs 50 → save/load → 50 | Exact domain-state JSON equality and tick 100; loaded world ID retained. |
| Old command ticks on a loaded CLI world | Rejected before advancement/write; original save bytes unchanged. |
| Coast, highlands and island seed 7319 | Each saves at tick 2 and loads at zero additional ticks with identical reported domain quantities. Storage tests also verify state equality and reconstructed physical-surface/river queries for all three. |
| Failure injection and compatibility fixture | Corrupt/schema/rule/content/count/numerical data rejected; checked-in v1 loads and advances; backup failure leaves primary untouched and cleans temporary output; previous file remains loadable as `.bak`. |
| App staging test | Saving pending placement does not build it; loading reinstates it and commits once on step; malformed input leaves the current simulation/world ID unchanged. |
| `git diff --check` | Passed. |

During implementation, format checking caught module ordering, corrected before the passing final run.
The initial 64 MiB cap failed a real highlands save, so the bounded cap was raised to 128 MiB and all three current regions were retested.
Actual compact-JSON sizes at seed 7319/tick 2: coast 65,273,761 bytes, highlands 82,902,230 bytes, island 52,845,346 bytes.
These sizes describe current materialized state, not a compression or performance improvement.
Synchronous save/load can pause the UI; latency and peak memory have not been benchmarked, and no throughput claim is made.

Evidence logs/captures are `/private/tmp/genesis-save-*.log`, `/private/tmp/genesis-save-native.png`, `/private/tmp/genesis-save-native.world.json`, and `/private/tmp/genesis-save-check/`.
The regular user quicksave slot was not touched by automated verification.
Device-loss recovery, other desktop targets, browser storage and the excluded legacy build remain unverified.

### Compatibility and next step

This introduces the first world schema; there are no older world saves to migrate.
Rules/content IDs are explicit compatibility markers, not a promise of future-build replay equivalence.
The generated physical surface still needs the compatible compiled rule recipe; modified terrain and other domain fields are materialized.
Future record/ID changes must retain the v1 fixture through migration or explicitly reject incompatible versions.

The next focused architectural slice is durable object allocation and references before deletion/repacking, with a v1 migration protecting the saves introduced here.
Saved-world menu integration, asynchronous snapshot encoding and smaller encodings are separate possible follow-ups, not claims of this checkpoint.
Unrelated rendering/assets remain intact. No commit or push was made.

## Third checkpoint — durable object identity

Continued on 12 October 2026. Applicable instructions, the previous assessment,
and `SimState::step`, settlement/path records, subsurface withdrawal, scenario
seeding, app selection/picking and save validation were inspected. The preceding
153-test save checkpoint is the baseline. The [source audit](current-state.md#durable-identity-slice-audit-12-october-2026)
and [identity decision](durable-identity.md) explain the demonstrated allocation
and reference risks and the migration contract.

### Changes and behavior

- `sim_core/identity.rs` and domain owners persist monotonic building, path and
  well allocators. Geometry/occupancy validation precedes allocation. Exhaustion
  rejects without construction or cover removal. Existing building/well numbers
  remain unchanged; production generators use the same counters.
- `settlement::Path` and `SimEvent::PathBuilt` carry durable IDs. App interaction,
  session selection, focus/selection geometry, inspector and native interaction
  smoke resolve IDs instead of vector indices. Removed targets clear selection.
  Added ID-zero fields in visual-only path literals preserve their geometry.
- Subsurface withdrawals use ascending well ID, preserving existing append-only
  arbitration and making reordered storage safe for shared scarce columns.
- `save/migration.rs` upgrades valid v1 records in memory. V2 validation checks
  uniqueness, nonzero IDs and persisted allocator bounds. The original v1 fixture
  is retained, alongside a populated v1 fixture with a pending well reference.
  Capture writes v2; staged loading, backups, logical timing and pending delivery
  keep the previous contract.

No deletion mechanic, resource model, storage replacement, dependency upgrade,
new threading, or rendering redesign was introduced. Scenario geometry and
normal append-only gameplay retain their behavior. `PathBuilt`'s serialized
field changes from `index` to `id`; action logs are unchanged. V2 saves require
this or a compatible newer reader. V1 remains supported.

### Verification

| Check | Result |
| --- | --- |
| `mise run ci` | Passed: app 74, headless 2, renderer 28, sim_core 47, storage 7, one compile-fail doctest = **159 passed**, 14 optional app tests ignored. Formatting and warning-free Clippy pass. |
| `mise run build` | All five packages build. |
| Removal/reordering/exhaustion tests | Highest removed IDs are not reused; stale well commands cannot target replacements; reordered wells retain scarce-water allocation; exhaustion creates no objects and clears no cover; path selection follows ID and rejects missing targets. |
| Save continuation/migration tests | Populated v1 references and pending actions survive migration; removal before save does not reset allocation; v2 round-trip preserves continued state. Missing/invalid counters, zero/duplicate IDs, malformed legacy identities and unknown schemas reject. All three generated region round trips still pass. |
| `target/debug/genesis-headless --load /private/tmp/genesis-save-native.world.json --ticks 0 --save /private/tmp/genesis-identity-migrated.json` | Previous checkpoint's actual v1 native save loads. Exact parsed equality after removing only added ID/counter fields and restoring the schema number; world identity, pending intent and all previous domain fields retained. |
| `target/debug/genesis-app --interaction-test /private/tmp/genesis-identity-interaction.png` | Exit 0 on Apple M4/Metal. Native route constructs/selects Footpath 6; highlight, hover and inspector capture visually inspected. |
| `target/debug/genesis-app --save-test /private/tmp/genesis-identity-save.png` | Exit 0 on Apple M4/Metal. Pending drilling survives v2 save/load and exact continuation; restored seven-dwelling/one-well scene visually inspected. User quicksave slot untouched. |
| `git diff --check` | Passed. |
| Headless dependency tree | Still no wgpu, winit, egui, renderer or Bevy dependency. |
| Original user work | Hash comparison preserves original dirty rendering/assets. The only new overlap in this slice is one `id: 0` field in the existing contacts test; stripping that line reproduces its original hash. |

The first CI run caught duplicate-path validation relying on whole-record
identity equality. It was corrected to compare geometry/width; the existing
ordered-duplicate test passes in the final run. This was an implementation
regression caught and fixed, not a baseline defect. Evidence is in
`/private/tmp/genesis-identity-*.log` and corresponding captures.

No performance improvement is claimed. Existing append-only wells need only the
ordering check; arbitrary reordering may require sorting. Native macOS remains
the verified platform; browser, other desktop targets, device loss and the
excluded legacy workspace remain unverified. Arbitrary setup mutations remain
possible through raw `SimState`, with invalid IDs/counters rejected on snapshot;
live hosts retain the read-only `Simulation` boundary.

The next useful slice is the terrain-action contract: establish which edits are
legal around existing foundations, paths and generated channels, and reject
invalid edits without partial effects before revising hydrological models.
Existing derived-worker revision checks should be retained. No absent colony
subsystem is required for that boundary work. No commit or push was made.

## Fourth checkpoint — staged terrain command boundary

Continued on 12 October 2026. The previous checkpoint's 159 passing tests/doc
checks and native interaction/save route form the baseline. Applicable project
instructions and the current vision/engine decision were retained. Inspection
covered brush execution, B-spline sampling, physical/channel surfaces, exposed
foundations, action ordering, app terrain revisions, worker tags and save rules.
See the [terrain audit](current-state.md#terrain-command-audit-12-october-2026)
and [decision](terrain-commands.md).

### Changes and scope

`rules/terraform.rs` now validates finite strength and region intersection,
widens coordinates before offset arithmetic, stages a sparse elevation patch,
and commits only after staging succeeds. The existing helper delegates to the
fallible operation. Its tests moved into a feature-specific child module.
`SimState::step` emits changed, unchanged or rejected outcomes. `Session::tick`
uses actual terrain changes for geometry revision and urgent rebuilding, while
showing the result in existing HUD feedback. Tests cover these boundaries and
pending terrain commands through the existing save adapter.

The source supports ground-following foundations and channel incision, so the
slice deliberately preserves those sandbox semantics. It does not introduce
foundation damage, path blocking, river rerouting, displacement of stored water,
or exclusion zones. Atomicity here covers the elevation patch; the existing
later tick stages still update sea depth/climate/life before world publication.
Normal tick progression and earlier accepted actions are not rolled back when a
later command rejects. Future coupled domain rules must join the validation and
commit boundary before they are implemented.

V1 migration, schema v2, content/rules markers, identity allocation, and normal
numerical behavior remain unchanged. New terrain event variants are an event API
addition; world saves do not retain this disposable history. Existing files and
pending intent remain supported. No graphical dependency or library upgrade was
introduced. Sparse staging adds bounded temporary allocation/tree lookup work;
no performance improvement or throughput claim is made.

### Evidence

- `mise run ci` passes: app 75, headless 2, renderer 28, sim_core 53, storage 8,
  and one compile-fail doctest = **167 passed**, 14 optional app checks ignored.
  No baseline or final test failure was found. The final signed-zero adjustment
  was included; the subsequent metrics-only match arm was checked by the direct
  CLI rejection run, package Clippy, and workspace build described below.
- `mise run build` passes for the workspace. Formatting and warning-free Clippy
  pass; the final headless metrics integration was additionally checked with
  `mise exec -- cargo clippy -p genesis-headless --all-targets -- -D warnings`.
- New brush tests cover clipped and wrapped edges, repeated visits, extreme
  coordinates, atomic rejection, saturation/no-op behavior, same-tick placement
  ordering, unchanged object identities and updated channel queries. Host tests
  distinguish revision changes from intent. Storage tests preserve a pending
  changed/rejected/no-op batch and its exact continuation.
- `target/debug/genesis-app --save-test /private/tmp/genesis-terrain-native.png`
  exits 0 on Apple M4/Metal. The existing camera/pause/terraform/placement/resize
  and persistence path remains usable; the seven-dwelling/one-well capture was
  visually inspected. The regular user quicksave slot was untouched.
- `target/debug/genesis-headless --scenario fixture --ticks 100 --commands
  crates/sim_core/fixtures/coastal.commands.json --save
  /private/tmp/genesis-terrain-full.json` matches the pre-identity/pre-terrain
  `/private/tmp/genesis-save-check/full.json` exactly for all domain fields after
  excluding only the new allocator/path-ID metadata. Logical tick is 100 and
  pending intent is empty in both.
- A CLI integration run loads the populated v1 fixture with bounded edges,
  executes brushes at both i32 coordinate extremes plus a zero-strength brush,
  and reports exactly two rejected commands at tick 1. Headless rejection metrics
  now include terrain failures; its README explains the new event counts.
- The headless dependency tree remains free of graphics/Bevy crates. Original
  dirty renderer/assets still match their preserved hashes. `git diff --check`
  passes. No commit or push was made.

Logs/captures use `/private/tmp/genesis-terrain-*`. Native macOS remains the
verified platform; device-loss handling, other desktop hosts, browser and the
excluded legacy workspace remain unverified. No performance claim is made.

The next small architecture check is world replacement with terrain work already
in flight: verify load/regeneration transitions against the existing revision
and edit-epoch guards, retaining the bounded single-worker design. Deeper
foundation/navigation/hydrology consequences need their own gameplay scope.

## Fifth checkpoint — world replacement and derived-work ownership

Continued on 12 October 2026. The prior checkpoint's 167 passing tests/doc checks,
native save smoke and preserved fixture behavior form the baseline. Applicable
repository instructions remain in force. Inspection covered all runtime session
replacement sites, cache invalidation in `View`, scene preparation, frontend
arrival and existing native world/save checks. The [audit](current-state.md#world-replacement-and-derived-work-audit-12-october-2026)
and [decision](derived-work.md) record the retained contracts.

### Findings and changes

No current stale-world overwrite was reproduced: existing terrain revisions,
view keys, survey settings and edit epochs already form a valid guard. The gaps
were duplicated replacement invalidation and tests limited to tag comparisons.

- `Session::replace` centralizes replacement revision advancement. Generated
  worlds use `replace_sim`; regeneration and frontend arrival route through it.
  Staged loads activate through the same boundary after successful validation.
  Failed loads retain the current world and valid pending presentation work.
- `view/preparation.rs` owns admission and completion in `Preparation`. One
  outstanding request includes an unconsumed result. Admission checks capacity
  before cloning input; the result channel has capacity one. Stale/failed results
  are discarded before reaching the GPU/cache publisher and release the slot.
- `View` consumes validated completion outcomes and retains existing synchronous
  replacement/edit rebuilding, cache reuse, picker updates, scenery invalidation
  and upload behavior. The original owner's rendering additions remain intact;
  a turn-local diff confirms changes are limited to preparation integration.
- Preparation tests moved into a feature-specific child module and now drive
  real staged loads, controlled late delivery, repeated same-world reloads,
  rejected admission while busy, failed loads, generated replacement, worker
  disconnection and eventual convergence to current terrain/picking.

No simulation rules, world identities, pending action semantics, save schema,
content metadata, dependency versions or supported host behavior changed.
The existing transient terrain revision remains the combined world/terrain
invalidation generation; no redundant world epoch or general job system was
introduced. Single-worker behavior was preserved and encapsulated, not presented
as a fix for a demonstrated unbounded worker leak.

### Verification and remaining scope

| Check | Result |
| --- | --- |
| `mise exec -- cargo test -p genesis-app view::preparation` | Five focused tests pass, including controlled delayed delivery across real loads and matching current-world picking. |
| `mise run ci` | Passed: app 78, headless 2, renderer 28, sim_core 53, storage 8, one compile-fail doctest = **170 passed**; 14 optional app tests remain ignored. Formatting and warning-free Clippy pass. |
| `mise run build` | Workspace build passes. |
| `target/debug/genesis-app --world-test /private/tmp/genesis-world-work-regeneration.png` | Exit 0 on Apple M4/Metal. Regeneration replaces the terrain, clears stale queued intent, preserves drilling/groundwater behavior and displays the rebuilt survey. Capture visually inspected. |
| `target/debug/genesis-app --save-test /private/tmp/genesis-world-work-save.png` | Exit 0 on Apple M4/Metal. Staged loading restores pending drilling and exact continuation; rebuilt seven-dwelling/one-well capture visually inspected. Regular quicksave slot untouched. |
| Headless dependency graph | Still excludes renderer, wgpu, winit, egui and Bevy. No manifests or lockfile changed in this slice. |
| Diff and owner work | `git diff --check` passes. Original dirty renderer/assets retain their hashes; the necessary `view.rs` overlap was reviewed against a turn-local copy and contains only preparation-owner integration. |

The initial CI run caught `ok().expect()` in the new test helpers; it was replaced
with direct `expect()` before the passing final run. This is a corrected style
issue, not a baseline defect. No simulation or visual regression was found.
Native smoke does not force a late-worker race; deterministic controlled delivery
tests provide that evidence. Logs/captures use `/private/tmp/genesis-world-work-*`.

No performance improvement is claimed. Whole-world snapshots and synchronous
urgent extraction remain, and a permanently stalled worker still holds its
request slot. Device recovery/view recreation, browser hosting, other desktop
targets and the excluded legacy workspace remain unverified. The next useful
slice is measurement of snapshot/extraction costs on a seeded generated region
before selecting a performance change. No absent gameplay subsystem is required.
No commit or push was made.

## Slice 6 — measure CPU preparation and narrow terrain queries

Inspected the applicable workspace/repository instructions, engine decision,
vision, prior checkpoints and the actual preparation pipeline (`view/preparation`,
`terrain`, `scene`, `subsurface`, `world_mesh`, existing render benchmark).
The baseline was the preceding slice's passing 170-test CI and native checks.
Source findings and acceptance were added to `current-state.md` before changing
terrain's query.

Added `app/extraction_benchmark.rs` and an early `--extraction-benchmark` CLI route,
with README instructions. It measures existing CPU preparation stages for seed
7319 at regional/settlement cameras and surface/water surveys. The existing
owner-authored render benchmark is untouched. Raw before/after samples and full
method/limits are in [extraction-performance.md](extraction-performance.md).

Measurements show snapshots around 0.5–0.6 ms; retain their existing ownership
and representation. Terrain coloring unnecessarily requested a full aquifer
reading just for its rock. Added `Subsurface::rock_at`, sharing the unchanged
weighted-tap selection with `reading`, and changed that single terrain call.
No rules, world state, scheduling, IDs, schemas, content versions, geometry,
renderer resources or dependencies changed. No additional allocation or cache.

Development-profile median scene extraction fell 7.7–9.6% across four cases;
near the settlement it fell 62.102 → 56.738 ms (surface) and 72.152 → 66.370 ms
(water survey). Counts and presentation vertex fingerprints match. Cold-cache
construction still dominates; no release, FPS or GPU improvement is claimed.
Raw reports also preserve small unrelated-stage regressions/noise rather than
attributing every timing difference to this change.

Validation:

- `mise run build`: passes before and after the optimization.
- Both `target/debug/genesis-app --extraction-benchmark ...` runs: exit 0;
  2 warmups + 20 measured iterations per case; fingerprints stable in each run
  and matching across reports.
- `mise run ci`: passes, **171 tests/doctests** (app 78, headless 2,
  renderer 28, sim_core 54, storage 8, compile-fail doctest 1), 14 optional app
  tests ignored; formatting and warning-free Clippy pass.
- New meaningful regression covers clamped boundaries and categorical tie
  selection, with expected rocks independent of the selector implementation.
- Native `--survey-test /private/tmp/genesis-extraction-survey.png`: exit 0,
  Apple M4/Metal; capture visually inspected, showing seven dwellings, one
  active well, terrain and water/mineral overlays.
- `git diff --check` and original-owner-file hash review pass; the `main.rs`
  overlap adds only the benchmark module/CLI route. No commit or push.

Save schema 2 and existing migrations remain compatible. CPU diagnostic execution
requires no GPU, but its app package links graphics; do not confuse it with the
separate headless dependency closure. macOS development-profile evidence only;
release, browser, other desktop hosts and lifecycle recovery remain unverified.
Logs/capture are `/private/tmp/genesis-extraction-*`; raw timing reports are retained
under `docs/architecture/measurements/`. Next useful performance work is staged
measurement inside cold terrain-cache construction, if further preparation
latency work is selected. No absent gameplay feature was added.

## Seventh checkpoint — safe life-seeding command boundary

Continued on 12 October 2026 from the existing six uncommitted checkpoints.
Re-read applicable instructions, engine decision, vision, architecture records and
the actual runtime/action/save/host paths; see the continuation audit in
[current-state.md](current-state.md#continuation-audit-life-seeding-command-boundary-12-october-2026).
The supplied owner brief already exists in full and was preserved.
Fresh baseline: 172 passing tests/doctests, 14 optional tests ignored, formatting
and warning-free Clippy. This differs from the earlier recorded 171-test checkpoint
because the entry working tree already has 79 passing app tests.

### Demonstrated problem and bounded change

A one-tick fixture playback with `SeedPlants { x: 2147483647, y: 0, radius: 1 }`
exited 101 before edits: `attempt to add with overflow` at `lib.rs:270`.
This is a reproduced baseline defect through the public command path.
`SimState::seed_life` performed i32 offset arithmetic before map validation.

Extracted that existing rule to `sim_core/src/rules/seeding.rs`, with focused
tests in `rules/seeding/tests.rs`. `lib.rs` delegates both seed actions through
`rules/mod.rs`. Centers and offsets now use widened arithmetic; wrapped centers
normalize first, and bounded brushes outside the region are no-ops.
No mutable host/renderer access or new dependency was introduced.

Retained ordinary brush coverage, circular footprint, clipped continental edges,
wrapped repeated visits, radius cap, 0.3 increments, saturation, underwater
exclusion, action order and subsequent tick stages. Water eligibility still uses
the existing committed-depth field, including after an earlier terrain action in
the same tick. No new rain, ecosystem or hydrology model is implied.
Intentional change: extreme coordinates execute safely with the map's defined
topology instead of overflowing. Legacy seed/outpost commands still lack structured
outcomes; that broader API change is deferred and no success events were invented.

### Verification

- `mise run test`, `mise run fmt-check`, `mise run clippy`: baseline passes.
- `mise exec -- cargo test -p genesis-sim-core rules::seeding`: four new tests pass,
  covering both species through real `SimState::step` commands, extreme coordinates,
  bounded edges, negative wrapped coordinates, water, saturation and later commands.
- `mise exec -- cargo test -p genesis-storage pending_seed`: pending intent remains
  unapplied when captured/loaded, then matches events and exact world state after
  ten ticks under both topologies. Existing v1 fixtures/v2 continuation tests pass.
- `mise run ci`: **177 tests/doctests pass**, 14 optional app tests ignored; app 79,
  headless 2, renderer 28, sim_core 58, storage 9, compile-fail doctest 1.
  Formatting and warning-free Clippy pass. `mise run build` passes.
- The formerly failing CLI command exits 0 after the change.
- Before/after CPU runs use fixture seed 42, 100 ticks and
  `crates/sim_core/fixtures/coastal.commands.json`; a second run adds fungus
  at tick 4 and two plant brushes at tick 5. Both runs match every saved domain
  field and metrics exactly, excluding only independently random host world IDs.
- `target/debug/genesis-app --save-test /private/tmp/genesis-seeding-native.png`
  exits 0 on Apple M4/Metal; pause, single-step, terraforming, building placement,
  pending drilling/save/load continuation, zero-size suspension and resize pass.
  The captured seven-building/one-well scene and HUD were visually inspected.
  This retained native route does not exercise a life-seeding UI; headless/domain
  tests cover that action. The normal user save slot was untouched.
- `mise exec -- cargo tree -p genesis-headless --edges normal` still excludes
  renderer, wgpu, winit, egui and Bevy. `git diff --check` passes.
- Turn-entry file hashes preserve every pre-existing file outside the five
  deliberately edited paths (core lib/rules module, storage tests, audit and log).
  The only added files are the seeding rule and its tests. No commit or push.

Schema 2, v1 migration, content/rules markers and supported save representations
remain unchanged. Existing normal command runs retain exact behavior; no replay
promise is made for previously overflowing commands. This correctness extraction
has no performance claim. Native macOS is verified; browser/other hosts, legacy
and full device-loss recovery remain unverified. Logs, command reproducers and
before/after saves are under `/private/tmp/genesis-seeding-*`; baseline checks use
`/private/tmp/genesis-brief-baseline-*`.

Next small step: define structured outcomes for legacy seed/outpost commands if
that boundary is selected, while preserving their normal gameplay semantics.
Cold-cache preparation measurement and device recovery remain independent work;
this checkpoint does not claim that the whole architecture brief is implemented.

## Publishing the integrated checkpoint — 12 October 2026

At the owner's request, the accumulated Genesis architecture, static asset,
renderer and roadmap work is being published together as a buildable checkpoint.
The changes share app/manifests and include the required embedded dwelling GLBs
(project-owned CC0, about 1.5 MB combined), authoring scripts, save fixtures and
measurement reports. Generated build output, captures and Python caches are excluded.
No unrelated repository files or credentials are included.

Fresh validation: `mise run ci` passes 177 tests/doctests (14 optional ignored),
formatting and warning-free Clippy; the native `on_native_gpu` suite passes all
six tests, including imported asset reload/culling and atmosphere/material paths.
Python authoring tests pass two cases. Previous native save smoke remains valid.
`git diff --check` passes and `origin/main` matches the local parent before commit.
The missing [cache performance report](cache-performance.md) now documents the
existing adaptive-query code and its retained before/after data without claiming
a new benchmark run. Continue with real edit-preparation measurements next.

## Committed-edit preparation diagnostic — 12 October 2026

The integrated checkpoint was committed as `133d873` and pushed to `origin/main`.
Continued development now adds `--terrain-edit-benchmark` through the existing
CPU diagnostic module. It runs actual queued Session commands, then the cache,
picker, scene and partition stages used by the urgent presentation path.
`PreparedWorld::upload_estimate` exposes read-only chunk replacement accounting
for the diagnostic; normal GPU submission is unchanged.

Source audit and scope precede this change in `current-state.md`. The workload,
method, raw data and limits are in [terrain-edit-performance.md](terrain-edit-performance.md).
The baseline fixes seed 7319, a nine-sample terrain brush at tick one, surface
view and three camera keys. Rebuilding a same-tick no-edit control is necessary
to avoid conflating cached horizon colors with the terrain edit. An initial
development run used a reused control cache; only the corrected control run is
retained as the baseline.

Result: median total CPU preparation is 261.447 ms regionally, 435.897 ms at
management distance and 526.035 ms at inspection distance. Against a rebuilt
same-tick control, the edit changes only two of 924 prepared chunks. Against
tick-zero presentation, normal environmental changes and refresh increase this
to 550. This identifies separate geometry and appearance invalidation needs;
it is measurement infrastructure, not a claimed speedup.

Validation: `mise run ci` passes 178 tests/doctests with 14 optional ignored,
formatting and warning-free Clippy; `mise run build` passes. A new regression
checks upload estimates for reuse, edits, additions and removals. The complete
diagnostic exits 0, verifies command consumption, logical tick, revision and
changed-cell/event agreement, picking coordinates and repeatable presentation
fingerprints across 66 trials. The six native GPU checks passed for the preceding
checkpoint; this slice changes no production drawing or host behavior.

Schemas, content, rules, dependencies and platform support are unchanged.
No actual GPU traffic or input-to-display latency is measured. Next implement
scoped terrain invalidation with sampling/normal halos, LOD seam handling and
matching river/picking publication, validating against this full reconstruction.

## Scoped committed-terrain refresh — 13 October 2026

Starting checkpoint: clean `3195938` on `main`, tracking `origin/main`. Inspected
workspace/local agent instructions, `.claude/CLAUDE.md`, accepted engine/vision
and architecture records, plus the Terraform → Session → View → cache/picker/
water/scene preparation path. The audit in `current-state.md` precedes the edits.
The measured problem was full CPU terrain reconstruction for a nine-sample edit;
LOD splits and channel endpoint slopes prevent naïve brush-only invalidation.

Changes: `sim_core` reports actual committed `SampleBounds` without changing brush
rules. `Session` keeps a bounded complete revision chain; `terrain/refresh.rs`
reuses unchanged immutable topology, samples, river data and picker buckets where
safe. LOD split changes, camera changes, missing history and other geometry changes
retain full reconstruction. `View::ensure_terrain` publishes the matching cache
and picker under the existing worker epoch/revision guard. Current ecological
colors, horizon, ground cover and contacts continue to refresh. The diagnostic
now supports `--full-rebuild`, compares local output against that reference, and
retains raw full/local reports. The native world script includes a committed
stationary-camera edit before regeneration.

Fresh comparison on Apple M4: median total CPU preparation falls 27.3% regionally
(253.305 → 184.111 ms) and 36.9% at inspection distance (514.929 → 324.873 ms).
Management changes LOD splits and falls back, measuring 3.3% slower in these
sequential runs (406.581 → 420.088 ms); no improvement claimed there. All three
full vertex fingerprints and upload estimates are identical. Workload, tails,
extra cache memory, run-order limitations and remaining global work are in
[terrain-edit-performance.md](terrain-edit-performance.md).

Validation:

- Baseline `mise run ci`: 178 tests/doctests, formatting and Clippy pass.
- Final `mise run ci`: 184 tests/doctests pass, 14 optional app tests ignored;
  formatting and warning-free Clippy pass. `mise run build` passes.
- Six added tests cover actual clipped/wrapped commit bounds, retained/expired
  revision ranges, construction/world replacement, exact full-reference geometry,
  water/color/picker results, wrapped interpolation halos, river endpoints and
  confluences, close/legacy detail, camera and topology fallbacks.
- Both benchmark commands exit 0; each runs 66 committed edits. Local output is
  also checked against fresh full extraction and oblique water/ground picks.
- `target/debug/genesis-app --world-test /private/tmp/genesis-local-world.png`:
  exit 0; local edit, regeneration, drilling and surveys. Initial/final captures
  visually inspected.
- `target/debug/genesis-app --interaction-test /private/tmp/genesis-local-interaction.png`:
  exit 0; roof selection, path preview, construction and path picking. Final
  capture visually inspected.

- `target/debug/genesis-app --save-test /private/tmp/genesis-local-save.png`:
  exit 0 on Apple M4/Metal; queued drilling survives save/load with exact
  continuation, free placement and zero-size/resize checks pass. Seven dwellings
  and one borehole remain visible after restoration; capture visually inspected.
  The user quicksave slot is untouched.
- `git diff --check` passes. Baseline/final logs and native captures are under
  `/private/tmp/genesis-local-*`; only source, documentation and benchmark JSON
  reports enter the checkpoint.

No intentional gameplay changes, save/content migrations, new dependencies or
platform claims. Events now include bounds; saved world/command schemas do not
change. Native macOS/Metal is checked; browser, other hosts and device-loss recovery
remain unverified. The graphics-independent headless package remains unchanged
and is built/tested by the workspace checks. No FPS claim: synchronous edit
preparation still exceeds a frame budget. Next handle the changed LOD-root seam
neighborhood locally, then target remaining horizon/color and extraction costs
with matching full-reference checks.

## Local terrain LOD transitions — 13 October 2026

Starting checkpoint: clean `579c016` on `main`. Read workspace/local instructions,
`.claude/CLAUDE.md`, accepted engine/vision documents and the previous measurement
record. The source audit in `current-state.md` identifies the one-root seam
dependency in `terrain/lod.rs`; `water/sheet.rs` depends on ordered receivers.

`terrain/refresh/topology.rs` now reconstructs changed LOD roots plus their seam
neighbors and compacts flat storage while preserving ordered triangles. `Cache`
retains ordered leaves; `lod.rs` shares its existing triangulation routine between
full and scoped construction. Outside that neighborhood, triangle connectivity
and surface samples are reused. Existing water and picker invalidation halos cover
the new seams, and revision/epoch publication guards remain intact. Camera/world
changes retain full reconstruction. No simulation code or GPU pipeline changed.

The benchmark now records retained leaf allocation bytes. The native world script
starts at the same management camera and applies the same 0.4-strength edit before
world replacement, exercising the previously falling-back path. No intentional
playable behavior changes; the altered camera/edit strength apply only to this
scripted diagnostic.

The before/after comparison reduces management-view total median CPU preparation
from 433.081 to 279.629 ms (35.4%), and p95 from 470.349 to 299.686 ms (36.3%).
All three benchmark views now refresh locally, preserving the original full mesh
fingerprints and predicted uploads. Regional/inspection changes are recorded but
not attributed to this algorithm because their split layouts already stayed fixed.
Raw reports, exact commands and limitations are in
[terrain-edit-performance.md](terrain-edit-performance.md).

Retained leaf allocations add about 1.5–6 MiB per measured cache. Global remapping,
horizon rebuild, scene extraction and partitioning remain; the result is not yet
frame-budget terrain editing. Repeated refinement/coarsening tests compare live
sample counts and ordered geometry with a fresh full cache to prevent abandoned
vertex accumulation. Vertex storage indices are disposable and may be repacked;
receiver triangle ordering is preserved.

Validation:

- Baseline `mise run ci`: 184 tests/doctests pass. Final `mise run ci`: 185 pass,
  14 optional app tests ignored; formatting and warning-free Clippy pass.
  `mise run build` passes.
- Full-reference tests cover terrain samples, ordered receiver triangles, scene/
  water vertices, ground cover, picker buckets/rays, camera fallback, map edges,
  repeated refinement/coarsening, and changed seams in unedited neighboring roots.
- Both 66-trial benchmarks exit 0; the after run also checks fresh full extraction
  and oblique picking in all three views outside timings.
- `target/debug/genesis-app --world-test /private/tmp/genesis-lod-world.png`:
  exit 0 on Apple M4/Metal; management-view edit, replacement, drilling and surveys.
  The edited management capture was visually inspected.
- `target/debug/genesis-app --interaction-test /private/tmp/genesis-lod-interaction.png`:
  exit 0; selection, previews, construction and path picking. Capture inspected.

- `target/debug/genesis-app --save-test /private/tmp/genesis-lod-save.png`:
  exit 0 on Apple M4/Metal; pending drilling survives save/load and exact
  continuation, placement and resize/suspend checks pass. Capture inspected;
  the user quicksave slot is untouched.
- `git diff --check` passes. Logs/captures stay under `/private/tmp/genesis-lod-*`;
  only code, documentation and raw benchmark reports enter this checkpoint.

No save/event/command schema, content, domain rule or dependency changes. The
workspace continues to build/test its independent headless simulation. Native
macOS is verified; other hosts and device-loss recovery remain unverified. Next
small renderer slice: retain unchanged horizon geometry while refreshing its
palette, with before/after evidence and full-reference output checks. The living
settlement remains the next separate product feature.

## Reusable horizon geometry and terrain palettes — 13 October 2026

Starting checkpoint: clean `4530d66` on `main`. Read workspace/local instructions,
`.claude/CLAUDE.md`, engine/vision decisions and previous audits/measurements.
Inspected `horizon.rs`, terrain sampling/coloring and refresh, scene extraction,
worker publication and renderer partitioning. The source findings and acceptance
criteria were recorded in `current-state.md` before implementation: all horizon
relief/normals and all terrain colors were recalculated after a local edit.

`horizon/geometry.rs` now owns immutable visual geometry and seam color recipes;
`horizon.rs` refreshes current environmental colors and shares unchanged expanded
ground pages. Bounded interior edits reuse geometry; edge/wrapped edits and changed
horizon parameters rebuild. `terrain/appearance.rs` tracks plants/moisture
interpolation dependencies and global palette inputs. `terrain/refresh/topology.rs`
returns temporary source indices so cached colors remain attached to the correct
samples after LOD compaction. Surveys use the full color extractor. Existing
worker bounds, revision guards and world invalidation remain intact.

No intentional gameplay or image changes. Normal terrain continues to reflect
current ecology, including changes outside a terrain brush. The prior horizon
palette lifetime across ordinary no-edit ticks is preserved. Full-reference scene,
water and picking checks remain exact; a frozen independent horizon extractor
checks the new geometry/color recipes. No simulation or renderer GPU code changed.

Measured median total CPU preparation improves from 176.556 to 133.398 ms in the
regional view (24.4%), 269.561 to 223.459 ms in management (17.1%), and 316.474 to
267.347 ms in inspection (15.5%). Full-rebuild medians remain comparable. All four
66-trial reports have identical corresponding mesh fingerprints/counts. Upload
estimates are unchanged. Cache coloring moved out of the scene phase, so phase
numbers alone must not be interpreted as the total benefit. Raw reports, p95,
commands, workload and auxiliary memory counts are in
[terrain-edit-performance.md](terrain-edit-performance.md).

Validation:

- Baseline `mise run ci`: 185 tests/doctests pass. Final `mise run ci`: 189 pass,
  14 optional app tests ignored; formatting and warning-free Clippy pass.
  `mise run build` passes. No baseline validation failures.
- Added checks cover horizon geometry/page reuse, distant palette changes,
  ordinary ecology, old-reader immutability, map edges, wrapping, sea changes,
  island replacement, interpolation boundaries, rock/drought changes and local
  terrain color reuse. Existing LOD/picker/survey/stale-worker tests pass.
- Four CPU benchmark runs exit 0; fresh full extraction and oblique picking are
  checked outside scoped timing, with equal fingerprints against the prior build.
- `target/debug/genesis-app --world-test /private/tmp/genesis-extraction-world.png`:
  exit 0 on Apple M4/Metal; management edit, replacement, drilling and surveys.
  Edited management capture visually inspected.
- `target/debug/genesis-app --interaction-test /private/tmp/genesis-extraction-interaction.png`:
  exit 0 on Metal; selection, previews, path construction and picking. Capture
  visually inspected.
- `target/debug/genesis-app --save-test /private/tmp/genesis-extraction-save.png`:
  exit 0 on Metal; pending drilling, exact save/load continuation, placement,
  zero-size and resize checks. Restored seven dwellings/one borehole visible in
  inspected capture; user quicksave untouched.
- `git diff --check` passes. Logs/captures stay in `/private/tmp/genesis-extraction-*`;
  source, documentation and raw benchmark JSON enter the checkpoint.

No rule, command/event/save schema, content or dependency changes. The independent
headless package remains built/tested by workspace CI. Native macOS is verified;
other hosts and device-loss recovery remain unverified. Additional cache memory
and global dependency scans are documented; synchronous preparation remains above
a frame budget. Next measure the remaining local-refresh stages before selecting
another bottleneck. The living-settlement feature remains separate product work.

## Profile local refresh and reuse river depth queries — 14 October 2026

Starting checkpoint: clean `72e65d4` on `main`. Read workspace/local `AGENTS.md`,
`.claude/CLAUDE.md`, workspace general conventions, engine/vision decisions and
previous audits/measurements. Inspected local terrain refresh, cover/contact
builders, receiver queries and river projection/slope/surface generation.
The source audit and adoption criteria are recorded in `current-state.md`.

Added observational hooks in `terrain/refresh.rs` and `water.rs`, exposed through
`extraction_benchmark/edits.rs`. The fixed management edit spends 53.728 ms in
final river surface extraction, 31.468 ms in projection and 2.710 ms in slope
setup; contact shading is 1.435 ms and ground cover is skipped at that altitude.
The measurement selected river surface generation as the next target.

`water/depth_cache.rs` memoizes repeated physical depth queries at shared polygon
vertices within one extraction. `water/sheet.rs` accepts that query function;
its clipping, interpolation, material and output order remain unchanged.
Exact float-bit keys and full-key comparisons prevent coordinate aliasing.
8,192 replacement slots use 128 KiB temporary storage on the measured target;
collisions only recompute. No cache entries survive an extraction or world change.
No simulation ownership, rules or scheduling changed.

River surface medians improve 58.4–60.3%; total CPU preparation changes from
135.916 to 107.112 ms regionally (21.2%), 228.081 to 187.668 ms in management
(17.7%), and 271.615 to 232.271 ms in inspection (14.5%). Complete mesh fingerprints,
vertex counts, appearance statistics and upload estimates remain identical.
Inspection p95 improves less than its median and partition timings vary; these
limits and raw reports are retained in
[terrain-edit-performance.md](terrain-edit-performance.md).
No FPS or display-latency claim; synchronous preparation still exceeds a frame.

Validation:

- Baseline `mise run ci`: 189 tests/doctests pass. Final `mise run ci`: 190 pass,
  14 optional app tests ignored; formatting and warning-free Clippy pass.
  `mise run build` passes. No baseline failures.
- Terrain refresh tests now additionally compare river vertices bit-for-bit with
  uncached depth queries across edits, LOD refinement/coarsening, river endpoints,
  confluences, boundaries and wrapping. Replacement-table checks cover collision
  eviction, repeated lookups, signed zero and bounded storage.
- Both recorded 66-trial CPU benchmarks exit 0, with full reconstruction/picking
  validation outside timings. Before/after geometry report objects are equal.
- `target/debug/genesis-app --world-test /private/tmp/genesis-refresh-world.png`:
  exit 0 on Apple M4/Metal; edited management scene, world replacement, drilling
  and surveys. Edited capture visually inspected.
- `target/debug/genesis-app --interaction-test /private/tmp/genesis-refresh-interaction.png`:
  exit 0; selection, previews, path construction and picking. Capture inspected.
- `target/debug/genesis-app --save-test /private/tmp/genesis-refresh-save.png`:
  exit 0; queued drilling and exact continuation across save/load, placement,
  zero-size and resize checks. Restored capture inspected; user quicksave untouched.
- `git diff --check` passes. Logs/captures remain in `/private/tmp/genesis-refresh-*`;
  source, documentation and the two raw comparison reports form the checkpoint.

No save/content/event schema, dependency or platform support changes. The independent
headless package remains built/tested by workspace CI. Native macOS is verified;
other hosts and device-loss recovery remain unverified. River invalidation still
rebuilds the whole network when touched; the new stage breakdown makes remaining
costs explicit. Further performance work is separable from the next playable
milestone: persistent villagers with a visible, explainable routine.

## 15 October 2026 — first persistent residents

- Inspected the applicable workspace/project instructions and engine/vision decisions;
  traced `sim_core::{runtime,identity,save,worldgen,surface::walking}` and the app's
  session, extraction, picking, inspector and native checks. Clean starting point
  `b636c63`; baseline `mise run ci` passed 190 tests, 14 optional tests ignored.
- Found no resident store or routine to preserve. Reused typed persistent allocation,
  fixed advancement and physical walking rules. Added `people.rs` and bounded local
  navigation, a compact inhabited fixture and new-scenario population. No ECS,
  dependency upgrade, scheduler replacement or new engine package.
- App figures, interpolation, picking, selection and inspection derive from committed
  residents; GPU ownership remains unchanged. Six people in default coast seed 7319
  walk between outdoor home thresholds and meeting spots. New HUD count and activity
  explanations make this behavior inspectable. This intentionally adds people to new
  Settlement worlds; old worlds and World Builder remain unpopulated.
- Schema 3 / rules 2 persist identities, homes, positions, committed routes and timers.
  Staged V1/V2 migration preserves the old worlds with empty resident stores. V1
  fixtures remain unchanged; added a V2 fixture. Malformed resident records fail
  validation. Native recovery/write behavior and content version remain unchanged.
- `mise run ci`: **198 tests pass**, 14 optional tests ignored; rustfmt and Clippy
  with warnings denied pass. `mise run build` passes. New checks cover complete
  routines, stable references under repacking, terrain/route blocking, deterministic
  snapshot and encoded save continuation, different frame partitions, presentation
  immutability, interpolation/picking and world replacement. Existing HUD layout
  checks now include the resident inspector at all supported sizes.
- CPU-only `--scenario settlement`: save at tick 35, reload and advance 1165 ticks
  produces the exact serialized state of an uninterrupted 1200-tick run (excluding
  host-generated world identity). Default coast also ran 1200 ticks with all six
  residents unblocked at the final boundary. `cargo tree -p genesis-headless --edges
  normal` contains no wgpu, winit, egui or Bevy dependency.
- Native Apple M4/Metal checks pass: `--people-test`, `--world-test`,
  `--interaction-test`, `--save-test`, with captures under `/private/tmp/genesis-people-*`.
  Inspected the resident capture: visible figures, correct selection ring, home,
  activity and destination. New smoke saves use capture siblings, never the user's
  normal save slot. Non-macOS graphical/lifecycle paths remain unverified.
- No performance improvement is claimed. Existing extraction benchmarks explicitly
  retain their original zero-person workload. This slice supports a small initial
  population; figures are placeholders without skeletal animation or citizen
  shadow casters. Local routing and lack of person-to-person avoidance are recorded
  limitations, not a population-scale claim. See [resident decision](residents.md).
- Next small slice: one resource stock and one useful resident task, with explicit
  reservations, completion and cancellation. ECS remains optional until actual
  storage/query evidence justifies it.

## 15 October 2026 — household water collection

- Inspected project/workspace instructions, engine/vision and resident/save decisions;
  traced `sim_core::{people,subsurface,runtime,save}`, storage and the app's session,
  inspectors and picking. Clean starting point `203433b`; baseline `mise run ci`
  passed 198 tests, 14 optional tests ignored. No baseline failure was found.
- Wells recorded only cumulative recovery; residents had no stocks or useful task.
  Added `sim_core/households.rs`, `people/water.rs` and `subsurface/supply.rs`:
  bounded well/home stocks, deterministic task claims, carried water, pickup and
  delivery. Existing extraction rates remain; excess buffer inflow is explicit
  outflow. One dwelling owns its stock, one resident owns cargo, and reservations
  derive from tasks. No ECS, dependency upgrade or renderer replacement.
- Intentional gameplay change: residents replenish low household water and consume
  small portions every 30 logical seconds. Shortages are inspectable without health
  penalties. Cancelled/stopped/blocked outward trips release claims; blocked carriers
  retain cargo and their delivery commitment. Existing social routines continue.
  App inspectors and a visible container derive from this state; cancellation queues
  an `Action`. New domain facts carry logical time and affected identities.
- Schema 4 / rules 3 persist stock, use phase, cargo, routes and claims. Staged V1–V3
  migration retains resident identities and routines, initializes empty home stock,
  and records historical well recovery as released water. V1/V2 fixtures are intact;
  the V3 fixture was captured with the prior executable before edits. Content and
  generator versions are unchanged; earlier binaries cannot read new saves.
- `mise run ci`: **206 tests pass**, 14 optional tests ignored; rustfmt and Clippy
  with warnings denied pass. `mise run build` and `git diff --check` pass.
  Tests cover conservation, overflow, six homes consuming delivered water, scarce
  stock/shared-home claims, cancellation, stopped wells, changed passability, retained
  cargo, migration, malformed commitments, encoded continuation, frame partitions,
  inspector layout and queued UI cancellation. Initial fixture failures exposed
  out-of-bounds inherited deposits and insufficient authored aquifer supply; those
  fixture inputs were corrected without relaxing domain checks.
- CPU-only `genesis-headless --scenario water`: save at tick 100 with 2 u in transit,
  reload and advance 1700 ticks. The resulting full snapshot equals an uninterrupted
  1800-tick run, excluding host-created world identity. All six homes consume water.
  `cargo tree -p genesis-headless --edges normal` has no wgpu/winit/egui/Bevy closure.
- Native `target/debug/genesis-app --water-test /private/tmp/genesis-water-final.png`
  passes on Apple M4/Metal: drilling in coast seed 7319, pickup, picking, delivery
  after a pump stop, and exact save continuation with the stop queued. Inspected the
  capture: selected carrier, container and correct supply/cargo inspector. Existing
  `--people-test`, `--world-test`, `--interaction-test`, `--save-test` all exit 0;
  captures use `/private/tmp/genesis-water-{people,world,interaction,save}.png`.
  Smoke saves are separate from the user slot. Other graphical hosts remain unverified.
- No performance improvement is claimed; the existing bounded local routing and
  small population remain the supported workload. Broader economy, health effects,
  demolition and generalized job scheduling are absent. See [water ownership and
  limits](household-water.md). Next small step: a settlement supply overview that
  identifies homes lacking reachable water before introducing additional resources.
