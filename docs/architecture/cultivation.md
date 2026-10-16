# Decision: one renewable crop using the existing food task

Accepted 16 October 2026, following finite food gathering.
This slice adds a barley plot and preserves the existing food/water task arbitration.
It is a demonstration crop model, not calibrated agriculture.

## Ownership and commands

`food::Source` owns either a finite wild harvest or a barley plot with growth and production state.
Both use the same stable `SourceId`, allocator, reservation, pickup and delivery boundary.
The simulation owns all growth; camera visibility, animation and renderer rebuilds have no role.
The conservation equation is now:

```text
initial wild food + cumulative crop production
    = source stock + household stock + resident cargo + consumed portions
```

Production is an explicit source term, counted once when a crop matures.
Reservations remain a subset of standing food, not additional inventory.
`CropPlanted`, `CropPlantRejected` and `CropMatured` events identify committed outcomes and logical time.
The existing food events record collection and delivery.

`Action::PlantCrop` validates a continuous position at execution.
The plot is a circle four metres across, sampled at half-metre spacing for dry, walkable, gentle ground.
It must clear buildings, paths, wells, trees, rocks, rivers, other plots and wild-food patches.
Later building/path/well placement also checks plot footprints.
Well preview and execution share `SimState::validate_well`.
There are at most 64 crop plots and 10,000 total food sources per world.
Rejected placement consumes no identity or stock and emits no planting fact.
Planting is currently free; there is no seed inventory, sowing labour, demolition or plot relocation.

## Growth and harvesting

Barley needs 60,000 logical milliseconds at full growth rate to produce 24 portions.
The rate is quantized to an integer from 0 to 1,000, derived from interpolated environmental moisture.
Growth stops at moisture 0.2 or less and reaches full speed at 0.6 or more, with linear interpolation between them.
The saved accumulator stores rate-weighted milliseconds; there is no wall-clock input.
The climate update runs before growth at each fixed tick.

Existing `rules/climate.rs` moisture is an ecological proxy recomputed from terrain, rainfall and drainage.
It is not a conserved soil-water volume, and growth does not debit it or household water.
Irrigation and real soil-water accounting remain separate future work.
Temperature, fertility, seasons and nutrition do not affect this first crop.

Mature food stays in the plot until collected, with no spoilage.
Residents reserve and harvest four portions using the same two-second gathering task and home capacity checks as wild food.
After the standing harvest is entirely removed, the plot automatically begins its next growth cycle.
This represents automatic resowing in the prototype; it is not a biological claim that barley is perennial.
There is no accumulation of future harvests while stock remains in the plot.
Dryness pauses unripe growth without erasing progress; already mature food remains harvestable.
Flooded, obstructed or excessively steep ground pauses growth and prevents new pickup, retaining progress and food.
The existing cancellation/retry rules preserve carried food and release uncollected claims.
Local navigation can still report an unreachable route even when the plot itself is usable.

## Presentation and saves

**Build → Barley plot** offers a continuous footprint preview and shared validation result.
The selected plot reports moisture, growth percentage, cumulative production and blocked/dry/harvestable state.
**Supplies → Food** includes both wild patches and crops.
Disposable geometry draws rows at current ground height, from short green seedlings to golden ears.
Rows thin as food is collected; small ground markers remain pickable throughout the cycle.
Placeholder geometry and the existing renderer are retained.

Schema 6 / rules 5 persist the source-kind discriminator, growth accumulator and cumulative production.
New saves reject a missing kind, invalid growth range and impossible production/stock accounting.
Staged V1–V5 migration preserves existing stocks, identities, residents, routes, cargo and priorities.
V5 missing source kinds become wild sources; migration does not create plots or production.
The V5 fixture was captured with `9dafadf` before editing, with food work already active.
Older worlds can now plant crops through the same command without starting over.
Generation recipe 10 and builtin content identity remain unchanged because plots are player-created and crop rules are compiled.
Older executables cannot read V6 saves; loading an old world does not promise old-rule replay equivalence.

## Validation and limits

The CPU-only `--scenario cultivation --ticks 6000` fixture has six households, one well and one crop, with prescribed rainfall 0.9.
The dry comparison uses rainfall 0.01 with the same initial settlement.
Tests cover repeated harvests, both supplies reaching every home, production accounting, dry/slow/blocked growth, command ordering, conflicting footprints, frame-duration partitions, save continuation during growth/gathering/carrying, pending planting and malformed records.
UI checks exercise tool switching, queued placement, immutable inspection and compact layouts.
The native `--crop-test` extends the generated-valley water check with planting, seedling picking, maturity and exact save continuation during growth.
Its capture and save sibling use the supplied temporary path, separate from the user's save slot.

The workload remains six residents, with no population/FPS performance claim.
Supply timing and routes still cause missed portions; crop production does not guarantee continuous household sufficiency.
No ECS, general job scheduler, new worker, dependency upgrade or renderer replacement is introduced.
Next useful work is an explicit irrigation intervention, after defining which water stock owns the applied water and how soil storage and losses are accounted.
