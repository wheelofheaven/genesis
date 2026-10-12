# Dwelling fixture

Project-owned CC0-1.0 geometry and procedural texture source:
[`scripts/build_dwelling.py`](../../../scripts/build_dwelling.py). No external art.
This is an import/material integration fixture, not a claim of archaeological
reconstruction or finished production art.

Regenerate all three embedded GLBs with `python3 scripts/build_dwelling.py`.
Metres, Y up, front +Z; 6.5 × 4.8 m footprint, 3.3 m parapet height. Doors remain
closed; no playable interior. Texture density is one UV repeat per metre.
Plaster, stone, timber and earthen roof use separate material slots with base
colour, tangent-space normal, roughness/metallic and occlusion PNG maps. Base
colour contains surface variation, not baked directional illumination.

| LOD | Triangles | Visible detail |
| --- | ---: | --- |
| 0 | 6,168 | Bevelled timber, sills and parapets; staggered masonry courses |
| 1 | 432 | Recesses, lintels, simplified joinery and masonry plinth |
| 2 | 168 | Wall/roof silhouette, plinth and flush door/window panels |

Start the game normally; press **F6** to import the files from this directory
again. All three must validate before replacement. Failed CPU imports retain the
previous resident asset and print an explanatory error. Startup uses embedded
copies so launching elsewhere does not depend on the working directory. Rebuild
the executable to update those embedded defaults.

Foundation adjustment, selection and placement use the simulation's existing
footprint/proxy. Keep replacement art within the checked envelope and preserve
its openings. The reload is a synchronous development command; automatic watching,
asynchronous loading, multiple prefab identities and general collision import
remain future work. See [the asset workflow](../../../docs/assets.md).

The bevel helper is [`scripts/dwelling_mesh.py`](../../../scripts/dwelling_mesh.py).
Bevels stay inside each piece's original bounds. Surface masonry protrudes up to
3.1 cm beyond the structural footprint. All LODs retain the same plinth band;
close LOD adds staggered individual stones. Wood UVs follow the long axis of
boards, beams and lintels. In-game palette variation uses persistent building ID.


Local ambient visibility is authored by
[`scripts/dwelling_ao.py`](../../../scripts/dwelling_ao.py) into UV1 occlusion
atlases. Regeneration casts short rays against box proxies; no lighting bake runs
at game startup. UV0 still controls metre-scale tiled surface textures. Atlas
borders are extruded for the supported filtering footprint. Occlusion affects
sky fill, leaving direct sunlight and emissive independent.
