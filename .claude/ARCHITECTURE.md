# Genesis — Crate Architecture

## Overview

Genesis is a Rust workspace organized into focused crates. The core principle remains: **simulation logic is engine-agnostic** (`sim_core`), and all Bevy-specific code lives in separate crates. The custom renderer replaces Bevy's default PBR pipeline to achieve a Tiny Glade-inspired painterly art style.

---

## Dependency Graph

```
                        ┌─────────┐
                        │   app   │  (binary — game entry point)
                        └────┬────┘
                             │ depends on all below
              ┌──────────────┼──────────────────┐
              │              │                  │
         ┌────▼────┐   ┌────▼────┐        ┌────▼────┐
         │   ui    │   │  world  │        │  input  │
         └────┬────┘   └────┬────┘        └─────────┘
              │              │
              │         ┌────▼────┐
              │         │ station │
              │         │ planet  │
              │         │ ocean   │
              │         │ terrain │
              │         │ vessel  │
              │         │building │
              │         └────┬────┘
              │              │
              │         ┌────▼─────┐
              └────────►│ renderer │  (custom render pipeline)
                        └────┬─────┘
                             │
                        ┌────▼─────┐
                        │ sim_core │  (pure Rust, no Bevy)
                        └──────────┘
```

All environment crates (`station`, `planet`, `ocean`, `terrain`, `vessel`, `buildings`) depend on `renderer` for materials and rendering primitives, and on `sim_core` for simulation data where needed.

---

## Crates

### `sim_core` — Simulation Engine

**Status:** Exists, largely complete for MVP.

**Purpose:** Pure Rust, deterministic, engine-agnostic simulation. Owns the world grid, climate, life spread, terraforming, and stability scoring.

**Dependencies:** `serde`, `rand` (no Bevy, no wgpu, no GPU types)

**Public API:**
- `SimState::new(config) -> SimState`
- `SimState::step(dt, actions) -> Vec<SimEvent>`
- Read-only queries: grid cells, global state, stability score

**Future additions:**
- Atmospheric composition model (for planet analysis gameplay)
- Ocean chemistry / depth model
- Settlement placement rules
- Research/tech tree state

```
sim_core/src/
├── lib.rs           # SimState, step()
├── grid.rs          # Grid, Cell, Biome
├── actions.rs       # Action enum
├── events.rs        # SimEvent enum
├── config.rs        # SimConfig
└── rules/
    ├── mod.rs
    ├── terraform.rs
    ├── climate.rs
    └── life.rs
```

**Rule:** No Bevy types. No GPU types. Must compile without any graphics dependencies.

---

### `renderer` — Custom Render Pipeline

**Status:** New. Core of the rendering rewrite.

**Purpose:** Replaces Bevy's default 3D renderer with a custom pipeline targeting the Tiny Glade aesthetic. All other visual crates depend on this for materials, shaders, and render graph integration.

**Dependencies:** `bevy` (render features), `wgpu` (via Bevy)

**Responsibilities:**
- Custom `RenderGraph` with named passes (shadow, opaque, transparent, post-process)
- Custom lighting model (stylized diffuse + specular, hemispheric ambient)
- Shadow mapping with temporal stabilization
- Post-processing stack (tonemapping, bloom, SSAO, DoF, color grading)
- Depth-buffer ray marching (contact shadows, SSGI, SSR)
- Custom `Material` trait for stylized surfaces
- Shader library (shared WGSL modules for lighting, noise, utilities)

