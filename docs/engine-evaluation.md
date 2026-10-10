# Genesis engine evaluation

October 10, 2026. Historical evaluation. The subsequent
[accepted decision](engine-decision.md) selects a custom Rust engine. The findings
and validation limits below remain evidence; the open-choice recommendations
are superseded.

Genesis is evaluating a focused Rust framework alongside its existing Bevy
foundation. Independence over development workflow and priorities is an explicit
selection criterion, alongside simulation speed, rendering flexibility, iteration
speed, and maintenance effort. Bevy's community policies alone do not establish
that the engine will decline technically.

## Recommendation before the decision

Keep the simulation independent of the presentation layer and keep Rust as the
baseline language. Evaluate a custom framework without replacing the existing
game. A custom framework is credible if owning its infrastructure is an accepted
part of the project. Its long-term advantage has not yet been demonstrated.

The first experiment lives in [the engine lab](../experiments/engine-lab/README.md).
It compares direct `winit`/`wgpu` with Bevy using identical static geometry and
camera settings. It tests basic infrastructure and provides a repeatable build
measurement harness. It is not a social-simulation benchmark or an assessment of
finished content workflows.

## Existing implementation findings

- `sim_core` owns the simulation without Bevy dependencies. Preserve this boundary.
- `app` currently registers camera, simulation, and planet plugins.
- The planet plugin uses a standard sphere as a debugging placeholder.
- Custom material and post-processing code exists, but the composite renderer
  plugin is not registered by the application.
- Simulation ticks run synchronously in an unbounded catch-up loop inside
  `Update`. A more expensive simulation will need deliberate scheduling and a
  policy for falling behind real time.
- The application contains Metal workarounds, including disabled indirect drawing.
  Their comments document prior failures; this investigation has not reproduced
  or resolved those original failures.
- The run task uses release builds with thin LTO. That is not an appropriate
  default for comparing development iteration times.

## Alternatives

| Approach | Benefits | Responsibilities and limitations |
| --- | --- | --- |
| Bevy with Genesis-specific plugins | Existing rendering, assets, animation, scheduling, and integration | Follow its APIs and upgrade path; build simulation-specific authoring tools |
| Independent Rust framework using winit and wgpu | Own the renderer and application architecture; select only needed systems | Own asset integration, animation integration, culling, materials, tools, lifecycle, and their interaction |
| Selected Bevy components with a separate renderer | Reuse useful modules without adopting the entire engine | More integration boundaries; component reuse does not automatically simplify maintenance |

The direct renderer does not eliminate upstream dependencies. It retains wgpu,
which Bevy also uses, so it cannot be assumed to fix lower-level Metal problems.
Each library's contribution policies and maintenance should be assessed before
making a long-term commitment; no blanket claim of AI-friendly dependencies has
been established.

## Experiment design

Both applications consume one shared fixture: 12 cuboid buildings, 48 geometric
inhabitant placeholders, ground and path, combined into a 4,392-vertex mesh.
Both use linear vertex colors with baked face shading, orthographic projection,
back-face culling, and no multisampling. This deliberately avoids comparing two
lighting implementations.

The custom application implements GPU and surface setup, shader bindings, a
camera uniform, depth handling, resize and surface recovery, and offscreen PNG
readback. Bevy provides most of those mechanisms through existing facilities.
Neither application includes simulation, imported assets, character animation,
selection, a person inspector, or the proposed Sable-inspired outline treatment.

The lab is an isolated Cargo workspace, with its own lockfile and build outputs.
The main game's source code and dependency versions are unchanged. Bevy 0.18.1
and wgpu 27.0.1 were chosen to match the project's existing engine generation.
Bevy 0.20 is the current release; this experiment does not claim to compare its
latest capabilities. Bevy's features are reduced to a 3D windowed baseline.

The build harness uses the same development optimization profile, separate fresh
target directories, offline dependencies, serial runs, and three repetitions for
no-change and actual application-source edits. It records compiler details,
commands, wall time, exit codes, and logs. OS caches are not flushed. These small
programs differ substantially in supplied capabilities, so build-time ratios
must not be presented as equivalent-engine efficiency ratios.

## Dependency footprint

`cargo tree --edges normal --target aarch64-apple-darwin` resolves 92 unique
packages for the custom application and 237 for the reduced-feature Bevy
application, including each application and the shared fixture. This count
excludes build-only dependencies. It describes the selected lockfile and features;
it is not a runtime or compile-speed measurement.

## Validation status

Dependency resolution, isolated workspace metadata validation, rustfmt checks,
local documentation links, and the measurement script's Python syntax and CLI
help checks passed. The workspace check did not finish: dependency processes
stalled repeatedly before the application crates could be validated. The two
build process groups started for this investigation were stopped.

On a subsequent attempt, the custom application built successfully with the pinned
Rust toolchain. Both shared-fixture unit tests passed. An offscreen capture was
rendered and visually inspected, and the native window opened successfully using
Apple M4 / Metal. The capture is `experiments/engine-lab/results/custom.png`.
The sandbox could not enumerate a Metal adapter; authorized execution outside
it succeeded. No rendering code change was required.

Bevy compilation, graphical parity, and Clippy remain unverified. No controlled
comparative build or runtime measurements have been produced. The successful
custom build reused artifacts from previous attempts and is not a cold-build
benchmark. Complete the remaining validation before drawing a migration conclusion.

The pinned Rust 1.93.1 toolchain was installed through mise. During validation,
`rustfmt --version` stalled at `_dyld_start`, before entering the program, both
inside and outside the sandbox. Dependency build scripts also stalled. A trivial
program compiled with the preinstalled Homebrew Rust compiler timed out on launch
after 15 seconds. Some processes later resumed. These observations indicate a
host launch problem, not an engine-specific result; its root cause is unconfirmed.
Do not publish timings collected during these delays as normal build performance.

## Criteria considered before the decision

Before adopting a custom framework, add the same small vertical slice to both
paths: one imported animated inhabitant, picking, an inspector explaining a
person's action, material editing, and an adapter to the same simulation.
Measure rebuilds and runtime behavior on an otherwise idle machine. Record time
spent making changes and debugging integration problems as well as frame times.

Keep persistent person IDs separate from rendering entity IDs. Keep saves and
simulation replay outside either renderer. Off-screen people must remain alive
in the simulation without requiring instantiated animated characters.

Choose the custom path if the independence and measured iteration benefits
justify owning the missing infrastructure. Choose Bevy if its supplied systems
save more effort than its integration and upgrade costs. Neither choice needs a
prediction that the other project will fail.

## Primary references

- [Bevy capabilities](https://bevy.org/)
- [Bevy 0.18 features](https://bevy.org/news/bevy-0-18/)
- [Bevy release history](https://bevy.org/news/)
- [Bevy build configuration](https://bevy.org/learn/quick-start/getting-started/setup/)
- [Bevy contribution AI policy](https://bevy.org/learn/contribute/policies/ai/)
- [wgpu 27 API](https://docs.rs/wgpu/27.0.1/wgpu/)
- [winit 0.30 API](https://docs.rs/winit/0.30.13/winit/)
