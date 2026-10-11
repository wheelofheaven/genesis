# Rendering direction and measurements

Genesis should look convincing from a person's eye level through a regional view,
with stable silhouettes, natural lighting and readable scientific overlays.
Keep the renderer focused on this game and measure improvements on the target
hardware. The simulation does not depend on presentation quality or visibility.

## Visual target

The accepted references are Anno 1800, Breath of the Wild and Manor Lords. Our
interpretation for Genesis is settlement readability and richness, expressive
natural light and atmospheric depth, and grounded materials and organic layouts.
Reference galleries: [Anno 1800](https://www.ubisoft.com/en-us/game/anno/1800),
[Breath of the Wild](https://zelda.nintendo.com/media/),
[Manor Lords](https://manorlords.com/). These are art-direction references, not
claims that Genesis uses their rendering techniques or will match their assets.

Keep scale, silhouettes and visual hierarchy coherent before adding detail. The
remaining large gains require authored vegetation and architecture, richer material
response, richer shadows and measured lighting improvements. Procedural
noise and post-processing alone cannot establish the intended quality.

## Vegetation at settlement boundaries

Cosmetic scenery and close ground cover now share a disposable construction
footprint index. House clearances follow rotated walls, the stone plinth and the
front step, replacing the circular bare patches around dwellings. Path clearances
include the feathered shoulder and the same sampled entrance walks used for
drawing. Grass, rushes, pebbles and litter allow for their horizontal mesh reach,
instance scale and a small wind margin; shrubs and rocks use wider clearances than
tree trunks along walks. Trees retain crown clearance around buildings while
allowing branches to overhang paths.

An 8 m spatial index restricts queries to nearby footprint segments. Long walks
are subdivided using the existing half-metre path samples, avoiding large diagonal
bounding boxes. It is rebuilt with the existing scenery/terrain caches, adds no
GPU pass or shader work, and does not modify simulation vegetation or construction
rules. This is visual clearance, not a growth, trampling or collision simulation.

The understorey crown index now applies the same construction and sea-level
visibility rules as tree rendering. Hidden trees no longer generate cosmetic
shade-dependent grass and litter. Tests compare indexed queries with exhaustive
geometry across cell boundaries and unequal margins, check rotated walls and
entrance links, and cover tree removal by buildings, paths and rising sea level.

## Terrain-conforming footpaths

Committed paths and cosmetic entrance walks now use a lit soil surface instead
of an unlit ribbon 6 cm above the ground. Each short strip is clipped against the
actual triangles of the current adaptive terrain cache. Intersections inherit
interpolated edge coverage, and heights follow the receiver plane with a 1.2 cm
vertical offset to prevent depth fighting. This avoids floating spans and buried
segments across terrain creases without changing the physical surface.

Four vertices across the strip define an opaque centre and feathered margins.
The outer margin extends 8 cm beyond the simulated half-width; coverage falls
across a roughly 22 cm shoulder. Ends also fade. Material tags 14–15 encode the
coverage in the existing vertex format. World alpha-to-coverage uses the existing
4× MSAA samples to expose terrain at the margins; opaque world fragments still
output alpha one. Small world-anchored irregularities, soil grain and mineral
chips filter out with pixel footprint. Soil tint follows local rock and moisture.
This is cosmetic wear, not traffic erosion or simulated compaction.

Paths receive geometry/cloud shadows and atmospheric haze. The spatial batcher
keeps them outside the caster prefix, alongside water, so the decal cannot shadow
the ground beneath it. There is no additional texture, pass or render target.
Clipping adds CPU preparation and geometry: the default house capture uploads
497,028 more vertex bytes (about 0.47 MiB) than the preceding building pass, with
the same 924 world chunks. That is one fixture, not a universal bound or frame-rate
claim. Nearby terrain patches are deduplicated before clipping; the whole regional
triangle list is not scanned for each path segment.

Entrance links choose the nearest point on stored path segments, rather than a
possibly distant waypoint. They remain presentation links and do not implement
obstacle-aware routing or add simulation paths. User-built route validation,
selection and construction actions are unchanged. Interaction overlays retain
strong solid colours for immediate feedback.

Path hover, selection, draft surfaces and waypoint rings share the terrain
clipping operation. Their unlit colours remain intact, with a 4 cm ground offset
and an additional 8 mm for the selection's inner fill. This replaces the older
9.5–10.5 cm raised selection ribbons. Invalid ocean previews remain above sea
level. A completed terrain-cache update also invalidates the small overlay buffer,
so highlights follow changes in terrain detail while the pointer is stationary.
The initial cache-loading fallback retains the original ribbon feedback. Tests
check projected width/area, interior height agreement at two detail levels,
unchanged interaction colours and ocean visibility. Hover still updates only the
overlay buffer; clipping adds CPU work when that buffer changes.

Building placement outlines, selected building footprints and the ground marker
also use the shared triangle clipping. Closed loops join with bounded miters,
preserving line width through ordinary corners and avoiding the former endpoint
seam; very sharp turns cap the extension at twice the half-width. Roof outlines
remain at roof height. Seafloor mode drapes ground markings directly on the
visible seabed triangles instead of moving every overlay vertex onto a separately
sampled physical surface afterward. This also avoids distorting selected building
shells when sea level changes. Without a terrain cache, the seabed fallback uses
the physical surface.

Highlight generation builds planar top faces directly, eliminating the terrain
height queries and discarded back faces previously used to prepare clipping.
Tests cover rotated closed-loop area, bounded degenerate turns, and full ground
marker/invalid building previews above water and on the seabed at two detail
levels. These changes add no rendering pass or GPU resource type.

Tests cover complete projected area/width, finite attributes, edge coverage and
height agreement within rendered triangles at close and distant terrain LODs.
Batching tests exclude both paths and water from shadow draws. Native house and
settlement captures check appearance; the interaction smoke test exercises
selection, path previews and paused construction. Remaining limits include the
four-sample edge coverage, cosmetic entrance routing, and rebuilding decal
geometry with the scene rather than maintaining an independent decal cache.

## Close dwelling materials and shadow filtering

Dwellings retain the 6.5 × 4.8 m footprint, 2.8 m walls and 3.3 m roof/parapet
height. The front wall now has actual openings in a 30 cm shell, with recessed
closed plank doors and shutters, narrow board joints, timber battens/lintels and
stone sills. Foundation masonry uses separate small blocks over a continuous
support. Detailed courses stop after the upper 1.4 m even when terraforming
exposes a much deeper support, keeping geometry below the tested 16,000-vertex
budget per dwelling. This is a procedural visual proxy, not a historical
reconstruction, structural solver or traversable interior.

Material tags 3.0/3.1/3.2/3.3 distinguish plaster, stone, timber and roof earth.
Triplanar metre-scale noise filters grain by pixel footprint. No texture, vertex
attribute or render pass is added; all geometry uses existing spatial world
batches. Directional face lighting is baked after rotation, so sun-facing walls
respond consistently. Terrain and building material normals now normalize the
screen-space cross product before selecting a degenerate fallback. Previously,
a fixed upward epsilon overwhelmed small derivatives and made vertical walls
sample horizontal texture coordinates, producing vertical stripes.

Shadow PCF now projects each comparison tap onto the local receiver plane and
adds a half-texel allowance for hardware filtering. This removes the diagonal
self-shadow bands seen on walls while permitting smaller constant depth offsets.
World derivatives are calculated before foliage discard; the two maps transform
them separately, with bounded slopes and a degenerate-projection fallback.
[NVIDIA's shadow-filtering discussion](https://developer.nvidia.com/gpugems/gpugems3/part-ii-light-and-shadows/chapter-8-summed-area-variance-shadow-maps)
explains why neighbouring PCF samples increase bias requirements. Genesis keeps
comparison-depth PCF; it does not adopt that chapter's variance-map technique.
Small features remain limited by the near map's approximately 16 cm texels;
this is not a contact-shadow pass.

Picking rejects rays outside padded horizontal building bounds before generating
or testing detailed triangles. The mesh builder reuses the caller's allocation.
Tests cover rotated world lighting, closed recessed doors, finite nondegenerate
geometry, bounded cost after large terrain changes, and conservative picking
against triangle hits at multiple angles. Native house captures and the
selection/path smoke test check shader compilation and interaction composition.
Use `--house-capture output.png` for the 1.7 m inspection camera at the first home.

## HDR world and display transform

World geometry, water and sky now render into 4× MSAA RGBA16Float, then resolve
into a persistent linear HDR texture. A single full-screen pass applies fixed
exposure and [Khronos PBR Neutral tone mapping](https://github.com/KhronosGroup/ToneMapping/tree/main/PBR_Neutral).
This preserves highlight values until display conversion; the sun disk and water
glints can exceed 1 without clipping in the world attachment. Fixed exposure avoids
brightness changes caused solely by camera framing. This is HDR internal storage
with SDR output, not HDR-monitor support, bloom or eye adaptation.

The display pass writes linear colour into the sRGB output view, which performs
the encoding once. Glass consumes that display image through sRGB textures; egui
still draws afterwards through its gamma-space UNORM view. UI colours are never
tone mapped. Scientific surveys bypass both exposure and the tone curve and retain
their previous lighting instead of receiving the new sun/sky colour tint.

Opaque surfaces use a modest warm sunlight tint and cooler, deeper cast shadows.
This is an art-directed lighting treatment on the existing normal/baked shading,
not PBR energy conservation or indirect illumination. Shadow coverage now uses
the two nested maps described below.

Resize recreates attachments and bindings while retaining compiled pipelines.
The cost is one full-screen read/write pass and 24 additional nominal attachment
bytes per physical pixel relative to the former SDR/MSAA route: 16 for the larger
four-sample colour attachment and 8 for the HDR resolve. At 2560×1600 that is
93.75 MiB additional nominal storage, before driver allocation/tile optimizations.
Geometry, draw submissions and texture assets do not increase; GPU time still
needs profiling before selecting quality tiers or adding further post effects.

Native regression:
`mise exec -- cargo test -p genesis-app hdr_display_preserves_highlights_survey_colours_and_resize -- --ignored --nocapture`.
It uploads known half-float swatches, including values above 1 and a negative
input, executes the real display shader, and reads pixels back. It checks distinct
HDR highlights, neutral greys, black, alpha, survey bypass, one sRGB encoding and
resized targets. Riverbank/settlement captures and the native survey resize smoke
test cover world, glass and UI composition. See [third-party notices](../THIRD_PARTY.md)
for the tone mapper's licence and adaptation.

## Nested settlement shadows

Two 2048² Depth32Float layers now cover 320 m and 1.28 km across in the light's
image plane. Their fixed extents and independently snapped light-space centres
keep texel scale stable as the camera moves. These are nested orthographic maps
centred on camera focus, not view-frustum splits or planet-wide shadow coverage.
The near depth extent is 1.6 km and the wider map's is 5.12 km, allowing terrain
and elevated off-camera objects to cast into the receiving area.

Receivers use nine comparisons in the selected map; only the overlap samples
both. The detailed map blends into the wider map over 72–94% of its half-extent,
and the outer map fades over 82–98%. Pixels outside both maps skip shadow reads.
Water uses the same visibility function for its highlights. The maps have separate
uniform buffers, so both projections survive one command submission; the shared
camera/atmosphere uniform is now 368 bytes, including cloud parameters.

Each layer has independent world-chunk and scenery culling. Near scenery uses the
existing medium-detail caster meshes; wide scenery uses far meshes. Non-tree
objects with conservative bounds smaller than a 2 m radius skip the wider layer;
grass, rushes, pebbles and litter cast only into the near map. Casters remain
independent of camera visibility. The fixed geometry choices avoid shadow LOD
switches while orbiting, but far tree silhouettes are approximations.

Cost: one additional shadow pass, 16 MiB additional nominal depth storage, a second
set of instance buffers and extra overlap sampling. In the default coast landscape
capture, the near layer submits 21,612 world vertices plus 1,666,440 scenery
vertices (1,142 instances); the wide layer adds 101,061 world vertices plus
5,198,532 scenery vertices (8,438 instances). Captures report both layers explicitly.

Updated Apple M4 CPU visibility samples (30 calls, optimized development profile):

| Camera | Clustered ms | Full scan ms | Objects tested | Near casters | Wide casters |
| --- | ---: | ---: | ---: | ---: | ---: |
| Settlement | 0.241 | 2.136 | 20,561 | 1,142 | 8,438 |
| Ground | 0.273 | 2.141 | 23,066 | 1,142 | 8,438 |
| Region | 0.788 | 2.259 | 114,034 | 1,142 | 8,438 |

Optional GPU timestamp queries record shadow and world passes on capture frames
only. Readback occurs during the already-blocking PNG capture; ordinary frames
neither record nor wait for timing results. Unsupported adapters omit timings.
Missing, equal or decreasing timestamps are reported as unavailable rather than
zero milliseconds. The local Metal world-pass counter returned invalid samples;
its cause is not established. One landscape sample reported 1.347 ms near and
2.547 ms wide; one ground sample reported 2.057 ms near and 2.005 ms wide. These
are individual pass observations under desktop load, not controlled benchmarks,
additive frame-time estimates or FPS guarantees; display, glass and UI are excluded.

Regression coverage checks stability within a texel for both maps, containment of
near receivers by the wide map, far-only tree casters, tiny-object rejection,
off-camera retention, and clustered/brute-force agreement for both caster lists.
Native landscape and ground captures validate both render passes and blending.
Depth bias, coarse distant silhouettes, temporal aliasing and missing shadows
outside the wide map remain limitations; there is no contact-shadow or GI pass.

## Shared cloud layer

A world-anchored cloud sheet sits 1,800 m above the scenario's sea datum. A seeded,
domain-warped density field repeats over 16 km; its ten mip levels are generated
once on the CPU and uploaded as a 512² R8Unorm texture. The nominal payload is
349,525 bytes (341.3 KiB), independent of viewport resolution. There is no extra
render pass, full-screen attachment, per-frame texture upload or geometry.

Sky rays intersect the sheet, use the projected pixel footprint to select a mip,
and sample toward the sun for a softly lit edge and cooler interior. Distant
clouds blend into the horizon haze. Water evaluates the same field at the reflected
ray intersection, with a wider filter cone to suppress small bright ripples.
Opaque surfaces trace toward the sun to this same sheet and attenuate the direct
lighting term; sky fill remains. Shadows use at least 125 m filtering and cover
the whole visible region independently of the geometry shadow maps. Receivers
above the sheet receive no cloud shadow. Water's body and glint are attenuated
before its sky reflection is combined.

`Atmosphere::clouds` separates height, artistic cover and shadow strength. Zero
cover bypasses the effect, and scientific surveys force zero cover. The current
8/4 m/s drift follows presentation time, like ripples and foliage; it is unrelated
to simulation speed or climate. A 4,000-second phase wrap is continuous on both
texture axes. Native captures retain their fixed presentation phase.

The shader adds one density sample per affected opaque fragment and up to three
per water fragment (two for the reflected cloud); open sky uses two. These are
sample counts, not measured frame-time savings. This is a single, artistically
shaded sheet, with no volumetric thickness, storm evolution, precipitation,
light shafts or planetary curvature. Flying through it and intersections with
mountains are not resolved as volumes. The repeated pattern and single weather
look remain limitations; exoplanet atmospheres will need their own parameters.

Tests cover periodicity, finite bounded density, repeatable generation, mip sizes
and average preservation, phase wrapping, sea datum and survey bypass. Native
Metal captures check riverbank sky/reflection and regional shadows; the survey
smoke test also checks surface resizing with the new binding layout.

## Unreal references

Epic's [Sky Atmosphere documentation](https://dev.epicgames.com/documentation/en-us/unreal-engine/sky-atmosphere-component-in-unreal-engine)
describes directional sunlight, atmospheric scattering and aerial perspective.
Genesis borrows the visual principle of a shared sky and atmospheric depth. Our
implementation is an art-directed gradient and an analytic exponential-height
haze term, not Unreal's physical atmosphere implementation. Sky, water reflection
and aerial colour share parameters. Haze is disabled during scientific surveys.

Epic's [cloud rendering documentation](https://dev.epicgames.com/documentation/en-us/unreal-engine/volumetric-cloud-component-in-unreal-engine)
connects cloud appearance, ground shadows and reflections, and describes quality
and tracing-cost tradeoffs. Genesis uses that shared-lighting principle with the
small two-dimensional field above; it does not implement Unreal's volumetric
cloud rendering or its Beer shadow-map pipeline.

Epic's [visibility documentation](https://dev.epicgames.com/documentation/en-us/unreal-engine/visibility-and-occlusion-culling-in-unreal-engine)
describes distance, frustum and occlusion rejection. Genesis now rejects spatial
clusters before individual scenery, then prepares separate camera and sun lists.
We have not implemented occlusion queries or a hierarchical depth buffer.

Epic's [Nanite content guidance](https://dev.epicgames.com/documentation/unreal-engine/working-with-naniteenabled-content)
discusses the difficulty of reducing disconnected aggregate geometry such as
foliage and the cost of overdraw. Our response is to preserve crown area and gaps
across detail levels and bound visible work. Our meshes and instances are ordinary
rasterized geometry; this is not a Nanite implementation.

[Lumen](https://dev.epicgames.com/documentation/en-us/unreal-engine/lumen-global-illumination-and-reflections-in-unreal-engine)
is a reference for dynamic indirect lighting and reflections. Genesis currently
uses direct sun shadows, approximate ambient fill and sky-only water reflection.
Global illumination remains future work; assets, materials and measured render
budgets need to mature before choosing a technique.

## Implemented pass — October 11, 2026

- 4× MSAA world rendering, resolved before glass and native-resolution UI.
- Derivative-smoothed foliage cutouts, complementary dithered detail transitions,
  and range fading without changing an object's physical size.
- 160 m scenery clusters with conservative bounds, followed by per-object tests.
  Camera and light use their respective clip volumes. Infinite reverse-depth
  camera projection retains distant geometry; the sun volume has bounded depth.
- Separate sun instance buffers with stable caster meshes. Objects outside the
  camera view can still cast shadows. The two 2048² maps are snapped in light
  space; sampling exits early outside their supported regions.
- A directional sky drawn at reverse-depth zero after opaque geometry, letting
  depth rejection avoid shading covered pixels. Clouds add one small density
  texture; no ray-marching pass or fog framebuffer is allocated.
- A 368-byte typed camera/atmosphere uniform replaces the per-frame heap-built
  float vector. All atmosphere colours are linear-light values.

## Earlier CPU visibility measurement (single shadow map)

Apple M4, optimized development/test profile, default generated Levant coast seed,
217,180 cached scenery objects. Each timing averages 30 calls; generation and GPU
rendering are excluded. The reference scans every object through the same tests.
Both paths produce identical draw lists. These are local observations, not a
comparison against Unreal or a claim about total frame rate.

| Camera | Clustered ms | Full scan ms | Objects tested | Main instances | Sun instances |
| --- | ---: | ---: | ---: | ---: | ---: |
| Settlement | 0.052 | 1.089 | 3,769 | 1,259 | 1,142 |
| Ground | 0.102 | 1.055 | 9,442 | 2,831 | 1,142 |
| Region | 0.588 | 1.099 | 114,034 | 10,384 | 1,142 |

Run `mise exec -- cargo test -p genesis-app visibility_timing -- --ignored --nocapture`.
Regular tests compare clustered and brute-force lists across aspect ratios and
headings, retain off-screen casters, check projection bounds, and preserve LOD
coverage. Native Metal captures validate atmosphere, surveys and surface resizing.

## Remaining limits and priorities

Scenery assets remain procedural stand-ins. Species-specific assets, richer ground
materials and believable river banks are necessary for realism. Atmospheric defaults
are a regional Earth-like look; cloud appearance is cosmetic, and no weather, planetary curvature or calibrated
scattering coefficients are simulated by this renderer.

World geometry now uses resident spatial buffers and separate camera/sun chunk
culling, as described below. CPU topology and colour generation still rebuild the
regional mesh, and neither world geometry nor scenery is streamed from storage.
Trees behind terrain are not occlusion
culled. Reduced-detail shadows can differ from close foliage silhouettes; terrain
beyond the wider map remains unshadowed. Dither fades can show stippling, and MSAA
adds memory and bandwidth cost without solving all temporal aliasing.

Next steps should pair authored asset/material work with GPU pass timings and
local CPU mesh invalidation. Evaluate improved shadow silhouettes, richer material
lighting and indirect illumination against measured cost and scene needs. Avoid making
a general-purpose renderer or cloning Unreal subsystems without a playable use.

## Resident world geometry and surface materials

World triangles now live in spatial GPU batches. Fine geometry uses 320 m cells;
coarser context geometry uses 1.28 km or 2.56 km cells based on triangle extent,
avoiding thousands of tiny horizon draws. A triangle belongs to one cell but its
complete geometric bounds drive visibility, so coast triangles crossing cells are
not clipped. This changes batching, not the terrain's shared-edge topology.

Content fingerprints skip GPU writes for unchanged chunks. Changed chunks reuse
their allocation where capacity permits; chunks absent from the new scene are
removed. Fingerprints are transient rendering cache keys, not persisted content
identities. Water follows opaque geometry in each buffer and is excluded from
shadow draws. Camera and sun use their own bounds tests.

Exposed-rock coverage travels in the fractional part of the terrain material tag,
retaining 28-byte vertices. Soil has dry patches, grain and pale fragments; exposed
rock blends three projections for grain, shallow fractures and weathered bands.
Detail fades with the full 3D pixel footprint, including on steep faces. Geological
base colours still come from the simulation; these patterns are cosmetic and do
not alter surface height, collisions, or mineral deposits. Survey materials bypass
this surface treatment.

The ground palette now uses living plant cover, moisture and summer drought, with
soil tinted by local rock. The horizon uses the same colour function with bounded
environmental fields clamped to the region edge; off-map ecology remains an
extrapolation. This avoids a separate bright-green context around drier active
terrain. Horizon colours are cached with geometry, so ecological changes there
become visible on the next terrain-cache rebuild.

Soil now uses softer, several-metre patches, with a slow domain warp and a weaker
rotated detail band. The former high-contrast half-metre pattern made open ground
look like camouflage from the settlement camera. The new mask fades toward its
mean coverage as it becomes subpixel, while reduced fine-grain contrast keeps
paths, buildings and shadows easier to distinguish. At eye height, finer grain and
sparse 2–6 cm mineral chips remain visible. A shared analytic noise-gradient
helper supplies subtle bump lighting for soil and retains the existing water
slopes. Soil applies a bounded lighting ratio to avoid lighting the baked terrain
twice. Chips use contained jittered ellipses in 18 cm cells, without a neighbour
search. They are shaded marks, not raised or collectible stones.

Each frequency fades by the full 3D pixel footprint; the grain, bump and chip branch
is skipped beyond a 4.5 cm footprint. Unresolved fine soil, patch breakup and broad
variation now also skip their noise evaluations. The additional close patch band
adds one noise sample when visible; this is not a demonstrated frame-time gain.
Broad variation fades at regional scales.
This retains 28-byte vertices and the same textures, buffers, passes and draw count,
but adds close-view fragment arithmetic; GPU frame-time impact is not yet measured.
Scientific overlays still bypass cosmetic detail. Native captures ignore live
keyboard/mouse events, while scripted smoke interactions remain enabled.

The softer-patch pass was inspected in matched native settlement captures and at
human height, plus a highlands regional capture. The settlement fixture retained
the same 75,101,796 world vertex bytes and 924 chunks before and after the shader
change. This is a material refinement, not added terrain displacement or a new
ecological distribution model.

Default Levant coast, camera at eye height near the first dwelling, Apple M4:

- Prepared scene: 3,168,378 vertices, 88,714,584 bytes, 924 resident chunks.
- Camera submission: 713,694 vertices (22.5% of the previous full submission).
- Sun submission: 773,706 vertices (24.4%, excluding water).
- After a 20 m camera move: 42 chunks changed, requiring 33,849,564 bytes instead
  of 88,580,520 bytes for the whole rebuilt mesh (38.2%).
- CPU partitioning took 46.19 ms in one optimized-development run. It runs on
  scene replacement, not each rendered frame. CPU reconstruction, colour updates
  and staging allocations remain costs; this is not an end-to-end frame benchmark.

Reproduce with `mise exec -- cargo test -p genesis-app world_chunk_timing -- --ignored --nocapture`.
Native buffer/pipeline regression:
`mise exec -- cargo test -p genesis-app resident_world_buffers_reuse_update_and_remove -- --ignored --nocapture`.
The latter needs GPU access and checks zero uploads on unchanged input, selective
edits, removal of stale chunks and an empty replacement, along with shader creation.
Capture commands report actual upload and visible-batch counts.

## Local riverbeds and water

Generation recipe 8 adds channel incision to the canonical simulation surface.
A sparse, serialized spatial index restricts each query to nearby river segments.
Cross sections have a submerged bed and smooth banks; bed and water elevations
follow edits to the underlying regional field. Incision fades into the seabed and
at regional boundaries so the outer terrain context retains its shared heights.
This is a geometric recipe, not a running hydrological or sediment model. In
particular, the smoothed longitudinal profile can contain uphill reaches and
confluences still need a network-wide water-level solution.

Near rivers use a separate surface, level across each section, clipped against the
actual adaptive terrain triangles, including transition fans. Depth then drives shallow
bed colour. Vertex RGB carries a negative bed-material tag (limestone, sandstone
or basalt) and longitudinal/transverse metres without increasing the vertex format.
Transverse distance includes the bank margins, so ripple scale stays consistent
across the full clipped ribbon. Damp bank colour comes from channel proximity.
Gravel under water is a material approximation, not refraction through a transparent pass.

`water.wgsl` layers two irregular ripple fields in channel coordinates, drifting
downstream. Analytic noise gradients supply surface slopes, with a rotated second
layer to break up the first layer's lattice. Screen derivatives convert slopes to
world space and filter each layer by its projected footprint. Compressed mappings
at bends have bounded gain. The geometric water-plane normal supplies the base
slope; ripples diminish at shallow contacts.
Bed visibility depends on depth and viewing angle, while reflections fade at the
bank contact instead of leaving a luminous outline. Ocean swell uses world-space
coordinates and a broken shoreline wash; streams have no ocean foam curtain.
These changes add no textures, render passes or vertex attributes. They are
procedural shading, not hydraulic flow, scene refraction or object reflections.

Rendering retains 1.25 m or finer river-cell spacing within 240 m, while the close
canonical mesh remains 0.625 m. Over 160–220 m the water smoothly approaches a
surface-conforming representation, and longitudinal samples spread from 1.5 m to
8 m. The distant surface retains a 4 cm display offset over simplified terrain,
but its optical depth now blends toward the shared simulation channel cross
section. Previously the display offset also supplied shader depth, turning wide
channels into uniform grey films. Near views still encode the real separation
between water and visible bed. Geometry is clipped using that separation before
the distant optical proxy is applied; the proxy is not used for collision or
survey readings. Shallow river edges resolve over the bed using existing 4× MSAA
coverage over the first 8 cm of depth, without an extra transparency pass.

Local incision, micro-height detail, normals and bank colour use the same
distance filter; otherwise fine gradients sampled at coarse vertices produce
serrated stripes across the regional view. The physical surface is unchanged by
this visual filtering. This avoids refining every watercourse in the region. A fully refined initial
attempt exceeded the 128 MiB scene budget; local refinement preserves the existing
budget checks for ground views in all three default regions. CPU region rebuilding
and LOD topology replacement remain limitations.

The apparent diagonal seam in the default highlands regional capture was traced
to a generated, straight river reach, not a crack in the terrain. Preserving optical
depth corrects its flat grey appearance. In recipe 8, the D8 drainage route itself
remained straight. The same recipe-8 regional capture reduced uploaded world vertex
bytes from 53,150,832 to 46,280,892 (about 6.55 MiB) by clipping to actual receiver
triangles instead of uniformly subdividing each river strip. This is one fixture,
not a frame-time claim. Native highlands and ground-level river captures check the
result. Tests cover positive finite triangles, complete projected area, interior
height agreement at two terrain detail levels, and retained centre/edge depth in
the distant approximation.

Recipe 9 changes the highlands relief before drainage: seeded valley bends at two
scales plus a smooth offset, a rounded floor, and smaller undulations influencing
tributaries. The valley retains a regional downstream fall; routing fills local
depressions and erosion cuts the resulting catchments. The same relief function
supplies the outer terrain context. This is a procedural landscape heuristic,
not sediment-driven meander evolution. Coastal and island relief are unchanged.
New highlands generations differ from recipe 8 for the same seed.

For default seed 7319, the longest straight reach across all river widths decreased
from approximately 1,093 m to 735 m; restricting the new measurement to channels at
least 4 m wide gives 505 m. The regression test limits that default-fixture measure
to 600 m, without imposing it as a hydrological rule on every seed. Seeds 0, 42 and
7319 also check acyclic downhill grid drainage, rainfall conservation, bounded river
widths and valid settlements; the default checks exact repeated generation.
Native regional and 1.7 m riverbank captures validate the new terrain on Metal.
Straight tributaries remain visible. The bank capture also exposes overlapping
water surfaces and shading discontinuities at a confluence; a consistent network
water-level and junction-mesh solution remains needed. The new highlands regional
capture uploaded 47,366,844 bytes of world geometry; this is a different landscape
from the recipe-8 capture and is not a like-for-like performance comparison.

`--river-capture output.png` selects a wide inland reach for native visual review.
Regression tests cover bounded incision, smooth cross sections, indexed-query
agreement, terrain edits, water clipping and depth agreement with the close mesh.
The batching measurements above predate recipe 8 and are retained as that pass's
baseline, not measurements of the new river meshes.

## Close ground cover

Ground cover now uses five shared meshes: 90-vertex curved grass tufts,
189-vertex rush-like clumps, 72-vertex rounded pebbles, 144-vertex curled fallen
leaves and 72-vertex scattered needles. Geometry supplies actual
normals; blades have gentle root-anchored wind, and pebbles use local limestone,
sandstone or basalt tint. Placement is cosmetic and seeded by world position.
Bank plants use channel height and climate proxies, while clustered dry-ground
coverage follows the existing vegetation field. Neither represents a calibrated
species distribution or simulated sediment deposition.

Candidates occupy 0.8 m jittered cells in a 54 m radius around the cached observer,
with fewer than 19,000 accepted instances across all five batches. Water,
construction and steep slopes reject placements; pebble bases are partly buried
on gentle slopes. Meshes and 40-byte instances are reused through the existing
scenery renderer, with at most five ground-cover draws per pass.

Materials 10 (blades), 11 (pebbles) and 12 (litter) use a root-distance coverage fade over
28–38 m from the current eye position. The 16 m cache margin covers the diagonal
offset within a 20 m camera-key bucket. Camera and shadow passes share coverage;
placement buffers need no per-frame update. Coverage dithering and very thin
blades can still alias, and fast movement can outrun an unfinished terrain build.
This budget is separate from the terrain/water vertex-memory budget.

A local 8 m crown index weights broadleaf litter, pine needles and grass suppression.
The index scans persistent placements once per cached patch, retaining only nearby
trees, then queries neighbouring bins per candidate. No leaf fall, decomposition
or soil feedback is simulated. Removing trees removes their litter on rebuilding.
Grass tint responds to moisture, drought and patch variation; pebbles also occur
on gentle ground away from channels.

Material 12 reuses the instance coverage pair for ground slopes (dh/dx, dh/dz).
The vertex shader shears litter onto that local tangent plane, transforms normals
with the inverse transpose, and restores [0, 1] coverage before the distance fade
in both camera and shadow passes. This retains the 40-byte instance format. It is
a local plane approximation; individual leaf vertices do not sample terrain.
`--understorey-capture output.png --tree broadleaf|pine|riparian` provides a native
eye-height view looking down beneath an existing crown. Ground target aiming now
updates pitch as well as heading, with a projected-target regression test.

Native review: `--river-ground-capture output.png` places the 1.7 m inspection
camera on an accessible bank. Tests check repeatability, overlap between shifted
patches, construction/water exclusion, bounded instances and finite metric meshes.

River ripple phase uses interpolated longitudinal distance rather than a dot
product of world position with changing tangents. This removes the origin-dependent
phase jumps that produced dense striping at bends. Procedural material noise also
uses an integer hash instead of a large-coordinate floating-point sine hash.

## Tree canopy structure

Tree meshes now use several tapered trunk segments with a buried base flare.
Pines carry four asymmetric limbs around each of six tiers, with bent branch tips
and a small leading crown. Broadleaf trees fork lower into seven scaffold limbs,
each with three smaller crowns; three upright leaders fill the canopy centre.
Riparian forms keep a taller stem, narrower crowns and one upper leader. These
are functional growth forms for presentation, not species models or simulated growth.

Leaf sprays use mixed inclinations and rotations instead of almost uniformly
vertical cards. A tree crown uses 90/24/5 sprays across its three detail levels;
spray size compensates to preserve total card area. This does not guarantee equal
projected opacity: overlap and different sample positions still change with LOD.
Existing crossfades, wind, instancing and independent shadow-caster lists apply.
No new textures, materials, instance batches or render passes are introduced.

Tests cover all variants and detail levels, finite geometry, conservative tree
culling bounds including a wind margin, less than 1% change in total foliage-card
area across LODs, and fewer than 18,000 close / 1,500 distant vertices per tree.
These are mesh budgets, not GPU timing or overdraw measurements. Leaf cutouts and
shadow silhouettes still need richer authored assets and temporal filtering.

For repeatable native inspection, add `--tree broadleaf|pine|riparian` to either
`--ecology-capture output.png` or `--ecology-ground-capture output.png`. The selector
uses an existing matching tree near the settlement and does not alter generation.
