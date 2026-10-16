# Decision: native world saves, schema v1

Accepted 12 October 2026, after the fixed-tick runtime checkpoint.

Historical V1 contract: new saves now use [V6 crop, food and water saves](cultivation.md),
building on [V2 durable identities](durable-identity.md).
V1 files remain supported through staged migration; the original format below
records the assumptions that migration validates.

## Ownership and format

`sim_core::save::Snapshot` owns the persisted domain contract and staged validation.
`genesis-storage` owns JSON encoding, native byte access and file replacement; both app and headless use it.
The separate host crate is justified by these two consumers and keeps filesystem access out of simulation advancement and graphics out of the runner.

The envelope contains `schema = 1`, `rules = "genesis:rules/1"`, `content = "genesis:builtin/1"`, persistent `world_id`, integer `completed_ticks`, ordered `pending` actions, and materialized `state`.
The built-in content marker covers the compiled planet/region/ecology/building definitions; there is no external domain-content pack system yet.
Unknown/incompatible metadata fails explicitly; there are no older world schemas to migrate.
The checked-in `crates/storage/fixtures/v1.json` is the initial compatibility fixture.
Changes to record interpretation require a schema migration or explicit incompatibility, and rule/physical-surface changes require a rules version change.

State includes the complete environment field, config/seed/topology, legacy floating time and research/stability values, outposts, buildings, paths, aquifer columns, deposits, wells/recovery, generated rivers and ecological cover.
These nested records are now part of the v1 contract; renaming their fields or enum variants is not a harmless internal refactor.
Existing serde-defaulted fields keep their declared defaults; current writers always emit them.
Channel bins and river bounding boxes are omitted and reconstructed after validation.
Climate/ecology/water proxy values are preserved exactly rather than recalculated at load.
`serde_json`'s `float_roundtrip` feature preserves the original floating values; continuation tests compare encoded state and events exactly on the supported target.

The host generates a random 128-bit hexadecimal world identifier once for a new world, independently of the scenario seed, and retains it across saves/loads.
It is metadata, never an input to generation or domain rules.
Presentation generations remain transient: successful app loads advance the terrain revision and reset the session's derived UI state so obsolete work cannot replace the loaded view.

## Identity, time and pending work

V1 preserves existing object vectors and IDs without changing gameplay.
Buildings/wells remain append-only, with IDs 1 through N in vector order and next ID N + 1; validation enforces that allocator representation.
Paths retain vector order because existing references are indices.
Deposit IDs must be unique and positive.
This protects today's references and continuation but does **not** authorize deletion/repacking; those require a new allocator/reference contract and a save migration before implementation.

Saving takes an immutable snapshot at a completed tick and copies the host's pending actions without applying them.
Restoration reinstates both the integer counter and the original floating accumulator.
The app starts paused after load; pending actions commit once when the player steps/resumes.
The runner advances an explicit number of additional ticks, admits only command-log ticks later than the saved boundary, and places saved pending intent before new intent at the next tick.
There are no other authoritative jobs, reservations, journeys or live RNG streams in the current simulation to serialize.
Future additions must extend the save contract before they can be discarded at a boundary.

## Validation and budgets

Load proceeds as bounded read → decode → metadata/domain validation → cache reconstruction → staged result → active-session replacement.
Failure leaves the current session and pending actions intact; user-facing error feedback may change.
Validation checks finite/ranged quantities, consistent dimensions/counts, water-depth consistency, current identity assumptions, geometry, positions, pending representations and cache reconstruction work.
It does not rerun placement validation against current terrain: existing valid gameplay permits terrain changes after construction, and pending commands may still be rejected at execution.

Budgets for this initial implementation:

- 128 MiB encoded file limit, checked before decoding.
- Grid axes 1–512, with exactly matching cell and aquifer dimensions/counts.
- At most 10,000 buildings, paths, wells, deposits, rivers and pending commands per corresponding collection.
- At most 500,000 cover records, 1,000,000 combined path/river/pending-path points, and 4,000,000 estimated channel-index entries.
- Explicit finite/range checks and a logical-tick bound below 10^12.
- Only the currently retained authored surface (recipe 0) and generated recipe 9 are accepted.

These are import/storage budgets, not performance/population targets.
An initial 64 MiB cap failed the actual generated highlands workload; 128 MiB was selected after that failure and checked against all three current regions.
Parsing and snapshots are synchronous and allocate full bounded records; no long-session or peak-memory optimization is claimed.

## Native file commits and player access

The shared writer validates before touching the destination.
It creates a unique sibling temporary file with exclusive creation, writes all bytes, syncs the file and renames it into place.
For an existing destination, it first validates the old save and commits that exact file to `filename.bak` through the same temporary-file procedure.
A failed backup or pre-rename write leaves the primary unchanged; temporary files created by this operation are cleaned up.
An invalid existing primary is not overwritten: recover the backup or choose another filename.
Non-regular primary destinations are rejected.

This is a single-writer native implementation, validated on macOS.
It does not claim cross-process writer coordination, browser storage, Windows rename parity, or full power-loss durability beyond native file sync/rename semantics; directory durability is not separately synced.

In a playing app session, F5 saves and F9 loads `~/Library/Application Support/Genesis/world.json` on macOS.
Other native hosts use `$XDG_DATA_HOME/Genesis/world.json` or `~/.local/share/Genesis/world.json`, but are unverified.
Keyboard help lists the controls; function-key mode on macOS may require Fn.
Successful quickload resets camera/selection, retains the current client mode, pauses, and rebuilds presentation from domain state.
Camera, menus, survey toggles, current construction previews, event notifications and wall-clock pacing debt are not world state and are not saved.
The current main-menu Continue action still resumes the in-memory session; saved-world menu integration is separate UI work.

For native verification, `genesis-app --save-test /tmp/genesis-save.png` uses a sibling `/tmp/genesis-save.world.json` rather than the user's slot.
It saves pending drilling, advances, loads, compares the restored boundary, advances again and compares exact continuation before rendering the loaded world.

See [checkpoint evidence](refactor-log.md#second-checkpoint--native-world-persistence).
