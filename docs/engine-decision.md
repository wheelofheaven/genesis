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
  The UI library remains to be selected; an immediate-mode inspector is a useful
  first tool, not a commitment to the final player interface.
- Saves use explicit, versioned records with a migration strategy. Rendering
  objects must be reconstructible from world state and content.

Use bounded simulation work per presentation update and a deliberate policy for
falling behind real time. Preserve fixed simulation ticks and reproducible action
ordering. Off-screen people continue living without instantiated visual actors.

## Transition plan

The current main workspace still contains the Bevy application. The working
custom renderer is an isolated experiment. This decision does not claim that
migration has already occurred.

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

## Related documents

- [Project vision](vision.md)
- [Engine evaluation and experimental evidence](engine-evaluation.md)
- [Comparison lab](../experiments/engine-lab/README.md)
