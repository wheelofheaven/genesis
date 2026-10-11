# Genesis project vision

Agreed direction, October 10, 2026.

Genesis is a long-term life, society, world, and civilization simulator. It
simulates individual lives within evolving societies and a changing natural
world. Players enter through different roles, historical settings, and game
modes, observing and influencing stories that emerge from the simulation.

Each version should deepen the same underlying world and provide a satisfying
playable expansion. The first focus is society and settlement building through
the role of an Elohim supervising inhabitants. The player belongs to a highly
advanced civilization with interplanetary and interstellar travel, life engineering,
and terraforming capabilities. Acting as gods describes the relationship with
local societies; the player's presentation is technological and spacefaring.
These capabilities are a long-term direction, not all implemented systems.

This vision supersedes the original ocean-terraforming MVP and the subsequent
Tiny Glade-inspired, rendering-first roadmap. It describes intended development;
the repository does not yet implement the life or civilization simulation.

## Shared simulation and multiple modes

Keep world identity, scenario content, and player capabilities separate:

| Concept | Responsibility |
| --- | --- |
| Planet definition | Physical properties, geography, atmosphere, climate, and biosphere; Earth is the first supported destination |
| World simulation | Geography, ecology, individuals, households, settlements, resources, institutions, and historical change |
| Scenario | Location, starting period, population, initial conditions, and available knowledge |
| Expedition / game mode | Player role, information access, equipment, resources, interventions, constraints, and objectives |

The long-term aim is to let players choose locations and historical periods.
Each scenario requires appropriate research and content; a date selector alone
cannot supply different technologies, institutions, or architecture. New modes
should reuse the simulation. Implement the first mode before generalizing for
unproven future requirements.

## First mode and setting

The first mode is **Settlement**, a colony/city builder informed by Dwarf Fortress,
Manor Lords, and Songs of Syx. The player is one of the Elohim, operating from an
advanced expedition and guiding a local community. Designate construction,
organize production, develop infrastructure, and manage resources. Individuals
should carry out work through their skills, needs, relationships, and motivations.

Earth is the first destination, with a long-term scenario range from the past
through the present day. The Levant in the tenth century BCE is an initial
historical candidate. The same architecture must admit other planets, biospheres,
and societies later; do not embed Earth or a single period into universal rules.
A complete galaxy or spaceflight simulator is not required for the first mode.

Local settlements retain architecture and material culture appropriate to their
world and period. Advanced surveys, equipment, biological engineering, and
terraforming belong to the player's civilization. Capabilities should eventually
be constrained by equipment, energy, logistics, and consequences. Avoid arbitrary
miracle currencies. Neither these constraints nor individual workers exist yet.

The historical reconstruction and the fictional Elohim premise must remain
distinguishable. History can diverge after supported starting conditions. People
retain agency and interpret interventions through their interests, experiences,
and beliefs; outcomes may differ from the player's intentions.

## Entering the game

The accepted opening is **orbit → destination → regional scenario → briefing →
settlement**. The title screen establishes an expedition above a planet, with
precise instrumentation and spacecraft forms. Continue returns directly to the
active settlement. Settlement is the primary starting experience; World Builder
remains a secondary sandbox using the same systems.

Choose a destination, then a supported place and scenario. Briefings should explain
local conditions, the community, and available interventions. Region selection
must distinguish procedural previews from researched historical reconstruction.
Do not offer arbitrary dates without matching cultures, technology, buildings,
and populations. Future exoplanets must not appear playable before content exists.

A short, skippable arrival links planetary observation to local scale. The first
living-settlement target begins beside an existing community: meet households,
understand needs such as water and shelter, then choose an intervention. Until
people are simulated, show actual dwellings and available survey/build tools;
never invent population numbers, needs, or task completion in the interface.

The surface interface uses light, mostly transparent frosted glass with strong
backdrop blur, dark readable text, and restrained teal highlights. Use consistent
line icons, short captions for main tools, and tooltips for utility controls; avoid
dark tinted slabs and redundant labels. Take layout cues from Anno 1800 and Manor Lords:
region and settlement summaries at the top, actions at the bottom, contextual
inspection at the right, navigation and time at the corners. Keep the central
landscape clear. Adapt to smaller windows through scrolling and compact controls;
retain immediate feedback and consistent keyboard navigation. Menus, surveys,
build tools, and pause/settings surfaces should share this glass treatment.

## Individual lives and emotional investment

Every person should have a persistent identity and a life that matters within
the simulation. The social model should connect:

