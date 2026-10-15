# Decision: typed residents and a first routine

Historical first-resident checkpoint: [household water](household-water.md) now adds
collection, delivery and periodic use through these stores. New saves use V4.

Accepted 15 October 2026. This is the first individual-life slice, not an ECS
migration or an economy. The existing Rust/wgpu engine and dependencies remain.

## Ownership and behavior

`sim_core::people::People` owns a private vector of `Person` records and a
persistent monotonic allocator. `PersonId` is distinct from a building ID or
render index. Read APIs expose immutable records. Each resident retains a name,
home ID, outdoor home threshold, position, individual meeting spot, activity,
blocking reason, remaining logical duration and committed route/cursor.

New inhabited generated scenarios seed up to six residents at safe home
thresholds after vegetation generation. The default coastal seed 7319 has six.
Other seeds may have fewer suitable thresholds; generation never spawns someone
inside an obstacle. The World Builder stays unpopulated. Construction does not
create additional residents. Names are illustrative, not a researched historical
population. `scenarios::living_settlement` is a small flat six-person fixture;
the original coastal fixture stays unchanged and unpopulated.

The routine is resting → walking to the meeting place → visiting → returning
home → resting. Initial departures stagger by one second; visits last eight
seconds and subsequent rests twelve seconds. These short durations demonstrate
behavior, not an implemented day/night or social model. Homes have no traversable
interior, so residents rest at their outdoor threshold. There is no hunger,
production, inventory, household, crowd collision, assignment or birth mechanic.

Advancement occurs in `SimState::step`, after actions and environmental updates,
before time publication. Both hosts use the existing fixed 100 ms `Simulation`
tick. Duration counters use integer milliseconds; legacy direct `step(dt)` callers
round their supplied interval to milliseconds. Arbitrary timestep equivalence is
not promised. Movement is capped at 1.2 m/s, stops at waypoint boundaries, and
checks current physical passability before committing. New facts use
`SimEvent::PersonRoutineChanged`; they carry identity, activity, blocking reason
and the existing logical seconds value. No host time enters decisions. The
existing global floating time accumulator is retained; completed ticks remain
the runtime's integer boundary.

## Bounded local navigation

The synchronous A* uses a 0.5 m search lattice, a five-metre margin and at most
128 × 128 nodes. Positions and destinations remain continuous. The lattice is
neither the environmental sample grid nor a building placement grid. Integer
costs, fixed neighbor order and node-index tie breaks make route choice stable.
This is local outdoor navigation, not a regional planner. Narrow passages below
its search resolution may report no local route.

Ground queries reuse `surface::walking`, including footprint, tree/rock, slope,
shore and river exclusions already used by the ground camera. A scoped query
filters cover candidates once for a search, without retaining an authoritative
navigation cache. Every movement sweeps the current surface again, so changed
terrain/occupancy cannot be bypassed using an old route. Failure retains position,
reports a reason and retries after two logical seconds. Unsafe ground underfoot
stops the resident until it becomes safe; no teleport or wall-clock job completion
chooses a recovery. There is no asynchronous work or unbounded queue.

## Presentation and interaction

The app draws small procedural figures through the existing reusable dynamic
vertex buffer. A six-entry previous-position map interpolates committed positions;
world replacement clears it. Selection rings and CPU triangle picking use the
same displayed positions and geometry. Selection tracks stable identity as the
person moves. The inspector shows home, activity, destination and blocking reason;
the HUD counts residents. Character geometry, basic gait, colors and interpolation
are disposable presentation. No GPU types enter simulation or saves.

Figures are placeholders: no skeletal assets, citizen shadow caster integration,
LOD, crowd avoidance or large-population performance claim. The first six figures
are extracted each displayed frame; broad world extraction retains its previous
behavior. Rendering benchmarks explicitly clear residents to preserve their
existing unpopulated comparison workload. No new FPS or speedup claim is made.

## Save compatibility

Captures use schema 3 and `genesis:rules/2`; builtin content stays version 1.
Schema 1 first receives the existing identity migration, then schemas 1/2 migrate
to an empty resident store. Existing people-free worlds are never populated on
load. Their terrain, quantities, identities, time and pending actions are retained.
The old files are untouched by reading; existing recoverable writes retain backups.
Older executables reject schema 3 instead of silently discarding residents.

The save includes all resident identities, allocator history, references, routes,
route cursors and duration counters, so saving does not complete a journey or
trigger a departure early. Validation rejects absent V3 stores, duplicate or
invalid IDs, missing homes, invalid positions/cursors/durations and incompatible
metadata. Resident count is capped at 10,000 for import, route nodes at 16,384 per
person and 100,000 overall, within the existing 128 MiB decode limit. These are
robustness limits, not supported population-performance targets. Current terrain
need not match an old route: movement revalidates it after loading.

`crates/storage/fixtures/v2-objects.json` preserves the former V2 representation;
both V1 fixtures remain unchanged. JSON round trips verify subsequent behavior,
not just immediate equality. Native macOS persistence remains the proven host;
no browser or console support is added.

## Reproduce

- `mise run run`: start a **new Settlement**, zoom to the homes, click a resident,
  use F to focus, and use time controls to observe visiting and returning.
  Previously saved worlds correctly remain unpopulated.
- `mise exec -- cargo run -p genesis-headless -- --scenario settlement --ticks 1200`:
  run the compact routine fixture; `people` in the JSON report contains records.
- `target/debug/genesis-app --people-test /tmp/genesis-people.png`: native generated
  settlement check, selection capture and a separate `.save.json` sibling.
- `mise run ci`: determinism, storage reordering, complete routine, blocked-ground
  and route changes, save continuation, frame partition, interpolation/picking,
  world replacement and existing gameplay regression checks.

The next useful slice is a real resource owner and one resident task with a
reservation and cancellation contract. Introduce ECS only if demonstrated query
or storage problems justify it; it is not needed to add that mechanic.
