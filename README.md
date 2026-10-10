# Genesis

A life, society, world, and civilization simulator where history emerges from
individual lives. The first planned mode places the player in the role of an
Elohim supervising human settlements, with Sable-inspired illustrated 3D.

Read the [project vision](docs/vision.md) for the agreed direction, social
simulation goals, development horizons, and open design choices.

**Current status:** early terraforming and planet-rendering prototype. Individual
lives, historical scenarios, and civilization gameplay are planned, not yet
implemented.

## Engine Direction

Genesis will use a focused **custom Rust engine**, starting with `winit` and
`wgpu`. The [accepted decision](docs/engine-decision.md) records the architecture,
tradeoffs, and transition milestones. The [evaluation](docs/engine-evaluation.md)
preserves the earlier findings.

The main workspace still runs the legacy Bevy prototype. The custom renderer
currently lives in the [engine lab](experiments/engine-lab/README.md) and can be
launched from the repository root:

```sh
mise exec -- cargo run --manifest-path experiments/engine-lab/Cargo.toml -p genesis-lab-custom
```

It displays a static test scene; integration with the simulation is the next
engineering milestone.

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
mise run run        # Launch the legacy Bevy prototype
```

### Development

```sh
mise run test       # Run all tests
mise run clippy     # Run lints
mise run fmt        # Format code
mise run ci         # Run all CI checks (fmt + clippy + tests)
```

## Prototype Controls

- **Left click** — Use active tool at cursor position
- **Tools** (left panel):
  - Raise Land / Lower Land — Terraform the ocean floor
  - Seed Fungus / Seed Plants — Introduce life
  - Build Outpost — Place expedition outpost
- **Overlays** (top bar) — Toggle data visualization layer
- **Speed** (left panel) — Pause, 1x, 4x, 16x

## Architecture

The legacy main workspace currently has four crates; see the
[engine decision](docs/engine-decision.md) for the target architecture:

```
crates/
├── sim_core/   # Pure Rust simulation (no engine dependencies)
├── renderer/   # Materials, camera, and visual effects
├── planet/     # Planet mesh and presentation
└── app/        # Bevy integration, UI, and input
```

**`sim_core`** owns the world grid and all simulation rules. It is deterministic, testable, and engine-agnostic — designed for potential future reuse via FFI.

**`app`** is the Bevy layer that handles rendering the grid, player input, and the egui overlay UI. It converts player inputs into `Action`s and calls `sim_core.step()` on fixed timesteps.

## Development Direction

The next playable target is a small living settlement: individual needs,
personalities, aspirations, relationships, and meaningful consequences from an
Elohim intervention. Subsequent iterations deepen lives, institutions, regional
connections, and additional modes.

The original prototype used a biosphere stability score of 80% as its objective.
Future objectives will be defined per mode; see the [vision](docs/vision.md).

## License

[CC0-1.0](LICENSE)
