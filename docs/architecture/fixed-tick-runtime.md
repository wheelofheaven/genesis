# Decision: retain rules behind a fixed-tick running world

Accepted 12 October 2026 for the first architecture slice.

The existing simulation crate already owns domain advancement.
The app supplied a constant 100 ms timestep, but its public mutable `SimState` and host-owned tick constant left that boundary unenforced, and no CPU execution CLI existed.

`runtime::Simulation` now takes ownership of an initialized `SimState`, advances only through ordered action slices at the existing 100 ms duration, and exposes an integer completed-tick counter.
Tick zero means the supplied initial world; the next action batch belongs to tick one.
Generation's zero-duration initialization steps remain unchanged and do not consume gameplay ticks.
`SimState::step` remains available to generators, tests, and the excluded legacy workspace.

Read-only `Deref<Target = SimState>` deliberately preserves all existing borrowed inspectors/extractors and worker snapshot cloning without moving or rewriting them.
There is no `DerefMut` or mutable state accessor; a compile-fail doctest protects this boundary.
Mutable setup stays possible before the world is handed to `Simulation`.
Replacing the running world creates a new timeline and existing session replacement clears pending actions/presentation state.

Keep the old floating `SimState::time` accumulator because changing its numerical accumulation is unnecessary for this extraction.
It is not independently writable through the runtime; logical command ordering uses integer ticks.
`SimConfig::tick_dt` remains legacy configuration and does not override this fixed gameplay cadence, matching the pre-refactor app.
The host clock retains pause/fraction, speed and overload policy: at most eight actual steps per frame, with excess wall-time debt discarded rather than logical steps skipped.
No cross-platform lockstep or offline progress is added.

The separate `genesis-headless` package keeps file I/O/JSON parsing/reporting outside sim_core and excludes graphics from its resolved dependency closure.
It uses the same runtime, not a second scheduler/rules implementation.
The initial subset provides scenario/seed/tick selection, bounded action-file playback and metrics.
At the first checkpoint, world persistence and a canonical fingerprint were deferred.
The subsequent [v1 save schema](saves-v1.md) restores domain meaning and the timeline explicitly, rather than treating `Simulation::new` as a load function.
A canonical fingerprint remains deferred.

No existing library versions, storage, rule order, action validation, numerical formulas, or renderer paths change.
See [source findings](current-state.md) and [verification](refactor-log.md).
