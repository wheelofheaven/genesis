# Earth terrain foundation

Earth is the first destination in a simulation intended to support other planets.
Earth scenarios target selectable places and periods from the past to the present. Terrain is one signed
elevation surface: land continues through the shoreline onto the continental
shelf, slope, and ocean floor. Water occupies space above that surface. An island
mask is a scenario choice, not a planetary assumption.

## Implemented

Generation recipe 7 offers procedural Levant coastal plain and inland hill
windows, plus the existing Mediterranean island. The local simulation covers
7.68 × 6.4 km with 20 m environmental samples. The shared physical ground is
queried at sub-metre resolution and rendered adaptively; see
[Human-scale world](human-scale-world.md). Buildings and terrain share metric dimensions.
Continental fields clamp boundary samples instead of wrapping east onto west.
Drainage exits at sea or the local boundary; upstream inflow from neighbouring
windows is not modelled. Coarser surrounding geometry provides visual continuity
but remains outside the selectable, simulated region.

`worldgen::Region` stores place identity and geographic anchors. Positions map
into an east/south local tangent frame. The anchors are scenario metadata;
current elevation, geology, resources, and vegetation are generated, not measured
at those coordinates. They must be presented as procedural previews.

The seafloor survey hides the water surface, exposes the actual bed, adds 25 m
depth contours, and picks terrain beneath the hidden sea. Ground and sea depth
readouts use metres; aquifer volumes and simulation time remain prototype units.

## Route to reconstructed places

1. Import a signed topography/bathymetry raster with declared horizontal and
   vertical reference, source, version, resolution, licence, and missing-data mask.
   Keep measured relief distinct from procedural small-scale detail. NOAA's
   [ETOPO Global Relief Model](https://www.ncei.noaa.gov/products/etopo-global-relief-model)
   combines land topography, bathymetry, and shoreline data. The
   [GEBCO grid](https://www.gebco.net/data-products/gridded-bathymetry-data)
   also provides global land and ocean elevations in metres at 15 arc-second
   spacing. Neither source alone resolves individual buildings or narrow streams.
2. Derive local windows from a shared geographic source, with halo samples,
   matching boundary heights, and catchment inflow/outflow. Current clamped
   boundaries and visual context are interim presentation, not seamless streaming.
3. Resolve reference datums and hydrology explicitly. Below-sea-level terrestrial
   basins cannot be flooded merely because their elevation is negative; ocean
   connectivity and separate lake levels must control water occupation. The current
   simple sea-level rule is suitable only for these coastal and upland previews.
4. Preserve measured low-frequency relief when adding plausible unresolved detail.
   Generation-time erosion must not silently reshape imported survey data.
5. Attach historical scenario layers separately: coastlines, river courses, lakes,
   land cover, climate, settlements, and culture. Modern relief is a starting point,
   not a reconstruction of the Levant in the tenth century BCE. Retain dates and
   uncertainty for each historical layer.

No global raster is bundled or downloaded by the game yet. This iteration
establishes place selection, regional boundaries, and inspectable underwater relief.
