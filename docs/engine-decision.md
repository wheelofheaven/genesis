# Custom Rust engine decision

Accepted October 10, 2026.

Genesis will use a focused custom Rust engine assembled from established
libraries. The starting foundation is `winit` for windowing and input and `wgpu`
for graphics, following the working custom prototype in the engine lab.
Bevy is now the legacy prototype, rather than the target architecture.

## Rationale and accepted costs

Owning development priorities, architecture, and contribution workflow matters
to this long-term project. A focused engine lets Genesis develop its simulation,
illustrated renderer, and authoring tools together on its own terms.

This is a choice for control and independence, not a demonstrated performance
victory over Bevy or a prediction about Bevy's future. Comparative compile-time
and runtime advantages remain unproven. The custom prototype renders on Apple
M4 / Metal, but has not established a complete game or content pipeline.

We accept responsibility for rendering integration, asset management, animation
integration, culling, picking, tools, and platform testing. Reuse suitable
libraries instead of recreating commodity infrastructure. Those libraries still
have their own maintenance schedules and contribution policies.

## Architecture boundaries

- `sim_core` remains pure Rust, deterministic, portable, and testable without a
  window or GPU. No rendering, windowing, UI, or engine entity types belong here.
- The application translates input into explicit simulation actions and handles
  time controls. Simulation rules remain in the simulation core.
- Presentation consumes visible state and events. Persistent person IDs belong
  to the simulation; GPU resources and presentation IDs do not enter saves.
- Rendering owns cameras, materials, meshes, GPU resources, and visual effects.
  Start from the lab's direct wgpu renderer and expand only for concrete needs.
- Tools expose simulation reasoning, relationships, resources, and events.
  The foundation uses egui for its immediate-mode inspector, without committing
  to it for the final player interface.
- Saves use explicit, versioned records with a migration strategy. Rendering
  objects must be reconstructible from world state and content.

Use bounded simulation work per presentation update and a deliberate policy for
falling behind real time. Preserve fixed simulation ticks and reproducible action
ordering. Off-screen people continue living without instantiated visual actors.

## Transition plan

The main workspace now runs the custom application with simulation-backed terrain,
camera navigation, a fixed clock, and an egui inspector. The Bevy prototype is
preserved in `legacy/`. The engine lab remains an isolated historical experiment.

1. **Establish the custom application.** Integrate the validated renderer into
   the main development workflow, separate application lifecycle from rendering,
   add camera navigation, and connect an adapter to the existing simulation.
   Provide one documented run command. Preserve the old prototype as a reference
   during the transition; direct new engine development toward the custom path.
2. **Prove the content workflow.** Load one animated character, support selection,
   and show a basic inspector. Test close and settlement views using the working
   Sable-inspired visual direction. Choose libraries when these tasks require them.
3. **Deliver the first living settlement.** Add a small community with persistent
   identities, needs, personalities, aspirations, relationships, and consequential
   intervention, following the product vision.

These are scoped milestones, not deadlines. Do not require further Bevy parity
work to reopen an already accepted engine decision. Keep the comparison fixture
and earlier evaluation as historical evidence; performance measurement remains
useful for optimizing the chosen implementation.

## First milestone acceptance criteria

- A documented command starts the custom application on macOS.
- The player can navigate a small scene and inspect simulation-backed state.
- Input produces explicit actions, and visible changes come from the simulation.
- Pause and resume preserve simulation state and fixed-tick behavior.
- Resize and shutdown handle GPU resources cleanly; zero-sized windows do not
  create invalid render targets.
- Simulation invariants, formatting, relevant Clippy checks, and a rendered
  smoke test pass. Record actual iteration timings without implying a Bevy comparison.

Avoid a general-purpose editor, universal engine abstraction, plugin marketplace,
or custom implementations of every subsystem. Build the infrastructure required
by playable Genesis milestones.

## Foundation validation — October 10, 2026

Validated on macOS / Apple M4 / Metal with Rust 1.93.1:

- Workspace tests: 24 passed (clock, action delivery, picking, camera, and core rules).
- Workspace Clippy including all targets: no warnings; formatting check passed.
- Native smoke test: pause, queued terraforming, single-step, camera movement,
  zero-sized surface suspension, restored surface, resized window, and PNG capture.
  The captured scene and inspector were visually reviewed, including Retina sizing.
- Legacy workspace Cargo metadata resolves; its native build was not repeated.

Observed Cargo-reported times: initial workspace check 20.58 s; test build/run
38.98 s; subsequent application build 1.19 s; renderer/application rebuild after
small edits 2.14 s; subsequent Clippy 0.54 s. These are individual development
observations with cached dependencies and mixed incremental state, not controlled
benchmarks or evidence of an advantage over Bevy.

The scene uses procedural placeholder geometry. Character assets, animation,
individual life simulation, saves, and final illustrated rendering remain future
work. The existing ecology rules have not been redesigned by this migration.

