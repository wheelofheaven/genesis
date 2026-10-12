# Static asset and benchmark workflow

October 12, 2026: first imported dwelling integrated into the custom forward
renderer. This implements a bounded first part of roadmap steps 1 and 2; their
full acceptance gates remain open.

## Import contract

The renderer accepts a documented subset of
[glTF 2.0](https://registry.khronos.org/glTF/specs/2.0/glTF-2.0.html): a GLB with one
embedded binary buffer, an unambiguous default scene, static positive-determinant
node transforms, indexed triangles, POSITION/NORMAL/TANGENT/TEXCOORD_0, optional TEXCOORD_1 for
occlusion, explicit
opaque single-sided materials and embedded PNG textures. The import currently
converts metre coordinates to Genesis's 20 m render units. Hierarchy transforms
are baked once; simulation data and saves never contain renderer resources.

Supported material inputs: base colour/factor, normal/scale, metallic/roughness,
occlusion/strength and emissive/factor. Repeat wrapping with linear magnification
and trilinear minification is the supported sampler contract. Unsupported
extensions, morphs, skinning, animations, sparse attributes, UV sets beyond UV1 or UV1 on non-occlusion maps,
transparency and external dependencies fail explicitly. There is no automatic
material approximation or tangent generation. Export normals, tangents and UVs.

Imports validate buffer/accessor ranges, index ranges, finite bases, node depth,
transforms and resource budgets: 32 MiB GLB, 2048² per image, 64 MiB decoded maps,
32 materials, 1024 nodes/primitives and one million expanded vertices. This is
an internal art-development importer, not a complete glTF implementation.

Base/emissive textures use sRGB GPU formats; normal, roughness, metallic and AO
remain linear. Mip generation averages colour in linear light, renormalizes
normals and filters roughness in GGX alpha-squared space. Trilinear sampling and
8× anisotropy support oblique close-up surfaces. Base, normal, surface and emissive textures are currently RGBA8; occlusion
uses R8. Identical images with the same filtering role share GPU storage across
materials and LODs during an asset replacement. Compressed formats remain future
work. UV1 occlusion charts need three extruded border texels; their dedicated
isotropic sampler caps mip selection at level 1 to avoid neighbouring charts.

## Rendering and authoring loop

The [project-owned dwelling](../assets/models/dwelling/README.md) has 6,168/432/168
triangles across its three LODs. The new instanced indexed draw path runs beside
the procedural terrain/scenery path. Screen-space size selects visible LOD;
independent near/wide caster streams use LOD1/LOD2. Reusable CPU stream buffers
and byte comparison avoid unchanged placement uploads. LOD changes are discrete;
transition hysteresis/fades remain future work.

The fragment shader uses GGX direct sunlight, tangent-space normal maps, the
existing two sun shadow maps, wind-driven cloud transmission and aerial depth.
A cached 32×16 diffuse-irradiance convolution samples the same directional sky
radiance LUT as the atmosphere. Its 32 samples per texel run only on LUT refresh.
Downward lighting uses an approximate ground bounce. This is daytime lighting
with the existing fixed sun, not a new moving sun/weather simulation. Roughness-
filtered specular environment reflections and locally occluded sky probes are
still missing; the first fixture deliberately uses rough non-metallic surfaces.

Normal gameplay uses the imported shell above the existing terrain-adjusted
foundation. Scientific surveys and construction previews retain the simpler
procedural shell. Picking/collision remain separate simulation-aligned proxies;
the importer does not invent simulation or interaction entities from GLB nodes.
F6 reloads all three fixture files transactionally after CPU validation. It is a
manual synchronous development operation, not an automatic asset watcher.

## Repeatable benchmark

```sh
mise exec -- cargo build -p genesis-app
# Native GPU access required, without opening a game window:
target/debug/genesis-app --render-benchmark /tmp/genesis-benchmark.json
```

Writes JSON and four adjacent 1920×1080 PNG captures: settlement, approach,
eye-level and descent. The fixture fixes seed 7319, the six-house coastal
settlement, fair weather, 4× MSAA, 12 warm-up frames and 60 samples per phase.
The route descends continuously from about 100 m away to a 1.7 m eye-height view.
It uses the same world, scenery, material, shadow, sky and tone-mapping passes as
the application. Terrain/ground cover stay resident at near-house detail, so this
isolates renderer work rather than testing background terrain streaming.

The report includes OS/backend/GPU, dev/release build, scene size, preparation
cost, per-frame CPU preparation/encoding, serialized submit-and-wait duration,
main draws/triangles, placement uploads, and median/p95/p99 summaries. GPU timings
cover near shadows, wide shadows, world and sky only; missing samples stay null,
and summaries include valid sample counts. Peak resident memory is explicitly
unavailable. Scenery culling/uploads run on every sample, whereas the app skips
unchanged camera streams. The report excludes simulation, UI, presentation and
streaming, and **does not measure interactive FPS or certify dense-city speed**.

Initial Apple M4/Metal dev-build run, 1920×1080:

| View | CPU prepare/encode median | Serialized CPU + submit/wait median | p95 |
| --- | ---: | ---: | ---: |
| Settlement | 1.00 ms | 18.96 ms | 20.70 ms |
| Approach | 0.90 ms | 17.32 ms | 17.62 ms |
| Eye level | 0.75 ms | 14.63 ms | 14.77 ms |
| Descent | 0.84 ms | 17.43 ms | 18.62 ms |

Stationary dwelling streams uploaded zero bytes after warm-up; the descent's
maximum dwelling placement update was 240 bytes. Whole-scene main geometry was
about 1.35–1.44 million triangles. World-pass timestamps were valid for only
1–5 of 60 frames per phase, so they are unsuitable for a performance conclusion.
This run establishes a reproducible baseline, not achievement of the provisional
60 FPS target. Forest/river/dense-city fixtures, full frame timing, peak memory
and a measured streaming route are still required by roadmap step 1.

## Validation

CPU tests cover metre scale, winding, LOD reduction, malformed ranges, unsupported
materials, transformed tangent bases, colour-space mips and camera-route bounds.
The native imported-asset test verifies real rasterization, finite HDR output,
reload consistency, stationary upload reuse, camera culling, LOD reduction and
resize alongside the shared sky/cloud/stone regressions:

```sh
mise exec -- cargo test -p genesis-app on_native_gpu -- --ignored --test-threads=1
```

Use the regular `--house-capture file.png --weather fair|overcast`,
`--settlement-capture file.png` and `--interaction-test file.png` commands for
visual and gameplay integration checks. Detailed art, environment reflections,
better foundation blending and wider settlement variety remain follow-up work.

## Dwelling detail and ground contact refinement

The second asset pass adds centimetre-scale bevels to exposed timber, sills,
parapets and masonry. Timber UVs now follow each piece's long axis, so lintels
and door boards have consistent grain direction. The stone plinth wraps the
whole building with staggered courses and a warmer palette closer to the existing
foundation. The structural footprint remains 6.5 × 4.8 m; surface stones project
up to 3.1 cm beyond it, inside the checked cosmetic envelope. Low LODs retain a
solid plinth so the broad material layout survives simplification.

High/medium/far meshes now contain 6,168/432/168 triangles. The extra close detail
uses the same four material slots and instancing pipeline. Modest per-dwelling
tints come from persistent building identity and reuse existing instance data;
they do not create new materials or change with camera position.

A 0.7 m contact falloff follows each rotated foundation edge in the existing
quarter-metre ground-occlusion field. It is a bounded, soft presentation proxy,
combined with scenery by maximum instead of accumulating to black. Terrain edits,
building placement and region replacement rebuild it through the existing cache.
It adds no render pass, texture or per-pixel building loop. It does not implement
interior lighting or general global illumination; the separate offline local-AO
bake described below now handles architectural crevices.

Matching Apple M4/Metal dev benchmark: serialized median durations were
18.76/17.21/14.26/17.30 ms for settlement/approach/eye/descent (previously
18.96/17.32/14.63/17.43 ms). These single diagnostic runs do not demonstrate a
speedup or interactive FPS. Main geometry increased by 360 triangles in the
settlement view and 20,880 at eye level; stationary dwelling uploads remained
zero bytes. Native close-up review confirmed bevel highlights, correctly oriented
wood and masonry continuing around the side walls. CPU checks cover winding,
LOD budgets, palette bounds, rotated contact and contact removal; the native
asset regression also checks the tint's effect on rendered HDR pixels.


## Offline architectural ambient visibility

`scripts/dwelling_ao.py` bakes separate 1024²/512²/512² occlusion atlases for the three LODs while
regenerating the GLBs. It casts 24 cosine-weighted short rays per sample against
the authored box kit, with a 0.65 m reach and smooth distance falloff. Boxes
approximate bevelled parts; this is a bounded local visibility approximation,
not triangle-exact ray tracing or indirect-light transport. A 30% residual fill
keeps crevices from becoming black. No sun direction, colour or cloud state is
baked into the asset.

The importer validates UV1 when an occlusion material requests it, retains UV0
fallback for older assets, and rejects UV1 on the other material maps. The shader
samples occlusion at the separate coordinates and applies it only to diffuse sky
fill. Direct sun, its dynamic shadows and emissive remain independent. The five
existing material samples are unchanged in count; vertex records add eight bytes
for the UV pair. Geometry, material draw count and placement uploads are unchanged.

Atlas charts have extruded padding and a dedicated bounded filter. The three
shared R8 atlases occupy about 2 MiB including mip chains; temporary decoded CPU
images are released after upload. Identical tiled maps are shared across LODs,
so the bake does not multiply every texture by the material count. Baking happens
only in the Python authoring command. Startup and F6 load the pre-baked GLBs.

Check the offline ray calculation with
`python3 -m unittest discover -s scripts -p 'test_*.py'`. Rust tests cover the
UV-selection contract and retained metre/winding/LOD invariants. The native asset
regression compares enabled/disabled AO and verifies that it only reduces HDR
illumination, while reload, tint, culling and LOD tests continue exercising the
same draw path.

Native fair-light benchmark after this pass: serialized medians were
18.37/17.17/14.59/17.17 ms for settlement/approach/eye/descent. Main triangle
counts exactly match the preceding bevel/contact pass, and stationary dwelling
placement uploads remain zero. Total shared asset texture payload is 2,796,195
bytes (about 2.67 MiB, including all material maps and mips, excluding driver
allocation overhead). These single-run timings remain diagnostics, not evidence
of improved gameplay FPS. The eye-level capture shows subdued depth around
lintels, shutters, beam ends and plinth joints without darkening open wall faces.