```
renderer/src/
├── lib.rs               # RendererPlugin, re-exports
├── plugin.rs            # RenderGraph setup, pass ordering
├── pipeline/
│   ├── mod.rs
│   ├── opaque.rs        # Opaque render pass
│   ├── transparent.rs   # Transparent / alpha-blended pass
│   └── shadow.rs        # Shadow map generation pass
├── lighting/
│   ├── mod.rs
│   ├── model.rs         # Custom lighting model (stylized PBR)
│   ├── ambient.rs       # Hemispheric ambient + SH encoding
│   └── shadows.rs       # Shadow sampling, contact shadows
├── postprocess/
│   ├── mod.rs
│   ├── tonemap.rs       # Tony McMapface integration
│   ├── bloom.rs         # Soft bloom
│   ├── ssao.rs          # Screen-space ambient occlusion
│   ├── dof.rs           # Tilt-shift depth of field
│   └── color_grade.rs   # LUT-based color grading
├── material/
│   ├── mod.rs
│   ├── stylized.rs      # Base stylized material
│   ├── terrain.rs       # Terrain-specific material (biome blending)
│   ├── water.rs         # Ocean/water material
│   └── emissive.rs      # Screens, lights, UI panels
├── camera/
│   ├── mod.rs
│   ├── orbit.rs         # Orbital camera controller
│   ├── fps.rs           # First-person camera
│   ├── follow.rs        # Third-person follow camera
│   └── transition.rs    # Smooth transitions between camera modes
└── shaders/             # WGSL shader sources
    ├── common/
    │   ├── noise.wgsl       # Noise functions (simplex, fbm, voronoi)
    │   ├── lighting.wgsl    # Shared lighting calculations
    │   ├── math.wgsl        # Math utilities
    │   └── color.wgsl       # Color space conversions
    ├── vertex/
    │   ├── standard.wgsl    # Standard vertex transform
    │   └── wave.wgsl        # Wave vertex displacement
    ├── fragment/
    │   ├── stylized.wgsl    # Stylized PBR fragment
    │   ├── terrain.wgsl     # Terrain fragment (biome blending)
    │   └── water.wgsl       # Water fragment
    └── postprocess/
        ├── bloom.wgsl
        ├── ssao.wgsl
        ├── dof.wgsl
        ├── ray_march.wgsl   # Depth-buffer ray marching
        └── color_grade.wgsl
```

**Rule:** This crate defines the visual language. All environment crates use `renderer`'s materials and shader modules — they do not define their own render passes.

---

### `planet` — Planet Generation and Rendering

**Status:** New. Phase 1.

**Purpose:** Procedural planet mesh generation, atmosphere configuration, orbital-scale rendering.

**Dependencies:** `bevy`, `renderer`, `sim_core`, `noise`

**Responsibilities:**
- Cube-sphere mesh generation with configurable LOD
- Planet heightmap (initially flat ocean, later driven by `sim_core` elevation)
- Atmosphere configuration (custom `ScatteringMedium` for alien atmosphere)
- Cloud layer rendering (noise-driven shell mesh or billboard)
- Starfield skybox generation
- Planet-to-terrain LOD transition (orbital → surface scale)

```
planet/src/
├── lib.rs              # PlanetPlugin
├── mesh.rs             # Cube-sphere generation, LOD chunking
├── heightmap.rs        # Heightmap generation + sim_core bridge
├── atmosphere.rs       # ScatteringMedium configuration
├── clouds.rs           # Cloud layer (noise-driven)
├── starfield.rs        # Procedural star background
└── lod.rs              # Distance-based LOD management
```

---

### `station` — Space Station Environment

**Status:** New. Phase 2.

**Purpose:** The orbital space station — exterior model, interior walkable spaces, in-world UI.

**Dependencies:** `bevy`, `renderer`, `sim_core`

**Responsibilities:**
- Station exterior mesh (modular ring/cylinder sections)
- Station interior: modular rooms (observation deck, lab, command center, corridors)
- Window rendering (planet/space visible through station windows)
- In-world UI: console screens, holographic planet display
- Interaction system: look-at detection, interactable objects
- Station orbital motion (orbiting the planet)

```
station/src/
├── lib.rs              # StationPlugin
├── exterior.rs         # Station exterior mesh + orbit motion
├── interior/
│   ├── mod.rs          # Interior module system
│   ├── rooms.rs        # Room definitions (observation, lab, command)
│   ├── props.rs        # Interior objects (consoles, panels, furniture)
│   └── windows.rs      # Window geometry + space view rendering
├── interaction.rs      # Look-at highlighting, interact prompts
└── displays/
    ├── mod.rs
    ├── planet_holo.rs  # Holographic planet display (command center)
    ├── spectral.rs     # Spectral analysis screen
    └── console.rs      # Generic data console (shows sim_core data)
```

