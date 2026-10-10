# Genesis project vision

Agreed direction, October 10, 2026.

Genesis is a long-term life, society, world, and civilization simulator. It
simulates individual lives within evolving societies and a changing natural
world. Players enter through different roles, historical settings, and game
modes, observing and influencing stories that emerge from the simulation.

Each version should deepen the same underlying world and provide a satisfying
playable expansion. The first focus is society and settlement building through
the role of an Elohim supervising human inhabitants. Terraforming remains a
possible future mode and an existing prototype subsystem.

This vision supersedes the original ocean-terraforming MVP and the subsequent
Tiny Glade-inspired, rendering-first roadmap. It describes intended development;
the repository does not yet implement the life or civilization simulation.

## Shared simulation and multiple modes

Separate three concepts:

| Concept | Responsibility |
| --- | --- |
| World simulation | Geography, ecology, individuals, households, settlements, resources, institutions, and historical change |
| Scenario | Location, starting period, population, initial conditions, and available knowledge |
| Game mode | Player role, information access, interventions, constraints, and objectives |

The long-term aim is to let players choose locations and historical periods.
Each scenario requires appropriate research and content; a date selector alone
cannot supply different technologies, institutions, or architecture. New modes
should reuse the simulation. Implement the first mode before generalizing for
unproven future requirements.

## First mode and setting

The player is one of the Creator Gods, the Elohim, supervising a region of Earth
and interacting with its human inhabitants. The Levant in the 10th century BC
is the initial scenario candidate, not a permanent restriction on the world.

The historical reconstruction and the fictional Elohim premise should remain
distinguishable. History can diverge after the starting conditions.

Human communities should have agency. Possible interventions include appearances,
material assistance, teachings, environmental changes, and relationships with
intermediaries. People interpret interventions through their own interests,
experiences, and beliefs; the consequences can differ from the player's intent.
The exact balance between direct settlement management and indirect influence
remains open.

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

The first seeded generator creates Mediterranean-inspired coastal islands with
relief, rainfall, drainage, aquifers, deposits, vegetation patches, and settlement
siting. It is a synthetic regional prototype. Historical Earth reconstruction,
multiple climate regimes, and evolving river and sediment systems remain separate
development steps; visual plausibility is not evidence of scientific calibration.

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

Human environments should use historically informed architecture and materials.
Elohim technology can use precise geometry, unusual materials, and controlled
luminous accents within the same rendering language. Carry the style across
regions and periods through shape and rendering; do not make a desert palette
universal. Test detail and outlines at both close and settlement distances.

Phantasy Star Online and Sable are visual references rather than fixed rendering
requirements. The prior Tiny Glade aesthetic and custom rendering roadmap
are no longer requirements.

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

The existing wrapped grid is the prototype's spatial model. Revisit spatial
representation deliberately when designing regional Earth scenarios; this vision
does not itself authorize or require an immediate topology rewrite.

Introduce modules and crates when concrete work requires them. Preserve bounded,
stable simulation values, invariant tests, and clear ownership of state.
