# Decision: durable object allocation and save v2

Accepted 12 October 2026 after the native persistence checkpoint.

Historical V2 contract: new captures use [V5 food and water saves](food-gathering.md).
The allocation rules below remain in force; V1 and V2 saves remain supported.

Buildings and wells retain their existing `u64` IDs. Committed paths now also
have `u64` IDs. Each domain owns one monotonically increasing allocation counter:
`SimState.building_ids`, `SimState.path_ids`, and `Subsurface.well_ids`. Counters
start at one, advance only for accepted construction, and persist even when
objects are removed. They are private to the simulation crate. Zero denotes
preview geometry, never a committed object. `u64::MAX` denotes exhaustion; it is
not issued. Exhaustion produces the existing domain rejection event with an
explicit reason, before cover or objects change.

Identity is scoped to a world and domain; `Object::Building` and `Object::Path`
keep selection domains distinct. No UUID/store/ECS replacement is introduced.
Deposit IDs remain the adequate positive, unique generation IDs. There is no
new deletion/demolition mechanic. Test removals demonstrate allocation safety
before any future removal mechanic is implemented.

`PathBuilt` now carries `id` instead of a transient `index`. Picking, selection,
focus, inspection and selection geometry resolve that ID against committed
paths. Equally close picked paths use ID as a stable tie-break. A missing target
cannot redirect selection to the object occupying its former slot. Session load
already resets selection, preventing references from crossing worlds.
Path duplicate detection explicitly compares geometry and width, excluding ID.
Visual entrance walks and draft routes use ID zero and never enter domain state.

Well withdrawal arbitration uses ascending persistent ID. The append-only
baseline already had that order, so existing saves and ordinary play retain
outcomes. Repacked wells are sorted before withdrawal; scarce shared columns
cannot select a winner from vector order. This is a correctness change, not a
measured optimization. Building/path vector order is still serialized; this
slice does not promise bitwise equality of every terrain floating-point reduction
under arbitrary storage reordering. References survive reordering; a future
storage optimization must separately verify numerical/query behavior.

## Save compatibility

New captures use schema 2; rules/content markers remain `genesis:rules/1` and
`genesis:builtin/1`. The numerical rules, authored scenarios and content are
unchanged. V2 adds the three counters and path IDs. Validation requires unique
positive IDs strictly below their domain's next counter, even if the highest
issued object is absent. Missing/default counters and zero/duplicate path IDs
are rejected; loading never guesses missing V2 allocator history.

V1 migration accepts only the old append-only building/well ID sequence and
unidentified paths. In a staged record it assigns path IDs `old index + 1` and
initializes counters to `old count + 1`, then runs all V2 domain validation before
cache reconstruction or world replacement. Logical time, world identity, pending
commands, object geometry, quantities and existing building/well IDs are retained.
No full JSON-tree conversion or second world generator is needed. The original
file is not changed on read; a subsequent write uses V2 and backs up exact prior
bytes through the existing storage adapter.

`crates/storage/fixtures/v1.json` remains unchanged. The added small
`v1-objects.json` fixture contains two buildings, two wells, one path and a pending
well command, exercising actual references and continued allocation. Unknown
schemas and malformed legacy identities fail explicitly. Older executables
cannot read V2. Action-log format is unchanged, but consumers of serialized
`PathBuilt` events must adopt the ID field; no historical event log is stored in
world saves. Cross-build replay compatibility is not promised.

Direct mutable `SimState` access remains available for setup and legacy code.
Production scenario generators allocate through the same counters; live clients
remain behind read-only `Simulation` access and tick actions. Arbitrary hand-made
setup objects must not invent IDs/counter history for a live saved world. Snapshot
validation rejects inconsistent setup data; importing general third-party
scenarios is not introduced here.

Storage byte/count limits, native backup behavior, CPU-only runner support and
platform verification limits remain those of [the save decision](saves-v1.md).