---

### `ocean` — Ocean Rendering

**Status:** New. Phase 3.

**Purpose:** Ocean surface rendering from close range — waves, foam, reflections, caustics.

**Dependencies:** `bevy`, `renderer`, `noise`

**Responsibilities:**
- Wave simulation (Gerstner initially, FFT compute shader later)
- Ocean surface material (subsurface scattering, depth color, fresnel)
- Foam generation (wave jacobian / whitecap detection)
- Underwater rendering (depth fog, caustics on seabed)
- Wake effects (vessel interaction with surface)

```
ocean/src/
├── lib.rs              # OceanPlugin
├── waves/
│   ├── mod.rs
│   ├── gerstner.rs     # Gerstner wave model (initial)
│   └── fft.rs          # FFT-based wave simulation (compute shader)
├── surface.rs          # Ocean mesh generation + tiling
├── foam.rs             # Foam / whitecap generation
├── underwater.rs       # Underwater effects (fog, caustics)
└── wake.rs             # Vessel wake effects
```

---

### `terrain` — Terrain Rendering

**Status:** New. Phase 4.

**Purpose:** Heightmap-based terrain mesh driven by `sim_core` grid data.

**Dependencies:** `bevy`, `renderer`, `sim_core`

**Responsibilities:**
- Heightmap mesh from `sim_core` elevation grid
- Real-time mesh updates when terrain changes
- Procedural texturing based on biome (terrain material in `renderer`)
- Terrain LOD (clipmap or CDLOD)
- Vegetation placement data (density maps from sim_core life values)

```
terrain/src/
├── lib.rs              # TerrainPlugin
├── mesh.rs             # Heightmap → mesh generation
├── update.rs           # Real-time mesh updates from sim_core
├── texturing.rs        # Biome → texture blending rules
├── lod.rs              # Terrain LOD system
└── vegetation.rs       # Vegetation density maps + instancing data
```

---

### `vessel` — Exploratory Vessel

**Status:** New. Phase 3.

**Purpose:** The player's descent/exploration vehicle.

**Dependencies:** `bevy`, `renderer`, `ocean`

```
vessel/src/
├── lib.rs              # VesselPlugin
├── mesh.rs             # Vessel model
├── physics.rs          # Buoyancy, basic movement
├── cockpit.rs          # Interior cockpit view + instruments
└── hud.rs              # Vessel HUD overlay
```

---

### `buildings` — Procedural Buildings and Settlements

**Status:** New. Phase 5.

**Purpose:** Procedural building generation, settlement layout, population visualization.

**Dependencies:** `bevy`, `renderer`, `sim_core`, `terrain`

```
buildings/src/
├── lib.rs              # BuildingsPlugin
├── generation/
│   ├── mod.rs
│   ├── shelter.rs      # Simple early structures
│   ├── modular.rs      # Walls, roofs, foundations (Tiny Glade-inspired)
│   └── placement.rs    # Terrain-conforming placement
├── settlement.rs       # Settlement layout rules, roads, paths
├── agriculture.rs      # Farmland, crop visualization
└── population.rs       # Human figure spawning + animation
```

---

### `world` — World State Management

**Status:** New. Introduced alongside environment crates.

**Purpose:** Manages the overall game world state, transitions between environments/chapters, and bridges `sim_core` to the visual crates.

**Dependencies:** `bevy`, `sim_core`, `planet`, `station`, `ocean`, `terrain`, `vessel`, `buildings`

```
world/src/
├── lib.rs              # WorldPlugin
├── state.rs            # Game state machine (Orbit, Station, Descent, Surface, Terraform)
├── chapter.rs          # Chapter/mode definitions
├── transition.rs       # Environment transition logic
└── bridge.rs           # sim_core ↔ render world data synchronization
```

---

### `ui` — User Interface

