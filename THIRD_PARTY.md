# Third-party code

## Khronos PBR Neutral tone mapper

`neutral_tonemap` in `crates/renderer/src/tonemap.wgsl` adapts
[pbrNeutral.glsl](https://github.com/KhronosGroup/ToneMapping/blob/main/PBR_Neutral/pbrNeutral.glsl)
from KhronosGroup/ToneMapping, retrieved October 11, 2026.

Copyright 2024 The Khronos Group, Inc. Licensed under Apache-2.0; the full
[licence](licenses/KhronosToneMapping-Apache-2.0.txt) is included. The source's
[REUSE metadata](https://github.com/KhronosGroup/ToneMapping/blob/main/.reuse/dep5)
identifies its copyright and licence.

Changes: GLSL translated to WGSL, negative input clamped, names adapted and the
function integrated with a fixed-exposure display pass and a survey bypass.
`tonemap.wgsl` is distributed under Apache-2.0. The renderer crate's manifest
records this alongside the project's CC0-1.0 code. Other dependencies and existing
assets retain their respective licences.
