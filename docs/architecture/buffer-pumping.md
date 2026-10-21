# Buffer-controlled well pumping

21 October 2026, following resident crop tending.

Select a borehole in **Survey → Hydrology** and enable **Fill buffer only**.
The action commits at the next simulation tick. Extraction then stops when its
20-unit collectable buffer is full and resumes when collection opens space. The
well stays enabled, so household and crop collection claims remain valid. Its
status distinguishes automatic waiting, manual stop, submersion and no yield.
The Water overview shows pump policy, status and cumulative released water.

New wells and older saves default to **Continuous**, preserving the existing
behavior until the player changes it. Continuous pumping can release overflow.
**Stop pumping** overrides both policies, retaining the existing cancellation of
uncollected resident tasks. Changing policy never implicitly resumes a stopped
well or returns previously released water to the aquifer.

## Ownership and numerical contract

`subsurface::pumping` owns the typed policy, buffer-room query, operational status
and limited withdrawal. `SetWellPumpMode` targets a stable well ID and emits
`WellPumpModeChanged`, or the existing `WellRejected` for a missing target.
There is no timer, pump latch, scheduler, or duplicate copy of buffer stock.

Columns recharge first; wells then draw in persistent-ID order. Fill-buffer mode
limits each withdrawal to available buffer space. It books the actual representable
loss from each f32 source column into the f64 well account. If rounding would
exceed receiving capacity, the next source balance is rounded upward instead.
This also handles duplicated edge taps. Room below 0.0001 units counts as full;
that tiny remainder stays underground rather than causing repeated tiny attempts.
Continuous mode retains its earlier withdrawal and yield arithmetic.

```text
well recovered = well stored + well collected + well released
well collected = household stored + household consumed + resident cargo + crop applied
```

Limited extraction adds no overflow release. It preserves water for later use;
it cannot replenish an exhausted aquifer, increase permeability or make demand
sustainable. Recharge remains the existing prescribed scenario process. Historical
released totals can include pre-collection-model extraction from older saves.

## Persistence

Schema 10 / rules 8 stores `Well::pump_mode`. V1–V9 migrate in stages; pre-V10 wells
retain Continuous with unchanged stocks, routes, resident tending and cargo.
`crates/storage/fixtures/v9-tending.json` was captured at tick 500 before this change.
Legacy envelopes reject non-default pump modes or pending policy commands. Unknown
mode values fail decoding. Full-buffer and working-buffer saves continue exactly,
including pending changes between both policies. Older executables cannot read V10.

## Reproduction and validation

```sh
mise exec -- cargo run -p genesis-headless -- --scenario irrigation --ticks 6000 --commands crates/sim_core/fixtures/conserving.commands.json
mise exec -- cargo run -p genesis-headless -- --scenario irrigation --ticks 6000 --commands crates/sim_core/fixtures/tending.commands.json
```

The same finite-aquifer fixture and crop-tending policy run in both cases. Only the
first enables buffer control. Tests compare harvest outcomes, water conservation,
full-buffer waiting/refill, resident reservations and delivery, manual stop and
submersion, continuous overflow, fractional/edge taps with large steps, and stable
arbitration between wells sharing scarce water. Storage tests protect V9 migration
and policy continuation. A real egui test checks that paused input queues a change
without mutating the world or implicitly enabling the pump.

Native `--survey-test output.png` now checks full-buffer conservation and saved
pending mode changes, then captures the enabled survey control. Test saves use a
sibling temporary path, separate from the player's world slot.

No energy cost, mechanical pump wear, pressure model, storage construction,
irrigation network or measured performance improvement is introduced.
