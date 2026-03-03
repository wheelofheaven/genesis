# Genesis

A serene planet-builder where the player terraforms an ocean world by raising land, tuning climate proxies, and seeding early life — watching the biosphere stabilize through readable scientific overlays.

## Getting Started

### Prerequisites

- [mise](https://mise.jdx.dev/) (manages Rust toolchain)
- macOS (primary dev platform)

### Setup

```sh
mise install        # Install Rust toolchain
mise run build      # Build all crates
```

### Run

```sh
mise run run        # Launch the game
```

### Development

```sh
mise run test       # Run all tests
mise run clippy     # Run lints
mise run fmt        # Format code
mise run ci         # Run all CI checks (fmt + clippy + tests)
```

## Controls

- **Left click** — Use active tool at cursor position
- **Tools** (left panel):
  - Raise Land / Lower Land — Terraform the ocean floor
  - Seed Fungus / Seed Plants — Introduce life
  - Build Outpost — Place expedition outpost
- **Overlays** (top bar) — Toggle data visualization layer
- **Speed** (left panel) — Pause, 1x, 4x, 16x

## Architecture

The project is a Rust workspace with two crates:

```
crates/
├── sim_core/   # Pure Rust simulation (no engine dependencies)
└── app/        # Bevy rendering, UI, and input
```

**`sim_core`** owns the world grid and all simulation rules. It is deterministic, testable, and engine-agnostic — designed for potential future reuse via FFI.

**`app`** is the Bevy layer that handles rendering the grid, player input, and the egui overlay UI. It converts player inputs into `Action`s and calls `sim_core.step()` on fixed timesteps.

## Objective

Stabilize a self-sustaining biosphere: terraform land from the ocean, seed fungus and plants, and achieve a stability score of 80%+.

## License

[CC0-1.0](LICENSE)
