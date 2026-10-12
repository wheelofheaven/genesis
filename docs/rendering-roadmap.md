# Genesis renderer roadmap

## Current direction — 12 October 2026

Build **a scalable, stylized renderer for a changing, inhabited landscape**.
The primary visual benchmark is the normal settlement-management view.
Close inspection, including the established eye-level requirement, and valley overview remain essential companion views.
Terrain, water, vegetation, camera behavior and interaction receive first priority.
This direction updates the delivery priorities below; the earlier assessment against `d81ed33` is retained as historical evidence, not a current source inventory.
Rust, the custom wgpu renderer and simulation independence remain binding.

| Quality | Acceptance intent |
| --- | --- |
| Readable | Terrain, buildings, paths, people and resources communicate their function at management distance; close-up detail alone cannot pass the scene. |
| Responsive | Moving the camera, hovering, selecting and previewing construction avoid recurrent stalls; measure edit and camera-transition spikes as well as steady frames. |
| Dynamic | Changed terrain, water, cover and settlement state produce coherent visible updates, including dependent borders and picking. |
| Scalable | Visible importance controls mesh, texture, foliage, animation, shadow, effect and label detail; camera visibility never decides simulation outcomes. |
| Faithful | Presentation represents committed domain state; decorative motion does not imply an unimplemented resource flow or activity. |
| Maintainable | Materials, lighting, assets and passes can evolve without rewriting domain rules or storing GPU objects in saves. |

Keep the existing forward renderer and explicit passes.
Keep deliberate CPU/GPU upload layouts, capability checks, optional timing counters, SDR presentation and correct color handling.
Preserve the separation of simulation, game presentation and GPU ownership described in the [architecture brief](architecture/GENESIS_ENGINE_ARCHITECTURE.md).
Global/camera-relative coordinates remain a future boundary: current region-centered f32 rendering is not a planetary precision implementation.
Do not change persistent coordinates or save identities to move the render origin.

### Current implementation and demonstrated gaps

Source review includes `app/src/terrain.rs`, `terrain/lod.rs`, `water/`, `view.rs`, `view/preparation.rs`, `benchmark.rs`, and `renderer/src/world_mesh.rs`, `instances.rs`, `assets/` and `camera.rs`.
Paths are relative to `crates/`.
The [asset workflow](assets.md) and [extraction measurements](architecture/extraction-performance.md) supply the existing measured evidence; this documentation update did not rerun benchmarks.

| Area | Retain | Remaining work |
| --- | --- | --- |
| Terrain and interaction | Adaptive continuous terrain, shared surface queries, triangle picking, conforming paths/water, validated placement, chunk culling and fingerprint-based upload reuse | A terrain revision still rebuilds the whole CPU cache and picker synchronously in `View::ensure_terrain`; urgent scene extraction is also synchronous. GPU upload reuse does not establish local CPU rebuilds. |
| Derived work | One outstanding background preparation, immutable input, revision/view/edit guards and matched terrain/water publication | Local invalidation with sampling halos and bounded publication/upload work; retain the stale-result guards and correct picking while changing this path. |
| Water | Coast and river geometry, channel beds, confluence handling, depth/shore treatment and optical shading | Generated rivers are not a live volume/flow solver. Canals, reservoir storage, flooding and changing flow need authoritative domain support before their visuals can claim those meanings. |
| Vegetation and architecture | Instanced scenery, foliage LOD, static imported dwelling with three LODs, material textures and transactional F6 reload | Species/content variety, smoother transitions and incremental updates; decorative vegetation must remain distinct from future harvestable identities. |
| People and building activity | Stable domain building identities and simulation-validated placement | No represented inhabitants, production construction stages or skinned character pipeline. Current GLB import explicitly rejects skinning/animation. Do not certify an inhabited settlement using decorative actors. |
| Light and atmosphere | Sun shadows, shared sky fill, cloud shade, haze, HDR intermediate and tone mapping | Stable motion at all view scales, coherent future weather inputs, local-light/night budgets and readability. Current weather previews are presentation settings. |
| Diagnostics and lifecycle | Resident render benchmark, CPU extraction benchmark, native interaction/resize checks, bounded scene worker | Full interaction/streaming frame traces, memory/upload budgets and complete device reconstruction. A resident benchmark excludes interactive streaming and simulation. |

