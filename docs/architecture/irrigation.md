# Explicit irrigation from well buffers

21 October 2026, continuing the [barley cultivation slice](cultivation.md).

Select a barley plot and choose **Apply 2 u water** in its inspector.
The expedition applies water immediately at the next simulation tick from the nearest eligible well within 30 m.
Existing household collection reservations remain protected.
The command records explicit plot and well identities; it revalidates both when executed and does not silently substitute another well.
While paused, Resume or Step commits the application.

This command is a manual expedition intervention. The subsequent [resident tending slice](resident-tending.md) adds optional walking water delivery alongside it.
It uses the same abstract water units as wells and households, with no claim of calibrated soil physics, litres, crop yields or historical irrigation practice.

## Storage and growth

Each plot owns a four-unit irrigation store.
Each application transfers exactly two units from an active, unsubmerged well's unreserved buffer.
The whole dose must fit; rejected commands transfer nothing and emit a rejection event.
Nearest eligible well selection uses distance then persistent well identity, independently of vector order.

Water drains at 0.05 units per logical second, representing a combined evaporation/drainage sink.
A dose therefore lasts 40 logical seconds.
Loss continues on mature, flooded and obstructed plots, and after their standing harvest has been cleared.
No water is returned to the aquifer or created by ambient rainfall in this first slice.
The inspector separates ambient climate moisture from the growth condition supplied by irrigation.

While water remains, the crop receives at least the existing 60% full-growth condition.
Otherwise the original ambient-moisture growth rate applies.
For an interval in which the store empties, only its wet portion gets the irrigation benefit.
Blocked ground still prevents growth and harvest.
A dry crop needs 60 logical seconds of supplied growth to mature, so one dose alone is insufficient.
Water can be wasted on a mature or naturally moist plot; the inspector exposes storage, losses and harvest status.

## Ownership and conservation

`food/cultivation/irrigation.rs` owns transfer validation, source selection and plot-water bookkeeping.
Plot storage and counters use integer millionths of an abstract water unit; loss is 50 millionths per logical millisecond.
Well buffers retain their existing floating-point units and accounting.

```text
well recovered = well stored + well collected + well released
well collected = household stored + household consumed + resident cargo + crop applied
crop applied = crop stored + crop lost
```

Irrigation debits the well's existing collection transaction and credits the plot atomically.
It cannot debit already reserved household or tending water, or fill capacity reserved by a resident at the plot.
Both forms of irrigation can compete with future household demand; there is no promise of settlement sufficiency.
The Water overview reports cumulative crop applications separately from household stocks and cargo.

## Save compatibility

Schema 7 / rules 6 introduced each crop's irrigation stock, cumulative applications and loss.
The [menu-save slice](menu-saves.md) adds expedition role in schema 8; [resident tending](resident-tending.md) adds opt-in hauling in schema 9 / rules 7.
V1–V6 saves migrate through the existing stages; V6 crops gain empty stores without changing growth, food, residents, routes or water commitments.
The checked-in V6 fixture was captured from `5ef1af4` at tick 100 before implementation.
New-format saves reject missing storage, impossible balances, excessive capacity, invalid increments and plot water without corresponding well collection.
Pending applications retain their exact targets and execute once after load.
Generator recipe 10 and builtin content identity remain unchanged.
Older executables cannot read V7 saves; old-rule replay equivalence is not promised.

## Reproduction and limits

```sh
mise exec -- cargo run -p genesis-headless -- --scenario irrigation --ticks 2200 --commands crates/sim_core/fixtures/irrigation.commands.json
```

The fixture is the six-home cultivation settlement with rainfall 0.01.
The command file applies two doses at tick 201 after the well has filled.
Without that command file the crop stays dry.
Food records expose the harvest and exact plot-water accounts.
The native `--crop-test` checks plot picking, application, active and pending irrigation save continuation, and captures the selected plot's water inspector.

There is no irrigation channel construction, flow of irrigation water across terrain, soil infiltration solver, nutrient transport, seasonal weather or health consequence.
Character animation remains a separate open milestone.
