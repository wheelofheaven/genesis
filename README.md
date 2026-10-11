# Genesis

A life, society, world, and civilization simulator. You belong to an advanced
spacefaring civilization: cross the stars, observe another world, and guide a
local community. The first focus is a colony/city builder with individual lives
at its heart. Earth is the first destination; the architecture should support
future exoplanets and their own environments and societies.

Phantasy Star Online and Xenoblade Chronicles X inform the technological visual
direction. Local settlements retain architecture appropriate to their setting.

Read the [project vision](docs/vision.md) and the
[custom Rust engine decision](docs/engine-decision.md).

**Current status:** an orbital title screen leads through destination selection,
regional scenarios, an expedition briefing, and a skippable settlement arrival.
Frosted instrument panels carry through menus, controls, and survey tools. Display
and navigation preferences persist between launches.
The custom application generates seeded coastal and inland regions with
irregular shores, mountain ridges, drainage networks, woodland patches, and exposed
rock. Six freely positioned dwellings seek suitable terrain near water. Terrain
inspection, terraforming, combined groundwater/mineral surveys, and working
boreholes are playable. Individual people, historical scenarios, and the final
illustrated art pipeline remain next-stage work.

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
currently provide different starting conditions and shared tools; workers,
households, production chains, and expedition logistics remain future systems.

**Menu** (or **Escape** after closing a tool) suspends world time. Return to the
main menu and choose **Continue expedition** to resume the same session. Saving
sessions is not implemented yet; continuing works only while the app stays open.
Replacing a session or quitting asks for confirmation.

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

The clock uses fixed 100 ms ticks, retains fractional time across pause, and
limits catch-up to eight ticks per frame. A frame contributes at most 250 ms of
wall time before the speed multiplier; excess whole-tick backlog is discarded.
Under overload simulation slows relative to wall time without skipping simulation
steps. Hidden or zero-sized windows suspend advancement.

## Development

```sh
mise run test
mise run clippy
mise run fmt-check
mise run build
```

For a native GPU smoke test that exercises camera movement, pause, queued actions,
stepping, free building placement, and resize before writing a PNG and exiting:

```sh
mise exec -- cargo run -- --smoke-test /tmp/genesis-foundation.png
```

`--menu-capture /tmp/genesis-title.png` captures the title menu.
`--menu-test /tmp/genesis-menu.png` checks the destination/region/briefing flow,
small-window settings, asynchronous settlement creation, paused arrival, pause,
and return/continue without losing the prior pause state. It captures each menu,
the arrival, and the resumed world.

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

The 384 × 320 region therefore spans **7.68 × 6.40 km**, sixteen times the previous
map area. This is a working geometry convention, not a claim of historical house accuracy or calibrated hydrology:
resource volumes, time, and existing survey readouts retain their prototype units.
`--settlement-capture /tmp/genesis-settlement.png` captures a close view for scale
review; `--landscape-capture /tmp/genesis-landscape.png` frames its surroundings.
Generation recipe 6 includes local surface detail and preserves jittered vegetation
sampling at 10 m intervals, with fertility and broad patches shaping the groves.
The environmental field still uses 20 m samples. Trees and rocks share GPU meshes;
each placement uses 32 bytes rather than duplicating its geometry. River widths
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
animated waves, sun glints, and a moving shoreline wash. River surfaces
follow the rendered valleys and sit close to the bed, avoiding floating ribbons
and gaps caused by sampling a different surface from the terrain mesh. These are
presentation effects, not simulated waves or a river flow solver; water animation
continues while the simulation is paused. Screenshot captures use a fixed phase.
Ground uses filtered, world-space soil and gravel detail; trees use irregular,
rounded foliage instead of diamond silhouettes. This remains procedural prototype
art rather than scanned terrain or vegetation assets.

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
weather-driven recharge, excavation, mineral extraction, and household water
consumption are not implemented. Exposed formations do not yet discharge water.

## Architecture

- `crates/sim_core`: deterministic simulation, explicit actions/events, scenario fixtures;
  no rendering, windowing, or UI dependencies.
- `crates/renderer`: wgpu resources, procedural geometry, and camera navigation.
- `crates/app`: winit lifecycle, input, fixed clock, egui inspector, and a read-only
  adapter from simulation state to render geometry.

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

## Next milestone

Introduce one persistent person with needs, a daily routine, and an inspector
that explains their choices; establish character loading and animation alongside
that slice. Expand into relationships and a living settlement incrementally.
The original biosphere score remains a prototype mechanic, not the overall goal.

## License

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

`mise exec -- cargo run -- --interaction-test /tmp/genesis-interactions.png`
checks roof picking, path drafting, paused construction, and selecting the result
on the native GPU. It also writes `.house.png` and `.draft.png` captures.
