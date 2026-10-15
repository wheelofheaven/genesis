# Decision: household water collection

Accepted 15 October 2026, following the typed-resident checkpoint.
This adds one useful task through existing simulation, navigation, command and save boundaries.
Rust, the custom wgpu engine and dependencies remain unchanged; there is no ECS migration.

## Ownership and accounting

Volumes retain the aquifer model's abstract water units (`u`), not litres or calibrated hydrology.
Continuous water uses the existing floating-point model; each household container carries exactly 2 u.

| Record | Authoritative quantity | Accounting only |
| --- | --- | --- |
| Aquifer column | Stored groundwater (`f32`) | Existing capacity/recharge parameters |
| Well supply | Collectable buffer, at most 20 u (`f64`) | Cumulative collected and released volumes |
| Resident | Carried water (`f64`) | Source well and active task |
| Occupied dwelling | Household stock, at most 8 u (`f64`) | Cumulative consumed water and unmet-use count |

`Well::recovered` remains cumulative extraction, not an additional inventory.
Pumping rates, withdrawals, recharge and the original recovery total are preserved.
Extracted water fills the new buffer; excess is recorded as external outflow in `Supply::released`.
There is no new modeled surface pond or pipe receiving that outflow.
Stopping the pump disables new collection as well as pumping; stored buffer water remains until service resumes.
Water already being carried continues home independently of pump state.

The accounting identities are:

```text
well recovered = buffer + cumulative collected + cumulative released
cumulative collected = household stock + carried water + cumulative consumed
initial modeled water + recharge = aquifer + buffer + cargo + home stock + consumed + released
```

Cumulative counters are excluded from physical stock totals.
Validation checks the first two identities (relative tolerances `1e-7 * (1 + recovered)` and `1e-8 * (1 + collected)`).
The unrecharged fixture checks the full balance within 0.01 u, matching the existing f32 aquifer tolerance.
No per-cell correction or hidden resource clamp is introduced.

## Household use and task policy

There is one household water store per occupied dwelling, addressed by the dwelling's durable ID.
This is not a new family/social identity model.
Stores start empty; migration and new construction do not create free water.
Every 30 logical seconds, each resident creates a demand of 0.5 u at their home.
Available stock is consumed; unavailable portions are counted for inspection.
There is no health, thirst damage, death or productivity penalty in this slice.
These short intervals are demonstration balance values, not historical water requirements.

At an ordinary departure from home, a resident prioritizes water if stock is below 4 u.
Otherwise the existing visit/rest routine continues.
If there is no available reachable source, the routine continues with an inspectable water issue; residents reconsider at their next home departure.
The source search checks at most eight active wells in distance/ID order, within 50 m, with sixteen possible pickup positions per well.
It uses the existing bounded local route planner and current physical traversal rules.
There is no asynchronous planning or camera-dependent task admission.

One active collection/delivery task claims the household at a time.
Fetching tasks also reserve 2 u of available well stock.
Both claims are derived from resident tasks, with no second mutable reservation table.
Residents advance in ascending persistent ID order, making scarce-stock arbitration independent of vector order.
The household capacity claim persists through delivery; consumption can only free additional space.

At pickup, execution rechecks the working well and stock before debiting the buffer and assigning cargo as one serial operation.
At home, delivery credits the stock and clears cargo/task as one operation.
Current passability is checked on every movement.
A stopped/submerged well or failed outward route releases uncollected claims and sends the resident home.
A blocked carrier retains water and the delivery commitment while retrying; it cannot discard cargo or publish a false delivery.
No well/dwelling/person demolition mechanic is added.

`CancelWaterTask { person }` is validated at execution.
The inspector offers cancellation before pickup; unknown/no-task targets produce a rejection fact without mutation.
Programmatic cancellation after pickup redirects the existing cargo home and retains its capacity claim.
Pickup, delivery, cancellation and issue changes produce `SimEvent` records with logical time and affected IDs.
The existing eight-record client event retention remains; current task state does not depend on those notifications.

Tick order is ordered commands → existing environmental/pumping stages → household consumption → resident planning/movement/transfers → time publication.
Consumption therefore observes stock before deliveries at the same boundary.
Both hosts use the existing 100 ms runtime; fractional render pacing never enters consumption or task decisions.

## Presentation and compatibility

The resident inspector shows household stock, source, reservation/cargo, activity and blocking reason.
Dwelling inspection shows stock, cumulative use, unmet portions and the active supplier.
The survey shows collectable and reserved well water alongside the existing extraction/yield readings.
A small container derives from committed cargo; it is cosmetic geometry using existing rendering and picking.

Schema 4 / `genesis:rules/3` persist household stocks, consumption phase, task claims, carried water and well accounting.
Schemas 1 and 2 retain their existing identity/population migrations; schema 3 receives empty household stocks and no water tasks.
Historical well recovery becomes `released`, so loading cannot turn old cumulative extraction into new collectable water.
Resident IDs, homes, positions, social routes and remaining durations survive migration.
Future behavior intentionally adopts the new household rules; this is save compatibility, not old-rule replay equivalence.
Builtin content and terrain recipe versions remain unchanged.
Older executables reject schema 4.

The checked-in V3 resident fixture was captured with the previous executable at tick 35, before source edits.
V1/V2 fixtures remain unchanged.
Staged loads reject missing new stores, invalid source/home references, inconsistent cargo or reservations, excess capacity and broken water accounting.
Pending commands, routes, cargo and consumption phase survive saves without advancing their commitment boundary.
The existing native recoverable-write adapter and byte/count limits remain in force.
No new browser storage or platform support is claimed.

## Validation and use

- `mise run run`: enter a Settlement, open Survey/Hydrology and drill on suitable clear ground within walking distance of the homes.
  Let the buffer fill and residents finish their current visit/rest cycle; use 4× speed to observe collection.
  Click a resident or dwelling to inspect supply; stop/resume pumping through the survey.
- `mise exec -- cargo run -p genesis-headless -- --scenario water --ticks 1800 --save /tmp/genesis-water.json` runs the compact six-home fixture with a central well.
- `target/debug/genesis-app --water-test /tmp/genesis-water.png` drills in generated coast seed 7319, checks pickup/picking/delivery, and verifies exact save continuation with a pending pump stop.
  The capture's `.save.json` sibling is separate from the user's quicksave slot.

Tests cover conservation, overflow accounting, all six households receiving and consuming water, stable scarce-stock arbitration, shared-home claims, cancellation, stopped wells, changed terrain, held cargo, V1–V3 migration, corrupt commitments, encoded continuation, frame partitions, UI cancellation and inspector layout.
Native verification uses Apple M4/Metal; other graphical hosts remain unverified.
No performance improvement or population-scale target is claimed.
The next useful step is a small supply overview showing which households lack reachable water, before expanding into additional jobs or needs.