The existing M4 development-profile extraction report measured cold cache medians of 288.573 ms at regional view and 485.658 ms near the settlement.
Those are isolated CPU construction timings, not frame times or a new performance result.
They make terrain preparation a concrete responsiveness investigation before expanding effects.
Snapshot cloning measured roughly 0.5 ms, so replacing simulation storage or adding a new thread system is not justified by this evidence.

### Updated delivery order

1. **Verify existing foundations under interaction.** Keep the authored assets, depth/color conventions, CPU picking and benchmarks. Extend measurement to cache stages and a repeatable moving-camera/edit route, with actual resolution, profile, seed, quality, CPU/GPU timing availability and upload bytes. Use existing stage hooks where present. The provisional M4/1080p/60 FPS target below remains a target, not an achieved result or minimum hardware promise.
2. **Make local landscape changes responsive and coherent.** Carry affected regions from committed edits into scoped cache rebuilding, including neighboring samples, river/ground contact and picking dependencies. Retain stable seams, bounded work and stale-result rejection. A small edit should not require a whole-map CPU rebuild; verify unchanged regions and compare equivalent before/after timings before accepting the optimization.
3. **Connect an inhabited settlement as domain mechanics arrive.** Add character import/animation and activity cues against persistent people and actual tasks; show construction/growth states only when owned by the simulation. Continue improving instanced vegetation and a coherent building kit. Authoring fixtures may use synthetic state only when clearly labelled.
4. **Make the landscape visually coherent.** Review sun, shadows, water, sky, wind, wetness and materials together. Prioritize shoreline coverage/elevation and readable flow cues over elaborate reflections. Weather and darkness must leave selection, surveys and construction usable.
5. **Scale and polish the proven scene.** Improve LOD transitions, foliage/shadow stability, animation scheduling, resource residency and upload budgets against measured bottlenecks. More advanced draw submission, a general render graph, ray tracing and planetary terrain need separate evidence and scope.

The next implementation slice is **scoped terrain-cache invalidation**, using the [committed-edit baseline](architecture/terrain-edit-performance.md) as the full-rebuild reference.
The benchmark now measures a real nine-sample edit through Session: only two of 924 prepared chunks differ from a rebuilt same-tick control, while CPU preparation takes roughly 261–526 ms across the three views on the development machine.
Ordinary environmental recoloring also changes uploads, so distinguish geometry and appearance invalidation.
Stage profiling and the adaptive receiver-query optimization remain in use; see the [cache report](architecture/cache-performance.md).
Do not combine cache representation, hydrology rules, threading and rendering algorithms in one change.
Keep explicit frame/pass resource ownership and the current GPU API versions.
Asset/animation work can progress independently when selected; it does not make absent gameplay a prerequisite for terrain improvements.

### Acceptance scene and motion review

The integrated target is a growing settlement beside an actively changing irrigation system.
It becomes an acceptance gate when people, growth, work and irrigation exist in the authoritative simulation; this renderer direction does not authorize implementing those systems as a checklist.
Until then, use the actual coastal starter settlement, real terrain/placement/well commands and documented limitations.
Pumping aquifer water is not an irrigation distribution loop.

Use a repeatable route through settlement management, close inspection and valley overview, including a slow orbit, forest zoom, water pan, crowded-object selection and a local edit across a terrain patch boundary.
Add following a working person when that mechanic exists.
Review silhouettes, shimmer, shadow stability, LOD transitions, depth flicker, exposure and stalls in motion, alongside captures.
The route should verify terrain/picking/preview agreement, valid/invalid cues using more than color alone, and continued domain behavior while off-screen.
Roof/vegetation occlusion handling and citizen selection remain explicit interaction requirements when applicable.

Record cold and warm behavior, median and tail CPU costs, available per-pass GPU timings, upload volume, visible instances/draws and memory where measurable.
Missing counters remain unavailable, never zero.
No new FPS, dense-population, browser, WebGL fallback, console or device-recovery claim follows from this roadmap.

## Earlier visual-quality assessment

