# Decision: finite food gathering alongside household water

Accepted 15 October 2026 after the supply overview checkpoint.
The [16 October cultivation slice](cultivation.md) adds renewable crop sources and schema 6;
this document records the original finite-food rules, which remain in effect for wild patches.
This adds a second explicit resident task; it does not replace the typed stores or introduce an ECS.

## Ownership and units

`sim_core::food::Food` owns finite source records, their persistent allocator,
household food stocks and the integer consumption phase.
Sources have typed `SourceId` values independent of their vector positions.
Food is counted in exact whole portions, separate from water's existing abstract floating-point units.
Each source's `initial` count and each home's `consumed` count are accounting records, not available stock.

```text
initial food = food remaining in sources + household stock + resident cargo + consumed food
available source food = remaining food - derived reservations
```

The save validator requires exact accounting, valid references, finite positions,
capacity bounds, unique source IDs and a valid allocator.
One food task per home reserves destination capacity; before pickup it also claims source food.
Claims are derived from resident tasks, with no independently mutable reservation table.
Each person can hold either a water or food task, never both.
Water's existing records, accounting and `CancelWaterTask` action remain intact.

## Current gameplay rules

New inhabited scenarios seed one reachable wild-food patch near the first resident,
using deterministic candidate positions and the existing local route query.
It begins with 256 portions. This is an authored scenario allowance, not a biomass
conversion, botanical yield estimate or regenerating resource.
No crop, species, season, nutrition, health penalty or agriculture model is introduced.
The patch stays in the world when depleted or obstructed; terrain/occupancy checks
can prevent gathering without erasing its stock.

A home stores at most 12 portions and seeks food below 6.
Each trip claims 4 portions, walks to the source, gathers for 2 logical seconds,
atomically transfers those portions into resident cargo, and carries them home.
Delivery credits the home and clears cargo/claims together.
Each resident consumes one portion every 30 logical seconds; unmet portions are
counted without health effects. These are demonstration balance values.
Consumption runs before resident delivery at each boundary, alongside existing water use.

At home departure, compare each stock divided by its own refill threshold.
Try the lower fraction first. Equal fractions prefer the resource other than the
resident's last completed delivery; the initial tie favors water.
If that task cannot be admitted, try the other resource at the same boundary.
Ongoing tasks finish before reconsidering priorities. Stable person-ID order
arbitrates scarce stocks; source choice is bounded to eight distance/ID-ordered
sources within 50 m. No worker completion or render rate decides task admission.
Supply rates and travel distances can still cause shortages; priority is not a guarantee of abundance.

`CancelFoodTask` validates the person and task at execution.
Before pickup, cancellation, an unsafe source or a blocked route releases claims.
After pickup, cancellation and route failures retain cargo and the delivery claim.
Blocked movement retries through the existing logical timer; no cargo disappears.
Food collection, delivery, issue changes and accepted/rejected cancellation emit
logical-time `SimEvent` facts. Rejected cancellation emits no success/transfer fact.

## Saves and generation

Schema 5 / `genesis:rules/4` persist food stocks, source allocation, reservations,
cargo, gathering/retry timers, consumption phase and last-delivery priority.
Missing priority state is rejected in new saves; only legacy migration initializes it.
Generation recipe 10 adds the food patch; terrain recipe 9 saves remain supported.
The existing builtin content marker stays unchanged because definitions remain compiled rules.

V1–V4 staged migration preserves old water stocks, claims, cargo, people and routes.
It initializes empty food homes and a new food-use phase, with no new source or free stock.
Older worlds therefore need a new settlement scenario to exercise gathering in this slice.
The V4 fixture was captured at tick 100 with water in transit using `07a6ef9` before editing.
Earlier fixtures remain unchanged. New saves cannot be read by older executables.
Loading compatibility does not promise replay equivalence with the old rules.

## Presentation and verification

The **Supplies** button replaces **Water supply**, retaining the water view and adding a Food tab.
Totals distinguish source stock, reservations, cargo and home stock.
Rows explain which resource a resident is collecting and report last-attempt shortages.
Source rows and visible patch geometry resolve stable IDs for selection and focus.
Food patches use small placeholder shrubs with fruit density derived from remaining stock;
empty patches look dry. Roots follow ground elevation, including submerged terrain.
Residents carrying food display a basket. These meshes are disposable presentation,
not authoritative harvest quantities. They use the existing renderer and CPU picking.

The CPU-only `--scenario supplies --ticks 3600` fixture exercises the same rules.
Tests cover both resources reaching all six homes, finite depletion, conservation,
stable scarce claims, priority ties/fallback, blocked routes and retained cargo,
cancel rejection, encoded gathering/cargo continuation, migration, malformed saves,
different frame partitions, read-only UI and cancellation through the command queue.
Native `--food-test` extends the existing generated-valley water check with source
picking, both supplies being consumed, cargo cancellation and exact save continuation.
Captures/save siblings are separate from the user's quicksave.

The tested population is small and native graphics verification is macOS/Metal only.
No performance improvement or scale claim is made.
The existing extraction benchmarks explicitly exclude residents and food sources.
No new graphics dependency, worker, renderer architecture or platform support is introduced.
The next useful design choice is how food is renewed through gathering seasons or cultivation;
depletion is intentional until that mechanic is authorized.
