# Genesis — Project Instructions

> **Engine decision:** [Custom Rust engine](../docs/engine-decision.md) is accepted.
> The main workspace now runs winit/wgpu with egui; Bevy lives in `legacy/`.
> Use winit/wgpu as the starting foundation. Bevy-specific requirements below
> describe the legacy prototype; follow the current decision for new development.

> **Original prototype instructions — superseded product direction (October 10, 2026).**
> Read [the current vision](../docs/vision.md) and [AGENTS.md](../AGENTS.md) first.
> Genesis now prioritizes individual life and society simulation, an initial Elohim
> settlement mode, Earth and future exoplanet scenarios, and spacefaring technology
> inspired by Phantasy Star Online and Xenoblade Chronicles X.
> The ocean-only MVP, exclusion of humans, Tiny Glade aesthetic, and rendering-first
> phase order below are historical, not current requirements. Engineering rules
> about deterministic simulation, separation from Bevy, and code quality still apply.
> Proposed crates and features below are not an inventory of implemented code.

## Overview

Genesis is a serene, scientific, simulation-first planet-builder game. The player terraforms an ocean world by raising land, tuning climate proxies, and seeding early life — watching the biosphere stabilize through readable scientific overlays.

**One-sentence pitch:** A serene planet-builder where the player terraforms an ocean world by raising land, tuning climate proxies, and seeding early life — watching the biosphere stabilize through readable scientific overlays.

## Tech Stack

- **Language:** Rust (stable)
- **Engine:** Bevy (pin to a specific version; do not chase main)
- **UI:** `bevy_egui` for overlays and panels
- **Optional:** `noise` crate for procedural generation (only if needed)
- **Platform:** macOS primary dev environment
- **Tooling:** `mise` for reproducible dev environment (preferred)

## Architecture (Critical)

The project is a Rust workspace with two crates:

### `crates/sim_core` — Pure Rust, deterministic, testable
- **No Bevy types.** Engine-agnostic simulation core.
- Owns the world grid and all simulation stepping.
- Exposes: `SimState`, `SimConfig`, `step(dt, actions) -> Vec<SimEvent>`, read-only queries.
- Must be deterministic: same seed + same actions = same results.
- Must be portable: designed for eventual FFI bridge or C++ rewrite for Unreal.
- Keep data schemas serde-friendly.

### `crates/app` — Bevy layer
- Rendering, input, UI.
- Converts player inputs into `Action`s.
- Calls `sim_core.step()` on fixed timesteps.
- Renders tiles/mesh/overlays based on sim state.

**Rule:** Never let Bevy types leak into `sim_core`. Never put simulation logic in the `app` crate.

## World Representation

Wrapped 2D grid (toroidal wrap). Cell size and dimensions configurable.

### Cell fields
- `elevation: f32` — below 0 = below sea level
- `water_depth: f32` — derived from sea_level - elevation (clamped)
- `temp: f32` — temperature proxy
- `moisture: f32` — moisture proxy
- `fertility: f32` — fertility proxy
- `toxicity: f32` — optional proxy (can be 0 in MVP)
- `biome: Biome` — derived enum
- `fungus: f32` — 0..1 coverage
- `plants: f32` — 0..1 coverage

### Global state
- `sea_level: f32`
- `time: f64`
- `rng_seed: u64`
- `stability_score: f32` (derived)

## Simulation Rules (MVP)

### Terraforming
- Player brush modifies elevation (+/-).
- Water depth updates immediately.

### Climate proxies
- `temp` = f(latitude band, elevation) — cooler at higher elevation.
- `moisture` = f(adjacency to water, simple blur for wind approximation).
- `fertility` = increases with fungus presence over time; decreases with erosion.

### Biome derivation
Small enum based on (temp, moisture, elevation):
`Ocean`, `Beach`, `Grassland`, `Forest`, `Tundra`, `Desert`, `Mountain`

### Life spread
- **Fungus:** grows in moderate moisture, any temp except extremes; diffusion spread.
- **Plants:** require fertility threshold; growth slowly increases fertility (clamped feedback).
- Both decay if unsuitable, cap at 1.0.
- All equations must be stable and bounded — no chaos, no NaNs.

### Determinism
- Use deterministic RNG per tick.
- Stepping must be reproducible given same seed + actions.

## Player Tools (MVP)
- Terraform brush: raise/lower land
- Seed brush: fungus / plants
- Build outpost: enables research points, unlocks stronger tools

## UI Layout
- **Left panel:** Tools (terraform raise/lower, seed fungus/plants, build outpost)
- **Right/top:** Overlay toggles (elevation/water, temp, moisture, fertility, biome, life coverage)
- **Bottom bar:** Time controls (pause, 1x, 4x, 16x)
- **Bottom:** Expedition log (generated from `SimEvent`)
- Use `bevy_egui`, keep it minimal.

## Rendering (MVP)
- Grid as mesh of quads or instanced tiles.
- Color determined by current overlay mode.
- Readability first, no fancy terrain.
- Optional basic smoothing/interpolation.

## Non-Goals (MVP)
- No humans, no civilization simulation.
- No spherical planet rendering (use wrapped 2D).
- No "realistic" physics/biology — gamey + legible.
- No heavy asset pipeline — procedural visuals and simple meshes.
- No networking.

## Win Condition
"Stabilize a self-sustaining biosphere" — X% land green + stability score over N cycles.

## Git Conventions

- **User:** `zarazinsfuss` / `zarazinsfuss@users.noreply.github.com`
- **License:** CC0-1.0
- **Commit messages:** imperative mood, under 72 chars, conventional prefixes (`feat:`, `fix:`, `refactor:`, `test:`, `chore:`, `docs:`)
- **Branch strategy:** `main` = production; feature branches = `feature/description`

## Code Quality Rules

- No giant files; use modules by feature.
- Avoid premature ECS complexity: keep sim state in `sim_core`, mirror only what's needed in Bevy.
- Unit tests for sim invariants:
  - Bounds (0..1 coverage)
  - Determinism (same seed + actions = same results)
  - Stability (no NaNs, no unbounded values)
- Use explicit types and clamping helpers.
- Log events through `SimEvent`, not ad-hoc prints.
- Feature flags to keep build lean.

## Code Style

- Use `rustfmt` defaults.
- Run `clippy` with no warnings.
- Prefer explicit error handling over `.unwrap()` in library code (`.unwrap()` is fine in tests and `main.rs` setup).
- Document public APIs in `sim_core`.

## Directory Layout

```
/
├── Cargo.toml              # workspace
├── crates/
│   ├── sim_core/
│   │   ├── src/
│   │   │   ├── lib.rs
│   │   │   ├── grid.rs
│   │   │   ├── rules/
│   │   │   ├── actions.rs
│   │   │   ├── events.rs
│   │   │   └── config.rs
│   │   └── tests/
│   └── app/
│       └── src/
│           ├── main.rs
│           ├── plugins/
│           ├── render/
│           ├── ui/
│           └── input/
├── assets/                 # optional placeholders
├── docs/
│   └── design.md
├── .claude/
│   └── CLAUDE.md
├── README.md
└── LICENSE
```

## Definition of Done (MVP)

- Runs on macOS with a single command (`cargo run`).
- Player can terraform and seed life.
- Overlays explain what's happening.
- Simulation runs deterministically.
- Clear objective and a "success" state.
