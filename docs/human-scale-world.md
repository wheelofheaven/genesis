# A world that holds up at human eye height

Accepted direction: October 11, 2026. A settlement and the ground around it must
remain convincing when viewed from an individual's perspective. City-building
and future individual play share physical places; they do not swap to a separate
scene with different terrain or dimensions.

## Spatial responsibilities

| Layer | Responsibility | Working resolution / policy |
| --- | --- | --- |
| Planet / region source | Place identity, large landforms, bathymetry and imported measurements | Source resolution, explicitly recorded; local metric coordinates |
| Environmental field | Climate, fertility, drainage and other aggregate processes | Current 20 m samples; never the maximum geometric detail |
| Physical surface | Ground heights, material regions, feature geometry and persistent edits | Lazy 0.625 m canonical ground lattice in this iteration |
| Objects | Buildings, tree trunks, rocks, walls, bridges and future interiors | Independent meshes and colliders, with dimensions in metres |
| Presentation | Visible terrain, plants, small stones and material grain | Adaptive mesh density and cosmetic instances; no authority over physical state |

The lattice is an implementation of the current ground provider, not a universal
voxel size. Doors, stairs, walls and bridges must not be quantized to it. Caves,
overhangs, roofs and multiple walkable storeys need separate geometry and collision
surfaces; a single height field cannot represent them.

## Implemented foundation

`sim_core::surface` defines stable ground independently of the camera. It combines
regional relief with seeded, bounded small undulations and interpolates a fixed
triangle lattice. Detail is tied to world coordinates and remains unchanged by
ecology ticks and renderer LOD changes. It fades out under existing foundations
and paths and at regional boundaries and shorelines. This suppression is not
earthmoving or construction grading.

Building validation, object attachment and ground inspection query that surface.
The renderer refines dyadic terrain patches down to 0.625 m near the observer and
uses coarser patches farther away. Coarse edges incorporate all adjacent fine
vertices, making a connected mesh without skirts. Picking uses the currently
displayed mesh; in the finest area it matches the canonical ground triangles.
Far meshes approximate the surface and never redefine the ground used for movement.

Camera movement requests new topology in 20 m horizontal / 40 m vertical buckets.
Recipe 8 incises local river cross sections in the canonical surface. Sparse
segment bins bound queries to nearby routes; terrain, object attachment and
walking share the resulting bed. The bed is typically 0.25–1.2 m below a separate
water surface, with 2 m bank transitions. Water follows the smoothed valley profile,
level across each cross section. Near water geometry is clipped to the visible bed.
River cells retain at most 1.25 m spacing within 240 m of the observer; the closest
terrain still reaches 0.625 m. Water gradually conforms to coarse terrain over
160–220 m and reduces longitudinal tessellation. This is a visual LOD approximation,
not a second physical surface.
Local height detail and bank shading fade over the same range to prevent narrow
features from becoming exaggerated spikes in the coarse regional mesh.
One background build may run at a time. The previous terrain stays visible until
the replacement is ready; world-revision and camera-key checks reject obsolete
results. Initial/world-edit builds and entering ground view before fine terrain is ready currently remain synchronous. Geometry changes
invalidate the picker, and building placement invalidates ground detail. The
regional environmental grid is not enlarged.

Before the channel-incision pass, measured Apple M4 default-seed ground views used
approximately 78–114 MiB for terrain plus river vertices and took 0.4–0.8 seconds
to build, including mesh coloring. These are CPU mesh measurements, not total
process/GPU memory or a frame-rate guarantee. Tests enforce a 128 MiB scene-mesh
budget for the three default regions. Upload and picker construction still occur
on the main thread when a completed mesh is adopted.

Ground detail uses five small shared meshes: curved grass tufts, rush-like bank
plants, rounded pebbles, fallen leaves and needles. Nearby crowns select litter
and suppress some grass. Deterministic patches respond to vegetation coverage,
channel proximity, ground slope, climate proxies and local rock colour. Roots stay
above water; construction clears the placements. These are cosmetic instances,
not simulated plants, sediment deposits or collision obstacles. Larger trees and
rocks retain their persistent placements.

