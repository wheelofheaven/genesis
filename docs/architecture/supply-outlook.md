# Household supply outlook

21 October 2026, following buffer-controlled pumping.

The HUD's **Supplies** control shows a shortage count when occupied households
lack enough delivered food or water for their next consumption cycle. A household
short of both is counted once. Opening the warning chooses the resource with more
short homes; ties open Water. The ordinary Supplies control still opens the last
selected resource when every household covers its next use.

Water and Food panels show the logical time until the corresponding consumption
boundary and the number of short homes. Short homes sort first, with persistent
home ID breaking ties. Each row shows the amount missing for its next use and any
supplies already being carried home. Clicking still focuses that dwelling by its
identity, including after building-vector reordering. Compact panels scroll.

## Meaning and ownership

`sim_core::supply_outlook::Outlook::read` is a pure query of the existing simulation.
It does not schedule work, change stock, advance clocks, or persist another copy of
needs. Both application and headless metrics use the same query.

Demand uses the actual number of residents in each home: one food portion and
0.5 water units per resident at the existing 30-second consumption intervals.
Each resource exposes its own remaining logical milliseconds, so restored worlds
with differing resource phases display the correct countdowns. Time pauses with
the simulation and follows the selected speed; it is not wall-clock time.

Only delivered household stocks cover demand. Source reserves and resident claims
remain at their original owners. Carried supplies are reported separately until
delivery, even if a route is blocked. Crop irrigation cargo never counts toward
household water. Supplies at one home cannot cover a different home's shortfall.
Empty dwellings have no invented demand; worlds without residents have no countdown
or shortage count.

This is a current-stock check for the next use, not a prediction that a delivery
will arrive in time, a long-term sufficiency estimate, or a health/hunger system.
Existing missed-portion accounting, task priorities and consumption remain intact.
The Food panel also recognizes residents busy carrying crop water, rather than
presenting them as idle while reporting their household food shortage.

## Compatibility and validation

Save schema 10 and rules 8 remain unchanged. No new persisted fields, commands,
external dependencies or gameplay rules are introduced. The headless metrics
record gains a `supply_outlook` object with canonical home ordering, per-resource
countdowns and separate household stock/cargo. Its absence of any population yields
empty `homes` and null countdowns.

Tests cover empty worlds, shared-home occupancy and partial stock, independent
consumption phases, actual consumption boundaries, reserved versus carried versus
delivered resources, irrigation-cargo exclusion, and exact outlook restoration
through JSON save/load. Real egui tests cover warning navigation, shortage-first
ordering, identity-based camera focus and absence of domain mutation or queued
commands. Existing compact-layout and last-row scrolling tests remain in force.

Native review uses `--water-overview-test output.png` on the generated settlement,
with temporary saves separate from the player's slot.
