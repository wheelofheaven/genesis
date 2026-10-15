# Headless Genesis

Run the existing simulation without wgpu, a window, GUI, renderer, or visual assets.
The executable handles arguments, command files and reporting; rules remain in `genesis-sim-core`.

```sh
mise exec -- cargo run -p genesis-headless -- --scenario coast --seed 7319 --ticks 100
mise exec -- cargo run -p genesis-headless -- --scenario fixture --ticks 100 --commands crates/sim_core/fixtures/coastal.commands.json
mise exec -- cargo tree -p genesis-headless --edges normal
```

`--scenario` accepts `coast` (default), `highlands`, `island`, `fixture`, `settlement`, `water`, or `supplies`.
Generated scenarios default to seed 7319; the authored coastal, settlement, water and supplies fixtures use seed 42 and reject `--seed`.
`--ticks` defaults to 100; zero inspects initial state without advancing.
Each gameplay tick runs the same 0.1-second rules step as the interactive application.
The integer `completed_ticks` is the logical boundary; `simulation_time` retains the old floating accumulator and its rounding for behavior compatibility.

Command files use existing serde `Action` values:

```json
{
  "version": 1,
  "commands": [
    { "tick": 1, "action": { "DrillWell": { "position": { "x": 18.25, "z": 21.5 } } } },
    { "tick": 50, "action": { "SetWellActive": { "id": 1, "active": false } } }
  ]
}
```

Use this example with `--scenario fixture --ticks 100`.
Ticks are assigned execution boundaries, starting at 1; equal-tick actions execute in file order before that tick's environmental rules.
Actions are consumed once, and domain validation remains in `SimState::step`.
The runner rejects unsupported log versions, decreasing/zero/out-of-run ticks, files larger than 1 MiB, and more than 10,000 commands before world generation or advancement.
The command file is a developer playback input for the chosen initial world and current build, not a cross-version replay archive or an untrusted-content sandbox.
Existing domain validation limitations also apply here; this slice doesn't redesign every action.

Stdout is one JSON metrics record; diagnostics/errors go to stderr.
It reports seed, generator version, completed ticks, command/event counts, object counts, water storage/recovery, and existing prototype scores.
`rejected_commands` counts explicit terrain/building/path/well and water/food-task cancellation rejection events; legacy life-seeding/outpost actions do not all return rejection events.
Terrain event counts include changed, unchanged and rejected requests.
Metrics are summaries, not a canonical state fingerprint or a world save.
The `people` array reports resident identities and routine state. Generated settlements and
`--scenario settlement` run the same resident rules as the game. The original
`fixture` remains unpopulated. The `water` fixture adds a working well and reports household stocks/use in
`households`. Use `--scenario water --ticks 1800` to exercise collection and delivery.
The `supplies` fixture adds finite wild food. `--scenario supplies --ticks 3600`
reports both resource loops; `food` includes sources, household portions and use.
There is no wall-clock/FPS performance claim.

The checked-in [coastal command fixture](../sim_core/fixtures/coastal.commands.json) terraforms at tick 1, submits two overlapping dwellings at tick 2 (one accepted), drills at tick 3 and stops pumping at tick 50.
After 100 ticks it has seven dwellings, one well and one explicit rejection.
Use the same build/target and initial scenario for reproducibility.

## Save and continue

```sh
mise exec -- cargo run -p genesis-headless -- --scenario fixture --ticks 50 --commands crates/sim_core/fixtures/coastal.commands.json --save /tmp/genesis-half.json
mise exec -- cargo run -p genesis-headless -- --load /tmp/genesis-half.json --ticks 50 --save /tmp/genesis-complete.json
```

`--load` replaces scenario/seed selection; combining them is an error.
`--ticks` means additional ticks after the loaded boundary.
Command-file ticks remain absolute: a log for a tick-50 save must start after tick 50.
Commands outside the run or at already-completed boundaries are rejected before advancement.
Saved pending commands execute at the next tick, before any newly supplied commands at that same tick.
`--ticks 0` inspects/resaves the loaded world without consuming pending intent.

`--save` writes the final boundary and preserves an existing valid destination in a `.bak` sibling.
Use `--load /path/world.json.bak` to recover a previous save; save it to a new filename if the primary is corrupt.
Directories must already exist for CLI output.
The [v5 format with v1–v4 migration](../../docs/architecture/food-gathering.md) stores materialized domain state, world identity, content/rule metadata and timeline; metrics stdout remains a summary rather than the save payload.
