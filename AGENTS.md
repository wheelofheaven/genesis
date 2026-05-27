# Genesis Codex Instructions

Genesis is a Rust/Bevy planet-builder game. It should remain serene,
simulation-first, and scientifically legible.

Read `../AGENTS.md` first for workspace-wide GitHub account isolation and
project conventions. The previous Claude Code context is `.claude/CLAUDE.md`;
it remains authoritative for this repo.

## Architecture

The workspace has two main crates:

- `crates/sim_core` - pure Rust deterministic simulation core.
- `crates/app` - Bevy rendering, input, UI, and integration layer.

Never let Bevy types leak into `sim_core`. Do not put simulation logic in the
app crate. `sim_core` should remain deterministic, serde-friendly, portable,
and testable.

## Simulation Rules

- Use a wrapped 2D grid.
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

## Definition of Done

- The game runs on macOS with a single command, typically `cargo run`.
- The player can terraform and seed life.
- Overlays explain the state clearly.
- The simulation is deterministic.
- There is a clear objective and success state.
