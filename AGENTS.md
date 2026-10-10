# Genesis Codex Instructions

Genesis is a Rust life, society, world, and civilization simulator with a
custom engine. Read `docs/engine-decision.md` for the accepted winit/wgpu direction
and transition plan. Bevy is the legacy prototype, not the target architecture.
Read `docs/vision.md` for the agreed product direction: individual lives, a shared
simulation supporting multiple modes, and Sable-inspired illustrated 3D.
The first focus is a living settlement supervised by an Elohim.

Read `../AGENTS.md` first for workspace-wide GitHub account isolation and
project conventions. The previous Claude Code context is `.claude/CLAUDE.md`;
its engineering conventions remain authoritative. `docs/vision.md` supersedes
the older product scope and rendering roadmap. `docs/engine-decision.md`
supersedes the Bevy engine requirement and earlier comparison recommendations.

## Architecture

The main workspace still has four crates from the legacy Bevy prototype:

- `crates/sim_core` - pure Rust deterministic simulation core.
- `crates/app` - Bevy input, UI, and integration layer.
- `crates/renderer` - rendering materials, camera, and visual effects.
- `crates/planet` - planet mesh and presentation.

The working custom renderer is in `experiments/engine-lab/custom`; its migration
into the main workflow is the next engineering milestone. Do not add new Bevy
features as the default path. Introduce modules or crates when needed by concrete work.

Never let rendering, windowing, UI, or engine entity types leak into `sim_core`.
Do not put simulation logic in the app crate. `sim_core` should remain deterministic, serde-friendly, portable,
and testable.

## Simulation Rules

- Preserve the prototype wrapped 2D grid until a deliberate spatial design change.
- Keep cell values bounded and stable; avoid NaNs and unbounded feedback.
- Player actions should become explicit `Action` values consumed by
  `sim_core.step()`.
- Report important outcomes through `SimEvent`, not ad-hoc prints.
- Same seed plus same actions should produce the same result.

## Code Quality

- Use `rustfmt` defaults.
- Run Clippy with no warnings when feasible.
- Prefer explicit error handling over `.unwrap()` in library code.
- Add unit tests for simulation invariants, especially bounds, determinism, and
  stability.
- Keep files modular by feature; avoid large mixed-responsibility modules.

## Iteration Goals

- Maintain a documented single-command macOS launch. `mise run run` currently
  starts the legacy Bevy application; see the engine lab for the custom launch.
- Each iteration delivers a playable, reviewable expansion of the shared simulation.
- The next target is a living settlement with understandable individual behavior
  and consequential Elohim intervention; see `docs/vision.md` for scope and open choices.
- Presentation explains simulation state and individual motivations.
- The simulation remains deterministic and testable.
- Define objectives and success criteria per mode and iteration; the original
  biosphere score is not the overall project objective.