| Layer | Role |
| --- | --- |
| Immediate needs | Hunger, fatigue, safety, companionship, and other immediate pressures |
| Personality | Relatively persistent tendencies influencing choices |
| Values and beliefs | What a person considers right, meaningful, or true |
| Current desires | Near-term pursuits such as better housing or reconciliation |
| Long-term aspirations | Ambitions spanning years, such as mastering a craft or raising a family |
| Relationships | Affection, trust, resentment, obligation, and rivalry between particular people |
| Memories | Significant experiences that affect subsequent behavior |
| Social circumstances | Household, occupation, resources, status, and institutional ties |

These attributes must cause observable behavior. Depth comes from connections:
food scarcity affects household choices, assistance creates obligations, and
remembered treatment influences trust. Avoid adding traits that have no effect.

Players should be able to follow individuals and households, understand why
people act, and receive meaningful life updates. Aging, family formation,
inheritance, and generational consequences are development horizons. Off-screen
lives must continue to progress. Whether a mode reveals private thoughts fully
or requires the player to learn about people remains an open design choice.

Songs of Syx, The Sims, and Dwarf Fortress are references for the desired social
depth and emotional attachment. Manor Lords informs both inhabited settlements and organic spatial layout:
continuous terrain, freely positioned buildings, curved paths, and irregular plots.
An internal environmental grid must not impose visible tiles or placement snapping. These references do not imply feature parity.

## Environment and society

Geography and ecology shape settlement and opportunity. Human activity changes
the environment. Households, production, culture, institutions, and power both
emerge from and constrain individual lives. Evaluate new systems by the meaningful
interactions they enable with existing systems.

World generation should produce coherent places before settlements are added:
terrain guides drainage, water and soils shape vegetation, and formations constrain
underground resources. Settlements should respond to those conditions. Continuous
positions, irregular boundaries, and curved watercourses must keep the internal
sampling grid out of the player's spatial experience.

Earth is the first supported world: selectable geographic regions such as the Levant,
with continuous terrain above and below sea level. Islands are one landform,
not the universal map structure. Ocean basins, shelves, slopes, and continental
interiors belong to the same elevation surface.

Current place presets provide procedural Levant coastal and inland windows, plus
an island, with geographic anchors, relief, rainfall, drainage, aquifers, deposits,
vegetation, and settlement siting. They remain synthetic interpretations.
Real topography/bathymetry and historical Earth reconstruction require a separate
data layer; see [Earth terrain](earth-terrain.md). Visual plausibility and attached
coordinates are not evidence of scientific or historical accuracy.

## Planetary instrumentation

The Elohim perspective includes advanced surveys of otherwise hidden world state.
Hydrology reveals aquifers, water tables, and underground supplies; geology reveals
host rock and mineral bodies. These are independent layers that can be combined
while the player continues inspecting and shaping the same continuous world.
Use distinct visual channels so combining layers preserves their meaning.

The intended loop is **observe → interpret → intervene → follow consequences**.
Seeing a resource should support decisions about settlements and inhabitants:
where water can be supplied, which deposits justify extraction, and how those
interventions change the environment and society over time. Knowledge does not
make resources inexhaustible. Future interventions should expose their costs and
consequences through the same surveys.

The current slice supplies synthetic groundwater and mineral data, a selected
column profile, and boreholes with depletion and recharge. Mineral extraction and
connections to individual lives remain future systems. Survey instruments belong
to presentation; the underlying resources and interventions belong to the shared
simulation, usable by future modes with different access to information.

## Art direction

The world must remain convincing from an individual human's point of view, not
only from the settlement camera. This is a shared-world requirement for future
individual play. Environmental simulation resolution must not cap ground detail;
see [the human-scale world architecture](human-scale-world.md) for the physical
surface, adaptive rendering, object, streaming and validation boundaries.

Keep proportions grounded in a shared physical scale: modest single-storey homes,
human-sized doors and paths, and compatible trees, rocks, and equipment. Settlements
should occupy a small, believable part of the wider terrain. Illustrated rendering
must preserve these proportions at regional and close observation distances.

Prioritize believable geography and material detail: branching valleys, irregular
coastlines, coastal lowlands, water that communicates depth, and natural vegetation.
The later realism direction supersedes flat, abstract terrain and oversized
geometric forms. Sable remains a reference for colour restraint and readability;
it should not limit terrain detail or physical credibility. Exact shaders,
outline treatment, texture detail, and camera limits remain exploratory.

