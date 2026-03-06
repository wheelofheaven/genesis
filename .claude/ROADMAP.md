# Genesis — Roadmap

## Vision

Genesis evolves from a 2D grid-based terraforming prototype into a fully 3D, custom-rendered experience inspired by Tiny Glade's painterly art style. The player begins aboard an orbital space station analyzing an exoplanet, descends to its oceanic surface, and ultimately terraforms and seeds life — watching civilizations emerge.

This is a long-term hobby project. Quality of rendering and art direction take priority over shipping speed.

---

## Art Direction

**Target aesthetic:** Tiny Glade — soft, painterly, warm lighting, miniature/diorama feel.

**Key rendering techniques to develop:**
- Custom render pipeline (replace Bevy's default PBR)
- Stylized lighting model (soft shadows, warm ambient, no harsh edges)
- Tony McMapface tonemapping (already available in Bevy as a built-in option)
- Tilt-shift depth of field for diorama feel (contextual — station interiors vs orbital views)
- Screenspace ray marching for contact shadows, SSGI, SSR
- Software ray tracing against proxy geometry for GI (long-term)
- Spherical harmonics for smooth indirect lighting encoding
- Post-processing stack: color grading, bloom, DoF, AO

---

## Phases

### Phase 0: Rendering Foundation

**Goal:** Replace the 2D texture renderer with a custom 3D render pipeline.

**Milestones:**

0.1 — **Custom render pipeline skeleton**
- Create `crates/renderer` with a custom `RenderPlugin`
- Implement a minimal custom render graph (clear → opaque pass → post-process → present)
- Custom WGSL shader infrastructure (vertex + fragment)
- Basic camera system (orbit + fly modes)
- Render a test scene (textured cube, ground plane) with custom shaders

0.2 — **Stylized lighting model**
- Custom fragment shader with soft diffuse + specular
- Hemispheric ambient lighting (sky color + ground bounce)
- Contact shadows via depth-buffer ray marching (port h3r2tic's gist to WGSL)
- Shadow mapping with temporal stabilization (no swimming during camera movement)

0.3 — **Post-processing stack**
- Tony McMapface tonemapping (use Bevy's built-in `TonyMcMapface` variant)
- Bloom (soft, wide kernel for the painterly glow)
- SSAO (start with Bevy's built-in GTAO, consider custom later)
- Tilt-shift depth of field (custom fullscreen pass via `FullscreenMaterial`)
- Color grading LUT support

0.4 — **Material system**
- Custom `Material` trait for stylized surfaces
- Procedural texture support (noise-based detail, weathering)
- Vertex color / vertex-driven variation
- GPU-side material instancing

**Crates introduced:** `renderer`

---

### Phase 1: The Planet

**Goal:** Render a procedural exoplanet visible from orbit — atmosphere, ocean surface, cloud layer.

**Milestones:**

1.1 — **Planet mesh**
- Cube-sphere geometry generation (6 faces, normalized to sphere)
- LOD system: chunked mesh with distance-dependent resolution
- UV mapping for texture projection
- Basic terrain heightmap (flat ocean world — minimal land initially)

1.2 — **Atmosphere**
- Use Bevy 0.18's built-in `Atmosphere` + `ScatteringMedium` system
- Configure for an alien atmosphere (tweak Rayleigh/Mie coefficients for non-Earth look)
- Atmospheric scattering visible from orbit (limb glow, horizon color gradient)
- Day/night terminator rendering

1.3 — **Ocean from orbit**
- Specular highlights on ocean surface (sun glint)
- Subsurface color (deep blue/teal based on depth)
- Optional: animated cloud layer (noise-driven, rendered as a shell above the surface)

1.4 — **Starfield and space environment**
- Procedural starfield skybox
- Distant star (sun) as directional light source
- Optional: nearby moon or ring system for visual interest

**Crates introduced:** `planet`

---

### Phase 2: The Space Station

**Goal:** A walkable orbital station interior where the player analyzes the planet below.

**Milestones:**

2.1 — **Station exterior**
- Modular station mesh (ring or cylindrical sections)
- Station orbiting the planet (visible through windows)
- Exterior camera: orbit around station with planet backdrop
- Docking ports for the exploratory vessel

2.2 — **Station interior — geometry**
- Modular room system (corridors, observation deck, lab, command center)
- Window geometry with planet/space view through them
- Basic interior props (consoles, panels, structural elements)
- Procedural detail: pipes, conduits, panel lines (shader-driven, not modeled)

2.3 — **First-person controller**
- Physics-free first-person camera (start with Bevy's `FreeCamera` or `bevy_flycam`, then custom)
- Head bob, smooth movement, collision with station geometry
- Interaction system: look-at highlighting, interact prompts
- Transition between FPS and external orbit camera

2.4 — **Station UI / planetary analysis**
- In-world screens showing planetary data (spectral analysis, surface composition)
- Holographic planet projection in command center
- Data overlays that connect to `sim_core` state (atmosphere composition, ocean chemistry, surface temperature)
- Research progression: unlock tools and knowledge through analysis

**Crates introduced:** `station`

---

### Phase 3: Descent and Ocean Surface

**Goal:** The player descends from orbit in an exploratory vessel and reaches the ocean surface.

**Milestones:**

3.1 — **Descent sequence**
- Camera transition: orbital → atmospheric entry → surface approach
- Atmospheric effects during descent (heat shimmer, clouds passing, sky color shift)
- Planet surface growing from abstract sphere to detailed terrain
- LOD transition: planet-scale mesh → terrain-scale mesh

3.2 — **Ocean rendering — surface**
- Gerstner wave vertex displacement (start with `bevy_water`, then custom)
- Ocean material: subsurface scattering, foam, depth-based color
- Fresnel reflections (sky + atmosphere reflected on water surface)
- Underwater fog / depth fade
- Wind-driven wave parameters (connect to sim_core weather systems later)

3.3 — **Ocean rendering — advanced**
- FFT-based wave simulation (compute shader)
- Foam generation from wave jacobian (whitecap detection)
- Caustics projected onto shallow seabed
- Wake effects from vessel movement
- Screen-space reflections on water surface

3.4 — **Exploratory vessel**
- Vessel mesh (submersible / amphibious craft)
- Third-person camera following vessel
- Basic vessel physics (buoyancy on wave surface)
- Vessel interior (minimal — cockpit with instrument panels)
- HUD: depth, coordinates, surface analysis readouts

**Crates introduced:** `ocean`, `vessel`

---

### Phase 4: Terraforming

**Goal:** The player begins modifying the planet — this is where `sim_core` reconnects with the 3D world.

**Milestones:**

4.1 — **Terrain system**
- Heightmap-based terrain mesh (generated from `sim_core` grid elevation data)
- Real-time terrain deformation (player raises land, mesh updates)
- Terrain material: procedural texturing based on biome (grass, rock, sand, snow)
- Terrain LOD: clipmap or CDLOD for draw distance

4.2 — **Climate visualization**
- Weather particles (rain, snow, dust)
- Cloud system (volumetric or billboard, driven by moisture data)
- Wind visualization (grass/vegetation sway, particle trails)
- Temperature visualization: heat shimmer, frost effects

4.3 — **Life rendering**
- Fungus: ground-cover shader (spreads across terrain procedurally)
- Plants: instanced vegetation (grass blades, shrubs, early trees)
- Growth animation: life visibly spreading over time
- Biome-specific vegetation sets

4.4 — **Sim ↔ Render bridge**
- `sim_core` grid state drives terrain heightmap
- Biome enum drives material/vegetation selection
- Life coverage values drive vegetation density
- Climate proxies drive weather/particle systems
- Event system triggers visual effects (land emerging, biome transitions)

**Crates introduced:** `terrain`, `vegetation`

---

### Phase 5: Settlements and Civilization

**Goal:** Early human settlements emerge on the terraformed land.

**Milestones:**

5.1 — **Building system**
- Procedural building generation (simple shelters → early structures)
- Modular construction: walls, roofs, foundations (inspired by Tiny Glade's procedural approach)
- Buildings placed on terrain, conforming to slope
- Construction animation (buildings assembling over time)

5.2 — **Settlement layout**
- Settlement placement rules (near water, on flat ground, resource proximity)
- Path/road generation between buildings
- Agricultural plots (tilled land, crops — driven by sim_core fertility)
- Infrastructure: wells, walls, gathering areas

5.3 — **Human figures**
- Simple stylized human models (low-poly, painterly)
- Basic animation: idle, walking, working
- Population driven by sim_core stability metrics
- Activity visualization: farming, building, exploring

5.4 — **Base / research facilities**
- Player-built research outposts (connect to sim_core outpost system)
- Advanced structures: labs, observation towers, terraforming equipment
- Technology progression visible in architecture style

**Crates introduced:** `buildings`, `settlement`

---

## Cross-Cutting Concerns

### Camera System (all phases)
- **Orbital camera:** Orbits planet or station (Phase 1-2)
- **First-person:** Station interior, vessel cockpit (Phase 2-3)
- **Third-person:** Following vessel, walking on surface (Phase 3-5)
- **God/overview:** Top-down terraforming view (Phase 4-5)
- Smooth transitions between all modes

### Audio (deferred)
- Ambient soundscapes per environment (space station hum, ocean waves, wind, life sounds)
- UI feedback sounds
- Music: generative/procedural ambient soundtrack
- Not a priority until rendering is mature

### UI Framework (revisit at Phase 2)
- Current: `bevy_egui` for dev overlays
- Long-term: in-world UI (diegetic screens, holographic displays)
- Consider: custom UI rendering in the custom pipeline, or `bevy_ui` improvements
- egui kept as debug/dev overlay regardless

### Save/Load (Phase 4+)
- `sim_core` already has serde support
- Save: sim state + player position + unlocks + camera state
- Autosave on significant events

### Performance Targets
- 60 FPS at 1080p on mid-range hardware (equivalent to GTX 1060 / M1 MacBook)
- Render budget: ~16ms frame time
- Sim budget: fixed timestep, decoupled from render
- Profile early and often — custom renderer enables fine-grained GPU budget control

---

## Current Status

**Completed:**
- `sim_core`: deterministic simulation with terraforming, climate, life spread, biome derivation
- `app`: basic 2D texture renderer, egui UI, mouse input, fixed-timestep sim integration
- Architecture: clean sim/render separation, engine-agnostic simulation core

**Next immediate step:** Phase 0.1 — Custom render pipeline skeleton in a new `renderer` crate.

---

## Reference Material

### Tiny Glade Rendering (primary inspiration)
- GPC 2024 talk: "Rendering Tiny Glades With Entirely Too Much Ray Marching" by Tomasz Stachowiak
- Tony McMapface tonemapper: github.com/h3r2tic/tony-mc-mapface
- Depth buffer ray marcher: gist.github.com/h3r2tic/9c8356bdaefbe80b1a22ae0aaee192db
- Kajiya GI renderer (precursor): github.com/EmbarkStudios/kajiya

### Bevy Rendering
- Custom render phase example: bevy.org/examples/shaders/custom-render-phase/
- FullscreenMaterial API (0.18): bevy::core_pipeline::fullscreen_material
- Atmosphere + ScatteringMedium (0.18): bevy.org/news/bevy-0-18/
- Solari (experimental RT): bevy's built-in raytraced lighting

### Ocean Rendering
- bevy_water: github.com/Neopallium/bevy_water (Gerstner waves, Bevy 0.18 compatible)
- Tessendorf "Simulating Ocean Water" (2001) — FFT-based wave simulation

### Planet Rendering
- Cube-sphere with LOD: blog.graysonhead.net/posts/bevy-proc-earth-1/
- bevy_terrain (UDLOD): github.com/kurtkuehnert/bevy_terrain

### Camera
- bevy_flycam: github.com/sburris0/bevy_flycam (Bevy 0.18)
- bevy_fps_controller: github.com/qhdwight/bevy_fps_controller
- dolly (composable camera rigs): github.com/h3r2tic/dolly
