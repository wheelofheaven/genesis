# Resident crop tending

21 October 2026, following irrigation and menu saves.

Select a barley plot and enable **Residents tend this plot**. The checkbox queues
`SetCropTending`; a paused game commits it through Step or Resume. Existing plots
remain manual until enabled. Hovering the setting explains admission and stopping.
The plot inspector names its assigned worker; the resident inspector shows the
well, destination plot, reserved/carried dose and any delivery problem. Water cargo
uses the existing visible container. The Water overview includes both household
and crop reservations and cargo.

## Admission and ownership

At home departure, each resident first tries their household food/water jobs under
the existing priority policy. Failed admissions may fall through to tending, so a
home waiting for a dry crop can help water it. Accepted jobs are not preempted.
There is no global household emergency priority or workforce assignment system.

A tended plot is eligible when its standing harvest is empty, ambient moisture is
below 60%, its water store is at most one unit, and its ground is usable. The
plot and well must each be within 50 m of departure; the well must be within
30 m of the plot. Plots sort by distance to the resident and ID; wells sort by
distance to the plot and ID. At most eight plot/well pairs and sixteen well-side
pickup points per pair are tried. Admission checks routes to the well and onward
to the plot, with movement continuing to check current terrain.

One resident may tend a plot at a time. The task reserves two units in an active,
unsubmerged well and two units of plot capacity. Household jobs and manual
applications share the same derived well reservations. Manual applications also
protect the delivery capacity and lifetime accounting limit reserved at the plot.
No independently mutable reservation counter exists.

At pickup, the well's collection transaction transfers the dose to resident cargo.
The well reservation ends; the plot reservation remains. At delivery, cargo becomes
plot storage and cumulative applied water. Existing drainage and growth rules then
apply. `WaterCollected`, `CropWatered`, routine and tending-status events report the
transitions. There is no separate timed pouring animation in this slice.

Turning tending off or stopping/submerging a well releases uncollected water. A
blocked pre-pickup route also releases the task. Once collected, the resident keeps
cargo and plot capacity until delivery, even if tending is disabled or the well
stops. Blocked delivery retries after two logical seconds. Mature crops can receive
an already committed dose. Water is never discarded on cancellation.

`people/irrigation.rs` owns admission and the resident transaction;
`food/cultivation/irrigation.rs` owns plot transfer/validation. Household and crop
consumption still reconcile against cumulative well collection. During resident
advancement, `People` is temporarily moved out of `SimState`; admission deliberately
reads claims from that active `People`, not the empty placeholder in `sim.people`.

## Saves and validation

Schema 9 / rules 7 introduced the opt-in crop flag, resident task/issue and activity values.
[Buffer-controlled pumping](buffer-pumping.md) subsequently adds pump policies in schema 10 / rules 8.
V1–V8 migrate through existing stages. V8 defaults to manual tending with no new
resident commitments; growth, food, water, routes and expedition role are preserved.
The pre-change `crates/storage/fixtures/v8-irrigation.json` contains active irrigation
at tick 500. Old envelopes reject new tending intent or commitments rather than
silently interpreting them under old rules. Older executables cannot read V9.

Validation checks task exclusivity, well/plot identities and positions, one worker
per plot, stock/capacity/lifetime reservations, activity/cargo consistency, and the
existing global conservation equations. Saved pre-pickup and carrying states,
including a pending stop command, continue exactly after JSON round trips.

## Reproduction and limits

```sh
mise exec -- cargo run -p genesis-headless -- --scenario irrigation --ticks 2200 --commands crates/sim_core/fixtures/tending.commands.json
```

The dry six-home fixture can produce a harvest through resident labour alone.
Its well drains a finite aquifer and can overflow before demand arrives; enabling
tending does not create water or guarantee continued production. A separate test
uses a larger finite aquifer to verify repeated harvests and household consumption.
Tests also cover household admission priority, stable ordering, unique claims,
manual competition, cancellation, blocked cargo recovery, humid plots, malformed
saves and the real paused checkbox. Native `--crop-test` retains manual checks and
round-trips pending tending intent before showing the enabled plot inspector.

There is no sowing labour, seed stock, occupation assignment, crop rotation,
irrigation network, health consequence or physical soil-water solver.