The renderer's current visual targets are **Anno 1800**, **The Legend of Zelda:
Breath of the Wild**, and **Manor Lords**. For Genesis, interpret these as readable,
richly populated settlements; clear silhouettes, expressive light and atmospheric
depth; and grounded terrain, natural materials and organic settlement placement.
This is a combined direction, not a promise to reproduce any game's rendering
technology or assets. Preserve human scale and ecological distinctions across
close inspection and regional views. Establish lighting, material response and
authored asset quality together; additional shader noise alone cannot deliver
the desired result. Measure costs on the target hardware as quality increases.

The primary technological references are **Phantasy Star Online** and
**Xenoblade Chronicles X**: an advanced civilization capable of exploring worlds,
with vessels, observation platforms, survey drones, and field equipment. These
are references for direction, not requests to copy assets or interfaces.

The player-facing UX uses light, translucent blurred glass, clear icons and
typography, precise geographic diagrams, and restrained luminous accents. Supersede the parchment,
antique gold, and biblical presentation of the early menus. Local communities
remain historically or ecologically grounded; their architecture need not match
the expedition's technology. Sable remains a secondary reference for restraint
and readability, not the overarching setting. The landscape retains believable
materials, scale, and detail rather than becoming a holographic abstraction.

## Iterative development horizons

| Horizon | Added depth | Player experience |
| --- | --- | --- |
| A living settlement | People, households, routines, needs, basic relationships | Know inhabitants and influence daily life |
| Lives unfolding | Aspirations, memories, aging, families, inheritance | Follow lives over time |
| An evolving society | Institutions, beliefs, status, law, collective decisions | See personal actions become social change |
| Connected regions | Trade, migration, diplomacy, cultural exchange | Watch communities transform one another |
| Additional perspectives | More scenarios and playable roles | Experience shared systems through different forms of agency |

These are adjustable horizons, not fixed release promises. The first playable
target is a small community with a few households, food and work routines,
distinct personalities, simple aspirations, relationships, and memories of
consequential events. One meaningful Elohim intervention should produce visible,
understandable consequences over a season. Population size, intervention,
objectives, and detailed acceptance criteria still need design work.

A visual study of the same settlement from oversight and close observation
distances should establish whether the art direction supports both experiences.

## Engineering principles

Use a focused custom Rust engine, initially built on `winit` and `wgpu`, with
engine-independent, deterministic simulation in `sim_core`. The accepted
[engine decision](engine-decision.md) defines the boundaries and transition plan.
Bevy remains in the legacy prototype during migration. Presentation and input
remain separate. Player interventions become explicit actions; important outcomes
become simulation events.

Prioritize stable person identities, save/load with an explicit migration
strategy, reproducibility, and tools explaining individual decisions. These are
implementation priorities, not claims of existing functionality. Schedule
different decisions at appropriate frequencies while preserving individual
continuity. Do not simulate every aspect of every person every rendering frame.

The environmental grid now supports bounded continental windows and wrapped
legacy/island fixtures. Actual Earth coverage requires shared geographic source
data and consistent neighbouring region boundaries; visual context meshes are
not a substitute for that simulation architecture.

Introduce modules and crates when concrete work requires them. Preserve bounded,
stable simulation values, invariant tests, and clear ownership of state.


Scenery must read as a real ecosystem at human scale: trees, bushes, shrubs,
stones and ground cover with coherent sizes, structure and placement. Botany,
climate and geology should determine local assemblages. Regional and future
exoplanet ecosystems must support distinct life forms and material definitions;
a universal tree mesh with biome colour swaps is insufficient. The first
implementation separates functional habitat rules from instanced mesh/material
catalogues, with local sunlight shadows and view-dependent detail.


## Sky and weather direction

The sky should read as a natural atmosphere at human and settlement scales.
Long-term weather scope includes varied cloud types and layers, clear and overcast
skies, wind and gusts, rain and drizzle, snow and sleet, hail, fog and mist, dust
and sand, thunderstorms with lightning and thunder, and their transitions.
Seasonal and diurnal changes should connect the atmosphere to local climate,
hydrology, ecosystems and individual life. Wetness, puddles, snow accumulation,
ice and drying/melting should follow world state rather than independent cosmetic
switches. Phenomena must be selected by each planet's atmosphere and climate;
Earth's blue sky, water cycle and cloud species are not universal assumptions.

This is a staged product direction, not a claim that these systems exist. The
renderer currently exposes molecular/aerosol parameters, layer height, thickness,
coverage, extinction and wind, plus clear/fair/overcast visual previews. A future
deterministic weather model belongs in `sim_core`; the application interpolates
its presentation state and the renderer consumes it. Time of day, precipitation,
surface accumulation, weather audio and gameplay consequences remain future work.
