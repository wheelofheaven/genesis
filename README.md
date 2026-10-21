# Genesis

A life, society, world, and civilization simulator. You belong to an advanced
spacefaring civilization: cross the stars, observe another world, and guide a
local community. The first focus is a colony/city builder with individual lives
at its heart. Earth is the first destination; the architecture should support
future exoplanets and their own environments and societies.

Phantasy Star Online and Xenoblade Chronicles X inform the technological visual
direction. Local settlements retain architecture appropriate to their setting.

Read the [project vision](docs/vision.md), the
[custom Rust engine decision](docs/engine-decision.md), and the
[imported asset / renderer benchmark workflow](docs/assets.md).

**Current status:** an orbital title screen leads through destination selection,
regional scenarios, an expedition briefing, and a skippable settlement arrival.
Frosted instrument panels carry through menus, controls, and survey tools. Display
and navigation preferences persist between launches.
The custom application generates seeded coastal and inland regions with
irregular shores, mountain ridges, drainage networks, woodland patches, and exposed
rock. Six freely positioned dwellings seek suitable terrain near water. Terrain
inspection, terraforming, combined groundwater/mineral surveys, and working
boreholes are playable. New settlements have up to six named residents who walk
to a meeting place and return home. Click a resident to inspect their home,
activity, destination and any blocked route; F focuses the selection. **Follow
resident** keeps the camera on their journey while you orbit and zoom. Figures are
placeholders. Residents now collect water from working nearby wells and deliver it
to household stocks. They also gather finite wild food, carry it home in baskets,
and consume whole portions. **Supplies** now flags homes short of food or water for
their next consumption cycle. The panels show logical countdowns, missing amounts
and carried supplies, with short households first; click a row to focus its home.
See [supply outlook](docs/architecture/supply-outlook.md).
At a planned home departure, residents without enough food or water for the next
use stay home if no supply or tending job is available. They retry every two
logical seconds; the inspector shows the countdown and last search results.
Resuming a well or bringing a crop to maturity can get them working again.
See [household supply waiting](docs/architecture/supply-waiting.md).
Inspect a home or resident for stock and task status;
use Survey/Hydrology to drill and control wells. At a selected borehole, enable
**Fill buffer only** to preserve groundwater when its collectable store is full.
Residents can still collect; the pump replenishes the buffer as space opens.
Continuous pumping and the manual stop/resume control remain available.
See [buffer-controlled pumping](docs/architecture/buffer-pumping.md).
New settlements include a selectable
wild-food patch. Wild sources remain finite; **Build → Barley plot** adds a renewable crop.
Barley grows with soil moisture, pauses on dry or flooded ground, and becomes food
that residents harvest and deliver. Plots automatically resow after their harvest
is cleared. Select a plot and use **Apply 2 u water** to draw from a working well
within 30 m. Its four-unit store drains over time and supports growth on dry ground;
existing household water reservations are protected. Enable **Residents tend this plot**
for residents to fetch and carry two-unit doses after their own household supply jobs.
Plot and well reservations protect water in transit; stopping tending releases
uncollected doses while carried water still delivers. Seed costs remain unimplemented.
See [resident tending](docs/architecture/resident-tending.md) and
[irrigation](docs/architecture/irrigation.md).
Further needs, jobs, historical scenarios and the final art pipeline remain future
work. See [household water](docs/architecture/household-water.md) and
[food gathering](docs/architecture/food-gathering.md), plus
[cultivation](docs/architecture/cultivation.md).

## Run