The ground itself now combines climate/cover/geology colour with metre-scale earth
patches, centimetre-scale mineral chips and fine grain/bump shading. Detail is
anchored in world metres and filtered by pixel footprint. These shader features
add no physical height or extra terrain vertices; stones that need silhouettes
remain geometry. The distant context shares the palette using edge-clamped
environmental samples, rather than its own fixed green colour.

Instances are prepared within 54 m of the cached camera position, including a
16 m margin around the visible radius. The shader fades them over 28–38 m using
the current camera position, also in the shadow pass. Small movements therefore
update coverage without waiting for a terrain rebuild or reseeding placements.
Fast travel can still outrun an asynchronous terrain build.

Press **G** or the person icon to enter ground inspection near the selected place:
1.7 m eye height, 1.4 m/s movement, 4 m/s with Shift, WASD/arrows and right-drag look.
G or Escape restores the prior overhead camera. Traversal checks the physical
surface, map boundary, slopes, watercourse exclusion and simple solid footprints
for buildings, trees and rocks. This is an inspection tool, not a simulated person,
character animation system, doorway/interior controller or new gameplay mode.

## Further implementation, in order

1. **Physical feature geometry.** Channels now have incised beds and banks, but
   longitudinal water levels still follow the valley profile rather than a
   hydraulic solution. Resolve confluences, longitudinal flow, sediment and
   terraces; add road sections and construction grading. Distant draped water
   remains a visual approximation. Noise cannot substitute for these features.
2. **Persistent local edits.** Store sparse, metric edit patches and semantic
   construction operations in simulation state. Resolve their order deterministically.
   Invalidate affected terrain, collision and navigation chunks with a halo, not
   every region. Connect excavation and grading to material accounting later.
3. **Streaming and transition quality.** Keep chunk geometry and buffers resident,
   replace only changed chunks, prefetch ahead of movement and morph LOD changes.
   GPU buffers now remain resident in spatial chunks; unchanged chunks skip upload
   and camera/sun passes cull them independently. Current background builds still
   rebuild the regional CPU mesh; they are not a completed planetary streaming
   system. Rapid teleportation
   and moving while a build is pending need explicit readiness/error budgets.
4. **Ground-level assets and lighting.** Replace prototype tree crowns and house
   forms with assets that work from the side and below. Add contact/cast shadows,
   more ground materials, shrubs, stones and ecological distribution. Extra
   polygons alone do not make a human view convincing.
5. **Individual locomotion.** Reuse the ground provider with character-sized
   collision, door and stair traversal, bridges, navigation, animation and agent
   identity. Do not tie simulation activity to whether the camera visits a place.
6. **Planet coordinates.** Keep planet locations in double precision (or integer
   sector coordinates), with renderer-relative local positions. Introduce that
   boundary when loading adjacent regions; extending the current regional f32
   coordinates to an entire planet would lose local precision.

## Acceptance and budgets

- Judge the same settlement from regional view, overhead close-up and 1.7 m eye
  height. Retain shared metric dimensions at all three distances.
- Test mesh area, winding and two-sided edge ownership at LOD transitions; test
  deterministic bounded detail, physical placement, traversal and restored camera
  state. Picking and feet must agree with the finest visible terrain.
- Budget memory and work by resident/viewed terrain, not total planetary area.
  A full 0.625 m lattice over this region would contain roughly 126 million cells;
  do not allocate it. Keep local vegetation bounded as the observer moves.
- Measure build/upload time, frame time and memory on the target machine before
  claiming smooth streaming. Seam correctness does not imply invisible temporal
  transitions. Full first-person visual quality is still an asset, lighting and
  physical-feature task as well as a terrain-resolution task.

For future terrain streaming and transition work, a primary reference is
[Asirvatham and Hoppe, Terrain Rendering Using GPU-Based Geometry Clipmaps](https://developer.nvidia.com/gpugems/gpugems2/part-i-geometric-complexity/chapter-2-terrain-rendering-using-gpu-based-geometry).
It describes viewer-centred nested grids and blended transitions. The current
implementation uses adaptive CPU patches and edge stitching; it is not that
paper's GPU clipmap implementation.
