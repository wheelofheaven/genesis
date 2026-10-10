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
depth and emotional attachment. Manor Lords informed the interest in inhabited
settlements. These references do not imply feature parity.

## Environment and society

Geography and ecology shape settlement and opportunity. Human activity changes
the environment. Households, production, culture, institutions, and power both
emerge from and constrain individual lives. Evaluate new systems by the meaningful
interactions they enable with existing systems.

## Art direction

Use an illustrated 3D world inspired by Sable: clear contours, broad areas of
color, restrained shading, and strong silhouettes. Adapt it to readable
settlement management and close observation of inhabitants. Exact shaders,
outline treatment, texture detail, and camera limits remain exploratory.

Human environments should use historically informed architecture and materials.
Elohim technology can use precise geometry, unusual materials, and controlled
luminous accents within the same rendering language. Carry the style across
regions and periods through shape and rendering; do not make a desert palette
universal. Test detail and outlines at both close and settlement distances.

Phantasy Star Online was an earlier visual reference; Sable is the selected
working direction. The prior Tiny Glade aesthetic and custom rendering roadmap
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