Install [mise](https://mise.jdx.dev/), then:

```sh
mise install
mise run run
```

With the configured Rust toolchain active, `cargo run` also starts the game.
macOS is the primary development platform.

Choose **Begin expedition** from the orbital title screen:

1. **Destination:** Earth is the only currently supported planet.
2. **Region:** choose the Levant coast, inland hills, or a synthetic Mediterranean
   island. Current scenarios are undated procedural previews. Historical periods
   and exoplanets are not yet selectable.
3. **Briefing:** review environmental conditions, the starting site, and available
   interventions. Expand **Generation settings** to edit or shuffle the seed.
4. **Deploy:** generation runs in the background. Arrival focuses on the actual
   village and reports real dwelling, watercourse, and deposit counts. Enter the
   settlement, choose a regional overview, or press Escape to skip the arrival.

Settlement is the primary mode. The secondary **World Builder sandbox** link on
the main menu uses the same expedition flow with uninhabited terrain. These modes
provide different starting conditions and shared tools. Settlement residents
collect water and gather food for their homes; broader household relationships, production chains,
and expedition logistics remain future systems.

**Menu** (or **Escape** after closing a tool) suspends world time. Return to the
main menu and choose **Continue expedition** to resume the same live session.
Use **Save expedition** in the pause menu, or **F5**, to save the world.
**Load saved expedition** is available from both the title and pause menus; **F9**
also loads during play. Loading over an open world asks for confirmation and opens
the saved world paused. The quit dialog offers **Save and quit**; starting another
expedition offers **Save and deploy**. Failed saves keep the current world open.
See [menu saves](docs/architecture/menu-saves.md).

**Settings** provides fullscreen, interface scale, a 30/60/120 frame limit, camera
sensitivity, and reversed scroll zoom. Apply saves these preferences to
`~/Library/Application Support/Genesis/settings.conf` on macOS; Cancel leaves
settings unchanged. Audio settings will follow when audio exists.

Use **Region → Generate next seed** during play to explore another region, or
preselect a seed in the new-world menu from the command line:

```sh
mise exec -- cargo run -- --seed 7320
```

The default seed is `7319`. Generating or recreating a region replaces the current
world and its edits; the World panel makes this explicit.

## Controls

- **WASD / arrows:** move; hold **Shift** for faster travel.
- **Right drag:** orbit; **middle drag** or **Alt + left drag:** pan.
- **Scroll:** zoom toward the pointer; **F:** focus selection; **R:** reset camera.
- **Left click:** select a dwelling, path, or landscape position. Objects highlight
  under the custom expedition cursor; selection stays highlighted with an inspector.
- **P / Paths:** click waypoints for a rounded footpath; **Enter** builds the clicked
  route, **Backspace** undoes a waypoint, **Escape** cancels the draft. Width is
  adjustable from 1.2–4 m; endpoints snap to existing paths within 1.5 m.
- **B / Build dwelling:** preview follows the pointer; **Q / E** rotate by 15°;
  **click** to place or **Shift-click** to continue placing.
- Green previews indicate suitable sites; red footprints explain blocked placement.
- **Escape:** cancel the active tool or clear selection, then open/resume the pause menu.
- **Space:** pause/resume; **1 / 2 / 3:** select 1× / 4× / 16× speed.
- **Landscape:** open the contextual panel to raise/lower terrain or plant.
- **V / Survey:** toggle hydrology and geology independently, or combine them.
  Click ground to inspect it; drill a borehole or stop/resume an existing one.
- **World:** inspect the seed, generate the next region, or recreate this seed.
- Actions wait for the next simulation tick. **Step** advances one tick while paused.
- **H / Help:** show all controls. Buttons also have tooltips.
- The compass follows the camera; click it to face north. Adjacent buttons provide
  zoom, focus, and home controls.

The HUD uses light frosted glass panels, a bottom tool dock, and contextual
shortcut hints. Exploration keeps the center clear; building, survey, and landscape panels
appear only for their active tool. The minimum window is 760 × 540 logical pixels,
with compact controls and scrolling where needed.

Open **Supplies** beside the resident count and choose **Water** or **Food** to see household stocks, active
collectors, cargo and recorded shortage reasons. Click a home row to select and
focus it; scroll for more homes, wells and food patches. Totals separate
stock at home, in transit and at the source; reservations are part of source
stock. Water uses abstract units and food uses whole portions. Failed collection attempts remain labeled as the last attempt until a
resident tries again. Escape closes the overview.

Use **Inspect [name]** under a household to open its resident's activity and route.
The dwelling inspector also links to its residents, and each resident links back
to their home. **Follow resident** tracks their displayed position; **Stop
following**, panning, another selection, a tool, or F/R/G releases the camera.
Orbit and zoom remain available while following. Following is a viewing choice
and clears when a world is loaded or replaced.
See [resident following](docs/architecture/resident-following.md).

The clock uses fixed 100 ms ticks, retains fractional time across pause, and
limits catch-up to eight ticks per frame. A frame contributes at most 250 ms of
wall time before the speed multiplier; excess whole-tick backlog is discarded.
Under overload simulation slows relative to wall time without skipping simulation
steps. Hidden or zero-sized windows suspend advancement.

**F5** saves the current world and queued actions; **F9** loads it and pauses at
the saved tick. On macOS the slot is
`~/Library/Application Support/Genesis/world.json`, with the previous valid save
at `world.json.bak`. Loading resets camera/selection and preserves the active world
if validation fails. Saves are synchronous; the current main-menu Continue button
still resumes the live session; Load saved expedition restores the slot. New saves use schema v11 with expedition role, resident routines,
water/food stocks, crops, tending, pump policies and task commitments; v1–v9 worlds migrate on load.
Existing populations and water commitments are retained. Old worlds gain empty
food stores without a new food patch when needed. Existing food sources remain
intact; both old and new worlds can plant barley.
Historical well recovery does not become collectable water. See [identity and compatibility](docs/architecture/durable-identity.md)
and [crop saves](docs/architecture/cultivation.md), plus [save recovery](docs/architecture/saves-v1.md).

## Development

```sh
mise run test
mise run clippy
mise run fmt-check
mise run build
```

The CPU-only runner uses the same fixed-tick simulation as the application:

```sh
mise exec -- cargo run -p genesis-headless -- --scenario coast --seed 7319 --ticks 100
mise exec -- cargo run -p genesis-headless -- --scenario fixture --ticks 100 --commands crates/sim_core/fixtures/coastal.commands.json
```

See [headless execution](crates/headless/README.md) for the command format and scope,
and the [architecture assessment](docs/architecture/current-state.md) and
[refactor checkpoint](docs/architecture/refactor-log.md) for findings and validation.
The [architecture brief](docs/architecture/GENESIS_ENGINE_ARCHITECTURE.md) records the target direction.

For a native GPU smoke test that exercises camera movement, pause, queued actions,
stepping, free building placement, and resize before writing a PNG and exiting:

```sh
mise exec -- cargo run -- --smoke-test /tmp/genesis-foundation.png
```

`--menu-capture /tmp/genesis-title.png` captures the title menu.
`--save-test /tmp/genesis-save.png` also checks saved pending intent, reload and
identical continuation, using a temporary sibling `.world.json` instead of the
normal quicksave slot.
`--menu-test /tmp/genesis-menu.png` checks the destination/region/briefing flow,
small-window settings, asynchronous settlement creation, paused arrival, pause,
return/continue, menu saving, load confirmation and restoration from the title screen.
It captures each menu, the arrival, and the paused restored world, using a temporary
sibling save slot.

Other capture commands bypass the menu to preserve scene regression checks.
`--capture /tmp/genesis.png` captures the initial scene without the smoke actions.
`--survey-test /tmp/genesis-survey.png` also checks borehole drawdown and picking,
and captures hydrology, geology, and their combination (the separate layers use
`.water.png` and `.geology.png` suffixes). Capture modes need access to the native
window system and GPU.

`--world-test /tmp/genesis-world.png` exercises generated regions: seed replacement,
clearing stale actions, aquifer drilling, and combined surveys. It captures the
initial seed and survey as `.initial.png` and `.survey.png` alongside the final
region. `--seed` works with this mode and `--capture`; the older smoke modes use
the small authored regression fixture.

## Seeded world generation

### Working spatial scale

The ground now has a separate, deterministic **0.625 m local surface** with adaptive
rendering. The 20 m environmental field no longer caps visible geometric detail.
Press **G** or the person icon beside the compass for **ground inspection** at
1.7 m eye height. Move with WASD/arrows, hold Shift for speed, right-drag to look,
and press G/Escape to restore the overhead view. Select dry ground or a dwelling
before entering; the observer starts in a nearby clear spot. This inspects the
shared world; it does not yet play a simulated inhabitant.

`--ground-capture /tmp/genesis-ground.png` captures that perspective. See
[Human-scale world](docs/human-scale-world.md) for what is implemented, the budgets,
and the remaining channel, asset, lighting and streaming work.

Geometry uses **20 metres per world unit**. A dwelling occupies **6.5 × 4.8 m**,
with 2.8 m walls and a roof/parapet reaching 3.3 m. Paths are 1.2 m wide, entrance
walks 0.9 m, and generated homes cluster at roughly 10–20 m spacing. The same
dimensions drive meshes, rotated placement footprints, clearance, and previews.
Trees, boulders, and well equipment use compatible proportions. A 45° perspective
camera gives distant ground and objects their expected reduction in apparent size.
The scale bar measures horizontal distances at the camera focus; ground farther
away appears smaller. Zoom remains anchored at the pointer's picked elevation,
and the camera can move close enough to inspect individual homes.

Dwellings use an imported, project-owned GLB fixture with recessed plank doors,
shutters, timber lintels, stone sills and separate plaster/stone/wood/roof maps.
Three indexed LODs reduce the shell from 6,168 to 168 triangles. Shared sun/cloud
shadows and cached diffuse sky lighting shade the materials; the procedural
foundation still conforms to terrain. Picking, surveys and placement previews
retain separate simpler geometry. Doors remain closed, with no playable interiors.
`--house-capture /tmp/genesis-house.png` gives a repeatable eye-height view.
**F6** reloads the dwelling files during development; see the
[asset authoring and benchmark workflow](docs/assets.md).

The 384 × 320 region therefore spans **7.68 × 6.40 km**, sixteen times the previous
map area. This is a working geometry convention, not a claim of historical house accuracy or calibrated hydrology:
resource volumes, time, and existing survey readouts retain their prototype units.
`--settlement-capture /tmp/genesis-settlement.png` captures a close view for scale
review; `--landscape-capture /tmp/genesis-landscape.png` frames its surroundings.
Generation recipe 9 adds seeded highland valley bends, a rounded valley floor,
and smaller relief that influences tributary routes. It retains recipe 8's shared
physical riverbeds and banks and recipe 7's climate-aware vegetation, clustered
understorey and formation-specific stones.
The environmental field still uses 20 m samples. Trees and rocks share GPU meshes;
each placement uses 40 bytes, including detail-transition coverage, rather than
duplicating its geometry. River widths
follow local rainfall-weighted catchment area (bounded to 0.8–12 m), instead of
resetting a visual taper at each polyline. This is a visual heuristic, not a
discharge model.

Run `mise exec -- cargo run -p genesis-sim-core --example scale_audit` to report
regional dimensions, relief, vegetation counts, river widths, and dwellings for
all three default-seed regions.

### Generation recipe

The active region is a 384 × 320 environmental sampling field with continuous geometry
and free placement. A deterministic generation recipe combines warped multi-scale
noise, overlapping uplands, saddles, coastal inlets, a submerged shelf, slope
weathering, and sixteen passes of channel incision. Coastal terraces transition
into foothills and nested ridges. Depression filling gives each inland sample a
downhill outlet; rainfall
accumulation identifies tributaries and channels. Curved river geometry rounds the
internal routing directions. Continental regions have bounded field edges; only
the optional island preset retains a submerged wrapping rim.

The inland-hills valley now varies with the seed at several scales before drainage
and erosion are calculated. Its rounded floor replaces the sharp central crease;
small undulations break up uniform slopes. River corridors, banks, and catchments
therefore derive from the same relief. This reduces the default valley's long
straight reaches, although the eight-direction routing still leaves some visibly
straight tributaries. Coastal and island relief recipes are unchanged in version 9.

Rainfall and drainage inform moisture and vegetation; elevation and broad formation
fields inform host rock. Lowlands and drainage influence aquifer thickness and
storage. Copper/iron bodies favor selected host formations; clay favors low,
depositional terrain. Deposits have seeded, rotated, irregular outlines. These
are geological plausibility heuristics, not a tectonics or ore-genesis solver.

The settlement scores gentle terrain near water, validates every dwelling, and
connects short dry walks. River corridors reject new buildings. Vegetation forms
seeded patches and avoids the initial settlement, paths, and channels; its meshes
are placeholders, not individual simulated organisms.

The generator offers a Levant coastal plain, Levant inland hills, and a Mediterranean
island. These are procedural regional interpretations, not surveyed reconstructions
of those coordinates or historical coastlines. They use the metric geometry
convention above and a prescribed regional climate.
Rivers and their drainage fields are generation snapshots: later terraforming does
not yet reroute them, create lakes, or simulate flooding. Ongoing sediment transport,
seasons, vegetation succession, and groundwater/surface-water coupling remain future
systems. The same seed reproduces this recipe; changing its version may change maps.

### Earth regions and underwater relief

The default is now **Levant · coastal plain**. Open **World**, choose a region,
and select **Generate selected region**. **Generate next seed** also uses that
selection. CLI equivalents are `--region coast`, `--region highlands`, and
`--region island`; they combine with `--seed` and capture modes.

Continental regions continue beyond the visible simulation window. A cached,
coarse context mesh extends the same base relief around the playable patch;
that surrounding context is visual only, not streamed simulation. Climate,
interpolation, and brushes use bounded edges to avoid opposite-edge leakage.
Border drainage can leave the region; neighbouring catchments are not yet loaded.

Terrain includes the seabed, a shelf, a continental slope, and submarine channels.
**Survey → Seafloor** removes the water surface, exposes depth contours every
25 m, and allows selecting the actual bed. Elevation and water depth are shown
in metres. Focus and cursor zoom follow the exposed bed. Use
`--seafloor-capture /tmp/genesis-seafloor.png` to inspect this view.

Place metadata includes a geographic anchor and a local east/south coordinate
frame. Coordinates identify the scenario window; they do not make its generated
terrain measured Earth data. The roadmap for actual elevation and bathymetry,
dataset provenance, and historical scenarios is in [Earth terrain](docs/earth-terrain.md).

### Surface presentation and navigation

Water is shaded using shelf depth, shallow bed colour, sky reflection, subtle
animated waves, filtered sun highlights, and a moving shoreline wash. A refracted
ray samples a procedural bed, with colour-dependent absorption revealing warmer
gravel in shallow water. Sky reflections remain lit under direct sun shadows.
Short ripples fade independently with distance; ocean crests bend across a slowly
varying field. This is a single-pass optical approximation, without scene refraction
or reflections of nearby objects. Close river surfaces sit
above incised beds, intersect the banks, and encode actual visible depth. Damp
alluvial banks, geology-tinted shallows and irregular downstream ripples distinguish
streams from the sea. A shared slope field smooths reflections across bends and
confluences. World-anchored ripples drift along the local downhill slope, using
overlapping animation cycles to avoid phase resets at ribbon boundaries; they fade
below pixel resolution. Reflections soften at the bank contact. Beyond 160–220 m, water
gradually conforms to the coarser terrain while retaining channel depth for its
shading, rather than treating the display offset as shallow water. River footprints
are combined into a single surface on the actual terrain triangles, so branches
share rendered heights and materials instead of overlapping at junctions. A filtered
presentation level smooths those joins; the physical bed and drainage stay unchanged.
Shallow bank edges use the existing MSAA coverage. These are
presentation effects, not simulated waves or a river flow solver; water animation
continues while the simulation is paused. Screenshot captures use a fixed phase.
Ground uses filtered, world-space soil and gravel detail; trees use irregular,
rounded foliage instead of diamond silhouettes. This remains procedural prototype
art rather than scanned terrain or vegetation assets.
Use `--river-capture /tmp/genesis-river.png` to inspect an inland channel.

The camera frames the enlarged region and retains metre-scale close inspection.
Terrain positions and normals are cached until editing or regeneration; river
geometry and foliage are cached until terrain or buildings change. Spatial chunks accelerate picking.
Surface colours refresh once per second; active survey colours refresh up to four
times per second, with immediate refresh for player actions and layer changes.

## Surface interface

Glass panels use a light, mostly transparent tint, rounded edges, soft shadows, and a real
GPU-blurred world backdrop. Dark text and vector icons remain sharp. The renderer reuses
quarter-resolution horizontal/vertical blur targets and composites frosting only
inside the painted panel bounds, including clipped popups and survey labels.
GPU pipelines remain cached on resize. The world and blur render in linear light;
egui blends through a gamma-space texture view so translucent colors stay clear. Orbital menus share the translucent theme
over their illustrated background.

- Upper left: region identity and actual dwelling/borehole counts.
- Upper right: region tools, help, and the session menu.
- Bottom center: Explore, Build, Paths, Landscape, and Survey icons with short captions.
- Right inspector: the active tool, readings, and interventions; close with × or Esc.
- Lower left: compass, camera controls, and metric scale.
- Lower right: time controls; these move under the menu on narrower windows.

Utility, camera, and playback icons have descriptive tooltips and keyboard hints.
Small windows simplify navigation while preserving keyboard controls.
Long inspectors scroll. Survey legends live inside the inspector while tools are
open, and remain available as a separate card while exploring with overlays.
Notifications stay at the left so they do not cover the placement site.
Pause, settings, help, and confirmation surfaces use the same glass treatment.

The layout adapts the overview/action separation seen in
[Anno 1800's official UI study](https://www.anno-union.com/devblog-user-interface-2/)
and the edge-mounted status bars, bottom action strip, and time controls visible in
[this Manor Lords gameplay screenshot](https://www.sector.sk/files/recenzie/202441514461275/manor-lords-39305-4264490-2.jpg).
These are layout references; Genesis retains its spacefaring visual language.
Population, approval, food, and production summaries should appear only when the
simulation actually supports them.

## Planetary survey

Survey layers represent the Elohim's advanced instrumentation. Hydrology colors
show stored groundwater; geology shows copper, iron, and clay bodies. Combined
mode keeps the blue water field and colored mineral outlines distinct. The panel
reports host rock, water-table depth, storage/capacity, permeability, recharge,
and local deposit concentration/depth. Expand **Underground depth profile** for
a schematic column, or **Mapped deposits** to select a deposit directly.
**Signal settings** adjusts opacity or restores the surface view. Layers remain
visible when switching to building or landscape tools.

Boreholes withdraw finite groundwater every tick. Stop pumping to allow recharge;
recovered water and current yield appear when selecting the borehole. Sites are
validated again when queued actions execute, including clearance from dwellings
and other boreholes. Pumping stops while a site is submerged.

This is a synthetic scenario using game units, not historical geological data.
The first model has independent storage columns, prescribed rainfall-based recharge, and a
permeability-dependent extraction rate. Bilinear sampling provides continuous
readings within the surveyed patch. Aquifer and deposit elevations stay fixed
when the surface is terraformed. Lateral groundwater flow, confined pressure,
weather-driven recharge, excavation, and mineral extraction are not implemented.
Exposed formations do not yet discharge water.

## Architecture

- `crates/sim_core`: deterministic simulation, explicit actions/events, scenario fixtures;
  no rendering, windowing, or UI dependencies.
- `crates/renderer`: wgpu resources, procedural geometry, and camera navigation.
- `crates/app`: winit lifecycle, input, fixed clock, egui inspector, and a read-only
  adapter from simulation state to render geometry.
- `crates/headless`: CPU-only scenario/action playback and JSON metrics using the
  same `sim_core::runtime::Simulation` as the application.
- `crates/storage`: native save encoding, staged loads and recoverable file writes,
  shared by the game and headless host; no graphics dependencies.

The renderer can be reconstructed from simulation state. Simulation rules remain
in `sim_core`; the application schedules ticks and delivers ordered actions.
The main workspace lockfile is tracked for reproducible dependency resolution.

The [legacy Bevy prototype](legacy/README.md) is preserved in a separate workspace:
`mise run legacy-run`. The [engine lab](experiments/engine-lab/README.md) preserves
the earlier comparison fixture; it is not the main application.

## Organic spatial model

Environmental values live on an internal sampling grid with bounded continental
windows and optional legacy wrapping. A cubic B-spline surface interpolates those samples into continuous terrain, with
smooth shading and a shared water plane. The grid does not constrain building
positions or rotations. Picking intersects the rendered terrain triangles and
water surface; the selection marker is circular.

`sim_core` owns continuous positions, building IDs and footprints, placement
validation, and path centerlines. Generated walks connect nearby dwellings; short
entrance walks are presentation geometry. There is no road editor, general obstacle-aware path
routing, or pedestrian navigation yet. Terraforming still edits the underlying
environmental samples. Existing structures follow the terrain visually; structural
damage and flooding consequences are not modeled yet.

## Interaction performance

Placement previews use a separate GPU buffer; pointer movement and camera motion
do not rebuild the landscape. Terrain picking reuses cached triangles, invalidated
on terraforming. Ecology colors refresh at most four times per second; queued
world edits refresh immediately on their simulation tick. Rendering remains
independent of that cadence, with a requested frame latency of one and input-driven
redraws. Drag/scroll sensitivity accounts for Retina scaling.

A local Apple M4 development-profile timing run (100 identical rays) measured
2.309 ms per pick for the previous height-recomputing code and 0.061 ms for cached
triangles. This measures CPU picking only, not overall frame rate or input latency.
Reproduce with:

```sh
mise exec -- cargo test -p genesis-app pointer_timing -- --ignored --nocapture
```

Measure worker snapshot, terrain cache, picker, scene extraction and spatial
partitioning separately on the generated coast:

```sh
mise run build
target/debug/genesis-app --extraction-benchmark /private/tmp/genesis-extraction.json
```

This CPU preparation diagnostic opens no window or GPU. It runs two fixed camera
positions and surface/water surveys, with 2 warmups and 20 measured samples per
stage. It reports timings and presentation vertex fingerprints, not frame rate.
Cold-cache timings also separate topology, height/normal sampling, ground cover,
contacts, horizon and river geometry.
See the [measured extraction checkpoint](docs/architecture/extraction-performance.md)
and [adaptive terrain query checkpoint](docs/architecture/cache-performance.md)
for the workload, before/after evidence and limits.

Measure CPU preparation after a committed nine-sample terrain edit at regional,
management and inspection distances:

```sh
mise run build
target/debug/genesis-app --terrain-edit-benchmark /private/tmp/genesis-terrain-edit.json
```

The diagnostic queues the real action through `Session`, steps once, and times
cache, picker, scene and chunk preparation. It reports actual combined CPU timing
and estimated world-vertex uploads, with a rebuilt same-tick no-edit control to
separate terrain effects from ordinary environmental changes. No GPU/window is
opened; this is not input-to-display latency or an FPS measurement.
See [the edit-preparation baseline](docs/architecture/terrain-edit-performance.md).

## Next milestone

Deepen the existing six-resident settlement with watering labour and clearer
household consequences, and establish character loading and animation. Expand
into relationships and individual motivations incrementally.
The original biosphere score remains a prototype mechanic, not the overall goal.

## License

The adapted Khronos tone-mapping shader is Apache-2.0; see
[third-party notices](THIRD_PARTY.md). Other project code retains the licence below.

[CC0-1.0](LICENSE)

## Planet and scenario boundaries

The engine-independent `sim_core::catalog` stores stable destination/scenario IDs
and associates each implemented region with its planet. Undated previews have no
historical start year. Player mode and capabilities belong to the expedition;
selecting an unsupported destination cannot silently invoke an Earth recipe.
Earth climate/biology assumptions in the current simulation still need explicit
physical models before non-Earth scenarios can be supported.

The orbital globe is a cached illustration built from Natural Earth land outlines,
with decorative shading and atmospheric patterns. It is separate from simulation
terrain; see [asset provenance](assets/maps/README.md). There is no simulated
spaceflight, historical Earth reconstruction, or global terrain streaming yet.


### Objects and footpath construction

House picking intersects the rendered wall/roof geometry and respects terrain
occlusion, including in ground inspection. Paths have independent selections and
length/width readouts. Hover and selection use a small separate overlay mesh;
hovering does not upload the regional terrain again. The vector cursor scales
with the UI and shows the hovered object or active construction tool.

Footpaths are continuous, rounded centre lines, validated at half-metre intervals
across their width. The simulation rejects water, watercourses, steep routes,
building/borehole collisions, invalid coordinates, and duplicate routes. Each
construction is bounded to 64 waypoints and 500 m. New dwellings cannot cover
existing paths. Building a path clears trees/rocks in its corridor and suppresses
nearby grass; it does not yet grade terrain, build bridges, charge resources, or
assign construction labour. Actions remain queued while paused until Resume or
Step. This is a geometric path network; agent routing is still future work.

Path surfaces are clipped to the current terrain triangles and offset by only
1.2 cm to avoid depth fighting. Lit soil responds to geology and moisture, with
soft worn edges resolved by the existing MSAA pass. Paths receive shadows but do
not cast opaque ribbon shadows. Automatic entrance walks join the nearest point
on the existing route; these remain cosmetic links without obstacle-aware routing.
Vegetation clearances follow rotated house footprints and steps, including these
entrance walks. Ground cover accounts for blade spread and instance size; hidden
trees no longer leave cosmetic understorey behind. Nearby footprint queries use
an 8 m spatial index during cache preparation.
Path hover, selection and construction previews also follow the visible terrain
triangles, and refresh when terrain detail changes. Their solid feedback colours
remain clear above the soil surface.
Building placement outlines and ground markers use the same terrain attachment,
with closed corner joins and direct seabed attachment in seafloor survey mode.

`mise exec -- cargo run -- --interaction-test /tmp/genesis-interactions.png`
checks roof picking, path drafting, paused construction, and selecting the result
on the native GPU. It also writes `.house.png` and `.draft.png` captures.


### Ecosystems and scenery

Recipe 7 separates habitat rules in `sim_core::ecology` from disposable scenery
meshes. Scenario climate records prescribe temperature, moisture scaling, and
summer drought; altitude, slope, local moisture, river proximity and rock type
influence the vegetation mix. Earth coordinates are not required by these rules.
The current catalogue contains functional Mediterranean-inspired forms: broadleaf,
open pine, riparian broadleaf, evergreen shrub and dry scrub. These are procedural
proxies, not authenticated species or historical botanical reconstructions.

Stable placement seeds drive mesh variant, scale, rotation and tint independently
of vector order. Vegetation forms patches and understorey clusters; limestone,
sandstone and basalt use different rock geometry/colour and nearby fragments.
Exposed terrain follows formation colours. Construction exclusions use continuous
path segments; trees and stones retain solid footprints while shrubs are traversable.

Rendering uses branching trunks, cutout leaf/needle sprays, two variants per type,
three mesh detail levels, frustum culling and bounded shrub visibility. Scenery
alone carries normals and leaf coordinates, leaving the terrain vertex budget
unchanged. Bark, leaves and rocks have separate procedural material treatments.
Rock meshes now sample one seeded weathered form across all detail levels, with
rounded fracture edges, tilted faces and smooth surface normals. Limestone,
sandstone and basalt retain distinct proportions and material responses; filtered
triplanar grain and shallow relief avoid stretched detail on vertical faces.
Dry stone also has view-dependent, rough dielectric sunlight reflections. Each
geology has its own roughness; filtered normals keep distant highlights broad,
and both geometry and cloud shadows suppress direct reflections.
Use `--rock-capture /tmp/genesis-rock.png --rock limestone` (or `sandstone` / `basalt`)
for a repeatable eye-height inspection of an exposed rock in the current region.
Use `--region island` for basalt; the current Levant recipes contain limestone and sandstone.
Pines have asymmetric branch tiers around a tapered stem; broadleaf trees fork
lower and carry smaller branching crowns plus upright leaders. Riparian forms
retain a taller, narrower crown. Mixed leaf angles give foliage volume from both
eye height and overhead. Crown occlusion shades the interior, curved lighting
normals soften the leaf cards, and sun-facing backlighting adds subtle translucency.
This reuses the existing geometry and rendering passes. Close tree meshes stay below 18,000 vertices, and distant
meshes below 1,500; these are geometry budgets, not frame-rate measurements.
Bark follows each tapered branch, with irregular plate fissures, shallow lighting
relief and separate muted palettes for pine, broadleaf and riparian forms. Detail
fades with projected size; branch geometry and instance storage stay unchanged.
Two 2048² sun-shadow maps cover 320 m and 1.28 km across around the camera focus,
with filtered comparison samples and a blended transition. Foliage sway affects
both maps. Their projections snap independently in light space to reduce crawling
while panning. Wider shadows use simpler caster meshes and omit tiny ground cover;
this remains bounded regional coverage, not planet-wide lighting. The wider layer
adds 16 MiB of depth storage and another shadow pass.
Nearby trees, shrubs and rocks also soften the ground lighting around their bases.
A cached 256 m contact field adds about 1.33 MiB of texture storage, with filtered
detail and a faded boundary. It follows exposed scenery and construction clearance;
this is a footprint approximation, not ray-traced ambient occlusion.

Adjacent scenery detail levels crossfade over 65–105 m and 260–380 m using
complementary, time-independent pixel masks. Range culling fades over the final
18% of visibility instead of shrinking shrubs or abruptly removing rocks. Far
crowns retain sparse leaf sprays and gaps instead of solid polygonal shells.
These are dithered transitions, not mesh morphs; fine stippling can remain visible
during a transition. Grass, bank plants and pebbles use a continuous 28–38 m
camera-distance fade, with a 16 m cached margin to cover camera-key changes.
Local canopy proximity favours fallen leaves beneath broadleaf trees, needles
beneath pines and less grass under dense crowns. Grass, rushes and litter follow
the ground slope and stay clear of exposed boulders and trunk bases. Grass has
fine, varied blades, shaded roots and sun-dependent translucency; its colour
varies with moisture and drought. Small stones also appear away from
riverbanks with their local geological tint. These details are cosmetic, with no
simulated leaf fall or decomposition.

World geometry renders with 4× MSAA, and foliage uses derivative-smoothed leaf
coverage with alpha-to-coverage. The resolved world feeds the glass compositor;
UI stays at native resolution. Bark and stone detail fades with pixel footprint,
and ambient fill keeps shadowed scenery readable. MSAA adds GPU memory and
rendering cost; this is an edge-quality improvement, not a measured speedup or
temporal anti-aliasing solution.

The sky now samples a directional scattering lookup, with configurable molecular
optical depth, aerosol loading, planet radius and atmospheric scale heights.
Clouds occupy a finite volume with layered billows, eroded silhouettes, shaded
interiors and wind drift. Packed shape detail keeps the density lookup compact;
sunlight is integrated towards cloud top, with approximate scattered light in
the shaded interior.
A half-resolution sky pass is capped at 1024×768; terrain edges and the solar disk
remain at display resolution. Sky and water share the clear-sky lookup; water uses
a cheaper version of the same cloud volume. Surface aerial perspective integrates
molecular, aerosol and regional haze layers analytically, preserving nearby detail
and separating distant light extinction from added atmospheric light. Ground cloud shadows
sample a cached, prefiltered sun-transmission map integrated through that volume.
Gentle default shading and height-correct projection keep the moving shade visible
on terrain, buildings, vegetation and water without darkening the landscape heavily. Wind
advects sky, reflections and shadows together. Survey views disable clouds and aerial tint.
This is an initial single-scattering and volumetric-cloud renderer, with approximate
cloud lighting and soft/noisy distant edges. It is not yet a dynamic weather system.

**Region → Sky and wind** offers live clear/fair/overcast previews, cloud cover,
wind speed, cloud-shade strength and a compass heading describing the direction
clouds travel towards.
Wind changes preserve cloud position; calm wind stops motion. Shadow mips are
reused during wind movement and strength edits, and the atmospheric scattering
lookup is refreshed only when altitude or its transport parameters change. These are visual
controls, not simulated weather or saved climate state.

`--weather clear|fair|overcast` selects a presentation preview; `fair` is the default.
For example, `mise exec -- cargo run -p genesis-app -- --weather overcast` launches
with a thicker cloud layer. `--sky-capture /tmp/genesis-sky.png --weather fair`
provides a fixed inspection camera and phase. Rain, snow, storms, fog banks, changing
daylight and weather-driven surface response remain planned; see
[the rendering roadmap](docs/rendering.md#sky-volume-and-weather-foundation).

Scenery is grouped into 160 m clusters and rejected against camera and sun volumes
before individual tests. Each shadow map gets its own caster list, including
off-screen objects, instead of redrawing the camera's complete list. Pixels outside
both maps skip depth samples. Captures report world/scenery submissions and optional
GPU pass timings; invalid counters are marked unavailable. See
[rendering notes](docs/rendering.md) for measured costs, references and remaining limits.

World terrain, water and settlement geometry now use persistent spatial GPU
batches as well. Only changed chunks are uploaded, and camera/sun passes reject
their own invisible chunks. Water and path decals do not cast opaque shadows. Larger batches
keep coarse horizon geometry from generating excessive draw calls. CPU terrain
reconstruction is still regional; this is not yet planetary streaming.

Periodic colour refreshes and camera-detail replacements prepare their scene
geometry, picking data and chunk fingerprints in one background worker. The
current scene stays visible until a matching replacement is ready; outdated edits,
camera buckets and survey results are rejected. This removes a measured ~112 ms
of recurring scene preparation from the input/render thread in the POV fixture.
GPU uploads, initial entry into detailed terrain and explicit edits still have
foreground costs; this is not a guarantee of a particular frame rate.

Ground colour follows living plant coverage, moisture, drought and local geology,
with the same palette continued into the distant context. Close views add irregular
earth patches, centimetre-scale mineral chips, fine grain and subtle surface-normal
relief without increasing terrain vertex size or draw count. Rock detail blends
projections to avoid vertical stretching on slopes; each detail scale fades with
pixel footprint. These are cosmetic surface treatments; geology, collision and
survey readings remain simulation data.

The world now renders into a linear HDR buffer before a neutral display transform.
Warm sunlight, cooler cast shadows and highlight compression add depth while the
glass UI stays outside tone mapping. Survey colours bypass the display curve.
HDR storage adds a full-screen pass and render-target memory; it is an SDR output
pipeline, without automatic exposure or bloom. The visual targets are Anno 1800,
Breath of the Wild and Manor Lords; see [rendering direction and costs](docs/rendering.md).

To repeat the CPU visibility comparison:
`mise exec -- cargo test -p genesis-app visibility_timing -- --ignored --nocapture`.

`--ecology-capture /tmp/ecology.png` frames vegetation near the settlement;
`--ecology-ground-capture /tmp/ecology-ground.png` inspects it at eye height.
Add `--tree broadleaf`, `--tree pine`, or `--tree riparian` to either ecology
capture to inspect the nearest matching tree to the settlement.
`--understorey-capture /tmp/understorey.png --tree broadleaf` places the camera at
human height beneath a crown, looking down at the ground detail.
These accept `--region coast|highlands|island` and `--seed`. Capture windows request
no focus and ignore live keyboard/mouse input so the inspection camera stays fixed.

Next realism work: species-specific authored assets, terrain-conforming roots and
stone bedding, seasonal phenology, leaf litter and deadwood, and streamed vegetation
chunks. Climate currently guides generation;
it does not yet simulate plant succession, growth, reproduction or mortality.
Future ecosystems need their own species/functional catalogues and habitat rules,
not just colour changes to this Mediterranean assemblage. Exoplanet chemistry,
gravity and life forms remain future simulation inputs.