**Status:** Exists as `app/src/ui/`. Will be extracted into its own crate.

**Purpose:** All UI — debug overlays, in-game HUD, menus.

**Dependencies:** `bevy`, `bevy_egui`, `sim_core`, `renderer`

```
ui/src/
├── lib.rs              # UiPlugin
├── overlay.rs          # Dev/debug overlay (egui)
├── hud.rs              # In-game HUD
├── menus.rs            # Main menu, pause menu
└── diegetic.rs         # In-world UI rendering helpers (for station screens)
```

---

### `input` — Input Handling

**Status:** Exists as `app/src/input/`. Will be extracted into its own crate.

**Purpose:** Maps raw input to game actions across all camera/interaction modes.

**Dependencies:** `bevy`, `sim_core` (for Action types)

```
input/src/
├── lib.rs              # InputPlugin
├── bindings.rs         # Key/mouse binding configuration
├── fps.rs              # First-person input mapping
├── tool.rs             # Terraforming tool input (existing)
└── interaction.rs      # Object interaction input (station consoles, etc.)
```

---

### `app` — Binary Entry Point

**Status:** Exists. Will slim down as crates are extracted.

**Purpose:** Composes all plugins, configures the Bevy app, contains only startup/bootstrap code.

**Dependencies:** All crates above.

```
app/src/
├── main.rs             # App::new(), plugin registration, window config
└── (nothing else — all logic lives in the crates above)
```

---

## Workspace Cargo.toml Structure

```toml
[workspace]
resolver = "2"
members = [
    "crates/sim_core",
    "crates/renderer",
    "crates/planet",
    "crates/station",
    "crates/ocean",
    "crates/terrain",
    "crates/vessel",
    "crates/buildings",
    "crates/world",
    "crates/ui",
    "crates/input",
    "crates/app",
]

[workspace.dependencies]
bevy = "0.18"
bevy_egui = "0.39"
noise = "0.9"
serde = { version = "1", features = ["derive"] }
rand = "0.8"

# Internal crates
genesis-sim-core = { path = "crates/sim_core" }
genesis-renderer = { path = "crates/renderer" }
genesis-planet = { path = "crates/planet" }
genesis-station = { path = "crates/station" }
genesis-ocean = { path = "crates/ocean" }
genesis-terrain = { path = "crates/terrain" }
genesis-vessel = { path = "crates/vessel" }
genesis-buildings = { path = "crates/buildings" }
genesis-world = { path = "crates/world" }
genesis-ui = { path = "crates/ui" }
genesis-input = { path = "crates/input" }
```

---

## Incremental Build Strategy

Crates are introduced **only when their phase begins**. The workspace starts small and grows:

| Phase | Crates Added | Total Crates |
|-------|-------------|--------------|
| Current | `sim_core`, `app` | 2 |
| Phase 0 | `renderer`, extract `ui` + `input` | 5 |
| Phase 1 | `planet` | 6 |
| Phase 2 | `station`, `world` | 8 |
| Phase 3 | `ocean`, `vessel` | 10 |
| Phase 4 | `terrain` | 11 |
| Phase 5 | `buildings` | 12 |

Do not create empty placeholder crates. Each crate is created when work on it actually begins.

---

## Key Architecture Rules

1. **`sim_core` stays pure.** No Bevy, no wgpu, no GPU types. Portable for FFI.
2. **`renderer` owns the visual language.** Environment crates use its materials and shaders; they do not create their own render passes or pipelines.
3. **One plugin per crate.** Each crate exposes a single top-level `XPlugin` that registers all its systems, resources, and render nodes.
4. **Data flows one way:** `sim_core` → `world` (bridge) → environment crates → `renderer`. The render layer never writes back to `sim_core`.
5. **Game state in `world`.** The `world` crate owns transitions between environments (orbit → station → descent → surface). Individual environment crates do not know about each other.
6. **Shaders live in `renderer/src/shaders/`.** Environment crates can provide additional shader modules but they are registered through `renderer`'s shader infrastructure.
7. **No premature crates.** Don't create a crate until you're actively building its phase.