Assessment: October 12, 2026, against renderer commit `d81ed33`.
The static GLB dwelling and resident settlement benchmark were subsequently implemented; see [workflow, baseline and remaining gates](assets.md).
The capability table and six-stage plan below preserve that assessment and its detailed art/quality gates.
Use the current source inventory and updated delivery order above when choosing new work.

## Outcome

Build one convincing, inhabited settlement that holds together at regional,
settlement and 1.7 m eye-level views. Use Anno 1800 as a benchmark for visual
richness, composition and readability; retain Genesis's organic layouts, physical
scale, Levant starting scenario and separate advanced Elohim interface. Copying
nineteenth-century architecture or Anno's proportions would not serve this setting.

Our largest gaps are asset quality and variety, consistent material/lighting
response, scene density and animated life. More procedural noise, cloud work or
post-processing alone will not close them. Build the content pipeline alongside
the renderer, and judge complete scenes as well as isolated effects.

## Evidence from Anno, and what we infer

Ubisoft's [asset-production account](https://www.anno-union.com/devblog-of-3d-architects-and-construction-workers/)
describes concept/blockout iteration, detailed source models, simpler runtime
meshes, normal baking and material work. Its
[visual-feedback account](https://www.anno-union.com/devblog-visual-feedback/)
describes props, effects, animation placement and building interaction setup.
Our inference: a repeatable authoring workflow and convincing scene content are
first-order graphics requirements, not polish to add after completing the engine.

The [Intel/Ubisoft rendering report](https://www.intel.com/content/dam/develop/external/us/en/documents/bringing-anno-1800-to-laptop-gamers-around-the-world-the-long-road-from-10-to-30-fps.pdf)
documents instancing, screen-size culling, building LODs, depth-distributed sun
shadows with GPU caster partitioning, irradiance lighting, Cook–Torrance shading,
HDR, reduced-resolution fog, and several scales of ocean waves. It also documents
quality reductions for integrated GPUs. These are historical Anno techniques,
not instructions to copy its exact implementation or its hardware results.

Ubisoft's [engine discussion](https://www.anno-union.com/devblog-the-anno-engine/)
emphasizes fast camera movement and region availability for a builder. Genesis
also makes eye-level play a primary requirement. We therefore need camera-aware
detail and bounded streaming, rather than treating an overhead view as sufficient.
The [day/night account](https://www.anno-union.com/bringing-light-into-darkness/)
also shows that night presentation required asset and lighting changes together.

## Capability assessment

“Partial” means working foundation, not production-complete. Priorities below
describe Genesis needs and recommendations, not additional claims about Anno.

| Area | Current Genesis | Required capability and priority |
| --- | --- | --- |
| Art and content | Procedural dwelling kit, small vegetation/rock library | **First:** authored building/plant/prop kits, controlled palette, silhouette variety, believable wear and environmental storytelling; review at all three camera scales. |
| Asset pipeline | Meshes largely generated in Rust; no general imported-scene pipeline | **First:** glTF/GLB import, indexed meshes, UVs/tangents, material slots, prefab hierarchy, collision/picking metadata, LOD chains, dependency tracking and hot reload. |
| Materials | Packed numeric tags, procedural colour/relief; GGX for water and stone | **First:** explicit material handles and shared PBR inputs: base colour, normal, roughness, metallic, AO and emissive; correct colour spaces, mipmaps, anisotropy and supported compressed formats. Keep procedural masks as complements. |
| Sun and skylight | Fixed sun; some CPU-baked directional shading; artistic ambient tint | **First:** one sun/sky lighting state; separate albedo, normals and occlusion from illumination; diffuse sky irradiance and roughness-filtered specular environment lighting. Then moving sun, moon and exposure presets. |
| Indirect light | Local static architectural visibility and ground contact approximation | **Next:** sky/environment probes, validated contact AO, restrained terrain/building bounce approximation. Consider local probes for shaded courts/interiors. AO must not substitute for bounce light. |
| Shadows | Two fixed 2048² nested maps, PCF, cached cloud transmission | **Next:** allocate coverage/detail by view and quality tier; improve small-object contact, caster LOD/culling and distant coverage. Evaluate adaptive cascades before complex GPU distribution. |
| Terrain and roads | Continuous sampled world, adaptive meshes, procedural materials, conforming paths | **Next:** layered soil/rock/vegetation materials, macro-to-micro consistency, authored cliff/outcrop meshes, blended object/ground transitions, independent decals for wear, dirt and construction. Preserve seam-free edits and physical scale. |
| Vegetation | Instancing, three LODs, cutout foliage, simple motion and translucency | **First content slice, then scale:** better species silhouettes, crown/branch structure, atlases with coverage-preserving mips, coherent wind, understorey and habitat transitions. Later tree impostors, seasonal variants and animation LOD. |
| Water | Procedural optical bed, ripples, sky/cloud reflection and sun highlights | **Next:** shore interaction, foam and flow detail, reflections of nearby land/buildings with fallbacks, then larger ocean waves and object wakes. Keep river, coast and open-ocean treatments distinct. |
| Atmosphere/weather | Scattering LUT, one cloud layer, analytical aerial depth, wind/shadows | **Next:** coherent cloud/sky/surface lighting and stable distant clouds. Later weather transitions, multiple cloud types, local fog, precipitation, wetness, puddles, snow and lightning. Presentation consumes deterministic weather state. |
| People and movement | No production character animation/crowd renderer | **Core visual requirement:** skinned characters, animation blending, attachments, locomotion, animation LOD and visible work routines; animals and appropriate vehicles later. Preserve individual identity while rendering only relevant agents. |
| Effects and night | No general particle/local-light system | **Next:** bounded smoke, dust, fire, splashes and soft particles; point/spot lights, emissive windows, lamps and effects anchors. Tie important activity to actual simulation events. |
| Image stability | 4× MSAA, filtered procedural detail, HDR and neutral tone mapping | **Next:** measure foliage/cloud shimmer in motion; motion vectors and history rejection if adopting TAA/upscaling. Add gentle bloom and colour grading after lighting is coherent; keep UI and surveys legible. |
| Scale and streaming | CPU culling, scenery instancing, chunk reuse, background terrain jobs | **Before large cities:** building instancing/LOD, asynchronous asset loading, bounded residency/upload work, screen-size culling and budgets. Add hierarchical depth occlusion/GPU indirect draws only when profiling warrants them. |
| Authoring and diagnostics | Capture commands, opt-in GPU regressions, incomplete pass timings | **First:** representative benchmark scenes, reliable GPU/frame-time analysis, material/lighting debug views, asset validation and an in-game inspection workflow. Artists must iterate without editing engine source. |

For the interchange contract, start with a documented subset of
[glTF 2.0](https://registry.khronos.org/glTF/specs/2.0/glTF-2.0.html), whose material
model supplies standard metallic/roughness, normal, occlusion and emissive inputs.
Reject unsupported features explicitly. Validate imported units and transforms,
then convert metres at the application boundary rather than changing simulation
units. Asset sourcing, licences, creation time and consistent art direction remain
real production work even after import succeeds.

For lighting, use the established diffuse/specular environment-light separation
described in [Filament's reference](https://google.github.io/filament/main/filament.html).
This is a proposed Genesis implementation, not a migration to Filament. Full
real-time GI, ray tracing, virtualized geometry and a deferred-renderer rewrite
are not prerequisites for the first convincing settlement. Keep the existing
forward renderer, evaluate clustered light lists when local-light counts demand
them, and grow explicit pass/resource ownership as new passes need shared buffers.

## Delivery sequence and acceptance gates

### 1. Establish a trustworthy comparison

Create repeatable settlement, riverbank, forest and dense-city benchmark scenes,
plus a continuous orbit-to-ground camera route. Record actual physical render
resolution, quality settings, seed, GPU/OS, warm-up, CPU and GPU frame times,
median/p95/p99, peak resident memory, draw/triangle counts and upload spikes.
Investigate unreliable world-pass timestamps with native capture tooling; never
interpret missing timings as zero. Separate simulation pressure from render work.

Provisional target pending hardware preference: Apple M4, 1920×1080 internal world
resolution, 60 FPS (16.67 ms frame interval), native-resolution UI. A candidate GPU
budget is about 12 ms with headroom; CPU critical-path and simulation budgets must
be measured separately because CPU/GPU work overlaps and shares memory pressure.
Add a 30 FPS quality tier and higher-resolution tiers only against measured scenes.
These are engineering targets, not achieved performance or a hardware guarantee.
Fix scene size, active-agent count and visible-asset density before calling a
benchmark representative; an empty landscape cannot certify city performance.

Gate: reproducible stationary and moving-camera measurements, honest unavailable
counters, and no shader compilation/large synchronous uploads during ordinary travel.

### 2. Build the first authored, consistently lit asset

Import one representative dwelling and its material textures, with at least three
LODs and separate picking geometry. Add stable mesh/material handles, normal and
UV/tangent attributes, texture mip generation and a development reload path.
Introduce a material-backed draw path alongside existing procedural geometry;
avoid changing every old material tag at once.

Separate baked directional light from base colour on this path. Feed one sun
direction/colour and sky environment to material shading, shadows and atmosphere.
Use a neutral material test scene to verify colour space, roughness and normal
orientation; then put the dwelling into the real settlement. The imported asset
must cast/receive shadows and remain selectable without renderer data entering saves.

Gate: the same asset reads convincingly under clear and overcast light, and at
eye/settlement/regional scales, without an increase in coarse-distance geometry.
Do not scale the art library until this import-to-game loop is usable.

### 3. Make one settlement visually complete

Proposed bounded slice: a roughly 200 m × 200 m area with 12–20 dwellings, three
architectural variants, one communal/work building, a curated prop kit, two tree
species, shrubs, ground cover, a path junction and a riverbank. Counts are planning
fixtures, not statements about historical settlement composition.

Add convincing material transitions, foundation contact, leaf silhouettes,
sky fill, small-scale shadows and basic water reflections. Use real props and
architectural detail instead of adding noise to empty surfaces. Support limited
controlled dressing on top of seeded generation; test other seeds after the hero
scene works. The slice should establish the quality target for later content.

Gate: agreed reference captures in fair/overcast light, with clear silhouettes,
consistent metre scale and texture density, no floating contacts, obvious seams
or disappearing major forms. Review slow pans and camera transitions, not only stills.

### 4. Make it feel inhabited

Introduce a small animated population, task-related props/effects, wind shared by
plants and weather, local lights and an evening setting. Start with a few useful
animation states and authored activity anchors. Population and activity must come
from simulation state; labelled renderer fixtures may use synthetic test agents.

Gate: motion and light explain who is doing what, preserve selection readability,
and meet the same moving-camera budget. Animation and shadow LOD must reduce work
without changing persistent identity or off-screen simulation.

### 5. Scale the proven scene

Test progressively larger populated scenes. Instance architecture, simplify small
props by projected size, reduce distant animation/shadow work, and stream assets
with bounded uploads. Evaluate impostors, merged distant groups, depth occlusion
and indirect draws against measured bottlenecks, not a feature checklist.

For Earth/exoplanets, separate double-precision/global location from a local render
origin and stream regional detail. Keep coarse geography resident or cheaply
recoverable, prefetch for fast camera travel, and shift the local origin without
breaking water, material coordinates, shadows or temporal history. Rendering a
planet does not mean keeping its entire human-scale surface resident.

Gate: stable camera travel and region changes under the selected memory and frame
budgets, no worker backlog or visible object duplication/loss, and quality controls
that reduce expensive work while retaining information needed to play.

### 6. Broaden atmosphere and finishing effects

Add the wider day/night and weather range, precipitation/surface accumulation,
ocean interactions and optional cinematic effects after the scene and asset
pipeline are sound. Introduce temporal reconstruction only with valid motion
vectors, disocclusion rejection and reset rules for camera jumps, edits, weather
changes and origin shifts. Evaluate ghosting as carefully as aliasing.

Gate: each weather/time combination has coherent sky, surface, water and local
lighting; low-quality modes preserve gameplay cues; surveys remain trustworthy.

## Remaining decisions

Final hardware/resolution targets, asset-production capacity, maximum visible settlement density and the extent of eye-level interiors remain open.
Use the current delivery order above rather than repeating the already implemented first dwelling/benchmark work.
Keep the earlier art and quality gates as references, and preserve the scope and limitations documented in [the asset workflow](assets.md).
