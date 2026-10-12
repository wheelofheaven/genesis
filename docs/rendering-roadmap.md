# Roadmap to Anno-class visual quality

Assessment: October 12, 2026, against renderer commit `d81ed33`.
Status: proposed implementation sequence, not a claim of feature parity or an
accepted hardware requirement. Existing product and custom-engine decisions stand.

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

## Recommended next implementation

Deliver the benchmark fixture and the first imported, material-backed dwelling
with shared sun/skylight. This unlocks visible gains across architecture,
vegetation and terrain, and makes subsequent renderer work judgeable against real
content. It is a stronger next milestone than another isolated shader embellishment.

Unresolved decisions: final hardware/resolution targets, asset-production workflow
and capacity, desired maximum visible settlement density, and how far eye-level
interiors belong in the first playable slice. Keep these explicit rather than
silently assuming a high-end GPU or a full AAA art team.