## Organic settlement iteration

The application now renders continuous terrain and water, a curved authored path,
and six freely positioned dwellings. `sim_core::spatial` samples the environmental
field; `sim_core::settlement` owns rotated footprints and placement validation.
Commands are revalidated when applied, so queued overlapping buildings cannot
bypass preview checks. The app provides a site marker, rotation control, and
placement feedback. Building/path data contains no GPU or windowing types.

Validation: 28 workspace tests, warning-free Clippy, formatting, and native Metal
smoke capture. Tests cover continuous/wrapped bounded sampling, fractional picking,
rotated footprint rejection, command ordering and replay, and finite terraforming.
The smoke test adds a fractional-position dwelling through the same action queue.

Road editing, obstacle-aware entrance routing, people, and damage to existing
buildings after terrain changes remain future work. These are placeholder meshes;
this iteration establishes spatial behavior rather than final Sable-style art.

## Subsurface survey iteration

`sim_core::subsurface` owns serializable aquifer columns, mineral bodies, and
boreholes. Explicit drilling and pump-state actions produce simulation events.
Withdrawal is conservative and bounded, recharge is prescribed, and fixed
underground elevations are independent of changes to surface elevation. The app
owns independent layer toggles, map colors/outlines, legends, and depth profiles.
Switching views does not advance or modify the simulation.

The model deliberately separates storage from permeability. The conceptual
reference is [USGS: Aquifers and Groundwater](https://www.usgs.gov/water-science-school/science/aquifers-and-groundwater);
the gameplay implementation is an uncalibrated column model in game units, with
no lateral flow or confined pressure solver. Scenario fields are synthetic, not
data for a real historical location. See the README for current limitations.

Validation: 35 workspace tests passed (one optional timing test ignored), including
withdrawal conservation, bounded depletion, recharge after pumping stops, rejected
sites, deterministic replay, and underground datum stability after terraforming.
Native Metal smoke checks exercise drilling, drawdown, and unchanged world picking;
screenshots cover separate and combined survey layers.

## Seeded regional generation

The application now starts `sim_core::worldgen::generate(seed)`. The old authored
coastal scenario remains a focused regression fixture. Generation is split into
noise, terrain/weathering, drainage, geology, and settlement modules; it contains
no rendering types or dependencies. Recipe version 1 produces a 96 × 80 field
and continuous features. Recreating a seed also reproduces formations and settlement
placement. The app invalidates its picking cache and clears queued actions on
region replacement.

Drainage uses an eight-neighbour priority flood, strict downhill receivers, and
reverse-order rainfall accumulation. The conceptual reference is
[Barnes, Lehman, and Mulla: Priority-Flood](https://arxiv.org/abs/1511.04463).
Slope redistribution and channel incision are bounded generation heuristics;
they do not constitute a calibrated erosion or sediment transport solver.
The watercourses are a generation snapshot, with smoothed presentation paths.
Terraforming does not yet recompute drainage or flooding. The README lists the
current generator's scope and controls.

Validation: 38 workspace tests passed (one optional timing test ignored), including
seed repeatability/variation, acyclic downhill routing, catchment rainfall
conservation, and six-seed checks for finite state, resources, and valid settlements.
Native Metal checks exercise region replacement, queued-action reset, aquifer
drilling, and combined surveys. Captures of seeds 7319 and 7320 were inspected.

## Physical scale iteration

Geometry now uses 20 metres per world unit. Shared simulation constants define
6.5 × 4.8 m dwellings, 2.8 m walls, a 1.2 m maximum foundation step, metre-scale
clearance, and narrow footpaths. Rendered doors, parapets, well equipment, trees,
and boulders use compatible proportions. Building meshes and placement previews
derive their footprint from the same constants as collision validation.

The generator evaluates neighbouring plots when choosing settlement sites and
uses a tighter layout (recipe version 2). The camera supports closer inspection;
a metric scale bar makes distances visible at every zoom. This calibrates geometry
only: simulation time and resource volumes still use prototype units.

The current scale pass uses a 45° perspective camera, with analytic picking rays
to preserve precision at close zoom far from the origin. The scale bar is local
to the focal plane. Reversed floating-point depth preserves thin surface details
from house inspection to regional views, and river geometry splits at terrain
triangle edges to avoid disappearing below creases. Recipe 5 samples vegetation independently at jittered 10 m
intervals and stores a local width profile on each river. Shared GPU meshes and
32-byte instances allow denser cover without duplicating tree geometry in world
uploads. Placements refresh after terrain edits, world replacement, and building
changes; ordinary ecological recoloring reuses the instance buffers.

## Related documents

- [Project vision](vision.md)
- [Engine evaluation and experimental evidence](engine-evaluation.md)
- [Comparison lab](../experiments/engine-lab/README.md)
